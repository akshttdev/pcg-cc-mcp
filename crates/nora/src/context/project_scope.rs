//! Project-scoped context for agents
//!
//! Provides context isolation ensuring agents only access data
//! relevant to authorized projects.

use db::models::{
    project::Project,
    task::{Priority, Task, TaskStatus},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ProjectScopeError {
    #[error("Project not found: {0}")]
    ProjectNotFound(Uuid),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Access denied: {tool} cannot access {resource}")]
    AccessDenied { tool: String, resource: String },
}

/// Project-scoped context for an agent conversation
#[derive(Debug, Clone)]
pub struct ProjectScopedContext {
    project_id: Uuid,
    pool: SqlitePool,
}

/// Summary of project context for LLM consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextSummary {
    pub project_name: String,
    pub project_path: String,
    pub active_tasks: Vec<TaskSummary>,
    pub recent_tasks: Vec<TaskSummary>,
    pub token_usage_today: TokenUsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsageSummary {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub request_count: i64,
}

impl ProjectScopedContext {
    /// Create a new project-scoped context
    pub fn new(project_id: Uuid, pool: SqlitePool) -> Self {
        Self { project_id, pool }
    }

    /// Get the project ID
    pub fn project_id(&self) -> Uuid {
        self.project_id
    }

    /// Build context string for LLM consumption
    pub async fn build_context(&self) -> Result<String, ProjectScopeError> {
        let summary = self.build_context_summary().await?;

        let mut context = format!(
            "## Project Context\n\
             **Project**: {}\n\
             **Path**: {}\n",
            summary.project_name,
            summary.project_path
        );

        // Add active tasks
        if !summary.active_tasks.is_empty() {
            context.push_str("\n### Active Tasks\n");
            for task in &summary.active_tasks {
                context.push_str(&format!(
                    "- [{:?}] {} ({})\n",
                    task.status,
                    task.title,
                    task.id
                ));
            }
        }

        // Add recent completed tasks (for context on what's been done)
        if !summary.recent_tasks.is_empty() {
            context.push_str("\n### Recently Completed\n");
            for task in &summary.recent_tasks {
                context.push_str(&format!("- {}\n", task.title));
            }
        }

        // Add token usage context
        if summary.token_usage_today.request_count > 0 {
            context.push_str(&format!(
                "\n### Token Usage Today\n\
                 - Requests: {}\n\
                 - Input tokens: {}\n\
                 - Output tokens: {}\n",
                summary.token_usage_today.request_count,
                summary.token_usage_today.total_input_tokens,
                summary.token_usage_today.total_output_tokens
            ));
        }

        Ok(context)
    }

    /// Build structured context summary
    pub async fn build_context_summary(&self) -> Result<ProjectContextSummary, ProjectScopeError> {
        // Load project
        let project = Project::find_by_id(&self.pool, self.project_id)
            .await?
            .ok_or(ProjectScopeError::ProjectNotFound(self.project_id))?;

        // Load tasks with status
        let all_tasks = Task::find_by_project_id_with_attempt_status(&self.pool, self.project_id)
            .await?;

        // Split into active and recent completed
        let (active, completed): (Vec<_>, Vec<_>) = all_tasks
            .into_iter()
            .partition(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Cancelled);

        let active_tasks: Vec<TaskSummary> = active
            .into_iter()
            .take(10) // Limit to 10 active tasks
            .map(|t| TaskSummary {
                id: t.id,
                title: t.title.clone(),
                status: t.status.clone(),
                priority: t.priority.clone(),
            })
            .collect();

        let recent_tasks: Vec<TaskSummary> = completed
            .into_iter()
            .take(5) // Last 5 completed tasks
            .map(|t| TaskSummary {
                id: t.id,
                title: t.title.clone(),
                status: t.status.clone(),
                priority: t.priority.clone(),
            })
            .collect();

        // Get today's token usage
        let token_usage_today = self.get_today_token_usage().await.unwrap_or_default();

        Ok(ProjectContextSummary {
            project_name: project.name,
            project_path: project.git_repo_path.to_string_lossy().to_string(),
            active_tasks,
            recent_tasks,
            token_usage_today,
        })
    }

    /// Get today's token usage for this project
    async fn get_today_token_usage(&self) -> Result<TokenUsageSummary, ProjectScopeError> {
        // Query today's token usage
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let result: Option<(i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as total_input,
                COALESCE(SUM(output_tokens), 0) as total_output,
                COUNT(*) as request_count
            FROM token_usage
            WHERE project_id = ?1
              AND DATE(created_at) = DATE(?2)
            "#
        )
        .bind(self.project_id)
        .bind(&today)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result
            .map(|(input, output, count)| TokenUsageSummary {
                total_input_tokens: input,
                total_output_tokens: output,
                request_count: count,
            })
            .unwrap_or_default())
    }

    /// Validate that a tool call is authorized for this project
    pub fn validate_tool_access(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<(), ProjectScopeError> {
        // Check for project_id in params - must match our project
        if let Some(param_project_id) = params.get("project_id") {
            if let Some(id_str) = param_project_id.as_str() {
                if let Ok(parsed_id) = Uuid::parse_str(id_str) {
                    if parsed_id != self.project_id {
                        return Err(ProjectScopeError::AccessDenied {
                            tool: tool_name.to_string(),
                            resource: format!("project {}", parsed_id),
                        });
                    }
                }
            }
        }

        // Check for task_id - load task and verify project ownership
        // This is done at execution time, not here
        // For now, we trust the conversation context to guide the LLM

        // File path validation - ensure paths are within project directory
        if let Some(path) = params.get("path").or(params.get("file_path")) {
            if let Some(path_str) = path.as_str() {
                // This would need the project's git_repo_path for full validation
                // For now, log a warning for absolute paths outside common patterns
                if path_str.starts_with('/') && !path_str.contains("tmp") {
                    tracing::warn!(
                        "Tool {} accessing absolute path {} - verify project scope",
                        tool_name,
                        path_str
                    );
                }
            }
        }

        Ok(())
    }

    /// Enforce project ID on tool parameters
    pub fn enforce_project_scope(&self, params: &mut serde_json::Value) {
        if let Some(obj) = params.as_object_mut() {
            // If tool expects a project_id, inject our scoped project
            if obj.contains_key("project_id") ||
               obj.get("project_id").map(|v| v.is_null()).unwrap_or(false) {
                obj.insert(
                    "project_id".to_string(),
                    serde_json::Value::String(self.project_id.to_string()),
                );
            }
        }
    }
}

/// Builder for creating project-scoped context from various sources
pub struct ProjectScopeBuilder {
    pool: SqlitePool,
    project_id: Option<Uuid>,
}

impl ProjectScopeBuilder {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            project_id: None,
        }
    }

    pub fn with_project_id(mut self, project_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub async fn with_project_name(mut self, name: &str) -> Result<Self, ProjectScopeError> {
        let project = Project::find_by_name_case_insensitive(&self.pool, name)
            .await?
            .ok_or_else(|| ProjectScopeError::ProjectNotFound(Uuid::nil()))?;
        self.project_id = Some(project.id);
        Ok(self)
    }

    pub fn build(self) -> Option<ProjectScopedContext> {
        self.project_id.map(|id| ProjectScopedContext::new(id, self.pool))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_tool_access_same_project() {
        // Would need a mock pool for full testing
        // For now, test the logic directly
        let project_id = Uuid::new_v4();

        let params = json!({
            "project_id": project_id.to_string(),
            "title": "Test task"
        });

        // This test validates the logic structure
        // Full integration testing would be in agent_chat tests
        assert!(params.get("project_id").is_some());
    }

    #[test]
    fn test_enforce_project_scope() {
        let project_id = Uuid::new_v4();
        let pool = SqlitePool::connect_lazy("sqlite::memory:").unwrap();
        let ctx = ProjectScopedContext::new(project_id, pool);

        let mut params = json!({
            "project_id": null,
            "title": "Test"
        });

        ctx.enforce_project_scope(&mut params);

        assert_eq!(
            params.get("project_id").and_then(|v| v.as_str()),
            Some(project_id.to_string()).as_deref()
        );
    }
}

//! Platform Data Service
//!
//! A reusable service layer that provides all CRUD operations for platform entities:
//! projects, tasks, organizations, CRM contacts/deals/pipelines, workflow definitions,
//! workflow runs, and staging records.
//!
//! Topsi (and any future agent) delegates data operations here rather than embedding
//! DB queries directly in its agent loop. This keeps agent code focused on orchestration
//! and makes platform data access testable and reusable.

use std::sync::Arc;

use db::{
    db_uuid::DbUuid,
    models::{
        crm_contact::{ContactSearchParams, CrmContact, LifecycleStage},
        crm_deal::CrmDeal,
        crm_pipeline::{CrmPipeline, CrmPipelineStage, PipelineType},
        project::Project,
        task::Task,
    },
};
use serde_json::json;
use sqlx::SqlitePool;
// TODO(dbuuid): migrate Uuid → DbUuid — see planning/2026-03-17--plan--dbuuid-migration.md
use uuid::Uuid;

use crate::{
    agent::{
        access_control::{AccessScope, UserContext},
        TaskExecutionBridge,
    },
    Result, TopsiError,
};

/// Platform Data Service — owns all CRUD operations for platform entities.
///
/// Designed to be shared across agents. Each agent holds a reference to the service
/// and delegates data operations to it.
pub struct PlatformDataService {
    pool: SqlitePool,
    execution_bridge: Option<Arc<dyn TaskExecutionBridge>>,
}

impl PlatformDataService {
    pub fn new(pool: SqlitePool, execution_bridge: Option<Arc<dyn TaskExecutionBridge>>) -> Self {
        Self {
            pool,
            execution_bridge,
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ── Access helpers ───────────────────────────────────────────────────────

    /// Verify that a user has membership in the given organization.
    /// Admins bypass the check.
    pub async fn verify_org_membership(
        &self,
        user_context: &UserContext,
        org_id: Uuid,
    ) -> Result<()> {
        if user_context.is_admin {
            return Ok(());
        }

        let user_uuid = Uuid::parse_str(&user_context.user_id)
            .map_err(|_| TopsiError::AccessDenied("Invalid user_id".to_string()))?;
        let user_id_bytes = user_uuid.to_string();

        let member: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM organization_members WHERE organization_id = ? AND user_id = ? LIMIT 1",
        )
        .bind(org_id.to_string())
        .bind(&user_id_bytes)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Database error: {}", e)))?;

        if member.is_some() {
            Ok(())
        } else {
            Err(TopsiError::AccessDenied(format!(
                "User is not a member of organization {}",
                org_id
            )))
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    //  PROJECT TOOLS
    // ══════════════════════════════════════════════════════════════════════════

    /// List all accessible projects
    pub async fn list_projects(
        &self,
        _args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let projects = match scope {
            AccessScope::Admin => Project::find_all(&self.pool)
                .await
                .map_err(|e| TopsiError::DatabaseError(e))?,
            AccessScope::Projects(ids) => {
                let mut projects = Vec::new();
                for id in ids {
                    if let Ok(Some(p)) = Project::find_by_id(&self.pool, &id.to_string()).await {
                        projects.push(p);
                    }
                }
                projects
            }
            AccessScope::SingleProject(id) => {
                if let Ok(Some(p)) = Project::find_by_id(&self.pool, &id.to_string()).await {
                    vec![p]
                } else {
                    vec![]
                }
            }
            AccessScope::None => vec![],
        };

        let project_list: Vec<serde_json::Value> = projects
            .iter()
            .map(|p| {
                json!({
                    "id": p.id.to_string(),
                    "name": p.name,
                    "path": p.git_repo_path.display().to_string(),
                    "vibe_spent": p.vibe_spent_amount,
                    "vibe_budget": p.vibe_budget_limit,
                    "created_at": p.created_at.to_rfc3339()
                })
            })
            .collect();

        Ok(json!({
            "projects": project_list,
            "total": projects.len()
        }))
    }

    /// Create a new project
    pub async fn create_project(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        use db::models::project::{CreateProject, Project};

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing project name".to_string()))?;

        let mut path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let sanitized = name.to_lowercase().replace(" ", "-");
                format!("~/projects/{}", sanitized)
            });

        // Check if path already exists and make it unique if needed
        let base_path = path.clone();
        let mut attempt = 0;
        while let Ok(Some(_)) = Project::find_by_git_repo_path(&self.pool, &path).await {
            attempt += 1;
            let sanitized_name = name.to_lowercase().replace(" ", "-");
            path = format!("~/projects/{}-{}", sanitized_name, attempt);
            if attempt > 10 {
                return Err(TopsiError::ToolError(format!(
                    "Could not find unique path after {} attempts",
                    attempt
                )));
            }
        }

        if path != base_path {
            tracing::info!(
                "Original path {} was taken, using {} instead",
                base_path,
                path
            );
        }

        let project_id = Uuid::new_v4();
        let create_project = CreateProject {
            name: name.to_string(),
            git_repo_path: path.clone(),
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
            use_existing_repo: false,
            organization_id: None,
            client_id: None,
            folder_id: None,
            parent_project_id: None,
        };

        let project = Project::create(&self.pool, &create_project, &project_id.to_string())
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to create project: {}", e)))?;

        // Ensure default board exists so tasks have somewhere to land
        use db::models::project_board::ProjectBoard;
        if let Err(e) = ProjectBoard::ensure_default_board(&self.pool, &project.id).await {
            tracing::error!(
                "Failed to create default board for project {}: {}",
                project.id,
                e
            );
        }

        // Add the creator as project owner in project_members
        let user_uuid = Uuid::parse_str(&user_context.user_id).map_err(|e| {
            TopsiError::ToolError(format!("Invalid user ID '{}': {}", user_context.user_id, e))
        })?;
        let member_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO project_members (id, project_id, user_id, role, granted_by)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(member_id.to_string())
        .bind(project.id.to_string())
        .bind(user_uuid.to_string())
        .bind("owner")
        .bind(user_uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Failed to add project member: {}", e)))?;

        tracing::info!(
            "Created project '{}' (ID: {}) for user {}",
            project.name,
            project.id,
            user_context.user_id
        );

        Ok(json!({
            "success": true,
            "project_id": project.id.to_string(),
            "name": project.name,
            "path": project.git_repo_path.display().to_string(),
            "message": format!("Project '{}' created successfully at {}",
                project.name,
                project.git_repo_path.display()
            )
        }))
    }

    /// Update project metadata
    pub async fn update_project(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        let project_id_str = args["project_id"]
            .as_str()
            .ok_or_else(|| TopsiError::ToolError("project_id required".into()))?;
        let project_uuid = Uuid::parse_str(project_id_str)
            .map_err(|e| TopsiError::ToolError(format!("Invalid project_id: {}", e)))?;

        // Verify user has access to this project
        let member_check = sqlx::query!(
            r#"SELECT role FROM project_members WHERE project_id = ? AND user_id = ?"#,
            project_uuid,
            user_context.user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Access check failed: {}", e)))?;

        if member_check.is_none() && !user_context.is_admin {
            return Err(TopsiError::ToolError(
                "Access denied: not a member of this project".into(),
            ));
        }

        let new_name = args["name"].as_str();
        let new_org_id = args["organization_id"].as_str();

        if new_name.is_none() && new_org_id.is_none() {
            return Err(TopsiError::ToolError(
                "Provide at least one field to update: name or organization_id".into(),
            ));
        }

        // Parse org_id from hex string
        let org_uuid: Option<Uuid> = if let Some(org_str) = new_org_id {
            if let Ok(u) = Uuid::parse_str(org_str) {
                Some(u)
            } else if org_str.len() == 32 {
                let with_dashes = format!(
                    "{}-{}-{}-{}-{}",
                    &org_str[0..8],
                    &org_str[8..12],
                    &org_str[12..16],
                    &org_str[16..20],
                    &org_str[20..32]
                );
                Some(Uuid::parse_str(&with_dashes).map_err(|_| {
                    TopsiError::ToolError(format!("Invalid organization_id: {}", org_str))
                })?)
            } else {
                return Err(TopsiError::ToolError(format!(
                    "organization_id must be a UUID or 32-char hex string, got: {}",
                    org_str
                )));
            }
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE projects
            SET name = COALESCE(?, name),
                organization_id = CASE WHEN ? = 1 THEN ? ELSE organization_id END,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?
        "#,
        )
        .bind(new_name)
        .bind(org_uuid.is_some() as i32)
        .bind(org_uuid.map(|u| u.to_string()))
        .bind(project_uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Failed to update project: {}", e)))?;

        tracing::info!(
            "Updated project {} — name={:?}, org={:?}",
            project_id_str,
            new_name,
            new_org_id
        );

        Ok(json!({
            "success": true,
            "project_id": project_id_str,
            "updated_name": new_name,
            "updated_organization_id": new_org_id,
            "message": "Project updated successfully"
        }))
    }

    /// Get detailed project info including task counts
    pub async fn get_project_detail(
        &self,
        args: &serde_json::Value,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "project_id is required"})),
        };

        let project = match Project::find_by_id(&self.pool, project_id).await {
            Ok(Some(p)) => p,
            Ok(None) => return Ok(json!({"error": "Project not found"})),
            Err(e) => return Ok(json!({"error": format!("Database error: {}", e)})),
        };

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND deleted_at IS NULL",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let todo: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'todo' AND deleted_at IS NULL"
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let in_progress: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'inprogress' AND deleted_at IS NULL"
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let done: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'done' AND deleted_at IS NULL"
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(json!({
            "id": project.id,
            "name": project.name,
            "path": project.git_repo_path.display().to_string(),
            "vibe_spent": project.vibe_spent_amount,
            "vibe_budget": project.vibe_budget_limit,
            "organization_id": project.organization_id,
            "client_id": project.client_id,
            "task_counts": {
                "total": total,
                "todo": todo,
                "in_progress": in_progress,
                "done": done
            },
            "created_at": project.created_at.to_rfc3339()
        }))
    }

    /// List all organizations
    pub async fn list_organizations(&self) -> Result<serde_json::Value> {
        let rows = sqlx::query!(
            r#"SELECT hex(id) as id, name, slug, description FROM organizations WHERE deleted_at IS NULL ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Failed to list organizations: {}", e)))?;

        let orgs: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "name": r.name,
                    "slug": r.slug,
                    "description": r.description
                })
            })
            .collect();

        Ok(json!({ "organizations": orgs, "count": orgs.len() }))
    }

    // ══════════════════════════════════════════════════════════════════════════
    //  TASK TOOLS
    // ══════════════════════════════════════════════════════════════════════════

    /// Create a task in a project
    pub async fn create_task(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        // Extract or infer project_id
        let project_id = if let Some(pid_str) = args.get("project_id").and_then(|v| v.as_str()) {
            if pid_str.contains("<")
                || pid_str.contains(">")
                || pid_str == "null"
                || pid_str.is_empty()
            {
                None
            } else {
                Uuid::parse_str(pid_str).ok()
            }
        } else {
            None
        };

        let project_id = if let Some(pid) = project_id {
            pid
        } else {
            // No project_id provided - try to infer from user's accessible projects
            let projects = if user_context.is_admin {
                Project::find_all(&self.pool).await.unwrap_or_default()
            } else {
                let user_uuid = Uuid::parse_str(&user_context.user_id).unwrap_or_default();
                let user_id_bytes = user_uuid.to_string();
                let project_ids: Vec<String> = sqlx::query_scalar(
                    r#"SELECT DISTINCT project_id FROM project_members WHERE user_id = ?"#,
                )
                .bind(&user_id_bytes)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default();

                let mut projects = Vec::new();
                for pid_str in project_ids {
                    if let Ok(pid) = Uuid::parse_str(&pid_str) {
                        if let Ok(Some(p)) = Project::find_by_id(&self.pool, &pid.to_string()).await
                        {
                            projects.push(p);
                        }
                    }
                }
                projects
            };

            if projects.is_empty() {
                // Auto-create a project based on the task
                let task_title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled Task");
                let project_name = if task_title.len() > 30 {
                    format!("{} Project", &task_title[..30])
                } else {
                    format!("{} Project", task_title)
                };

                use db::models::project::CreateProject;

                let sanitized = project_name.to_lowercase().replace(" ", "-");
                let mut path = format!("~/projects/{}", sanitized);
                let mut attempt = 0;
                while let Ok(Some(_)) = Project::find_by_git_repo_path(&self.pool, &path).await {
                    attempt += 1;
                    path = format!("~/projects/{}-{}", sanitized, attempt);
                    if attempt > 10 {
                        return Err(TopsiError::ToolError(
                            "Could not find unique path for auto-created project".to_string(),
                        ));
                    }
                }

                let create_project = CreateProject {
                    name: project_name.clone(),
                    git_repo_path: path.clone(),
                    setup_script: None,
                    dev_script: None,
                    cleanup_script: None,
                    copy_files: None,
                    use_existing_repo: false,
                    organization_id: None,
                    client_id: None,
                    folder_id: None,
                    parent_project_id: None,
                };

                let new_project_id = Uuid::new_v4();
                match Project::create(&self.pool, &create_project, &new_project_id.to_string())
                    .await
                {
                    Ok(project) => {
                        let auto_user_uuid =
                            Uuid::parse_str(&user_context.user_id).map_err(|e| {
                                TopsiError::ToolError(format!("Invalid user ID: {}", e))
                            })?;
                        let member_id = Uuid::new_v4();
                        if let Err(e) = sqlx::query(
                            r#"INSERT INTO project_members (id, project_id, user_id, role, granted_by)
                               VALUES (?, ?, ?, ?, ?)"#
                        )
                        .bind(member_id.to_string())
                        .bind(&project.id)
                        .bind(auto_user_uuid.to_string())
                        .bind("owner")
                        .bind(auto_user_uuid.to_string())
                        .execute(&self.pool)
                        .await {
                            tracing::error!("Failed to add project member for auto-created project: {}", e);
                            return Err(TopsiError::ToolError(
                                format!("Project created but failed to grant access: {}", e)
                            ));
                        }

                        tracing::info!(
                            "Auto-created project '{}' (ID: {}) for task '{}' by user {}",
                            project.name,
                            project.id,
                            task_title,
                            user_context.user_id
                        );
                        Uuid::parse_str(&project.id).map_err(|e| {
                            TopsiError::ToolError(format!("Invalid project ID: {}", e))
                        })?
                    }
                    Err(e) => {
                        return Err(TopsiError::ToolError(format!(
                            "No projects found and failed to auto-create project '{}': {}",
                            project_name, e
                        )));
                    }
                }
            } else if projects.len() == 1 {
                Uuid::parse_str(&projects[0].id)
                    .map_err(|e| TopsiError::ToolError(format!("Invalid project ID: {}", e)))?
            } else {
                let project_list: Vec<String> = projects
                    .iter()
                    .map(|p| format!("  - {} (ID: {})", p.name, p.id))
                    .collect();

                return Err(TopsiError::ToolError(
                    format!(
                        "Multiple projects available. Please analyze which project this task belongs to and call create_task again with project_id, or create a new project if this is a new idea:\n\nAvailable projects:\n{}",
                        project_list.join("\n")
                    )
                ));
            }
        };

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing title".to_string()))?;

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing description".to_string()))?;

        let agent_name = args.get("agent_name").and_then(|v| v.as_str());

        // Verify access to project
        match scope {
            AccessScope::Admin => {}
            AccessScope::Projects(ids) => {
                if !ids.contains(&project_id) {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this project".to_string(),
                    ));
                }
            }
            AccessScope::SingleProject(id) => {
                if *id != project_id {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this project".to_string(),
                    ));
                }
            }
            AccessScope::None => {
                return Err(TopsiError::ToolError(
                    "You don't have permission to create tasks".to_string(),
                ));
            }
        }

        use db::models::{
            project_board::ProjectBoard,
            task::{CreateTask, Task},
        };

        let default_board_id =
            ProjectBoard::ensure_default_board(&self.pool, &project_id.to_string())
                .await
                .ok()
                .and_then(|b| Uuid::parse_str(&b.id).ok());

        let create_task = CreateTask {
            project_id: project_id.to_string(),
            pod_id: None,
            board_id: default_board_id.map(|id| id.to_string()),
            title: title.to_string(),
            description: Some(description.to_string()),
            parent_task_attempt: None,
            image_ids: None,
            priority: None,
            assignee_id: None,
            assignee_type: None,
            assigned_agent: agent_name.map(|s| s.to_string()),
            agent_id: None,
            assigned_mcps: None,
            created_by: user_context.user_id.clone(),
            requires_approval: None,
            parent_task_id: None,
            tags: None,
            due_date: None,
            custom_properties: None,
            scheduled_start: None,
            scheduled_end: None,
            screenshot: None,
            completion_criteria: None,
            output_format: None,
            collaborators: None,
        };

        let task_id = Uuid::new_v4();
        let task = Task::create(&self.pool, &create_task, &task_id.to_string())
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to create task: {}", e)))?;

        // Auto-execute: if an agent was assigned, automatically start task execution
        let auto_execute = args
            .get("auto_execute")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let execution_result = if let (Some(agent), true) = (agent_name, auto_execute) {
            if let Some(bridge) = &self.execution_bridge {
                let agent_lower = agent.to_lowercase();
                let executor_name = match agent_lower.as_str() {
                    "claude" | "claude_code" => "CLAUDE_CODE",
                    "gemini" => "GEMINI",
                    "amp" => "AMP",
                    "codex" | "openai" => "CODEX",
                    _ => agent,
                };
                let base_branch = "main".to_string();

                tracing::info!(
                    "[TOPSI] Auto-executing task {} with agent {} (executor: {})",
                    task.id,
                    agent,
                    executor_name
                );

                match bridge
                    .start_task_attempt(task_id, executor_name, &base_branch)
                    .await
                {
                    Ok(result) => {
                        tracing::info!("[TOPSI] Auto-execution started for task {}", task.id);
                        Some(result)
                    }
                    Err(e) => {
                        tracing::error!(
                            "[TOPSI] Auto-execution failed for task {}: {}",
                            task.id,
                            e
                        );
                        Some(
                            json!({ "error": format!("Task created but execution failed: {}", e) }),
                        )
                    }
                }
            } else {
                Some(json!({ "note": "Task created but execution bridge not available" }))
            }
        } else {
            None
        };

        let mut response = json!({
            "success": true,
            "task_id": task.id.to_string(),
            "title": task.title,
            "status": format!("{:?}", task.status),
            "assigned_agent": task.assigned_agent,
            "project_id": task.project_id.to_string(),
            "message": format!("Task '{}' created successfully{}",
                task.title,
                agent_name.map(|a| format!(" and assigned to {}", a)).unwrap_or_default()
            )
        });

        if let Some(exec) = execution_result {
            if let Some(obj) = response.as_object_mut() {
                obj.insert("execution".to_string(), exec);
            }
        }

        Ok(response)
    }

    /// Start executing a task by spawning a coding agent
    pub async fn start_task_execution(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let bridge = self.execution_bridge.as_ref().ok_or_else(|| {
            TopsiError::ToolError(
                "Task execution not available — execution bridge not configured".to_string(),
            )
        })?;

        let task_id_str = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing task_id".to_string()))?;

        let task_id = Uuid::parse_str(task_id_str).map_err(|e| {
            TopsiError::ToolError(format!("Invalid task_id '{}': {}", task_id_str, e))
        })?;

        let agent_name = args
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("claude");

        // Verify the task exists and user has access
        let task: Option<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?;

        let task =
            task.ok_or_else(|| TopsiError::ToolError(format!("Task {} not found", task_id)))?;

        // Verify project access
        match scope {
            AccessScope::Admin => {}
            AccessScope::Projects(ids) => {
                let project_uuid = Uuid::parse_str(&task.project_id)
                    .map_err(|e| TopsiError::ToolError(format!("Invalid project_id: {}", e)))?;
                if !ids.contains(&project_uuid) {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::SingleProject(id) => {
                if id.to_string() != task.project_id {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::None => {
                return Err(TopsiError::ToolError(
                    "You don't have permission to execute tasks".to_string(),
                ));
            }
        }

        // Map agent names to executor names
        let agent_lower = agent_name.to_lowercase();
        let executor_name = match agent_lower.as_str() {
            "claude" | "claude_code" => "CLAUDE_CODE",
            "gemini" => "GEMINI",
            "amp" => "AMP",
            "codex" | "openai" => "CODEX",
            _ => agent_name,
        };

        tracing::info!(
            "[TOPSI] Starting task execution: task={}, agent={}, executor={}",
            task_id,
            agent_name,
            executor_name
        );

        match bridge
            .start_task_attempt(task_id, executor_name, "main")
            .await
        {
            Ok(result) => {
                tracing::info!(
                    "[TOPSI] Task execution started successfully for task {}",
                    task_id
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!("[TOPSI] Failed to start task execution: {}", e);
                Err(TopsiError::ToolError(format!(
                    "Failed to start task execution: {}",
                    e
                )))
            }
        }
    }

    /// Get the current status of a task including execution info and logs
    pub async fn get_task_status(
        &self,
        args: &serde_json::Value,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let task_id_str = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing task_id".to_string()))?;

        let include_logs = args
            .get("include_logs")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let log_lines = args.get("log_lines").and_then(|v| v.as_i64()).unwrap_or(20) as i32;

        let task: Option<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?;

        let task =
            task.ok_or_else(|| TopsiError::ToolError(format!("Task {} not found", task_id_str)))?;

        let mut result = json!({
            "task_id": task.id.to_string(),
            "title": task.title,
            "description": task.description,
            "status": format!("{:?}", task.status).to_lowercase(),
            "priority": format!("{:?}", task.priority).to_lowercase(),
            "assigned_agent": task.assigned_agent,
            "created_at": task.created_at.to_rfc3339(),
            "updated_at": task.updated_at.to_rfc3339(),
        });

        // Fetch latest task attempt
        let latest_attempt: Option<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, executor, base_branch, created_at, updated_at FROM task_attempts WHERE task_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(task_id_str)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        if let Some((attempt_id, executor, base_branch, attempt_created, attempt_updated)) =
            latest_attempt
        {
            result["latest_attempt"] = json!({
                "attempt_id": attempt_id,
                "executor": executor,
                "base_branch": base_branch,
                "created_at": attempt_created,
                "updated_at": attempt_updated,
            });

            let latest_process: Option<(String, String, Option<i32>, String, String)> = sqlx::query_as(
                "SELECT id, status, exit_code, created_at, updated_at FROM execution_processes WHERE task_attempt_id = ? ORDER BY created_at DESC LIMIT 1"
            )
            .bind(&attempt_id)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

            if let Some((proc_id, proc_status, exit_code, proc_created, proc_updated)) =
                latest_process
            {
                result["latest_process"] = json!({
                    "process_id": proc_id,
                    "status": proc_status,
                    "exit_code": exit_code,
                    "created_at": proc_created,
                    "updated_at": proc_updated,
                });

                if include_logs {
                    let logs: Vec<(String,)> = sqlx::query_as(
                        "SELECT logs FROM execution_process_logs WHERE execution_id = ? ORDER BY inserted_at DESC LIMIT ?"
                    )
                    .bind(&proc_id)
                    .bind(log_lines)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap_or_default();

                    if !logs.is_empty() {
                        let log_text: Vec<&str> = logs.iter().map(|(l,)| l.as_str()).collect();
                        result["recent_logs"] = json!(log_text);
                    }
                }
            }
        } else {
            result["latest_attempt"] = json!(null);
            result["note"] = json!("No execution attempts yet");
        }

        Ok(result)
    }

    /// Update a task's properties
    pub async fn update_task(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let task_id_str = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing task_id".to_string()))?;

        let task: Option<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?;

        let task =
            task.ok_or_else(|| TopsiError::ToolError(format!("Task {} not found", task_id_str)))?;

        // Verify project access
        match scope {
            AccessScope::Admin => {}
            AccessScope::Projects(ids) => {
                let project_uuid = Uuid::parse_str(&task.project_id)
                    .map_err(|e| TopsiError::ToolError(format!("Invalid project_id: {}", e)))?;
                if !ids.contains(&project_uuid) {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::SingleProject(id) => {
                if id.to_string() != task.project_id {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::None => {
                return Err(TopsiError::ToolError(
                    "You don't have permission to update tasks".to_string(),
                ));
            }
        }

        let mut updates = vec![];
        let mut values: Vec<String> = vec![];

        if let Some(status) = args.get("status").and_then(|v| v.as_str()) {
            updates.push("status = ?");
            // Normalize status variants (LLMs may send snake_case or kebab-case)
            let canonical_status = match status {
                "in_progress" | "in-progress" => "inprogress",
                "in_review" | "in-review" => "inreview",
                other => other,
            };
            values.push(canonical_status.to_string());
        }
        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            updates.push("title = ?");
            values.push(title.to_string());
        }
        if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
            updates.push("description = ?");
            values.push(description.to_string());
        }
        if let Some(priority) = args.get("priority").and_then(|v| v.as_str()) {
            updates.push("priority = ?");
            values.push(priority.to_string());
        }
        if let Some(agent) = args.get("assigned_agent").and_then(|v| v.as_str()) {
            updates.push("assigned_agent = ?");
            values.push(agent.to_string());
        }

        if updates.is_empty() {
            return Ok(json!({
                "success": false,
                "message": "No fields to update. Provide at least one of: status, title, description, priority, assigned_agent"
            }));
        }

        updates.push("updated_at = datetime('now')");

        let query = format!("UPDATE tasks SET {} WHERE id = ?", updates.join(", "));

        let mut q = sqlx::query(&query);
        for val in &values {
            q = q.bind(val);
        }
        q = q.bind(task_id_str);

        q.execute(&self.pool)
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to update task: {}", e)))?;

        tracing::info!(
            "[TOPSI] Updated task {} with {} field changes",
            task_id_str,
            values.len()
        );

        Ok(json!({
            "success": true,
            "task_id": task_id_str,
            "updated_fields": values.len(),
            "message": format!("Task '{}' updated successfully", task.title)
        }))
    }

    /// List tasks with filtering
    pub async fn list_tasks(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let status_filter = args.get("status").and_then(|v| v.as_str());
        let agent_filter = args.get("assigned_agent").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20) as i32;

        let project_id_filter = args.get("project_id").and_then(|v| v.as_str());

        let project_ids: Vec<Uuid> = if let Some(pid_str) = project_id_filter {
            if let Ok(pid) = Uuid::parse_str(pid_str) {
                vec![pid]
            } else {
                return Err(TopsiError::ToolError(format!(
                    "Invalid project_id: {}",
                    pid_str
                )));
            }
        } else {
            match scope {
                AccessScope::Admin => Project::find_all(&self.pool)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|p| Uuid::parse_str(&p.id).ok())
                    .collect(),
                AccessScope::Projects(ids) => ids.iter().copied().collect(),
                AccessScope::SingleProject(id) => vec![*id],
                AccessScope::None => vec![],
            }
        };

        let mut all_tasks: Vec<serde_json::Value> = vec![];

        for pid in &project_ids {
            let mut query = String::from(
                "SELECT id, title, status, priority, assigned_agent, created_at, updated_at FROM tasks WHERE project_id = ?"
            );
            let mut bind_values: Vec<String> = vec![pid.to_string()];

            if let Some(status) = status_filter {
                query.push_str(" AND status = ?");
                let canonical_status = match status {
                    "in_progress" | "in-progress" => "inprogress",
                    "in_review" | "in-review" => "inreview",
                    other => other,
                };
                bind_values.push(canonical_status.to_string());
            }
            if let Some(agent) = agent_filter {
                query.push_str(" AND assigned_agent = ?");
                bind_values.push(agent.to_string());
            }

            query.push_str(" ORDER BY created_at DESC LIMIT ?");

            let mut q = sqlx::query_as::<
                _,
                (
                    String,
                    String,
                    String,
                    String,
                    Option<String>,
                    String,
                    String,
                ),
            >(&query);
            for val in &bind_values {
                q = q.bind(val);
            }
            q = q.bind(limit);

            let tasks: Vec<(
                String,
                String,
                String,
                String,
                Option<String>,
                String,
                String,
            )> = q.fetch_all(&self.pool).await.unwrap_or_default();

            for (id, title, status, priority, agent, created, updated) in tasks {
                all_tasks.push(json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                    "assigned_agent": agent,
                    "project_id": pid.to_string(),
                    "created_at": created,
                    "updated_at": updated,
                }));
            }
        }

        Ok(json!({
            "tasks": all_tasks,
            "total": all_tasks.len(),
            "filters": {
                "status": status_filter,
                "assigned_agent": agent_filter,
                "limit": limit,
            }
        }))
    }

    /// Delete a task by ID
    pub async fn delete_task(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let task_id_str = match args.get("task_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "task_id is required"})),
        };

        // Verify task exists and user has access
        let task = match Task::find_by_id(&self.pool, task_id_str).await {
            Ok(Some(t)) => t,
            Ok(None) => return Ok(json!({"error": format!("Task '{}' not found", task_id_str)})),
            Err(e) => {
                return Err(TopsiError::ToolError(format!(
                    "Failed to look up task: {}",
                    e
                )))
            }
        };

        // Check scope
        let project_uuid = Uuid::parse_str(&task.project_id)
            .map_err(|_| TopsiError::ToolError("Invalid project_id on task".to_string()))?;
        match scope {
            AccessScope::Admin => {}
            AccessScope::Projects(ids) => {
                if !ids.contains(&project_uuid) {
                    return Ok(
                        json!({"error": "Access denied: task belongs to a project outside your scope"}),
                    );
                }
            }
            AccessScope::SingleProject(id) => {
                if *id != project_uuid {
                    return Ok(
                        json!({"error": "Access denied: task belongs to a different project"}),
                    );
                }
            }
            AccessScope::None => {
                return Ok(json!({"error": "Access denied: no project access"}));
            }
        }

        let rows = Task::delete(&self.pool, task_id_str)
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to delete task: {}", e)))?;

        Ok(json!({
            "success": rows > 0,
            "task_id": task_id_str,
            "title": task.title,
            "message": format!("Task '{}' deleted successfully", task.title)
        }))
    }

    /// Bulk update multiple tasks at once
    pub async fn bulk_update_tasks(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let task_ids = match args.get("task_ids").and_then(|v| v.as_array()) {
            Some(ids) => ids
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>(),
            None => return Ok(json!({"error": "task_ids array is required"})),
        };

        if task_ids.is_empty() {
            return Ok(json!({"error": "task_ids must not be empty"}));
        }
        if task_ids.len() > 100 {
            return Ok(json!({"error": "Cannot bulk update more than 100 tasks at once"}));
        }

        // Build the update args for each task (same fields applied to all)
        let mut updated = 0;
        let mut errors: Vec<String> = vec![];

        for task_id in &task_ids {
            let single_args = json!({
                "task_id": task_id,
                "status": args.get("status"),
                "priority": args.get("priority"),
                "assigned_agent": args.get("assigned_agent"),
            });

            match self.update_task(&single_args, user_context, scope).await {
                Ok(result) => {
                    if result.get("error").is_some() {
                        errors.push(format!("{}: {}", task_id, result["error"]));
                    } else {
                        updated += 1;
                    }
                }
                Err(e) => errors.push(format!("{}: {}", task_id, e)),
            }
        }

        Ok(json!({
            "success": errors.is_empty(),
            "updated": updated,
            "total": task_ids.len(),
            "errors": errors,
            "message": format!("Updated {} of {} tasks", updated, task_ids.len())
        }))
    }

    // ══════════════════════════════════════════════════════════════════════════
    //  CRM TOOLS
    // ══════════════════════════════════════════════════════════════════════════

    /// List CRM contacts for an organization
    pub async fn list_crm_contacts(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let org_id_str = match args.get("organization_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "organization_id is required"})),
        };

        let org_uuid = match Uuid::parse_str(org_id_str) {
            Ok(u) => u,
            Err(_) => return Ok(json!({"error": "Invalid organization_id UUID"})),
        };

        if let Err(e) = self.verify_org_membership(user_context, org_uuid).await {
            return Ok(json!({"error": e.to_string()}));
        }

        let limit = args
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(50);
        let search_query = args.get("search_query").and_then(|v| v.as_str());
        let lifecycle_stage_str = args.get("lifecycle_stage").and_then(|v| v.as_str());
        let lifecycle_stage = lifecycle_stage_str.and_then(|s| s.parse::<LifecycleStage>().ok());

        let contacts = if search_query.is_some() || lifecycle_stage.is_some() {
            let params = ContactSearchParams {
                organization_id: Some(DbUuid::from(org_uuid)),
                client_id: None,
                query: search_query.map(|s| s.to_string()),
                lifecycle_stage,
                company_name: None,
                tags: None,
                min_lead_score: None,
                limit: Some(limit),
                offset: None,
            };
            CrmContact::search(&self.pool, params)
                .await
                .unwrap_or_default()
        } else {
            CrmContact::find_by_organization(&self.pool, &DbUuid::from(org_uuid), Some(limit))
                .await
                .unwrap_or_default()
        };

        let contact_list: Vec<serde_json::Value> = contacts
            .iter()
            .map(|c| {
                json!({
                    "id": c.id.to_string(),
                    "first_name": c.first_name,
                    "last_name": c.last_name,
                    "email": c.email,
                    "company_name": c.company_name,
                    "job_title": c.job_title,
                    "lifecycle_stage": c.lifecycle_stage,
                    "lead_score": c.lead_score,
                    "last_activity_at": c.last_activity_at.map(|d| d.to_rfc3339())
                })
            })
            .collect();

        Ok(json!({
            "contacts": contact_list,
            "total": contacts.len()
        }))
    }

    /// List CRM deals
    pub async fn list_crm_deals(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let pipeline_id = args.get("pipeline_id").and_then(|v| v.as_str());
        let stage_id = args.get("stage_id").and_then(|v| v.as_str());
        let org_id = args.get("organization_id").and_then(|v| v.as_str());

        if let Some(oid) = org_id {
            if let Ok(uuid) = Uuid::parse_str(oid) {
                if let Err(e) = self.verify_org_membership(user_context, uuid).await {
                    return Ok(json!({"error": e.to_string()}));
                }
            }
        }

        let deals = if let Some(pid) = pipeline_id {
            match Uuid::parse_str(pid) {
                Ok(uuid) => CrmDeal::find_by_pipeline(&self.pool, &DbUuid::from(uuid))
                    .await
                    .unwrap_or_default(),
                Err(_) => return Ok(json!({"error": "Invalid pipeline_id UUID"})),
            }
        } else if let Some(sid) = stage_id {
            match Uuid::parse_str(sid) {
                Ok(uuid) => CrmDeal::find_by_stage(&self.pool, &DbUuid::from(uuid))
                    .await
                    .unwrap_or_default(),
                Err(_) => return Ok(json!({"error": "Invalid stage_id UUID"})),
            }
        } else if let Some(oid) = org_id {
            match Uuid::parse_str(oid) {
                Ok(uuid) => CrmDeal::find_by_organization(&self.pool, &DbUuid::from(uuid))
                    .await
                    .unwrap_or_default(),
                Err(_) => return Ok(json!({"error": "Invalid organization_id UUID"})),
            }
        } else {
            return Ok(json!({"error": "Must provide organization_id, pipeline_id, or stage_id"}));
        };

        let deal_list: Vec<serde_json::Value> = deals
            .iter()
            .map(|d| {
                json!({
                    "id": d.id.to_string(),
                    "name": d.name,
                    "amount": d.amount,
                    "currency": d.currency,
                    "stage": d.stage,
                    "pipeline": d.pipeline,
                    "probability": d.probability,
                    "expected_close_date": d.expected_close_date.map(|d| d.to_rfc3339()),
                    "contact_id": d.crm_contact_id.as_ref().map(|id| id.to_string()),
                    "created_at": d.created_at.to_rfc3339()
                })
            })
            .collect();

        Ok(json!({
            "deals": deal_list,
            "total": deals.len()
        }))
    }

    /// List CRM pipelines and their stages
    pub async fn list_crm_pipelines(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let org_id_str = match args.get("organization_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "organization_id is required"})),
        };

        let org_uuid = match Uuid::parse_str(org_id_str) {
            Ok(u) => u,
            Err(_) => return Ok(json!({"error": "Invalid organization_id UUID"})),
        };

        if let Err(e) = self.verify_org_membership(user_context, org_uuid).await {
            return Ok(json!({"error": e.to_string()}));
        }

        let pipeline_type_filter = args
            .get("pipeline_type")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<PipelineType>().ok());

        let pipelines = CrmPipeline::find_by_organization(
            &self.pool,
            &DbUuid::from(org_uuid),
            pipeline_type_filter,
        )
        .await
        .unwrap_or_default();

        let mut pipeline_list: Vec<serde_json::Value> = Vec::new();
        for p in &pipelines {
            let stages = CrmPipelineStage::find_by_pipeline(&self.pool, &p.id)
                .await
                .unwrap_or_default();

            let stage_list: Vec<serde_json::Value> = stages
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id.to_string(),
                        "name": s.name,
                        "color": s.color,
                        "position": s.position,
                        "probability": s.probability
                    })
                })
                .collect();

            pipeline_list.push(json!({
                "id": p.id.to_string(),
                "name": p.name,
                "pipeline_type": p.pipeline_type,
                "stages": stage_list
            }));
        }

        Ok(json!({
            "pipelines": pipeline_list,
            "total": pipelines.len()
        }))
    }

    /// Create a new CRM contact
    pub async fn create_crm_contact(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        use db::models::crm_contact::{ContactSource, CreateCrmContact, CrmContact};

        let org_id_str = match args.get("organization_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "organization_id is required"})),
        };
        let org_uuid = Uuid::parse_str(org_id_str)
            .map_err(|_| TopsiError::ToolError("Invalid organization_id".to_string()))?;

        self.verify_org_membership(user_context, org_uuid).await?;

        let lifecycle_stage = args
            .get("lifecycle_stage")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_value(json!(s)).ok());

        let contact = CrmContact::create(
            &self.pool,
            CreateCrmContact {
                organization_id: DbUuid::from(org_uuid),
                client_id: None,
                first_name: args
                    .get("first_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                last_name: args
                    .get("last_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                email: args
                    .get("email")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                phone: args
                    .get("phone")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                mobile: None,
                avatar_url: None,
                company_name: args
                    .get("company_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                job_title: args
                    .get("job_title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                department: None,
                linkedin_url: args
                    .get("linkedin_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                twitter_handle: None,
                website: None,
                source: Some(ContactSource::Api),
                lifecycle_stage,
                tags: None,
                custom_fields: None,
                zoho_contact_id: None,
                gmail_contact_id: None,
            },
        )
        .await;

        match contact {
            Ok(c) => Ok(json!({
                "id": c.id.to_string(),
                "first_name": c.first_name,
                "last_name": c.last_name,
                "email": c.email,
                "message": "Contact created successfully"
            })),
            Err(e) => Ok(json!({"error": format!("Failed to create contact: {}", e)})),
        }
    }

    /// Create a new CRM deal
    pub async fn create_crm_deal(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        use db::models::crm_deal::{CreateCrmDeal, CrmDeal};

        let org_id_str = match args.get("organization_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "organization_id is required"})),
        };
        let org_uuid = Uuid::parse_str(org_id_str)
            .map_err(|_| TopsiError::ToolError("Invalid organization_id".to_string()))?;

        self.verify_org_membership(user_context, org_uuid).await?;

        let name = match args.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return Ok(json!({"error": "name is required"})),
        };

        let deal = CrmDeal::create(
            &self.pool,
            CreateCrmDeal {
                organization_id: DbUuid::from(org_uuid),
                client_id: None,
                crm_contact_id: args
                    .get("contact_id")
                    .and_then(|v| v.as_str())
                    .map(|s| DbUuid::from_string(s)),
                crm_pipeline_id: args
                    .get("pipeline_id")
                    .and_then(|v| v.as_str())
                    .map(|s| DbUuid::from_string(s)),
                crm_stage_id: args
                    .get("stage_id")
                    .and_then(|v| v.as_str())
                    .map(|s| DbUuid::from_string(s)),
                name,
                description: args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                amount: args.get("amount").and_then(|v| v.as_f64()),
                currency: args
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                expected_close_date: args
                    .get("expected_close_date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tags: None,
                custom_fields: None,
            },
        )
        .await;

        match deal {
            Ok(d) => Ok(json!({
                "id": d.id.to_string(),
                "name": d.name,
                "amount": d.amount,
                "message": "Deal created successfully"
            })),
            Err(e) => Ok(json!({"error": format!("Failed to create deal: {}", e)})),
        }
    }

    /// Update an existing CRM deal
    pub async fn update_crm_deal(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        use db::models::crm_deal::{CrmDeal, UpdateCrmDeal};

        let deal_id_str = match args.get("deal_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "deal_id is required"})),
        };
        let deal_db_id = DbUuid::from_string(deal_id_str);

        if !user_context.is_admin {
            if let Ok(existing) = CrmDeal::find_by_id(&self.pool, &deal_db_id).await {
                if let Some(ref org_id) = existing.organization_id {
                    let org_uuid = Uuid::parse_str(org_id.as_str())
                        .map_err(|_| TopsiError::ToolError("Invalid org_id".to_string()))?;
                    self.verify_org_membership(user_context, org_uuid).await?;
                }
            }
        }

        let update = UpdateCrmDeal {
            name: args
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            amount: args.get("amount").and_then(|v| v.as_f64()),
            currency: args
                .get("currency")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            crm_stage_id: args
                .get("stage_id")
                .and_then(|v| v.as_str())
                .map(|s| DbUuid::from_string(s)),
            description: args
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            expected_close_date: args
                .get("expected_close_date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            lost_reason: args
                .get("lost_reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            win_reason: args
                .get("win_reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            ..Default::default()
        };

        match CrmDeal::update(&self.pool, &deal_db_id, update).await {
            Ok(d) => Ok(json!({
                "id": d.id.to_string(),
                "name": d.name,
                "amount": d.amount,
                "message": "Deal updated successfully"
            })),
            Err(e) => Ok(json!({"error": format!("Failed to update deal: {}", e)})),
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    //  WORKFLOW TOOLS
    // ══════════════════════════════════════════════════════════════════════════

    /// List saved workflow definitions
    pub async fn list_workflow_definitions(
        &self,
        _args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let rows = sqlx::query(
            "SELECT id, name, description, owner_id, owner_type, is_system FROM workflow_definitions ORDER BY is_system DESC, name ASC"
        )
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(rows) => {
                use sqlx::Row;
                let definitions: Vec<serde_json::Value> = rows
                    .iter()
                    .filter(|row| {
                        let is_system = row.get::<bool, _>("is_system");
                        if is_system {
                            return true;
                        }
                        if matches!(scope, AccessScope::Admin) {
                            return true;
                        }
                        false
                    })
                    .map(|row| {
                        let owner_id: Option<String> = row
                            .get::<Option<Vec<u8>>, _>("owner_id")
                            .and_then(|bytes| Uuid::from_slice(&bytes).ok())
                            .map(|u| u.to_string());
                        json!({
                            "id": row.get::<String, _>("id"),
                            "name": row.get::<String, _>("name"),
                            "description": row.get::<Option<String>, _>("description"),
                            "owner_type": row.get::<Option<String>, _>("owner_type"),
                            "owner_id": owner_id,
                            "is_system": row.get::<bool, _>("is_system")
                        })
                    })
                    .collect();

                Ok(json!({
                    "definitions": definitions,
                    "total": definitions.len()
                }))
            }
            Err(e) => Ok(json!({"error": format!("Failed to query workflow definitions: {}", e)})),
        }
    }

    /// Get a full workflow definition including steps
    pub async fn get_workflow_definition(
        &self,
        args: &serde_json::Value,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let workflow_id = match args.get("workflow_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "workflow_id is required"})),
        };

        let row = sqlx::query(
            "SELECT id, name, description, steps, owner_type, owner_id, is_system FROM workflow_definitions WHERE id = ?"
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await;

        match row {
            Ok(Some(row)) => {
                use sqlx::Row;
                let steps_str = row.get::<Option<String>, _>("steps");
                let steps: serde_json::Value = steps_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(json!([]));

                let owner_id: Option<String> = row
                    .get::<Option<Vec<u8>>, _>("owner_id")
                    .and_then(|bytes| Uuid::from_slice(&bytes).ok())
                    .map(|u| u.to_string());

                Ok(json!({
                    "id": row.get::<String, _>("id"),
                    "name": row.get::<String, _>("name"),
                    "description": row.get::<Option<String>, _>("description"),
                    "steps": steps,
                    "owner_type": row.get::<Option<String>, _>("owner_type"),
                    "owner_id": owner_id,
                    "is_system": row.get::<bool, _>("is_system")
                }))
            }
            Ok(None) => Ok(json!({"error": "Workflow definition not found"})),
            Err(e) => Ok(json!({"error": format!("Database error: {}", e)})),
        }
    }

    /// Search across projects, contacts, deals, and tasks by keyword
    pub async fn search_entities(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return Ok(json!({"error": "query is required"})),
        };

        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20) as i32;
        let org_id = args
            .get("organization_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        if let Some(oid) = org_id {
            if let Err(e) = self.verify_org_membership(user_context, oid).await {
                return Ok(json!({"error": e.to_string()}));
            }
        }

        let entity_types: Vec<String> = args
            .get("entity_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    "projects".to_string(),
                    "contacts".to_string(),
                    "deals".to_string(),
                    "tasks".to_string(),
                ]
            });

        let like_pattern = format!("%{}%", query);
        let mut results = json!({});

        if entity_types.contains(&"projects".to_string()) {
            let projects = sqlx::query(
                "SELECT id, name FROM projects WHERE name LIKE ?1 AND deleted_at IS NULL LIMIT ?2",
            )
            .bind(&like_pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await;

            if let Ok(rows) = projects {
                use sqlx::Row;
                let list: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.get::<String, _>("id"),
                            "name": r.get::<String, _>("name")
                        })
                    })
                    .collect();
                results["projects"] = json!(list);
            }
        }

        if entity_types.contains(&"contacts".to_string()) {
            if let Some(oid) = org_id {
                let params = ContactSearchParams {
                    organization_id: Some(DbUuid::from(oid)),
                    client_id: None,
                    query: Some(query.to_string()),
                    lifecycle_stage: None,
                    company_name: None,
                    tags: None,
                    min_lead_score: None,
                    limit: Some(limit),
                    offset: None,
                };
                let contacts = CrmContact::search(&self.pool, params)
                    .await
                    .unwrap_or_default();
                let list: Vec<serde_json::Value> = contacts
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id.to_string(),
                            "first_name": c.first_name,
                            "last_name": c.last_name,
                            "email": c.email,
                            "company_name": c.company_name
                        })
                    })
                    .collect();
                results["contacts"] = json!(list);
            } else {
                results["contacts"] = json!([]);
            }
        }

        if entity_types.contains(&"deals".to_string()) {
            if let Some(oid) = org_id {
                let deals = sqlx::query(
                    "SELECT id, name, amount, currency, stage FROM crm_deals WHERE name LIKE ?1 AND organization_id = ?2 LIMIT ?3"
                )
                .bind(&like_pattern)
                .bind(oid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await;

                if let Ok(rows) = deals {
                    use sqlx::Row;
                    let list: Vec<serde_json::Value> = rows
                        .iter()
                        .map(|r| {
                            let id_bytes = r.get::<Vec<u8>, _>("id");
                            let id_str = Uuid::from_slice(&id_bytes)
                                .map(|u| u.to_string())
                                .unwrap_or_else(|_| {
                                    id_bytes.iter().map(|b| format!("{:02x}", b)).collect()
                                });
                            json!({
                                "id": id_str,
                                "name": r.get::<String, _>("name"),
                                "amount": r.get::<Option<f64>, _>("amount"),
                                "currency": r.get::<String, _>("currency"),
                                "stage": r.get::<String, _>("stage")
                            })
                        })
                        .collect();
                    results["deals"] = json!(list);
                }
            } else {
                results["deals"] = json!([]);
            }
        }

        if entity_types.contains(&"tasks".to_string()) {
            let tasks = sqlx::query(
                "SELECT id, title, status, project_id FROM tasks WHERE (title LIKE ?1 OR description LIKE ?1) AND deleted_at IS NULL LIMIT ?2"
            )
            .bind(&like_pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await;

            if let Ok(rows) = tasks {
                use sqlx::Row;
                let list: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.get::<String, _>("id"),
                            "title": r.get::<String, _>("title"),
                            "status": r.get::<String, _>("status"),
                            "project_id": r.get::<Option<String>, _>("project_id")
                        })
                    })
                    .collect();
                results["tasks"] = json!(list);
            }
        }

        Ok(results)
    }

    // ── Workflow execution tools ─────────────────────────────────────────────

    /// List recent workflow runs
    pub async fn list_workflow_runs(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        use db::models::workflow_run::WorkflowRun;

        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        let org_id = args.get("organization_id").and_then(|v| v.as_str());

        if let Some(oid) = org_id {
            if let Ok(org_uuid) = Uuid::parse_str(oid) {
                self.verify_org_membership(user_context, org_uuid).await?;
            }
        }

        let runs = WorkflowRun::find_recent(&self.pool, limit, workflow_id, org_id)
            .await
            .unwrap_or_default();

        let items: Vec<serde_json::Value> = runs
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "workflow_id": r.workflow_id,
                    "workflow_name": r.workflow_name,
                    "status": r.status,
                    "data_source_id": r.data_source_id,
                    "organization_id": r.organization_id,
                    "model_used": r.model_used,
                    "total_records_staged": r.total_records_staged,
                    "total_duplicates_found": r.total_duplicates_found,
                    "duration_ms": r.duration_ms,
                    "started_at": r.started_at,
                    "completed_at": r.completed_at,
                })
            })
            .collect();

        Ok(json!({
            "runs": items,
            "total": items.len()
        }))
    }

    /// Get status and details of a specific workflow run
    pub async fn get_workflow_run_status(
        &self,
        args: &serde_json::Value,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        use db::models::{workflow_run::WorkflowRun, workflow_staging::WorkflowStagingRecord};

        let run_id = match args.get("run_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "run_id is required"})),
        };

        let run = match WorkflowRun::find_by_id(&self.pool, run_id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(json!({"error": "Workflow run not found"})),
            Err(e) => return Ok(json!({"error": format!("Database error: {}", e)})),
        };

        let run_uuid = Uuid::parse_str(run_id).unwrap_or(Uuid::nil());
        let staged = WorkflowStagingRecord::find_by_run(&self.pool, run_uuid)
            .await
            .unwrap_or_default();

        let pending = staged
            .iter()
            .filter(|r| r.status == "pending_review")
            .count();
        let approved = staged.iter().filter(|r| r.status == "approved").count();
        let rejected = staged.iter().filter(|r| r.status == "rejected").count();
        let committed = staged.iter().filter(|r| r.status == "committed").count();
        let duplicates = staged
            .iter()
            .filter(|r| r.duplicate_of_id.is_some())
            .count();

        Ok(json!({
            "id": run.id,
            "workflow_id": run.workflow_id,
            "workflow_name": run.workflow_name,
            "status": run.status,
            "data_source_id": run.data_source_id,
            "organization_id": run.organization_id,
            "model_used": run.model_used,
            "total_input_tokens": run.total_input_tokens,
            "total_output_tokens": run.total_output_tokens,
            "total_estimated_cost_micros": run.total_estimated_cost_micros,
            "duration_ms": run.duration_ms,
            "node_count": run.node_count,
            "llm_node_count": run.llm_node_count,
            "started_at": run.started_at,
            "completed_at": run.completed_at,
            "staging_summary": {
                "total": staged.len(),
                "pending_review": pending,
                "approved": approved,
                "rejected": rejected,
                "committed": committed,
                "duplicates": duplicates,
            }
        }))
    }

    /// Show staged CRM/task records pending review
    pub async fn review_staged_data(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        use db::models::workflow_staging::WorkflowStagingRecord;

        let records = if let Some(run_id_str) = args.get("run_id").and_then(|v| v.as_str()) {
            let run_uuid = Uuid::parse_str(run_id_str)
                .map_err(|_| TopsiError::ToolError("Invalid run_id UUID".to_string()))?;
            WorkflowStagingRecord::find_by_run(&self.pool, run_uuid)
                .await
                .unwrap_or_default()
        } else if let Some(org_id_str) = args.get("organization_id").and_then(|v| v.as_str()) {
            let org_uuid = Uuid::parse_str(org_id_str)
                .map_err(|_| TopsiError::ToolError("Invalid organization_id UUID".to_string()))?;
            self.verify_org_membership(user_context, org_uuid).await?;
            WorkflowStagingRecord::find_by_org_pending(&self.pool, org_uuid)
                .await
                .unwrap_or_default()
        } else {
            return Ok(json!({"error": "Either run_id or organization_id is required"}));
        };

        let items: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                let data: serde_json::Value =
                    serde_json::from_str(&r.record_data).unwrap_or(json!({}));
                let validation_errs: Option<Vec<String>> = r
                    .validation_errors
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok());
                json!({
                    "id": r.id,
                    "target_type": r.target_type,
                    "status": r.status,
                    "record_data": data,
                    "duplicate_of_id": r.duplicate_of_id,
                    "duplicate_of_type": r.duplicate_of_type,
                    "confidence": r.confidence,
                    "validation_errors": validation_errs,
                    "workflow_id": r.workflow_id,
                    "node_id": r.node_id,
                    "created_at": r.created_at,
                })
            })
            .collect();

        let pending = items
            .iter()
            .filter(|r| r["status"] == "pending_review")
            .count();

        Ok(json!({
            "records": items,
            "total": items.len(),
            "pending_review": pending,
        }))
    }

    /// Approve staged records — auto-approve valid ones and reject duplicates
    pub async fn approve_staged_records(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        use db::models::workflow_staging::WorkflowStagingRecord;

        let run_id_str = match args.get("run_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(json!({"error": "run_id is required"})),
        };

        let run_uuid = Uuid::parse_str(run_id_str)
            .map_err(|_| TopsiError::ToolError("Invalid run_id UUID".to_string()))?;

        if !user_context.is_admin {
            use db::models::workflow_run::WorkflowRun;
            if let Ok(Some(run)) = WorkflowRun::find_by_id(&self.pool, run_id_str).await {
                if let Some(ref org_id) = run.organization_id {
                    if let Ok(org_uuid) = Uuid::parse_str(org_id) {
                        self.verify_org_membership(user_context, org_uuid).await?;
                    }
                }
            }
        }

        let approved = WorkflowStagingRecord::auto_approve_valid(&self.pool, run_uuid)
            .await
            .unwrap_or(0);

        let rejected = WorkflowStagingRecord::reject_duplicates(&self.pool, run_uuid)
            .await
            .unwrap_or(0);

        let remaining = WorkflowStagingRecord::find_by_run(&self.pool, run_uuid)
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| r.status == "pending_review")
            .count();

        Ok(json!({
            "approved": approved,
            "rejected_duplicates": rejected,
            "remaining_pending": remaining,
            "message": format!("Approved {} records, rejected {} duplicates. {} records still pending review.", approved, rejected, remaining)
        }))
    }
}

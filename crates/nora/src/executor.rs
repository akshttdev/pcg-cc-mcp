//! Task execution engine for Nora
//! Handles autonomous task creation and management across projects

use db::models::{
    project::{CreateProject, Project},
    project_board::ProjectBoard,
    project_pod::{CreateProjectPod, ProjectPod},
    task::{CreateTask, Priority, Task},
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{NoraError, Result};

/// Task executor for creating and managing tasks
pub struct TaskExecutor {
    pool: SqlitePool,
}

const DEFAULT_POD_TEMPLATES: &[(&str, &str)] = &[
    (
        "Research Pod",
        "Capture requirements, scope, and primary research threads",
    ),
    (
        "Build Pod",
        "Own rapid prototyping, code drops, and automation",
    ),
    (
        "Launch Pod",
        "Package updates, briefs, and delivery assets",
    ),
];

impl TaskExecutor {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new task in the database
    pub async fn create_task(
        &self,
        project_id: Uuid,
        definition: TaskDefinition,
    ) -> Result<Task> {
        tracing::info!(
            "Nora creating task '{}' in project {}",
            definition.title,
            project_id
        );

        let task_id = Uuid::new_v4();

        let create_task = CreateTask {
            project_id,
            pod_id: definition.pod_id,
            board_id: definition.board_id,
            title: definition.title,
            description: definition.description,
            parent_task_attempt: None,
            image_ids: None,
            priority: definition.priority,
            assignee_id: definition.assignee_id,
            assigned_agent: Some("nora".to_string()),
            assigned_mcps: None,
            created_by: "nora".to_string(),
            requires_approval: Some(false),
            parent_task_id: None,
            tags: definition.tags,
            due_date: None,
            custom_properties: None,
            scheduled_start: None,
            scheduled_end: None,
        };

        let task = Task::create(&self.pool, &create_task, task_id)
            .await
            .map_err(NoraError::DatabaseError)?;

        tracing::info!("Task created successfully: {} ({})", task.title, task.id);

        Ok(task)
    }

    /// Create multiple tasks as a batch
    pub async fn create_tasks_batch(
        &self,
        project_id: Uuid,
        tasks: Vec<TaskDefinition>,
    ) -> Result<Vec<Task>> {
        let mut created_tasks = Vec::new();

        for task_def in tasks {
            let task = self.create_task(project_id, task_def).await?;
            created_tasks.push(task);
        }

        tracing::info!(
            "Batch created {} tasks for project {}",
            created_tasks.len(),
            project_id
        );

        Ok(created_tasks)
    }

    /// Find project by name or return error with suggestions
    pub async fn find_project_by_name(&self, name: &str) -> Result<Uuid> {
        // Query the database for project by name
        let pattern = format!("%{}%", name);
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM projects WHERE LOWER(name) LIKE LOWER(?) LIMIT 1"
        )
        .bind(&pattern)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        match result {
            Some((uuid_str,)) => {
                Uuid::parse_str(&uuid_str).map_err(|e| {
                    NoraError::ConfigError(format!("Invalid project ID format: {}", e))
                })
            }
            None => Err(NoraError::ConfigError(format!(
                "Project '{}' not found. Please check the project name.",
                name
            ))),
        }
    }

    /// Get all projects for context
    pub async fn get_all_projects(&self) -> Result<Vec<ProjectInfo>> {
        let projects: Vec<ProjectInfo> = sqlx::query_as(
            "SELECT id, name FROM projects ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        Ok(projects)
    }

    pub async fn find_project_record_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Project>> {
        Project::find_by_name_case_insensitive(&self.pool, name)
            .await
            .map_err(NoraError::DatabaseError)
    }

    pub async fn create_project_entry(&self, payload: CreateProject) -> Result<Project> {
        let project_id = Uuid::new_v4();
        Project::create(&self.pool, &payload, project_id)
            .await
            .map_err(NoraError::DatabaseError)
    }

    pub async fn ensure_default_boards(&self, project_id: Uuid) -> Result<Vec<ProjectBoard>> {
        ProjectBoard::ensure_default_boards(&self.pool, project_id)
            .await
            .map_err(NoraError::DatabaseError)
    }

    pub async fn find_board_by_slug(
        &self,
        project_id: Uuid,
        slug: &str,
    ) -> Result<Option<ProjectBoard>> {
        ProjectBoard::find_by_slug(&self.pool, project_id, slug)
            .await
            .map_err(NoraError::DatabaseError)
    }

    pub async fn get_default_board_for_tasks(
        &self,
        project_id: Uuid,
    ) -> Result<Option<ProjectBoard>> {
        if let Some(board) = self.find_board_by_slug(project_id, "dev-assets").await? {
            return Ok(Some(board));
        }

        let mut boards = ProjectBoard::list_by_project(&self.pool, project_id)
            .await
            .map_err(NoraError::DatabaseError)?;

        Ok(boards.pop())
    }

    pub async fn seed_default_pods(&self, project_id: Uuid) -> Result<Vec<ProjectPod>> {
        let mut pods = Vec::new();
        for (title, description) in DEFAULT_POD_TEMPLATES {
            let pod = ProjectPod::create(
                &self.pool,
                Uuid::new_v4(),
                &CreateProjectPod {
                    project_id,
                    title: title.to_string(),
                    description: Some(description.to_string()),
                    status: Some("active".to_string()),
                    lead: None,
                },
            )
            .await
            .map_err(NoraError::DatabaseError)?;
            pods.push(pod);
        }

        Ok(pods)
    }

    /// Get detailed project information including tasks, boards, and pods
    pub async fn get_project_details(&self, project_id: Uuid) -> Result<ProjectDetails> {
        // Get project basic info
        let project: (String, String, String) = sqlx::query_as(
            "SELECT id, name, git_repo_path FROM projects WHERE id = ?"
        )
        .bind(project_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        // Get tasks for this project
        let tasks: Vec<TaskInfo> = sqlx::query_as(
            "SELECT id, title, description, status, priority, assignee_id, created_at, updated_at
             FROM tasks WHERE project_id = ? ORDER BY created_at DESC"
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        // Get boards for this project
        let boards: Vec<BoardInfo> = sqlx::query_as(
            "SELECT id, name, description FROM boards WHERE project_id = ? ORDER BY name"
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Get pods for this project
        let pods: Vec<PodInfo> = sqlx::query_as(
            "SELECT id, name, description FROM pods WHERE project_id = ? ORDER BY name"
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        Ok(ProjectDetails {
            id: project.0,
            name: project.1,
            git_repo_path: project.2,
            tasks,
            boards,
            pods,
        })
    }

    /// Get tasks by project name
    pub async fn get_tasks_by_project_name(&self, project_name: &str) -> Result<Vec<TaskInfo>> {
        let project_id = self.find_project_by_name(project_name).await?;

        let tasks: Vec<TaskInfo> = sqlx::query_as(
            "SELECT id, title, description, status, priority, assignee_id, created_at, updated_at
             FROM tasks WHERE project_id = ? ORDER BY created_at DESC"
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        Ok(tasks)
    }

    /// Get tasks by status
    pub async fn get_tasks_by_status(&self, project_id: Uuid, status: &str) -> Result<Vec<TaskInfo>> {
        let tasks: Vec<TaskInfo> = sqlx::query_as(
            "SELECT id, title, description, status, priority, assignee_id, created_at, updated_at
             FROM tasks WHERE project_id = ? AND status = ? ORDER BY created_at DESC"
        )
        .bind(project_id.to_string())
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        Ok(tasks)
    }

    /// Search tasks by keyword
    pub async fn search_tasks(&self, project_id: Uuid, keyword: &str) -> Result<Vec<TaskInfo>> {
        let pattern = format!("%{}%", keyword);
        let tasks: Vec<TaskInfo> = sqlx::query_as(
            "SELECT id, title, description, status, priority, assignee_id, created_at, updated_at
             FROM tasks
             WHERE project_id = ? AND (title LIKE ? OR description LIKE ?)
             ORDER BY created_at DESC"
        )
        .bind(project_id.to_string())
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        Ok(tasks)
    }

    /// Get all tasks across all projects
    pub async fn get_all_tasks(&self) -> Result<Vec<TaskInfo>> {
        let tasks: Vec<TaskInfo> = sqlx::query_as(
            "SELECT HEX(id) as id, title, description, status, priority, assignee_id, created_at, updated_at
             FROM tasks ORDER BY created_at DESC LIMIT 100"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        Ok(tasks)
    }

    /// Update task status
    pub async fn update_task_status(&self, task_id: Uuid, status: &str) -> Result<()> {
        sqlx::query(
            "UPDATE tasks SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(status)
        .bind(task_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        tracing::info!("Updated task {} status to {}", task_id, status);
        Ok(())
    }

    /// Get project statistics
    pub async fn get_project_stats(&self, project_id: Uuid) -> Result<ProjectStats> {
        let total_tasks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?"
        )
        .bind(project_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| NoraError::DatabaseError(e))?;

        let completed_tasks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'completed'"
        )
        .bind(project_id.to_string())
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let in_progress_tasks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'in_progress'"
        )
        .bind(project_id.to_string())
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let blocked_tasks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'blocked'"
        )
        .bind(project_id.to_string())
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(ProjectStats {
            total_tasks: total_tasks as usize,
            completed_tasks: completed_tasks as usize,
            in_progress_tasks: in_progress_tasks as usize,
            blocked_tasks: blocked_tasks as usize,
        })
    }
}

/// Project statistics
#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub in_progress_tasks: usize,
    pub blocked_tasks: usize,
}

/// Task definition for creation
#[derive(Debug, Clone)]
pub struct TaskDefinition {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Vec<String>>,
    pub assignee_id: Option<String>,
    pub board_id: Option<Uuid>,
    pub pod_id: Option<Uuid>,
}

/// Project information
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
}

/// Detailed project information with tasks
#[derive(Debug, Clone)]
pub struct ProjectDetails {
    pub id: String,
    pub name: String,
    pub git_repo_path: String,
    pub tasks: Vec<TaskInfo>,
    pub boards: Vec<BoardInfo>,
    pub pods: Vec<PodInfo>,
}

/// Task information
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: Option<String>,
    pub assignee_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Board information
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BoardInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Pod information
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PodInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

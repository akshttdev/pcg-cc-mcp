use std::{future::Future, path::PathBuf};

use db::models::{
    comment::{AuthorType, CommentType, CreateTaskComment, TaskComment},
    project::Project,
    task::{CreateTask, Task, TaskStatus},
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json;
use services::services::pcg_policy::{self, PolicyAction, PolicyCheckContext};
use sqlx::{SqlitePool, types::Json as SqlxJson};
use uuid::Uuid;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskRequest {
    #[schemars(description = "The ID of the project to create the task in. This is required!")]
    pub project_id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateTaskResponse {
    pub success: bool,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectSummary {
    #[schemars(description = "The unique identifier of the project")]
    pub id: String,
    #[schemars(description = "The name of the project")]
    pub name: String,
    #[schemars(description = "The path to the git repository")]
    pub git_repo_path: PathBuf,
    #[schemars(description = "Optional setup script for the project")]
    pub setup_script: Option<String>,
    #[schemars(description = "Optional cleanup script for the project")]
    pub cleanup_script: Option<String>,
    #[schemars(description = "Optional development script for the project")]
    pub dev_script: Option<String>,
    #[schemars(description = "When the project was created")]
    pub created_at: String,
    #[schemars(description = "When the project was last updated")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListProjectsResponse {
    pub success: bool,
    pub projects: Vec<ProjectSummary>,
    pub count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTasksRequest {
    #[schemars(description = "The ID of the project to list tasks from")]
    pub project_id: String,
    #[schemars(
        description = "Optional status filter: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'"
    )]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of tasks to return (default: 50)")]
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskSummary {
    #[schemars(description = "The unique identifier of the task")]
    pub id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
    #[schemars(description = "Current status of the task")]
    pub status: String,
    #[schemars(description = "When the task was created")]
    pub created_at: String,
    #[schemars(description = "When the task was last updated")]
    pub updated_at: String,
    #[schemars(description = "Whether the task has an in-progress execution attempt")]
    pub has_in_progress_attempt: Option<bool>,
    #[schemars(description = "Whether the task has a merged execution attempt")]
    pub has_merged_attempt: Option<bool>,
    #[schemars(description = "Whether the last execution attempt failed")]
    pub last_attempt_failed: Option<bool>,
    #[schemars(description = "Task priority: critical, high, medium, low")]
    pub priority: Option<String>,
    #[schemars(description = "Assigned user ID (if any)")]
    pub assignee_id: Option<String>,
    #[schemars(description = "Due date (ISO 8601, if set)")]
    pub due_date: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksResponse {
    pub success: bool,
    pub tasks: Vec<TaskSummary>,
    pub count: usize,
    pub project_id: String,
    pub project_name: Option<String>,
    pub applied_filters: ListTasksFilters,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksFilters {
    pub status: Option<String>,
    pub limit: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EvaluatePolicyRequest {
    #[schemars(description = "Project UUID that the task belongs to")]
    pub project_id: String,
    #[schemars(description = "Task UUID under evaluation")]
    pub task_id: String,
    #[schemars(description = "Optional severity (low|medium|high)")]
    pub severity: Option<String>,
    #[schemars(description = "Context tags (e.g. crisis, compliance)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Actor identifier (agent or user)")]
    pub actor: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EvaluatePolicyResponse {
    pub allow: bool,
    pub actions: Vec<String>,
    pub notes: Option<String>,
}

fn parse_task_status(status_str: &str) -> Option<TaskStatus> {
    match status_str.to_lowercase().as_str() {
        "todo" => Some(TaskStatus::Todo),
        "inprogress" | "in-progress" | "in_progress" => Some(TaskStatus::InProgress),
        "inreview" | "in-review" | "in_review" => Some(TaskStatus::InReview),
        "done" | "completed" => Some(TaskStatus::Done),
        "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn task_status_to_string(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Todo => "todo".to_string(),
        TaskStatus::InProgress => "in-progress".to_string(),
        TaskStatus::InReview => "in-review".to_string(),
        TaskStatus::Done => "done".to_string(),
        TaskStatus::Cancelled => "cancelled".to_string(),
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to update")]
    pub task_id: String,
    #[schemars(description = "New title for the task")]
    pub title: Option<String>,
    #[schemars(description = "New description for the task")]
    pub description: Option<String>,
    #[schemars(description = "New status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct UpdateTaskResponse {
    pub success: bool,
    pub message: String,
    pub task: Option<TaskSummary>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to delete")]
    pub task_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DeleteTaskResponse {
    pub success: bool,
    pub message: String,
    pub deleted_task_id: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SimpleTaskResponse {
    pub success: bool,
    pub message: String,
    pub task_title: String,
    pub new_status: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to retrieve")]
    pub task_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskResponse {
    pub success: bool,
    pub task: Option<TaskSummary>,
    pub project_name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssignTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to assign")]
    pub task_id: String,
    #[schemars(description = "The username or UUID of the user to assign the task to. Use 'unassign' to clear.")]
    pub assignee: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddCommentRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to comment on")]
    pub task_id: String,
    #[schemars(description = "The comment text content")]
    pub content: String,
    #[schemars(description = "Optional: 'comment', 'status_update', 'review', 'system', 'handoff'. Default: 'comment'")]
    pub comment_type: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AddCommentResponse {
    pub success: bool,
    pub comment_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListProjectMembersRequest {
    #[schemars(description = "The ID of the project to list members for")]
    pub project_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectMemberSummary {
    pub user_id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListProjectMembersResponse {
    pub success: bool,
    pub members: Vec<ProjectMemberSummary>,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct TaskServer {
    pub pool: SqlitePool,
    /// User ID for scoping operations (None = admin/unscoped for backward compat)
    pub user_id: Option<Uuid>,
    /// Whether this user is admin (bypasses project membership checks)
    pub is_admin: bool,
    tool_router: ToolRouter<TaskServer>,
}

impl TaskServer {
    #[allow(dead_code)]
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            user_id: None,
            is_admin: true, // Default: backward-compatible admin access
            tool_router: Self::tool_router(),
        }
    }

    /// Create a user-scoped MCP task server
    pub fn new_for_user(pool: SqlitePool, user_id: Uuid, is_admin: bool) -> Self {
        Self {
            pool,
            user_id: Some(user_id),
            is_admin,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl TaskServer {
    #[tool(
        description = "Create a new task/ticket in a project. Always pass the `project_id` of the project you want to create the task in - it is required!"
    )]
    async fn create_task(
        &self,
        Parameters(CreateTaskRequest {
            project_id,
            title,
            description,
        }): Parameters<CreateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Parse project_id from string to UUID
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format. Must be a valid UUID.",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Invalid project ID format".to_string()),
                )]));
            }
        };

        // Check if project exists
        match Project::exists(&self.pool, project_uuid).await {
            Ok(false) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Project not found",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Project not found".to_string()),
                )]));
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to check project existence",
                    "details": e.to_string(),
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]));
            }
            Ok(true) => {}
        }

        let task_id = Uuid::new_v4();
        let create_task_data = CreateTask {
            project_id: project_uuid,
            pod_id: None,
            board_id: None,
            title: title.clone(),
            description: description.clone(),
            parent_task_attempt: None,
            image_ids: None,
            priority: None,
            assignee_id: None,
            assigned_agent: None,
            agent_id: None,
            assigned_mcps: None,
            created_by: self.user_id.map(|id| id.to_string()).unwrap_or_else(|| "mcp".to_string()),
            requires_approval: None,
            parent_task_id: None,
            tags: None,
            due_date: None,
            custom_properties: None,
            scheduled_start: None,
            scheduled_end: None,
        };

        match Task::create(&self.pool, &create_task_data, task_id).await {
            Ok(_task) => {
                let success_response = CreateTaskResponse {
                    success: true,
                    task_id: task_id.to_string(),
                    message: "Task created successfully".to_string(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&success_response)
                        .unwrap_or_else(|_| "Task created successfully".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to create task",
                    "details": e.to_string(),
                    "project_id": project_id,
                    "title": title
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Failed to create task".to_string()),
                )]))
            }
        }
    }

    #[tool(description = "List all projects accessible to the current user")]
    async fn list_projects(&self) -> Result<CallToolResult, ErrorData> {
        // Scope to user's projects unless admin
        let projects_result = if self.is_admin || self.user_id.is_none() {
            Project::find_all(&self.pool).await
        } else {
            // Fetch only projects the user has membership in
            let user_id = self.user_id.unwrap();
            let user_id_bytes = user_id.as_bytes().to_vec();

            #[derive(sqlx::FromRow)]
            struct ProjId { project_id: String }

            let project_ids: Vec<String> = match sqlx::query_as::<_, ProjId>(
                "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?"
            )
            .bind(&user_id_bytes)
            .fetch_all(&self.pool)
            .await {
                Ok(rows) => rows.into_iter().map(|r| r.project_id).collect(),
                Err(e) => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Failed to retrieve user projects",
                        "details": e.to_string()
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response)
                            .unwrap_or_else(|_| "Database error".to_string()),
                    )]));
                }
            };

            let mut projects = Vec::new();
            for pid_str in project_ids {
                if let Ok(pid) = Uuid::parse_str(&pid_str) {
                    if let Ok(Some(project)) = Project::find_by_id(&self.pool, pid).await {
                        projects.push(project);
                    }
                }
            }
            Ok(projects)
        };

        match projects_result {
            Ok(projects) => {
                let count = projects.len();
                let project_summaries: Vec<ProjectSummary> = projects
                    .into_iter()
                    .map(|project| ProjectSummary {
                        id: project.id.to_string(),
                        name: project.name,
                        git_repo_path: project.git_repo_path,
                        setup_script: project.setup_script,
                        cleanup_script: project.cleanup_script,
                        dev_script: project.dev_script,
                        created_at: project.created_at.to_rfc3339(),
                        updated_at: project.updated_at.to_rfc3339(),
                    })
                    .collect();

                let response = ListProjectsResponse {
                    success: true,
                    projects: project_summaries,
                    count,
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "Failed to serialize projects".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to retrieve projects",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "List all the task/tickets in a project with optional filtering and execution status. `project_id` is required!"
    )]
    async fn list_tasks(
        &self,
        Parameters(ListTasksRequest {
            project_id,
            status,
            limit,
        }): Parameters<ListTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format. Must be a valid UUID.",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Invalid project ID format".to_string()),
                )]));
            }
        };

        let status_filter = if let Some(ref status_str) = status {
            match parse_task_status(status_str) {
                Some(status) => Some(status),
                None => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Invalid status filter. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'",
                        "provided_status": status_str
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response)
                            .unwrap_or_else(|_| "Invalid status filter".to_string()),
                    )]));
                }
            }
        } else {
            None
        };

        let project = match Project::find_by_id(&self.pool, project_uuid).await {
            Ok(Some(project)) => project,
            Ok(None) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Project not found",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Project not found".to_string()),
                )]));
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to check project existence",
                    "details": e.to_string(),
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]));
            }
        };

        let task_limit = limit.unwrap_or(50).clamp(1, 200); // Reasonable limits

        let tasks_result =
            Task::find_by_project_id_with_attempt_status(&self.pool, project_uuid).await;

        match tasks_result {
            Ok(tasks) => {
                let filtered_tasks: Vec<_> = tasks
                    .into_iter()
                    .filter(|task| {
                        if let Some(ref filter_status) = status_filter {
                            &task.status == filter_status
                        } else {
                            true
                        }
                    })
                    .take(task_limit as usize)
                    .collect();

                let task_summaries: Vec<TaskSummary> = filtered_tasks
                    .into_iter()
                    .map(|task| TaskSummary {
                        id: task.id.to_string(),
                        title: task.title.clone(),
                        description: task.description.clone(),
                        status: task_status_to_string(&task.status),
                        created_at: task.created_at.to_rfc3339(),
                        updated_at: task.updated_at.to_rfc3339(),
                        has_in_progress_attempt: Some(task.has_in_progress_attempt),
                        has_merged_attempt: Some(task.has_merged_attempt),
                        last_attempt_failed: Some(task.last_attempt_failed),
                        priority: Some(format!("{:?}", task.priority).to_lowercase()),
                        assignee_id: task.assignee_id.clone(),
                        due_date: task.due_date.map(|d| d.to_rfc3339()),
                    })
                    .collect();

                let count = task_summaries.len();
                let response = ListTasksResponse {
                    success: true,
                    tasks: task_summaries,
                    count,
                    project_id: project_id.clone(),
                    project_name: Some(project.name),
                    applied_filters: ListTasksFilters {
                        status: status.clone(),
                        limit: task_limit,
                    },
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "Failed to serialize tasks".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to retrieve tasks",
                    "details": e.to_string(),
                    "project_id": project_id
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Update an existing task/ticket's title, description, or status. `project_id` and `task_id` are required! `title`, `description`, and `status` are optional."
    )]
    async fn update_task(
        &self,
        Parameters(UpdateTaskRequest {
            project_id,
            task_id,
            title,
            description,
            status,
        }): Parameters<UpdateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format. Must be a valid UUID.",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid task ID format. Must be a valid UUID.",
                    "task_id": task_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let status_enum = if let Some(ref status_str) = status {
            match parse_task_status(status_str) {
                Some(status) => Some(status),
                None => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Invalid status. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'",
                        "provided_status": status_str
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response).unwrap(),
                    )]));
                }
            }
        } else {
            None
        };

        let current_task =
            match Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Task not found in the specified project",
                        "task_id": task_id,
                        "project_id": project_id
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response).unwrap(),
                    )]));
                }
                Err(e) => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Failed to retrieve task",
                        "details": e.to_string()
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response).unwrap(),
                    )]));
                }
            };

        let new_title = title.unwrap_or(current_task.title.clone());
        let new_description = description.or(current_task.description.clone());
        let new_status = status_enum.unwrap_or(current_task.status.clone());
        let new_parent_task_attempt = current_task.parent_task_attempt;

        let custom_properties = current_task
            .custom_properties
            .as_ref()
            .map(|json| json.0.clone())
            .map(SqlxJson);
        match Task::update(
            &self.pool,
            task_uuid,
            project_uuid,
            new_title,
            new_description,
            new_status,
            new_parent_task_attempt,
            current_task.pod_id,
            current_task.board_id,
            current_task.priority,
            current_task.assignee_id,
            current_task.assigned_agent,
            current_task.assigned_mcps,
            current_task.requires_approval,
            current_task.approval_status,
            current_task.parent_task_id,
            current_task.tags,
            current_task.due_date,
            custom_properties,
            current_task.scheduled_start,
            current_task.scheduled_end,
        )
        .await
        {
            Ok(updated_task) => {
                let task_summary = TaskSummary {
                    id: updated_task.id.to_string(),
                    title: updated_task.title,
                    description: updated_task.description,
                    status: task_status_to_string(&updated_task.status),
                    created_at: updated_task.created_at.to_rfc3339(),
                    updated_at: updated_task.updated_at.to_rfc3339(),
                    has_in_progress_attempt: None,
                    has_merged_attempt: None,
                    last_attempt_failed: None,
                    priority: Some(format!("{:?}", updated_task.priority).to_lowercase()),
                    assignee_id: updated_task.assignee_id.clone(),
                    due_date: updated_task.due_date.map(|d| d.to_rfc3339()),
                };

                let response = UpdateTaskResponse {
                    success: true,
                    message: "Task updated successfully".to_string(),
                    task: Some(task_summary),
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to update task",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
        }
    }

    #[tool(
        description = "Delete a task/ticket from a project. `project_id` and `task_id` are required!"
    )]
    async fn delete_task(
        &self,
        Parameters(DeleteTaskRequest {
            project_id,
            task_id,
        }): Parameters<DeleteTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid task ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        match Task::exists(&self.pool, task_uuid, project_uuid).await {
            Ok(true) => {
                // Delete the task
                match Task::delete(&self.pool, task_uuid).await {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            let response = DeleteTaskResponse {
                                success: true,
                                message: "Task deleted successfully".to_string(),
                                deleted_task_id: Some(task_id),
                            };
                            Ok(CallToolResult::success(vec![Content::text(
                                serde_json::to_string_pretty(&response).unwrap(),
                            )]))
                        } else {
                            let error_response = serde_json::json!({
                                "success": false,
                                "error": "Task not found or already deleted"
                            });
                            Ok(CallToolResult::error(vec![Content::text(
                                serde_json::to_string_pretty(&error_response).unwrap(),
                            )]))
                        }
                    }
                    Err(e) => {
                        let error_response = serde_json::json!({
                            "success": false,
                            "error": "Failed to delete task",
                            "details": e.to_string()
                        });
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&error_response).unwrap(),
                        )]))
                    }
                }
            }
            Ok(false) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Task not found in the specified project"
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to check task existence",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
        }
    }

    #[tool(
        description = "Get detailed information about a specific task/ticket. `project_id` and `task_id` are required!"
    )]
    async fn get_task(
        &self,
        Parameters(GetTaskRequest {
            project_id,
            task_id,
        }): Parameters<GetTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid task ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_result =
            Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await;
        let project_result = Project::find_by_id(&self.pool, project_uuid).await;

        match (task_result, project_result) {
            (Ok(Some(task)), Ok(Some(project))) => {
                let task_summary = TaskSummary {
                    id: task.id.to_string(),
                    title: task.title.clone(),
                    description: task.description,
                    status: task_status_to_string(&task.status),
                    created_at: task.created_at.to_rfc3339(),
                    updated_at: task.updated_at.to_rfc3339(),
                    has_in_progress_attempt: None,
                    has_merged_attempt: None,
                    last_attempt_failed: None,
                    priority: Some(format!("{:?}", task.priority).to_lowercase()),
                    assignee_id: task.assignee_id.clone(),
                    due_date: task.due_date.map(|d| d.to_rfc3339()),
                };

                let response = GetTaskResponse {
                    success: true,
                    task: Some(task_summary),
                    project_name: Some(project.name),
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            (Ok(None), _) | (_, Ok(None)) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Task or project not found"
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
            (Err(e), _) | (_, Err(e)) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to retrieve task or project",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
        }
    }

    #[tool(
        description = "Assign a task to a team member. Pass 'unassign' as the assignee to clear assignment. `project_id` and `task_id` are required!"
    )]
    async fn assign_task(
        &self,
        Parameters(AssignTaskRequest {
            project_id,
            task_id,
            assignee,
        }): Parameters<AssignTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid project ID format"}"#,
                )]));
            }
        };
        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid task ID format"}"#,
                )]));
            }
        };

        // Find the task
        let current_task =
            match Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        r#"{"success": false, "error": "Task not found in project"}"#,
                    )]));
                }
                Err(e) => {
                    let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                    return Ok(CallToolResult::error(vec![Content::text(msg)]));
                }
            };

        // Resolve assignee — either "unassign" or a username/UUID
        let assignee_id: Option<String> = if assignee.to_lowercase() == "unassign" {
            None
        } else {
            // Try as UUID first, then look up by username
            match Uuid::parse_str(&assignee) {
                Ok(uuid) => Some(uuid.to_string()),
                Err(_) => {
                    // Look up by username
                    #[derive(sqlx::FromRow)]
                    struct UserId { id: Vec<u8> }
                    match sqlx::query_as::<_, UserId>(
                        "SELECT id FROM users WHERE username = ? COLLATE NOCASE AND is_active = 1",
                    )
                    .bind(&assignee)
                    .fetch_optional(&self.pool)
                    .await
                    {
                        Ok(Some(row)) => Uuid::from_slice(&row.id).ok().map(|u| u.to_string()),
                        _ => {
                            let msg = format!(
                                r#"{{"success": false, "error": "User '{}' not found"}}"#,
                                assignee
                            );
                            return Ok(CallToolResult::error(vec![Content::text(msg)]));
                        }
                    }
                }
            }
        };

        let custom_properties = current_task
            .custom_properties
            .as_ref()
            .map(|json| json.0.clone())
            .map(SqlxJson);

        let task_title = current_task.title.clone();
        match Task::update(
            &self.pool,
            task_uuid,
            project_uuid,
            current_task.title,
            current_task.description,
            current_task.status,
            current_task.parent_task_attempt,
            current_task.pod_id,
            current_task.board_id,
            current_task.priority,
            assignee_id.clone(),
            current_task.assigned_agent,
            current_task.assigned_mcps,
            current_task.requires_approval,
            current_task.approval_status,
            current_task.parent_task_id,
            current_task.tags,
            current_task.due_date,
            custom_properties,
            current_task.scheduled_start,
            current_task.scheduled_end,
        )
        .await
        {
            Ok(_) => {
                let msg = if assignee_id.is_some() {
                    format!("Task '{}' assigned to '{}'", task_title, assignee)
                } else {
                    format!("Task '{}' unassigned", task_title)
                };
                let response = serde_json::json!({ "success": true, "message": msg });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                Ok(CallToolResult::error(vec![Content::text(msg)]))
            }
        }
    }

    #[tool(
        description = "Add a comment to a task. Use this to leave notes, status updates, or handoff messages. `project_id`, `task_id`, and `content` are required!"
    )]
    async fn add_comment(
        &self,
        Parameters(AddCommentRequest {
            project_id,
            task_id,
            content,
            comment_type,
        }): Parameters<AddCommentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let _project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid project ID format"}"#,
                )]));
            }
        };
        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid task ID format"}"#,
                )]));
            }
        };

        let ct = match comment_type.as_deref() {
            Some("status_update") => CommentType::StatusUpdate,
            Some("review") => CommentType::Review,
            Some("system") => CommentType::System,
            Some("handoff") => CommentType::Handoff,
            _ => CommentType::Comment,
        };

        let author_id = self
            .user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "mcp-agent".to_string());

        let data = CreateTaskComment {
            task_id: task_uuid,
            author_id,
            author_type: AuthorType::Agent,
            content,
            comment_type: Some(ct),
            parent_comment_id: None,
            mentions: None,
            metadata: None,
        };

        match TaskComment::create(&self.pool, &data).await {
            Ok(comment) => {
                let response = AddCommentResponse {
                    success: true,
                    comment_id: comment.id.to_string(),
                    message: "Comment added successfully".to_string(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                Ok(CallToolResult::error(vec![Content::text(msg)]))
            }
        }
    }

    #[tool(
        description = "List all members of a project with their roles. `project_id` is required!"
    )]
    async fn list_project_members(
        &self,
        Parameters(ListProjectMembersRequest { project_id }): Parameters<ListProjectMembersRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid project ID format"}"#,
                )]));
            }
        };

        #[derive(sqlx::FromRow)]
        struct MemberRow {
            user_id: Vec<u8>,
            username: String,
            full_name: String,
            role: String,
            is_admin: i32,
        }

        let members = sqlx::query_as::<_, MemberRow>(
            r#"SELECT pm.user_id, u.username, u.full_name, pm.role, u.is_admin
               FROM project_members pm
               JOIN users u ON pm.user_id = u.id
               WHERE pm.project_id = ? AND u.is_active = 1"#,
        )
        .bind(project_uuid.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await;

        match members {
            Ok(rows) => {
                let members: Vec<ProjectMemberSummary> = rows
                    .iter()
                    .filter_map(|r| {
                        Uuid::from_slice(&r.user_id).ok().map(|uid| ProjectMemberSummary {
                            user_id: uid.to_string(),
                            username: r.username.clone(),
                            full_name: r.full_name.clone(),
                            role: r.role.clone(),
                            is_admin: r.is_admin == 1,
                        })
                    })
                    .collect();
                let count = members.len();
                let response = ListProjectMembersResponse {
                    success: true,
                    members,
                    count,
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                Ok(CallToolResult::error(vec![Content::text(msg)]))
            }
        }
    }

    #[tool(description = "Evaluate PCG governance policies for a task before execution.")]
    async fn evaluate_policy(
        &self,
        Parameters(EvaluatePolicyRequest {
            project_id,
            task_id,
            severity,
            tags,
            actor,
        }): Parameters<EvaluatePolicyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let tags_vec = tags.unwrap_or_default();
        let context = PolicyCheckContext {
            task_id: task_id.clone(),
            project_id: project_id.clone(),
            severity: severity.clone(),
            tags: tags_vec.clone(),
            actor,
        };

        match pcg_policy::evaluate(context.clone()) {
            Ok(decision) => {
                pcg_policy::record_decision(&context, &decision);
                let actions: Vec<String> = decision
                    .actions
                    .iter()
                    .map(|action| match action {
                        PolicyAction::RequireHumanReview => "require_human_review".to_string(),
                        PolicyAction::EscalateCrisis => "escalate_crisis".to_string(),
                        PolicyAction::StartWorkflow { workflow } => {
                            format!("start_workflow:{workflow}")
                        }
                        PolicyAction::Annotate { key, value } => {
                            format!("annotate:{key}={value}")
                        }
                    })
                    .collect();

                let response = EvaluatePolicyResponse {
                    allow: decision.allow,
                    actions,
                    notes: decision.notes.clone(),
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "Policy evaluation complete".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to evaluate PCG policy",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Policy evaluation error".to_string()),
                )]))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "pcg-dashboard-mcp".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("PCG Dashboard MCP exposes project management tools. Always pass the `project_id` you are working with. Call `list_tasks` to discover task IDs, `list_project_members` to find team members. TOOLS: 'list_projects', 'list_tasks', 'create_task', 'get_task', 'update_task', 'delete_task', 'assign_task', 'add_comment', 'list_project_members', 'evaluate_policy'. Provide `project_id`/`task_id` as required.".to_string()),
        }
    }
}

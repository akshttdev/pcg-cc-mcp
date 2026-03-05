use std::{collections::HashMap, future::Future, path::PathBuf};

use chrono::{DateTime, Utc};
use db::models::{
    agent::{Agent, AgentStatus},
    project::Project,
    project_knowledge_source::{
        KnowledgeSourceType, ProjectKnowledgeSource,
    },
    task::{CreateTask, Priority, Task, TaskStatus, TaskWithAttemptStatus},
    task_dependency::{CreateTaskDependency, DependencyType, TaskDependency},
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        Annotated, CallToolResult, Content, Implementation, ProtocolVersion,
        ReadResourceRequestParam, ReadResourceResult, ResourceContents,
        ResourceTemplate, RawResourceTemplate, ServerCapabilities, ServerInfo,
        ListResourceTemplatesResult,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use services::services::pcg_policy::{self, PolicyAction, PolicyCheckContext};
use sqlx::{SqlitePool, types::Json as SqlxJson};
use uuid::Uuid;

// ─── Helper Functions ───────────────────────────────────────────────────────

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

fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "critical" => Some(Priority::Critical),
        "high" => Some(Priority::High),
        "medium" => Some(Priority::Medium),
        "low" => Some(Priority::Low),
        _ => None,
    }
}

fn priority_to_string(p: &Priority) -> String {
    match p {
        Priority::Critical => "critical".to_string(),
        Priority::High => "high".to_string(),
        Priority::Medium => "medium".to_string(),
        Priority::Low => "low".to_string(),
    }
}

fn parse_iso_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first, then common ISO formats
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc());
    }
    None
}

fn parse_uuid(s: &str, field_name: &str) -> Result<Uuid, CallToolResult> {
    Uuid::parse_str(s).map_err(|_| {
        let err = serde_json::json!({
            "success": false,
            "error": format!("Invalid {} format. Must be a valid UUID.", field_name),
        });
        CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&err).unwrap(),
        )])
    })
}

fn error_result(error: &str, details: Option<&str>) -> CallToolResult {
    let mut obj = serde_json::json!({ "success": false, "error": error });
    if let Some(d) = details {
        obj["details"] = Value::String(d.to_string());
    }
    CallToolResult::error(vec![Content::text(
        serde_json::to_string_pretty(&obj).unwrap(),
    )])
}

fn success_json<T: Serialize>(value: &T) -> CallToolResult {
    CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(value).unwrap(),
    )])
}

/// Build TaskSummary from a Task model
fn task_to_summary(task: &Task) -> TaskSummary {
    let tags: Option<Vec<String>> = task
        .tags
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    TaskSummary {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: task_status_to_string(&task.status),
        priority: priority_to_string(&task.priority),
        assignee_id: task.assignee_id.clone(),
        assigned_agent: task.assigned_agent.clone(),
        tags,
        due_date: task.due_date.map(|d| d.to_rfc3339()),
        parent_task_id: task.parent_task_id.map(|id| id.to_string()),
        requires_approval: task.requires_approval,
        approval_status: task.approval_status.as_ref().map(|s| format!("{:?}", s).to_lowercase()),
        created_by: task.created_by.clone(),
        vibe_cost: None,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
        has_in_progress_attempt: None,
        has_merged_attempt: None,
        last_attempt_failed: None,
    }
}

/// Build TaskSummary from TaskWithAttemptStatus
fn task_with_status_to_summary(task: &TaskWithAttemptStatus) -> TaskSummary {
    let tags: Option<Vec<String>> = task.tags.as_deref().and_then(|s| serde_json::from_str(s).ok());

    TaskSummary {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: task_status_to_string(&task.status),
        priority: priority_to_string(&task.priority),
        assignee_id: task.assignee_id.clone(),
        assigned_agent: task.assigned_agent.clone(),
        tags,
        due_date: task.due_date.map(|d| d.to_rfc3339()),
        parent_task_id: task.parent_task_id.map(|id| id.to_string()),
        requires_approval: task.requires_approval,
        approval_status: task.approval_status.as_ref().map(|s| format!("{:?}", s).to_lowercase()),
        created_by: task.created_by.clone(),
        vibe_cost: task.vibe_cost,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
        has_in_progress_attempt: Some(task.has_in_progress_attempt),
        has_merged_attempt: Some(task.has_merged_attempt),
        last_attempt_failed: Some(task.last_attempt_failed),
    }
}

// ─── Request / Response Types ───────────────────────────────────────────────

// Phase 1: Enriched CreateTaskRequest
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskRequest {
    #[schemars(description = "The ID of the project to create the task in. This is required!")]
    pub project_id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
    #[schemars(description = "Priority: critical, high, medium, low (default: medium)")]
    pub priority: Option<String>,
    #[schemars(description = "User ID to assign this task to")]
    pub assignee_id: Option<String>,
    #[schemars(description = "Agent name to assign (e.g. 'Nora')")]
    pub assigned_agent: Option<String>,
    #[schemars(description = "Tags for categorization")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Due date (ISO 8601, e.g. '2026-03-15' or '2026-03-15T10:00:00Z')")]
    pub due_date: Option<String>,
    #[schemars(description = "Scheduled start date (ISO 8601)")]
    pub scheduled_start: Option<String>,
    #[schemars(description = "Scheduled end date (ISO 8601)")]
    pub scheduled_end: Option<String>,
    #[schemars(description = "Parent task UUID for subtasks")]
    pub parent_task_id: Option<String>,
    #[schemars(description = "Board UUID to place the task on")]
    pub board_id: Option<String>,
    #[schemars(description = "Whether this task requires approval before completion")]
    pub requires_approval: Option<bool>,
    #[schemars(description = "Arbitrary custom properties as JSON object")]
    pub custom_properties: Option<Value>,
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

// Phase 1: Enriched ListTasksRequest
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
    #[schemars(description = "Page number (1-based, default: 1)")]
    pub page: Option<i32>,
    #[schemars(description = "Filter by priority: critical, high, medium, low")]
    pub priority: Option<String>,
    #[schemars(description = "Filter by assignee user ID")]
    pub assignee_id: Option<String>,
    #[schemars(description = "Filter by assigned agent name")]
    pub assigned_agent: Option<String>,
    #[schemars(description = "Filter by tag (tasks containing this tag)")]
    pub tag: Option<String>,
    #[schemars(description = "Search keyword in title and description")]
    pub search: Option<String>,
    #[schemars(description = "Sort by: created_at, updated_at, priority, due_date, title (default: created_at)")]
    pub sort_by: Option<String>,
    #[schemars(description = "Sort direction: asc or desc (default: desc)")]
    pub sort_direction: Option<String>,
}

// Phase 1: Enriched TaskSummary
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
    #[schemars(description = "Priority level")]
    pub priority: String,
    #[schemars(description = "Assigned user ID")]
    pub assignee_id: Option<String>,
    #[schemars(description = "Assigned agent name")]
    pub assigned_agent: Option<String>,
    #[schemars(description = "Tags")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Due date (ISO 8601)")]
    pub due_date: Option<String>,
    #[schemars(description = "Parent task ID for subtasks")]
    pub parent_task_id: Option<String>,
    #[schemars(description = "Whether this task requires approval")]
    pub requires_approval: bool,
    #[schemars(description = "Approval status if applicable")]
    pub approval_status: Option<String>,
    #[schemars(description = "Who created this task")]
    pub created_by: String,
    #[schemars(description = "Total VIBE cost for this task")]
    pub vibe_cost: Option<i64>,
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
}

// Phase 1: Enriched ListTasksResponse
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksResponse {
    pub success: bool,
    pub tasks: Vec<TaskSummary>,
    pub count: usize,
    pub total_count: usize,
    pub page: i32,
    pub total_pages: i32,
    pub project_id: String,
    pub project_name: Option<String>,
    pub applied_filters: ListTasksFilters,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksFilters {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee_id: Option<String>,
    pub assigned_agent: Option<String>,
    pub tag: Option<String>,
    pub search: Option<String>,
    pub sort_by: String,
    pub sort_direction: String,
    pub limit: i32,
    pub page: i32,
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

// Phase 1: Enriched UpdateTaskRequest
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
    #[schemars(description = "New priority: critical, high, medium, low")]
    pub priority: Option<String>,
    #[schemars(description = "New assignee user ID (empty string to clear)")]
    pub assignee_id: Option<String>,
    #[schemars(description = "New assigned agent name (empty string to clear)")]
    pub assigned_agent: Option<String>,
    #[schemars(description = "New tags (replaces existing tags)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "New due date (ISO 8601, empty string to clear)")]
    pub due_date: Option<String>,
    #[schemars(description = "New scheduled start (ISO 8601, empty string to clear)")]
    pub scheduled_start: Option<String>,
    #[schemars(description = "New scheduled end (ISO 8601, empty string to clear)")]
    pub scheduled_end: Option<String>,
    #[schemars(description = "New parent task ID (empty string to clear)")]
    pub parent_task_id: Option<String>,
    #[schemars(description = "New board ID (empty string to clear)")]
    pub board_id: Option<String>,
    #[schemars(description = "Whether this task requires approval")]
    pub requires_approval: Option<bool>,
    #[schemars(description = "New custom properties (JSON object, null to clear)")]
    pub custom_properties: Option<Value>,
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

// ─── Phase 2: Bulk Operations Types ─────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkCreateTaskItem {
    #[schemars(description = "Task title")]
    pub title: String,
    #[schemars(description = "Task description")]
    pub description: Option<String>,
    #[schemars(description = "Priority: critical, high, medium, low")]
    pub priority: Option<String>,
    #[schemars(description = "Assignee user ID")]
    pub assignee_id: Option<String>,
    #[schemars(description = "Tags")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Due date (ISO 8601)")]
    pub due_date: Option<String>,
    #[schemars(description = "Parent task UUID for subtasks")]
    pub parent_task_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkCreateTasksRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Tasks to create (max 50)")]
    pub tasks: Vec<BulkCreateTaskItem>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkUpdateTaskItem {
    #[schemars(description = "Task UUID to update")]
    pub task_id: String,
    #[schemars(description = "New status")]
    pub status: Option<String>,
    #[schemars(description = "New priority")]
    pub priority: Option<String>,
    #[schemars(description = "New assignee")]
    pub assignee_id: Option<String>,
    #[schemars(description = "New assigned agent")]
    pub assigned_agent: Option<String>,
    #[schemars(description = "New tags")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkUpdateTasksRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Updates to apply (max 50)")]
    pub updates: Vec<BulkUpdateTaskItem>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchTasksRequest {
    #[schemars(description = "Search keyword (matches title and description)")]
    pub query: String,
    #[schemars(description = "Optional project UUID to scope search")]
    pub project_id: Option<String>,
    #[schemars(description = "Optional status filter")]
    pub status: Option<String>,
    #[schemars(description = "Optional priority filter")]
    pub priority: Option<String>,
    #[schemars(description = "Max results (default: 20, max: 100)")]
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ManageDependenciesRequest {
    #[schemars(description = "Action: add, remove, or list")]
    pub action: String,
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Source task UUID (required for add/remove)")]
    pub source_task_id: Option<String>,
    #[schemars(description = "Target task UUID (required for add/remove)")]
    pub target_task_id: Option<String>,
    #[schemars(description = "Dependency type: blocks or relates_to (default: blocks)")]
    pub dependency_type: Option<String>,
    #[schemars(description = "Task UUID to list dependencies for (required for list action)")]
    pub task_id: Option<String>,
}

// ─── Phase 3: Knowledge Types ───────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddKnowledgeRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Source type: conversation, artifact, pulse_content, context_injection, entity, topology_snapshot")]
    pub source_type: String,
    #[schemars(description = "Unique identifier for this knowledge source")]
    pub source_id: String,
    #[schemars(description = "Human-readable title")]
    pub source_title: String,
    #[schemars(description = "Brief summary of what this source covers")]
    pub source_summary: Option<String>,
    #[schemars(description = "Coverage score 0.0-1.0 (how much of the topic this source covers)")]
    pub coverage_score: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListKnowledgeRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Optional filter by source_type")]
    pub source_type: Option<String>,
    #[schemars(description = "Include stale sources (default: false)")]
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetKnowledgeCompletenessRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ManageKnowledgeRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Action: mark_stale, mark_refreshed, or delete")]
    pub action: String,
    #[schemars(description = "Knowledge source UUID")]
    pub source_id: String,
}

// ─── Phase 4: Topology / VIBE / Agent Types ─────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTopologyRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Filter by node type: agent, task, resource, account, workflow")]
    pub node_type: Option<String>,
    #[schemars(description = "Filter by edge type: can_execute, has_access, depends_on, etc.")]
    pub edge_type: Option<String>,
    #[schemars(description = "Only return active nodes/edges (default: true)")]
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTopologyIssuesRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Filter by severity: info, warning, error, critical")]
    pub severity: Option<String>,
    #[schemars(description = "Include resolved issues (default: false)")]
    pub include_resolved: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindTopologyPathRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(description = "Starting node UUID")]
    pub from_node_id: String,
    #[schemars(description = "Target node UUID")]
    pub to_node_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetVibeBudgetRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListAgentsRequest {
    #[schemars(description = "Filter by status: active, inactive, maintenance, training")]
    pub status: Option<String>,
}

// ─── Server ─────────────────────────────────────────────────────────────────

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
            is_admin: true,
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

    /// Resolve project IDs accessible to the current user
    async fn accessible_project_ids(&self) -> Result<Option<Vec<Uuid>>, sqlx::Error> {
        if self.is_admin || self.user_id.is_none() {
            return Ok(None); // None = all projects accessible
        }
        let user_id = self.user_id.unwrap();
        let user_id_bytes = user_id.as_bytes().to_vec();

        #[derive(sqlx::FromRow)]
        struct ProjId {
            project_id: String,
        }

        let rows: Vec<ProjId> = sqlx::query_as(
            "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?",
        )
        .bind(&user_id_bytes)
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<Uuid> = rows
            .into_iter()
            .filter_map(|r| Uuid::parse_str(&r.project_id).ok())
            .collect();
        Ok(Some(ids))
    }
}

#[tool_router]
impl TaskServer {
    // ═════════════════════════════════════════════════════════════════════════
    // PHASE 1 — Enhanced existing tools
    // ═════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Create a new task/ticket in a project with full field support. Always pass the `project_id` — it is required!"
    )]
    async fn create_task(
        &self,
        Parameters(req): Parameters<CreateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Project::exists(&self.pool, project_uuid).await {
            Ok(false) => return Ok(error_result("Project not found", None)),
            Err(e) => return Ok(error_result("Failed to check project", Some(&e.to_string()))),
            Ok(true) => {}
        }

        let priority = req.priority.as_deref().and_then(parse_priority);
        let due_date = req.due_date.as_deref().and_then(parse_iso_datetime);
        let scheduled_start = req.scheduled_start.as_deref().and_then(parse_iso_datetime);
        let scheduled_end = req.scheduled_end.as_deref().and_then(parse_iso_datetime);
        let parent_task_id = req
            .parent_task_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());
        let board_id = req
            .board_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());

        let task_id = Uuid::new_v4();
        let create_task_data = CreateTask {
            project_id: project_uuid,
            pod_id: None,
            board_id,
            title: req.title.clone(),
            description: req.description.clone(),
            parent_task_attempt: None,
            image_ids: None,
            priority,
            assignee_id: req.assignee_id.clone(),
            assigned_agent: req.assigned_agent.clone(),
            agent_id: None,
            assigned_mcps: None,
            created_by: self
                .user_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "mcp".to_string()),
            requires_approval: req.requires_approval,
            parent_task_id,
            tags: req.tags.clone(),
            due_date,
            custom_properties: req.custom_properties.clone(),
            scheduled_start,
            scheduled_end,
            screenshot: None,
        };

        match Task::create(&self.pool, &create_task_data, task_id).await {
            Ok(_task) => Ok(success_json(&CreateTaskResponse {
                success: true,
                task_id: task_id.to_string(),
                message: "Task created successfully".to_string(),
            })),
            Err(e) => Ok(error_result("Failed to create task", Some(&e.to_string()))),
        }
    }

    #[tool(description = "List all projects accessible to the current user")]
    async fn list_projects(&self) -> Result<CallToolResult, ErrorData> {
        let projects_result = if self.is_admin || self.user_id.is_none() {
            Project::find_all(&self.pool).await
        } else {
            let user_id = self.user_id.unwrap();
            let user_id_bytes = user_id.as_bytes().to_vec();

            #[derive(sqlx::FromRow)]
            struct ProjId {
                project_id: String,
            }

            let project_ids: Vec<String> = match sqlx::query_as::<_, ProjId>(
                "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?",
            )
            .bind(&user_id_bytes)
            .fetch_all(&self.pool)
            .await
            {
                Ok(rows) => rows.into_iter().map(|r| r.project_id).collect(),
                Err(e) => return Ok(error_result("Failed to retrieve user projects", Some(&e.to_string()))),
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

                Ok(success_json(&ListProjectsResponse {
                    success: true,
                    projects: project_summaries,
                    count,
                }))
            }
            Err(e) => Ok(error_result("Failed to retrieve projects", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "List tasks in a project with filtering, search, sorting, and pagination. `project_id` is required!"
    )]
    async fn list_tasks(
        &self,
        Parameters(req): Parameters<ListTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let status_filter = if let Some(ref status_str) = req.status {
            match parse_task_status(status_str) {
                Some(s) => Some(s),
                None => return Ok(error_result(
                    "Invalid status filter. Valid: todo, inprogress, inreview, done, cancelled",
                    None,
                )),
            }
        } else {
            None
        };

        let priority_filter = req.priority.as_deref().and_then(parse_priority);

        let project = match Project::find_by_id(&self.pool, project_uuid).await {
            Ok(Some(p)) => p,
            Ok(None) => return Ok(error_result("Project not found", None)),
            Err(e) => return Ok(error_result("Failed to check project", Some(&e.to_string()))),
        };

        let task_limit = req.limit.unwrap_or(50).clamp(1, 200);
        let page = req.page.unwrap_or(1).max(1);
        let sort_by = req.sort_by.clone().unwrap_or_else(|| "created_at".to_string());
        let sort_dir = req.sort_direction.clone().unwrap_or_else(|| "desc".to_string());

        let tasks_result =
            Task::find_by_project_id_with_attempt_status(&self.pool, project_uuid).await;

        match tasks_result {
            Ok(tasks) => {
                // Apply filters
                let mut filtered: Vec<&TaskWithAttemptStatus> = tasks
                    .iter()
                    .filter(|t| {
                        if let Some(ref fs) = status_filter {
                            if &t.status != fs {
                                return false;
                            }
                        }
                        if let Some(ref fp) = priority_filter {
                            if &t.priority != fp {
                                return false;
                            }
                        }
                        if let Some(ref fa) = req.assignee_id {
                            if t.assignee_id.as_deref() != Some(fa.as_str()) {
                                return false;
                            }
                        }
                        if let Some(ref fag) = req.assigned_agent {
                            if t.assigned_agent.as_deref() != Some(fag.as_str()) {
                                return false;
                            }
                        }
                        if let Some(ref ft) = req.tag {
                            let has_tag = t
                                .tags
                                .as_deref()
                                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                                .map(|tags| tags.iter().any(|tag| tag.eq_ignore_ascii_case(ft)))
                                .unwrap_or(false);
                            if !has_tag {
                                return false;
                            }
                        }
                        if let Some(ref search) = req.search {
                            let kw = search.to_lowercase();
                            let in_title = t.title.to_lowercase().contains(&kw);
                            let in_desc = t
                                .description
                                .as_deref()
                                .map(|d| d.to_lowercase().contains(&kw))
                                .unwrap_or(false);
                            if !in_title && !in_desc {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                // Sort
                filtered.sort_by(|a, b| {
                    let cmp = match sort_by.as_str() {
                        "updated_at" => a.updated_at.cmp(&b.updated_at),
                        "priority" => {
                            let p = |pr: &Priority| match pr {
                                Priority::Critical => 0,
                                Priority::High => 1,
                                Priority::Medium => 2,
                                Priority::Low => 3,
                            };
                            p(&a.priority).cmp(&p(&b.priority))
                        }
                        "due_date" => a.due_date.cmp(&b.due_date),
                        "title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                        _ => a.created_at.cmp(&b.created_at), // default: created_at
                    };
                    if sort_dir == "asc" {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });

                let total_count = filtered.len();
                let total_pages = ((total_count as f64) / (task_limit as f64)).ceil() as i32;
                let skip = ((page - 1) * task_limit) as usize;

                let page_tasks: Vec<TaskSummary> = filtered
                    .into_iter()
                    .skip(skip)
                    .take(task_limit as usize)
                    .map(task_with_status_to_summary)
                    .collect();

                let count = page_tasks.len();

                Ok(success_json(&ListTasksResponse {
                    success: true,
                    tasks: page_tasks,
                    count,
                    total_count,
                    page,
                    total_pages: total_pages.max(1),
                    project_id: req.project_id.clone(),
                    project_name: Some(project.name),
                    applied_filters: ListTasksFilters {
                        status: req.status.clone(),
                        priority: req.priority.clone(),
                        assignee_id: req.assignee_id.clone(),
                        assigned_agent: req.assigned_agent.clone(),
                        tag: req.tag.clone(),
                        search: req.search.clone(),
                        sort_by,
                        sort_direction: sort_dir,
                        limit: task_limit,
                        page,
                    },
                }))
            }
            Err(e) => Ok(error_result("Failed to retrieve tasks", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Update an existing task with full field support. `project_id` and `task_id` are required!"
    )]
    async fn update_task(
        &self,
        Parameters(req): Parameters<UpdateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let task_uuid = match parse_uuid(&req.task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let status_enum = if let Some(ref s) = req.status {
            match parse_task_status(s) {
                Some(st) => Some(st),
                None => return Ok(error_result(
                    "Invalid status. Valid: todo, inprogress, inreview, done, cancelled",
                    None,
                )),
            }
        } else {
            None
        };

        let current_task =
            match Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await {
                Ok(Some(t)) => t,
                Ok(None) => return Ok(error_result("Task not found in the specified project", None)),
                Err(e) => return Ok(error_result("Failed to retrieve task", Some(&e.to_string()))),
            };

        let new_title = req.title.unwrap_or_else(|| current_task.title.clone());
        let new_description = req.description.or_else(|| current_task.description.clone());
        let new_status = status_enum.unwrap_or_else(|| current_task.status.clone());

        let new_priority = req
            .priority
            .as_deref()
            .and_then(parse_priority)
            .unwrap_or_else(|| current_task.priority.clone());

        let new_assignee_id = match &req.assignee_id {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s.clone()),
            None => current_task.assignee_id.clone(),
        };

        let new_assigned_agent = match &req.assigned_agent {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s.clone()),
            None => current_task.assigned_agent.clone(),
        };

        let new_tags = match &req.tags {
            Some(t) => Some(serde_json::to_string(t).unwrap()),
            None => current_task.tags.clone(),
        };

        let new_due_date = match &req.due_date {
            Some(s) if s.is_empty() => None,
            Some(s) => parse_iso_datetime(s).or(current_task.due_date),
            None => current_task.due_date,
        };

        let new_scheduled_start = match &req.scheduled_start {
            Some(s) if s.is_empty() => None,
            Some(s) => parse_iso_datetime(s).or(current_task.scheduled_start),
            None => current_task.scheduled_start,
        };

        let new_scheduled_end = match &req.scheduled_end {
            Some(s) if s.is_empty() => None,
            Some(s) => parse_iso_datetime(s).or(current_task.scheduled_end),
            None => current_task.scheduled_end,
        };

        let new_parent_task_id = match &req.parent_task_id {
            Some(s) if s.is_empty() => None,
            Some(s) => Uuid::parse_str(s).ok().or(current_task.parent_task_id),
            None => current_task.parent_task_id,
        };

        let new_board_id = match &req.board_id {
            Some(s) if s.is_empty() => None,
            Some(s) => Uuid::parse_str(s).ok().or(current_task.board_id),
            None => current_task.board_id,
        };

        let new_requires_approval = req.requires_approval.unwrap_or(current_task.requires_approval);

        let new_custom_properties = match &req.custom_properties {
            Some(Value::Null) => None,
            Some(v) => Some(SqlxJson(v.clone())),
            None => current_task
                .custom_properties
                .as_ref()
                .map(|j| SqlxJson(j.0.clone())),
        };

        match Task::update(
            &self.pool,
            task_uuid,
            project_uuid,
            new_title,
            new_description,
            new_status,
            current_task.parent_task_attempt,
            current_task.pod_id,
            new_board_id,
            new_priority,
            new_assignee_id,
            new_assigned_agent,
            current_task.assigned_mcps,
            new_requires_approval,
            current_task.approval_status,
            new_parent_task_id,
            new_tags,
            new_due_date,
            new_custom_properties,
            new_scheduled_start,
            new_scheduled_end,
        )
        .await
        {
            Ok(updated) => Ok(success_json(&UpdateTaskResponse {
                success: true,
                message: "Task updated successfully".to_string(),
                task: Some(task_to_summary(&updated)),
            })),
            Err(e) => Ok(error_result("Failed to update task", Some(&e.to_string()))),
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
        let project_uuid = match parse_uuid(&project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let task_uuid = match parse_uuid(&task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Task::exists(&self.pool, task_uuid, project_uuid).await {
            Ok(true) => match Task::delete(&self.pool, task_uuid).await {
                Ok(rows) if rows > 0 => Ok(success_json(&DeleteTaskResponse {
                    success: true,
                    message: "Task deleted successfully".to_string(),
                    deleted_task_id: Some(task_id),
                })),
                Ok(_) => Ok(error_result("Task not found or already deleted", None)),
                Err(e) => Ok(error_result("Failed to delete task", Some(&e.to_string()))),
            },
            Ok(false) => Ok(error_result("Task not found in the specified project", None)),
            Err(e) => Ok(error_result("Failed to check task existence", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Get detailed information about a specific task. `project_id` and `task_id` are required!"
    )]
    async fn get_task(
        &self,
        Parameters(GetTaskRequest {
            project_id,
            task_id,
        }): Parameters<GetTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let task_uuid = match parse_uuid(&task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let task_result =
            Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await;
        let project_result = Project::find_by_id(&self.pool, project_uuid).await;

        match (task_result, project_result) {
            (Ok(Some(task)), Ok(Some(project))) => Ok(success_json(&GetTaskResponse {
                success: true,
                task: Some(task_to_summary(&task)),
                project_name: Some(project.name),
            })),
            (Ok(None), _) | (_, Ok(None)) => Ok(error_result("Task or project not found", None)),
            (Err(e), _) | (_, Err(e)) => Ok(error_result(
                "Failed to retrieve task or project",
                Some(&e.to_string()),
            )),
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

                Ok(success_json(&EvaluatePolicyResponse {
                    allow: decision.allow,
                    actions,
                    notes: decision.notes.clone(),
                }))
            }
            Err(e) => Ok(error_result("Failed to evaluate PCG policy", Some(&e.to_string()))),
        }
    }

    // ═════════════════════════════════════════════════════════════════════════
    // PHASE 2 — Bulk Operations + Search + Dependencies
    // ═════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Create multiple tasks at once (max 50). Returns per-item success/failure. `project_id` is required!"
    )]
    async fn bulk_create_tasks(
        &self,
        Parameters(req): Parameters<BulkCreateTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Project::exists(&self.pool, project_uuid).await {
            Ok(false) => return Ok(error_result("Project not found", None)),
            Err(e) => return Ok(error_result("Failed to check project", Some(&e.to_string()))),
            Ok(true) => {}
        }

        if req.tasks.is_empty() {
            return Ok(error_result("No tasks provided", None));
        }
        if req.tasks.len() > 50 {
            return Ok(error_result("Maximum 50 tasks per bulk create", None));
        }

        let created_by = self
            .user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "mcp".to_string());

        let mut results: Vec<Value> = Vec::new();
        let mut success_count = 0;

        for (i, item) in req.tasks.iter().enumerate() {
            let task_id = Uuid::new_v4();
            let priority = item.priority.as_deref().and_then(parse_priority);
            let due_date = item.due_date.as_deref().and_then(parse_iso_datetime);
            let parent_task_id = item
                .parent_task_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok());

            let create_data = CreateTask {
                project_id: project_uuid,
                pod_id: None,
                board_id: None,
                title: item.title.clone(),
                description: item.description.clone(),
                parent_task_attempt: None,
                image_ids: None,
                priority,
                assignee_id: item.assignee_id.clone(),
                assigned_agent: None,
                agent_id: None,
                assigned_mcps: None,
                created_by: created_by.clone(),
                requires_approval: None,
                parent_task_id,
                tags: item.tags.clone(),
                due_date,
                custom_properties: None,
                scheduled_start: None,
                scheduled_end: None,
                screenshot: None,
            };

            match Task::create(&self.pool, &create_data, task_id).await {
                Ok(_) => {
                    success_count += 1;
                    results.push(serde_json::json!({
                        "index": i,
                        "success": true,
                        "task_id": task_id.to_string(),
                        "title": item.title,
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": i,
                        "success": false,
                        "title": item.title,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "total_requested": req.tasks.len(),
            "total_created": success_count,
            "total_failed": req.tasks.len() - success_count,
            "results": results,
        })))
    }

    #[tool(
        description = "Update multiple tasks at once (max 50). Returns per-item results. `project_id` is required!"
    )]
    async fn bulk_update_tasks(
        &self,
        Parameters(req): Parameters<BulkUpdateTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        if req.updates.is_empty() {
            return Ok(error_result("No updates provided", None));
        }
        if req.updates.len() > 50 {
            return Ok(error_result("Maximum 50 updates per bulk update", None));
        }

        let mut results: Vec<Value> = Vec::new();
        let mut success_count = 0;

        for (i, item) in req.updates.iter().enumerate() {
            let task_uuid = match Uuid::parse_str(&item.task_id) {
                Ok(u) => u,
                Err(_) => {
                    results.push(serde_json::json!({
                        "index": i, "success": false, "task_id": item.task_id,
                        "error": "Invalid task_id UUID"
                    }));
                    continue;
                }
            };

            let current =
                match Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        results.push(serde_json::json!({
                            "index": i, "success": false, "task_id": item.task_id,
                            "error": "Task not found"
                        }));
                        continue;
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "index": i, "success": false, "task_id": item.task_id,
                            "error": e.to_string()
                        }));
                        continue;
                    }
                };

            let new_status = item
                .status
                .as_deref()
                .and_then(parse_task_status)
                .unwrap_or_else(|| current.status.clone());
            let new_priority = item
                .priority
                .as_deref()
                .and_then(parse_priority)
                .unwrap_or_else(|| current.priority.clone());
            let new_assignee = match &item.assignee_id {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s.clone()),
                None => current.assignee_id.clone(),
            };
            let new_agent = match &item.assigned_agent {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s.clone()),
                None => current.assigned_agent.clone(),
            };
            let new_tags = match &item.tags {
                Some(t) => Some(serde_json::to_string(t).unwrap()),
                None => current.tags.clone(),
            };

            let cp = current
                .custom_properties
                .as_ref()
                .map(|j| SqlxJson(j.0.clone()));

            match Task::update(
                &self.pool,
                task_uuid,
                project_uuid,
                current.title.clone(),
                current.description.clone(),
                new_status,
                current.parent_task_attempt,
                current.pod_id,
                current.board_id,
                new_priority,
                new_assignee,
                new_agent,
                current.assigned_mcps,
                current.requires_approval,
                current.approval_status,
                current.parent_task_id,
                new_tags,
                current.due_date,
                cp,
                current.scheduled_start,
                current.scheduled_end,
            )
            .await
            {
                Ok(_) => {
                    success_count += 1;
                    results.push(serde_json::json!({
                        "index": i, "success": true, "task_id": item.task_id,
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": i, "success": false, "task_id": item.task_id,
                        "error": e.to_string()
                    }));
                }
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "total_requested": req.updates.len(),
            "total_updated": success_count,
            "total_failed": req.updates.len() - success_count,
            "results": results,
        })))
    }

    #[tool(
        description = "Search tasks by keyword across title and description. Optionally scope to a project."
    )]
    async fn search_tasks(
        &self,
        Parameters(req): Parameters<SearchTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = req.limit.unwrap_or(20).clamp(1, 100);
        let status_filter = req.status.as_deref().and_then(parse_task_status);
        let priority_filter = req.priority.as_deref().and_then(parse_priority);

        // Determine which projects to search
        let project_ids: Vec<Uuid> = if let Some(ref pid) = req.project_id {
            match parse_uuid(pid, "project_id") {
                Ok(u) => vec![u],
                Err(r) => return Ok(r),
            }
        } else {
            // Search accessible projects
            match self.accessible_project_ids().await {
                Ok(None) => {
                    // Admin: get all projects
                    match Project::find_all(&self.pool).await {
                        Ok(ps) => ps.into_iter().map(|p| p.id).collect(),
                        Err(e) => return Ok(error_result("Failed to list projects", Some(&e.to_string()))),
                    }
                }
                Ok(Some(ids)) => ids,
                Err(e) => return Ok(error_result("Failed to determine accessible projects", Some(&e.to_string()))),
            }
        };

        let mut all_tasks: Vec<TaskSummary> = Vec::new();

        for pid in &project_ids {
            let tasks = match Task::find_by_project_id_with_attempt_status(&self.pool, *pid).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            for t in &tasks {
                let kw_lower = req.query.to_lowercase();
                let in_title = t.title.to_lowercase().contains(&kw_lower);
                let in_desc = t
                    .description
                    .as_deref()
                    .map(|d| d.to_lowercase().contains(&kw_lower))
                    .unwrap_or(false);
                let in_tags = t
                    .tags
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(&kw_lower))
                    .unwrap_or(false);

                if !in_title && !in_desc && !in_tags {
                    continue;
                }
                if let Some(ref sf) = status_filter {
                    if &t.status != sf {
                        continue;
                    }
                }
                if let Some(ref pf) = priority_filter {
                    if &t.priority != pf {
                        continue;
                    }
                }

                all_tasks.push(task_with_status_to_summary(t));
                if all_tasks.len() >= limit as usize {
                    break;
                }
            }
            if all_tasks.len() >= limit as usize {
                break;
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "query": req.query,
            "count": all_tasks.len(),
            "tasks": all_tasks,
        })))
    }

    #[tool(
        description = "Manage task dependencies: add, remove, or list. Use action 'add' with source_task_id + target_task_id, 'remove' with source+target, or 'list' with task_id."
    )]
    async fn manage_task_dependencies(
        &self,
        Parameters(req): Parameters<ManageDependenciesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match req.action.as_str() {
            "add" => {
                let source = match req.source_task_id.as_deref().map(|s| parse_uuid(s, "source_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("source_task_id required for 'add'", None)),
                };
                let target = match req.target_task_id.as_deref().map(|s| parse_uuid(s, "target_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("target_task_id required for 'add'", None)),
                };

                let dep_type = match req.dependency_type.as_deref().unwrap_or("blocks") {
                    "blocks" => DependencyType::Blocks,
                    "relates_to" => DependencyType::RelatesTo,
                    _ => return Ok(error_result("dependency_type must be 'blocks' or 'relates_to'", None)),
                };

                let payload = CreateTaskDependency {
                    project_id: project_uuid,
                    source_task_id: source,
                    target_task_id: target,
                    dependency_type: dep_type,
                };

                match TaskDependency::create(&self.pool, &payload).await {
                    Ok(dep) => Ok(success_json(&serde_json::json!({
                        "success": true,
                        "message": "Dependency created",
                        "dependency_id": dep.id.to_string(),
                        "source_task_id": dep.source_task_id.to_string(),
                        "target_task_id": dep.target_task_id.to_string(),
                        "dependency_type": format!("{:?}", dep.dependency_type).to_lowercase(),
                    }))),
                    Err(e) => Ok(error_result("Failed to create dependency", Some(&e.to_string()))),
                }
            }
            "remove" => {
                let source = match req.source_task_id.as_deref().map(|s| parse_uuid(s, "source_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("source_task_id required for 'remove'", None)),
                };
                let target = match req.target_task_id.as_deref().map(|s| parse_uuid(s, "target_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("target_task_id required for 'remove'", None)),
                };

                // Find matching dependency and delete
                let deps = match TaskDependency::list_by_task(&self.pool, source).await {
                    Ok(d) => d,
                    Err(e) => return Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
                };

                let matching = deps.iter().find(|d| {
                    (d.source_task_id == source && d.target_task_id == target)
                        || (d.source_task_id == target && d.target_task_id == source)
                });

                match matching {
                    Some(dep) => match TaskDependency::delete(&self.pool, dep.id).await {
                        Ok(_) => Ok(success_json(&serde_json::json!({
                            "success": true,
                            "message": "Dependency removed",
                        }))),
                        Err(e) => Ok(error_result("Failed to delete dependency", Some(&e.to_string()))),
                    },
                    None => Ok(error_result("No matching dependency found", None)),
                }
            }
            "list" => {
                let task_uuid = match req.task_id.as_deref().map(|s| parse_uuid(s, "task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => {
                        // Fall back to listing all project dependencies
                        match TaskDependency::list_by_project(&self.pool, project_uuid).await {
                            Ok(deps) => {
                                let items: Vec<Value> = deps
                                    .iter()
                                    .map(|d| serde_json::json!({
                                        "id": d.id.to_string(),
                                        "source_task_id": d.source_task_id.to_string(),
                                        "target_task_id": d.target_task_id.to_string(),
                                        "dependency_type": format!("{:?}", d.dependency_type).to_lowercase(),
                                        "created_at": d.created_at.to_rfc3339(),
                                    }))
                                    .collect();
                                return Ok(success_json(&serde_json::json!({
                                    "success": true,
                                    "project_id": req.project_id,
                                    "count": items.len(),
                                    "dependencies": items,
                                })));
                            }
                            Err(e) => return Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
                        }
                    }
                };

                match TaskDependency::list_by_task(&self.pool, task_uuid).await {
                    Ok(deps) => {
                        let items: Vec<Value> = deps
                            .iter()
                            .map(|d| serde_json::json!({
                                "id": d.id.to_string(),
                                "source_task_id": d.source_task_id.to_string(),
                                "target_task_id": d.target_task_id.to_string(),
                                "dependency_type": format!("{:?}", d.dependency_type).to_lowercase(),
                                "created_at": d.created_at.to_rfc3339(),
                            }))
                            .collect();
                        Ok(success_json(&serde_json::json!({
                            "success": true,
                            "task_id": req.task_id,
                            "count": items.len(),
                            "dependencies": items,
                        })))
                    }
                    Err(e) => Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
                }
            }
            _ => Ok(error_result("Invalid action. Use 'add', 'remove', or 'list'", None)),
        }
    }

    // ═════════════════════════════════════════════════════════════════════════
    // PHASE 3 — Knowledge Management
    // ═════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Add or update a knowledge source for a project. Used to track what information the project has ingested."
    )]
    async fn add_knowledge(
        &self,
        Parameters(req): Parameters<AddKnowledgeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let source_type: KnowledgeSourceType = match req.source_type.parse() {
            Ok(st) => st,
            Err(_) => return Ok(error_result(
                "Invalid source_type. Valid: conversation, artifact, pulse_content, context_injection, entity, topology_snapshot",
                None,
            )),
        };

        let coverage = req.coverage_score.unwrap_or(0.5).clamp(0.0, 1.0);

        match ProjectKnowledgeSource::upsert_source(
            &self.pool,
            project_uuid,
            &source_type,
            &req.source_id,
            &req.source_title,
            req.source_summary.as_deref(),
            coverage,
        )
        .await
        {
            Ok(()) => Ok(success_json(&serde_json::json!({
                "success": true,
                "message": "Knowledge source added/updated",
                "project_id": req.project_id,
                "source_type": req.source_type,
                "source_id": req.source_id,
            }))),
            Err(e) => Ok(error_result("Failed to add knowledge source", Some(&e.to_string()))),
        }
    }

    #[tool(description = "List knowledge sources for a project, optionally filtered by type.")]
    async fn list_knowledge(
        &self,
        Parameters(req): Parameters<ListKnowledgeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let include_stale = req.include_stale.unwrap_or(false);

        match ProjectKnowledgeSource::find_by_project(&self.pool, project_uuid).await {
            Ok(sources) => {
                let filtered: Vec<Value> = sources
                    .iter()
                    .filter(|s| {
                        if !include_stale && s.is_stale {
                            return false;
                        }
                        if let Some(ref st) = req.source_type {
                            if s.source_type != *st {
                                return false;
                            }
                        }
                        true
                    })
                    .map(|s| serde_json::json!({
                        "id": s.id.to_string(),
                        "source_type": s.source_type,
                        "source_id": s.source_id,
                        "source_title": s.source_title,
                        "source_summary": s.source_summary,
                        "coverage_score": s.coverage_score,
                        "is_stale": s.is_stale,
                        "last_refreshed_at": s.last_refreshed_at.to_rfc3339(),
                        "created_at": s.created_at.to_rfc3339(),
                    }))
                    .collect();

                Ok(success_json(&serde_json::json!({
                    "success": true,
                    "project_id": req.project_id,
                    "count": filtered.len(),
                    "sources": filtered,
                })))
            }
            Err(e) => Ok(error_result("Failed to list knowledge sources", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Get knowledge completeness metrics for a project — total sources, freshness, coverage, and completeness score."
    )]
    async fn get_knowledge_completeness(
        &self,
        Parameters(req): Parameters<GetKnowledgeCompletenessRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match ProjectKnowledgeSource::get_completeness(&self.pool, project_uuid).await {
            Ok(Some(kc)) => Ok(success_json(&serde_json::json!({
                "success": true,
                "project_id": req.project_id,
                "total_sources": kc.total_sources,
                "fresh_sources": kc.fresh_sources,
                "avg_coverage": kc.avg_coverage,
                "type_count": kc.type_count,
                "knowledge_completeness": kc.knowledge_completeness,
            }))),
            Ok(None) => Ok(success_json(&serde_json::json!({
                "success": true,
                "project_id": req.project_id,
                "total_sources": 0,
                "fresh_sources": 0,
                "avg_coverage": 0.0,
                "type_count": 0,
                "knowledge_completeness": 0.0,
            }))),
            Err(e) => Ok(error_result("Failed to get knowledge completeness", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Manage a knowledge source: mark_stale, mark_refreshed, or delete. `source_id` is the knowledge source UUID."
    )]
    async fn manage_knowledge(
        &self,
        Parameters(req): Parameters<ManageKnowledgeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let _project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let source_uuid = match parse_uuid(&req.source_id, "source_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match req.action.as_str() {
            "mark_stale" => {
                match ProjectKnowledgeSource::mark_stale(&self.pool, source_uuid).await {
                    Ok(()) => Ok(success_json(&serde_json::json!({
                        "success": true,
                        "message": "Knowledge source marked as stale",
                    }))),
                    Err(e) => Ok(error_result("Failed to mark stale", Some(&e.to_string()))),
                }
            }
            "mark_refreshed" => {
                match ProjectKnowledgeSource::mark_refreshed(&self.pool, source_uuid).await {
                    Ok(()) => Ok(success_json(&serde_json::json!({
                        "success": true,
                        "message": "Knowledge source marked as refreshed",
                    }))),
                    Err(e) => Ok(error_result("Failed to mark refreshed", Some(&e.to_string()))),
                }
            }
            "delete" => {
                let id_bytes = source_uuid.as_bytes().to_vec();
                match sqlx::query("DELETE FROM project_knowledge_sources WHERE id = ?")
                    .bind(&id_bytes)
                    .execute(&self.pool)
                    .await
                {
                    Ok(r) => {
                        if r.rows_affected() > 0 {
                            Ok(success_json(&serde_json::json!({
                                "success": true,
                                "message": "Knowledge source deleted",
                            })))
                        } else {
                            Ok(error_result("Knowledge source not found", None))
                        }
                    }
                    Err(e) => Ok(error_result("Failed to delete knowledge source", Some(&e.to_string()))),
                }
            }
            _ => Ok(error_result("Invalid action. Use 'mark_stale', 'mark_refreshed', or 'delete'", None)),
        }
    }

    // ═════════════════════════════════════════════════════════════════════════
    // PHASE 4 — Topology, VIBE Budget, Agents
    // ═════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Get the topology graph (nodes, edges, clusters) for a project. Optionally filter by node/edge type."
    )]
    async fn get_topology(
        &self,
        Parameters(req): Parameters<GetTopologyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let active_only = req.active_only.unwrap_or(true);
        let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

        // Fetch nodes
        let nodes_query = if let Some(ref nt) = req.node_type {
            format!(
                "SELECT id, project_id, node_type, ref_id, label, capabilities, status, metadata, weight, created_at, updated_at \
                 FROM topology_nodes WHERE project_id = '{}' AND node_type = '{}' {} ORDER BY created_at",
                pid_hex, nt, if active_only { "AND status = 'active'" } else { "" }
            )
        } else {
            format!(
                "SELECT id, project_id, node_type, ref_id, label, capabilities, status, metadata, weight, created_at, updated_at \
                 FROM topology_nodes WHERE project_id = '{}' {} ORDER BY created_at",
                pid_hex, if active_only { "AND status = 'active'" } else { "" }
            )
        };

        #[derive(Debug, sqlx::FromRow)]
        struct NodeRow {
            id: String,
            node_type: String,
            ref_id: Option<String>,
            label: String,
            capabilities: Option<String>,
            status: String,
            weight: Option<f64>,
        }

        let node_rows = sqlx::query_as::<_, NodeRow>(&nodes_query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let nodes_json: Vec<Value> = node_rows
            .iter()
            .map(|n| serde_json::json!({
                "id": n.id,
                "node_type": n.node_type,
                "ref_id": n.ref_id,
                "label": n.label,
                "capabilities": n.capabilities.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "status": n.status,
                "weight": n.weight,
            }))
            .collect();

        // Fetch edges
        let edges_query = if let Some(ref et) = req.edge_type {
            format!(
                "SELECT id, from_node_id, to_node_id, edge_type, weight, status, metadata \
                 FROM topology_edges WHERE project_id = '{}' AND edge_type = '{}' {} ORDER BY created_at",
                pid_hex, et, if active_only { "AND status = 'active'" } else { "" }
            )
        } else {
            format!(
                "SELECT id, from_node_id, to_node_id, edge_type, weight, status, metadata \
                 FROM topology_edges WHERE project_id = '{}' {} ORDER BY created_at",
                pid_hex, if active_only { "AND status = 'active'" } else { "" }
            )
        };

        #[derive(Debug, sqlx::FromRow)]
        struct EdgeRow {
            id: String,
            from_node_id: String,
            to_node_id: String,
            edge_type: String,
            weight: Option<f64>,
            status: String,
        }

        let edge_rows = sqlx::query_as::<_, EdgeRow>(&edges_query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let edges_json: Vec<Value> = edge_rows
            .iter()
            .map(|e| serde_json::json!({
                "id": e.id,
                "from_node_id": e.from_node_id,
                "to_node_id": e.to_node_id,
                "edge_type": e.edge_type,
                "weight": e.weight,
                "status": e.status,
            }))
            .collect();

        // Fetch clusters
        let clusters_query = format!(
            "SELECT id, name, purpose, node_ids, leader_node_id, is_active \
             FROM topology_clusters WHERE project_id = '{}' {}",
            pid_hex,
            if active_only { "AND is_active = 1" } else { "" }
        );

        #[derive(Debug, sqlx::FromRow)]
        struct ClusterRow {
            id: String,
            name: String,
            purpose: Option<String>,
            node_ids: Option<String>,
            leader_node_id: Option<String>,
            is_active: bool,
        }

        let cluster_rows = sqlx::query_as::<_, ClusterRow>(&clusters_query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let clusters_json: Vec<Value> = cluster_rows
            .iter()
            .map(|c| serde_json::json!({
                "id": c.id,
                "name": c.name,
                "purpose": c.purpose,
                "node_ids": c.node_ids.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "leader_node_id": c.leader_node_id,
                "is_active": c.is_active,
            }))
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "node_count": nodes_json.len(),
            "edge_count": edges_json.len(),
            "cluster_count": clusters_json.len(),
            "nodes": nodes_json,
            "edges": edges_json,
            "clusters": clusters_json,
        })))
    }

    #[tool(
        description = "Get topology issues (bottlenecks, holes, orphans, etc.) for a project. Optionally filter by severity."
    )]
    async fn get_topology_issues(
        &self,
        Parameters(req): Parameters<GetTopologyIssuesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let include_resolved = req.include_resolved.unwrap_or(false);
        let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

        let mut query = format!(
            "SELECT id, issue_type, severity, affected_nodes, affected_edges, description, \
             suggested_action, resolved_at, resolution_notes, created_at \
             FROM topology_issues WHERE project_id = '{}'",
            pid_hex
        );

        if !include_resolved {
            query.push_str(" AND resolved_at IS NULL");
        }
        if let Some(ref sev) = req.severity {
            query.push_str(&format!(" AND severity = '{}'", sev));
        }
        query.push_str(" ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'error' THEN 1 WHEN 'warning' THEN 2 ELSE 3 END, created_at DESC");

        #[derive(Debug, sqlx::FromRow)]
        struct IssueRow {
            id: String,
            issue_type: String,
            severity: String,
            affected_nodes: Option<String>,
            affected_edges: Option<String>,
            description: String,
            suggested_action: Option<String>,
            resolved_at: Option<String>,
            created_at: String,
        }

        let rows = sqlx::query_as::<_, IssueRow>(&query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let issues: Vec<Value> = rows
            .iter()
            .map(|i| serde_json::json!({
                "id": i.id,
                "issue_type": i.issue_type,
                "severity": i.severity,
                "affected_nodes": i.affected_nodes.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "affected_edges": i.affected_edges.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "description": i.description,
                "suggested_action": i.suggested_action,
                "resolved_at": i.resolved_at,
                "created_at": i.created_at,
            }))
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "count": issues.len(),
            "issues": issues,
        })))
    }

    #[tool(
        description = "Find the shortest path between two topology nodes using BFS. Returns the path nodes, edges, and total weight."
    )]
    async fn find_topology_path(
        &self,
        Parameters(req): Parameters<FindTopologyPathRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

        // Load active edges
        #[derive(Debug, sqlx::FromRow, Clone)]
        struct EdgeRow {
            id: String,
            from_node_id: String,
            to_node_id: String,
            edge_type: String,
            weight: Option<f64>,
        }

        let edges = sqlx::query_as::<_, EdgeRow>(&format!(
            "SELECT id, from_node_id, to_node_id, edge_type, weight \
             FROM topology_edges WHERE project_id = '{}' AND status = 'active'",
            pid_hex
        ))
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Build adjacency list
        let mut adj: HashMap<String, Vec<(String, &EdgeRow)>> = HashMap::new();
        for e in &edges {
            adj.entry(e.from_node_id.clone())
                .or_default()
                .push((e.to_node_id.clone(), e));
            // Treat as undirected for path-finding
            adj.entry(e.to_node_id.clone())
                .or_default()
                .push((e.from_node_id.clone(), e));
        }

        // BFS
        let from = &req.from_node_id;
        let to = &req.to_node_id;

        let mut visited: HashMap<String, (String, String)> = HashMap::new(); // node -> (prev_node, edge_id)
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        queue.push_back(from.clone());
        visited.insert(from.clone(), ("".to_string(), "".to_string()));

        let mut found = false;
        while let Some(current) = queue.pop_front() {
            if current == *to {
                found = true;
                break;
            }
            if let Some(neighbors) = adj.get(&current) {
                for (next, edge) in neighbors {
                    if !visited.contains_key(next) {
                        visited.insert(next.clone(), (current.clone(), edge.id.clone()));
                        queue.push_back(next.clone());
                    }
                }
            }
        }

        if !found {
            return Ok(success_json(&serde_json::json!({
                "success": true,
                "path_found": false,
                "message": "No path found between the specified nodes",
                "from_node_id": from,
                "to_node_id": to,
            })));
        }

        // Reconstruct path
        let mut path_nodes: Vec<String> = Vec::new();
        let mut path_edges: Vec<String> = Vec::new();
        let mut total_weight: f64 = 0.0;
        let mut current = to.clone();

        while current != *from {
            path_nodes.push(current.clone());
            let (prev, edge_id) = &visited[&current];
            path_edges.push(edge_id.clone());
            if let Some(edge) = edges.iter().find(|e| e.id == *edge_id) {
                total_weight += edge.weight.unwrap_or(1.0);
            }
            current = prev.clone();
        }
        path_nodes.push(from.clone());
        path_nodes.reverse();
        path_edges.reverse();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "path_found": true,
            "from_node_id": from,
            "to_node_id": to,
            "path_length": path_nodes.len(),
            "total_weight": total_weight,
            "path_nodes": path_nodes,
            "path_edges": path_edges,
        })))
    }

    #[tool(
        description = "Get VIBE budget information for a project — limit, spent, remaining, and whether budget is available."
    )]
    async fn get_vibe_budget(
        &self,
        Parameters(req): Parameters<GetVibeBudgetRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Project::find_by_id(&self.pool, project_uuid).await {
            Ok(Some(project)) => {
                let remaining = project.remaining_vibe();
                let has_budget = project.has_vibe_budget(0);

                Ok(success_json(&serde_json::json!({
                    "success": true,
                    "project_id": req.project_id,
                    "project_name": project.name,
                    "vibe_budget_limit": project.vibe_budget_limit,
                    "vibe_spent_amount": project.vibe_spent_amount,
                    "vibe_remaining": remaining,
                    "has_budget": has_budget,
                    "is_unlimited": project.vibe_budget_limit.is_none(),
                    "currency_note": "1 VIBE = $0.01 USD",
                })))
            }
            Ok(None) => Ok(error_result("Project not found", None)),
            Err(e) => Ok(error_result("Failed to retrieve project", Some(&e.to_string()))),
        }
    }

    #[tool(description = "List registered AI agents. Optionally filter by status.")]
    async fn list_agents(
        &self,
        Parameters(req): Parameters<ListAgentsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let agents = if let Some(ref status_str) = req.status {
            let status = match status_str.to_lowercase().as_str() {
                "active" => AgentStatus::Active,
                "inactive" => AgentStatus::Inactive,
                "maintenance" => AgentStatus::Maintenance,
                "training" => AgentStatus::Training,
                _ => return Ok(error_result(
                    "Invalid status. Valid: active, inactive, maintenance, training",
                    None,
                )),
            };
            match if status == AgentStatus::Active {
                Agent::find_active(&self.pool).await
            } else {
                Agent::find_all(&self.pool).await.map(|agents| {
                    agents
                        .into_iter()
                        .filter(|a| a.status == status)
                        .collect()
                })
            } {
                Ok(a) => a,
                Err(e) => return Ok(error_result("Failed to list agents", Some(&e.to_string()))),
            }
        } else {
            match Agent::find_all(&self.pool).await {
                Ok(a) => a,
                Err(e) => return Ok(error_result("Failed to list agents", Some(&e.to_string()))),
            }
        };

        let agent_list: Vec<Value> = agents
            .iter()
            .map(|a| {
                let capabilities: Option<Vec<String>> =
                    a.capabilities.as_deref().and_then(|s| serde_json::from_str(s).ok());
                let tools: Option<Vec<String>> =
                    a.tools.as_deref().and_then(|s| serde_json::from_str(s).ok());

                serde_json::json!({
                    "id": a.id.to_string(),
                    "short_name": a.short_name,
                    "designation": a.designation,
                    "description": a.description,
                    "status": format!("{:?}", a.status).to_lowercase(),
                    "autonomy_level": format!("{:?}", a.autonomy_level).to_lowercase(),
                    "default_model": a.default_model,
                    "capabilities": capabilities,
                    "tools": tools,
                    "tasks_completed": a.tasks_completed,
                    "tasks_failed": a.tasks_failed,
                    "average_rating": a.average_rating,
                    "avatar_url": a.avatar_url,
                    "agent_tier": a.agent_tier,
                })
            })
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "count": agent_list.len(),
            "agents": agent_list,
        })))
    }
}

// ─── MCP Resources ──────────────────────────────────────────────────────────

/// Build the 6 MCP resource templates
fn pcg_resource_templates() -> Vec<ResourceTemplate> {
    let templates = vec![
        ("pcg://projects/{project_id}", "Project Detail", "Get project details including VIBE budget and configuration"),
        ("pcg://projects/{project_id}/tasks", "Project Tasks", "List all tasks in a project"),
        ("pcg://projects/{project_id}/tasks/{task_id}", "Task Detail", "Get detailed information about a specific task"),
        ("pcg://projects/{project_id}/knowledge", "Project Knowledge", "List knowledge sources for a project"),
        ("pcg://projects/{project_id}/topology", "Project Topology", "Get the topology graph (nodes, edges, clusters)"),
        ("pcg://projects/{project_id}/health", "Project Health", "Get health summary including issues and knowledge completeness"),
    ];

    templates
        .into_iter()
        .map(|(uri, name, desc)| {
            Annotated::new(RawResourceTemplate {
                uri_template: uri.to_string(),
                name: name.to_string(),
                description: Some(desc.to_string()),
                mime_type: Some("application/json".to_string()),
            }, None)
        })
        .collect()
}

#[tool_handler]
impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "pcg-dashboard-mcp".to_string(),
                version: "2.0.0".to_string(),
            },
            instructions: Some(
                "PCG Dashboard MCP v2 — Atlas-level project management for AI agents. \
                 20 tools + 6 MCP resources. Use `list_projects` to discover project IDs. \
                 Tools: list_projects, list_tasks, create_task, get_task, update_task, delete_task, \
                 evaluate_policy, bulk_create_tasks, bulk_update_tasks, search_tasks, \
                 manage_task_dependencies, add_knowledge, list_knowledge, get_knowledge_completeness, \
                 manage_knowledge, get_topology, get_topology_issues, find_topology_path, \
                 get_vibe_budget, list_agents. \
                 Resources: pcg://projects/{project_id}[/tasks|/knowledge|/topology|/health]"
                    .to_string(),
            ),
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, ErrorData>> + Send + '_
    {
        std::future::ready(Ok(ListResourceTemplatesResult {
            resource_templates: pcg_resource_templates(),
            next_cursor: None,
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_
    {
        let pool = self.pool.clone();
        let uri = request.uri.clone();

        async move {
            let content = match resolve_resource(&pool, &uri).await {
                Ok(json) => json,
                Err(msg) => serde_json::json!({ "error": msg }),
            };

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string_pretty(&content).unwrap_or_default(),
                    uri,
                )],
            })
        }
    }
}

/// Resolve a pcg:// resource URI to JSON
async fn resolve_resource(pool: &SqlitePool, uri: &str) -> Result<Value, String> {
    // Parse: pcg://projects/{project_id}[/tasks[/{task_id}]|/knowledge|/topology|/health]
    let path = uri
        .strip_prefix("pcg://projects/")
        .ok_or_else(|| format!("Unknown resource URI: {}", uri))?;

    let parts: Vec<&str> = path.splitn(3, '/').collect();
    let project_uuid = Uuid::parse_str(parts[0])
        .map_err(|_| "Invalid project_id in URI".to_string())?;

    let project = Project::find_by_id(pool, project_uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    if parts.len() == 1 {
        // Project detail
        return Ok(serde_json::json!({
            "id": project.id.to_string(),
            "name": project.name,
            "git_repo_path": project.git_repo_path.to_string_lossy(),
            "vibe_budget_limit": project.vibe_budget_limit,
            "vibe_spent_amount": project.vibe_spent_amount,
            "vibe_remaining": project.remaining_vibe(),
            "setup_script": project.setup_script,
            "dev_script": project.dev_script,
            "cleanup_script": project.cleanup_script,
            "created_at": project.created_at.to_rfc3339(),
            "updated_at": project.updated_at.to_rfc3339(),
        }));
    }

    match parts[1] {
        "tasks" => {
            if parts.len() == 3 {
                // Single task detail
                let task_uuid = Uuid::parse_str(parts[2])
                    .map_err(|_| "Invalid task_id in URI".to_string())?;
                let task = Task::find_by_id_and_project_id(pool, task_uuid, project_uuid)
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "Task not found".to_string())?;
                Ok(serde_json::to_value(task_to_summary(&task)).unwrap())
            } else {
                // Task list
                let tasks = Task::find_by_project_id_with_attempt_status(pool, project_uuid)
                    .await
                    .map_err(|e| e.to_string())?;
                let summaries: Vec<Value> = tasks
                    .iter()
                    .map(|t| serde_json::to_value(task_with_status_to_summary(t)).unwrap())
                    .collect();
                Ok(serde_json::json!({
                    "project_id": project.id.to_string(),
                    "project_name": project.name,
                    "count": summaries.len(),
                    "tasks": summaries,
                }))
            }
        }
        "knowledge" => {
            let sources = ProjectKnowledgeSource::find_by_project(pool, project_uuid)
                .await
                .map_err(|e| e.to_string())?;
            let items: Vec<Value> = sources
                .iter()
                .map(|s| serde_json::json!({
                    "id": s.id.to_string(),
                    "source_type": s.source_type,
                    "source_id": s.source_id,
                    "source_title": s.source_title,
                    "source_summary": s.source_summary,
                    "coverage_score": s.coverage_score,
                    "is_stale": s.is_stale,
                }))
                .collect();
            Ok(serde_json::json!({
                "project_id": project.id.to_string(),
                "count": items.len(),
                "sources": items,
            }))
        }
        "topology" => {
            let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

            #[derive(sqlx::FromRow)]
            struct CountRow { cnt: i64 }

            let node_count: i64 = sqlx::query_as::<_, CountRow>(
                &format!("SELECT COUNT(*) as cnt FROM topology_nodes WHERE project_id = '{}' AND status = 'active'", pid_hex)
            )
            .fetch_one(pool)
            .await
            .map(|r| r.cnt)
            .unwrap_or(0);

            let edge_count: i64 = sqlx::query_as::<_, CountRow>(
                &format!("SELECT COUNT(*) as cnt FROM topology_edges WHERE project_id = '{}' AND status = 'active'", pid_hex)
            )
            .fetch_one(pool)
            .await
            .map(|r| r.cnt)
            .unwrap_or(0);

            let cluster_count: i64 = sqlx::query_as::<_, CountRow>(
                &format!("SELECT COUNT(*) as cnt FROM topology_clusters WHERE project_id = '{}' AND is_active = 1", pid_hex)
            )
            .fetch_one(pool)
            .await
            .map(|r| r.cnt)
            .unwrap_or(0);

            Ok(serde_json::json!({
                "project_id": project.id.to_string(),
                "active_nodes": node_count,
                "active_edges": edge_count,
                "active_clusters": cluster_count,
                "note": "Use get_topology tool for full graph data",
            }))
        }
        "health" => {
            let pid_bytes = project_uuid.as_bytes().to_vec();
            let health_map = ProjectKnowledgeSource::get_health_batch(pool, &[pid_bytes])
                .await
                .unwrap_or_default();

            let health = health_map
                .get(&project.id.to_string())
                .cloned();

            match health {
                Some(h) => Ok(serde_json::to_value(h).unwrap()),
                None => Ok(serde_json::json!({
                    "project_id": project.id.to_string(),
                    "health_status": "unknown",
                    "active_issues_count": 0,
                    "critical_issues": 0,
                    "warning_issues": 0,
                    "knowledge_completeness": 0.0,
                })),
            }
        }
        _ => Err(format!("Unknown resource path segment: {}", parts[1])),
    }
}

use std::path::PathBuf;

use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    #[schemars(
        description = "Structured completion criteria — measurable conditions that define when this task is done (e.g. 'All tests pass, coverage > 80%, PR approved')"
    )]
    pub completion_criteria: Option<String>,
    #[schemars(
        description = "Expected output format — what deliverables look like (e.g. 'Rust module with pub fn, unit tests, updated CHANGELOG')"
    )]
    pub output_format: Option<String>,
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
    #[schemars(
        description = "Sort by: created_at, updated_at, priority, due_date, title (default: created_at)"
    )]
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
    #[schemars(description = "Structured completion criteria for self-evaluation")]
    pub completion_criteria: Option<String>,
    #[schemars(description = "Expected output format/deliverables")]
    pub output_format: Option<String>,
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
    #[schemars(
        description = "New completion criteria — measurable conditions for task completion"
    )]
    pub completion_criteria: Option<String>,
    #[schemars(description = "New output format — expected deliverable format")]
    pub output_format: Option<String>,
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
    #[schemars(
        description = "The username or UUID of the user to assign the task to. Use 'unassign' to clear."
    )]
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
    #[schemars(
        description = "Optional: 'comment', 'status_update', 'review', 'system', 'handoff'. Default: 'comment'"
    )]
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckDependenciesRequest {
    #[schemars(description = "The task UUID to check dependencies for")]
    pub task_id: String,
    #[schemars(description = "Project UUID the task belongs to")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UnifiedSearchRequest {
    #[schemars(
        description = "Search query — matches against titles, names, descriptions across all entity types"
    )]
    pub query: String,
    #[schemars(
        description = "Entity types to search: 'projects', 'tasks', 'knowledge', 'persons'. Default: all"
    )]
    pub entity_types: Option<Vec<String>>,
    #[schemars(description = "Optional project UUID to scope task/knowledge search")]
    pub project_id: Option<String>,
    #[schemars(description = "Max results per entity type (default: 10, max: 50)")]
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScaffoldProjectRequest {
    #[schemars(description = "Project name")]
    pub name: String,
    #[schemars(
        description = "Template type: 'software', 'research', 'marketing', 'client_onboarding'"
    )]
    pub template: String,
    #[schemars(description = "Path to git repository (will be created if it doesn't exist)")]
    pub git_repo_path: String,
    #[schemars(description = "Optional organization UUID to associate with")]
    pub organization_id: Option<String>,
    #[schemars(description = "Optional client UUID to associate with")]
    pub client_id: Option<String>,
    #[schemars(
        description = "Optional topic/subject for research projects — used to generate contextual tasks"
    )]
    pub topic: Option<String>,
    #[schemars(
        description = "Optional client name for client onboarding — used to personalize task descriptions"
    )]
    pub client_name: Option<String>,
}

// ─── Phase 3: Knowledge Types ───────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddKnowledgeRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(
        description = "Source type: conversation, artifact, pulse_content, context_injection, entity, topology_snapshot"
    )]
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

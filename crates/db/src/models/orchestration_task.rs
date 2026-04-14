//! Orchestration Task Model
//!
//! Child entities of agent_flows for parallel task execution within a flow.
//! Mirrors demo_impl/src/Task.ts task types and statuses.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;

use crate::db_uuid::DbUuid;

// ── Task Type Enum ────────────────────────────────────────────────────────────

/// The type of orchestration task, determining how it executes.
/// Mirrors demo_impl TaskType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize, TS)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrchestrationTaskType {
    /// Execute a bash command locally
    LocalBash,
    /// Run a local AI agent (Claude Code, etc.)
    LocalAgent,
    /// Delegate to a remote agent via API
    RemoteAgent,
    /// Run as an in-process teammate (shared context)
    InProcessTeammate,
    /// Execute a predefined workflow
    LocalWorkflow,
    /// Monitor an MCP server for events
    MonitorMcp,
    /// Background/idle task that runs when nothing else is active
    Dream,
}

impl std::fmt::Display for OrchestrationTaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::LocalBash => "local_bash",
            Self::LocalAgent => "local_agent",
            Self::RemoteAgent => "remote_agent",
            Self::InProcessTeammate => "in_process_teammate",
            Self::LocalWorkflow => "local_workflow",
            Self::MonitorMcp => "monitor_mcp",
            Self::Dream => "dream",
        };
        write!(f, "{}", s)
    }
}

// ── Task Status Enum ──────────────────────────────────────────────────────────

/// The execution status of an orchestration task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize, TS)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrchestrationTaskStatus {
    /// Task is waiting to be executed
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was manually killed
    Killed,
}

impl OrchestrationTaskStatus {
    /// Returns true if this status is terminal (no more transitions possible)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Killed)
    }
}

impl std::fmt::Display for OrchestrationTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Killed => "killed",
        };
        write!(f, "{}", s)
    }
}

// ── Orchestration Task Struct ─────────────────────────────────────────────────

/// An orchestration task within an agent flow.
/// Multiple tasks can run concurrently based on their is_concurrency_safe flag.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrchestrationTask {
    pub id: String,
    pub agent_flow_id: String,
    pub task_type: OrchestrationTaskType,
    pub status: OrchestrationTaskStatus,
    pub description: String,
    /// The tool_use_id that spawned this task (for traceability)
    pub tool_use_id: Option<String>,
    /// When the task started executing (epoch ms)
    pub start_time_ms: Option<i64>,
    /// When the task finished (epoch ms)
    pub end_time_ms: Option<i64>,
    /// Task output on success
    pub output: Option<String>,
    /// Error message on failure
    pub error: Option<String>,
    /// Execution order within the flow
    pub position: i32,
    /// Whether this task can run concurrently with others
    pub is_concurrency_safe: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Parameters for creating a new orchestration task
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateOrchestrationTask {
    pub agent_flow_id: String,
    pub task_type: OrchestrationTaskType,
    pub description: String,
    pub tool_use_id: Option<String>,
    pub is_concurrency_safe: bool,
    pub position: i32,
}

impl OrchestrationTask {
    /// Create a new orchestration task
    pub async fn create(
        pool: &SqlitePool,
        params: CreateOrchestrationTask,
    ) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new().to_string();
        let task_type_str = params.task_type.to_string();

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO orchestration_tasks
                (id, agent_flow_id, task_type, description, tool_use_id, is_concurrency_safe, position)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&params.agent_flow_id)
        .bind(&task_type_str)
        .bind(&params.description)
        .bind(&params.tool_use_id)
        .bind(params.is_concurrency_safe as i32)
        .bind(params.position)
        .fetch_one(pool)
        .await
    }

    /// Find a task by ID
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM orchestration_tasks WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find all tasks for a flow, ordered by position
    pub async fn find_by_flow_id(
        pool: &SqlitePool,
        agent_flow_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM orchestration_tasks
            WHERE agent_flow_id = ?1
            ORDER BY position ASC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await
    }

    /// Find pending tasks for a flow, ordered by position
    pub async fn find_pending_by_flow_id(
        pool: &SqlitePool,
        agent_flow_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM orchestration_tasks
            WHERE agent_flow_id = ?1 AND status = 'pending'
            ORDER BY position ASC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await
    }

    /// Find running tasks for a flow
    pub async fn find_running_by_flow_id(
        pool: &SqlitePool,
        agent_flow_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM orchestration_tasks
            WHERE agent_flow_id = ?1 AND status = 'running'
            ORDER BY position ASC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await
    }

    /// Update task status, setting timestamps appropriately
    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: OrchestrationTaskStatus,
    ) -> Result<Self, sqlx::Error> {
        let status_str = status.to_string();
        let now_ms = chrono::Utc::now().timestamp_millis();

        // Set start_time_ms when transitioning to Running
        let start_time = if status == OrchestrationTaskStatus::Running {
            Some(now_ms)
        } else {
            None
        };

        // Set end_time_ms when reaching a terminal state
        let end_time = if status.is_terminal() {
            Some(now_ms)
        } else {
            None
        };

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET status = ?2,
                start_time_ms = COALESCE(?3, start_time_ms),
                end_time_ms = COALESCE(?4, end_time_ms),
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&status_str)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(pool)
        .await
    }

    /// Set task output (typically on completion)
    pub async fn set_output(
        pool: &SqlitePool,
        id: &str,
        output: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET output = ?2, updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(output)
        .fetch_one(pool)
        .await
    }

    /// Set task error and transition to failed status
    pub async fn set_error(pool: &SqlitePool, id: &str, error: &str) -> Result<Self, sqlx::Error> {
        let now_ms = chrono::Utc::now().timestamp_millis();

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET error = ?2,
                status = 'failed',
                end_time_ms = ?3,
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(error)
        .bind(now_ms)
        .fetch_one(pool)
        .await
    }

    /// Mark task as completed with output
    pub async fn complete(pool: &SqlitePool, id: &str, output: &str) -> Result<Self, sqlx::Error> {
        let now_ms = chrono::Utc::now().timestamp_millis();

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET output = ?2,
                status = 'completed',
                end_time_ms = ?3,
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(output)
        .bind(now_ms)
        .fetch_one(pool)
        .await
    }

    /// Kill a running task
    pub async fn kill(pool: &SqlitePool, id: &str) -> Result<Self, sqlx::Error> {
        let now_ms = chrono::Utc::now().timestamp_millis();

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET status = 'killed',
                end_time_ms = ?2,
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(now_ms)
        .fetch_one(pool)
        .await
    }

    /// Get duration in milliseconds (if task has started)
    pub fn duration_ms(&self) -> Option<i64> {
        match (self.start_time_ms, self.end_time_ms) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) => Some(chrono::Utc::now().timestamp_millis() - start),
            _ => None,
        }
    }

    /// Check if all tasks in a flow are complete (terminal state)
    pub async fn all_complete(pool: &SqlitePool, agent_flow_id: &str) -> Result<bool, sqlx::Error> {
        let incomplete_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM orchestration_tasks
            WHERE agent_flow_id = ?1 AND status NOT IN ('completed', 'failed', 'killed')
            "#,
        )
        .bind(agent_flow_id)
        .fetch_one(pool)
        .await?;

        Ok(incomplete_count == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_serialization() {
        assert_eq!(
            serde_json::to_string(&OrchestrationTaskType::LocalBash).unwrap(),
            "\"local_bash\""
        );
        assert_eq!(
            serde_json::to_string(&OrchestrationTaskType::InProcessTeammate).unwrap(),
            "\"in_process_teammate\""
        );
        assert_eq!(
            serde_json::to_string(&OrchestrationTaskType::MonitorMcp).unwrap(),
            "\"monitor_mcp\""
        );
    }

    #[test]
    fn test_task_type_deserialization() {
        let parsed: OrchestrationTaskType = serde_json::from_str("\"remote_agent\"").unwrap();
        assert_eq!(parsed, OrchestrationTaskType::RemoteAgent);

        let parsed: OrchestrationTaskType = serde_json::from_str("\"local_workflow\"").unwrap();
        assert_eq!(parsed, OrchestrationTaskType::LocalWorkflow);
    }

    #[test]
    fn test_task_type_display() {
        assert_eq!(OrchestrationTaskType::LocalBash.to_string(), "local_bash");
        assert_eq!(OrchestrationTaskType::Dream.to_string(), "dream");
    }

    #[test]
    fn test_status_is_terminal() {
        assert!(!OrchestrationTaskStatus::Pending.is_terminal());
        assert!(!OrchestrationTaskStatus::Running.is_terminal());
        assert!(OrchestrationTaskStatus::Completed.is_terminal());
        assert!(OrchestrationTaskStatus::Failed.is_terminal());
        assert!(OrchestrationTaskStatus::Killed.is_terminal());
    }

    #[test]
    fn test_status_serialization() {
        assert_eq!(
            serde_json::to_string(&OrchestrationTaskStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&OrchestrationTaskStatus::Running).unwrap(),
            "\"running\""
        );
    }

    #[test]
    fn test_status_display() {
        assert_eq!(OrchestrationTaskStatus::Pending.to_string(), "pending");
        assert_eq!(OrchestrationTaskStatus::Completed.to_string(), "completed");
    }
}

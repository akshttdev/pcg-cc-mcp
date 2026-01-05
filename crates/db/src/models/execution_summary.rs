use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionSummaryError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Execution summary not found")]
    NotFound,
    #[error("Failed to create execution summary: {0}")]
    CreateFailed(String),
    #[error("Invalid rating: must be between 1 and 5")]
    InvalidRating,
}

/// Completion status of an agent execution
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "completion_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CompletionStatus {
    /// Agent finished with full confidence
    Full,
    /// Agent made progress but flagged for review
    Partial,
    /// Agent needs human input to continue
    Blocked,
    /// Agent hit unrecoverable error
    Failed,
}

impl Default for CompletionStatus {
    fn default() -> Self {
        Self::Full
    }
}

/// Structured summary of what an agent accomplished during execution
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionSummary {
    pub id: Uuid,
    pub task_attempt_id: Uuid,
    pub execution_process_id: Option<Uuid>,

    // What was accomplished
    pub files_modified: i32,
    pub files_created: i32,
    pub files_deleted: i32,
    pub commands_run: i32,
    pub commands_failed: i32,
    /// JSON array of tool names used during execution
    pub tools_used: Option<String>,

    // Outcome tracking
    pub completion_status: CompletionStatus,
    pub blocker_summary: Option<String>,
    pub error_summary: Option<String>,

    // Timing
    pub execution_time_ms: i64,

    // Human feedback
    pub human_rating: Option<i32>,
    pub human_notes: Option<String>,
    pub is_reference_example: bool,

    // Workflow tagging
    pub workflow_tags: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new execution summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateExecutionSummary {
    pub task_attempt_id: Uuid,
    pub execution_process_id: Option<Uuid>,

    pub files_modified: i32,
    pub files_created: i32,
    pub files_deleted: i32,
    pub commands_run: i32,
    pub commands_failed: i32,
    pub tools_used: Option<Vec<String>>,

    pub completion_status: CompletionStatus,
    pub blocker_summary: Option<String>,
    pub error_summary: Option<String>,

    pub execution_time_ms: i64,
    pub workflow_tags: Option<Vec<String>>,
}

/// Data for updating human feedback on an execution summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateExecutionSummaryFeedback {
    pub human_rating: Option<i32>,
    pub human_notes: Option<String>,
    pub is_reference_example: Option<bool>,
}

/// Lightweight summary for display in task cards
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExecutionSummaryBrief {
    pub files_modified: i32,
    pub files_created: i32,
    pub commands_failed: i32,
    pub completion_status: CompletionStatus,
    pub tools_used: Option<Vec<String>>,
    pub human_rating: Option<i32>,
    pub execution_time_ms: i64,
}

impl ExecutionSummary {
    /// Find execution summary by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ExecutionSummaryError> {
        let summary = sqlx::query_as!(
            ExecutionSummary,
            r#"SELECT
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                execution_process_id as "execution_process_id: Uuid",
                files_modified as "files_modified!: i32",
                files_created as "files_created!: i32",
                files_deleted as "files_deleted!: i32",
                commands_run as "commands_run!: i32",
                commands_failed as "commands_failed!: i32",
                tools_used,
                completion_status as "completion_status!: CompletionStatus",
                blocker_summary,
                error_summary,
                execution_time_ms as "execution_time_ms!: i64",
                human_rating as "human_rating: i32",
                human_notes,
                is_reference_example as "is_reference_example!: bool",
                workflow_tags,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM execution_summaries
            WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(summary)
    }

    /// Find the latest execution summary for a task attempt
    pub async fn find_by_task_attempt_id(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Option<Self>, ExecutionSummaryError> {
        let summary = sqlx::query_as!(
            ExecutionSummary,
            r#"SELECT
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                execution_process_id as "execution_process_id: Uuid",
                files_modified as "files_modified!: i32",
                files_created as "files_created!: i32",
                files_deleted as "files_deleted!: i32",
                commands_run as "commands_run!: i32",
                commands_failed as "commands_failed!: i32",
                tools_used,
                completion_status as "completion_status!: CompletionStatus",
                blocker_summary,
                error_summary,
                execution_time_ms as "execution_time_ms!: i64",
                human_rating as "human_rating: i32",
                human_notes,
                is_reference_example as "is_reference_example!: bool",
                workflow_tags,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM execution_summaries
            WHERE task_attempt_id = ?
            ORDER BY created_at DESC
            LIMIT 1"#,
            task_attempt_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(summary)
    }

    /// Find all execution summaries for a task attempt
    pub async fn find_all_by_task_attempt_id(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionSummaryError> {
        let summaries = sqlx::query_as!(
            ExecutionSummary,
            r#"SELECT
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                execution_process_id as "execution_process_id: Uuid",
                files_modified as "files_modified!: i32",
                files_created as "files_created!: i32",
                files_deleted as "files_deleted!: i32",
                commands_run as "commands_run!: i32",
                commands_failed as "commands_failed!: i32",
                tools_used,
                completion_status as "completion_status!: CompletionStatus",
                blocker_summary,
                error_summary,
                execution_time_ms as "execution_time_ms!: i64",
                human_rating as "human_rating: i32",
                human_notes,
                is_reference_example as "is_reference_example!: bool",
                workflow_tags,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM execution_summaries
            WHERE task_attempt_id = ?
            ORDER BY created_at DESC"#,
            task_attempt_id
        )
        .fetch_all(pool)
        .await?;

        Ok(summaries)
    }

    /// Create a new execution summary
    pub async fn create(
        pool: &SqlitePool,
        data: CreateExecutionSummary,
    ) -> Result<Self, ExecutionSummaryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let tools_used_json = data
            .tools_used
            .map(|t| serde_json::to_string(&t).unwrap_or_default());
        let workflow_tags_json = data
            .workflow_tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default());

        let summary = sqlx::query_as!(
            ExecutionSummary,
            r#"INSERT INTO execution_summaries (
                id, task_attempt_id, execution_process_id,
                files_modified, files_created, files_deleted,
                commands_run, commands_failed, tools_used,
                completion_status, blocker_summary, error_summary,
                execution_time_ms, workflow_tags,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                execution_process_id as "execution_process_id: Uuid",
                files_modified as "files_modified!: i32",
                files_created as "files_created!: i32",
                files_deleted as "files_deleted!: i32",
                commands_run as "commands_run!: i32",
                commands_failed as "commands_failed!: i32",
                tools_used,
                completion_status as "completion_status!: CompletionStatus",
                blocker_summary,
                error_summary,
                execution_time_ms as "execution_time_ms!: i64",
                human_rating as "human_rating: i32",
                human_notes,
                is_reference_example as "is_reference_example!: bool",
                workflow_tags,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.task_attempt_id,
            data.execution_process_id,
            data.files_modified,
            data.files_created,
            data.files_deleted,
            data.commands_run,
            data.commands_failed,
            tools_used_json,
            data.completion_status,
            data.blocker_summary,
            data.error_summary,
            data.execution_time_ms,
            workflow_tags_json,
            now,
            now
        )
        .fetch_one(pool)
        .await?;

        Ok(summary)
    }

    /// Update human feedback on an execution summary
    pub async fn update_feedback(
        pool: &SqlitePool,
        id: Uuid,
        feedback: UpdateExecutionSummaryFeedback,
    ) -> Result<(), ExecutionSummaryError> {
        // Validate rating if provided
        if let Some(rating) = feedback.human_rating {
            if !(1..=5).contains(&rating) {
                return Err(ExecutionSummaryError::InvalidRating);
            }
        }

        let now = Utc::now();

        sqlx::query!(
            r#"UPDATE execution_summaries SET
                human_rating = COALESCE(?, human_rating),
                human_notes = COALESCE(?, human_notes),
                is_reference_example = COALESCE(?, is_reference_example),
                updated_at = ?
            WHERE id = ?"#,
            feedback.human_rating,
            feedback.human_notes,
            feedback.is_reference_example,
            now,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get a brief summary for task card display
    pub fn to_brief(&self) -> ExecutionSummaryBrief {
        let tools_used = self
            .tools_used
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok());

        ExecutionSummaryBrief {
            files_modified: self.files_modified,
            files_created: self.files_created,
            commands_failed: self.commands_failed,
            completion_status: self.completion_status.clone(),
            tools_used,
            human_rating: self.human_rating,
            execution_time_ms: self.execution_time_ms,
        }
    }

    /// Find reference examples for workflow learning
    pub async fn find_reference_examples(
        pool: &SqlitePool,
        limit: i32,
    ) -> Result<Vec<Self>, ExecutionSummaryError> {
        let summaries = sqlx::query_as!(
            ExecutionSummary,
            r#"SELECT
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                execution_process_id as "execution_process_id: Uuid",
                files_modified as "files_modified!: i32",
                files_created as "files_created!: i32",
                files_deleted as "files_deleted!: i32",
                commands_run as "commands_run!: i32",
                commands_failed as "commands_failed!: i32",
                tools_used,
                completion_status as "completion_status!: CompletionStatus",
                blocker_summary,
                error_summary,
                execution_time_ms as "execution_time_ms!: i64",
                human_rating as "human_rating: i32",
                human_notes,
                is_reference_example as "is_reference_example!: bool",
                workflow_tags,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM execution_summaries
            WHERE is_reference_example = TRUE
            ORDER BY human_rating DESC, created_at DESC
            LIMIT ?"#,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(summaries)
    }

    /// Delete execution summary by task attempt ID
    pub async fn delete_by_task_attempt_id(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<(), ExecutionSummaryError> {
        sqlx::query!(
            "DELETE FROM execution_summaries WHERE task_attempt_id = ?",
            task_attempt_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Collaborator information for task display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskCollaborator {
    pub actor_id: String,
    pub actor_type: String, // "human", "agent", "mcp", "system"
    pub last_action: String,
    pub last_action_at: DateTime<Utc>,
}

impl TaskCollaborator {
    /// Parse collaborators from JSON string
    pub fn parse_from_json(json: Option<&str>) -> Vec<Self> {
        json.and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Serialize collaborators to JSON string
    pub fn to_json(collaborators: &[Self]) -> Option<String> {
        if collaborators.is_empty() {
            None
        } else {
            serde_json::to_string(collaborators).ok()
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionCheckpointError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Checkpoint not found")]
    NotFound,
    #[error("Checkpoint already reviewed")]
    AlreadyReviewed,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "checkpoint_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CheckpointStatus {
    Pending,
    Approved,
    Rejected,
    AutoApproved,
    Skipped,
    Expired,
}

impl std::fmt::Display for CheckpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointStatus::Pending => write!(f, "pending"),
            CheckpointStatus::Approved => write!(f, "approved"),
            CheckpointStatus::Rejected => write!(f, "rejected"),
            CheckpointStatus::AutoApproved => write!(f, "auto_approved"),
            CheckpointStatus::Skipped => write!(f, "skipped"),
            CheckpointStatus::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionCheckpoint {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub checkpoint_definition_id: Option<Uuid>,
    pub checkpoint_data: String, // JSON
    pub trigger_reason: Option<String>,
    pub status: CheckpointStatus,
    pub reviewer_id: Option<String>,
    pub reviewer_name: Option<String>,
    pub review_note: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateExecutionCheckpoint {
    pub execution_process_id: Uuid,
    pub checkpoint_definition_id: Option<Uuid>,
    pub checkpoint_data: Value,
    pub trigger_reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ReviewCheckpoint {
    pub reviewer_id: String,
    pub reviewer_name: Option<String>,
    pub decision: CheckpointStatus,
    pub review_note: Option<String>,
}

impl ExecutionCheckpoint {
    /// Create a new checkpoint instance
    pub async fn create(
        pool: &SqlitePool,
        data: CreateExecutionCheckpoint,
    ) -> Result<Self, ExecutionCheckpointError> {
        let id = Uuid::new_v4();
        let checkpoint_data_str = data.checkpoint_data.to_string();

        let checkpoint = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"
            INSERT INTO execution_checkpoints (
                id, execution_process_id, checkpoint_definition_id,
                checkpoint_data, trigger_reason, expires_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(data.checkpoint_definition_id)
        .bind(checkpoint_data_str)
        .bind(&data.trigger_reason)
        .bind(data.expires_at)
        .fetch_one(pool)
        .await?;

        Ok(checkpoint)
    }

    /// Find by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ExecutionCheckpointError> {
        let checkpoint = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"SELECT * FROM execution_checkpoints WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(checkpoint)
    }

    /// Find all checkpoints for an execution
    pub async fn find_by_execution(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionCheckpointError> {
        let checkpoints = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"
            SELECT * FROM execution_checkpoints
            WHERE execution_process_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(checkpoints)
    }

    /// Find pending checkpoints for an execution
    pub async fn find_pending(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionCheckpointError> {
        let checkpoints = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"
            SELECT * FROM execution_checkpoints
            WHERE execution_process_id = ?1 AND status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(checkpoints)
    }

    /// Find all pending checkpoints across all executions
    pub async fn find_all_pending(pool: &SqlitePool) -> Result<Vec<Self>, ExecutionCheckpointError> {
        let checkpoints = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"
            SELECT * FROM execution_checkpoints
            WHERE status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(checkpoints)
    }

    /// Review a checkpoint (approve, reject, etc.)
    pub async fn review(
        pool: &SqlitePool,
        id: Uuid,
        review: ReviewCheckpoint,
    ) -> Result<Self, ExecutionCheckpointError> {
        // Check if already reviewed
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or(ExecutionCheckpointError::NotFound)?;

        if current.status != CheckpointStatus::Pending {
            return Err(ExecutionCheckpointError::AlreadyReviewed);
        }

        let status_str = review.decision.to_string();

        let checkpoint = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"
            UPDATE execution_checkpoints
            SET status = ?2, reviewer_id = ?3, reviewer_name = ?4,
                review_note = ?5, reviewed_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status_str)
        .bind(&review.reviewer_id)
        .bind(&review.reviewer_name)
        .bind(&review.review_note)
        .fetch_one(pool)
        .await?;

        Ok(checkpoint)
    }

    /// Auto-approve expired checkpoints
    pub async fn auto_approve_expired(pool: &SqlitePool) -> Result<u64, ExecutionCheckpointError> {
        let result = sqlx::query(
            r#"
            UPDATE execution_checkpoints
            SET status = 'auto_approved', reviewed_at = datetime('now', 'subsec')
            WHERE status = 'pending'
              AND expires_at IS NOT NULL
              AND expires_at < datetime('now', 'subsec')
            "#,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Skip a checkpoint
    pub async fn skip(
        pool: &SqlitePool,
        id: Uuid,
        reason: Option<String>,
    ) -> Result<Self, ExecutionCheckpointError> {
        let checkpoint = sqlx::query_as::<_, ExecutionCheckpoint>(
            r#"
            UPDATE execution_checkpoints
            SET status = 'skipped', review_note = ?2, reviewed_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reason)
        .fetch_one(pool)
        .await?;

        Ok(checkpoint)
    }

    /// Parse checkpoint_data as JSON Value
    pub fn checkpoint_data_json(&self) -> Option<Value> {
        serde_json::from_str(&self.checkpoint_data).ok()
    }

    /// Check if checkpoint is still pending
    pub fn is_pending(&self) -> bool {
        self.status == CheckpointStatus::Pending
    }

    /// Check if checkpoint was approved (manually or auto)
    pub fn is_approved(&self) -> bool {
        matches!(
            self.status,
            CheckpointStatus::Approved | CheckpointStatus::AutoApproved
        )
    }

    /// Check if checkpoint blocks execution
    pub fn is_blocking(&self) -> bool {
        self.status == CheckpointStatus::Pending || self.status == CheckpointStatus::Rejected
    }
}

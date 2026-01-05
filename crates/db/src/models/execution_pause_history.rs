use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionPauseHistoryError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Pause history entry not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "pause_action", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PauseAction {
    Pause,
    Resume,
}

impl std::fmt::Display for PauseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PauseAction::Pause => write!(f, "pause"),
            PauseAction::Resume => write!(f, "resume"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionPauseHistory {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub action: PauseAction,
    pub reason: Option<String>,
    pub initiated_by: String,
    pub initiated_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreatePauseHistoryEntry {
    pub execution_process_id: Uuid,
    pub action: PauseAction,
    pub reason: Option<String>,
    pub initiated_by: String,
    pub initiated_by_name: Option<String>,
}

impl ExecutionPauseHistory {
    /// Create a new pause history entry
    pub async fn create(
        pool: &SqlitePool,
        data: CreatePauseHistoryEntry,
    ) -> Result<Self, ExecutionPauseHistoryError> {
        let id = Uuid::new_v4();
        let action_str = data.action.to_string();

        let entry = sqlx::query_as::<_, ExecutionPauseHistory>(
            r#"
            INSERT INTO execution_pause_history (
                id, execution_process_id, action, reason, initiated_by, initiated_by_name
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(action_str)
        .bind(&data.reason)
        .bind(&data.initiated_by)
        .bind(&data.initiated_by_name)
        .fetch_one(pool)
        .await?;

        Ok(entry)
    }

    /// Find history for an execution
    pub async fn find_by_execution(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionPauseHistoryError> {
        let entries = sqlx::query_as::<_, ExecutionPauseHistory>(
            r#"
            SELECT * FROM execution_pause_history
            WHERE execution_process_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(entries)
    }

    /// Get latest action for an execution
    pub async fn find_latest(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Option<Self>, ExecutionPauseHistoryError> {
        let entry = sqlx::query_as::<_, ExecutionPauseHistory>(
            r#"
            SELECT * FROM execution_pause_history
            WHERE execution_process_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(execution_process_id)
        .fetch_optional(pool)
        .await?;

        Ok(entry)
    }

    /// Record a pause action
    pub async fn record_pause(
        pool: &SqlitePool,
        execution_process_id: Uuid,
        reason: Option<String>,
        initiated_by: String,
        initiated_by_name: Option<String>,
    ) -> Result<Self, ExecutionPauseHistoryError> {
        Self::create(
            pool,
            CreatePauseHistoryEntry {
                execution_process_id,
                action: PauseAction::Pause,
                reason,
                initiated_by,
                initiated_by_name,
            },
        )
        .await
    }

    /// Record a resume action
    pub async fn record_resume(
        pool: &SqlitePool,
        execution_process_id: Uuid,
        reason: Option<String>,
        initiated_by: String,
        initiated_by_name: Option<String>,
    ) -> Result<Self, ExecutionPauseHistoryError> {
        Self::create(
            pool,
            CreatePauseHistoryEntry {
                execution_process_id,
                action: PauseAction::Resume,
                reason,
                initiated_by,
                initiated_by_name,
            },
        )
        .await
    }
}

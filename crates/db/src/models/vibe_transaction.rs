use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum VibeTransactionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("VIBE transaction not found")]
    NotFound,
    #[error("Invalid source type: {0}")]
    InvalidSourceType(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum VibeSourceType {
    Agent,
    Project,
}

impl std::fmt::Display for VibeSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VibeSourceType::Agent => write!(f, "agent"),
            VibeSourceType::Project => write!(f, "project"),
        }
    }
}

impl std::str::FromStr for VibeSourceType {
    type Err = VibeTransactionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "agent" => Ok(VibeSourceType::Agent),
            "project" => Ok(VibeSourceType::Project),
            _ => Err(VibeTransactionError::InvalidSourceType(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AptosTxStatus {
    Pending,
    Confirmed,
    Failed,
}

impl std::fmt::Display for AptosTxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AptosTxStatus::Pending => write!(f, "pending"),
            AptosTxStatus::Confirmed => write!(f, "confirmed"),
            AptosTxStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VibeTransaction {
    pub id: Uuid,
    pub source_type: String,
    pub source_id: Uuid,
    pub amount_vibe: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub calculated_cost_cents: Option<i64>,
    pub aptos_tx_hash: Option<String>,
    pub aptos_tx_status: Option<String>,
    pub task_id: Option<Uuid>,
    pub task_attempt_id: Option<Uuid>,
    pub process_id: Option<Uuid>,
    pub description: Option<String>,
    pub metadata: Option<String>,
    /// Whether this transaction has been synced to the blockchain
    pub on_chain_synced: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateVibeTransaction {
    pub source_type: VibeSourceType,
    pub source_id: Uuid,
    pub amount_vibe: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub calculated_cost_cents: Option<i64>,
    pub task_id: Option<Uuid>,
    pub task_attempt_id: Option<Uuid>,
    pub process_id: Option<Uuid>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, FromRow, Serialize, TS)]
#[ts(export)]
pub struct VibeTransactionSummary {
    pub total_vibe: i64,
    pub total_cost_cents: Option<i64>,
    pub transaction_count: i64,
}

impl VibeTransaction {
    /// Create a new VIBE transaction
    pub async fn create(
        pool: &SqlitePool,
        data: CreateVibeTransaction,
    ) -> Result<Self, VibeTransactionError> {
        let id = Uuid::new_v4();
        let source_type_str = data.source_type.to_string();
        let metadata_str = data.metadata.map(|v| v.to_string());

        let tx = sqlx::query_as::<_, VibeTransaction>(
            r#"
            INSERT INTO vibe_transactions (
                id, source_type, source_id, amount_vibe,
                input_tokens, output_tokens, model, provider,
                calculated_cost_cents, task_id, task_attempt_id, process_id,
                description, metadata, aptos_tx_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'pending')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&source_type_str)
        .bind(data.source_id)
        .bind(data.amount_vibe)
        .bind(data.input_tokens)
        .bind(data.output_tokens)
        .bind(&data.model)
        .bind(&data.provider)
        .bind(data.calculated_cost_cents)
        .bind(data.task_id)
        .bind(data.task_attempt_id)
        .bind(data.process_id)
        .bind(&data.description)
        .bind(&metadata_str)
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    /// Update Aptos transaction status
    pub async fn update_aptos_status(
        pool: &SqlitePool,
        id: Uuid,
        tx_hash: &str,
        status: AptosTxStatus,
    ) -> Result<Self, VibeTransactionError> {
        let status_str = status.to_string();

        let tx = sqlx::query_as::<_, VibeTransaction>(
            r#"
            UPDATE vibe_transactions
            SET aptos_tx_hash = $2, aptos_tx_status = $3, updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tx_hash)
        .bind(&status_str)
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    /// List transactions by source (agent wallet or project)
    pub async fn list_by_source(
        pool: &SqlitePool,
        source_type: VibeSourceType,
        source_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, VibeTransactionError> {
        let source_type_str = source_type.to_string();

        let txs = sqlx::query_as::<_, VibeTransaction>(
            r#"
            SELECT * FROM vibe_transactions
            WHERE source_type = $1 AND source_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(&source_type_str)
        .bind(source_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(txs)
    }

    /// Get total VIBE spent for a source since a given time
    pub async fn sum_by_source(
        pool: &SqlitePool,
        source_type: VibeSourceType,
        source_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<VibeTransactionSummary, VibeTransactionError> {
        let source_type_str = source_type.to_string();
        let since_str = since
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

        let summary = sqlx::query_as::<_, VibeTransactionSummary>(
            r#"
            SELECT
                COALESCE(SUM(amount_vibe), 0) as total_vibe,
                SUM(calculated_cost_cents) as total_cost_cents,
                COUNT(*) as transaction_count
            FROM vibe_transactions
            WHERE source_type = $1 AND source_id = $2 AND created_at >= $3
            "#,
        )
        .bind(&source_type_str)
        .bind(source_id)
        .bind(&since_str)
        .fetch_one(pool)
        .await?;

        Ok(summary)
    }

    /// List transactions by task
    pub async fn list_by_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, VibeTransactionError> {
        let txs = sqlx::query_as::<_, VibeTransaction>(
            r#"
            SELECT * FROM vibe_transactions
            WHERE task_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(txs)
    }

    /// Get transactions that haven't been synced to the blockchain yet
    pub async fn list_pending_sync(
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<Self>, VibeTransactionError> {
        let txs = sqlx::query_as::<_, VibeTransaction>(
            r#"
            SELECT * FROM vibe_transactions
            WHERE on_chain_synced = 0
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(txs)
    }

    /// Mark a transaction as synced to the blockchain
    pub async fn mark_synced(
        pool: &SqlitePool,
        id: Uuid,
        tx_hash: &str,
    ) -> Result<Self, VibeTransactionError> {
        let tx = sqlx::query_as::<_, VibeTransaction>(
            r#"
            UPDATE vibe_transactions
            SET on_chain_synced = 1,
                aptos_tx_hash = $2,
                aptos_tx_status = 'confirmed',
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tx_hash)
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    /// Update cost details on an existing (typically zero-amount placeholder) transaction
    pub async fn update_cost(
        pool: &SqlitePool,
        id: Uuid,
        amount_vibe: i64,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
        model: Option<&str>,
        provider: Option<&str>,
        cost_cents: Option<i64>,
        process_id: Option<Uuid>,
        task_attempt_id: Option<Uuid>,
        description: Option<&str>,
    ) -> Result<Self, VibeTransactionError> {
        let tx = sqlx::query_as::<_, VibeTransaction>(
            r#"
            UPDATE vibe_transactions
            SET amount_vibe = $2,
                input_tokens = COALESCE($3, input_tokens),
                output_tokens = COALESCE($4, output_tokens),
                model = COALESCE($5, model),
                provider = COALESCE($6, provider),
                calculated_cost_cents = COALESCE($7, calculated_cost_cents),
                process_id = COALESCE($8, process_id),
                task_attempt_id = COALESCE($9, task_attempt_id),
                description = COALESCE($10, description),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(amount_vibe)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(model)
        .bind(provider)
        .bind(cost_cents)
        .bind(process_id)
        .bind(task_attempt_id)
        .bind(description)
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    /// Find a pending (zero-amount) transaction for a task
    pub async fn find_by_task_pending(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Option<Self>, VibeTransactionError> {
        let tx = sqlx::query_as::<_, VibeTransaction>(
            r#"
            SELECT * FROM vibe_transactions
            WHERE task_id = $1 AND amount_vibe = 0
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(task_id)
        .fetch_optional(pool)
        .await?;

        Ok(tx)
    }

    /// Mark a transaction sync as failed
    pub async fn mark_sync_failed(
        pool: &SqlitePool,
        id: Uuid,
        error_message: &str,
    ) -> Result<Self, VibeTransactionError> {
        let metadata = serde_json::json!({
            "sync_error": error_message
        }).to_string();

        let tx = sqlx::query_as::<_, VibeTransaction>(
            r#"
            UPDATE vibe_transactions
            SET aptos_tx_status = 'failed',
                metadata = COALESCE(metadata, '{}') || $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&metadata)
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum DepositStatus {
    Pending,
    Confirmed,
    Credited,
    Failed,
}

impl std::fmt::Display for DepositStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepositStatus::Pending => write!(f, "pending"),
            DepositStatus::Confirmed => write!(f, "confirmed"),
            DepositStatus::Credited => write!(f, "credited"),
            DepositStatus::Failed => write!(f, "failed"),
        }
    }
}

/// VIBE deposit from user wallet to treasury
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VibeDeposit {
    pub id: Uuid,
    pub project_id: Uuid,
    pub tx_hash: String,
    pub sender_address: String,
    #[ts(type = "number")]
    pub amount_vibe: i64,
    pub status: String,
    pub block_height: Option<i64>,
    #[ts(type = "Date")]
    pub detected_at: DateTime<Utc>,
    #[ts(type = "Date | null")]
    pub credited_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVibeDeposit {
    pub project_id: Uuid,
    pub tx_hash: String,
    pub sender_address: String,
    pub amount_vibe: i64,
    pub block_height: Option<i64>,
}

impl VibeDeposit {
    /// Create a new deposit record (status: pending)
    pub async fn create(pool: &SqlitePool, data: CreateVibeDeposit) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"INSERT INTO vibe_deposits (
                id, project_id, tx_hash, sender_address, amount_vibe, status, block_height
            ) VALUES (?, ?, ?, ?, ?, 'pending', ?)"#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.tx_hash)
        .bind(&data.sender_address)
        .bind(data.amount_vibe)
        .bind(data.block_height)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Find deposit by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, VibeDeposit>(
            r#"SELECT * FROM vibe_deposits WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Find deposit by transaction hash
    pub async fn find_by_tx_hash(pool: &SqlitePool, tx_hash: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, VibeDeposit>(
            r#"SELECT * FROM vibe_deposits WHERE tx_hash = ?"#,
        )
        .bind(tx_hash)
        .fetch_optional(pool)
        .await
    }

    /// List deposits for a project
    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, VibeDeposit>(
            r#"SELECT * FROM vibe_deposits
               WHERE project_id = ?
               ORDER BY created_at DESC
               LIMIT ?"#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Mark deposit as confirmed (on-chain confirmation received)
    pub async fn mark_confirmed(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"UPDATE vibe_deposits
               SET status = 'confirmed', updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Mark deposit as credited (balance updated)
    pub async fn mark_credited(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"UPDATE vibe_deposits
               SET status = 'credited',
                   credited_at = datetime('now', 'subsec'),
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Mark deposit as failed
    pub async fn mark_failed(
        pool: &SqlitePool,
        id: Uuid,
        error_message: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"UPDATE vibe_deposits
               SET status = 'failed',
                   error_message = ?,
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(error_message)
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Get total deposited for a project
    pub async fn total_deposited(pool: &SqlitePool, project_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COALESCE(SUM(amount_vibe), 0)
               FROM vibe_deposits
               WHERE project_id = ? AND status = 'credited'"#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }
}

/// VIBE withdrawal request from project balance
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VibeWithdrawal {
    pub id: Uuid,
    pub project_id: Uuid,
    pub destination_address: String,
    #[ts(type = "number")]
    pub amount_vibe: i64,
    pub status: String,
    pub tx_hash: Option<String>,
    #[ts(type = "Date")]
    pub requested_at: DateTime<Utc>,
    #[ts(type = "Date | null")]
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateVibeWithdrawal {
    pub project_id: Uuid,
    pub destination_address: String,
    #[ts(type = "number")]
    pub amount_vibe: i64,
}

impl VibeWithdrawal {
    /// Create a new withdrawal request
    pub async fn create(pool: &SqlitePool, data: CreateVibeWithdrawal) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"INSERT INTO vibe_withdrawals (
                id, project_id, destination_address, amount_vibe, status
            ) VALUES (?, ?, ?, ?, 'pending')"#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.destination_address)
        .bind(data.amount_vibe)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Find withdrawal by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, VibeWithdrawal>(
            r#"SELECT * FROM vibe_withdrawals WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// List withdrawals for a project
    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, VibeWithdrawal>(
            r#"SELECT * FROM vibe_withdrawals
               WHERE project_id = ?
               ORDER BY created_at DESC
               LIMIT ?"#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// List pending withdrawals (for treasury processing)
    pub async fn list_pending(pool: &SqlitePool, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, VibeWithdrawal>(
            r#"SELECT * FROM vibe_withdrawals
               WHERE status = 'pending'
               ORDER BY created_at ASC
               LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Mark withdrawal as processing
    pub async fn mark_processing(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"UPDATE vibe_withdrawals
               SET status = 'processing', updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Mark withdrawal as completed
    pub async fn mark_completed(
        pool: &SqlitePool,
        id: Uuid,
        tx_hash: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"UPDATE vibe_withdrawals
               SET status = 'completed',
                   tx_hash = ?,
                   processed_at = datetime('now', 'subsec'),
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(tx_hash)
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Mark withdrawal as failed
    pub async fn mark_failed(
        pool: &SqlitePool,
        id: Uuid,
        error_message: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"UPDATE vibe_withdrawals
               SET status = 'failed',
                   error_message = ?,
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(error_message)
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Get total withdrawn for a project
    pub async fn total_withdrawn(pool: &SqlitePool, project_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COALESCE(SUM(amount_vibe), 0)
               FROM vibe_withdrawals
               WHERE project_id = ? AND status = 'completed'"#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }
}

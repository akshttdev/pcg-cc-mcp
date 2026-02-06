use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqliteQueryResult};
use ts_rs::TS;
use uuid::Uuid;

/// Agent wallet for tracking VIBE budget and spending
/// Note: On-chain operations are handled at the project/treasury level, not per-agent
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentWallet {
    pub id: Uuid,
    pub profile_key: String,
    pub display_name: String,
    /// APT budget limit in octas (legacy, not used)
    #[ts(type = "number")]
    pub budget_limit: i64,
    /// APT spent amount in octas (legacy, not used)
    #[ts(type = "number")]
    pub spent_amount: i64,
    /// VIBE budget limit (1 VIBE = $0.01 USD)
    #[ts(type = "number | null")]
    pub vibe_budget_limit: Option<i64>,
    /// VIBE spent amount
    #[ts(type = "number")]
    pub vibe_spent_amount: i64,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentWalletTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub direction: String,
    #[ts(type = "number")]
    pub amount: i64,
    pub description: String,
    pub metadata: Option<String>,
    pub task_id: Option<Uuid>,
    pub process_id: Option<Uuid>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpsertAgentWallet {
    pub profile_key: String,
    #[ts(optional)]
    pub display_name: Option<String>,
    #[ts(type = "number")]
    pub budget_limit: i64,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateWalletTransaction {
    pub wallet_id: Uuid,
    pub direction: String,
    #[ts(type = "number")]
    pub amount: i64,
    #[ts(optional)]
    pub description: Option<String>,
    #[ts(optional)]
    pub metadata: Option<String>,
    #[ts(optional)]
    pub task_id: Option<Uuid>,
    #[ts(optional)]
    pub process_id: Option<Uuid>,
}

impl AgentWallet {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentWallet>(
            r#"SELECT
                id,
                profile_key,
                display_name,
                budget_limit,
                spent_amount,
                vibe_budget_limit,
                COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                created_at,
                updated_at
            FROM agent_wallets
            ORDER BY profile_key"#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_profile_key(
        pool: &SqlitePool,
        profile_key: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentWallet>(
            r#"SELECT
                id,
                profile_key,
                display_name,
                budget_limit,
                spent_amount,
                vibe_budget_limit,
                COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                created_at,
                updated_at
            FROM agent_wallets
            WHERE profile_key = ?"#,
        )
        .bind(profile_key)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentWallet>(
            r#"SELECT
                id,
                profile_key,
                display_name,
                budget_limit,
                spent_amount,
                vibe_budget_limit,
                COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                created_at,
                updated_at
            FROM agent_wallets
            WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        payload: &UpsertAgentWallet,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let display_name = payload
            .display_name
            .clone()
            .unwrap_or_else(|| payload.profile_key.clone());

        sqlx::query!(
            r#"INSERT INTO agent_wallets (
                id,
                profile_key,
                display_name,
                budget_limit,
                spent_amount
            ) VALUES (?, ?, ?, ?, 0)
            ON CONFLICT(profile_key) DO UPDATE SET
                display_name = excluded.display_name,
                budget_limit = excluded.budget_limit,
                updated_at = datetime('now', 'subsec')
            "#,
            id,
            payload.profile_key,
            display_name,
            payload.budget_limit
        )
        .execute(pool)
        .await?;

        AgentWallet::find_by_profile_key(pool, &payload.profile_key)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn adjust_spent(
        pool: &SqlitePool,
        wallet_id: Uuid,
        delta: i64,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!(
            r#"UPDATE agent_wallets
               SET spent_amount = spent_amount + ?,
                   updated_at = datetime('now', 'subsec')
             WHERE id = ?"#,
            delta,
            wallet_id
        )
        .execute(pool)
        .await
    }

    /// Set VIBE budget limit for an agent wallet
    pub async fn set_vibe_budget(
        pool: &SqlitePool,
        wallet_id: Uuid,
        vibe_budget_limit: Option<i64>,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!(
            r#"UPDATE agent_wallets
               SET vibe_budget_limit = ?,
                   updated_at = datetime('now', 'subsec')
             WHERE id = ?"#,
            vibe_budget_limit,
            wallet_id
        )
        .execute(pool)
        .await
    }

    /// Adjust VIBE spent amount for an agent wallet
    pub async fn adjust_vibe_spent(
        pool: &SqlitePool,
        wallet_id: Uuid,
        delta: i64,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!(
            r#"UPDATE agent_wallets
               SET vibe_spent_amount = COALESCE(vibe_spent_amount, 0) + ?,
                   updated_at = datetime('now', 'subsec')
             WHERE id = ?"#,
            delta,
            wallet_id
        )
        .execute(pool)
        .await
    }

    /// Check if agent has sufficient VIBE budget
    pub fn has_vibe_budget(&self, required_vibe: i64) -> bool {
        match self.vibe_budget_limit {
            Some(limit) => (limit - self.vibe_spent_amount) >= required_vibe,
            None => true, // No limit means unlimited
        }
    }

    /// Get remaining VIBE budget
    pub fn remaining_vibe(&self) -> Option<i64> {
        self.vibe_budget_limit.map(|limit| limit - self.vibe_spent_amount)
    }
}

impl AgentWalletTransaction {
    pub async fn list_by_wallet(
        pool: &SqlitePool,
        wallet_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentWalletTransaction>(
            r#"SELECT
                id,
                wallet_id,
                direction,
                amount,
                description,
                metadata,
                task_id,
                process_id,
                created_at
            FROM agent_wallet_transactions
            WHERE wallet_id = ?
            ORDER BY created_at DESC
            LIMIT ?"#,
        )
        .bind(wallet_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateWalletTransaction,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let description = payload
            .description
            .clone()
            .unwrap_or_else(|| "Manual adjustment".to_string());

        sqlx::query!(
            r#"INSERT INTO agent_wallet_transactions (
                id,
                wallet_id,
                direction,
                amount,
                description,
                metadata,
                task_id,
                process_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            id,
            payload.wallet_id,
            payload.direction,
            payload.amount,
            description,
            payload.metadata,
            payload.task_id,
            payload.process_id
        )
        .execute(pool)
        .await?;

        let delta = match payload.direction.as_str() {
            "debit" => payload.amount,
            "credit" => -payload.amount,
            _ => 0,
        };

        if delta != 0 {
            AgentWallet::adjust_spent(pool, payload.wallet_id, delta).await?;
        }

        sqlx::query_as::<_, AgentWalletTransaction>(
            r#"SELECT
                id,
                wallet_id,
                direction,
                amount,
                description,
                metadata,
                task_id,
                process_id,
                created_at
            FROM agent_wallet_transactions
            WHERE id = ?"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }
}

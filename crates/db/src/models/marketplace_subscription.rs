use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MarketplaceSubscription {
    pub id: Uuid,
    pub consumer_name: String,
    pub consumer_type: String,
    pub consumer_project_id: Option<Uuid>,
    pub consumer_org_id: Option<Uuid>,
    pub contact_email: Option<String>,
    pub service_type: String,
    pub listing_id: Option<Uuid>,
    // Note: api_key is NOT returned in normal queries — use find_by_api_key internally
    pub api_key: String,
    pub api_key_prefix: String,
    pub vibe_budget: f64,
    pub vibe_spent: f64,
    pub auto_refill_threshold: Option<f64>,
    pub auto_refill_amount: Option<f64>,
    pub status: String,
    pub suspend_reason: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
    #[ts(type = "Date | null")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl MarketplaceSubscription {
    /// Balance remaining
    pub fn vibe_balance(&self) -> f64 {
        self.vibe_budget - self.vibe_spent
    }

    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    pub fn has_balance(&self, required: f64) -> bool {
        self.vibe_balance() >= required
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSubscription {
    pub consumer_name: String,
    pub consumer_type: Option<String>,
    pub consumer_project_id: Option<Uuid>,
    pub consumer_org_id: Option<Uuid>,
    pub contact_email: Option<String>,
    pub service_type: String,
    pub listing_id: Option<Uuid>,
    pub vibe_budget: Option<f64>,
    pub auto_refill_threshold: Option<f64>,
    pub auto_refill_amount: Option<f64>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Safe view — omits api_key, shows only prefix
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SubscriptionView {
    pub id: Uuid,
    pub consumer_name: String,
    pub consumer_type: String,
    pub service_type: String,
    pub listing_id: Option<Uuid>,
    pub api_key_prefix: String,
    pub vibe_budget: f64,
    pub vibe_spent: f64,
    pub vibe_balance: f64,
    pub status: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date | null")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<MarketplaceSubscription> for SubscriptionView {
    fn from(s: MarketplaceSubscription) -> Self {
        let balance = s.vibe_balance();
        Self {
            id: s.id,
            consumer_name: s.consumer_name,
            consumer_type: s.consumer_type,
            service_type: s.service_type,
            listing_id: s.listing_id,
            api_key_prefix: s.api_key_prefix,
            vibe_budget: s.vibe_budget,
            vibe_spent: s.vibe_spent,
            vibe_balance: balance,
            status: s.status,
            created_at: s.created_at,
            expires_at: s.expires_at,
        }
    }
}

/// Full response including the API key — only returned on creation
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SubscriptionCreated {
    pub id: Uuid,
    pub consumer_name: String,
    pub service_type: String,
    pub api_key: String, // shown ONCE on creation
    pub api_key_prefix: String,
    pub vibe_budget: f64,
    pub status: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

fn generate_api_key() -> (String, String) {
    // Two UUIDs joined = 64 hex chars of entropy
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let full = format!("apn_{}", token);
    let prefix = full[..12].to_string(); // "apn_" + first 8 chars
    (full, prefix)
}

impl MarketplaceSubscription {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateSubscription,
    ) -> anyhow::Result<SubscriptionCreated> {
        let id = Uuid::new_v4();
        let consumer_type = data.consumer_type.unwrap_or_else(|| "external".to_string());
        let vibe_budget = data.vibe_budget.unwrap_or(0.0);
        let (api_key, api_key_prefix) = generate_api_key();

        sqlx::query(
            "INSERT INTO marketplace_subscriptions
             (id, consumer_name, consumer_type, consumer_project_id, consumer_org_id,
              contact_email, service_type, listing_id, api_key, api_key_prefix,
              vibe_budget, auto_refill_threshold, auto_refill_amount, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&data.consumer_name)
        .bind(&consumer_type)
        .bind(data.consumer_project_id)
        .bind(data.consumer_org_id)
        .bind(&data.contact_email)
        .bind(&data.service_type)
        .bind(data.listing_id)
        .bind(&api_key)
        .bind(&api_key_prefix)
        .bind(vibe_budget)
        .bind(data.auto_refill_threshold)
        .bind(data.auto_refill_amount)
        .bind(data.expires_at)
        .execute(pool)
        .await?;

        Ok(SubscriptionCreated {
            id,
            consumer_name: data.consumer_name,
            service_type: data.service_type,
            api_key, // shown only here
            api_key_prefix,
            vibe_budget,
            status: "active".to_string(),
            created_at: Utc::now(),
        })
    }

    pub async fn find_by_api_key(pool: &SqlitePool, api_key: &str) -> anyhow::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM marketplace_subscriptions WHERE api_key = ? AND status = 'active'",
        )
        .bind(api_key)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> anyhow::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM marketplace_subscriptions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> anyhow::Result<Vec<SubscriptionView>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM marketplace_subscriptions WHERE consumer_project_id = ? ORDER BY created_at DESC"
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_all(pool: &SqlitePool, limit: i64) -> anyhow::Result<Vec<SubscriptionView>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM marketplace_subscriptions ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Top up VIBE balance
    pub async fn add_budget(pool: &SqlitePool, id: Uuid, amount: f64) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE marketplace_subscriptions SET
             vibe_budget = vibe_budget + ?,
             updated_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(amount)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Debit VIBE spend (called after successful gateway request)
    pub async fn record_spend(pool: &SqlitePool, id: Uuid, amount: f64) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE marketplace_subscriptions SET
             vibe_spent = vibe_spent + ?,
             updated_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(amount)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn suspend(pool: &SqlitePool, id: Uuid, reason: &str) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE marketplace_subscriptions SET status = 'suspended', suspend_reason = ?, updated_at = datetime('now','subsec') WHERE id = ?"
        )
        .bind(reason)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn cancel(pool: &SqlitePool, id: Uuid) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE marketplace_subscriptions SET status = 'cancelled', updated_at = datetime('now','subsec') WHERE id = ?"
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

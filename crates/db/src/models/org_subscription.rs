use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Error)]
pub enum OrgSubscriptionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("subscription not found")]
    NotFound,
}

/// Plan tiers. Limits are colocated with the enum so callers don't have to
/// touch a config map — `plan.limits()` is the single source of truth.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum PlanTier {
    Free,
    Starter,
    Pro,
    Scale,
    Enterprise,
}

#[derive(Debug, Clone, Copy)]
pub struct PlanLimits {
    pub seat_limit: i32,
    pub action_limit: i32,
    pub allow_overage: bool,
}

impl PlanTier {
    pub fn limits(self) -> PlanLimits {
        match self {
            // Matches the spec table exactly.
            PlanTier::Free => PlanLimits {
                seat_limit: 2,
                action_limit: 500,
                allow_overage: false,
            },
            PlanTier::Starter => PlanLimits {
                seat_limit: 5,
                action_limit: 2_000,
                allow_overage: false,
            },
            PlanTier::Pro => PlanLimits {
                seat_limit: 15,
                action_limit: 10_000,
                allow_overage: true,
            },
            PlanTier::Scale => PlanLimits {
                seat_limit: 50,
                action_limit: 50_000,
                allow_overage: true,
            },
            // Enterprise is custom — we record `i32::MAX` as a sentinel for
            // "unlimited", and allow overage so usage checks always pass.
            PlanTier::Enterprise => PlanLimits {
                seat_limit: i32::MAX,
                action_limit: i32::MAX,
                allow_overage: true,
            },
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PlanTier::Free => "free",
            PlanTier::Starter => "starter",
            PlanTier::Pro => "pro",
            PlanTier::Scale => "scale",
            PlanTier::Enterprise => "enterprise",
        }
    }
}

impl std::str::FromStr for PlanTier {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(PlanTier::Free),
            "starter" => Ok(PlanTier::Starter),
            "pro" => Ok(PlanTier::Pro),
            "scale" => Ok(PlanTier::Scale),
            "enterprise" => Ok(PlanTier::Enterprise),
            other => Err(format!("unknown plan tier: {other}")),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrgSubscription {
    pub id: DbUuid,
    pub organization_id: DbUuid,
    pub plan: String,
    pub status: String,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub stripe_price_id: Option<String>,
    pub seat_limit: i64,
    pub action_limit: i64,
    pub allow_overage: i64,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at_period_end: i64,
    pub metadata: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OrgSubscription {
    /// Get the existing row or create a free-tier one. This is what every
    /// "what plan is this org on?" query goes through so we never have to
    /// special-case "no row" downstream.
    pub async fn ensure_free(
        pool: &SqlitePool,
        organization_id: &DbUuid,
    ) -> Result<Self, OrgSubscriptionError> {
        if let Some(existing) = Self::find_by_org(pool, organization_id).await? {
            return Ok(existing);
        }
        let id = DbUuid::new();
        let limits = PlanTier::Free.limits();
        let row = sqlx::query_as::<_, OrgSubscription>(
            r#"
            INSERT INTO org_subscriptions (
                id, organization_id, plan, status, seat_limit, action_limit, allow_overage
            )
            VALUES (?1, ?2, 'free', 'active', ?3, ?4, ?5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(organization_id)
        .bind(limits.seat_limit as i64)
        .bind(limits.action_limit as i64)
        .bind(if limits.allow_overage { 1_i64 } else { 0_i64 })
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_org(
        pool: &SqlitePool,
        organization_id: &DbUuid,
    ) -> Result<Option<Self>, OrgSubscriptionError> {
        let row = sqlx::query_as::<_, OrgSubscription>(
            r#"
            SELECT id, organization_id, plan, status, stripe_customer_id,
                   stripe_subscription_id, stripe_price_id, seat_limit, action_limit,
                   allow_overage, current_period_start, current_period_end,
                   cancel_at_period_end, metadata, created_at, updated_at
            FROM org_subscriptions
            WHERE organization_id = ?1
            "#,
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_stripe_customer(
        pool: &SqlitePool,
        stripe_customer_id: &str,
    ) -> Result<Option<Self>, OrgSubscriptionError> {
        let row = sqlx::query_as::<_, OrgSubscription>(
            r#"
            SELECT id, organization_id, plan, status, stripe_customer_id,
                   stripe_subscription_id, stripe_price_id, seat_limit, action_limit,
                   allow_overage, current_period_start, current_period_end,
                   cancel_at_period_end, metadata, created_at, updated_at
            FROM org_subscriptions
            WHERE stripe_customer_id = ?1
            "#,
        )
        .bind(stripe_customer_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Set the Stripe customer ID after a checkout-session creation. Idempotent.
    pub async fn set_stripe_customer(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        stripe_customer_id: &str,
    ) -> Result<(), OrgSubscriptionError> {
        sqlx::query(
            r#"
            UPDATE org_subscriptions SET
                stripe_customer_id = ?2,
                updated_at = datetime('now','subsec')
            WHERE organization_id = ?1
            "#,
        )
        .bind(organization_id)
        .bind(stripe_customer_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Apply a webhook-driven plan change. Persists the new limits derived
    /// from the tier so the hot path never has to compute them.
    pub async fn upgrade(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        plan: PlanTier,
        status: &str,
        stripe_subscription_id: Option<&str>,
        stripe_price_id: Option<&str>,
        current_period_start: Option<&str>,
        current_period_end: Option<&str>,
        cancel_at_period_end: bool,
    ) -> Result<Self, OrgSubscriptionError> {
        let limits = plan.limits();
        sqlx::query_as::<_, OrgSubscription>(
            r#"
            UPDATE org_subscriptions SET
                plan = ?2,
                status = ?3,
                stripe_subscription_id = COALESCE(?4, stripe_subscription_id),
                stripe_price_id        = COALESCE(?5, stripe_price_id),
                seat_limit             = ?6,
                action_limit           = ?7,
                allow_overage          = ?8,
                current_period_start   = ?9,
                current_period_end     = ?10,
                cancel_at_period_end   = ?11,
                updated_at             = datetime('now','subsec')
            WHERE organization_id = ?1
            RETURNING id, organization_id, plan, status, stripe_customer_id,
                      stripe_subscription_id, stripe_price_id, seat_limit, action_limit,
                      allow_overage, current_period_start, current_period_end,
                      cancel_at_period_end, metadata, created_at, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(plan.as_str())
        .bind(status)
        .bind(stripe_subscription_id)
        .bind(stripe_price_id)
        .bind(limits.seat_limit as i64)
        .bind(limits.action_limit as i64)
        .bind(if limits.allow_overage { 1_i64 } else { 0_i64 })
        .bind(current_period_start)
        .bind(current_period_end)
        .bind(if cancel_at_period_end { 1_i64 } else { 0_i64 })
        .fetch_optional(pool)
        .await?
        .ok_or(OrgSubscriptionError::NotFound)
    }

    /// Mark a subscription canceled — usually triggered by
    /// `customer.subscription.deleted`. Caller decides whether to drop the
    /// org back to free immediately or wait for `current_period_end`.
    pub async fn mark_canceled(
        pool: &SqlitePool,
        stripe_subscription_id: &str,
    ) -> Result<(), OrgSubscriptionError> {
        sqlx::query(
            r#"
            UPDATE org_subscriptions SET
                status = 'canceled',
                cancel_at_period_end = 1,
                updated_at = datetime('now','subsec')
            WHERE stripe_subscription_id = ?1
            "#,
        )
        .bind(stripe_subscription_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

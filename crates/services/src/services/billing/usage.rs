//! Per-org per-period action metering.
//!
//! The hot path is `record_action(pool, org, vibe_cost)`:
//!   1. Resolve current `org_subscriptions` row (creates a free-tier row if missing).
//!   2. Resolve / create the `usage_records` row for this billing period.
//!   3. Atomically increment action_count + vibe_spent.
//!   4. Return a `UsageStatus` so the caller can warn/block before launching
//!      the next agent.
//!
//! A dispatcher should call this *before* dispatch and abort on `Blocked`.

use chrono::{Datelike, NaiveDate, Utc};
use db::{
    db_uuid::DbUuid,
    models::{
        org_subscription::{OrgSubscription, PlanTier},
        usage_record::UsageRecord,
    },
};
use serde::Serialize;
use sqlx::SqlitePool;

use super::BillingError;

/// What a metering call returned. Callers turn `Blocked` into a 402 / upgrade prompt.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageStatus {
    Ok {
        plan: String,
        action_count: i64,
        action_limit: i64,
    },
    /// Crossed 80% of plan limit for the first time — caller should fire a
    /// notification once.
    Warning {
        plan: String,
        action_count: i64,
        action_limit: i64,
        pct: i64,
    },
    /// At or over the plan limit AND overage isn't allowed. Caller MUST refuse.
    Blocked {
        plan: String,
        action_count: i64,
        action_limit: i64,
        upgrade_recommended: &'static str,
    },
    /// Over the plan limit but plan permits overage — Stripe metered billing
    /// will charge for it.
    Overage {
        plan: String,
        action_count: i64,
        action_limit: i64,
    },
}

/// Increment the org's metered usage by `actions` actions and `vibe_spent` VIBE.
/// Returns the post-increment status for the caller to act on.
pub async fn record_action(
    pool: &SqlitePool,
    organization_id: &DbUuid,
    actions: i64,
    vibe_spent: f64,
) -> Result<UsageStatus, BillingError> {
    let sub = OrgSubscription::ensure_free(pool, organization_id).await?;
    let (start, end) = current_period_bounds(&sub);
    let record = UsageRecord::current_period(pool, organization_id, &start, &end).await?;
    let updated = UsageRecord::increment(pool, &record.id, actions, vibe_spent).await?;

    let status = classify(&sub, updated.action_count);
    if matches!(status, UsageStatus::Warning { .. }) && updated.warned_at_80pct == 0 {
        let _ = UsageRecord::mark_warned(pool, &updated.id).await;
    }
    Ok(status)
}

/// Read-only — return current usage without incrementing. Used by the
/// dashboard widget + GET /billing/subscription.
pub async fn usage_status(
    pool: &SqlitePool,
    organization_id: &DbUuid,
) -> Result<UsageStatus, BillingError> {
    let sub = OrgSubscription::ensure_free(pool, organization_id).await?;
    let (start, _) = current_period_bounds(&sub);
    let count = UsageRecord::find_for_period(pool, organization_id, &start)
        .await?
        .map(|r| r.action_count)
        .unwrap_or(0);
    Ok(classify(&sub, count))
}

fn classify(sub: &OrgSubscription, action_count: i64) -> UsageStatus {
    let plan = sub.plan.clone();
    let limit = sub.action_limit;

    if action_count < (limit * 80 / 100) {
        return UsageStatus::Ok {
            plan,
            action_count,
            action_limit: limit,
        };
    }
    if action_count < limit {
        let pct = (action_count * 100) / limit.max(1);
        return UsageStatus::Warning {
            plan,
            action_count,
            action_limit: limit,
            pct,
        };
    }
    if sub.allow_overage == 1 {
        return UsageStatus::Overage {
            plan,
            action_count,
            action_limit: limit,
        };
    }
    UsageStatus::Blocked {
        plan,
        action_count,
        action_limit: limit,
        upgrade_recommended: recommended_upgrade(sub.plan.as_str()),
    }
}

/// Suggest the next tier up for an upgrade-prompt UI.
fn recommended_upgrade(current_plan: &str) -> &'static str {
    match current_plan {
        "free" => "starter",
        "starter" => "pro",
        "pro" => "scale",
        _ => "enterprise",
    }
}

/// Resolve `[period_start, period_end)` for the current usage window. If
/// Stripe gave us a billing period, use it verbatim. Otherwise — for free-tier
/// orgs that never went through checkout — use the calendar month.
fn current_period_bounds(sub: &OrgSubscription) -> (String, String) {
    if let (Some(s), Some(e)) = (&sub.current_period_start, &sub.current_period_end) {
        // Strip to date if Stripe gave us datetime
        let s_date = s.split('T').next().unwrap_or(s.as_str());
        let e_date = e.split('T').next().unwrap_or(e.as_str());
        return (s_date.to_string(), e_date.to_string());
    }
    let today = Utc::now().date_naive();
    let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    // First of next month
    let next = if today.month() == 12 {
        NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap_or(today)
    } else {
        NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap_or(today)
    };
    (first.to_string(), next.to_string())
}

#[allow(dead_code)] // referenced for plan-default convenience
fn default_free_plan() -> PlanTier {
    PlanTier::Free
}

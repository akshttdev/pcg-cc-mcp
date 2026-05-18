use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Error)]
pub enum UsageRecordError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UsageRecord {
    pub id: DbUuid,
    pub organization_id: DbUuid,
    pub period_start: String,
    pub period_end: String,
    pub action_count: i64,
    pub vibe_spent: f64,
    pub warned_at_80pct: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UsageRecord {
    /// Get-or-create the row for the period covering "now".
    pub async fn current_period(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        period_start: &str,
        period_end: &str,
    ) -> Result<Self, UsageRecordError> {
        if let Some(existing) = Self::find_for_period(pool, organization_id, period_start).await? {
            return Ok(existing);
        }
        let id = DbUuid::new();
        let row = sqlx::query_as::<_, UsageRecord>(
            r#"
            INSERT INTO usage_records (id, organization_id, period_start, period_end)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING id, organization_id, period_start, period_end, action_count,
                      vibe_spent, warned_at_80pct, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(organization_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    pub async fn find_for_period(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        period_start: &str,
    ) -> Result<Option<Self>, UsageRecordError> {
        let row = sqlx::query_as::<_, UsageRecord>(
            r#"
            SELECT id, organization_id, period_start, period_end, action_count,
                   vibe_spent, warned_at_80pct, created_at, updated_at
            FROM usage_records
            WHERE organization_id = ?1 AND period_start = ?2
            "#,
        )
        .bind(organization_id)
        .bind(period_start)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Atomic increment so concurrent dispatches don't race. Returns the
    /// post-increment row so callers can immediately compare against limits.
    pub async fn increment(
        pool: &SqlitePool,
        id: &DbUuid,
        actions: i64,
        vibe_spent: f64,
    ) -> Result<Self, UsageRecordError> {
        let row = sqlx::query_as::<_, UsageRecord>(
            r#"
            UPDATE usage_records SET
                action_count = action_count + ?2,
                vibe_spent   = vibe_spent + ?3,
                updated_at   = datetime('now','subsec')
            WHERE id = ?1
            RETURNING id, organization_id, period_start, period_end, action_count,
                      vibe_spent, warned_at_80pct, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(actions)
        .bind(vibe_spent)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Flag that we've already sent the 80%-utilization warning so the
    /// notification doesn't fire on every action over the threshold.
    pub async fn mark_warned(pool: &SqlitePool, id: &DbUuid) -> Result<(), UsageRecordError> {
        sqlx::query(
            r#"
            UPDATE usage_records SET
                warned_at_80pct = 1,
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

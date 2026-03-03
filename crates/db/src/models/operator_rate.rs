use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Private rate for an operator within an organization.
/// Only visible to admin and project managers.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OperatorRate {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub operator_id: Uuid,
    /// hourly | daily | weekly | monthly | project
    pub duration: String,
    pub rate_vibe: i64,
    pub rate_usd: f64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertOperatorRate {
    pub organization_id: Uuid,
    pub operator_id: Uuid,
    pub duration: String,
    pub rate_vibe: i64,
    pub rate_usd: Option<f64>,
    pub notes: Option<String>,
}

impl OperatorRate {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM operator_rates WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn upsert(pool: &SqlitePool, input: UpsertOperatorRate) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let rate_usd = input.rate_usd.unwrap_or(0.0);

        sqlx::query(
            r#"INSERT INTO operator_rates
               (id, organization_id, operator_id, duration, rate_vibe, rate_usd, notes)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(organization_id, operator_id, duration)
               DO UPDATE SET
                 rate_vibe  = excluded.rate_vibe,
                 rate_usd   = excluded.rate_usd,
                 notes      = excluded.notes,
                 updated_at = datetime('now','subsec')"#,
        )
        .bind(id)
        .bind(input.organization_id)
        .bind(input.operator_id)
        .bind(&input.duration)
        .bind(input.rate_vibe)
        .bind(rate_usd)
        .bind(&input.notes)
        .execute(pool)
        .await?;

        // Fetch the actual record (upsert may have returned existing id)
        sqlx::query_as(
            "SELECT * FROM operator_rates WHERE organization_id = ? AND operator_id = ? AND duration = ?",
        )
        .bind(input.organization_id)
        .bind(input.operator_id)
        .bind(&input.duration)
        .fetch_one(pool)
        .await
    }

    pub async fn list_for_org(pool: &SqlitePool, org_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM operator_rates WHERE organization_id = ? ORDER BY operator_id, duration",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_for_operator(
        pool: &SqlitePool,
        operator_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM operator_rates WHERE operator_id = ? ORDER BY organization_id, duration",
        )
        .bind(operator_id)
        .fetch_all(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM operator_rates WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

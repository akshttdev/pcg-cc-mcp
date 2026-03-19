use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandIntakeToken {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl BrandIntakeToken {
    pub async fn create(
        pool: &SqlitePool,
        org_id: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_bytes = id.to_string();
        let org_bytes = org_id.to_string();
        sqlx::query(
            "INSERT INTO brand_intake_tokens (id, organization_id, token, expires_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&id_bytes)
        .bind(&org_bytes)
        .bind(token)
        .bind(expires_at.to_rfc3339())
        .execute(pool)
        .await?;

        Self::find_by_token(pool, token).await.map(|o| o.unwrap())
    }

    pub async fn find_by_token(
        pool: &SqlitePool,
        token: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, BrandIntakeToken>(
            "SELECT id, organization_id, token, expires_at, used_at, created_at
             FROM brand_intake_tokens WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
    }

    pub async fn mark_used(pool: &SqlitePool, token: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE brand_intake_tokens SET used_at = datetime('now','subsec') WHERE token = ?",
        )
        .bind(token)
        .execute(pool)
        .await?;
        Ok(())
    }
}

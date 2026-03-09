use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// Shareable review link token for a deliverable.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewToken {
    pub id: Uuid,
    pub token: String,
    pub deliverable_id: Uuid,
    pub created_by: Option<Uuid>,
    pub expires_at: Option<String>,
    pub view_count: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl ReviewToken {
    /// Generate a new review token for a deliverable.
    pub async fn generate(
        pool: &SqlitePool,
        deliverable_id: Uuid,
        created_by: Option<Uuid>,
        expires_at: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        // 32-char base64url-safe token from two UUIDs
        let raw = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let token = raw[..32].to_string();

        sqlx::query(
            "INSERT INTO review_tokens (id, token, deliverable_id, created_by, expires_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&token)
        .bind(deliverable_id)
        .bind(created_by)
        .bind(&expires_at)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM review_tokens WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Validate a token string, increment view_count, and return (token, deliverable_id).
    pub async fn validate_and_increment(
        pool: &SqlitePool,
        token: &str,
    ) -> Result<Option<(Self, Uuid)>, sqlx::Error> {
        let row: Option<Self> = sqlx::query_as(
            "SELECT * FROM review_tokens WHERE token = ? AND is_active = 1 \
             AND (expires_at IS NULL OR expires_at > datetime('now'))",
        )
        .bind(token)
        .fetch_optional(pool)
        .await?;

        if let Some(rt) = row {
            sqlx::query(
                "UPDATE review_tokens SET view_count = view_count + 1 WHERE id = ?",
            )
            .bind(rt.id)
            .execute(pool)
            .await?;
            let deliverable_id = rt.deliverable_id;
            Ok(Some((rt, deliverable_id)))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_deliverable(
        pool: &SqlitePool,
        deliverable_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM review_tokens WHERE deliverable_id = ? ORDER BY created_at DESC",
        )
        .bind(deliverable_id)
        .fetch_all(pool)
        .await
    }

    /// Return the most recent active token for a deliverable, generating one if none exists.
    pub async fn get_or_create(
        pool: &SqlitePool,
        deliverable_id: Uuid,
        created_by: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        let existing: Option<Self> = sqlx::query_as(
            "SELECT * FROM review_tokens WHERE deliverable_id = ? AND is_active = 1 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(deliverable_id)
        .fetch_optional(pool)
        .await?;

        if let Some(t) = existing {
            Ok(t)
        } else {
            Self::generate(pool, deliverable_id, created_by, None).await
        }
    }
}

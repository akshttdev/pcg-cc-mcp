use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// A frame-accurate or general comment on a deliverable review.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: Uuid,
    pub deliverable_id: Uuid,
    pub token_id: Option<Uuid>,
    pub timecode_seconds: Option<f64>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub content: String,
    pub is_resolved: bool,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewComment {
    pub timecode_seconds: Option<f64>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub content: String,
}

impl ReviewComment {
    pub async fn create(
        pool: &SqlitePool,
        deliverable_id: Uuid,
        token_id: Option<Uuid>,
        input: CreateReviewComment,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let author = input.author_name.unwrap_or_else(|| "Reviewer".into());

        sqlx::query(
            r#"INSERT INTO review_comments
               (id, deliverable_id, token_id, timecode_seconds, author_name, author_email, content)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(deliverable_id)
        .bind(token_id)
        .bind(input.timecode_seconds)
        .bind(&author)
        .bind(&input.author_email)
        .bind(&input.content)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM review_comments WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_deliverable(
        pool: &SqlitePool,
        deliverable_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM review_comments WHERE deliverable_id = ? \
             ORDER BY COALESCE(timecode_seconds, 9999999) ASC, created_at ASC",
        )
        .bind(deliverable_id)
        .fetch_all(pool)
        .await
    }

    pub async fn resolve(
        pool: &SqlitePool,
        id: Uuid,
        resolved_by: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query(
            "UPDATE review_comments SET is_resolved = 1, resolved_by = ?, \
             resolved_at = datetime('now','subsec'), updated_at = datetime('now','subsec') \
             WHERE id = ?",
        )
        .bind(resolved_by)
        .bind(id)
        .execute(pool)
        .await?;
        Self::find_by_id(pool, id).await
    }
}

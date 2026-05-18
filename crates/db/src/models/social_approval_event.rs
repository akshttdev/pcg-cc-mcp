use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Audit-trail entry for social post approval state changes.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialApprovalEvent {
    pub id: Uuid,
    pub post_id: Uuid,
    pub actor: String,
    pub from_status: String,
    pub to_status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSocialApprovalEvent {
    pub post_id: Uuid,
    pub actor: String,
    pub from_status: String,
    pub to_status: String,
    pub note: Option<String>,
}

impl SocialApprovalEvent {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateSocialApprovalEvent,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, SocialApprovalEvent>(
            r#"
            INSERT INTO social_approval_events (id, post_id, actor, from_status, to_status, note)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING *
        "#,
        )
        .bind(id)
        .bind(data.post_id)
        .bind(&data.actor)
        .bind(&data.from_status)
        .bind(&data.to_status)
        .bind(&data.note)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_post(pool: &SqlitePool, post_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SocialApprovalEvent>(
            "SELECT * FROM social_approval_events WHERE post_id = ?1 ORDER BY created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
    }
}

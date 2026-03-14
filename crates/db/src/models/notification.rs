use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub organization_id: Option<String>,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub read_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotification {
    pub user_id: String,
    pub organization_id: Option<String>,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub source: Option<String>,
    pub source_id: Option<String>,
}

impl Notification {
    pub async fn create(pool: &SqlitePool, data: &CreateNotification) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO notifications (id, user_id, organization_id, title, message, notification_type, source, source_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(&data.user_id)
        .bind(&data.organization_id)
        .bind(&data.title)
        .bind(&data.message)
        .bind(&data.notification_type)
        .bind(&data.source)
        .bind(&data.source_id)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_user(
        pool: &SqlitePool,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM notifications WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_unread_by_user(
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM notifications WHERE user_id = ? AND read_at IS NULL ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    pub async fn count_unread(pool: &SqlitePool, user_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }

    pub async fn mark_read(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE notifications SET read_at = datetime('now', 'subsec') WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn mark_all_read(pool: &SqlitePool, user_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE notifications SET read_at = datetime('now', 'subsec') WHERE user_id = ? AND read_at IS NULL",
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM notifications WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

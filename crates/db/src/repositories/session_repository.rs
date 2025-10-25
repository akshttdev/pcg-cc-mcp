// Session database repository
use crate::models::user::Session;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct SessionRepository;

impl SessionRepository {
    /// Create a new session
    pub async fn create(
        pool: &PgPool,
        session_id: &str,
        user_id: Uuid,
    ) -> Result<Session, sqlx::Error> {
        let now = Utc::now();
        let expires_at = now + Duration::days(30); // Session valid for 30 days

        sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO sessions (id, user_id, created_at, expires_at, last_accessed)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(session_id)
        .bind(user_id)
        .bind(now)
        .bind(expires_at)
        .bind(now)
        .fetch_one(pool)
        .await
    }

    /// Find session by ID
    pub async fn find_by_id(pool: &PgPool, session_id: &str) -> Result<Option<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE id = $1 AND expires_at > NOW()"
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
    }

    /// Update last accessed time
    pub async fn update_last_accessed(pool: &PgPool, session_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE sessions SET last_accessed = NOW() WHERE id = $1"
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete session (logout)
    pub async fn delete(pool: &PgPool, session_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all sessions for a user
    pub async fn delete_user_sessions(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

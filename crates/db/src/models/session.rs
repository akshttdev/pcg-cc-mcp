use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Session {
    pub id: Uuid,
    pub task_attempt_id: Uuid,
    pub executor: Option<String>,
    pub agent_working_dir: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateSession {
    pub task_attempt_id: Uuid,
    pub executor: Option<String>,
    pub agent_working_dir: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateSession {
    pub executor: Option<String>,
    pub agent_working_dir: Option<String>,
}

impl Session {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Session,
            r#"SELECT
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                executor,
                agent_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM agent_sessions
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_task_attempt_id(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Session,
            r#"SELECT
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                executor,
                agent_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM agent_sessions
            WHERE task_attempt_id = $1
            ORDER BY updated_at DESC"#,
            task_attempt_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateSession) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Session,
            r#"INSERT INTO agent_sessions (id, task_attempt_id, executor, agent_working_dir)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                executor,
                agent_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.task_attempt_id,
            data.executor,
            data.agent_working_dir
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateSession,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = Self::find_by_id(pool, id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let executor = data.executor.as_ref().or(existing.executor.as_ref());
        let agent_working_dir = data
            .agent_working_dir
            .as_ref()
            .or(existing.agent_working_dir.as_ref());

        sqlx::query_as!(
            Session,
            r#"UPDATE agent_sessions
            SET executor = $2, agent_working_dir = $3, updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                executor,
                agent_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            executor,
            agent_working_dir
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM agent_sessions WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

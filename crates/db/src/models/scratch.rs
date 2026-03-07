use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Scratch {
    pub id: Uuid,
    pub scratch_type: String,
    pub payload: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScratchPayload {
    DraftTask { message: String },
    DraftFollowUp { message: String, executor: Option<String> },
    UiPreferences { data: serde_json::Value },
    WorkspaceNotes { task_attempt_id: String, notes: String },
    ProjectRepoDefaults { project_id: String, repo_ids: Vec<String> },
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateScratch {
    pub scratch_type: String,
    pub payload: String,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateScratch {
    pub payload: String,
}

impl Scratch {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Scratch,
            r#"SELECT
                id as "id!: Uuid",
                scratch_type,
                payload,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM scratch
            ORDER BY updated_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_type(
        pool: &SqlitePool,
        scratch_type: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Scratch,
            r#"SELECT
                id as "id!: Uuid",
                scratch_type,
                payload,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM scratch
            WHERE scratch_type = $1
            ORDER BY updated_at DESC"#,
            scratch_type
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Scratch,
            r#"SELECT
                id as "id!: Uuid",
                scratch_type,
                payload,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM scratch
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert(pool: &SqlitePool, data: &CreateScratch) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Scratch,
            r#"INSERT INTO scratch (id, scratch_type, payload)
            VALUES ($1, $2, $3)
            ON CONFLICT(scratch_type) DO UPDATE SET
                payload = excluded.payload,
                updated_at = datetime('now', 'subsec')
            RETURNING
                id as "id!: Uuid",
                scratch_type,
                payload,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.scratch_type,
            data.payload
        )
        .fetch_one(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateScratch) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Scratch,
            r#"INSERT INTO scratch (id, scratch_type, payload)
            VALUES ($1, $2, $3)
            RETURNING
                id as "id!: Uuid",
                scratch_type,
                payload,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.scratch_type,
            data.payload
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateScratch,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Scratch,
            r#"UPDATE scratch
            SET payload = $2, updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                scratch_type,
                payload,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.payload
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM scratch WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

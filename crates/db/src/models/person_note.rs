use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PersonNote {
    pub id: Uuid,
    pub person_id: Uuid,
    pub author_id: Option<Uuid>,
    pub text: String,
    /// open | follow_up | resolved | pinned
    pub status: String,
    /// JSON: [{name, url, mime_type, size_bytes}]
    pub attachments: String,
    pub proposal_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePersonNote {
    pub person_id: Uuid,
    pub author_id: Option<Uuid>,
    pub text: String,
    pub status: Option<String>,
    pub proposal_id: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdatePersonNote {
    pub text: Option<String>,
    pub status: Option<String>,
    pub proposal_id: Option<Uuid>,
}

impl PersonNote {
    pub async fn create(pool: &SqlitePool, input: CreatePersonNote) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let status = input.status.unwrap_or_else(|| "open".into());

        sqlx::query(
            r#"INSERT INTO person_notes (id, person_id, author_id, text, status, proposal_id)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(input.person_id)
        .bind(input.author_id)
        .bind(&input.text)
        .bind(&status)
        .bind(input.proposal_id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM person_notes WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list_for_person(
        pool: &SqlitePool,
        person_id: Uuid,
        status: Option<&str>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        if let Some(s) = status {
            sqlx::query_as(
                "SELECT * FROM person_notes WHERE person_id = ? AND status = ? ORDER BY created_at DESC",
            )
            .bind(person_id)
            .bind(s)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT * FROM person_notes WHERE person_id = ? ORDER BY created_at DESC",
            )
            .bind(person_id)
            .fetch_all(pool)
            .await
        }
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdatePersonNote,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::new(
            "UPDATE person_notes SET updated_at = datetime('now','subsec')",
        );
        if let Some(v) = input.text {
            qb.push(", text = ").push_bind(v);
        }
        if let Some(v) = input.status {
            qb.push(", status = ").push_bind(v);
        }
        if let Some(v) = input.proposal_id {
            qb.push(", proposal_id = ").push_bind(v);
        }
        qb.push(" WHERE id = ").push_bind(id);
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM person_notes WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}

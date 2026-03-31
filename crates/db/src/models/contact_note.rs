use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

use crate::db_uuid::DbUuid;

const COLUMNS: &str =
    "id, crm_contact_id, author_id, text, status, attachments, proposal_id, created_at, updated_at";

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ContactNote {
    pub id: Uuid,
    pub crm_contact_id: Option<String>,
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
pub struct CreateContactNote {
    pub crm_contact_id: String,
    pub author_id: Option<Uuid>,
    pub text: String,
    pub status: Option<String>,
    pub proposal_id: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateContactNote {
    pub text: Option<String>,
    pub status: Option<String>,
    pub proposal_id: Option<Uuid>,
}

impl ContactNote {
    pub async fn create(pool: &SqlitePool, input: CreateContactNote) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        let status = input.status.unwrap_or_else(|| "open".into());

        sqlx::query(
            r#"INSERT INTO contact_notes (id, crm_contact_id, author_id, text, status, proposal_id)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(&input.crm_contact_id)
        .bind(input.author_id.map(|u| u.to_string()))
        .bind(&input.text)
        .bind(&status)
        .bind(input.proposal_id.map(|u| u.to_string()))
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id.to_uuid())
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM contact_notes WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(pool)
            .await
    }

    pub async fn list_for_contact(
        pool: &SqlitePool,
        contact_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM contact_notes WHERE crm_contact_id = ? AND status = ? ORDER BY created_at DESC"
            ))
            .bind(contact_id)
            .bind(s)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM contact_notes WHERE crm_contact_id = ? ORDER BY created_at DESC"
            ))
            .bind(contact_id)
            .fetch_all(pool)
            .await
        }
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateContactNote,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::new(
            "UPDATE contact_notes SET updated_at = datetime('now','subsec')",
        );
        if let Some(v) = input.text {
            qb.push(", text = ").push_bind(v);
        }
        if let Some(v) = input.status {
            qb.push(", status = ").push_bind(v);
        }
        if let Some(v) = input.proposal_id {
            qb.push(", proposal_id = ").push_bind(v.to_string());
        }
        qb.push(" WHERE id = ").push_bind(id.to_string());
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM contact_notes WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}

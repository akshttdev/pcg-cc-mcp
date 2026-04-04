use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Deliverable for a project — tracks creative/code output through review stages.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Deliverable {
    pub id: Uuid,
    pub project_id: Uuid,
    pub proposal_id: Option<Uuid>,

    /// video | audio | graphic | copy | code | document | other
    pub deliverable_type: String,
    pub title: String,
    pub description: String,

    /// working | internal_review | client_review | revision | client_revision | done
    pub status: String,

    pub revision_rounds_allowed: i64,
    pub revision_rounds_used: i64,

    pub working_file_url: Option<String>,
    pub final_link: Option<String>,
    pub due_date: Option<String>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    /// ID of the active Editron cinematic brief, if one has been dispatched
    pub cinematic_brief_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateDeliverable {
    pub project_id: Uuid,
    pub proposal_id: Option<Uuid>,
    pub deliverable_type: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub revision_rounds_allowed: Option<i64>,
    pub working_file_url: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateDeliverable {
    pub title: Option<String>,
    pub description: Option<String>,
    pub deliverable_type: Option<String>,
    pub revision_rounds_allowed: Option<i64>,
    pub working_file_url: Option<String>,
    pub final_link: Option<String>,
    pub due_date: Option<String>,
}

impl Deliverable {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM deliverables WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &SqlitePool, input: CreateDeliverable) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let dtype = input.deliverable_type.unwrap_or_else(|| "other".into());
        let description = input.description.unwrap_or_default();
        let revisions = input.revision_rounds_allowed.unwrap_or(2);

        sqlx::query(
            r#"INSERT INTO deliverables
               (id, project_id, proposal_id, deliverable_type, title, description,
                revision_rounds_allowed, working_file_url, due_date)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(input.project_id)
        .bind(input.proposal_id)
        .bind(&dtype)
        .bind(&input.title)
        .bind(&description)
        .bind(revisions)
        .bind(&input.working_file_url)
        .bind(&input.due_date)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn list_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM deliverables WHERE project_id = ? ORDER BY created_at ASC")
            .bind(project_id)
            .fetch_all(pool)
            .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateDeliverable,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::new(
            "UPDATE deliverables SET updated_at = datetime('now','subsec')",
        );
        if let Some(v) = input.title {
            qb.push(", title = ").push_bind(v);
        }
        if let Some(v) = input.description {
            qb.push(", description = ").push_bind(v);
        }
        if let Some(v) = input.deliverable_type {
            qb.push(", deliverable_type = ").push_bind(v);
        }
        if let Some(v) = input.revision_rounds_allowed {
            qb.push(", revision_rounds_allowed = ").push_bind(v);
        }
        if let Some(v) = input.working_file_url {
            qb.push(", working_file_url = ").push_bind(v);
        }
        if let Some(v) = input.final_link {
            qb.push(", final_link = ").push_bind(v);
        }
        if let Some(v) = input.due_date {
            qb.push(", due_date = ").push_bind(v);
        }
        qb.push(" WHERE id = ").push_bind(id);
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    /// Advance status; auto-increments revision counter and sets delivered_at.
    pub async fn move_status(
        pool: &SqlitePool,
        id: Uuid,
        new_status: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let sql = match new_status {
            "done" => {
                "UPDATE deliverables SET status = ?, delivered_at = datetime('now','subsec'), \
                 updated_at = datetime('now','subsec') WHERE id = ?"
            }
            "revision" | "client_revision" => {
                "UPDATE deliverables SET status = ?, \
                 revision_rounds_used = revision_rounds_used + 1, \
                 updated_at = datetime('now','subsec') WHERE id = ?"
            }
            _ => {
                "UPDATE deliverables SET status = ?, \
                 updated_at = datetime('now','subsec') WHERE id = ?"
            }
        };

        sqlx::query(sql)
            .bind(new_status)
            .bind(id)
            .execute(pool)
            .await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM deliverables WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

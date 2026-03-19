use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseCollectionRun {
    pub id: String,
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub items_collected: i64,
    pub items_new: i64,
    pub items_duplicate: i64,
    pub status: String,
    pub error: Option<String>,
    pub vibe_cost: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePulseCollectionRun {
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source_id: String,
}

impl PulseCollectionRun {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_collection_runs WHERE project_id = ? ORDER BY started_at DESC LIMIT ?",
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_source(
        pool: &SqlitePool,
        project_id: Uuid,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_collection_runs WHERE project_id = ? AND source_id = ? ORDER BY started_at DESC LIMIT ?",
        )
        .bind(project_id)
        .bind(source_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreatePulseCollectionRun,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO pulse_collection_runs (id, project_id, organization_id, source_id)
               VALUES (?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(data.project_id)
        .bind(data.organization_id)
        .bind(&data.source_id)
        .fetch_one(pool)
        .await
    }

    pub async fn complete(
        pool: &SqlitePool,
        id: &str,
        items_collected: i64,
        items_new: i64,
        items_duplicate: i64,
        error: Option<&str>,
        vibe_cost: Option<f64>,
    ) -> Result<(), sqlx::Error> {
        let status = if error.is_some() {
            "failed"
        } else {
            "completed"
        };

        sqlx::query(
            r#"UPDATE pulse_collection_runs SET
               completed_at = datetime('now', 'subsec'), items_collected = ?, items_new = ?,
               items_duplicate = ?, status = ?, error = ?, vibe_cost = ?
               WHERE id = ?"#,
        )
        .bind(items_collected)
        .bind(items_new)
        .bind(items_duplicate)
        .bind(status)
        .bind(error)
        .bind(vibe_cost)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

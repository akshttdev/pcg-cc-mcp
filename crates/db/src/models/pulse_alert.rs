use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseAlert {
    pub id: String,
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub rule_id: String,
    pub content_item_id: String,
    pub priority: String,
    pub acknowledged: bool,
    pub auto_task_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePulseAlert {
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub rule_id: String,
    pub content_item_id: String,
    pub priority: String,
    pub auto_task_id: Option<Uuid>,
}

impl PulseAlert {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_alerts WHERE project_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_unacknowledged(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_alerts WHERE project_id = ? AND acknowledged = 0 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreatePulseAlert) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO pulse_alerts (id, project_id, organization_id, rule_id, content_item_id, priority, auto_task_id)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(data.project_id)
        .bind(data.organization_id)
        .bind(&data.rule_id)
        .bind(&data.content_item_id)
        .bind(&data.priority)
        .bind(data.auto_task_id)
        .fetch_one(pool)
        .await
    }

    pub async fn acknowledge(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE pulse_alerts SET acknowledged = 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn count_unacknowledged(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pulse_alerts WHERE project_id = ? AND acknowledged = 0",
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }
}

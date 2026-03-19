use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TriggerExecution {
    pub id: String,
    pub trigger_id: String,
    pub workflow_run_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
    pub records_staged: i64,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTriggerExecution {
    pub trigger_id: String,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub metadata: Option<String>,
}

impl TriggerExecution {
    pub async fn create(
        pool: &SqlitePool,
        input: CreateTriggerExecution,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO trigger_executions (id, trigger_id, status, source_type, source_id, metadata)
               VALUES (?1, ?2, 'pending', ?3, ?4, ?5)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(&input.trigger_id)
        .bind(&input.source_type)
        .bind(&input.source_id)
        .bind(&input.metadata)
        .fetch_one(pool)
        .await
    }

    pub async fn mark_running(
        pool: &SqlitePool,
        id: &str,
        workflow_run_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE trigger_executions
               SET status = 'running', workflow_run_id = ?2
               WHERE id = ?1"#,
        )
        .bind(id)
        .bind(workflow_run_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_completed(
        pool: &SqlitePool,
        id: &str,
        records_staged: i64,
        duration_ms: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE trigger_executions
               SET status = 'completed',
                   completed_at = datetime('now', 'subsec'),
                   duration_ms = ?2,
                   records_staged = ?3
               WHERE id = ?1"#,
        )
        .bind(id)
        .bind(duration_ms)
        .bind(records_staged)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(
        pool: &SqlitePool,
        id: &str,
        error: &str,
        duration_ms: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE trigger_executions
               SET status = 'failed',
                   completed_at = datetime('now', 'subsec'),
                   duration_ms = ?2,
                   error = ?3
               WHERE id = ?1"#,
        )
        .bind(id)
        .bind(duration_ms)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_trigger(
        pool: &SqlitePool,
        trigger_id: &str,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM trigger_executions
               WHERE trigger_id = ?1
               ORDER BY started_at DESC
               LIMIT ?2"#,
        )
        .bind(trigger_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_recent(pool: &SqlitePool, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM trigger_executions
               ORDER BY started_at DESC
               LIMIT ?1"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

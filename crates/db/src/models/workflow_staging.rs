use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowStagingRecord {
    pub id: Uuid,
    pub workflow_run_id: Uuid,
    pub workflow_id: String,
    pub node_id: String,
    pub data_source_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub target_type: String,
    pub record_data: String,        // JSON string
    pub status: String,             // pending_review, approved, rejected, committed, error
    pub duplicate_of_id: Option<Uuid>,
    pub duplicate_of_type: Option<String>,
    pub confidence: Option<f64>,
    pub error_message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub committed_at: Option<DateTime<Utc>>,
    pub validation_errors: Option<String>,  // JSON array string of validation errors, or null
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStagingRecord {
    pub workflow_run_id: Uuid,
    pub workflow_id: String,
    pub node_id: String,
    pub data_source_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub target_type: String,
    pub record_data: Value,
    pub duplicate_of_id: Option<Uuid>,
    pub duplicate_of_type: Option<String>,
    pub confidence: Option<f64>,
    pub validation_errors: Option<Vec<String>>,
}

impl WorkflowStagingRecord {
    pub async fn create(pool: &SqlitePool, data: CreateStagingRecord) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let record_data_str = data.record_data.to_string();
        let validation_errors_str = data.validation_errors.map(|errs| {
            serde_json::to_string(&errs).unwrap_or_else(|_| "[]".to_string())
        });

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workflow_output_staging
               (id, workflow_run_id, workflow_id, node_id, data_source_id, organization_id, project_id,
                target_type, record_data, status, duplicate_of_id, duplicate_of_type, confidence, validation_errors)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending_review', ?10, ?11, ?12, ?13)
               RETURNING *"#,
        )
        .bind(id)
        .bind(data.workflow_run_id)
        .bind(&data.workflow_id)
        .bind(&data.node_id)
        .bind(data.data_source_id)
        .bind(data.organization_id)
        .bind(data.project_id)
        .bind(&data.target_type)
        .bind(&record_data_str)
        .bind(data.duplicate_of_id)
        .bind(&data.duplicate_of_type)
        .bind(data.confidence)
        .bind(&validation_errors_str)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_run(pool: &SqlitePool, workflow_run_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_output_staging WHERE workflow_run_id = ?1 ORDER BY created_at ASC",
        )
        .bind(workflow_run_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_org_pending(pool: &SqlitePool, organization_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_output_staging WHERE organization_id = ?1 AND status = 'pending_review' ORDER BY created_at ASC",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_output_staging WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        reviewed_by: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE workflow_output_staging
               SET status = ?1, reviewed_by = ?2, reviewed_at = datetime('now', 'subsec'),
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?3
               RETURNING *"#,
        )
        .bind(status)
        .bind(reviewed_by)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn mark_committed(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE workflow_output_staging
               SET status = 'committed', committed_at = datetime('now', 'subsec'),
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?1"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_error(pool: &SqlitePool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE workflow_output_staging
               SET status = 'error', error_message = ?1, updated_at = datetime('now', 'subsec')
               WHERE id = ?2"#,
        )
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_record_data(pool: &SqlitePool, id: Uuid, record_data: &Value) -> Result<Self, sqlx::Error> {
        let data_str = record_data.to_string();
        sqlx::query_as::<_, Self>(
            r#"UPDATE workflow_output_staging
               SET record_data = ?1, updated_at = datetime('now', 'subsec')
               WHERE id = ?2
               RETURNING *"#,
        )
        .bind(&data_str)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub fn parsed_data(&self) -> Option<Value> {
        serde_json::from_str(&self.record_data).ok()
    }

    pub async fn auto_approve_valid(pool: &SqlitePool, workflow_run_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE workflow_output_staging
               SET status = 'approved', reviewed_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec')
               WHERE workflow_run_id = ?1 AND status = 'pending_review' AND duplicate_of_id IS NULL"#
        )
        .bind(workflow_run_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn reject_duplicates(pool: &SqlitePool, workflow_run_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE workflow_output_staging
               SET status = 'rejected', reviewed_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec')
               WHERE workflow_run_id = ?1 AND status = 'pending_review' AND duplicate_of_id IS NOT NULL"#
        )
        .bind(workflow_run_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

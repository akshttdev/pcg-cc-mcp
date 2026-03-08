use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowRun {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub data_source_id: Option<String>,
    pub organization_id: Option<String>,
    pub project_id: Option<String>,
    pub model_used: Option<String>,
    pub status: String,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub total_estimated_cost_micros: Option<i64>,
    pub total_records_staged: Option<i64>,
    pub total_records_approved: Option<i64>,
    pub total_records_rejected: Option<i64>,
    pub total_records_committed: Option<i64>,
    pub total_duplicates_found: Option<i64>,
    pub total_validation_errors: Option<i64>,
    pub node_count: Option<i64>,
    pub llm_node_count: Option<i64>,
    pub duration_ms: Option<i64>,
    pub content_hash: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRun {
    pub id: Uuid,
    pub workflow_id: String,
    pub workflow_name: String,
    pub data_source_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub model_used: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRunOnComplete {
    pub status: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_estimated_cost_micros: i64,
    pub total_records_staged: i64,
    pub total_duplicates_found: i64,
    pub node_count: i64,
    pub llm_node_count: i64,
    pub duration_ms: i64,
}

impl WorkflowRun {
    pub async fn create(pool: &SqlitePool, data: CreateWorkflowRun) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workflow_runs
               (id, workflow_id, workflow_name, data_source_id, organization_id, project_id, model_used, content_hash, status)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'running')
               RETURNING *"#,
        )
        .bind(data.id.to_string())
        .bind(&data.workflow_id)
        .bind(&data.workflow_name)
        .bind(data.data_source_id.map(|u| u.to_string()))
        .bind(data.organization_id.map(|u| u.to_string()))
        .bind(data.project_id.map(|u| u.to_string()))
        .bind(&data.model_used)
        .bind(&data.content_hash)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_content_hash(pool: &SqlitePool, hash: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_runs WHERE content_hash = ?1 AND status = 'completed' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM workflow_runs WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_workflow(pool: &SqlitePool, workflow_id: &str, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_runs WHERE workflow_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(workflow_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_organization(pool: &SqlitePool, organization_id: &str, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_runs WHERE organization_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(organization_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_recent(
        pool: &SqlitePool,
        limit: i64,
        workflow_id: Option<&str>,
        organization_id: Option<&str>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        if let Some(wid) = workflow_id {
            Self::find_by_workflow(pool, wid, limit).await
        } else if let Some(oid) = organization_id {
            Self::find_by_organization(pool, oid, limit).await
        } else {
            sqlx::query_as::<_, Self>(
                "SELECT * FROM workflow_runs ORDER BY created_at DESC LIMIT ?1",
            )
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }

    pub async fn update_on_complete(
        pool: &SqlitePool,
        id: &str,
        data: UpdateWorkflowRunOnComplete,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE workflow_runs
               SET status = ?1,
                   total_input_tokens = ?2,
                   total_output_tokens = ?3,
                   total_estimated_cost_micros = ?4,
                   total_records_staged = ?5,
                   total_duplicates_found = ?6,
                   node_count = ?7,
                   llm_node_count = ?8,
                   duration_ms = ?9,
                   completed_at = datetime('now', 'subsec')
               WHERE id = ?10
               RETURNING *"#,
        )
        .bind(&data.status)
        .bind(data.total_input_tokens)
        .bind(data.total_output_tokens)
        .bind(data.total_estimated_cost_micros)
        .bind(data.total_records_staged)
        .bind(data.total_duplicates_found)
        .bind(data.node_count)
        .bind(data.llm_node_count)
        .bind(data.duration_ms)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn mark_failed(pool: &SqlitePool, id: &str, duration_ms: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE workflow_runs
               SET status = 'failed', duration_ms = ?1, completed_at = datetime('now', 'subsec')
               WHERE id = ?2"#,
        )
        .bind(duration_ms)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

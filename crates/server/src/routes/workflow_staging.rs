//! Workflow Output Staging Routes
//!
//! Manages staged records extracted by workflows, pending human review
//! before committing to CRM contacts, companies, deals, or tasks.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, patch},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::workflow_staging::{WorkflowStagingRecord, CreateStagingRecord};
use db::models::crm_contact::{CrmContact, CreateCrmContact, ContactSource, LifecycleStage};
use db::models::company::{Company, CreateCompany};
use db::models::crm_deal::{CrmDeal, CreateCrmDeal};
use db::models::crm_pipeline::{CrmPipeline, CrmPipelineStage};
use db::models::task::{Task, CreateTask, Priority};

// ── Query params ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ListStagingQuery {
    workflow_run_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct PendingQuery {
    organization_id: Uuid,
}

// ── Update request ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UpdateStagingRequest {
    status: Option<String>,       // "approved" or "rejected"
    record_data: Option<Value>,   // edited record data
    reviewed_by: Option<Uuid>,
}

// ── Batch types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BatchAction {
    ids: Vec<Uuid>,
    action: String,  // "approve" or "reject"
}

#[derive(Debug, Deserialize)]
struct BatchCommitRequest {
    workflow_run_id: Uuid,
}

#[derive(Debug, Serialize)]
struct CommitResult {
    id: Uuid,
    target_type: String,
    created_id: Option<Uuid>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchCommitResult {
    committed: i64,
    errors: i64,
    results: Vec<CommitResult>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/workflow-staging?workflow_run_id=X
async fn list_staging_records(
    Query(params): Query<ListStagingQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowStagingRecord>>>, ApiError> {
    let pool = &deployment.db().pool;
    let records = WorkflowStagingRecord::find_by_run(pool, params.workflow_run_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list staging records: {e}")))?;
    Ok(Json(ApiResponse::success(records)))
}

/// GET /api/workflow-staging/pending?organization_id=X
async fn list_pending_records(
    Query(params): Query<PendingQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowStagingRecord>>>, ApiError> {
    let pool = &deployment.db().pool;
    let records = WorkflowStagingRecord::find_by_org_pending(pool, params.organization_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list pending records: {e}")))?;
    Ok(Json(ApiResponse::success(records)))
}

/// GET /api/workflow-staging/:id
async fn get_staging_record(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<WorkflowStagingRecord>>, ApiError> {
    let pool = &deployment.db().pool;
    let record = WorkflowStagingRecord::find_by_id(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get staging record: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Staging record not found".to_string()))?;
    Ok(Json(ApiResponse::success(record)))
}

/// PATCH /api/workflow-staging/:id
async fn update_staging_record(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<UpdateStagingRequest>,
) -> Result<Json<ApiResponse<WorkflowStagingRecord>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify record exists
    WorkflowStagingRecord::find_by_id(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Staging record not found".to_string()))?;

    // Update record_data if provided
    if let Some(ref data) = req.record_data {
        WorkflowStagingRecord::update_record_data(pool, id, data)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to update record data: {e}")))?;
    }

    // Update status if provided
    let record = if let Some(ref status) = req.status {
        if !["approved", "rejected", "pending_review"].contains(&status.as_str()) {
            return Err(ApiError::BadRequest(
                "status must be 'approved', 'rejected', or 'pending_review'".to_string(),
            ));
        }
        WorkflowStagingRecord::update_status(pool, id, status, req.reviewed_by)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to update status: {e}")))?
    } else {
        WorkflowStagingRecord::find_by_id(pool, id)
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
            .ok_or_else(|| ApiError::NotFound("Staging record not found".to_string()))?
    };

    Ok(Json(ApiResponse::success(record)))
}

/// POST /api/workflow-staging/batch
async fn batch_action(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<BatchAction>,
) -> Result<Json<ApiResponse<Vec<WorkflowStagingRecord>>>, ApiError> {
    let pool = &deployment.db().pool;
    let status = match req.action.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        _ => return Err(ApiError::BadRequest("action must be 'approve' or 'reject'".to_string())),
    };

    let mut results = Vec::new();
    for id in req.ids {
        let record = WorkflowStagingRecord::update_status(pool, id, status, None)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to update record {id}: {e}")))?;
        results.push(record);
    }

    Ok(Json(ApiResponse::success(results)))
}

/// POST /api/workflow-staging/:id/commit
async fn commit_single(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<CommitResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let record = WorkflowStagingRecord::find_by_id(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Staging record not found".to_string()))?;

    if record.status != "approved" {
        return Err(ApiError::BadRequest(
            "Record must be approved before committing".to_string(),
        ));
    }

    let result = commit_record(pool, &record).await;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/workflow-staging/batch-commit
async fn batch_commit(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<BatchCommitRequest>,
) -> Result<Json<ApiResponse<BatchCommitResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let records = WorkflowStagingRecord::find_by_run(pool, req.workflow_run_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list records: {e}")))?;

    let approved: Vec<_> = records.into_iter().filter(|r| r.status == "approved").collect();
    let mut committed: i64 = 0;
    let mut errors: i64 = 0;
    let mut results = Vec::new();

    for record in &approved {
        let result = commit_record(pool, record).await;
        if result.error.is_some() {
            errors += 1;
        } else {
            committed += 1;
        }
        results.push(result);
    }

    Ok(Json(ApiResponse::success(BatchCommitResult {
        committed,
        errors,
        results,
    })))
}

// ── Commit logic ────────────────────────────────────────────────────────────

async fn commit_record(pool: &SqlitePool, record: &WorkflowStagingRecord) -> CommitResult {
    let result = match record.target_type.as_str() {
        "crm_contact" => commit_contact(pool, record).await,
        "company" => commit_company(pool, record).await,
        "crm_deal" => commit_deal(pool, record).await,
        "task" => commit_task(pool, record).await,
        _ => Err(format!("Unknown target type: {}", record.target_type)),
    };

    match result {
        Ok(created_id) => {
            let _ = WorkflowStagingRecord::mark_committed(pool, record.id).await;
            CommitResult {
                id: record.id,
                target_type: record.target_type.clone(),
                created_id: Some(created_id),
                error: None,
            }
        }
        Err(err) => {
            let _ = WorkflowStagingRecord::mark_error(pool, record.id, &err).await;
            CommitResult {
                id: record.id,
                target_type: record.target_type.clone(),
                created_id: None,
                error: Some(err),
            }
        }
    }
}

async fn commit_contact(pool: &SqlitePool, record: &WorkflowStagingRecord) -> Result<Uuid, String> {
    let data: Value = serde_json::from_str(&record.record_data).map_err(|e| e.to_string())?;
    let project_id = record.project_id.ok_or("No project_id set")?;

    let lifecycle_stage = data["lifecycle_stage"]
        .as_str()
        .and_then(|s| s.parse::<LifecycleStage>().ok());

    let tags: Option<Vec<String>> = data["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    let create = CreateCrmContact {
        project_id,
        first_name: data["first_name"].as_str().map(|s| s.to_string()),
        last_name: data["last_name"].as_str().map(|s| s.to_string()),
        email: data["email"].as_str().map(|s| s.to_string()),
        phone: data["phone"].as_str().map(|s| s.to_string()),
        mobile: data["mobile"].as_str().map(|s| s.to_string()),
        avatar_url: None,
        company_name: data["company_name"].as_str().map(|s| s.to_string()),
        job_title: data["job_title"].as_str().map(|s| s.to_string()),
        department: data["department"].as_str().map(|s| s.to_string()),
        linkedin_url: data["linkedin_url"].as_str().map(|s| s.to_string()),
        twitter_handle: data["twitter_handle"].as_str().map(|s| s.to_string()),
        website: data["website"].as_str().map(|s| s.to_string()),
        source: Some(ContactSource::Workflow),
        lifecycle_stage,
        tags,
        custom_fields: None,
        zoho_contact_id: None,
        gmail_contact_id: None,
    };

    let contact = CrmContact::create(pool, create).await.map_err(|e| e.to_string())?;
    Ok(contact.id)
}

async fn commit_company(pool: &SqlitePool, record: &WorkflowStagingRecord) -> Result<Uuid, String> {
    let data: Value = serde_json::from_str(&record.record_data).map_err(|e| e.to_string())?;
    let name = data["name"].as_str().ok_or("Company name is required")?;

    let company = Company::find_or_create(
        pool,
        name,
        record.organization_id,
        data["website"].as_str().map(|s| s.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(company.id)
}

async fn commit_deal(pool: &SqlitePool, record: &WorkflowStagingRecord) -> Result<Uuid, String> {
    let data: Value = serde_json::from_str(&record.record_data).map_err(|e| e.to_string())?;
    let project_id = record.project_id.ok_or("No project_id set")?;
    let name = data["name"].as_str().ok_or("Deal name is required")?.to_string();

    // Find a sales pipeline for this project
    let pipelines = CrmPipeline::find_by_project(pool, project_id)
        .await
        .map_err(|e| e.to_string())?;

    let pipeline = pipelines
        .iter()
        .find(|p| p.pipeline_type == "sales")
        .or_else(|| pipelines.iter().find(|p| p.is_default == Some(1)))
        .or_else(|| pipelines.first());

    let (pipeline_id, stage_id) = if let Some(p) = pipeline {
        let stages = CrmPipelineStage::find_by_pipeline(pool, p.id)
            .await
            .map_err(|e| e.to_string())?;
        let first_stage_id = stages.first().map(|s| s.id);
        (Some(p.id), first_stage_id)
    } else {
        (None, None)
    };

    let tags: Option<Vec<String>> = data["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    let create = CreateCrmDeal {
        project_id,
        crm_contact_id: None,
        crm_pipeline_id: pipeline_id,
        crm_stage_id: stage_id,
        name,
        description: data["description"].as_str().map(|s| s.to_string()),
        amount: data["amount"].as_f64(),
        currency: data["currency"].as_str().map(|s| s.to_string()),
        expected_close_date: data["expected_close_date"].as_str().map(|s| s.to_string()),
        tags,
        custom_fields: None,
    };

    let deal = CrmDeal::create(pool, create).await.map_err(|e| e.to_string())?;
    Ok(deal.id)
}

async fn commit_task(pool: &SqlitePool, record: &WorkflowStagingRecord) -> Result<Uuid, String> {
    let data: Value = serde_json::from_str(&record.record_data).map_err(|e| e.to_string())?;
    let project_id = record.project_id.ok_or("No project_id set")?;
    let title = data["title"].as_str().ok_or("Task title is required")?.to_string();

    let priority = data["priority"]
        .as_str()
        .and_then(|s| match s.to_lowercase().as_str() {
            "critical" => Some(Priority::Critical),
            "high" => Some(Priority::High),
            "medium" => Some(Priority::Medium),
            "low" => Some(Priority::Low),
            _ => None,
        });

    let tags: Option<Vec<String>> = data["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    let task_id = Uuid::new_v4();
    let create = CreateTask {
        project_id,
        pod_id: None,
        board_id: None,
        title,
        description: data["description"].as_str().map(|s| s.to_string()),
        parent_task_attempt: None,
        image_ids: None,
        priority,
        assignee_id: None,
        assigned_agent: None,
        agent_id: None,
        assigned_mcps: None,
        created_by: "workflow".to_string(),
        requires_approval: None,
        parent_task_id: None,
        tags,
        due_date: None,
        custom_properties: None,
        scheduled_start: None,
        scheduled_end: None,
        screenshot: None,
    };

    let task = Task::create(pool, &create, task_id).await.map_err(|e| e.to_string())?;
    Ok(task.id)
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/workflow-staging", get(list_staging_records))
        .route("/workflow-staging/pending", get(list_pending_records))
        .route("/workflow-staging/batch", post(batch_action))
        .route("/workflow-staging/batch-commit", post(batch_commit))
        .route("/workflow-staging/{id}", get(get_staging_record).patch(update_staging_record))
        .route("/workflow-staging/{id}/commit", post(commit_single))
}

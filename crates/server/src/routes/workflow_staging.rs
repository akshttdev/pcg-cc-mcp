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
use db::models::workflow_staging::WorkflowStagingRecord;
use db::models::crm_contact::{CrmContact, CreateCrmContact, UpdateCrmContact, ContactSource, LifecycleStage};
use db::models::company::Company;
use db::models::crm_deal::{CrmDeal, CreateCrmDeal, UpdateCrmDeal};
use db::models::crm_pipeline::{CrmPipeline, CrmPipelineStage, CreateCrmPipeline, PipelineType, CreateCrmPipelineStage};
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
    let organization_id = record.organization_id.ok_or("No organization_id set")?;

    let lifecycle_stage = data["lifecycle_stage"]
        .as_str()
        .and_then(|s| s.parse::<LifecycleStage>().ok());

    let tags: Option<Vec<String>> = data["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    // Build source traceability custom_fields, merging with any existing custom_fields from data
    let custom_fields = build_source_custom_fields(record, &data);

    // Parse name fields — staging data may have a single "name" field instead of first/last
    let (first_name, last_name) = if data["first_name"].is_string() || data["last_name"].is_string() {
        (
            data["first_name"].as_str().map(|s| s.to_string()),
            data["last_name"].as_str().map(|s| s.to_string()),
        )
    } else if let Some(full) = data["name"].as_str() {
        let parts: Vec<&str> = full.splitn(2, ' ').collect();
        match parts.len() {
            1 => (Some(parts[0].to_string()), None),
            _ => (Some(parts[0].to_string()), Some(parts[1].to_string())),
        }
    } else {
        (None, None)
    };

    // Fallback: "company" → "company_name", "role" → "job_title"
    // Treat "Unknown" (case-insensitive) as None
    let company_name = data["company_name"].as_str()
        .or_else(|| data["company"].as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.eq_ignore_ascii_case("unknown"));
    let job_title = data["job_title"].as_str()
        .or_else(|| data["role"].as_str())
        .map(|s| s.to_string());

    let email = data["email"].as_str().map(|s| s.to_string());

    // Email-based deduplication: check if a contact with the same email already exists
    if let Some(ref email_str) = email {
        if !email_str.trim().is_empty() {
            if let Ok(Some(existing)) = CrmContact::find_by_email(pool, organization_id, email_str).await {
                tracing::info!(
                    existing_contact_id = %existing.id,
                    email = %email_str,
                    "Merged staging contact into existing contact {}",
                    existing.id
                );

                // Build update with only new/different fields
                let update = UpdateCrmContact {
                    first_name: first_name.filter(|v| Some(v.as_str()) != existing.first_name.as_deref()),
                    last_name: last_name.filter(|v| Some(v.as_str()) != existing.last_name.as_deref()),
                    email: None, // same email, no need to update
                    phone: data["phone"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.phone.as_deref()),
                    mobile: data["mobile"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.mobile.as_deref()),
                    avatar_url: data["avatar_url"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.avatar_url.as_deref()),
                    company_name: company_name.clone().filter(|v| Some(v.as_str()) != existing.company_name.as_deref()),
                    job_title: job_title.clone().filter(|v| Some(v.as_str()) != existing.job_title.as_deref()),
                    department: data["department"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.department.as_deref()),
                    linkedin_url: data["linkedin_url"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.linkedin_url.as_deref()),
                    twitter_handle: data["twitter_handle"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.twitter_handle.as_deref()),
                    website: data["website"].as_str().map(|s| s.to_string()).filter(|v| Some(v.as_str()) != existing.website.as_deref()),
                    source: None, // preserve existing source
                    lifecycle_stage,
                    tags,
                    custom_fields,
                    ..Default::default()
                };

                CrmContact::update(pool, existing.id, update).await.map_err(|e| e.to_string())?;

                // Auto-link company for existing contact too
                auto_link_company(pool, existing.id, &company_name, &data, record.organization_id).await;

                return Ok(existing.id);
            }
        }
    }

    // No existing contact found — create new
    let create = CreateCrmContact {
        organization_id,
        client_id: None,
        first_name,
        last_name,
        email,
        phone: data["phone"].as_str().map(|s| s.to_string()),
        mobile: data["mobile"].as_str().map(|s| s.to_string()),
        avatar_url: data["avatar_url"].as_str().map(|s| s.to_string()),
        company_name: company_name.clone(),
        job_title,
        department: data["department"].as_str().map(|s| s.to_string()),
        linkedin_url: data["linkedin_url"].as_str().map(|s| s.to_string()),
        twitter_handle: data["twitter_handle"].as_str().map(|s| s.to_string()),
        website: data["website"].as_str().map(|s| s.to_string()),
        source: Some(ContactSource::Workflow),
        lifecycle_stage,
        tags,
        custom_fields,
        zoho_contact_id: None,
        gmail_contact_id: None,
    };

    let contact = CrmContact::create(pool, create).await.map_err(|e| e.to_string())?;

    // Auto-link: if company_name is present, find or create the Company record
    auto_link_company(pool, contact.id, &company_name, &data, record.organization_id).await;

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

    // Update additional fields if present in record data (find_or_create only sets name/website)
    let has_extra_fields = data["industry"].as_str().is_some()
        || data["description"].as_str().is_some()
        || data["logo_url"].as_str().is_some()
        || data["headquarters"].as_str().is_some();

    if has_extra_fields {
        use db::models::company::UpdateCompany;
        let update = UpdateCompany {
            industry: data["industry"].as_str().map(|s| s.to_string()),
            description: data["description"].as_str().map(|s| s.to_string()),
            logo_url: data["logo_url"].as_str().map(|s| s.to_string()),
            headquarters: data["headquarters"].as_str().map(|s| s.to_string()),
            ..Default::default()
        };
        if let Err(e) = Company::update(pool, company.id, update).await {
            tracing::warn!(
                company_id = %company.id,
                error = %e,
                "Failed to update company with additional fields from workflow"
            );
        }
    }

    Ok(company.id)
}

async fn commit_deal(pool: &SqlitePool, record: &WorkflowStagingRecord) -> Result<Uuid, String> {
    let data: Value = serde_json::from_str(&record.record_data).map_err(|e| e.to_string())?;
    let organization_id = record.organization_id.ok_or("No organization_id set")?;
    let name = data["name"].as_str()
        .or_else(|| data["title"].as_str())
        .ok_or("Deal name is required")?.to_string();

    // Find a sales pipeline for this organization
    let pipelines = CrmPipeline::find_by_organization(pool, organization_id, Some(PipelineType::Sales))
        .await
        .map_err(|e| e.to_string())?;

    let pipeline = pipelines
        .iter()
        .find(|p| p.is_default == Some(1))
        .or_else(|| pipelines.first());

    let (pipeline_id, stage_id) = if let Some(p) = pipeline {
        let stages = CrmPipelineStage::find_by_pipeline(pool, p.id)
            .await
            .map_err(|e| e.to_string())?;
        let first_stage_id = stages.first().map(|s| s.id);
        (Some(p.id), first_stage_id)
    } else {
        // No pipeline exists — create a default "Sales Pipeline" for this organization
        tracing::warn!(
            organization_id = %organization_id,
            "No pipeline found for organization, creating default Sales Pipeline"
        );
        match create_default_sales_pipeline(pool, organization_id).await {
            Ok((pid, sid)) => (Some(pid), sid),
            Err(e) => {
                tracing::error!(
                    organization_id = %organization_id,
                    error = %e,
                    "Failed to create default Sales Pipeline — deal will have no pipeline"
                );
                (None, None)
            }
        }
    };

    // Auto-link: look up CRM contact by email or name within the same organization
    let crm_contact_id = resolve_deal_contact(pool, organization_id, &data).await;

    // Deal deduplication: check for existing deal with same name + contact + pipeline
    if let (Some(contact_id), Some(pip_id)) = (crm_contact_id, pipeline_id) {
        if let Ok(Some(existing_deal)) = CrmDeal::find_by_name_contact_pipeline(pool, &name, contact_id, pip_id).await {
            tracing::info!(
                existing_deal_id = %existing_deal.id,
                deal_name = %name,
                "Merged staging deal into existing deal {}",
                existing_deal.id
            );

            // Build source traceability custom_fields
            let custom_fields = build_source_custom_fields(record, &data);
            let tags: Option<Vec<String>> = data["tags"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

            let update = UpdateCrmDeal {
                description: data["description"].as_str().map(|s| s.to_string()),
                amount: data["amount"].as_f64(),
                currency: data["currency"].as_str().map(|s| s.to_string()),
                expected_close_date: data["expected_close_date"].as_str().map(|s| s.to_string()),
                tags,
                custom_fields,
                ..Default::default()
            };

            CrmDeal::update(pool, existing_deal.id, update).await.map_err(|e| e.to_string())?;
            return Ok(existing_deal.id);
        }
    }

    let tags: Option<Vec<String>> = data["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    // Build source traceability custom_fields, merging with any existing custom_fields from data
    let custom_fields = build_source_custom_fields(record, &data);

    let create = CreateCrmDeal {
        organization_id,
        client_id: None,
        crm_contact_id,
        crm_pipeline_id: pipeline_id,
        crm_stage_id: stage_id,
        name,
        description: data["description"].as_str().map(|s| s.to_string()),
        amount: data["amount"].as_f64(),
        currency: data["currency"].as_str().map(|s| s.to_string()),
        expected_close_date: data["expected_close_date"].as_str().map(|s| s.to_string()),
        tags,
        custom_fields,
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

    // Build source traceability as custom_properties
    let custom_properties = {
        let mut props = if let Some(existing) = data["custom_properties"].as_object() {
            existing.clone()
        } else {
            serde_json::Map::new()
        };
        if let Some(ds_id) = &record.data_source_id {
            props.insert("source_data_source_id".to_string(), serde_json::json!(ds_id.to_string()));
        }
        props.insert("source_workflow_run_id".to_string(), serde_json::json!(record.workflow_run_id.to_string()));
        Some(Value::Object(props))
    };

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
        assignee_type: None,
        assigned_agent: None,
        agent_id: None,
        assigned_mcps: None,
        created_by: "workflow".to_string(),
        requires_approval: None,
        parent_task_id: None,
        tags,
        due_date: None,
        custom_properties,
        scheduled_start: None,
        scheduled_end: None,
        screenshot: None,
    };

    let task = Task::create(pool, &create, task_id).await.map_err(|e| e.to_string())?;
    Ok(task.id)
}

// ── Helper functions ──────────────────────────────────────────────────────

/// Auto-link a contact to a company by name (find or create).
async fn auto_link_company(
    pool: &SqlitePool,
    contact_id: Uuid,
    company_name: &Option<String>,
    data: &Value,
    organization_id: Option<Uuid>,
) {
    if let Some(name) = company_name {
        if !name.trim().is_empty() {
            let company_website = data["website"].as_str().map(|s| s.to_string());
            match Company::find_or_create(pool, name, organization_id, company_website).await {
                Ok(company) => {
                    tracing::info!(
                        contact_id = %contact_id,
                        company_id = %company.id,
                        company_name = %name,
                        "Auto-linked contact to company"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        contact_id = %contact_id,
                        company_name = %name,
                        error = %e,
                        "Failed to auto-link contact to company"
                    );
                }
            }
        }
    }
}

/// Build custom_fields JSON with source traceability info, merging with any existing custom_fields.
fn build_source_custom_fields(record: &WorkflowStagingRecord, data: &Value) -> Option<Value> {
    let mut fields = if let Some(existing) = data["custom_fields"].as_object() {
        existing.clone()
    } else {
        serde_json::Map::new()
    };

    if let Some(ds_id) = &record.data_source_id {
        fields.insert("source_data_source_id".to_string(), serde_json::json!(ds_id.to_string()));
    }
    fields.insert("source_workflow_run_id".to_string(), serde_json::json!(record.workflow_run_id.to_string()));

    Some(Value::Object(fields))
}

/// Resolve a CRM contact for a deal by looking up contact_email or contact_name in the organization.
async fn resolve_deal_contact(pool: &SqlitePool, organization_id: Uuid, data: &Value) -> Option<Uuid> {
    // Try by email first
    if let Some(email) = data["contact_email"].as_str() {
        if !email.trim().is_empty() {
            match CrmContact::find_by_email(pool, organization_id, email).await {
                Ok(Some(contact)) => {
                    tracing::info!(
                        deal_contact_email = email,
                        contact_id = %contact.id,
                        "Auto-linked deal to contact by email"
                    );
                    return Some(contact.id);
                }
                Ok(None) => {
                    tracing::debug!(
                        deal_contact_email = email,
                        "No contact found with email for deal auto-link"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        deal_contact_email = email,
                        error = %e,
                        "Error looking up contact by email for deal"
                    );
                }
            }
        }
    }

    // Fallback: try by contact_name (search by full_name)
    if let Some(name) = data["contact_name"].as_str() {
        if !name.trim().is_empty() {
            let search_params = db::models::crm_contact::ContactSearchParams {
                organization_id: Some(organization_id),
                client_id: None,
                query: Some(name.to_string()),
                lifecycle_stage: None,
                company_name: None,
                tags: None,
                min_lead_score: None,
                limit: Some(1),
                offset: None,
            };
            match CrmContact::search(pool, search_params).await {
                Ok(contacts) if !contacts.is_empty() => {
                    tracing::info!(
                        deal_contact_name = name,
                        contact_id = %contacts[0].id,
                        "Auto-linked deal to contact by name"
                    );
                    return Some(contacts[0].id);
                }
                Ok(_) => {
                    tracing::debug!(
                        deal_contact_name = name,
                        "No contact found with name for deal auto-link"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        deal_contact_name = name,
                        error = %e,
                        "Error searching contact by name for deal"
                    );
                }
            }
        }
    }

    None
}

/// Create a default Sales Pipeline with standard stages for an organization.
async fn create_default_sales_pipeline(pool: &SqlitePool, organization_id: Uuid) -> Result<(Uuid, Option<Uuid>), String> {
    let pipeline = CrmPipeline::create(pool, CreateCrmPipeline {
        organization_id: Some(organization_id),
        client_id: None,
        name: "Sales Pipeline".to_string(),
        description: Some("Auto-created by workflow commit".to_string()),
        pipeline_type: PipelineType::Sales,
        icon: Some("trending-up".to_string()),
        color: Some("#3B82F6".to_string()),
    })
    .await
    .map_err(|e| e.to_string())?;

    let default_stages = vec![
        ("Lead", "#3B82F6", 0, false, false, 10),
        ("Qualified", "#F59E0B", 1, false, false, 30),
        ("Proposal", "#8B5CF6", 2, false, false, 60),
        ("Won", "#22C55E", 3, true, true, 100),
        ("Lost", "#9CA3AF", 4, true, false, 0),
    ];

    let mut first_stage_id = None;
    for (name, color, position, is_closed, is_won, probability) in default_stages {
        let stage = CrmPipelineStage::create(pool, CreateCrmPipelineStage {
            pipeline_id: pipeline.id,
            name: name.to_string(),
            description: None,
            color: color.to_string(),
            position,
            is_closed: Some(is_closed),
            is_won: Some(is_won),
            probability,
        })
        .await
        .map_err(|e| e.to_string())?;

        if first_stage_id.is_none() {
            first_stage_id = Some(stage.id);
        }
    }

    Ok((pipeline.id, first_stage_id))
}

// ── Batch convenience endpoints ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RunIdRequest {
    workflow_run_id: Uuid,
}

#[derive(Debug, Serialize)]
struct BulkActionResult {
    affected: u64,
}

/// POST /api/workflow-staging/auto-approve — approve all non-duplicate records for a run
async fn auto_approve_valid(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<RunIdRequest>,
) -> Result<Json<ApiResponse<BulkActionResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let affected = WorkflowStagingRecord::auto_approve_valid(pool, req.workflow_run_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to auto-approve records: {e}")))?;
    Ok(Json(ApiResponse::success(BulkActionResult { affected })))
}

/// POST /api/workflow-staging/reject-duplicates — reject all duplicate records for a run
async fn reject_duplicates(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<RunIdRequest>,
) -> Result<Json<ApiResponse<BulkActionResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let affected = WorkflowStagingRecord::reject_duplicates(pool, req.workflow_run_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to reject duplicates: {e}")))?;
    Ok(Json(ApiResponse::success(BulkActionResult { affected })))
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/workflow-staging", get(list_staging_records))
        .route("/workflow-staging/pending", get(list_pending_records))
        .route("/workflow-staging/batch", post(batch_action))
        .route("/workflow-staging/batch-commit", post(batch_commit))
        .route("/workflow-staging/auto-approve", post(auto_approve_valid))
        .route("/workflow-staging/reject-duplicates", post(reject_duplicates))
        .route("/workflow-staging/{id}", get(get_staging_record).patch(update_staging_record))
        .route("/workflow-staging/{id}/commit", post(commit_single))
}

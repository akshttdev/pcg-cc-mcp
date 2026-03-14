//! Workflow Output Staging Routes
//!
//! Manages staged records extracted by workflows, pending human review
//! before committing to CRM contacts, companies, deals, or tasks.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post},
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

/// Normalize a date or datetime string to RFC3339 format.
/// Handles "2026-08-01" → "2026-08-01T00:00:00Z" and passes through already-valid datetimes.
fn normalize_datetime(s: &str) -> String {
    let trimmed = s.trim();
    // If it looks like a date-only string (YYYY-MM-DD), append time component
    if trimmed.len() == 10 && trimmed.chars().nth(4) == Some('-') && trimmed.chars().nth(7) == Some('-') {
        format!("{}T00:00:00Z", trimmed)
    } else {
        trimmed.to_string()
    }
}
use db::models::crm_contact::{CrmContact, CreateCrmContact, UpdateCrmContact, ContactSource, LifecycleStage};
use db::models::company::Company;
use db::models::crm_deal::{CrmDeal, CreateCrmDeal, UpdateCrmDeal};
use db::models::crm_pipeline::{CrmPipeline, CrmPipelineStage, CreateCrmPipeline, PipelineType, CreateCrmPipelineStage};
use db::models::task::{Task, CreateTask, Priority};

// ── Pre-commit validation ───────────────────────────────────────────────────

/// Validate a staging record's data for common issues that would cause commit-time failures.
/// Returns a list of human-readable warning strings (empty if valid).
pub fn validate_staging_record(target_type: &str, data: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let obj = match data.as_object() {
        Some(o) => o,
        None => {
            errors.push("Record data is not a JSON object".to_string());
            return errors;
        }
    };

    match target_type {
        "crm_deal" => {
            // Check name
            let name = obj.get("name").or_else(|| obj.get("title"));
            match name {
                None | Some(Value::Null) => {
                    errors.push("Deal is missing 'name' (or 'title') field".to_string());
                }
                Some(Value::String(s)) if s.trim().is_empty() => {
                    errors.push("Deal 'name' is empty".to_string());
                }
                _ => {}
            }
            // Check expected_close_date format
            if let Some(Value::String(date_str)) = obj.get("expected_close_date") {
                let trimmed = date_str.trim();
                if !trimmed.is_empty() {
                    // Warn if it looks like a date-only string (YYYY-MM-DD) rather than RFC3339
                    if trimmed.len() == 10
                        && trimmed.chars().nth(4) == Some('-')
                        && trimmed.chars().nth(7) == Some('-')
                    {
                        errors.push(format!(
                            "expected_close_date '{}' is date-only (not RFC3339) — will be auto-normalized to {}T00:00:00Z",
                            trimmed, trimmed
                        ));
                    }
                }
            }
        }
        "crm_contact" => {
            let has_email = matches!(obj.get("email"), Some(Value::String(s)) if !s.trim().is_empty());
            let has_full_name = matches!(obj.get("full_name"), Some(Value::String(s)) if !s.trim().is_empty());
            let has_first_name = matches!(obj.get("first_name"), Some(Value::String(s)) if !s.trim().is_empty());
            let has_name = matches!(obj.get("name"), Some(Value::String(s)) if !s.trim().is_empty());
            if !has_email && !has_full_name && !has_first_name && !has_name {
                errors.push("Contact is missing both email and name (full_name/first_name/name) — record may be unusable".to_string());
            }
        }
        "company" => {
            match obj.get("name") {
                None | Some(Value::Null) => {
                    errors.push("Company is missing 'name' field".to_string());
                }
                Some(Value::String(s)) if s.trim().is_empty() => {
                    errors.push("Company 'name' is empty".to_string());
                }
                _ => {}
            }
        }
        _ => {} // No extra validation for other types
    }

    errors
}


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
    let existing = WorkflowStagingRecord::find_by_id(pool, id)

        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Staging record not found".to_string()))?;

    // Update record_data if provided, and re-validate

    if let Some(ref data) = req.record_data {
        WorkflowStagingRecord::update_record_data(pool, id, data)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to update record data: {e}")))?;

        // Re-run pre-commit validation on the new data
        let validation_errs = validate_staging_record(&existing.target_type, data);
        let errs_opt = if validation_errs.is_empty() { None } else { Some(validation_errs) };
        WorkflowStagingRecord::update_validation_errors(pool, id, errs_opt.as_deref())
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to update validation errors: {e}")))?;

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

    let mut approved: Vec<_> = records.into_iter().filter(|r| r.status == "approved").collect();

    // Sort: companies first, then contacts, then deals, then tasks
    // This ensures companies exist before contacts try to link to them
    fn target_type_order(t: &str) -> u8 {
        match t {
            "company" => 0,
            "crm_contact" => 1,
            "crm_deal" => 2,
            "task" => 3,
            _ => 4,
        }
    }
    approved.sort_by_key(|r| target_type_order(&r.target_type));


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

    // Post-commit linking pass: link contacts to companies by matching company_name
    let org_id = approved.first().and_then(|r| r.organization_id);
    if let Some(org_id) = org_id {
        for result in &results {
            if result.target_type == "crm_contact" && result.created_id.is_some() && result.error.is_none() {
                let contact_id = result.created_id.unwrap();
                // Try to link the contact to a company by name
                if let Ok(contact) = CrmContact::find_by_id(pool, contact_id).await {
                    if let Some(company_name) = &contact.company_name {
                        if let Ok(Some(company)) = Company::find_by_name_and_org(pool, company_name, org_id).await {
                            // Store company_id in contact's custom_fields
                            let _ = store_company_id_in_custom_fields(pool, contact.id, company.id).await;
                        }
                    }
                }
            }
        }
    }

    // Auto-start execution for agent-assigned tasks
    for (result, record) in results.iter().zip(approved.iter()) {
        if result.target_type == "task" && result.error.is_none() {
            if let Some(task_id) = result.created_id {
                let data: Value = match serde_json::from_str(&record.record_data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if data["agent_id"].as_str().is_some() {
                    let pool = pool.clone();
                    let dep = deployment.clone();
                    tokio::spawn(async move {
                        if let Err(e) = auto_start_agent_execution(&pool, &dep, task_id).await {
                            tracing::warn!("Auto-execute for task {task_id} failed: {e}");
                        }
                    });
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(BatchCommitResult {
        committed,
        errors,
        results,
    })))
}

// ── Commit logic ────────────────────────────────────────────────────────────

/// Public commit function for use by auto-approve trigger flow.
/// Commits a single approved staging record and marks it committed/error in DB.
pub async fn commit_record_internal(pool: &SqlitePool, record: &WorkflowStagingRecord) -> Result<Uuid, String> {
    let result = match record.target_type.as_str() {
        "crm_contact" => commit_contact(pool, record).await,
        "company" => commit_company(pool, record).await,
        "crm_deal" => commit_deal(pool, record).await,
        "task" => commit_task(pool, record).await,
        _ => Err(format!("Unknown target type: {}", record.target_type)),
    };

    match &result {
        Ok(_) => { let _ = WorkflowStagingRecord::mark_committed(pool, record.id).await; }
        Err(err) => { let _ = WorkflowStagingRecord::mark_error(pool, record.id, err).await; }
    }

    result
}

async fn commit_record(pool: &SqlitePool, record: &WorkflowStagingRecord) -> CommitResult {
    match commit_record_internal(pool, record).await {
        Ok(created_id) => CommitResult {
            id: record.id,
            target_type: record.target_type.clone(),
            created_id: Some(created_id),
            error: None,
        },
        Err(err) => CommitResult {
            id: record.id,
            target_type: record.target_type.clone(),
            created_id: None,
            error: Some(err),
        },
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

    // Org-scoped dedup: if organization_id is available, check for an existing company
    // with the same name within that org before falling through to find_or_create.
    if let Some(org_id) = record.organization_id {
        if let Ok(Some(existing)) = Company::find_by_name_and_org(pool, name, org_id).await {
            tracing::info!(
                existing_company_id = %existing.id,
                company_name = %name,
                organization_id = %org_id,
                "Merged staging company into existing company {} (org-scoped dedup)",
                existing.id
            );
            update_company_extra_fields(pool, existing.id, &data).await;
            return Ok(existing.id);
        }
    }


    let company = Company::find_or_create(
        pool,
        name,
        record.organization_id,
        data["website"].as_str().map(|s| s.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;

    update_company_extra_fields(pool, company.id, &data).await;

    Ok(company.id)
}

/// Update additional fields on a company if present in staging record data.
async fn update_company_extra_fields(pool: &SqlitePool, company_id: Uuid, data: &Value) {
    let has_extra_fields = data["industry"].as_str().is_some()
        || data["description"].as_str().is_some()
        || data["logo_url"].as_str().is_some()
        || data["headquarters"].as_str().is_some()
        || data["relationship"].as_str().is_some()
        || data["context"].as_str().is_some();

    if has_extra_fields {
        use db::models::company::UpdateCompany;

        // Merge context into description if no explicit description provided
        let description = data["description"].as_str()
            .or_else(|| data["context"].as_str())
            .map(|s| s.to_string());

        // Store relationship as a tag (e.g. "relationship:partner")
        let tags = data["relationship"].as_str().map(|r| {
            let tag_list = vec![format!("relationship:{}", r)];
            serde_json::to_string(&tag_list).unwrap_or_default()
        });

        let update = UpdateCompany {
            industry: data["industry"].as_str().map(|s| s.to_string()),
            description,
            logo_url: data["logo_url"].as_str().map(|s| s.to_string()),
            headquarters: data["headquarters"].as_str().map(|s| s.to_string()),
            tags,
            ..Default::default()
        };
        if let Err(e) = Company::update(pool, company_id, update).await {
            tracing::warn!(
                company_id = %company_id,

                error = %e,
                "Failed to update company with additional fields from workflow"
            );
        }
    }

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
    let existing_deal = if let (Some(contact_id), Some(pip_id)) = (crm_contact_id, pipeline_id) {
        CrmDeal::find_by_name_contact_pipeline(pool, &name, contact_id, pip_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    // Fallback dedup: search by name + organization_id (catches partial-failure duplicates
    // where contact_id or pipeline_id may not match exactly)
    let existing_deal = match existing_deal {
        Some(deal) => Some(deal),
        None => CrmDeal::find_by_name_and_org(pool, &name, organization_id)
            .await
            .ok()
            .flatten(),
    };

    if let Some(existing_deal) = existing_deal {
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
            expected_close_date: data["expected_close_date"].as_str().map(|s| normalize_datetime(s)),
            tags,
            custom_fields,
            ..Default::default()
        };

        CrmDeal::update(pool, existing_deal.id, update).await.map_err(|e| e.to_string())?;
        return Ok(existing_deal.id);
    }


    let tags: Option<Vec<String>> = data["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    // Build source traceability custom_fields, merging with any existing custom_fields from data
    let custom_fields = build_source_custom_fields(record, &data);

    let deal_name = name.clone();
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
        expected_close_date: data["expected_close_date"].as_str().map(|s| normalize_datetime(s)),

        tags,
        custom_fields,
    };

    match CrmDeal::create(pool, create).await {
        Ok(deal) => Ok(deal.id),
        Err(e) => {
            // The INSERT may have succeeded but the RETURNING SELECT failed (e.g., datetime
            // decode error). Check if the deal was actually created before propagating the error.
            tracing::warn!(
                deal_name = %deal_name,
                error = %e,
                "CrmDeal::create failed — checking if deal was partially created"
            );
            if let Ok(Some(recovered_deal)) = CrmDeal::find_by_name_and_org(pool, &deal_name, organization_id).await {
                tracing::info!(
                    recovered_deal_id = %recovered_deal.id,
                    deal_name = %deal_name,
                    "Recovered partially-created deal {}",
                    recovered_deal.id
                );
                Ok(recovered_deal.id)
            } else {
                Err(e.to_string())
            }
        }
    }

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

    // Extract optional agent/assignee fields from workflow output data
    let assignee_id = data["assignee_id"].as_str().map(|s| s.to_string());
    let assignee_type = data["assignee_type"].as_str().map(|s| s.to_string());
    let assigned_agent = data["assigned_agent"].as_str().map(|s| s.to_string());
    let agent_id = data["agent_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());
    let board_id = data["board_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());
    let completion_criteria = data["completion_criteria"].as_str().map(|s| s.to_string());
    let output_format = data["output_format"].as_str().map(|s| s.to_string());

    let task_id = Uuid::new_v4();
    let create = CreateTask {
        project_id,
        pod_id: None,
        board_id,
        title,
        description: data["description"].as_str().map(|s| s.to_string()),
        parent_task_attempt: None,
        image_ids: None,
        priority,
        assignee_id,
        assignee_type,
        assigned_agent,
        agent_id,
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
        completion_criteria,
        output_format,
    };

    let task = Task::create(pool, &create, &task_id.to_string()).await.map_err(|e| e.to_string())?;
    Ok(Uuid::parse_str(&task.id).map_err(|e| e.to_string())?)
}

// ── Auto-execute for agent-assigned tasks ────────────────────────────────

/// When a workflow creates a task with an `agent_id`, automatically start
/// execution: look up the agent's execution config for the executor profile,
/// create a TaskAttempt, and start the container.
pub async fn auto_start_agent_execution(
    pool: &SqlitePool,
    deployment: &DeploymentImpl,
    task_id: Uuid,
) -> Result<(), String> {
    use db::models::agent_execution_config::AgentExecutionConfig;
    use db::models::task::Task;
    use db::models::task_attempt::{CreateTaskAttempt, TaskAttempt};
    use executors::executors::BaseCodingAgent;
    use executors::profile::ExecutorProfileId;
    use services::services::container::ContainerService;
    use std::str::FromStr;

    let task = Task::find_by_id(pool, &task_id.to_string())
        .await
        .map_err(|e| format!("Failed to find task: {e}"))?
        .ok_or("Task not found")?;

    // Fix 4: Guard against duplicate TaskAttempts — skip if one already exists
    let existing_attempts = TaskAttempt::find_by_task_id_with_project(pool, task_id)
        .await
        .unwrap_or_default();
    if !existing_attempts.is_empty() {
        tracing::info!(
            "Task {} already has {} attempt(s) — skipping auto-execute",
            task_id,
            existing_attempts.len()
        );
        return Ok(());
    }

    let agent_id = task
        .agent_id
        .as_ref()
        .ok_or("No agent_id on task")?;

    // Look up execution config for the agent
    let config = AgentExecutionConfig::find_by_agent_id(pool, agent_id)
        .await
        .map_err(|e| format!("Failed to find agent execution config: {e}"))?;

    // Auto-register agent watchers (e.g., QA agent watches tasks assigned to Dev agent)
    if let Some(ref cfg) = config {
        for watcher_id in cfg.get_auto_watch_agent_ids() {
            if let Err(e) =
                Task::add_agent_watcher(pool, &task_id.to_string(), &watcher_id).await
            {
                tracing::warn!(
                    "Failed to add agent watcher {} to task {}: {e}",
                    watcher_id,
                    task_id
                );
            } else {
                tracing::info!(
                    "Auto-registered agent watcher {} on task {}",
                    watcher_id,
                    task_id
                );
            }
        }
    }

    // Parse executor profile from config, default to CLAUDE_CODE
    let profile_str = config.and_then(|c| c.execution_profile_id);
    let executor_profile_id = if let Some(ref profile_str) = profile_str {
        // Format: "CLAUDE_CODE" or "CLAUDE_CODE:PLAN"
        let parts: Vec<&str> = profile_str.splitn(2, ':').collect();
        let executor = BaseCodingAgent::from_str(parts[0])
            .map_err(|_| format!("Unknown executor: {}", parts[0]))?;
        let variant = parts.get(1).map(|s| s.to_string());
        ExecutorProfileId { executor, variant }
    } else {
        ExecutorProfileId::new(BaseCodingAgent::ClaudeCode)
    };

    // Resolve base branch from git repo (default to "main")
    let base_branch = {
        use db::models::project::Project;
        let project = Project::find_by_id(pool, &task.project_id)
            .await
            .ok()
            .flatten();
        if let Some(ref proj) = project {
            // Try to read default branch from git config
            let repo_path = &proj.git_repo_path;
            std::process::Command::new("git")
                .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
                .current_dir(repo_path)
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok().map(|s| {
                            s.trim()
                                .strip_prefix("origin/")
                                .unwrap_or(s.trim())
                                .to_string()
                        })
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "main".to_string())
        } else {
            "main".to_string()
        }
    };

    // Create TaskAttempt
    let task_attempt = TaskAttempt::create(
        pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor.clone(),
            base_branch,
        },
        task_id,
    )
    .await
    .map_err(|e| format!("Failed to create task attempt: {e}"))?;

    // Start execution
    let _process = deployment
        .container()
        .start_attempt(&task_attempt, executor_profile_id)
        .await
        .map_err(|e| format!("Failed to start execution: {e}"))?;

    tracing::info!(
        "Auto-started execution for workflow task {} (attempt {})",
        task_id,
        task_attempt.id
    );

    Ok(())
}

// ── Helper functions ──────────────────────────────────────────────────────

/// Auto-link a contact to a company by name (find or create).
/// Returns the company ID if a company was found or created, so callers can use it.
/// Also stores the company_id in the contact's custom_fields for traceability
/// (CrmContact has no company_id FK, so we use custom_fields to persist the link).
async fn auto_link_company(
    pool: &SqlitePool,
    contact_id: Uuid,
    company_name: &Option<String>,
    data: &Value,
    organization_id: Option<Uuid>,
) -> Option<Uuid> {
    let name = company_name.as_ref()?;
    if name.trim().is_empty() {
        return None;
    }

    // Prefer company_website over the contact's own website for the company record
    let company_website = data["company_website"]
        .as_str()
        .or_else(|| data["website"].as_str())
        .map(|s| s.to_string());

    match Company::find_or_create(pool, name, organization_id, company_website).await {
        Ok(company) => {
            tracing::info!(
                contact_id = %contact_id,
                company_id = %company.id,
                company_name = %name,
                "Auto-linked contact to company"
            );

            // Persist the company_id link in the contact's custom_fields
            if let Err(e) = store_company_id_in_custom_fields(pool, contact_id, company.id).await {
                tracing::warn!(
                    contact_id = %contact_id,
                    company_id = %company.id,
                    error = %e,
                    "Failed to store company_id in contact custom_fields"
                );
            }

            Some(company.id)
        }
        Err(e) => {
            tracing::warn!(
                contact_id = %contact_id,
                company_name = %name,
                error = %e,
                "Failed to auto-link contact to company"
            );
            None
        }
    }
}

/// Persist a company_id inside the contact's custom_fields JSON.
/// Merges with any existing custom_fields rather than overwriting them.
pub async fn store_company_id_in_custom_fields(
    pool: &SqlitePool,
    contact_id: Uuid,
    company_id: Uuid,
) -> Result<(), String> {
    let contact = CrmContact::find_by_id(pool, contact_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut fields: serde_json::Map<String, Value> = contact
        .custom_fields
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    fields.insert("company_id".to_string(), serde_json::json!(company_id.to_string()));

    let update = UpdateCrmContact {
        custom_fields: Some(Value::Object(fields)),
        ..Default::default()
    };

    CrmContact::update(pool, contact_id, update)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Build custom_fields JSON with source traceability info, merging with any existing custom_fields.
/// Also persists extended schema fields (deal_type, next_steps, estimated_value) that don't
/// have dedicated DB columns.

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

    // Persist extended deal schema fields that have no dedicated DB column
    if let Some(deal_type) = data["deal_type"].as_str().or_else(|| data["type"].as_str()) {
        fields.insert("deal_type".to_string(), serde_json::json!(deal_type));
    }
    if let Some(next_steps) = data["next_steps"].as_array() {
        fields.insert("next_steps".to_string(), serde_json::json!(next_steps));
    }
    if let Some(est_val) = data["estimated_value"].as_str() {
        fields.insert("estimated_value".to_string(), serde_json::json!(est_val));
    }

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

use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use db::models::{
    data_source::DataSource,
    trigger_execution::{CreateTriggerExecution, TriggerExecution},
    workflow_trigger::{CreateWorkflowTrigger, UpdateWorkflowTrigger, WorkflowTrigger},
};
use deployment::Deployment;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tracing::{error, info, warn};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

#[derive(Debug, Deserialize)]
struct ListTriggersQuery {
    workflow_id: Option<String>,
}

/// GET /api/workflows/triggers
async fn list_triggers(
    Query(params): Query<ListTriggersQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowTrigger>>>, ApiError> {
    let pool = &deployment.db().pool;
    let triggers = if let Some(ref wid) = params.workflow_id {
        WorkflowTrigger::find_by_workflow(pool, wid).await
    } else {
        WorkflowTrigger::find_all(pool).await
    }
    .map_err(|e| ApiError::InternalError(format!("Failed to list triggers: {e}")))?;
    Ok(Json(ApiResponse::success(triggers)))
}

/// POST /api/workflows/triggers
async fn create_trigger(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateWorkflowTrigger>,
) -> Result<Json<ApiResponse<WorkflowTrigger>>, ApiError> {
    let pool = &deployment.db().pool;

    if body.workflow_id.is_empty() {
        return Err(ApiError::BadRequest("workflow_id is required".to_string()));
    }
    if body.name.is_empty() {
        return Err(ApiError::BadRequest("name is required".to_string()));
    }

    let trigger = WorkflowTrigger::create(pool, body)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create trigger: {e}")))?;
    Ok(Json(ApiResponse::success(trigger)))
}

/// GET /api/workflows/triggers/:id
async fn get_trigger(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<WorkflowTrigger>>, ApiError> {
    let pool = &deployment.db().pool;
    let trigger = WorkflowTrigger::find_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get trigger: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Trigger not found".to_string()))?;
    Ok(Json(ApiResponse::success(trigger)))
}

/// PUT /api/workflows/triggers/:id
async fn update_trigger(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<UpdateWorkflowTrigger>,
) -> Result<Json<ApiResponse<WorkflowTrigger>>, ApiError> {
    let pool = &deployment.db().pool;
    let trigger = WorkflowTrigger::update(pool, &id, body)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update trigger: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Trigger not found".to_string()))?;
    Ok(Json(ApiResponse::success(trigger)))
}

/// DELETE /api/workflows/triggers/:id
async fn delete_trigger(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    WorkflowTrigger::delete(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete trigger: {e}")))?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/workflows/triggers/:id/toggle
async fn toggle_trigger(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<WorkflowTrigger>>, ApiError> {
    let pool = &deployment.db().pool;
    let trigger = WorkflowTrigger::toggle(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to toggle trigger: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Trigger not found".to_string()))?;
    Ok(Json(ApiResponse::success(trigger)))
}

#[derive(Debug, Deserialize)]
struct CheckTriggersRequest {
    data_source_id: String,
}

/// POST /api/workflows/triggers/check — given a data_source_id, return which triggers would fire
async fn check_triggers(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CheckTriggersRequest>,
) -> Result<Json<ApiResponse<Vec<WorkflowTrigger>>>, ApiError> {
    let pool = &deployment.db().pool;
    let ds_id = Uuid::parse_str(&body.data_source_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid data_source_id: {e}")))?;

    let ds = DataSource::find_by_id(pool, &ds_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to find data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    let org_id = ds.organization_id.as_deref();
    let proj_id = ds.project_id.as_deref();

    let triggers = WorkflowTrigger::find_matching_triggers(pool, &ds.data_type, org_id, proj_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check triggers: {e}")))?;

    Ok(Json(ApiResponse::success(triggers)))
}

/// POST /api/webhooks/triggers/:trigger_id — Webhook endpoint for external systems
/// Validates HMAC-SHA256 signature, checks cooldown, fires the associated workflow.
async fn webhook_trigger_handler(
    Path(trigger_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;

    let trigger = WorkflowTrigger::find_by_id(pool, &trigger_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to find trigger: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Trigger not found".to_string()))?;

    // Verify it's a webhook trigger
    if trigger.trigger_type != "webhook" {
        return Err(ApiError::BadRequest(
            "This trigger is not a webhook trigger".to_string(),
        ));
    }

    // Verify it's enabled
    if !trigger.enabled {
        return Err(ApiError::BadRequest("Trigger is disabled".to_string()));
    }

    // Atomic cooldown check — prevents race condition where concurrent webhook
    // requests both pass the cooldown check and double-fire
    let claimed = WorkflowTrigger::try_claim_trigger(&pool, &trigger_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Cooldown check failed: {}", e)))?;
    if claimed.is_none() {
        return Err(ApiError::TooManyRequests(
            "Trigger is in cooldown period".to_string(),
        ));
    }

    // Validate HMAC-SHA256 signature
    let secret = trigger.webhook_secret.as_deref().unwrap_or_default();
    if secret.is_empty() {
        warn!(
            "Webhook trigger {} has no secret configured, rejecting request",
            trigger_id
        );
        return Err(ApiError::Unauthorized(
            "Webhook trigger has no secret configured".to_string(),
        ));
    }
    {
        let sig_header = headers
            .get("x-webhook-signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                warn!(
                    "Webhook trigger {} missing X-Webhook-Signature header",
                    trigger_id
                );
                ApiError::Unauthorized("Missing X-Webhook-Signature header".to_string())
            })?;

        let expected_hex = sig_header.strip_prefix("sha256=").unwrap_or(sig_header);
        let expected_bytes = hex::decode(expected_hex)
            .map_err(|_| ApiError::Unauthorized("Invalid signature format".to_string()))?;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| ApiError::InternalError(format!("HMAC error: {e}")))?;
        mac.update(&body);

        mac.verify_slice(&expected_bytes).map_err(|_| {
            warn!(
                "Webhook trigger {} signature verification failed",
                trigger_id
            );
            ApiError::Unauthorized("Signature verification failed".to_string())
        })?;
    }

    // Parse body as content for the workflow
    let content = String::from_utf8_lossy(&body).to_string();

    // Create audit trail entry
    let execution = TriggerExecution::create(
        pool,
        CreateTriggerExecution {
            trigger_id: trigger_id.clone(),
            source_type: Some("webhook".to_string()),
            source_id: None,
            metadata: Some(
                serde_json::json!({
                    "content_length": body.len(),
                    "content_type": headers.get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown"),
                })
                .to_string(),
            ),
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create execution record: {e}")))?;

    info!(
        "[WEBHOOK] Firing trigger '{}' (workflow={}), execution={}",
        trigger_id, trigger.workflow_id, execution.id
    );

    // Increment trigger count
    if let Err(e) = WorkflowTrigger::increment_trigger_count(pool, &trigger_id).await {
        warn!("[WEBHOOK] Failed to increment trigger count: {e}");
    }

    // Fire the workflow in the background
    let bg_pool = pool.clone();
    let bg_deployment = deployment.clone();
    let exec_id = execution.id.clone();
    let workflow_id = trigger.workflow_id.clone();
    let model_override = trigger.model_override.clone();
    let trigger_auto_approve = trigger.auto_approve;
    let max_retries = trigger.max_retries;
    let bg_trigger_id = trigger_id.clone();
    let trigger_org_id = trigger.filter_organization_id.clone();

    tokio::spawn(async move {
        let start = std::time::Instant::now();

        let result = execute_webhook_trigger(
            &bg_pool,
            &bg_deployment,
            &workflow_id,
            &content,
            model_override,
            trigger_auto_approve,
            &bg_trigger_id,
            &exec_id,
            trigger_org_id.clone(),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok(records_staged) => {
                let _ = TriggerExecution::mark_completed(
                    &bg_pool,
                    &exec_id,
                    records_staged,
                    duration_ms,
                )
                .await;
                let _ = WorkflowTrigger::clear_error(&bg_pool, &bg_trigger_id).await;
                info!(
                    "[WEBHOOK] Completed trigger '{}' execution {} ({} records, {}ms)",
                    bg_trigger_id, exec_id, records_staged, duration_ms
                );
            }
            Err(e) => {
                let err_msg = format!("{e}");
                let _ =
                    TriggerExecution::mark_failed(&bg_pool, &exec_id, &err_msg, duration_ms).await;
                let _ = WorkflowTrigger::record_error(&bg_pool, &bg_trigger_id, &err_msg).await;
                error!(
                    "[WEBHOOK] Failed trigger '{}' execution {}: {} (retry_budget={})",
                    bg_trigger_id, exec_id, err_msg, max_retries
                );
            }
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "execution_id": execution.id,
        "trigger_id": trigger_id,
        "status": "accepted",
    }))))
}

/// Execute a webhook-triggered workflow run.
async fn execute_webhook_trigger(
    pool: &sqlx::SqlitePool,
    deployment: &DeploymentImpl,
    workflow_id: &str,
    content: &str,
    model_override: Option<String>,
    auto_approve: bool,
    trigger_id: &str,
    execution_id: &str,
    organization_id: Option<String>,
) -> Result<i64, String> {
    use db::models::workflow_run::{CreateWorkflowRun, WorkflowRun};

    use super::data_source_workflows::load_workflow;

    let workflow = load_workflow(pool, workflow_id)
        .await
        .map_err(|e| format!("Failed to load workflow: {e}"))?
        .ok_or_else(|| format!("Workflow '{}' not found", workflow_id))?;

    let model = model_override
        .or(workflow.default_model.clone())
        .unwrap_or_default();

    let workflow_run_id = Uuid::new_v4();

    // Create workflow run record
    WorkflowRun::create(
        pool,
        CreateWorkflowRun {
            id: workflow_run_id,
            workflow_id: workflow.id.clone(),
            workflow_name: workflow.name.clone(),
            data_source_id: None,
            organization_id: organization_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok()),
            project_id: None,
            model_used: if model.is_empty() {
                None
            } else {
                Some(model.clone())
            },
            content_hash: None,
        },
    )
    .await
    .map_err(|e| format!("Failed to create workflow run: {e}"))?;

    // Link execution to workflow run
    let _ = TriggerExecution::mark_running(pool, execution_id, Some(&workflow_run_id.to_string()))
        .await;

    let opts = super::workflow_engine::ExecutionOptions {
        model: model.clone(),
        workflow_run_id: Some(workflow_run_id),
        create_artifacts: true,
        create_staging: true,
        artifact_metadata_extra: Some(serde_json::json!({
            "trigger_id": trigger_id,
            "execution_id": execution_id,
            "source": "webhook",
        })),
        auto_approve,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result =
        super::workflow_engine::execute_workflow_nodes(pool, &workflow, content, &opts).await;

    let duration_ms = start.elapsed().as_millis() as i64;
    super::workflow_engine::finalize_workflow_run(
        pool,
        workflow_run_id,
        &workflow,
        &result,
        duration_ms,
        "completed",
    )
    .await;

    // Auto-approve if configured
    if auto_approve && result.staged_records > 0 {
        let label = format!("webhook trigger '{trigger_id}'");
        super::workflow_engine::auto_approve_staged_records(
            pool,
            workflow_run_id,
            result.staged_records,
            deployment,
            &label,
        )
        .await;
    }

    Ok(result.staged_records)
}

/// GET /api/workflows/triggers/:id/executions — List execution history for a trigger
async fn list_trigger_executions(
    Extension(access): Extension<AccessContext>,
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<TriggerExecution>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Authorization: load trigger and verify caller has access
    let trigger = WorkflowTrigger::find_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to find trigger: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Trigger not found".to_string()))?;

    // Non-admin users can only view executions for triggers scoped to their organization
    if !access.is_admin {
        if let Some(ref trigger_org_id) = trigger.filter_organization_id {
            // Look up user's home org to compare
            let user_org: Option<(Option<String>,)> =
                sqlx::query_as("SELECT home_organization_id FROM users WHERE id = ?1")
                    .bind(access.user_id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!("Failed to look up user org: {e}"))
                    })?;

            let user_home_org = user_org.and_then(|row| row.0);
            if user_home_org.as_deref() != Some(trigger_org_id.as_str()) {
                return Err(ApiError::Forbidden(
                    "You do not have access to this trigger's executions".to_string(),
                ));
            }
        }
        // If trigger has no filter_organization_id, only admins should see it
        else {
            return Err(ApiError::Forbidden(
                "Admin access required to view unscoped trigger executions".to_string(),
            ));
        }
    }

    let executions = TriggerExecution::find_by_trigger(pool, &id, 50)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list executions: {e}")))?;
    Ok(Json(ApiResponse::success(executions)))
}

/// Authenticated routes — CRUD operations on triggers (behind require_auth)
pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/workflows/triggers",
            get(list_triggers).post(create_trigger),
        )
        .route("/workflows/triggers/check", post(check_triggers))
        .route(
            "/workflows/triggers/{id}",
            get(get_trigger).put(update_trigger).delete(delete_trigger),
        )
        .route("/workflows/triggers/{id}/toggle", post(toggle_trigger))
        .route(
            "/workflows/triggers/{id}/executions",
            get(list_trigger_executions),
        )
}

/// Public routes — webhook endpoint uses HMAC auth, not session auth
pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route(
        "/webhooks/triggers/{trigger_id}",
        post(webhook_trigger_handler),
    )
}

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use db::models::data_source::DataSource;
use db::models::workflow_trigger::{CreateWorkflowTrigger, UpdateWorkflowTrigger, WorkflowTrigger};
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use deployment::Deployment;

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

    let triggers = WorkflowTrigger::find_matching_triggers(
        pool,
        &ds.data_type,
        org_id,
        proj_id,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to check triggers: {e}")))?;

    Ok(Json(ApiResponse::success(triggers)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/workflows/triggers", get(list_triggers).post(create_trigger))
        .route("/workflows/triggers/check", post(check_triggers))
        .route("/workflows/triggers/{id}", get(get_trigger).put(update_trigger).delete(delete_trigger))
        .route("/workflows/triggers/{id}/toggle", post(toggle_trigger))
}

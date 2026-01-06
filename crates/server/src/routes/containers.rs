use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    execution_slot::{ExecutionSlot, ProjectCapacity},
    task_attempt::TaskAttempt,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::container::ContainerService;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Serialize, TS)]
pub struct ContainerInfo {
    pub attempt_id: Uuid,
    pub task_id: Uuid,
    pub project_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ContainerQuery {
    #[serde(rename = "ref")]
    pub container_ref: String,
}

pub async fn get_container_info(
    Query(query): Query<ContainerQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ContainerInfo>>, ApiError> {
    let pool = &deployment.db().pool;

    let (attempt_id, task_id, project_id) =
        TaskAttempt::resolve_container_ref(pool, &query.container_ref)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => ApiError::Database(e),
                _ => ApiError::Database(e),
            })?;

    let container_info = ContainerInfo {
        attempt_id,
        task_id,
        project_id,
    };

    Ok(ResponseJson(ApiResponse::success(container_info)))
}

/// Get project capacity (max slots and current usage)
pub async fn get_project_capacity(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ProjectCapacity>>, ApiError> {
    let capacity = deployment
        .container()
        .get_project_capacity(project_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(capacity)))
}

/// Get all active execution slots for a project
pub async fn get_active_slots(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionSlot>>>, ApiError> {
    let slots = deployment
        .container()
        .get_active_slots(project_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(slots)))
}

/// Get all running execution processes across all projects
#[derive(Debug, Serialize, TS)]
pub struct ActiveExecutionsResponse {
    pub count: i64,
    pub executions: Vec<db::models::execution_process::ExecutionProcess>,
}

pub async fn get_active_executions(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ActiveExecutionsResponse>>, ApiError> {
    let executions = deployment
        .container()
        .get_running_executions()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let count = executions.len() as i64;

    Ok(ResponseJson(ApiResponse::success(ActiveExecutionsResponse {
        count,
        executions,
    })))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/containers/info", get(get_container_info))
        .route("/projects/{project_id}/capacity", get(get_project_capacity))
        .route("/projects/{project_id}/slots", get(get_active_slots))
        .route("/execution/active", get(get_active_executions))
}

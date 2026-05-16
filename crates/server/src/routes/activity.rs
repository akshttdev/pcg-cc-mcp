use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::activity::{ActivityLog, CreateActivityLog};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct TaskIdPath {
    task_id: String,
}

pub async fn get_activity(
    State(deployment): State<DeploymentImpl>,
    Path(TaskIdPath { task_id }): Path<TaskIdPath>,
) -> Result<ResponseJson<ApiResponse<Vec<ActivityLog>>>, ApiError> {
    let activity = ActivityLog::find_by_task_id(&deployment.db().pool, &task_id).await?;
    Ok(ResponseJson(ApiResponse::success(activity)))
}

pub async fn create_activity(
    State(deployment): State<DeploymentImpl>,
    Path(TaskIdPath { task_id }): Path<TaskIdPath>,
    Json(mut payload): Json<CreateActivityLog>,
) -> Result<ResponseJson<ApiResponse<ActivityLog>>, ApiError> {
    payload.task_id = task_id;

    let activity = ActivityLog::create(&deployment.db().pool, &payload).await?;

    Ok(ResponseJson(ApiResponse::success(activity)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route(
        "/{task_id}/activity",
        get(get_activity).post(create_activity),
    )
}

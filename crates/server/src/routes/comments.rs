use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::comment::{CreateTaskComment, TaskComment};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct TaskIdPath {
    task_id: Uuid,
}

pub async fn get_comments(
    State(deployment): State<DeploymentImpl>,
    Path(TaskIdPath { task_id }): Path<TaskIdPath>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskComment>>>, ApiError> {
    let comments = TaskComment::find_by_task_id(&deployment.db().pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(comments)))
}

pub async fn create_comment(
    State(deployment): State<DeploymentImpl>,
    Path(TaskIdPath { task_id }): Path<TaskIdPath>,
    Json(mut payload): Json<CreateTaskComment>,
) -> Result<ResponseJson<ApiResponse<TaskComment>>, ApiError> {
    // Ensure task_id in path matches task_id in payload
    payload.task_id = task_id;

    let comment = TaskComment::create(&deployment.db().pool, &payload).await?;

    Ok(ResponseJson(ApiResponse::success(comment)))
}

pub async fn delete_comment(
    State(deployment): State<DeploymentImpl>,
    Path(comment_id): Path<Uuid>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    let rows_affected = TaskComment::delete(&deployment.db().pool, comment_id).await?;

    if rows_affected == 0 {
        return Err(ApiError::Database(sqlx::Error::RowNotFound));
    }

    Ok((StatusCode::OK, ResponseJson(ApiResponse::success(()))))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/{task_id}/comments",
            get(get_comments).post(create_comment),
        )
        .route(
            "/comments/{comment_id}",
            axum::routing::delete(delete_comment),
        )
}

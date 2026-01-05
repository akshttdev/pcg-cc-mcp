use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, delete},
};
use db::models::{
    task_artifact::{ArtifactRole, LinkArtifactToTask, TaskArtifact},
    execution_artifact::ExecutionArtifact,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct LinkArtifactPayload {
    pub artifact_id: Uuid,
    pub artifact_role: Option<ArtifactRole>,
    pub display_order: Option<i32>,
    pub pinned: Option<bool>,
    pub added_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListByRoleQuery {
    pub role: Option<ArtifactRole>,
    pub pinned_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRolePayload {
    pub role: ArtifactRole,
}

#[derive(Debug, Deserialize)]
pub struct ReorderPayload {
    pub new_order: i32,
}

#[derive(Debug, serde::Serialize)]
pub struct TaskArtifactWithDetails {
    #[serde(flatten)]
    pub link: TaskArtifact,
    pub artifact: Option<ExecutionArtifact>,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/tasks/{task_id}/artifacts",
            get(list_task_artifacts).post(link_artifact),
        )
        .route(
            "/tasks/{task_id}/artifacts/{artifact_id}",
            delete(unlink_artifact),
        )
        .route(
            "/tasks/{task_id}/artifacts/{artifact_id}/role",
            post(update_role),
        )
        .route(
            "/tasks/{task_id}/artifacts/{artifact_id}/pin",
            post(toggle_pin),
        )
        .route(
            "/tasks/{task_id}/artifacts/{artifact_id}/reorder",
            post(reorder_artifact),
        )
        .route("/artifacts/{artifact_id}/tasks", get(list_artifact_tasks))
        .with_state(deployment.clone())
}

async fn list_task_artifacts(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<ListByRoleQuery>,
) -> Result<Json<ApiResponse<Vec<TaskArtifactWithDetails>>>, ApiError> {
    let pool = &deployment.db().pool;

    let links = if query.pinned_only.unwrap_or(false) {
        TaskArtifact::find_pinned(pool, task_id).await?
    } else if let Some(role) = query.role {
        TaskArtifact::find_by_role(pool, task_id, role).await?
    } else {
        TaskArtifact::find_by_task(pool, task_id).await?
    };

    // Fetch artifact details for each link
    let mut results = Vec::with_capacity(links.len());
    for link in links {
        let artifact = ExecutionArtifact::find_by_id(pool, link.artifact_id).await?;
        results.push(TaskArtifactWithDetails { link, artifact });
    }

    Ok(Json(ApiResponse::success(results)))
}

async fn link_artifact(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<LinkArtifactPayload>,
) -> Result<(StatusCode, Json<ApiResponse<TaskArtifact>>), ApiError> {
    let link = TaskArtifact::link(
        &deployment.db().pool,
        LinkArtifactToTask {
            task_id,
            artifact_id: payload.artifact_id,
            artifact_role: payload.artifact_role,
            display_order: payload.display_order,
            pinned: payload.pinned,
            added_by: payload.added_by,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(link))))
}

async fn unlink_artifact(
    State(deployment): State<DeploymentImpl>,
    Path((task_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let removed = TaskArtifact::unlink(&deployment.db().pool, task_id, artifact_id).await?;
    Ok(Json(ApiResponse::success(removed)))
}

async fn update_role(
    State(deployment): State<DeploymentImpl>,
    Path((task_id, artifact_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<ApiResponse<TaskArtifact>>, ApiError> {
    let link =
        TaskArtifact::update_role(&deployment.db().pool, task_id, artifact_id, payload.role)
            .await?;
    Ok(Json(ApiResponse::success(link)))
}

async fn toggle_pin(
    State(deployment): State<DeploymentImpl>,
    Path((task_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<TaskArtifact>>, ApiError> {
    let link = TaskArtifact::toggle_pin(&deployment.db().pool, task_id, artifact_id).await?;
    Ok(Json(ApiResponse::success(link)))
}

async fn reorder_artifact(
    State(deployment): State<DeploymentImpl>,
    Path((task_id, artifact_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ReorderPayload>,
) -> Result<Json<ApiResponse<TaskArtifact>>, ApiError> {
    let link =
        TaskArtifact::reorder(&deployment.db().pool, task_id, artifact_id, payload.new_order)
            .await?;
    Ok(Json(ApiResponse::success(link)))
}

async fn list_artifact_tasks(
    State(deployment): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<TaskArtifact>>>, ApiError> {
    let links = TaskArtifact::find_by_artifact(&deployment.db().pool, artifact_id).await?;
    Ok(Json(ApiResponse::success(links)))
}

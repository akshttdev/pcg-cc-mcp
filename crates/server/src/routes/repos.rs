use axum::{
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
    Json, Router,
};
use db::models::repo::{CreateRepo, Repo, RepoWithTargetBranch};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Deserialize)]
pub struct RepoQuery {
    pub project_id: Option<Uuid>,
    pub task_attempt_id: Option<Uuid>,
}

pub async fn get_repos(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<RepoQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<Repo>>>, ApiError> {
    let pool = &deployment.db().pool;
    let repos = match query.project_id {
        Some(project_id) => Repo::find_by_project_id(pool, project_id).await?,
        None => Repo::find_all(pool).await?,
    };
    Ok(ResponseJson(ApiResponse::success(repos)))
}

pub async fn get_attempt_repos(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<RepoQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<RepoWithTargetBranch>>>, ApiError> {
    let attempt_id = query.task_attempt_id.ok_or(ApiError::BadRequest(
        "task_attempt_id is required".to_string(),
    ))?;
    let repos = Repo::find_by_task_attempt_id(&deployment.db().pool, attempt_id).await?;
    Ok(ResponseJson(ApiResponse::success(repos)))
}

pub async fn get_repo(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Repo>>, ApiError> {
    let repo = Repo::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or(ApiError::NotFound("Repo not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(repo)))
}

pub async fn create_repo(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateRepo>,
) -> Result<ResponseJson<ApiResponse<Repo>>, ApiError> {
    let repo = Repo::create(&deployment.db().pool, &data).await?;
    Ok(ResponseJson(ApiResponse::success(repo)))
}

pub async fn delete_repo(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Repo::delete(&deployment.db().pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
pub struct LinkRepoRequest {
    pub project_id: Uuid,
    pub repo_id: Uuid,
    pub is_default: Option<bool>,
}

pub async fn link_repo_to_project(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<LinkRepoRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Repo::link_to_project(
        &deployment.db().pool,
        data.project_id,
        data.repo_id,
        data.is_default.unwrap_or(false),
    )
    .await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
pub struct LinkAttemptRepoRequest {
    pub task_attempt_id: Uuid,
    pub repo_id: Uuid,
    pub target_branch: String,
}

pub async fn link_repo_to_attempt(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<LinkAttemptRepoRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Repo::link_to_attempt(
        &deployment.db().pool,
        data.task_attempt_id,
        data.repo_id,
        &data.target_branch,
    )
    .await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/repos",
        Router::new()
            .route("/", get(get_repos).post(create_repo))
            .route("/attempt-repos", get(get_attempt_repos))
            .route("/link-project", post(link_repo_to_project))
            .route("/link-attempt", post(link_repo_to_attempt))
            .route("/{id}", get(get_repo).delete(delete_repo)),
    )
}

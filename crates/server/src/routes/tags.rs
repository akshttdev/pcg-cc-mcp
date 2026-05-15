use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::tag::{CreateTag, Tag, UpdateTag};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct TagQuery {
    pub project_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    /// If true, returns project-specific + org-wide tags for the given project_id
    pub include_org: Option<bool>,
}

pub async fn get_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TagQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<Tag>>>, ApiError> {
    let pool = &deployment.db().pool;
    let tags = match (query.project_id, query.organization_id, query.include_org) {
        (Some(pid), _, Some(true)) => Tag::find_for_project_context(pool, pid).await?,
        (Some(pid), _, _) => Tag::find_by_project_id(pool, pid).await?,
        (_, Some(oid), _) => Tag::find_by_organization_id(pool, oid).await?,
        _ => Tag::find_all(pool).await?,
    };
    Ok(ResponseJson(ApiResponse::success(tags)))
}

pub async fn get_tag(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Tag>>, ApiError> {
    let tag = Tag::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or(ApiError::NotFound("Tag not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(tag)))
}

pub async fn create_tag(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateTag>,
) -> Result<ResponseJson<ApiResponse<Tag>>, ApiError> {
    let tag = Tag::create(&deployment.db().pool, &data).await?;
    Ok(ResponseJson(ApiResponse::success(tag)))
}

pub async fn update_tag(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateTag>,
) -> Result<ResponseJson<ApiResponse<Tag>>, ApiError> {
    let tag = Tag::update(&deployment.db().pool, id, &data)
        .await?
        .ok_or(ApiError::NotFound("Tag not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(tag)))
}

pub async fn delete_tag(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Tag::delete(&deployment.db().pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/tags",
        Router::new()
            .route("/", get(get_tags).post(create_tag))
            .route("/{id}", get(get_tag).put(update_tag).delete(delete_tag)),
    )
}

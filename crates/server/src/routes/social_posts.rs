//! Social Post Management Routes
//!
//! Handles content CRUD, scheduling, and publishing operations.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete, patch},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::social_post::{CreateSocialPost, SocialPost, UpdateSocialPost};

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    pub project_id: Option<Uuid>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

/// GET /social/posts - List posts with filters
async fn list_posts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListPostsQuery>,
) -> Result<Json<ApiResponse<Vec<SocialPost>>>, ApiError> {
    let pool = &deployment.db().pool;

    let posts = SocialPost::find_scheduled(pool, query.project_id).await?;

    // Apply status filter
    let posts = if let Some(status) = &query.status {
        posts.into_iter().filter(|p| p.status == *status).collect()
    } else {
        posts
    };

    // Apply limit
    let posts = if let Some(limit) = query.limit {
        posts.into_iter().take(limit as usize).collect()
    } else {
        posts
    };

    Ok(Json(ApiResponse::success(posts)))
}

/// GET /social/posts/:id - Get single post
async fn get_post(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(post)))
}

/// POST /social/posts - Create new post
async fn create_post(
    State(deployment): State<DeploymentImpl>,
    Json(create): Json<CreateSocialPost>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::create(pool, create).await?;
    Ok(Json(ApiResponse::success(post)))
}

/// PATCH /social/posts/:id - Update post
async fn update_post(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateSocialPost>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(post)))
}

/// DELETE /social/posts/:id - Delete post
async fn delete_post(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    SocialPost::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /social/posts/due - Get posts due for publishing
async fn get_due_posts(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<SocialPost>>>, ApiError> {
    let pool = &deployment.db().pool;
    let posts = SocialPost::find_due_for_publish(pool).await?;
    Ok(Json(ApiResponse::success(posts)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/posts", get(list_posts))
        .route("/social/posts", post(create_post))
        .route("/social/posts/due", get(get_due_posts))
        .route("/social/posts/{id}", get(get_post))
        .route("/social/posts/{id}", patch(update_post))
        .route("/social/posts/{id}", delete(delete_post))
}

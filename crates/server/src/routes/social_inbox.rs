//! Social Inbox Routes
//!
//! Unified inbox for mentions, comments, and DMs across all platforms.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, patch},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::social_mention::{CreateSocialMention, SocialMention, UpdateSocialMention};

#[derive(Debug, Deserialize)]
pub struct InboxQuery {
    pub project_id: Option<Uuid>,
    pub social_account_id: Option<Uuid>,
    pub unread_only: Option<bool>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InboxStats {
    pub total_unread: i64,
    pub high_priority: i64,
}

/// GET /social/inbox - List mentions/messages
async fn list_inbox(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<InboxQuery>,
) -> Result<Json<ApiResponse<Vec<SocialMention>>>, ApiError> {
    let pool = &deployment.db().pool;

    let mentions = if let Some(account_id) = query.social_account_id {
        SocialMention::find_by_account(pool, account_id, query.limit).await?
    } else if let Some(project_id) = query.project_id {
        if query.unread_only.unwrap_or(false) {
            SocialMention::find_unread(pool, project_id).await?
        } else {
            SocialMention::find_by_project(pool, project_id, query.limit).await?
        }
    } else {
        return Err(ApiError::BadRequest("project_id or social_account_id required".to_string()));
    };

    Ok(Json(ApiResponse::success(mentions)))
}

/// GET /social/inbox/stats/:project_id - Get inbox statistics
async fn get_inbox_stats(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ApiResponse<InboxStats>>, ApiError> {
    let pool = &deployment.db().pool;

    let total_unread = SocialMention::count_unread(pool, project_id).await?;
    let high_priority_mentions = SocialMention::find_high_priority(pool, project_id).await?;

    Ok(Json(ApiResponse::success(InboxStats {
        total_unread,
        high_priority: high_priority_mentions.len() as i64,
    })))
}

/// GET /social/inbox/:id - Get single mention
async fn get_mention(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SocialMention>>, ApiError> {
    let pool = &deployment.db().pool;
    let mention = SocialMention::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(mention)))
}

/// PATCH /social/inbox/:id - Update mention
async fn update_mention(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateSocialMention>,
) -> Result<Json<ApiResponse<SocialMention>>, ApiError> {
    let pool = &deployment.db().pool;
    let mention = SocialMention::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(mention)))
}

/// POST /social/inbox/:id/read - Mark mention as read
async fn mark_read(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    SocialMention::mark_read(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /social/inbox - Create mention (for manual entry or webhook)
async fn create_mention(
    State(deployment): State<DeploymentImpl>,
    Json(create): Json<CreateSocialMention>,
) -> Result<Json<ApiResponse<SocialMention>>, ApiError> {
    let pool = &deployment.db().pool;
    let mention = SocialMention::create(pool, create).await?;
    Ok(Json(ApiResponse::success(mention)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/inbox", get(list_inbox))
        .route("/social/inbox", post(create_mention))
        .route("/social/inbox/stats/{project_id}", get(get_inbox_stats))
        .route("/social/inbox/{id}", get(get_mention))
        .route("/social/inbox/{id}", patch(update_mention))
        .route("/social/inbox/{id}/read", post(mark_read))
}

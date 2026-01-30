//! Email Messages Routes
//!
//! API endpoints for viewing and managing email messages (inbox).

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, patch},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::email_message::{
    EmailMessage, EmailMessageFilter, UpdateEmailMessage, EmailInboxStats
};

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub project_id: Option<Uuid>,
    pub email_account_id: Option<Uuid>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub is_archived: Option<bool>,
    pub needs_response: Option<bool>,
    pub crm_contact_id: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /email/messages - List email messages (inbox)
async fn list_messages(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<ApiResponse<Vec<EmailMessage>>>, ApiError> {
    let pool = &deployment.db().pool;

    let filter = EmailMessageFilter {
        project_id: query.project_id,
        email_account_id: query.email_account_id,
        is_read: query.is_read,
        is_starred: query.is_starred,
        is_archived: query.is_archived,
        is_spam: Some(false),
        is_trash: Some(false),
        needs_response: query.needs_response,
        crm_contact_id: query.crm_contact_id,
        search: query.search,
        limit: query.limit,
        offset: query.offset,
    };

    let messages = EmailMessage::find_by_filter(pool, filter).await?;
    Ok(Json(ApiResponse::success(messages)))
}

/// GET /email/messages/stats/:project_id - Get inbox statistics
async fn get_inbox_stats(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ApiResponse<EmailInboxStats>>, ApiError> {
    let pool = &deployment.db().pool;
    let stats = EmailMessage::get_inbox_stats(pool, project_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /email/messages/:id - Get single message
async fn get_message(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<EmailMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let message = EmailMessage::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(message)))
}

/// GET /email/messages/thread/:thread_id - Get messages in a thread
async fn get_thread(
    State(deployment): State<DeploymentImpl>,
    Path(thread_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<EmailMessage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let messages = EmailMessage::find_by_thread(pool, &thread_id).await?;
    Ok(Json(ApiResponse::success(messages)))
}

/// PATCH /email/messages/:id - Update message (read, starred, etc.)
async fn update_message(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateEmailMessage>,
) -> Result<Json<ApiResponse<EmailMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let message = EmailMessage::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(message)))
}

/// POST /email/messages/:id/read - Mark message as read
async fn mark_as_read(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    EmailMessage::mark_as_read(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /email/messages/:id/unread - Mark message as unread
async fn mark_as_unread(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    EmailMessage::mark_as_unread(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /email/messages/:id/star - Toggle star
async fn toggle_star(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<EmailMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let message = EmailMessage::toggle_star(pool, id).await?;
    Ok(Json(ApiResponse::success(message)))
}

/// POST /email/messages/:id/trash - Move to trash
async fn move_to_trash(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    EmailMessage::move_to_trash(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/email/messages", get(list_messages))
        .route("/email/messages/stats/{project_id}", get(get_inbox_stats))
        .route("/email/messages/{id}", get(get_message))
        .route("/email/messages/{id}", patch(update_message))
        .route("/email/messages/thread/{thread_id}", get(get_thread))
        .route("/email/messages/{id}/read", post(mark_as_read))
        .route("/email/messages/{id}/unread", post(mark_as_unread))
        .route("/email/messages/{id}/star", post(toggle_star))
        .route("/email/messages/{id}/trash", post(move_to_trash))
}

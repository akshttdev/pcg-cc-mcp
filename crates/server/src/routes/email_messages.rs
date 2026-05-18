//! Email Messages Routes
//!
//! API endpoints for viewing and managing email messages (inbox).

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Json, Router,
};
use db::models::{
    email_account::EmailAccount,
    email_message::{EmailInboxStats, EmailMessage, EmailMessageFilter, UpdateEmailMessage},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, helpers::uuid_params::parse_db_uuid_param, DeploymentImpl};

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
    Path(project_id): Path<String>,
) -> Result<Json<ApiResponse<EmailInboxStats>>, ApiError> {
    let project_id = parse_db_uuid_param(&project_id, "project ID")?.to_uuid();
    let pool = &deployment.db().pool;
    let stats = EmailMessage::get_inbox_stats(pool, project_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /email/messages/:id - Get single message
async fn get_message(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmailMessage>>, ApiError> {
    let id = parse_db_uuid_param(&id, "email message ID")?.to_uuid();
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
    Path(id): Path<String>,
    Json(update): Json<UpdateEmailMessage>,
) -> Result<Json<ApiResponse<EmailMessage>>, ApiError> {
    let id = parse_db_uuid_param(&id, "email message ID")?.to_uuid();
    let pool = &deployment.db().pool;
    let message = EmailMessage::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(message)))
}

/// POST /email/messages/:id/read - Mark message as read
async fn mark_as_read(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id = parse_db_uuid_param(&id, "email message ID")?.to_uuid();
    let pool = &deployment.db().pool;
    EmailMessage::mark_as_read(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /email/messages/:id/unread - Mark message as unread
async fn mark_as_unread(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id = parse_db_uuid_param(&id, "email message ID")?.to_uuid();
    let pool = &deployment.db().pool;
    EmailMessage::mark_as_unread(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /email/messages/:id/star - Toggle star
async fn toggle_star(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmailMessage>>, ApiError> {
    let id = parse_db_uuid_param(&id, "email message ID")?.to_uuid();
    let pool = &deployment.db().pool;
    let message = EmailMessage::toggle_star(pool, id).await?;
    Ok(Json(ApiResponse::success(message)))
}

#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    /// id of the email_account to send from.
    pub from_account_id: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct SendEmailResponse {
    pub provider_message_id: String,
}

/// POST /email/send — send an email from a connected account.
/// Dispatches to Gmail or Zoho based on the account's provider.
async fn send_email(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<SendEmailRequest>,
) -> Result<Json<ApiResponse<SendEmailResponse>>, ApiError> {
    use services::services::agent_channels::{AgentChannelService, ChannelOwner};

    if req.to.is_empty() {
        return Err(ApiError::BadRequest(
            "`to` must contain at least one address".into(),
        ));
    }
    if req.subject.trim().is_empty() {
        return Err(ApiError::BadRequest("`subject` is required".into()));
    }

    let pool = &deployment.db().pool;
    let account_id = parse_db_uuid_param(&req.from_account_id, "from_account_id")?.to_uuid();
    let account = EmailAccount::find_by_id(pool, account_id).await?;

    let owner = parse_owner(&account.owner_type, &account.owner_id).ok_or_else(|| {
        ApiError::InternalError(format!(
            "email account {} has unparseable owner ({}/{})",
            account.id, account.owner_type, account.owner_id
        ))
    })?;

    let svc = AgentChannelService::new(pool.clone());
    let provider_message_id = svc
        .send_email(&owner, &req.to, &req.subject, &req.body)
        .await
        .map_err(|e| ApiError::InternalError(format!("send failed: {e}")))?;

    Ok(Json(ApiResponse::success(SendEmailResponse {
        provider_message_id,
    })))
}

fn parse_owner(
    owner_type: &str,
    owner_id: &str,
) -> Option<services::services::agent_channels::ChannelOwner> {
    use services::services::agent_channels::ChannelOwner;
    let id = Uuid::parse_str(owner_id).ok()?;
    Some(match owner_type {
        "agent" => ChannelOwner::Agent(id),
        "user" => ChannelOwner::User(id),
        "organization" => ChannelOwner::Organization(id),
        "project" => ChannelOwner::Project(id),
        _ => return None,
    })
}

/// POST /email/messages/:id/trash - Move to trash
async fn move_to_trash(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id = parse_db_uuid_param(&id, "email message ID")?.to_uuid();
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
        .route("/email/send", post(send_email))
}

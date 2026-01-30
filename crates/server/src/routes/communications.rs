//! Communications Routes
//!
//! API endpoints for phone calls and SMS messages (Twilio integration).

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
use db::models::call_log::{CallLog, CallStats, UpdateCallLog};
use db::models::sms_message::{SmsMessage, SmsStats, UpdateSmsMessage};

// ============================================================================
// Call Logs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListCallsQuery {
    pub project_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /communications/calls - List call logs
async fn list_calls(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListCallsQuery>,
) -> Result<Json<ApiResponse<Vec<CallLog>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let calls = if let Some(project_id) = query.project_id {
        CallLog::find_by_project(pool, project_id, limit, offset).await?
    } else {
        vec![]
    };

    Ok(Json(ApiResponse::success(calls)))
}

/// GET /communications/calls/stats/:project_id - Get call statistics
async fn get_call_stats(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CallStats>>, ApiError> {
    let pool = &deployment.db().pool;
    let stats = CallLog::get_stats(pool, project_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /communications/calls/:id - Get single call
async fn get_call(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CallLog>>, ApiError> {
    let pool = &deployment.db().pool;
    let call = CallLog::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(call)))
}

/// PATCH /communications/calls/:id - Update call
async fn update_call(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateCallLog>,
) -> Result<Json<ApiResponse<CallLog>>, ApiError> {
    let pool = &deployment.db().pool;
    let call = CallLog::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(call)))
}

// ============================================================================
// SMS Messages
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListSmsQuery {
    pub project_id: Option<Uuid>,
    pub is_read: Option<bool>,
    pub phone_number: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /communications/sms - List SMS messages
async fn list_sms(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListSmsQuery>,
) -> Result<Json<ApiResponse<Vec<SmsMessage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let messages = if let Some(project_id) = query.project_id {
        if let Some(phone) = &query.phone_number {
            SmsMessage::find_conversation(pool, project_id, phone, limit).await?
        } else if query.is_read == Some(false) {
            SmsMessage::find_unread_by_project(pool, project_id, limit).await?
        } else {
            SmsMessage::find_by_project(pool, project_id, limit, offset).await?
        }
    } else {
        vec![]
    };

    Ok(Json(ApiResponse::success(messages)))
}

/// GET /communications/sms/stats/:project_id - Get SMS statistics
async fn get_sms_stats(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SmsStats>>, ApiError> {
    let pool = &deployment.db().pool;
    let stats = SmsMessage::get_stats(pool, project_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /communications/sms/:id - Get single SMS
async fn get_sms(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SmsMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let msg = SmsMessage::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(msg)))
}

/// PATCH /communications/sms/:id - Update SMS
async fn update_sms(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateSmsMessage>,
) -> Result<Json<ApiResponse<SmsMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let msg = SmsMessage::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(msg)))
}

/// POST /communications/sms/:id/read - Mark SMS as read
async fn mark_sms_read(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    SmsMessage::mark_as_read(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /communications/sms/:id/star - Toggle star
async fn toggle_sms_star(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SmsMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let msg = SmsMessage::toggle_star(pool, id).await?;
    Ok(Json(ApiResponse::success(msg)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Call routes
        .route("/communications/calls", get(list_calls))
        .route("/communications/calls/stats/{project_id}", get(get_call_stats))
        .route("/communications/calls/{id}", get(get_call))
        .route("/communications/calls/{id}", patch(update_call))
        // SMS routes
        .route("/communications/sms", get(list_sms))
        .route("/communications/sms/stats/{project_id}", get(get_sms_stats))
        .route("/communications/sms/{id}", get(get_sms))
        .route("/communications/sms/{id}", patch(update_sms))
        .route("/communications/sms/{id}/read", post(mark_sms_read))
        .route("/communications/sms/{id}/star", post(toggle_sms_star))
}

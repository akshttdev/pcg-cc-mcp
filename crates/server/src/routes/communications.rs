//! Communications Routes
//!
//! API endpoints for phone calls and SMS messages (Twilio integration).

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Extension, Json, Router,
};
use db::models::{
    call_log::{CallLog, CallStats, UpdateCallLog},
    sms_message::{SmsMessage, SmsStats, UpdateSmsMessage},
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

// ============================================================================
// Call Logs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListCallsQuery {
    pub project_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /communications/calls - List call logs
async fn list_calls(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListCallsQuery>,
) -> Result<Json<ApiResponse<Vec<CallLog>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let calls = if let Some(deal_id) = query.crm_deal_id {
        // Deal-scoped query may span projects; filter to accessible ones
        let all_calls = CallLog::find_by_deal(pool, deal_id, limit).await?;
        let mut accessible = Vec::new();
        for call in all_calls {
            if access_context
                .get_project_role(pool, &call.project_id.to_string())
                .await?
                .is_some()
            {
                accessible.push(call);
            }
        }
        accessible
    } else if let Some(project_id) = query.project_id {
        access_context
            .require_viewer(pool, &project_id.to_string())
            .await?;
        CallLog::find_by_project(pool, project_id, limit, offset).await?
    } else {
        vec![]
    };

    Ok(Json(ApiResponse::success(calls)))
}

/// GET /communications/calls/stats/:project_id - Get call statistics
async fn get_call_stats(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<String>,
) -> Result<Json<ApiResponse<CallStats>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context.require_viewer(pool, &project_id).await?;
    let project_id = Uuid::parse_str(&project_id)
        .map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", project_id)))?;
    let stats = CallLog::get_stats(pool, project_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /communications/calls/:id - Get single call
async fn get_call(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CallLog>>, ApiError> {
    let pool = &deployment.db().pool;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?;
    let call = CallLog::find_by_id(pool, id).await?;
    access_context
        .require_viewer(pool, &call.project_id.to_string())
        .await?;
    Ok(Json(ApiResponse::success(call)))
}

/// PATCH /communications/calls/:id - Update call
async fn update_call(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(update): Json<UpdateCallLog>,
) -> Result<Json<ApiResponse<CallLog>>, ApiError> {
    let pool = &deployment.db().pool;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?;
    // Load existing call to get project_id, then verify editor access
    let existing = CallLog::find_by_id(pool, id).await?;
    access_context
        .require_editor(pool, &existing.project_id.to_string())
        .await?;
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
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListSmsQuery>,
) -> Result<Json<ApiResponse<Vec<SmsMessage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let messages = if let Some(project_id) = query.project_id {
        access_context
            .require_viewer(pool, &project_id.to_string())
            .await?;
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
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<String>,
) -> Result<Json<ApiResponse<SmsStats>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context.require_viewer(pool, &project_id).await?;
    let project_id = Uuid::parse_str(&project_id)
        .map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", project_id)))?;
    let stats = SmsMessage::get_stats(pool, project_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /communications/sms/:id - Get single SMS
async fn get_sms(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SmsMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?;
    let msg = SmsMessage::find_by_id(pool, id).await?;
    access_context
        .require_viewer(pool, &msg.project_id.to_string())
        .await?;
    Ok(Json(ApiResponse::success(msg)))
}

/// PATCH /communications/sms/:id - Update SMS
async fn update_sms(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(update): Json<UpdateSmsMessage>,
) -> Result<Json<ApiResponse<SmsMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?;
    // Load existing SMS to get project_id, then verify editor access
    let existing = SmsMessage::find_by_id(pool, id).await?;
    access_context
        .require_editor(pool, &existing.project_id.to_string())
        .await?;
    let msg = SmsMessage::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(msg)))
}

/// POST /communications/sms/:id/read - Mark SMS as read
async fn mark_sms_read(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?;
    let existing = SmsMessage::find_by_id(pool, id).await?;
    access_context
        .require_editor(pool, &existing.project_id.to_string())
        .await?;
    SmsMessage::mark_as_read(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /communications/sms/:id/star - Toggle star
async fn toggle_sms_star(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SmsMessage>>, ApiError> {
    let pool = &deployment.db().pool;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?;
    let existing = SmsMessage::find_by_id(pool, id).await?;
    access_context
        .require_editor(pool, &existing.project_id.to_string())
        .await?;
    let msg = SmsMessage::toggle_star(pool, id).await?;
    Ok(Json(ApiResponse::success(msg)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Call routes
        .route("/communications/calls", get(list_calls))
        .route(
            "/communications/calls/stats/{project_id}",
            get(get_call_stats),
        )
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

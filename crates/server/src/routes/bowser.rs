//! Bowser Browser Agent API Routes
//!
//! REST endpoints for browser automation, screenshots, and URL allowlisting.

use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post},
};
use deployment::Deployment;
use db::models::{
    browser_action::BrowserAction,
    browser_allowlist::{BrowserAllowlist, PatternType},
    browser_screenshot::BrowserScreenshot,
    browser_session::{BrowserSession, BrowserType},
};
use serde::{Deserialize, Serialize};
use services::services::bowser::{BowserService, BrowserSessionDetails, BowserSummary};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ========== Request/Response Types ==========

#[derive(Debug, Deserialize, TS)]
pub struct StartSessionRequest {
    pub execution_process_id: Uuid,
    pub browser_type: Option<BrowserType>,
    pub viewport_width: Option<i32>,
    pub viewport_height: Option<i32>,
    pub headless: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
pub struct NavigateRequest {
    pub url: String,
    pub project_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
pub struct AddAllowlistRequest {
    pub project_id: Option<Uuid>,
    pub pattern: String,
    #[serde(default)]
    pub pattern_type: PatternType,
    pub description: Option<String>,
    #[serde(default)]
    pub is_global: bool,
}

#[derive(Debug, Deserialize, TS)]
pub struct CheckUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize, TS)]
pub struct CheckUrlResponse {
    pub allowed: bool,
    pub url: String,
}

// ========== Session Endpoints ==========

/// Start a new browser session
pub async fn start_session(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<StartSessionRequest>,
) -> Result<ResponseJson<ApiResponse<BrowserSession>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let viewport = match (req.viewport_width, req.viewport_height) {
        (Some(w), Some(h)) => Some((w, h)),
        _ => None,
    };

    let session = bowser
        .start_session(req.execution_process_id, req.browser_type, viewport, req.headless)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(session)))
}

/// Get session by ID
pub async fn get_session(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<BrowserSession>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let session = bowser
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(session)))
}

/// Get session with full details (screenshots, actions)
pub async fn get_session_details(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<BrowserSessionDetails>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let details = bowser
        .get_session_details(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(details)))
}

/// Get all active browser sessions
pub async fn get_active_sessions(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<BrowserSession>>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let sessions = bowser
        .get_active_sessions()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(sessions)))
}

/// Close a browser session
pub async fn close_session(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<BrowserSession>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let session = bowser
        .close_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(session)))
}

// ========== Navigation Endpoints ==========

/// Navigate to a URL (with allowlist check)
pub async fn navigate(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<NavigateRequest>,
) -> Result<ResponseJson<ApiResponse<BrowserAction>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let action = bowser
        .navigate(session_id, req.project_id, &req.url)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(action)))
}

// ========== Screenshot Endpoints ==========

/// Get screenshots for a session
pub async fn get_screenshots(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<BrowserScreenshot>>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let screenshots = bowser
        .get_screenshots(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(screenshots)))
}

/// Get screenshots with visual diffs
pub async fn get_screenshots_with_diffs(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<BrowserScreenshot>>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let screenshots = bowser
        .get_screenshots_with_diffs(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(screenshots)))
}

// ========== Action Endpoints ==========

/// Get actions for a session
pub async fn get_actions(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<BrowserAction>>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let actions = bowser
        .get_actions(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(actions)))
}

/// Get failed actions for a session
pub async fn get_failed_actions(
    Path(session_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<BrowserAction>>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let actions = bowser
        .get_failed_actions(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(actions)))
}

// ========== Allowlist Endpoints ==========

/// Get allowlist for a project
pub async fn get_allowlist(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<BrowserAllowlist>>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let entries = bowser
        .get_allowlist(project_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(entries)))
}

/// Add pattern to allowlist
pub async fn add_to_allowlist(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<AddAllowlistRequest>,
) -> Result<ResponseJson<ApiResponse<BrowserAllowlist>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let entry = bowser
        .add_to_allowlist(
            req.project_id,
            req.pattern,
            req.pattern_type,
            req.description,
            req.is_global,
            None, // TODO: Get from auth context
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(entry)))
}

/// Remove from allowlist
pub async fn remove_from_allowlist(
    Path(entry_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    bowser
        .remove_from_allowlist(entry_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(())))
}

/// Check if URL is allowed
pub async fn check_url(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<CheckUrlRequest>,
) -> Result<ResponseJson<ApiResponse<CheckUrlResponse>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let allowed = bowser
        .is_url_allowed(project_id, &req.url)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(CheckUrlResponse {
        allowed,
        url: req.url,
    })))
}

// ========== Summary Endpoint ==========

/// Get Bowser summary for Mission Control
pub async fn get_summary(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<BowserSummary>>, ApiError> {
    let bowser = BowserService::new(deployment.db().clone());

    let summary = bowser
        .get_summary()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(summary)))
}

// ========== Router ==========

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Session management
        .route("/bowser/sessions", post(start_session))
        .route("/bowser/sessions", get(get_active_sessions))
        .route("/bowser/sessions/{session_id}", get(get_session))
        .route("/bowser/sessions/{session_id}/details", get(get_session_details))
        .route("/bowser/sessions/{session_id}/close", post(close_session))
        // Navigation
        .route("/bowser/sessions/{session_id}/navigate", post(navigate))
        // Screenshots
        .route("/bowser/sessions/{session_id}/screenshots", get(get_screenshots))
        .route("/bowser/sessions/{session_id}/screenshots/diffs", get(get_screenshots_with_diffs))
        // Actions
        .route("/bowser/sessions/{session_id}/actions", get(get_actions))
        .route("/bowser/sessions/{session_id}/actions/failed", get(get_failed_actions))
        // Allowlist
        .route("/bowser/allowlist", post(add_to_allowlist))
        .route("/bowser/allowlist/{entry_id}", delete(remove_from_allowlist))
        .route("/bowser/projects/{project_id}/allowlist", get(get_allowlist))
        .route("/bowser/projects/{project_id}/check-url", post(check_url))
        // Summary
        .route("/bowser/summary", get(get_summary))
}

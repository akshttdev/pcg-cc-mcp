//! APN Data Service REST API endpoints
//!
//! Provides endpoints for triggering data sync via APN network
//! and checking sync status.

use axum::{
    Extension,
    extract::State,
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use deployment::Deployment;
use serde::Serialize;
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::AccessContext,
};

#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub enabled: bool,
    pub connected: bool,
    pub last_sync_at: Option<String>,
    pub sync_version: u64,
    pub projects_synced: usize,
    pub tasks_synced: usize,
    pub device_id: String,
    pub provider_id: String,
    pub role: String,
}

/// GET /api/apn/sync/status
/// Returns the current APN data sync status
pub async fn get_sync_status(
    Extension(_access_context): Extension<AccessContext>,
) -> Result<ResponseJson<ApiResponse<SyncStatus>>, ApiError> {
    let config = crate::apn_data_service::APNDataServiceConfig::from_env()
        .map_err(|e| ApiError::InternalError(format!("Config error: {}", e)))?;

    let status = SyncStatus {
        enabled: config.enabled,
        connected: false, // Would need shared state to know this
        last_sync_at: None,
        sync_version: 0,
        projects_synced: 0,
        tasks_synced: 0,
        device_id: config.device_id,
        provider_id: config.provider_id,
        role: format!("{:?}", config.role),
    };

    Ok(ResponseJson(ApiResponse::success(status)))
}

/// POST /api/apn/sync/trigger
/// Trigger a data sync for the current user via APN network
pub async fn trigger_sync(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TriggerSyncResponse>>, ApiError> {
    let config = crate::apn_data_service::APNDataServiceConfig::from_env()
        .map_err(|e| ApiError::InternalError(format!("Config error: {}", e)))?;

    if !config.enabled {
        return Ok(ResponseJson(ApiResponse::error(
            "APN Data Service is not enabled. Set APN_DATA_SERVICE_ENABLED=true",
        )));
    }

    // Resolve username from user_id
    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(access_context.user_id.to_string())
        .fetch_optional(&deployment.db().pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?
        .unwrap_or_default();

    // Connect to NATS and send sync request
    match async_nats::connect(&config.nats_url).await {
        Ok(client) => {
            match crate::apn_data_service::request_user_sync(
                &client,
                &config.provider_id,
                &config.device_id,
                &access_context.user_id.to_string(),
                &username,
            )
            .await
            {
                Ok(()) => {
                    Ok(ResponseJson(ApiResponse::success(TriggerSyncResponse {
                        message: "Sync request sent to APN provider".to_string(),
                        provider_id: config.provider_id,
                        device_id: config.device_id,
                    })))
                }
                Err(e) => Err(ApiError::InternalError(format!(
                    "Failed to send sync request: {}",
                    e
                ))),
            }
        }
        Err(e) => Err(ApiError::InternalError(format!(
            "Failed to connect to NATS at {}: {}",
            config.nats_url, e
        ))),
    }
}

#[derive(Debug, Serialize)]
pub struct TriggerSyncResponse {
    pub message: String,
    pub provider_id: String,
    pub device_id: String,
}

/// Simple test handler to verify route registration
pub async fn apn_ping() -> ResponseJson<ApiResponse<String>> {
    ResponseJson(ApiResponse::success("pong from APN data service".to_string()))
}

/// Router for APN data endpoints — no auth layer (added by protected_routes group)
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/apn-sync-status", get(get_sync_status))
        .route("/apn-sync-trigger", post(trigger_sync))
}

/// Public routes (no auth required) for basic health-check style endpoints
pub fn public_router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/apn-ping", get(apn_ping))
}

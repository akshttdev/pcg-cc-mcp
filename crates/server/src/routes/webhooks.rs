use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use utils::response::ApiResponse;

use crate::DeploymentImpl;

/// Dropbox webhook verification challenge
#[derive(Debug, Deserialize)]
pub struct DropboxChallenge {
    challenge: String,
}

/// Dropbox webhook notification payload
#[derive(Debug, Deserialize)]
pub struct DropboxWebhookPayload {
    list_folder: Option<DropboxListFolderPayload>,
    delta: Option<DropboxDeltaPayload>,
}

#[derive(Debug, Deserialize)]
pub struct DropboxListFolderPayload {
    accounts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DropboxDeltaPayload {
    users: Vec<String>,
}

/// Response for successful webhook processing
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    success: bool,
    message: String,
    batches_created: Option<usize>,
}

/// GET /api/webhooks/dropbox - Verification endpoint
/// Dropbox sends a challenge parameter that we must echo back
pub async fn dropbox_webhook_verify(
    Query(params): Query<DropboxChallenge>,
) -> impl IntoResponse {
    info!("Dropbox webhook verification challenge received");

    // Dropbox requires us to echo back the challenge
    // Return as plain text, not JSON
    (StatusCode::OK, params.challenge)
}

/// POST /api/webhooks/dropbox - Webhook notification handler
/// Called by Dropbox when files change in monitored folders
pub async fn dropbox_webhook_handler(
    State(_deployment): State<DeploymentImpl>,
    body: String,
) -> Result<Json<ApiResponse<WebhookResponse>>, StatusCode> {
    info!("Dropbox webhook notification received");

    // Parse the webhook payload
    let payload: DropboxWebhookPayload = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse Dropbox webhook payload: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Extract affected accounts/users
    let mut affected_accounts = Vec::new();

    if let Some(list_folder) = payload.list_folder {
        affected_accounts.extend(list_folder.accounts);
    }

    if let Some(delta) = payload.delta {
        affected_accounts.extend(delta.users);
    }

    if affected_accounts.is_empty() {
        warn!("Dropbox webhook received but no accounts affected");
        return Ok(Json(ApiResponse::success(WebhookResponse {
            success: true,
            message: "Webhook received but no accounts affected".to_string(),
            batches_created: Some(0),
        })));
    }

    info!(
        "Dropbox webhook affecting {} account(s): {:?}",
        affected_accounts.len(),
        affected_accounts
    );

    // TODO: Implement actual processing
    // For now, we'll just acknowledge receipt
    // In the future, this should:
    // 1. Fetch the list of changed files from Dropbox API
    // 2. Filter for video files in monitored folders
    // 3. Trigger media ingestion via NORA tools
    // 4. Create project board tasks

    Ok(Json(ApiResponse::success(WebhookResponse {
        success: true,
        message: format!(
            "Webhook processed for {} account(s). Auto-ingestion not yet implemented.",
            affected_accounts.len()
        ),
        batches_created: Some(0),
    })))
}

/// Router for webhook endpoints
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/webhooks/dropbox", get(dropbox_webhook_verify))
        .route("/webhooks/dropbox", post(dropbox_webhook_handler))
}

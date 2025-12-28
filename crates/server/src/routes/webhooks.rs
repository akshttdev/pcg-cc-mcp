use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use deployment::Deployment;
use db::models::dropbox_source::{render_reference_name, DropboxSource};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use services::services::media_pipeline::{MediaBatchIngestRequest, MediaStorageTier};
use sha2::Sha256;
use tracing::{error, info, warn};
use utils::response::ApiResponse;
use uuid::Uuid;

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
    #[serde(default)]
    editron_batches: Vec<DropboxEditronBatch>,
}

#[derive(Debug, Deserialize)]
pub struct DropboxListFolderPayload {
    accounts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DropboxDeltaPayload {
    users: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DropboxEditronBatch {
    pub source_url: String,
    pub reference_name: Option<String>,
    pub project_id: Option<String>,
    pub storage_tier: Option<String>,
    pub checksum_required: Option<bool>,
}

/// Response for successful webhook processing
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    success: bool,
    message: String,
    batches_created: Option<usize>,
    batch_ids: Option<Vec<String>>,
    errors: Option<Vec<String>>,
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
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ApiResponse<WebhookResponse>>, StatusCode> {
    info!("Dropbox webhook notification received");

    verify_dropbox_signature(&headers, &body)?;

    let payload: DropboxWebhookPayload = serde_json::from_slice(&body).map_err(|err| {
        error!("Failed to parse Dropbox webhook payload: {}", err);
        StatusCode::BAD_REQUEST
    })?;

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
            batch_ids: None,
            errors: None,
        })));
    }

    info!(
        "Dropbox webhook affecting {} account(s): {:?}",
        affected_accounts.len(),
        affected_accounts
    );

    let mut created_batches = Vec::new();
    let mut errors = Vec::new();

    let pipeline = deployment.media_pipeline().clone();

    if !payload.editron_batches.is_empty() {
        for hint in &payload.editron_batches {
            let tier_value = hint.storage_tier.as_deref().unwrap_or("hot");
            let storage_tier = match MediaStorageTier::from_str(tier_value) {
                Ok(tier) => tier,
                Err(err) => {
                    warn!(
                        "Ignoring batch '{}' due to invalid storage tier: {}",
                        hint.source_url, err
                    );
                    errors.push(format!(
                        "{}: invalid storage tier '{}': {}",
                        hint.source_url, tier_value, err
                    ));
                    continue;
                }
            };

            let project_uuid = match parse_uuid(&hint.project_id) {
                Ok(value) => value,
                Err(err) => {
                    warn!(
                        "Invalid project_id in Dropbox webhook for {}: {}",
                        hint.source_url, err
                    );
                    errors.push(format!("{}: {}", hint.source_url, err));
                    continue;
                }
            };

            let request = MediaBatchIngestRequest {
                source_url: hint.source_url.clone(),
                reference_name: hint.reference_name.clone(),
                storage_tier,
                checksum_required: hint.checksum_required.unwrap_or(true),
                project_id: project_uuid,
            };

            match pipeline.ingest_batch(request).await {
                Ok(batch) => {
                    info!("Auto-ingested Dropbox batch {}", batch.id);
                    created_batches.push(batch.id.to_string());
                }
                Err(err) => {
                    error!(
                        "Failed to auto-ingest Dropbox batch {}: {}",
                        hint.source_url, err
                    );
                    errors.push(format!(
                        "{}: failed to ingest ({})",
                        hint.source_url, err
                    ));
                }
            }
        }
    }

    if !affected_accounts.is_empty() {
        match DropboxSource::find_by_accounts(&deployment.db().pool, &affected_accounts).await {
            Ok(sources) => {
                for source in sources {
                    if !source.auto_ingest {
                        continue;
                    }

                    if source.ingest_strategy.to_lowercase() != "shared_link" {
                        errors.push(format!(
                            "{}: ingest_strategy '{}' not supported yet",
                            source.label, source.ingest_strategy
                        ));
                        continue;
                    }

                    let Some(url) = source.source_url.clone() else {
                        errors.push(format!("{}: missing source_url", source.label));
                        continue;
                    };

                    let storage_tier = match MediaStorageTier::from_str(&source.storage_tier) {
                        Ok(tier) => tier,
                        Err(err) => {
                            errors.push(format!(
                                "{}: invalid storage tier '{}': {}",
                                source.label, source.storage_tier, err
                            ));
                            continue;
                        }
                    };

                    let request = MediaBatchIngestRequest {
                        source_url: url,
                        reference_name: Some(render_reference_name(
                            source.reference_name_template.as_deref(),
                            &source.label,
                        )),
                        storage_tier,
                        checksum_required: source.checksum_required,
                        project_id: source.project_id,
                    };

                    match pipeline.ingest_batch(request).await {
                        Ok(batch) => {
                            info!(
                                "Auto-ingested Dropbox source '{}' -> {}",
                                source.label, batch.id
                            );
                            created_batches.push(batch.id.to_string());
                            let _ = DropboxSource::mark_processed(
                                &deployment.db().pool,
                                source.id,
                                source.cursor.clone(),
                                Utc::now(),
                            )
                            .await;
                        }
                        Err(err) => {
                            error!(
                                "Failed to auto-ingest Dropbox source '{}': {}",
                                source.label, err
                            );
                            errors.push(format!(
                                "{}: failed to ingest ({})",
                                source.label, err
                            ));
                        }
                    }
                }
            }
            Err(err) => {
                error!("Failed to load Dropbox sources: {}", err);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    let batch_count = created_batches.len();
    let success = errors.is_empty();
    let message = if batch_count > 0 {
        format!(
            "Webhook processed for {} account(s); queued {} ingest batch(es)",
            affected_accounts.len(),
            batch_count
        )
    } else if !payload.editron_batches.is_empty() {
        "Webhook processed but all auto-ingest hints failed".to_string()
    } else {
        format!(
            "Webhook processed for {} account(s); no auto-ingest hints provided",
            affected_accounts.len()
        )
    };

    Ok(Json(ApiResponse::success(WebhookResponse {
        success,
        message,
        batches_created: Some(batch_count),
        batch_ids: (!created_batches.is_empty()).then_some(created_batches),
        errors: (!errors.is_empty()).then_some(errors),
    })))
}

/// Router for webhook endpoints
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/webhooks/dropbox", get(dropbox_webhook_verify))
        .route("/webhooks/dropbox", post(dropbox_webhook_handler))
}

fn parse_uuid(value: &Option<String>) -> Result<Option<Uuid>, String> {
    match value {
        Some(raw) => Uuid::parse_str(raw)
            .map(Some)
            .map_err(|err| format!("invalid project_id '{}': {}", raw, err)),
        None => Ok(None),
    }
}

fn verify_dropbox_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), StatusCode> {
    let secret = std::env::var("DROPBOX_WEBHOOK_SECRET").map_err(|_| {
        error!("DROPBOX_WEBHOOK_SECRET is not set; refusing to process webhook");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let signature = headers
        .get("X-Dropbox-Signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            warn!("Dropbox webhook missing signature header");
            StatusCode::UNAUTHORIZED
        })?;

    let signature_bytes = hex::decode(signature).map_err(|_| {
        warn!("Dropbox webhook signature header was not valid hex");
        StatusCode::UNAUTHORIZED
    })?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|err| {
        error!("Failed to construct HMAC: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    mac.update(body);

    mac.verify_slice(&signature_bytes).map_err(|_| {
        warn!("Dropbox webhook signature verification failed");
        StatusCode::UNAUTHORIZED
    })
}

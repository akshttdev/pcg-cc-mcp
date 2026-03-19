use axum::{
    Router,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
};
use chrono::Utc;
use db::models::dropbox_source::{DropboxSource, render_reference_name};
use deployment::Deployment;
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
pub async fn dropbox_webhook_verify(Query(params): Query<DropboxChallenge>) -> impl IntoResponse {
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
                    errors.push(format!("{}: failed to ingest ({})", hint.source_url, err));
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
                            errors.push(format!("{}: failed to ingest ({})", source.label, err));
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
        .route("/webhooks/github", post(github_webhook_handler))
}

// ── GitHub Webhooks ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GitHubIssue {
    number: u64,
    title: String,
    body: Option<String>,
    html_url: String,
    user: Option<GitHubUser>,
    labels: Option<Vec<GitHubLabel>>,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubIssueEvent {
    action: String,
    issue: GitHubIssue,
    repository: Option<GitHubRepo>,
}

#[derive(Debug, Deserialize)]
struct GitHubRepo {
    full_name: String,
}

/// Maximum body size for GitHub webhook payloads (256 KB).
/// GitHub payloads are typically <50 KB; this is generous headroom.
const GITHUB_WEBHOOK_MAX_BODY_SIZE: usize = 256 * 1024;

/// POST /api/webhooks/github
///
/// Handles GitHub webhook events. Currently supports:
/// - `issues` events (opened, edited, labeled) → creates DataSource → triggers workflows
///
/// Security: always validates `X-Hub-Signature-256` via `GITHUB_WEBHOOK_SECRET`.
/// In development mode (`RUST_ENV=development`), validation is skipped if the secret is unset.
async fn github_webhook_handler(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Fix 2: Reject oversized payloads
    if body.len() > GITHUB_WEBHOOK_MAX_BODY_SIZE {
        warn!(
            "GitHub webhook payload too large: {} bytes (max {})",
            body.len(),
            GITHUB_WEBHOOK_MAX_BODY_SIZE
        );
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let pool = &deployment.db().pool;

    // Fix 1: Require webhook secret — fail closed unless RUST_ENV=development
    match std::env::var("GITHUB_WEBHOOK_SECRET") {
        Ok(secret) => verify_github_signature(&headers, &body, &secret)?,
        Err(_) => {
            let is_dev = std::env::var("RUST_ENV")
                .map(|v| v == "development")
                .unwrap_or(false);
            if is_dev {
                warn!(
                    "GITHUB_WEBHOOK_SECRET not set — accepting unvalidated webhook (development mode)"
                );
            } else {
                error!(
                    "GITHUB_WEBHOOK_SECRET not set — rejecting webhook. Set the secret or use RUST_ENV=development to bypass."
                );
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    match event_type {
        "issues" => handle_github_issue_event(pool, &body, deployment.clone()).await,
        "ping" => {
            info!("GitHub webhook ping received");
            Ok(StatusCode::OK)
        }
        _ => {
            tracing::debug!("Ignoring GitHub event: {event_type}");
            Ok(StatusCode::OK)
        }
    }
}

async fn handle_github_issue_event(
    pool: &sqlx::SqlitePool,
    body: &[u8],
    deployment: crate::DeploymentImpl,
) -> Result<StatusCode, StatusCode> {
    use db::models::data_source::{CreateDataSource, DataSource};

    let event: GitHubIssueEvent = serde_json::from_slice(body).map_err(|e| {
        warn!("Invalid GitHub issue event payload: {e}");
        StatusCode::BAD_REQUEST
    })?;

    // Only process opened, edited, and labeled events
    if !["opened", "edited", "labeled"].contains(&event.action.as_str()) {
        return Ok(StatusCode::OK);
    }

    let issue = &event.issue;
    let repo_name = event
        .repository
        .as_ref()
        .map(|r| r.full_name.as_str())
        .unwrap_or("unknown");
    let reporter = issue
        .user
        .as_ref()
        .map(|u| u.login.as_str())
        .unwrap_or("unknown");
    let labels: Vec<String> = issue
        .labels
        .as_ref()
        .map(|l| l.iter().map(|lab| lab.name.clone()).collect())
        .unwrap_or_default();

    let content = format!(
        "GitHub Issue #{}: {}\n\nRepository: {}\nReporter: {}\nLabels: {}\nURL: {}\n\n{}",
        issue.number,
        issue.title,
        repo_name,
        reporter,
        labels.join(", "),
        issue.html_url,
        issue.body.as_deref().unwrap_or("(no description)"),
    );

    let metadata = serde_json::json!({
        "github_issue_number": issue.number,
        "github_issue_url": issue.html_url,
        "github_repo": repo_name,
        "github_reporter": reporter,
        "github_labels": labels,
        "github_action": event.action,
    });

    // Scope to ORCHA Platform project's organization
    let org_id = {
        use db::{constants::BUGREPORTS_PROJECT_ID, models::project::Project};
        Project::find_by_id(pool, &BUGREPORTS_PROJECT_ID.to_string())
            .await
            .ok()
            .flatten()
            .and_then(|p| p.organization_id)
    };

    // Fix 3: Deduplicate — check for existing DataSource for this issue number + repo
    let external_key = format!("{}#{}", repo_name, issue.number);
    let existing = sqlx::query_as::<_, DataSource>(
        r#"SELECT * FROM data_sources
           WHERE data_type = 'github_issue'
             AND json_extract(metadata, '$.github_issue_number') = ?
             AND json_extract(metadata, '$.github_repo') = ?
             AND archived_at IS NULL
           LIMIT 1"#,
    )
    .bind(issue.number as i64)
    .bind(repo_name)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let ds = if let Some(existing_ds) = existing {
        // Update existing DataSource instead of creating a duplicate
        info!(
            "Updating existing DataSource {} for GitHub issue {}",
            existing_ds.id, external_key
        );
        use db::models::data_source::UpdateDataSource;
        let update = UpdateDataSource {
            title: Some(format!("GitHub Issue #{}: {}", issue.number, issue.title)),
            description: issue.body.clone(),
            content: Some(content),
            metadata: Some(metadata.to_string()),
            ..Default::default()
        };
        DataSource::update(pool, &existing_ds.id, update)
            .await
            .map_err(|e| {
                error!("Failed to update DataSource for GitHub issue: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .unwrap_or(existing_ds)
    } else {
        let create_ds = CreateDataSource {
            organization_id: org_id,
            project_id: Some(db::constants::BUGREPORTS_PROJECT_ID.to_string()),
            created_by: Some(reporter.to_string()),
            title: format!("GitHub Issue #{}: {}", issue.number, issue.title),
            description: issue.body.clone(),
            data_type: "github_issue".to_string(),
            source_type: Some("integration".to_string()),
            file_type: None,
            content: Some(content),
            file_name: None,
            file_path: None,
            file_size_bytes: None,
            file_hash: None,
            metadata: Some(metadata.to_string()),
            folder: Some("GitHub Issues".to_string()),
        };

        DataSource::create(pool, create_ds).await.map_err(|e| {
            error!("Failed to create DataSource from GitHub issue: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Fire workflow triggers in the background
    let trigger_pool = pool.clone();
    let ds_id = ds.id.clone();
    tokio::spawn(async move {
        super::data_source_workflows::fire_triggers_for_data_source(
            trigger_pool,
            ds_id,
            deployment,
        )
        .await;
    });

    info!(
        "Created DataSource from GitHub issue #{} ({}), firing workflow triggers",
        issue.number, event.action
    );

    Ok(StatusCode::OK)
}

fn verify_github_signature(
    headers: &HeaderMap,
    body: &[u8],
    secret: &str,
) -> Result<(), StatusCode> {
    let sig_header = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            warn!("GitHub webhook missing X-Hub-Signature-256 header");
            StatusCode::UNAUTHORIZED
        })?;

    let expected_hex = sig_header.strip_prefix("sha256=").unwrap_or(sig_header);
    let expected_bytes = hex::decode(expected_hex).map_err(|_| {
        warn!("GitHub webhook signature was not valid hex");
        StatusCode::UNAUTHORIZED
    })?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
        error!("Failed to construct HMAC: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    mac.update(body);

    mac.verify_slice(&expected_bytes).map_err(|_| {
        warn!("GitHub webhook signature verification failed");
        StatusCode::UNAUTHORIZED
    })
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

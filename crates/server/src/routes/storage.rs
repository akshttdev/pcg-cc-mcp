//! Cloud storage HTTP routes — OneDrive / Dropbox / Google Drive.
//!
//! Mirrors `integrations.rs` but lives separately because storage providers
//! aren't social platforms — they have their own connector trait, account
//! state table, and sync worker.
//!
//! HTTP surface:
//! - `GET    /storage/connect/:provider`         Start OAuth (redirect)
//! - `GET    /storage/callback/:provider`        OAuth callback (exchange + persist)
//! - `GET    /storage/accounts`                  List org's connected accounts
//! - `DELETE /storage/accounts/:id`              Disconnect (revoke + delete row)
//! - `POST   /storage/accounts/:id/sync`         Force a sync now
//! - `PATCH  /storage/accounts/:id`              Update auto_sync / interval / root_path

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Duration;
use db::{
    db_uuid::DbUuid,
    models::{
        cloud_storage_account::{
            CloudStorageAccount, CreateCloudStorageAccount, UpdateCloudStorageAccount,
        },
        integration_connection::OAuthPendingState,
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    oauth_token_manager::{self, PlainTokens, StoreTokensInput},
    storage::{self, sync_worker},
};
use tracing::warn;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ─── Configuration ─────────────────────────────────────────────────────────

fn redirect_base() -> String {
    std::env::var("OAUTH_REDIRECT_BASE").unwrap_or_else(|_| "http://localhost:3002/api".into())
}

fn provider_redirect_uri(provider: &str) -> String {
    format!("{}/storage/callback/{}", redirect_base(), provider)
}

fn frontend_settings_url() -> String {
    std::env::var("STORAGE_OAUTH_SUCCESS_REDIRECT").unwrap_or_else(|_| "/settings/storage".into())
}

fn generate_state_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn validate_provider(provider: &str) -> Result<(), ApiError> {
    if matches!(provider, "onedrive" | "dropbox" | "gdrive") {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Unknown storage provider: {provider}"
        )))
    }
}

// ─── Request types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
    pub redirect_after: Option<String>,
    pub sync_root_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AccountView {
    pub id: Uuid,
    pub provider: String,
    pub account_email: Option<String>,
    pub display_name: Option<String>,
    pub status: String,
    pub auto_sync: bool,
    pub sync_interval_secs: i64,
    pub sync_root_path: Option<String>,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub total_files_synced: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<CloudStorageAccount> for AccountView {
    fn from(a: CloudStorageAccount) -> Self {
        Self {
            id: a.id,
            provider: a.provider,
            account_email: a.account_email,
            display_name: a.display_name,
            status: a.status,
            auto_sync: a.auto_sync,
            sync_interval_secs: a.sync_interval_secs,
            sync_root_path: a.sync_root_path,
            last_sync_at: a.last_sync_at,
            last_error: a.last_error,
            total_files_synced: a.total_files_synced,
            created_at: a.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub files_added: i64,
    pub files_updated: i64,
    pub files_deleted: i64,
    pub folders_seen: i64,
}

// ─── Route handlers ────────────────────────────────────────────────────────

/// `GET /storage/connect/:provider?organization_id=...`
async fn connect(
    State(deployment): State<DeploymentImpl>,
    Path(provider): Path<String>,
    Query(query): Query<ConnectQuery>,
) -> Result<Redirect, ApiError> {
    validate_provider(&provider)?;
    let pool = &deployment.db().pool;

    let connector = storage::get_connector(&provider)
        .map_err(|e| ApiError::BadRequest(format!("Connector unavailable: {e}")))?;

    // Encode the requested sync_root_path on the pending state so the callback
    // can pass it to upsert without a second round-trip.
    let metadata = query
        .sync_root_path
        .as_ref()
        .map(|p| serde_json::json!({ "sync_root_path": p }).to_string());

    let state_token = generate_state_token();
    OAuthPendingState::create(
        pool,
        &state_token,
        &provider,
        query.organization_id,
        query.project_id,
        query.redirect_after.as_deref(),
        metadata.as_deref(),
        Duration::minutes(10),
    )
    .await?;

    let redirect_uri = provider_redirect_uri(&provider);
    let auth_url = connector
        .get_auth_url(&redirect_uri, &state_token)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build auth URL: {e}")))?;

    Ok(Redirect::temporary(&auth_url))
}

/// `GET /storage/callback/:provider?code=...&state=...`
async fn callback(
    State(deployment): State<DeploymentImpl>,
    Path(provider): Path<String>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, ApiError> {
    validate_provider(&provider)?;
    let pool = &deployment.db().pool;

    if let Some(err) = query.error {
        warn!(
            "Storage OAuth provider {} returned error: {} ({})",
            provider,
            err,
            query.error_description.as_deref().unwrap_or("")
        );
        return Ok(Redirect::temporary(&format!(
            "{}?error={}",
            frontend_settings_url(),
            urlencoding::encode(&err)
        )));
    }

    let code = query
        .code
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".into()))?;
    let state_token = query
        .state
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".into()))?;

    let pending = OAuthPendingState::consume(pool, &state_token)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Invalid or expired OAuth state".into()))?;

    if pending.provider != provider {
        return Err(ApiError::BadRequest(
            "State token does not match callback provider".into(),
        ));
    }

    let connector = storage::get_connector(&provider)
        .map_err(|e| ApiError::BadRequest(format!("Connector unavailable: {e}")))?;

    let redirect_uri = provider_redirect_uri(&provider);
    let tokens = connector
        .exchange_code(&code, &redirect_uri)
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {e}")))?;

    let account_info = connector
        .get_account_info(&tokens.access_token)
        .await
        .map_err(|e| ApiError::InternalError(format!("Account info fetch failed: {e}")))?;

    // 1) Persist tokens via the unified OAuth manager.
    let connection = oauth_token_manager::store_tokens(
        pool,
        StoreTokensInput {
            organization_id: pending.organization_id,
            project_id: pending.project_id,
            provider: provider.clone(),
            provider_account_id: account_info.account_id.clone(),
            display_name: account_info.display_name.clone(),
            avatar_url: None,
            tokens: PlainTokens {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_at: tokens.expires_at,
                scopes: tokens.scope,
            },
            metadata: Some(account_info.extras.to_string()),
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to persist tokens: {e}")))?;

    // 2) Pull the optional sync_root_path that was stashed at /connect.
    let sync_root_path: Option<String> =
        serde_json::from_str::<serde_json::Value>(&pending.metadata)
            .ok()
            .and_then(|v| {
                v.get("sync_root_path")
                    .and_then(|p| p.as_str().map(|s| s.to_string()))
            });

    // 3) Upsert the storage account row.
    CloudStorageAccount::upsert(
        pool,
        CreateCloudStorageAccount {
            organization_id: pending.organization_id,
            project_id: pending.project_id,
            integration_connection_id: Some(connection.id),
            provider: provider.clone(),
            account_email: account_info.email.clone(),
            display_name: account_info.display_name.clone(),
            sync_root_path,
            metadata: Some(account_info.extras.to_string()),
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to upsert storage account: {e}")))?;

    let target = pending.redirect_after.unwrap_or_else(frontend_settings_url);
    Ok(Redirect::temporary(&format!(
        "{}?provider={}&status=connected",
        target, provider
    )))
}

/// `GET /storage/accounts?organization_id=...`
async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<ApiResponse<Vec<AccountView>>>, ApiError> {
    let pool = &deployment.db().pool;
    let accounts = CloudStorageAccount::find_by_org(pool, query.organization_id).await?;
    Ok(Json(ApiResponse::success(
        accounts.into_iter().map(AccountView::from).collect(),
    )))
}

/// `DELETE /storage/accounts/:id`
async fn disconnect(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid account ID".into()))?
        .to_uuid();

    let Some(account) = CloudStorageAccount::find_by_id(pool, id).await? else {
        return Err(ApiError::NotFound("Storage account not found".into()));
    };

    // Revoke the underlying integration_connection if linked, then delete the
    // storage account row. We keep cloud_files / cloud_storage_files (audit trail).
    if let Some(conn_id) = account.integration_connection_id {
        oauth_token_manager::revoke(pool, conn_id)
            .await
            .map_err(|e| ApiError::InternalError(format!("Revoke failed: {e}")))?;
    }
    CloudStorageAccount::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// `POST /storage/accounts/:id/sync`
async fn sync_now(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SyncResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid account ID".into()))?
        .to_uuid();

    let stats = sync_worker::sync_account(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Sync failed: {e}")))?;

    Ok(Json(ApiResponse::success(SyncResult {
        files_added: stats.files_added,
        files_updated: stats.files_updated,
        files_deleted: stats.files_deleted,
        folders_seen: stats.folders_seen,
    })))
}

/// `PATCH /storage/accounts/:id`
async fn update_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateCloudStorageAccount>,
) -> Result<Json<ApiResponse<AccountView>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid account ID".into()))?
        .to_uuid();

    let updated = CloudStorageAccount::update(pool, id, payload)
        .await?
        .ok_or_else(|| ApiError::NotFound("Storage account not found".into()))?;
    Ok(Json(ApiResponse::success(AccountView::from(updated))))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/storage/connect/{provider}", get(connect))
        .route("/storage/callback/{provider}", get(callback))
        .route("/storage/accounts", get(list_accounts))
        .route(
            "/storage/accounts/{id}",
            delete(disconnect).patch(update_account),
        )
        .route("/storage/accounts/{id}/sync", post(sync_now))
        .with_state(deployment.clone())
}

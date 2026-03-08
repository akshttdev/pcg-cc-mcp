//! QuickBooks Online Integration Routes
//!
//! Handles OAuth 2.0 connection flow, account management, and sync controls.
//! Each organization connects its own QBO account.

use axum::{
    Router,
    extract::{Path, Query, State},
    response::Redirect,
    routing::{delete, get, patch, post},
    Json,
};
use chrono::{Duration, Utc};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::quickbooks_account::{
    CreateQuickBooksAccount, QuickBooksAccount, QuickBooksEntityMap, UpdateQuickBooksAccount,
};

// ─── Configuration ──────────────────────────────────────────────────────────

fn qb_client_id() -> Result<String, ApiError> {
    std::env::var("QUICKBOOKS_CLIENT_ID")
        .map_err(|_| ApiError::InternalError("QUICKBOOKS_CLIENT_ID not configured".into()))
}

fn qb_client_secret() -> Result<String, ApiError> {
    std::env::var("QUICKBOOKS_CLIENT_SECRET")
        .map_err(|_| ApiError::InternalError("QUICKBOOKS_CLIENT_SECRET not configured".into()))
}

fn qb_redirect_uri() -> String {
    std::env::var("QUICKBOOKS_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3002/api/quickbooks/callback".into())
}

fn qb_is_sandbox() -> bool {
    std::env::var("QUICKBOOKS_ENVIRONMENT")
        .map(|v| v == "sandbox")
        .unwrap_or(true)
}

// ─── Request/Response Types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub realm_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct QBConnectionStatus {
    pub connected: bool,
    pub account: Option<QuickBooksAccount>,
    pub needs_reauth: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    token_type: String,
    #[serde(default)]
    x_refresh_token_expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub entity_types: Option<Vec<String>>,
}

// ─── Route Handlers ─────────────────────────────────────────────────────────

/// GET /quickbooks/connect?organization_id=...
/// Initiates OAuth 2.0 authorization code flow with Intuit
async fn connect(
    Query(query): Query<ConnectQuery>,
) -> Result<Redirect, ApiError> {
    let client_id = qb_client_id()?;
    let redirect_uri = qb_redirect_uri();

    // Encode org ID in state parameter for the callback
    let state = format!("org_{}", query.organization_id);

    let auth_url = format!(
        "https://appcenter.intuit.com/connect/oauth2?\
         client_id={}&\
         redirect_uri={}&\
         response_type=code&\
         scope=com.intuit.quickbooks.accounting&\
         state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state),
    );

    Ok(Redirect::temporary(&auth_url))
}

/// GET /quickbooks/callback?code=...&state=...&realmId=...
/// Handles OAuth callback from Intuit, exchanges code for tokens
async fn callback(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, ApiError> {
    // Check for error from Intuit
    if let Some(error) = query.error {
        tracing::error!("QuickBooks OAuth error: {}", error);
        return Ok(Redirect::temporary(
            &format!("/settings/integrations?qb_error={}", urlencoding::encode(&error)),
        ));
    }

    let code = query.code.ok_or_else(|| {
        ApiError::BadRequest("Missing authorization code".into())
    })?;

    let realm_id = query.realm_id.ok_or_else(|| {
        ApiError::BadRequest("Missing realmId".into())
    })?;

    let state = query.state.unwrap_or_default();

    // Extract organization_id from state
    let organization_id = state
        .strip_prefix("org_")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter".into()))?;

    // Exchange authorization code for tokens
    let client_id = qb_client_id()?;
    let client_secret = qb_client_secret()?;
    let redirect_uri = qb_redirect_uri();

    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer")
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {}", e)))?;

    if !token_response.status().is_success() {
        let error_body = token_response.text().await.unwrap_or_default();
        tracing::error!("QBO token exchange failed: {}", error_body);
        return Ok(Redirect::temporary(
            "/settings/integrations?qb_error=token_exchange_failed",
        ));
    }

    let tokens: TokenResponse = token_response
        .json()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse token response: {}", e)))?;

    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in);
    let pool = &deployment.db().pool;

    let environment = if qb_is_sandbox() { "sandbox" } else { "production" };
    let env_enum = if qb_is_sandbox() {
        db::models::quickbooks_account::QBEnvironment::Sandbox
    } else {
        db::models::quickbooks_account::QBEnvironment::Production
    };

    // Fetch company name from QBO
    let company_name = fetch_company_name(
        &tokens.access_token,
        &realm_id,
        environment,
    )
    .await
    .ok();

    // Check if this org+realm already exists
    let existing = QuickBooksAccount::find_by_realm(pool, organization_id, &realm_id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(existing_account) = existing {
        // Update existing connection
        QuickBooksAccount::update_tokens(
            pool,
            existing_account.id,
            &tokens.access_token,
            Some(&tokens.refresh_token),
            Some(expires_at),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

        if let Some(name) = &company_name {
            QuickBooksAccount::update(
                pool,
                existing_account.id,
                UpdateQuickBooksAccount {
                    company_name: Some(name.clone()),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        }

        tracing::info!(
            "Reconnected QuickBooks for org {} realm {}",
            organization_id,
            realm_id
        );
    } else {
        // Create new connection
        QuickBooksAccount::create(
            pool,
            CreateQuickBooksAccount {
                organization_id,
                realm_id: realm_id.clone(),
                company_name,
                access_token: Some(tokens.access_token),
                refresh_token: Some(tokens.refresh_token),
                token_expires_at: Some(expires_at),
                environment: Some(env_enum),
                connected_by: None,
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

        tracing::info!(
            "Connected QuickBooks for org {} realm {}",
            organization_id,
            realm_id
        );
    }

    Ok(Redirect::temporary(
        "/settings/integrations?qb_connected=true",
    ))
}

/// GET /quickbooks/status?organization_id=...
/// Returns connection status for an organization
async fn status(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiResponse<QBConnectionStatus>>, ApiError> {
    let pool = &deployment.db().pool;
    let accounts =
        QuickBooksAccount::find_by_organization(pool, query.organization_id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    let status = if let Some(account) = accounts.into_iter().next() {
        let needs_reauth = account.needs_token_refresh() || account.status == "expired";
        QBConnectionStatus {
            connected: account.status == "active",
            needs_reauth,
            account: Some(account),
        }
    } else {
        QBConnectionStatus {
            connected: false,
            account: None,
            needs_reauth: false,
        }
    };

    Ok(Json(ApiResponse::success(status)))
}

/// GET /quickbooks/accounts?organization_id=...
/// Lists all QB accounts for an organization
async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<QuickBooksAccount>>>, ApiError> {
    let pool = &deployment.db().pool;
    let accounts =
        QuickBooksAccount::find_by_organization(pool, query.organization_id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(accounts)))
}

/// GET /quickbooks/accounts/:id
async fn get_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<QuickBooksAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = QuickBooksAccount::find_by_id(pool, id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(account)))
}

/// PATCH /quickbooks/accounts/:id
/// Update sync settings for a QB connection
async fn update_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateQuickBooksAccount>,
) -> Result<Json<ApiResponse<QuickBooksAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = QuickBooksAccount::update(pool, id, update).await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(account)))
}

/// DELETE /quickbooks/accounts/:id
/// Disconnect QuickBooks for this organization
async fn disconnect(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Revoke the token at Intuit (best effort)
    let account = QuickBooksAccount::find_by_id(pool, id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    if let Some(ref token) = account.refresh_token {
        let _ = revoke_token(token).await;
    }

    QuickBooksAccount::delete(pool, id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /quickbooks/accounts/:id/refresh
/// Force-refresh the OAuth token
async fn refresh_token(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = QuickBooksAccount::find_by_id(pool, id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    let refresh = account.refresh_token.as_deref().ok_or_else(|| {
        ApiError::BadRequest("No refresh token available".into())
    })?;

    let tokens = exchange_refresh_token(refresh).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in);

    QuickBooksAccount::update_tokens(
        pool,
        id,
        &tokens.access_token,
        Some(&tokens.refresh_token),
        Some(expires_at),
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// POST /quickbooks/accounts/:id/sync
/// Trigger a manual sync
async fn trigger_sync(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(_req): Json<SyncRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let _account = QuickBooksAccount::find_by_id(pool, id).await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    // TODO: Implement actual sync logic - for now mark the sync attempt
    QuickBooksAccount::update_sync_status(pool, id, "active").await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Sync initiated",
        "account_id": id.to_string(),
    }))))
}

/// GET /quickbooks/accounts/:id/entity-map
/// List all entity mappings for a QB account
async fn list_entity_maps(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<QuickBooksEntityMap>>>, ApiError> {
    let pool = &deployment.db().pool;
    let maps = QuickBooksEntityMap::list_for_account(pool, id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(maps)))
}

// ─── Helper Functions ───────────────────────────────────────────────────────

async fn fetch_company_name(
    access_token: &str,
    realm_id: &str,
    environment: &str,
) -> Result<String, String> {
    let base_url = if environment == "sandbox" {
        "https://sandbox-quickbooks.api.intuit.com"
    } else {
        "https://quickbooks.api.intuit.com"
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "{}/v3/company/{}/companyinfo/{}?minorversion=75",
            base_url, realm_id, realm_id
        ))
        .header("Accept", "application/json")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch company info: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse company info: {}", e))?;

    body["CompanyInfo"]["CompanyName"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Company name not found in response".into())
}

async fn exchange_refresh_token(refresh_token: &str) -> Result<TokenResponse, ApiError> {
    let client_id = qb_client_id()?;
    let client_secret = qb_client_secret()?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer")
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Token refresh failed: {}", e)))?;

    if !response.status().is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(ApiError::InternalError(format!(
            "Token refresh failed: {}",
            error_body
        )));
    }

    response
        .json::<TokenResponse>()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse refresh response: {}", e)))
}

async fn revoke_token(token: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let _ = client
        .post("https://developer.api.intuit.com/v2/oauth2/tokens/revoke")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await;
    Ok(())
}

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // OAuth flow (connect is public-ish, callback must be accessible)
        .route("/quickbooks/connect", get(connect))
        .route("/quickbooks/callback", get(callback))
        // Account management
        .route("/quickbooks/status", get(status))
        .route("/quickbooks/accounts", get(list_accounts))
        .route("/quickbooks/accounts/{id}", get(get_account))
        .route("/quickbooks/accounts/{id}", patch(update_account))
        .route("/quickbooks/accounts/{id}", delete(disconnect))
        .route("/quickbooks/accounts/{id}/refresh", post(refresh_token))
        .route("/quickbooks/accounts/{id}/sync", post(trigger_sync))
        .route(
            "/quickbooks/accounts/{id}/entity-map",
            get(list_entity_maps),
        )
}

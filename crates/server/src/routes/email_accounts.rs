//! Email Account Management Routes
//!
//! Handles Gmail and Zoho Mail connections, OAuth flows, and email account operations.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete, patch},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::email_account::{
    EmailAccount, CreateEmailAccount, UpdateEmailAccount, EmailProvider
};

#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    pub project_id: Option<Uuid>,
    pub provider: Option<String>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct OAuthUrlResponse {
    pub auth_url: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct InitiateOAuthRequest {
    pub project_id: Uuid,
    pub provider: String,
    pub redirect_uri: String,
}

/// GET /email/accounts - List email accounts
async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<ApiResponse<Vec<EmailAccount>>>, ApiError> {
    let pool = &deployment.db().pool;

    let accounts = if let Some(project_id) = query.project_id {
        EmailAccount::find_by_project(pool, project_id).await?
    } else if query.active_only.unwrap_or(false) {
        EmailAccount::find_active(pool).await?
    } else {
        // Return empty if no project specified and not active_only
        vec![]
    };

    // Filter by provider if specified
    let accounts = if let Some(provider) = query.provider {
        accounts
            .into_iter()
            .filter(|a| a.provider == provider)
            .collect()
    } else {
        accounts
    };

    Ok(Json(ApiResponse::success(accounts)))
}

/// POST /email/accounts - Create email account (after OAuth completion)
async fn create_account(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateEmailAccount>,
) -> Result<Json<ApiResponse<EmailAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = EmailAccount::create(pool, data).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// GET /email/accounts/:id - Get single account
async fn get_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<EmailAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = EmailAccount::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// PATCH /email/accounts/:id - Update account
async fn update_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateEmailAccount>,
) -> Result<Json<ApiResponse<EmailAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = EmailAccount::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// DELETE /email/accounts/:id - Disconnect account
async fn delete_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    EmailAccount::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /email/accounts/:id/sync - Trigger manual sync
async fn trigger_sync(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<EmailAccount>>, ApiError> {
    let pool = &deployment.db().pool;

    // Update sync status to indicate sync is starting
    EmailAccount::update_sync_status(pool, id, "active", None).await?;

    // TODO: Trigger actual email sync background job here
    // For now, just return the updated account

    let account = EmailAccount::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// POST /email/oauth/initiate - Start OAuth flow
async fn initiate_oauth(
    Json(request): Json<InitiateOAuthRequest>,
) -> Result<Json<ApiResponse<OAuthUrlResponse>>, ApiError> {
    let provider: EmailProvider = request.provider.parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;

    // Generate a state token that encodes the project along with a random nonce
    let state_nonce = uuid::Uuid::new_v4().to_string();
    let state_raw = format!("{}:{}", request.project_id, state_nonce);
    let state_param = urlencoding::encode(&state_raw);

    let auth_url = match provider {
        EmailProvider::Gmail => {
            let client_id = std::env::var("GOOGLE_CLIENT_ID")
                .unwrap_or_default();
            let scopes = EmailAccount::gmail_scopes().join(" ");

            format!(
                "https://accounts.google.com/o/oauth2/v2/auth?\
                client_id={}&\
                redirect_uri={}&\
                response_type=code&\
                scope={}&\
                state={}&\
                access_type=offline&\
                prompt=consent",
                client_id,
                urlencoding::encode(&request.redirect_uri),
                urlencoding::encode(&scopes),
                state_param
            )
        }
        EmailProvider::Zoho => {
            let client_id = std::env::var("ZOHO_CLIENT_ID")
                .unwrap_or_default();
            let scopes = [
                EmailAccount::zoho_mail_scopes(),
                EmailAccount::zoho_crm_scopes(),
            ].concat().join(",");

            format!(
                "https://accounts.zoho.com/oauth/v2/auth?\
                client_id={}&\
                redirect_uri={}&\
                response_type=code&\
                scope={}&\
                state={}&\
                access_type=offline&\
                prompt=consent",
                client_id,
                urlencoding::encode(&request.redirect_uri),
                urlencoding::encode(&scopes),
                state_param
            )
        }
        EmailProvider::ImapCustom => {
            return Err(ApiError::BadRequest("Custom IMAP does not use OAuth".into()));
        }
    };

    Ok(Json(ApiResponse::success(OAuthUrlResponse { auth_url, state: state_raw })))
}

/// GET /email/oauth/gmail/callback - Handle Gmail OAuth callback
async fn gmail_oauth_callback(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<ApiResponse<EmailAccount>>, ApiError> {
    let pool = &deployment.db().pool;

    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| ApiError::InternalError("GOOGLE_CLIENT_ID not configured".into()))?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .map_err(|_| ApiError::InternalError("GOOGLE_CLIENT_SECRET not configured".into()))?;
    let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/oauth/gmail/callback".into());

    // Exchange code for tokens
    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", query.code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {}", e)))?;

    // Check response status and get body for debugging
    let status = token_response.status();
    let body = token_response.text().await
        .map_err(|e| ApiError::InternalError(format!("Failed to read token response: {}", e)))?;

    tracing::info!("[GMAIL_OAUTH] Token exchange response status: {}", status);
    tracing::debug!("[GMAIL_OAUTH] Token exchange response body: {}", &body[..body.len().min(500)]);

    if !status.is_success() {
        tracing::error!("[GMAIL_OAUTH] Token exchange failed: {}", body);
        return Err(ApiError::InternalError(format!("Google token exchange failed: {}", body)));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }

    let tokens: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| ApiError::InternalError(format!("Failed to parse token response: {} - body: {}", e, &body[..body.len().min(200)])))?;

    // Get user info
    let userinfo_response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get user info: {}", e)))?;

    #[derive(Deserialize)]
    struct UserInfo {
        email: String,
        name: Option<String>,
        picture: Option<String>,
    }

    let user_info: UserInfo = userinfo_response
        .json()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse user info: {}", e)))?;

    // Parse project_id from state (format: "project_id:random_uuid")
    let project_id: Uuid = query.state.split(':').next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter".into()))?;

    // Calculate token expiry
    let token_expires_at = tokens.expires_in.map(|secs| {
        chrono::Utc::now() + chrono::Duration::seconds(secs)
    });

    // Create the email account
    let account = EmailAccount::create(pool, CreateEmailAccount {
        project_id,
        provider: EmailProvider::Gmail,
        account_type: None,
        email_address: user_info.email,
        display_name: user_info.name,
        avatar_url: user_info.picture,
        access_token: Some(tokens.access_token),
        refresh_token: tokens.refresh_token,
        token_expires_at,
        imap_host: None,
        imap_port: None,
        smtp_host: None,
        smtp_port: None,
        use_ssl: None,
        granted_scopes: Some(EmailAccount::gmail_scopes().iter().map(|s| s.to_string()).collect()),
        metadata: None,
    }).await?;

    Ok(Json(ApiResponse::success(account)))
}

/// GET /email/oauth/zoho/callback - Handle Zoho OAuth callback
async fn zoho_oauth_callback(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<ApiResponse<EmailAccount>>, ApiError> {
    let pool = &deployment.db().pool;

    let client_id = std::env::var("ZOHO_CLIENT_ID")
        .map_err(|_| ApiError::InternalError("ZOHO_CLIENT_ID not configured".into()))?;
    let client_secret = std::env::var("ZOHO_CLIENT_SECRET")
        .map_err(|_| ApiError::InternalError("ZOHO_CLIENT_SECRET not configured".into()))?;
    let redirect_uri = std::env::var("ZOHO_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/oauth/zoho/callback".into());

    // Zoho uses different domains for different regions
    let zoho_domain = std::env::var("ZOHO_DOMAIN").unwrap_or_else(|_| "com".into());

    // Exchange code for tokens
    let client = reqwest::Client::new();
    let token_response = client
        .post(format!("https://accounts.zoho.{}/oauth/v2/token", zoho_domain))
        .form(&[
            ("code", query.code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {}", e)))?;

    // Check response status and get body for debugging
    let status = token_response.status();
    let body = token_response.text().await
        .map_err(|e| ApiError::InternalError(format!("Failed to read token response: {}", e)))?;

    tracing::info!("[ZOHO_OAUTH] Token exchange response status: {}", status);
    tracing::debug!("[ZOHO_OAUTH] Token exchange response body: {}", &body[..body.len().min(500)]);

    if !status.is_success() {
        tracing::error!("[ZOHO_OAUTH] Token exchange failed: {}", body);
        return Err(ApiError::InternalError(format!("Zoho token exchange failed: {}", body)));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }

    let tokens: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| ApiError::InternalError(format!("Failed to parse token response: {} - body: {}", e, &body[..body.len().min(200)])))?;

    // Get user info from Zoho
    let userinfo_response = client
        .get(format!("https://mail.zoho.{}/api/accounts", zoho_domain))
        .header("Authorization", format!("Zoho-oauthtoken {}", tokens.access_token))
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get user info: {}", e)))?;

    #[derive(Deserialize)]
    struct ZohoAccountsResponse {
        data: Vec<ZohoAccount>,
    }

    #[derive(Deserialize)]
    struct ZohoAccount {
        #[serde(rename = "accountId")]
        account_id: String,
        #[serde(rename = "emailAddress")]
        email_address: Vec<ZohoEmailAddress>,
        #[serde(rename = "displayName")]
        display_name: Option<String>,
    }

    #[derive(Deserialize)]
    struct ZohoEmailAddress {
        #[serde(rename = "mailId")]
        mail_id: String,
        #[serde(rename = "isPrimary")]
        is_primary: bool,
    }

    let accounts_response: ZohoAccountsResponse = userinfo_response
        .json()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse Zoho response: {}", e)))?;

    let zoho_account = accounts_response.data.first()
        .ok_or_else(|| ApiError::InternalError("No Zoho accounts found".into()))?;

    let primary_email = zoho_account.email_address.iter()
        .find(|e| e.is_primary)
        .or_else(|| zoho_account.email_address.first())
        .map(|e| e.mail_id.clone())
        .ok_or_else(|| ApiError::InternalError("No email address found".into()))?;

    // Parse project_id from state
    let project_id: Uuid = query.state.split(':').next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter".into()))?;

    // Calculate token expiry
    let token_expires_at = tokens.expires_in.map(|secs| {
        chrono::Utc::now() + chrono::Duration::seconds(secs)
    });

    let scopes: Vec<String> = [
        EmailAccount::zoho_mail_scopes(),
        EmailAccount::zoho_crm_scopes(),
    ].concat().iter().map(|s| s.to_string()).collect();

    // Create the email account
    let account = EmailAccount::create(pool, CreateEmailAccount {
        project_id,
        provider: EmailProvider::Zoho,
        account_type: None,
        email_address: primary_email,
        display_name: zoho_account.display_name.clone(),
        avatar_url: None,
        access_token: Some(tokens.access_token),
        refresh_token: tokens.refresh_token,
        token_expires_at,
        imap_host: None,
        imap_port: None,
        smtp_host: None,
        smtp_port: None,
        use_ssl: None,
        granted_scopes: Some(scopes),
        metadata: Some(serde_json::json!({
            "zoho_account_id": zoho_account.account_id,
            "zoho_domain": zoho_domain,
        })),
    }).await?;

    Ok(Json(ApiResponse::success(account)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/email/accounts", get(list_accounts))
        .route("/email/accounts", post(create_account))
        .route("/email/accounts/{id}", get(get_account))
        .route("/email/accounts/{id}", patch(update_account))
        .route("/email/accounts/{id}", delete(delete_account))
        .route("/email/accounts/{id}/sync", post(trigger_sync))
        .route("/email/oauth/initiate", post(initiate_oauth))
        .route("/email/oauth/gmail/callback", get(gmail_oauth_callback))
        .route("/email/oauth/zoho/callback", get(zoho_oauth_callback))
}

//! Centralized OAuth token store-and-refresh.
//!
//! All providers (social, cloud storage, email, financial, calendar) use this
//! service to persist and rotate their tokens. It owns the encryption boundary:
//! callers hand it plaintext tokens from a fresh OAuth exchange, and it returns
//! plaintext on the way back out — the database only ever sees ciphertext.
//!
//! ```text
//!   OAuth callback   ──► store_tokens(plaintext)  ──► AES-GCM ──► DB
//!   business logic   ◄── get_access_token()       ◄── AES-GCM ◄── DB
//!   refresh worker   ──► refresh_one() (auto-rotates near expiry)
//! ```

use chrono::{DateTime, Utc};
use db::models::integration_connection::{
    CreateIntegrationConnection, IntegrationConnection, UpdateTokens,
};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::services::{oauth_crypto, social, storage};

#[derive(Debug, Error)]
pub enum TokenManagerError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Crypto(#[from] oauth_crypto::CryptoError),
    #[error("connection not found: {0}")]
    NotFound(Uuid),
    #[error("connection has no refresh token")]
    NoRefreshToken,
    #[error("provider {0} does not support automatic token refresh yet")]
    UnsupportedProvider(String),
    #[error("refresh failed: {0}")]
    RefreshFailed(String),
}

/// Plaintext tokens — only ever held in memory, never written to disk.
#[derive(Debug, Clone)]
pub struct PlainTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
}

/// Identifying info for a freshly-completed OAuth exchange.
#[derive(Debug, Clone)]
pub struct StoreTokensInput {
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
    pub provider: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub tokens: PlainTokens,
    /// Optional JSON metadata (provider-specific extras: realm_id, page_id, ...).
    pub metadata: Option<String>,
}

/// Store tokens after an OAuth exchange. Encrypts before persisting.
pub async fn store_tokens(
    pool: &SqlitePool,
    input: StoreTokensInput,
) -> Result<IntegrationConnection, TokenManagerError> {
    let access_ct = oauth_crypto::encrypt(&input.tokens.access_token)?;
    let refresh_ct = oauth_crypto::encrypt_optional(input.tokens.refresh_token.as_deref())?;

    let conn = IntegrationConnection::upsert(
        pool,
        CreateIntegrationConnection {
            organization_id: input.organization_id,
            project_id: input.project_id,
            provider: input.provider,
            provider_account_id: input.provider_account_id,
            display_name: input.display_name,
            avatar_url: input.avatar_url,
            access_token_ciphertext: Some(access_ct),
            refresh_token_ciphertext: refresh_ct,
            token_expires_at: input.tokens.expires_at,
            scopes: input.tokens.scopes,
            metadata: input.metadata,
        },
    )
    .await?;
    Ok(conn)
}

/// Get a usable access token for a connection. Auto-refreshes if it's within
/// 5 minutes of expiry. Returns the *plaintext* access token.
pub async fn get_access_token(
    pool: &SqlitePool,
    connection_id: Uuid,
) -> Result<String, TokenManagerError> {
    let conn = IntegrationConnection::find_by_id(pool, connection_id)
        .await?
        .ok_or(TokenManagerError::NotFound(connection_id))?;

    if conn.needs_refresh() && conn.refresh_token_ciphertext.is_some() {
        let refreshed = refresh_one(pool, &conn).await?;
        return decrypt_access(&refreshed);
    }

    decrypt_access(&conn)
}

/// Decrypt the access token from a connection row.
pub fn decrypt_access(conn: &IntegrationConnection) -> Result<String, TokenManagerError> {
    let ct = conn
        .access_token_ciphertext
        .as_deref()
        .ok_or_else(|| TokenManagerError::RefreshFailed("connection has no access token".into()))?;
    Ok(oauth_crypto::decrypt(ct)?)
}

/// Refresh tokens for a single connection by calling the provider's refresh
/// endpoint. Updates the DB row in place and returns the fresh row.
///
/// Marks the connection `status='expired'` and propagates the error if the
/// provider rejects the refresh.
pub async fn refresh_one(
    pool: &SqlitePool,
    conn: &IntegrationConnection,
) -> Result<IntegrationConnection, TokenManagerError> {
    let refresh_ct = conn
        .refresh_token_ciphertext
        .as_deref()
        .ok_or(TokenManagerError::NoRefreshToken)?;
    let refresh_plain = oauth_crypto::decrypt(refresh_ct)?;

    let new_tokens = match dispatch_refresh(&conn.provider, &refresh_plain).await {
        Ok(t) => t,
        Err(e) => {
            // Mark as expired so the worker stops trying. User must reconnect.
            let _ =
                IntegrationConnection::mark_status(pool, conn.id, "expired", Some(&e.to_string()))
                    .await;
            return Err(e);
        }
    };

    let access_ct = oauth_crypto::encrypt(&new_tokens.access_token)?;
    let new_refresh_ct = oauth_crypto::encrypt_optional(new_tokens.refresh_token.as_deref())?;

    let updated = IntegrationConnection::update_tokens(
        pool,
        conn.id,
        UpdateTokens {
            access_token_ciphertext: Some(access_ct),
            refresh_token_ciphertext: new_refresh_ct,
            token_expires_at: new_tokens.expires_at,
            scopes: new_tokens.scopes,
        },
    )
    .await?;
    Ok(updated)
}

/// Disconnect a connection — keeps the row for audit but blanks the tokens.
pub async fn revoke(pool: &SqlitePool, connection_id: Uuid) -> Result<(), TokenManagerError> {
    sqlx::query(
        "UPDATE integration_connections SET \
            access_token_ciphertext = NULL, \
            refresh_token_ciphertext = NULL, \
            status = 'disconnected', \
            updated_at = datetime('now','subsec') \
         WHERE id = ?1",
    )
    .bind(connection_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── Provider dispatch ─────────────────────────────────────────────────────
//
// Each provider exposes a refresh primitive somewhere. We translate the
// `provider` column into a call against that primitive. New integrations add
// their arm here as they come online.

async fn dispatch_refresh(
    provider: &str,
    refresh_token: &str,
) -> Result<PlainTokens, TokenManagerError> {
    // Social platforms route through the existing PlatformConnector trait.
    if let Ok(platform) = provider.parse::<db::models::social_account::SocialPlatform>() {
        let connector = social::get_connector(platform)
            .map_err(|e| TokenManagerError::UnsupportedProvider(e.to_string()))?;
        let tokens = connector
            .refresh_token(refresh_token)
            .await
            .map_err(|e| TokenManagerError::RefreshFailed(e.to_string()))?;
        return Ok(PlainTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: tokens.expires_at,
            scopes: tokens.scope,
        });
    }

    // Cloud storage providers (onedrive / dropbox / gdrive) route through StorageConnector.
    if matches!(provider, "onedrive" | "dropbox" | "gdrive") {
        let connector = storage::get_connector(provider)
            .map_err(|e| TokenManagerError::UnsupportedProvider(e.to_string()))?;
        let tokens = connector
            .refresh_token(refresh_token)
            .await
            .map_err(|e| TokenManagerError::RefreshFailed(e.to_string()))?;
        return Ok(PlainTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: tokens.expires_at,
            scopes: tokens.scope,
        });
    }

    // Calendar providers: standard OAuth2 `refresh_token` grant against the
    // provider's token endpoint. Each reads its client credentials from the
    // same env vars used during the initial authorization (see
    // `crates/server/src/routes/calendar.rs`). Google does not rotate the
    // refresh token on refresh; Microsoft does — `update_tokens` uses
    // COALESCE so `None` preserves the existing one and `Some(new)` replaces it.
    if provider == "google_calendar" {
        let client_id = std::env::var("GOOGLE_CLIENT_ID").map_err(|_| {
            TokenManagerError::RefreshFailed("GOOGLE_CLIENT_ID not configured".into())
        })?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| {
            TokenManagerError::RefreshFailed("GOOGLE_CLIENT_SECRET not configured".into())
        })?;
        return refresh_via_oauth2(
            "https://oauth2.googleapis.com/token",
            &client_id,
            &client_secret,
            refresh_token,
            None,
        )
        .await;
    }

    if provider == "outlook_calendar" {
        let client_id = std::env::var("MICROSOFT_CLIENT_ID").map_err(|_| {
            TokenManagerError::RefreshFailed("MICROSOFT_CLIENT_ID not configured".into())
        })?;
        let client_secret = std::env::var("MICROSOFT_CLIENT_SECRET").map_err(|_| {
            TokenManagerError::RefreshFailed("MICROSOFT_CLIENT_SECRET not configured".into())
        })?;
        let scope = std::env::var("MICROSOFT_CALENDAR_SCOPES")
            .unwrap_or_else(|_| "Calendars.Read offline_access User.Read openid".into());
        return refresh_via_oauth2(
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            &client_id,
            &client_secret,
            refresh_token,
            Some(&scope),
        )
        .await;
    }

    Err(TokenManagerError::UnsupportedProvider(provider.to_string()))
}

/// Standard OAuth2 `grant_type=refresh_token` exchange. Returns plaintext
/// tokens parsed from the provider's JSON response; surfaces a structured
/// error if the request fails or the provider rejects the refresh token.
async fn refresh_via_oauth2(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    scope: Option<&str>,
) -> Result<PlainTokens, TokenManagerError> {
    #[derive(serde::Deserialize)]
    struct RefreshResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
        scope: Option<String>,
    }

    let mut form: Vec<(&str, &str)> = vec![
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    if let Some(s) = scope {
        form.push(("scope", s));
    }

    let resp = reqwest::Client::new()
        .post(token_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| TokenManagerError::RefreshFailed(format!("network: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(TokenManagerError::RefreshFailed(format!(
            "{status}: {body}"
        )));
    }

    let parsed: RefreshResponse = resp
        .json()
        .await
        .map_err(|e| TokenManagerError::RefreshFailed(format!("parse: {e}")))?;

    let expires_at = parsed
        .expires_in
        .map(|s| Utc::now() + chrono::Duration::seconds(s));

    Ok(PlainTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at,
        scopes: parsed.scope,
    })
}

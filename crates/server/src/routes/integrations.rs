//! Unified OAuth integration routes + background token refresh worker.
//!
//! HTTP surface:
//! - `GET  /social/connect/:platform`    Start OAuth (redirect to provider)
//! - `GET  /social/callback/:platform`   Provider redirect back; exchange + persist
//! - `GET  /integrations/connections`    List active connections for an org
//! - `DELETE /integrations/connections/:id`  Disconnect (revoke + clear tokens)
//!
//! Background:
//! - `spawn_token_refresh_loop(pool)`    15-minute scan for tokens expiring < 1h
//!
//! All token storage flows through `services::oauth_token_manager`, which owns
//! AES-GCM at-rest encryption.

use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::{delete, get},
    Json, Router,
};
use chrono::{Duration, Utc};
use db::{
    db_uuid::DbUuid,
    models::{
        integration_connection::{IntegrationConnection, OAuthPendingState},
        social_account::{CreateSocialAccount, SocialAccount, SocialPlatform},
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    oauth_token_manager::{self, PlainTokens, StoreTokensInput, TokenManagerError},
    social,
};
use sqlx::SqlitePool;
use tokio::time::interval;
use tracing::{error, info, warn};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ─── Configuration ─────────────────────────────────────────────────────────

fn redirect_base() -> String {
    std::env::var("OAUTH_REDIRECT_BASE").unwrap_or_else(|_| "http://localhost:3002/api".into())
}

fn provider_redirect_uri(platform: &str) -> String {
    format!("{}/social/callback/{}", redirect_base(), platform)
}

fn frontend_settings_url() -> String {
    std::env::var("OAUTH_SUCCESS_REDIRECT").unwrap_or_else(|_| "/settings/integrations".into())
}

fn generate_state_token() -> String {
    // 256 bits of entropy from two v4 UUIDs — plenty for CSRF state.
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

// ─── Request types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
    /// Optional override for the post-success redirect (defaults to /settings/integrations).
    pub redirect_after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListConnectionsQuery {
    pub organization_id: Uuid,
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionView {
    pub id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub scopes: Option<String>,
    pub token_expires_at: Option<chrono::DateTime<Utc>>,
    pub last_sync_at: Option<chrono::DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<IntegrationConnection> for ConnectionView {
    fn from(c: IntegrationConnection) -> Self {
        Self {
            id: c.id,
            provider: c.provider,
            provider_account_id: c.provider_account_id,
            display_name: c.display_name,
            avatar_url: c.avatar_url,
            status: c.status,
            scopes: c.scopes,
            token_expires_at: c.token_expires_at,
            last_sync_at: c.last_sync_at,
            last_error: c.last_error,
            created_at: c.created_at,
        }
    }
}

// ─── Route handlers ────────────────────────────────────────────────────────

/// `GET /social/connect/:platform?organization_id=...`
///
/// Stores a CSRF state token, then 302s the user to the platform's authorize URL.
async fn connect_social(
    State(deployment): State<DeploymentImpl>,
    Path(platform): Path<String>,
    Query(query): Query<ConnectQuery>,
) -> Result<Redirect, ApiError> {
    let pool = &deployment.db().pool;
    let platform_enum = SocialPlatform::from_str(&platform)
        .map_err(|e| ApiError::BadRequest(format!("Unknown platform: {e}")))?;

    let connector = social::get_connector(platform_enum)
        .map_err(|e| ApiError::BadRequest(format!("Connector unavailable: {e}")))?;

    let state_token = generate_state_token();
    OAuthPendingState::create(
        pool,
        &state_token,
        &platform,
        query.organization_id,
        query.project_id,
        query.redirect_after.as_deref(),
        None,
        Duration::minutes(10),
    )
    .await?;

    let redirect_uri = provider_redirect_uri(&platform);
    let auth_url = connector
        .get_auth_url(&redirect_uri, &state_token)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build auth URL: {e}")))?;

    Ok(Redirect::temporary(&auth_url))
}

/// `GET /social/callback/:platform?code=...&state=...`
async fn callback_social(
    State(deployment): State<DeploymentImpl>,
    Path(platform): Path<String>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, ApiError> {
    let pool = &deployment.db().pool;

    if let Some(err) = query.error {
        warn!(
            "OAuth provider {} returned error: {} ({})",
            platform,
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

    if pending.provider != platform {
        return Err(ApiError::BadRequest(
            "State token does not match callback platform".into(),
        ));
    }

    let platform_enum = SocialPlatform::from_str(&platform)
        .map_err(|e| ApiError::BadRequest(format!("Unknown platform: {e}")))?;
    let connector = social::get_connector(platform_enum)
        .map_err(|e| ApiError::BadRequest(format!("Connector unavailable: {e}")))?;

    let redirect_uri = provider_redirect_uri(&platform);
    // For PKCE-mandatory providers (TikTok), the state token IS the code_verifier —
    // it was hashed into code_challenge in get_auth_url. For non-PKCE providers,
    // the connector ignores this param.
    let tokens = connector
        .exchange_code(&code, &redirect_uri, Some(&state_token))
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {e}")))?;

    let profile = connector
        .get_profile(&tokens.access_token)
        .await
        .map_err(|e| ApiError::InternalError(format!("Profile fetch failed: {e}")))?;

    // 1) Persist the canonical tokens via the OAuth Token Manager.
    let connection = oauth_token_manager::store_tokens(
        pool,
        StoreTokensInput {
            organization_id: pending.organization_id,
            project_id: pending.project_id,
            provider: platform.clone(),
            provider_account_id: profile.platform_account_id.clone(),
            display_name: profile.display_name.clone(),
            avatar_url: profile.avatar_url.clone(),
            tokens: PlainTokens {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_at: tokens.expires_at,
                scopes: tokens.scope,
            },
            metadata: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to persist tokens: {e}")))?;

    // 2) Mirror provider-specific data into social_accounts so the existing
    //    publishing/scheduling code keeps working. The `integration_connection_id`
    //    FK lets new code defer to the unified store for live tokens.
    if let Some(project_id) = pending.project_id {
        upsert_social_account_link(pool, project_id, &connection, &platform_enum, &profile).await?;
    } else {
        // Org-owned brand account — no project_id, owned by the organization directly.
        upsert_org_social_account_link(
            pool,
            &pending.organization_id.to_string(),
            &connection,
            &platform_enum,
            &profile,
        )
        .await?;
    }

    let target = pending.redirect_after.unwrap_or_else(frontend_settings_url);
    Ok(Redirect::temporary(&format!(
        "{}?provider={}&status=connected",
        target, platform
    )))
}

async fn upsert_social_account_link(
    pool: &SqlitePool,
    project_id: Uuid,
    connection: &IntegrationConnection,
    platform: &SocialPlatform,
    profile: &social::ProfileInfo,
) -> Result<(), ApiError> {
    let existing: Option<SocialAccount> = sqlx::query_as(
        "SELECT * FROM social_accounts \
         WHERE project_id = ?1 AND platform = ?2 AND platform_account_id = ?3 \
         LIMIT 1",
    )
    // project_id stored as TEXT to match projects.id FK target (see social_account.rs)
    .bind(project_id.to_string())
    .bind(platform.to_string())
    .bind(&profile.platform_account_id)
    .fetch_optional(pool)
    .await?;

    if let Some(existing) = existing {
        sqlx::query(
            "UPDATE social_accounts SET \
                integration_connection_id = ?1, \
                username = COALESCE(?2, username), \
                display_name = COALESCE(?3, display_name), \
                profile_url = COALESCE(?4, profile_url), \
                avatar_url = COALESCE(?5, avatar_url), \
                follower_count = COALESCE(?6, follower_count), \
                status = 'active', \
                last_error = NULL, \
                updated_at = datetime('now','subsec') \
             WHERE id = ?7",
        )
        .bind(connection.id)
        .bind(&profile.username)
        .bind(&profile.display_name)
        .bind(&profile.profile_url)
        .bind(&profile.avatar_url)
        .bind(profile.follower_count)
        .bind(existing.id)
        .execute(pool)
        .await?;
    } else {
        let create = CreateSocialAccount {
            project_id: Some(project_id),
            organization_id: None,
            user_id: None,
            platform: *platform,
            account_type: None,
            platform_account_id: profile.platform_account_id.clone(),
            username: Some(profile.username.clone()),
            display_name: profile.display_name.clone(),
            profile_url: profile.profile_url.clone(),
            avatar_url: profile.avatar_url.clone(),
            // Tokens live in integration_connections — keep these NULL going forward.
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            metadata: None,
        };
        let new_account = SocialAccount::create(pool, create).await?;
        sqlx::query("UPDATE social_accounts SET integration_connection_id = ?1 WHERE id = ?2")
            .bind(connection.id)
            .bind(new_account.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Create or update a social account owned by an organization (no project_id).
async fn upsert_org_social_account_link(
    pool: &SqlitePool,
    organization_id: &str,
    connection: &IntegrationConnection,
    platform: &SocialPlatform,
    profile: &social::ProfileInfo,
) -> Result<(), ApiError> {
    let existing: Option<SocialAccount> = sqlx::query_as(
        "SELECT * FROM social_accounts \
         WHERE organization_id = ?1 AND platform = ?2 AND platform_account_id = ?3 \
         LIMIT 1",
    )
    .bind(organization_id)
    .bind(platform.to_string())
    .bind(&profile.platform_account_id)
    .fetch_optional(pool)
    .await?;

    if let Some(existing) = existing {
        sqlx::query(
            "UPDATE social_accounts SET \
                integration_connection_id = ?1, \
                username = COALESCE(?2, username), \
                display_name = COALESCE(?3, display_name), \
                profile_url = COALESCE(?4, profile_url), \
                avatar_url = COALESCE(?5, avatar_url), \
                follower_count = COALESCE(?6, follower_count), \
                status = 'active', \
                last_error = NULL, \
                updated_at = datetime('now','subsec') \
             WHERE id = ?7",
        )
        .bind(connection.id)
        .bind(&profile.username)
        .bind(&profile.display_name)
        .bind(&profile.profile_url)
        .bind(&profile.avatar_url)
        .bind(profile.follower_count)
        .bind(existing.id)
        .execute(pool)
        .await?;
    } else {
        let create = CreateSocialAccount {
            project_id: None,
            organization_id: Some(organization_id.to_string()),
            user_id: None,
            platform: *platform,
            account_type: None,
            platform_account_id: profile.platform_account_id.clone(),
            username: Some(profile.username.clone()),
            display_name: profile.display_name.clone(),
            profile_url: profile.profile_url.clone(),
            avatar_url: profile.avatar_url.clone(),
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            metadata: None,
        };
        let new_account = SocialAccount::create(pool, create).await?;
        sqlx::query("UPDATE social_accounts SET integration_connection_id = ?1 WHERE id = ?2")
            .bind(connection.id)
            .bind(new_account.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// `GET /integrations/connections?organization_id=...`
async fn list_connections(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListConnectionsQuery>,
) -> Result<Json<ApiResponse<Vec<ConnectionView>>>, ApiError> {
    let pool = &deployment.db().pool;
    let connections = match query.provider {
        Some(p) => {
            IntegrationConnection::find_by_org_and_provider(pool, query.organization_id, &p).await?
        }
        None => IntegrationConnection::find_by_org(pool, query.organization_id).await?,
    };
    let views = connections.into_iter().map(ConnectionView::from).collect();
    Ok(Json(ApiResponse::success(views)))
}

/// `DELETE /integrations/connections/:id`
async fn disconnect(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid connection ID".into()))?
        .to_uuid();
    oauth_token_manager::revoke(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Revoke failed: {e}")))?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/connect/{platform}", get(connect_social))
        .route("/social/callback/{platform}", get(callback_social))
        .route("/integrations/connections", get(list_connections))
        .route("/integrations/connections/{id}", delete(disconnect))
        .with_state(deployment.clone())
}

// ─── Background: token refresh worker ──────────────────────────────────────

/// Spawns a tokio task that scans every 15 minutes for active connections
/// whose access token is expiring within the next hour and rotates them.
///
/// On refresh failure the connection is marked `status='expired'` and the
/// admin will need to reconnect — there is no user notification yet.
pub fn spawn_token_refresh_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        // Wait 60s before the first scan so the rest of the server is up.
        let mut ticker = interval(std::time::Duration::from_secs(900)); // 15 min
        ticker.tick().await; // discard immediate fire
        loop {
            ticker.tick().await;
            if let Err(e) = refresh_expiring(&pool).await {
                error!("OAuth token refresh loop error: {e}");
            }
            if let Err(e) = OAuthPendingState::cleanup_expired(&pool).await {
                error!("OAuth pending state cleanup error: {e}");
            }
        }
    });
    info!("OAuth token refresh worker spawned (15-min interval)");
}

async fn refresh_expiring(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let due = IntegrationConnection::find_expiring_within(pool, Duration::hours(1)).await?;
    if due.is_empty() {
        return Ok(());
    }
    info!("Refreshing {} OAuth token(s) nearing expiry", due.len());

    for conn in due {
        let conn_id = conn.id;
        let provider = conn.provider.clone();
        match oauth_token_manager::refresh_one(pool, &conn).await {
            Ok(_) => info!("Refreshed token for {} (connection {})", provider, conn_id),
            Err(TokenManagerError::UnsupportedProvider(p)) => {
                // Skip silently — provider hasn't been wired up yet. The connection
                // stays active until expiry, after which it will flip to 'expired'.
                warn!("Skipping refresh for unsupported provider: {p}");
            }
            Err(e) => {
                error!("Token refresh failed for {} ({}): {}", provider, conn_id, e);
            }
        }
    }
    Ok(())
}

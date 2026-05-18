//! Social Media OAuth Routes
//!
//! Handles OAuth 2.0 connect + callback flows for social platform accounts.
//! Supported platforms: LinkedIn, Instagram, Twitter/X, TikTok.
//!
//! Flow:
//!   GET /social/oauth/:platform/connect?[project_id=&org_id=] (JWT required)
//!     → redirects user to platform consent screen
//!   GET /social/oauth/:platform/callback?code=&state= (public — platform redirects here)
//!     → exchanges code, gets profile, upserts social_accounts row
//!     → redirects to APP_BASE_URL/social?connected=<platform>

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::get,
    Router,
};
use db::models::social_account::SocialPlatform;
use deployment::Deployment;
use serde::Deserialize;
use services::services::social::get_connector;
use sqlx::SqlitePool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// The redirect URI we register with each platform. Must match exactly what
/// is configured in the platform's app settings.
fn social_callback_url(platform: &str) -> String {
    let base =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    format!("{}/api/social/oauth/{}/callback", base, platform)
}

/// Frontend success redirect target.
fn frontend_redirect(platform: &str) -> String {
    let base =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    format!("{}/social?connected={}", base, platform)
}

/// Frontend error redirect target.
fn frontend_error_redirect(platform: &str, err: &str) -> String {
    let base =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    format!(
        "{}/social?error={}&platform={}",
        base,
        urlencoding::encode(err),
        platform
    )
}

/// Verify that the required env vars exist for a platform. Returns 501 if not.
fn check_platform_configured(platform: &str) -> Result<(), ApiError> {
    let configured = match platform {
        "linkedin" => {
            std::env::var("LINKEDIN_CLIENT_ID").is_ok()
                && std::env::var("LINKEDIN_CLIENT_SECRET").is_ok()
        }
        "instagram" => {
            std::env::var("INSTAGRAM_APP_ID").is_ok()
                && std::env::var("INSTAGRAM_APP_SECRET").is_ok()
        }
        "twitter" => {
            std::env::var("TWITTER_CLIENT_ID").is_ok()
                && std::env::var("TWITTER_CLIENT_SECRET").is_ok()
        }
        "tiktok" => {
            std::env::var("TIKTOK_CLIENT_KEY").is_ok()
                && std::env::var("TIKTOK_CLIENT_SECRET").is_ok()
        }
        _ => false,
    };

    if !configured {
        Err(ApiError::InternalError(format!(
            "Platform '{}' is not configured — missing client credentials",
            platform
        )))
    } else {
        Ok(())
    }
}

/// Parse platform string into `SocialPlatform` enum, returning a 400 on unknown values.
fn parse_platform(platform: &str) -> Result<SocialPlatform, ApiError> {
    platform
        .parse::<SocialPlatform>()
        .map_err(|_| ApiError::BadRequest(format!("Unsupported platform: {platform}")))
}

// ─── State encoding ──────────────────────────────────────────────────────────
//
// We encode: `{platform}:{owner_type}:{owner_id}`
// owner_type: "project" | "org"
// owner_id:   UUID string

fn make_oauth_state(platform: &str, owner_type: &str, owner_id: &str) -> String {
    format!("{platform}:{owner_type}:{owner_id}")
}

/// Returns (platform, owner_type, owner_id)
fn parse_oauth_state(state: &str) -> Option<(&str, &str, &str)> {
    let mut parts = state.splitn(3, ':');
    let platform = parts.next()?;
    let owner_type = parts.next()?;
    let owner_id = parts.next()?;
    Some((platform, owner_type, owner_id))
}

// ─── Query param types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub project_id: Option<Uuid>,
    pub org_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /social/oauth/:platform/connect
///
/// Protected — requires JWT. Redirects user to platform OAuth consent screen.
async fn connect(
    Path(platform): Path<String>,
    Query(q): Query<ConnectQuery>,
) -> Result<Redirect, ApiError> {
    check_platform_configured(&platform)?;
    let social_platform = parse_platform(&platform)?;

    // Determine owner type and ID
    let (owner_type, owner_id) = if let Some(proj_id) = q.project_id {
        ("project".to_string(), proj_id.to_string())
    } else if let Some(org_id) = q.org_id {
        ("org".to_string(), org_id)
    } else {
        return Err(ApiError::BadRequest(
            "Either project_id or org_id is required".into(),
        ));
    };

    let connector =
        get_connector(social_platform).map_err(|e| ApiError::InternalError(e.to_string()))?;

    let redirect_uri = social_callback_url(&platform);
    let state = make_oauth_state(&platform, &owner_type, &owner_id);

    let auth_url = connector
        .get_auth_url(&redirect_uri, &state)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build auth URL: {e}")))?;

    Ok(Redirect::temporary(&auth_url))
}

/// GET /social/oauth/:platform/callback
///
/// PUBLIC — platform redirects here after user consents. Exchanges code, upserts
/// `social_accounts`, then redirects to the frontend success page.
async fn callback(
    State(deployment): State<DeploymentImpl>,
    Path(platform): Path<String>,
    Query(q): Query<CallbackQuery>,
) -> Redirect {
    // Surface errors to the user via redirect rather than 4xx (platforms expect 200).
    match callback_inner(deployment, platform.clone(), q).await {
        Ok(redirect) => redirect,
        Err(e) => {
            error!(platform = %platform, error = %e, "Social OAuth callback failed");
            Redirect::temporary(&frontend_error_redirect(&platform, &e.to_string()))
        }
    }
}

async fn callback_inner(
    deployment: DeploymentImpl,
    platform: String,
    q: CallbackQuery,
) -> Result<Redirect, ApiError> {
    // Handle user-denied or platform error
    if let Some(err) = q.error {
        let desc = q.error_description.as_deref().unwrap_or(&err);
        warn!(platform = %platform, error = %err, "Platform denied OAuth");
        return Ok(Redirect::temporary(&frontend_error_redirect(
            &platform, desc,
        )));
    }

    let code = q
        .code
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code from platform".into()))?;
    let state = q
        .state
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter from platform".into()))?;

    let (state_platform, owner_type, owner_id) = parse_oauth_state(&state)
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter — could not parse".into()))?;

    // Sanity check: state platform matches the path parameter
    if state_platform != platform {
        return Err(ApiError::BadRequest(format!(
            "State platform '{state_platform}' does not match path '{platform}'"
        )));
    }

    let social_platform = parse_platform(&platform)?;
    let connector =
        get_connector(social_platform).map_err(|e| ApiError::InternalError(e.to_string()))?;

    let redirect_uri = social_callback_url(&platform);

    // Exchange code for tokens
    let tokens = connector
        .exchange_code(&code, &redirect_uri, None)
        .await
        .map_err(|e| {
            error!(platform = %platform, error = %e, "Token exchange failed");
            ApiError::InternalError(format!("Token exchange failed: {e}"))
        })?;

    // Fetch platform profile
    let profile = connector
        .get_profile(&tokens.access_token)
        .await
        .map_err(|e| {
            error!(platform = %platform, error = %e, "Profile fetch failed");
            ApiError::InternalError(format!("Could not fetch profile: {e}"))
        })?;

    info!(
        platform = %platform,
        username = %profile.username,
        owner_type = %owner_type,
        owner_id = %owner_id,
        "Social OAuth complete — upserting account"
    );

    let pool = &deployment.db().pool;

    // Determine project_id / organization_id from owner type
    let (project_id_str, organization_id_str): (Option<String>, Option<String>) = match owner_type {
        "project" => (Some(owner_id.to_string()), None),
        "org" => (None, Some(owner_id.to_string())),
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unknown owner_type: {owner_type}"
            )));
        }
    };

    upsert_social_account(
        pool,
        &platform,
        project_id_str.as_deref(),
        organization_id_str.as_deref(),
        &profile.platform_account_id,
        profile.username.as_str(),
        profile.display_name.as_deref(),
        profile.profile_url.as_deref(),
        profile.avatar_url.as_deref(),
        &tokens.access_token,
        tokens.refresh_token.as_deref(),
        tokens.expires_at,
        profile.follower_count,
    )
    .await?;

    Ok(Redirect::temporary(&frontend_redirect(&platform)))
}

/// Upsert a social account row. Checks for an existing row by
/// (platform, platform_account_id, project_id OR organization_id); inserts if
/// absent, updates tokens/profile if present.
///
/// `social_accounts.id` is BLOB; we read it as `Uuid` via sqlx's uuid feature
/// and rebind as `Uuid` for the UPDATE WHERE clause to avoid text/blob mismatch.
#[allow(clippy::too_many_arguments)]
async fn upsert_social_account(
    pool: &SqlitePool,
    platform: &str,
    project_id: Option<&str>,
    organization_id: Option<&str>,
    platform_account_id: &str,
    username: &str,
    display_name: Option<&str>,
    profile_url: Option<&str>,
    avatar_url: Option<&str>,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    follower_count: Option<i64>,
) -> Result<(), ApiError> {
    // Try to find existing account — read id as Uuid (BLOB) so we can rebind correctly
    #[derive(sqlx::FromRow)]
    struct IdRow {
        id: Uuid,
    }

    let existing: Option<IdRow> = sqlx::query_as(
        "SELECT id FROM social_accounts \
         WHERE platform = ?1 AND platform_account_id = ?2 \
           AND (project_id = ?3 OR organization_id = ?4) \
         LIMIT 1",
    )
    .bind(platform)
    .bind(platform_account_id)
    .bind(project_id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = existing {
        // UPDATE existing account — refresh tokens and profile
        sqlx::query(
            "UPDATE social_accounts SET \
                access_token = ?2, \
                refresh_token = COALESCE(?3, refresh_token), \
                token_expires_at = ?4, \
                username = ?5, \
                display_name = COALESCE(?6, display_name), \
                avatar_url = COALESCE(?7, avatar_url), \
                profile_url = COALESCE(?8, profile_url), \
                follower_count = COALESCE(?9, follower_count), \
                status = 'active', \
                updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(row.id) // Uuid bound as BLOB — matches the BLOB primary key
        .bind(access_token)
        .bind(refresh_token)
        .bind(token_expires_at)
        .bind(username)
        .bind(display_name)
        .bind(avatar_url)
        .bind(profile_url)
        .bind(follower_count)
        .execute(pool)
        .await?;
    } else {
        // INSERT new account — id as BLOB (Uuid bound directly)
        let new_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO social_accounts \
                (id, project_id, organization_id, platform, account_type, \
                 platform_account_id, username, display_name, profile_url, avatar_url, \
                 access_token, refresh_token, token_expires_at, follower_count, status) \
             VALUES (?1, ?2, ?3, ?4, 'personal', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'active')",
        )
        .bind(new_id) // Uuid → BLOB
        .bind(project_id)
        .bind(organization_id)
        .bind(platform)
        .bind(platform_account_id)
        .bind(username)
        .bind(display_name)
        .bind(profile_url)
        .bind(avatar_url)
        .bind(access_token)
        .bind(refresh_token)
        .bind(token_expires_at)
        .bind(follower_count)
        .execute(pool)
        .await?;
    }

    Ok(())
}

// ─── Routers ─────────────────────────────────────────────────────────────────

/// Protected router — merge into protected_routes (JWT middleware applied by caller).
/// Contains the connect endpoint that kicks off the OAuth flow.
pub fn protected_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/oauth/{platform}/connect", get(connect))
        .with_state(deployment.clone())
}

/// Public router — merge into base_routes (no auth). Platform redirects back here.
pub fn public_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/oauth/{platform}/callback", get(callback))
        .with_state(deployment.clone())
}

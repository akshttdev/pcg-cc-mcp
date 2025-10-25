use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{Next, from_fn_with_state},
    response::{Json as ResponseJson, Response},
    routing::{get, post},
};
use deployment::Deployment;
use octocrab::auth::Continue;
use serde::{Deserialize, Serialize};
use services::services::{
    auth::{AuthError, DeviceFlowStartResponse},
    config::save_config_to_file,
    github_service::{GitHubService, GitHubServiceError},
};
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

// Import new auth types (will be conditionally compiled when PostgreSQL is available)
#[cfg(feature = "postgres")]
use db::{
    models::user::{LoginRequest, LoginResponse, UserProfile},
    repositories::{UserRepository, SessionRepository},
    services::AuthService,
};

// Import SQLite auth handlers
mod auth_sqlite;

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let mut router = Router::new()
        .route("/auth/github/device/start", post(device_start))
        .route("/auth/github/device/poll", post(device_poll))
        .route("/auth/github/check", get(github_check_token));

    // Add session-based auth routes when PostgreSQL feature is enabled
    #[cfg(feature = "postgres")]
    {
        router = router
            .route("/auth/login", post(login))
            .route("/auth/me", get(get_current_user))
            .route("/auth/logout", post(logout));
    }

    // Add SQLite auth routes when PostgreSQL is NOT enabled
    #[cfg(not(feature = "postgres"))]
    {
        router = router
            .route("/auth/login", post(auth_sqlite::login))
            .route("/auth/me", get(auth_sqlite::get_current_user))
            .route("/auth/logout", post(auth_sqlite::logout));
    }

    router.layer(from_fn_with_state(
        deployment.clone(),
        sentry_user_context_middleware,
    ))
}

/// POST /auth/github/device/start
async fn device_start(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<DeviceFlowStartResponse>>, ApiError> {
    let device_start_response = deployment.auth().device_start().await?;
    Ok(ResponseJson(ApiResponse::success(device_start_response)))
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(use_ts_enum)]
pub enum DevicePollStatus {
    SlowDown,
    AuthorizationPending,
    Success,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(use_ts_enum)]
pub enum CheckTokenResponse {
    Valid,
    Invalid,
}

/// POST /auth/github/device/poll
async fn device_poll(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<DevicePollStatus>>, ApiError> {
    let user_info = match deployment.auth().device_poll().await {
        Ok(info) => info,
        Err(AuthError::Pending(Continue::SlowDown)) => {
            return Ok(ResponseJson(ApiResponse::success(
                DevicePollStatus::SlowDown,
            )));
        }
        Err(AuthError::Pending(Continue::AuthorizationPending)) => {
            return Ok(ResponseJson(ApiResponse::success(
                DevicePollStatus::AuthorizationPending,
            )));
        }
        Err(e) => return Err(e.into()),
    };
    // Save to config
    {
        let config_path = utils::assets::config_path();
        let mut config = deployment.config().write().await;
        config.github.username = Some(user_info.username.clone());
        config.github.primary_email = user_info.primary_email.clone();
        config.github.oauth_token = Some(user_info.token.to_string());
        config.github_login_acknowledged = true; // Also acknowledge the GitHub login step
        save_config_to_file(&config.clone(), &config_path).await?;
    }
    let _ = deployment.update_sentry_scope().await;
    let props = serde_json::json!({
        "username": user_info.username,
        "email": user_info.primary_email,
    });
    deployment
        .track_if_analytics_allowed("$identify", props)
        .await;
    Ok(ResponseJson(ApiResponse::success(
        DevicePollStatus::Success,
    )))
}

/// GET /auth/github/check
async fn github_check_token(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CheckTokenResponse>>, ApiError> {
    let gh_config = deployment.config().read().await.github.clone();
    let Some(token) = gh_config.token() else {
        return Ok(ResponseJson(ApiResponse::success(
            CheckTokenResponse::Invalid,
        )));
    };
    let gh = GitHubService::new(&token)?;
    match gh.check_token().await {
        Ok(()) => Ok(ResponseJson(ApiResponse::success(
            CheckTokenResponse::Valid,
        ))),
        Err(GitHubServiceError::TokenInvalid) => Ok(ResponseJson(ApiResponse::success(
            CheckTokenResponse::Invalid,
        ))),
        Err(e) => Err(e.into()),
    }
}

/// Middleware to set Sentry user context for every request
pub async fn sentry_user_context_middleware(
    State(deployment): State<DeploymentImpl>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let _ = deployment.update_sentry_scope().await;
    Ok(next.run(req).await)
}

// ============================================================================
// Session-based Authentication Endpoints (PostgreSQL)
// ============================================================================

#[cfg(feature = "postgres")]
/// POST /auth/login
/// Returns session cookie for authentication
async fn login(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Get PostgreSQL pool from deployment
    let pool = deployment.pg_pool().ok_or_else(|| {
        ApiError::InternalError("PostgreSQL not configured".to_string())
    })?;

    // Find user by username
    let user = UserRepository::find_by_username(&pool, &req.username)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("Invalid credentials".to_string()))?;

    // Verify password
    if !AuthService::verify_password(&req.password, &user.password_hash)
        .map_err(|e| ApiError::InternalError(format!("Password verification error: {}", e)))?
    {
        return Err(ApiError::BadRequest("Invalid credentials".to_string()));
    }

    // Check if user is active
    if !user.is_active {
        return Err(ApiError::BadRequest("Account is disabled".to_string()));
    }

    // Update last login time
    UserRepository::update_last_login(&pool, user.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    // Generate session ID
    let session_id = AuthService::generate_session_id();

    // Create session in database
    SessionRepository::create(&pool, &session_id, user.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // Get user profile with organizations
    let profile = UserRepository::get_user_profile(&pool, user.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::InternalError("User profile not found".to_string()))?;

    // Create response with session cookie
    let cookie = format!(
        "session_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session_id,
        30 * 24 * 60 * 60 // 30 days
    );

    let response = LoginResponse {
        user: profile,
        session_id: session_id.clone(),
    };

    Ok((
        [(header::SET_COOKIE, HeaderValue::from_str(&cookie).unwrap())],
        ResponseJson(ApiResponse::<LoginResponse, LoginResponse>::success(response))
    ))
}

#[cfg(feature = "postgres")]
/// GET /auth/me
/// Returns the current user's profile from session cookie
async fn get_current_user(
    State(deployment): State<DeploymentImpl>,
    req: Request,
) -> Result<ResponseJson<ApiResponse<UserProfile>>, ApiError> {
    // Get PostgreSQL pool
    let pool = deployment.pg_pool().ok_or_else(|| {
        ApiError::InternalError("PostgreSQL not configured".to_string())
    })?;

    // Extract session ID from cookie
    let session_id = req.headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';')
                .find_map(|cookie| {
                    let cookie = cookie.trim();
                    cookie.strip_prefix("session_id=")
                })
        })
        .ok_or_else(|| ApiError::BadRequest("No session cookie found".to_string()))?;

    // Find session in database
    let session = SessionRepository::find_by_id(&pool, session_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("Invalid or expired session".to_string()))?;

    // Update last accessed time
    SessionRepository::update_last_accessed(&pool, session_id)
        .await
        .ok(); // Ignore errors for last accessed update

    // Get user profile
    let profile = UserRepository::get_user_profile(&pool, session.user_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("User not found".to_string()))?;

    Ok(ResponseJson(ApiResponse::success(profile)))
}

#[cfg(feature = "postgres")]
#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct LogoutResponse {
    pub message: String,
}

#[cfg(feature = "postgres")]
/// POST /auth/logout
/// Deletes the session from database and clears cookie
async fn logout(
    State(deployment): State<DeploymentImpl>,
    req: Request,
) -> Result<impl IntoResponse, ApiError> {
    let pool = deployment.pg_pool().ok_or_else(|| {
        ApiError::InternalError("PostgreSQL not configured".to_string())
    })?;

    // Extract session ID from cookie
    if let Some(session_id) = req.headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';')
                .find_map(|cookie| {
                    let cookie = cookie.trim();
                    cookie.strip_prefix("session_id=")
                })
        })
    {
        // Delete session from database
        SessionRepository::delete(&pool, session_id)
            .await
            .ok(); // Ignore errors
    }

    // Clear cookie
    let cookie = "session_id=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Ok((
        [(header::SET_COOKIE, HeaderValue::from_str(cookie).unwrap())],
        ResponseJson(ApiResponse::<LogoutResponse, LogoutResponse>::success(LogoutResponse {
            message: "Logged out successfully".to_string(),
        }))
    ))
}

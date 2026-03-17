// SQLite-based authentication endpoints
use axum::{
    Json as ResponseJson,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Returns true if the server is running in a context where Secure cookies are appropriate
/// (i.e., not localhost HTTP development).
fn is_secure_context() -> bool {
    let host = std::env::var("HOST").unwrap_or_default();
    let env = std::env::var("RUST_ENV").unwrap_or_default();
    if env == "development" {
        return false;
    }
    !matches!(host.as_str(), "localhost" | "127.0.0.1" | "0.0.0.0" | "")
}

/// Load platform roles for a user (public alias for cross-module use)
pub async fn load_platform_roles_pub(pool: &sqlx::SqlitePool, user_id: &str) -> Vec<String> {
    load_platform_roles(pool, user_id).await
}

/// Load platform roles for a user
async fn load_platform_roles(pool: &sqlx::SqlitePool, user_id: &str) -> Vec<String> {
    #[derive(FromRow)]
    struct RoleRow {
        role: String,
    }
    sqlx::query_as::<_, RoleRow>(
        "SELECT role FROM user_platform_roles WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| r.role)
    .collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_active: i32,
    pub is_admin: i32,
    pub home_organization_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
    pub organizations: Vec<UserOrganization>,
    pub platform_roles: Vec<String>,
    pub home_organization_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserOrganization {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub role: String,
}

/// POST /auth/login
/// SQLite version - returns session cookie for authentication
pub async fn login(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<LoginRequest>,
) -> Result<Response, ApiError> {
    // Get SQLite pool from deployment
    let pool = &deployment.db().pool;

    // Find user by username OR email (case-insensitive)
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, email, password_hash, full_name, avatar_url, is_active, is_admin,
                CASE WHEN typeof(home_organization_id) = 'blob' THEN lower(substr(hex(home_organization_id),1,8)||'-'||substr(hex(home_organization_id),9,4)||'-'||substr(hex(home_organization_id),13,4)||'-'||substr(hex(home_organization_id),17,4)||'-'||substr(hex(home_organization_id),21,12)) ELSE home_organization_id END as home_organization_id
         FROM users
         WHERE (username = ? COLLATE NOCASE OR email = ? COLLATE NOCASE) AND is_active = 1",
    )
    .bind(&req.username)
    .bind(&req.username)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("Invalid credentials".to_string()))?;

    // Verify password using bcrypt
    let is_valid =
        db::services::AuthService::verify_password(&req.password, &user.password_hash)
            .map_err(|e| ApiError::InternalError(format!("Password verification error: {}", e)))?;

    if !is_valid {
        return Err(ApiError::BadRequest("Invalid credentials".to_string()));
    }

    // Update last login time
    sqlx::query("UPDATE users SET last_login_at = datetime('now') WHERE id = ?")
        .bind(&user.id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    // Run onboarding for existing users who don't have Orcha yet
    // Check if user has a home_project_id
    let has_home: bool = sqlx::query_scalar::<_, Option<String>>(
        "SELECT home_project_id FROM users WHERE id = ?",
    )
    .bind(&user.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v)
    .is_some();

    if !has_home {
        let user_uuid = Uuid::parse_str(&user.id).unwrap_or_else(|_| Uuid::new_v4());
        if let Err(e) = services::services::user_onboarding::UserOnboardingService::onboard_user(
            pool,
            user_uuid,
            &user.username,
        )
        .await
        {
            tracing::warn!("Login onboarding failed for {}: {}", user.id, e);
        }
    }

    // Generate session ID using secure random UUID
    let session_id = db::services::AuthService::generate_session_id();

    // Hash the session token before storage (SHA256 for fast verification)
    let session_token_hash = db::services::AuthService::hash_session_token(&session_id);

    // Create session in database with hashed token
    // Reduced expiration from 30 days to 7 days for better security
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at, last_used_at)
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(&session_token_hash)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // Get user organizations
    #[derive(FromRow)]
    struct OrgRow {
        id: String,
        name: String,
        slug: String,
        role: String,
    }

    let orgs = sqlx::query_as::<_, OrgRow>(
        "SELECT o.id, o.name, o.slug, om.role
         FROM organizations o
         JOIN organization_members om ON o.id = om.organization_id
         WHERE om.user_id = ? AND o.is_active = 1",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let organizations = orgs
        .into_iter()
        .map(|row| UserOrganization {
            id: row.id.clone(),
            name: row.name,
            slug: row.slug,
            role: row.role,
        })
        .collect();

    let platform_roles = load_platform_roles(pool, &user.id).await;
    let effective_admin = user.is_admin == 1 || platform_roles.iter().any(|r| r == "platform_admin");

    let profile = UserProfile {
        id: user.id.clone(),
        username: user.username,
        email: user.email,
        full_name: user.full_name,
        avatar_url: user.avatar_url,
        is_admin: effective_admin,
        organizations,
        platform_roles,
        home_organization_id: user.home_organization_id,
    };

    let response = LoginResponse {
        user: profile,
        session_id: session_id.clone(),
    };

    let secure_flag = if is_secure_context() { "; Secure" } else { "" };
    let cookie = format!(
        "session_id={}; Path=/; HttpOnly; SameSite=Lax{secure_flag}; Max-Age={}",
        session_id,
        7 * 24 * 60 * 60
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        ResponseJson(ApiResponse::<LoginResponse, LoginResponse>::success(
            response,
        )),
    )
        .into_response())
}

/// GET /auth/me
/// Get current user from session cookie
pub async fn get_current_user(
    State(deployment): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    // Extract session from cookie
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("No session cookie".to_string()))?;

    let session_id = cookie_header
        .split(';')
        .find_map(|part| {
            let part = part.trim();
            part.strip_prefix("session_id=")
        })
        .ok_or_else(|| ApiError::BadRequest("No session ID in cookie".to_string()))?;

    let pool = &deployment.db().pool;

    // Hash the session token to look up in database
    let session_token_hash = db::services::AuthService::hash_session_token(session_id);

    // Find session and check if it's valid
    #[derive(FromRow)]
    struct Session {
        user_id: String,
        expires_at: String,
    }

    let session = sqlx::query_as::<_, Session>(
        "SELECT user_id, expires_at FROM sessions WHERE token_hash = ?",
    )
    .bind(&session_token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("Invalid session".to_string()))?;

    // Check if session expired (handle both RFC 3339 and SQLite datetime formats)
    let expires_at = chrono::DateTime::parse_from_rfc3339(&session.expires_at)
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(&session.expires_at, "%Y-%m-%d %H:%M:%S")
                .map(|naive| naive.and_utc().fixed_offset())
        })
        .map_err(|e| ApiError::InternalError(format!("Invalid expiry date: {}", e)))?;

    if expires_at < chrono::Utc::now() {
        return Err(ApiError::BadRequest("Session expired".to_string()));
    }

    // Get user
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, email, password_hash, full_name, avatar_url, is_active, is_admin,
                home_organization_id
         FROM users WHERE id = ?",
    )
    .bind(&session.user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("User not found".to_string()))?;

    // Get organizations
    #[derive(FromRow)]
    struct OrgRow {
        id: String,
        name: String,
        slug: String,
        role: String,
    }

    let orgs = sqlx::query_as::<_, OrgRow>(
        "SELECT o.id, o.name, o.slug, om.role
         FROM organizations o
         JOIN organization_members om ON o.id = om.organization_id
         WHERE om.user_id = ? AND o.is_active = 1",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let organizations = orgs
        .into_iter()
        .map(|row| UserOrganization {
            id: row.id.clone(),
            name: row.name,
            slug: row.slug,
            role: row.role,
        })
        .collect();

    let platform_roles = load_platform_roles(pool, &user.id).await;
    let effective_admin = user.is_admin == 1 || platform_roles.iter().any(|r| r == "platform_admin");

    let profile = UserProfile {
        id: user.id.clone(),
        username: user.username,
        email: user.email,
        full_name: user.full_name,
        avatar_url: user.avatar_url,
        is_admin: effective_admin,
        organizations,
        platform_roles,
        home_organization_id: user.home_organization_id,
    };

    // Wrap in ApiResponse
    #[derive(Serialize)]
    struct ApiResponse {
        data: UserProfile,
    }

    Ok((StatusCode::OK, ResponseJson(ApiResponse { data: profile })).into_response())
}

/// POST /auth/register
/// Invite-gated public registration — requires a valid invite_token on an org.
pub async fn register(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<RegisterRequest>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;

    // 1. Look up org by invite token
    #[derive(FromRow)]
    struct OrgRow {
        id: String,
        name: String,
        #[allow(dead_code)]
        pending_owner_email: Option<String>,
    }

    let org = sqlx::query_as::<_, OrgRow>(
        "SELECT id, name, pending_owner_email FROM organizations WHERE invite_token = ? AND is_active = 1",
    )
    .bind(&req.invite_token)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("Invalid or expired invite token".into()))?;

    // 2. Check username / email uniqueness
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE username = ? COLLATE NOCASE OR email = ? COLLATE NOCASE",
    )
    .bind(&req.username)
    .bind(req.email.as_deref().unwrap_or(""))
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if existing.unwrap_or(0) > 0 {
        return Err(ApiError::BadRequest("Username or email already taken".into()));
    }

    // 3. Hash password and create user
    let password_hash = db::services::AuthService::hash_password(&req.password)
        .map_err(|e| ApiError::InternalError(format!("Password hashing error: {}", e)))?;

    let user_id = Uuid::new_v4();
    let email = req.email.as_deref().unwrap_or("");

    let user_id_str = user_id.to_string();
    sqlx::query(
        "INSERT INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
         VALUES (?, ?, ?, ?, ?, 0, 1)",
    )
    .bind(&user_id_str)
    .bind(&req.username)
    .bind(email)
    .bind(&req.full_name)
    .bind(&password_hash)
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create user: {}", e)))?;

    // 4. Transfer org ownership to new user
    sqlx::query(
        "UPDATE organizations SET owner_id = ?, invite_token = NULL, pending_owner_email = NULL, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&user_id_str)
    .bind(&org.id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to update org: {}", e)))?;

    // 5. Add new user as org admin member
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role) VALUES (?, ?, ?, 'admin')",
    )
    .bind(&member_id)
    .bind(&org.id)
    .bind(&user_id_str)
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to add org member: {}", e)))?;

    // 6. Bridge persons.user_id for persons whose company_org_id matches
    sqlx::query(
        "UPDATE persons SET user_id = ?, updated_at = datetime('now','subsec') WHERE company_org_id = ? AND user_id IS NULL",
    )
    .bind(&user_id_str)
    .bind(&org.id)
    .execute(pool)
    .await
    .ok(); // non-fatal

    // 7. Run onboarding
    if let Err(e) = services::services::user_onboarding::UserOnboardingService::onboard_user(
        pool,
        user_id,
        &req.username,
    )
    .await
    {
        tracing::warn!("Register onboarding failed for {}: {}", user_id, e);
    }

    // 8. Create session
    let session_id = db::services::AuthService::generate_session_id();
    let session_token_hash = db::services::AuthService::hash_session_token(&session_id);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at, last_used_at)
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user_id_str)
    .bind(&session_token_hash)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // 9. Build response
    let organizations = vec![UserOrganization {
        id: org.id.clone(),
        name: org.name,
        slug: String::new(), // slug not needed in response
        role: "admin".into(),
    }];

    let profile = UserProfile {
        id: user_id.to_string(),
        username: req.username,
        email: email.to_string(),
        full_name: req.full_name,
        avatar_url: None,
        is_admin: false,
        organizations,
        platform_roles: vec![],
        home_organization_id: None,
    };

    let response = LoginResponse {
        user: profile,
        session_id: session_id.clone(),
    };

    let secure_flag = if is_secure_context() { "; Secure" } else { "" };
    let cookie = format!(
        "session_id={}; Path=/; HttpOnly; SameSite=Lax{secure_flag}; Max-Age={}",
        session_id,
        7 * 24 * 60 * 60
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        ResponseJson(ApiResponse::<LoginResponse, LoginResponse>::success(response)),
    )
        .into_response())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub email: Option<String>,
    pub invite_token: String,
}

/// POST /auth/logout
/// Clear session
pub async fn logout(
    State(deployment): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    // Extract session from cookie
    if let Some(cookie_header) = headers.get(header::COOKIE).and_then(|h| h.to_str().ok()) {
        if let Some(session_id) = cookie_header.split(';').find_map(|part| {
            let part = part.trim();
            part.strip_prefix("session_id=")
        }) {
            let pool = &deployment.db().pool;

            // Hash the session token to find and delete in database
            let session_token_hash = db::services::AuthService::hash_session_token(session_id);

            // Delete session from database
            let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
                .bind(&session_token_hash)
                .execute(pool)
                .await;
        }
    }

    // Clear cookie
    let secure_flag = if is_secure_context() { "; Secure" } else { "" };
    let cookie = format!("session_id=; Path=/; HttpOnly; SameSite=Lax{secure_flag}; Max-Age=0");

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        ResponseJson(serde_json::json!({"message": "Logged out"})),
    )
        .into_response())
}

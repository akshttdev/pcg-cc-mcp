// SQLite-based authentication endpoints
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json as ResponseJson,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

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
    #[sqlx(try_from = "Vec<u8>")]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_active: i32,
    pub is_admin: i32,
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

    // Find user by username
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, email, password_hash, full_name, avatar_url, is_active, is_admin 
         FROM users 
         WHERE username = ? AND is_active = 1"
    )
    .bind(&req.username)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("Invalid credentials".to_string()))?;

    // Verify password using bcrypt
    let is_valid = db::services::AuthService::verify_password(&req.password, &user.password_hash)
        .map_err(|e| ApiError::InternalError(format!("Password verification error: {}", e)))?;

    if !is_valid {
        return Err(ApiError::BadRequest("Invalid credentials".to_string()));
    }

    // Update last login time
    sqlx::query("UPDATE users SET last_login_at = datetime('now') WHERE id = ?")
        .bind(user.id.as_bytes().as_slice())
        .execute(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    // Generate session ID (simple UUID for now)
    let session_id = Uuid::new_v4().to_string();

    // Create session in database
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at, last_used_at)
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))"
    )
    .bind(Uuid::new_v4().as_bytes().as_slice())
    .bind(user.id.as_bytes().as_slice())
    .bind(&session_id)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // Get user organizations
    #[derive(FromRow)]
    struct OrgRow {
        #[sqlx(try_from = "Vec<u8>")]
        id: Uuid,
        name: String,
        slug: String,
        role: String,
    }

    let orgs = sqlx::query_as::<_, OrgRow>(
        "SELECT o.id, o.name, o.slug, om.role
         FROM organizations o
         JOIN organization_members om ON o.id = om.organization_id
         WHERE om.user_id = ? AND o.is_active = 1"
    )
    .bind(user.id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let organizations = orgs
        .into_iter()
        .map(|row| UserOrganization {
            id: row.id.to_string(),
            name: row.name,
            slug: row.slug,
            role: row.role,
        })
        .collect();

    let profile = UserProfile {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
        full_name: user.full_name,
        avatar_url: user.avatar_url,
        is_admin: user.is_admin == 1,
        organizations,
    };

    let response = LoginResponse {
        user: profile,
        session_id: session_id.clone(),
    };

    // Wrap in ApiResponse
    #[derive(Serialize)]
    struct ApiResponse {
        data: LoginResponse,
    }

    let api_response = ApiResponse { data: response };

    // Set session cookie
    let cookie = format!(
        "session_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session_id,
        30 * 24 * 60 * 60 // 30 days in seconds
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        ResponseJson(api_response),
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

    // Find session and check if it's valid
    #[derive(FromRow)]
    struct Session {
        #[sqlx(try_from = "Vec<u8>")]
        user_id: Uuid,
        expires_at: String,
    }

    let session = sqlx::query_as::<_, Session>(
        "SELECT user_id, expires_at FROM sessions WHERE token_hash = ?"
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("Invalid session".to_string()))?;

    // Check if session expired
    let expires_at = chrono::DateTime::parse_from_rfc3339(&session.expires_at)
        .map_err(|e| ApiError::InternalError(format!("Invalid expiry date: {}", e)))?;
    
    if expires_at < chrono::Utc::now() {
        return Err(ApiError::BadRequest("Session expired".to_string()));
    }

    // Get user
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, email, password_hash, full_name, avatar_url, is_active, is_admin 
         FROM users WHERE id = ?"
    )
    .bind(session.user_id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("User not found".to_string()))?;

    // Get organizations
    #[derive(FromRow)]
    struct OrgRow {
        #[sqlx(try_from = "Vec<u8>")]
        id: Uuid,
        name: String,
        slug: String,
        role: String,
    }

    let orgs = sqlx::query_as::<_, OrgRow>(
        "SELECT o.id, o.name, o.slug, om.role
         FROM organizations o
         JOIN organization_members om ON o.id = om.organization_id
         WHERE om.user_id = ? AND o.is_active = 1"
    )
    .bind(user.id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let organizations = orgs
        .into_iter()
        .map(|row| UserOrganization {
            id: row.id.to_string(),
            name: row.name,
            slug: row.slug,
            role: row.role,
        })
        .collect();

    let profile = UserProfile {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
        full_name: user.full_name,
        avatar_url: user.avatar_url,
        is_admin: user.is_admin == 1,
        organizations,
    };

    // Wrap in ApiResponse
    #[derive(Serialize)]
    struct ApiResponse {
        data: UserProfile,
    }

    Ok((
        StatusCode::OK,
        ResponseJson(ApiResponse { data: profile }),
    )
        .into_response())
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
            
            // Delete session from database
            let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
                .bind(session_id)
                .execute(pool)
                .await;
        }
    }

    // Clear cookie
    let cookie = "session_id=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        ResponseJson(serde_json::json!({"message": "Logged out"})),
    )
        .into_response())
}

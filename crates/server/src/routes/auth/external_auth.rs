// External authentication for federated identity (e.g., Jungleverse SSO)
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json as ResponseJson,
};
use deployment::Deployment;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::auth_sqlite::{UserOrganization, UserProfile};
use crate::{error::ApiError, DeploymentImpl};

/// JWT claims from external provider (e.g., Jungleverse)
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalClaims {
    pub sub: String,          // External user ID
    pub email: String,        // User email
    pub name: Option<String>, // User display name
    pub provider: String,     // Provider name (e.g., "jungleverse")
    pub iat: i64,             // Issued at
    pub exp: i64,             // Expiration
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateTokenResponse {
    pub user: UserProfile,
    pub session_id: String,
}

#[derive(Debug, FromRow)]
struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    #[allow(dead_code)]
    pub is_active: i32,
    pub is_admin: i32,
    pub home_organization_id: Option<String>,
}

/// POST /auth/external/validate
/// Validates a JWT token from an external provider and creates/returns a PCG session
pub async fn validate_external_token(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<ValidateTokenRequest>,
) -> Result<Response, ApiError> {
    // Get JWT secret from environment
    let jwt_secret = std::env::var("JUNGLEVERSE_JWT_SECRET")
        .map_err(|_| ApiError::InternalError("JWT secret not configured".to_string()))?;

    // Decode and validate JWT
    let token_data = decode::<ExternalClaims>(
        &req.token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|e| ApiError::BadRequest(format!("Invalid token: {}", e)))?;

    let claims = token_data.claims;

    // Validate provider
    if claims.provider != "jungleverse" {
        return Err(ApiError::BadRequest("Unknown provider".to_string()));
    }

    let pool = &deployment.db().pool;

    // Look up user by external provider + external_id
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, username, email, full_name, avatar_url, is_active, is_admin, home_organization_id
         FROM users
         WHERE external_provider = ? AND external_id = ?",
    )
    .bind(&claims.provider)
    .bind(&claims.sub)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let user = match existing_user {
        Some(u) => {
            // Update email/name if changed
            sqlx::query(
                "UPDATE users SET email = ?, full_name = ?, updated_at = datetime('now')
                 WHERE id = ?",
            )
            .bind(&claims.email)
            .bind(claims.name.as_deref().unwrap_or(&claims.email))
            .bind(&u.id)
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
            u
        }
        None => {
            // Create new external user
            let user_id = Uuid::new_v4();
            let user_id_str = user_id.to_string();
            let username = format!("jv_{}", &claims.sub[..8.min(claims.sub.len())]);
            let full_name = claims.name.unwrap_or_else(|| claims.email.clone());

            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, full_name, external_provider, external_id, is_active, is_admin, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 1, 0, datetime('now'), datetime('now'))",
            )
            .bind(&user_id_str)
            .bind(&username)
            .bind(&claims.email)
            .bind("external_auth_only") // Placeholder - cannot login with password
            .bind(&full_name)
            .bind(&claims.provider)
            .bind(&claims.sub)
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to create user: {}", e)))?;

            User {
                id: user_id_str,
                username,
                email: claims.email.clone(),
                full_name,
                avatar_url: None,
                is_active: 1,
                is_admin: 0,
                home_organization_id: None,
            }
        }
    };

    // Generate session ID
    let session_id = Uuid::new_v4().to_string();

    // Create session in database
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at, last_used_at)
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(&session_id)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // Get user organizations (likely empty for external users)
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

    let organizations: Vec<UserOrganization> = orgs
        .into_iter()
        .map(|row| UserOrganization {
            id: row.id.clone(),
            name: row.name,
            slug: row.slug,
            role: row.role,
        })
        .collect();

    let platform_roles = super::auth_sqlite::load_platform_roles_pub(pool, &user.id).await;
    let effective_admin =
        user.is_admin == 1 || platform_roles.iter().any(|r| r == "platform_admin");

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

    let response = ValidateTokenResponse {
        user: profile,
        session_id: session_id.clone(),
    };

    // Set session cookie - SameSite=None for cross-origin iframe
    let cookie = format!(
        "session_id={}; Path=/; HttpOnly; SameSite=None; Secure; Max-Age={}",
        session_id,
        30 * 24 * 60 * 60 // 30 days in seconds
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        ResponseJson(serde_json::json!({
            "data": response
        })),
    )
        .into_response())
}

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectRole {
    Owner,
    Admin,
    Editor,
    Viewer,
}

impl ProjectRole {
    pub fn can_read(&self) -> bool {
        matches!(
            self,
            ProjectRole::Owner | ProjectRole::Admin | ProjectRole::Editor | ProjectRole::Viewer
        )
    }

    pub fn can_write(&self) -> bool {
        matches!(
            self,
            ProjectRole::Owner | ProjectRole::Admin | ProjectRole::Editor
        )
    }

    pub fn can_manage_members(&self) -> bool {
        matches!(self, ProjectRole::Owner | ProjectRole::Admin)
    }

    pub fn can_delete(&self) -> bool {
        matches!(self, ProjectRole::Owner)
    }
}

impl std::fmt::Display for ProjectRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectRole::Owner => write!(f, "owner"),
            ProjectRole::Admin => write!(f, "admin"),
            ProjectRole::Editor => write!(f, "editor"),
            ProjectRole::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for ProjectRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(ProjectRole::Owner),
            "admin" => Ok(ProjectRole::Admin),
            "editor" => Ok(ProjectRole::Editor),
            "viewer" => Ok(ProjectRole::Viewer),
            _ => Err(format!("Invalid project role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ProjectMember {
    pub id: Vec<u8>,
    pub project_id: Vec<u8>,
    pub user_id: Vec<u8>,
    pub role: String,
    pub permissions: String,
    pub granted_by: Option<Vec<u8>>,
    pub granted_at: String,
}

#[derive(Debug, Clone)]
pub struct AccessContext {
    pub user_id: Uuid,
    pub is_admin: bool,
    pub is_active: bool,
}

impl AccessContext {
    /// Check if user has admin access
    pub fn require_admin(&self) -> Result<(), ApiError> {
        if !self.is_admin {
            return Err(ApiError::Forbidden("Admin access required".to_string()));
        }
        if !self.is_active {
            return Err(ApiError::Forbidden("User account is inactive".to_string()));
        }
        Ok(())
    }

    /// Check if user is active
    pub fn require_active(&self) -> Result<(), ApiError> {
        if !self.is_active {
            return Err(ApiError::Forbidden("User account is inactive".to_string()));
        }
        Ok(())
    }

    /// Check if user has access to a specific project
    pub async fn check_project_access(
        &self,
        pool: &sqlx::SqlitePool,
        project_id: &str,
        required_role: ProjectRole,
    ) -> Result<ProjectRole, ApiError> {
        // Admins have full access to all projects
        if self.is_admin {
            return Ok(ProjectRole::Owner);
        }

        // Convert project_id string to UUID bytes for BLOB comparison
        let project_uuid = Uuid::parse_str(project_id)
            .map_err(|e| ApiError::InternalError(format!("Invalid project UUID: {}", e)))?;
        let project_id_bytes = project_uuid.as_bytes().to_vec();

        // Check project membership
        let member: Option<ProjectMember> =
            sqlx::query_as("SELECT * FROM project_members WHERE project_id = ? AND user_id = ?")
                .bind(&project_id_bytes)
                .bind(self.user_id.as_bytes().to_vec())
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

        match member {
            Some(m) => {
                let role = m
                    .role
                    .parse::<ProjectRole>()
                    .map_err(|e| ApiError::InternalError(e))?;

                // Check if user has required permission level
                let has_access = match required_role {
                    ProjectRole::Viewer => role.can_read(),
                    ProjectRole::Editor => role.can_write(),
                    ProjectRole::Admin => role.can_manage_members(),
                    ProjectRole::Owner => role.can_delete(),
                };

                if has_access {
                    Ok(role)
                } else {
                    Err(ApiError::Forbidden(format!(
                        "Insufficient permissions. Required: {:?}, Has: {:?}",
                        required_role, role
                    )))
                }
            }
            None => Err(ApiError::Forbidden(
                "You do not have access to this project".to_string(),
            )),
        }
    }

    /// Get user's role for a project (returns None if no access)
    pub async fn get_project_role(
        &self,
        pool: &sqlx::SqlitePool,
        project_id: &str,
    ) -> Result<Option<ProjectRole>, ApiError> {
        // Admins have owner access to all projects
        if self.is_admin {
            return Ok(Some(ProjectRole::Owner));
        }

        let project_uuid = Uuid::parse_str(project_id)
            .map_err(|e| ApiError::InternalError(format!("Invalid project UUID: {}", e)))?;
        let project_id_bytes = project_uuid.as_bytes().to_vec();

        let member: Option<ProjectMember> =
            sqlx::query_as("SELECT * FROM project_members WHERE project_id = ? AND user_id = ?")
                .bind(&project_id_bytes)
                .bind(self.user_id.as_bytes().to_vec())
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

        match member {
            Some(m) => {
                let role = m
                    .role
                    .parse::<ProjectRole>()
                    .map_err(|e| ApiError::InternalError(e))?;
                Ok(Some(role))
            }
            None => Ok(None),
        }
    }
}

/// Extract user from session token or cookie
pub async fn get_current_user(
    deployment: &DeploymentImpl,
    auth_header: Option<&str>,
    cookie_header: Option<&str>,
) -> Result<AccessContext, ApiError> {
    let pool = deployment.db().pool.clone();

    // Try to get session from cookie first (SQLite auth)
    if let Some(cookies) = cookie_header {
        if let Some(session_id) = extract_session_from_cookies(cookies) {
            // Hash the session token before lookup (sessions are stored as SHA256 hashes)
            let session_token_hash = db::services::AuthService::hash_session_token(&session_id);

            // Find session and join with user
            #[derive(FromRow)]
            struct UserSession {
                id: Vec<u8>,
                is_admin: i32,
                is_active: i32,
            }

            let result: Option<UserSession> = sqlx::query_as(
                r#"
                SELECT u.id, u.is_admin, u.is_active
                FROM sessions s
                JOIN users u ON s.user_id = u.id
                WHERE s.token_hash = ? AND s.expires_at > datetime('now')
                "#,
            )
            .bind(&session_token_hash)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

            if let Some(user_session) = result {
                let user_id = Uuid::from_slice(&user_session.id)
                    .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;

                return Ok(AccessContext {
                    user_id,
                    is_admin: user_session.is_admin == 1,
                    is_active: user_session.is_active == 1,
                });
            }
        }
    }

    // Try Bearer token (for API access)
    if let Some(token) = auth_header.and_then(|h| h.strip_prefix("Bearer ")) {
        // Hash the token using SHA256 (same as session tokens)
        let token_hash = db::services::AuthService::hash_session_token(token);

        // Find session and join with user
        #[derive(FromRow)]
        struct UserSession {
            id: Vec<u8>,
            is_admin: i32,
            is_active: i32,
        }

        let result: Option<UserSession> = sqlx::query_as(
            r#"
            SELECT u.id, u.is_admin, u.is_active
            FROM sessions s
            JOIN users u ON s.user_id = u.id
            WHERE s.token_hash = ? AND s.expires_at > datetime('now')
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

        if let Some(user_session) = result {
            let user_id = Uuid::from_slice(&user_session.id)
                .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;

            return Ok(AccessContext {
                user_id,
                is_admin: user_session.is_admin == 1,
                is_active: user_session.is_active == 1,
            });
        }
    }

    Err(ApiError::Unauthorized(
        "Missing or invalid authentication".to_string(),
    ))
}

/// Extract session ID from cookie header
fn extract_session_from_cookies(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|cookie| {
        let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
        if parts.len() == 2 && parts[0] == "session_id" {
            Some(parts[1].to_string())
        } else {
            None
        }
    })
}

/// Middleware to require authentication
pub async fn require_auth(
    State(deployment): State<DeploymentImpl>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    let cookie_header = req.headers().get("cookie").and_then(|h| h.to_str().ok());

    match get_current_user(&deployment, auth_header, cookie_header).await {
        Ok(context) => {
            req.extensions_mut().insert(context);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Middleware to require admin access
pub async fn require_admin(
    State(deployment): State<DeploymentImpl>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    let cookie_header = req.headers().get("cookie").and_then(|h| h.to_str().ok());

    match get_current_user(&deployment, auth_header, cookie_header).await {
        Ok(context) => {
            if context.require_admin().is_ok() {
                req.extensions_mut().insert(context);
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

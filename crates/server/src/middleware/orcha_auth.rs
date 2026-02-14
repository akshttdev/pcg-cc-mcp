//! ORCHA Authentication Middleware with Federated Routing
//!
//! Extends the standard authentication to include user-to-Topsi routing.
//! When a user authenticates, we determine which device serves their Topsi instance
//! and route all queries accordingly.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sqlx::SqlitePool;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::{AccessContext, get_current_user},
    orcha_routing::{OrchaRouter, TopsiRoute},
};

/// Extended access context with ORCHA routing information
#[derive(Debug, Clone)]
pub struct OrchaAccessContext {
    /// Standard access context
    pub access: AccessContext,

    /// Topsi routing information
    pub topsi_route: TopsiRoute,

    /// Database pool for this user's Topsi instance
    pub topsi_pool: Option<SqlitePool>,
}

impl OrchaAccessContext {
    /// Get the database pool for queries
    /// If topsi_pool is set, use it; otherwise fall back to deployment DB
    pub fn db_pool<'a>(&'a self, deployment: &'a DeploymentImpl) -> &'a SqlitePool {
        self.topsi_pool.as_ref()
            .unwrap_or(&deployment.db().pool)
    }
}

/// Middleware to require ORCHA authentication with routing
pub async fn require_orcha_auth(
    State(deployment): State<DeploymentImpl>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    let cookie_header = req.headers().get("cookie").and_then(|h| h.to_str().ok());

    // First, authenticate the user
    let access_context = match get_current_user(&deployment, auth_header, cookie_header).await {
        Ok(ctx) => ctx,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // Load ORCHA router
    let orcha_config_path = std::env::var("ORCHA_CONFIG")
        .unwrap_or_else(|_| "orcha_config.toml".to_string());

    let router = match OrchaRouter::from_file(&orcha_config_path) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to load ORCHA config: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get username from user_id
    let username = match get_username_from_id(&deployment.db().pool, &access_context.user_id).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to get username: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Route user to their Topsi instance
    let topsi_route = match router.route_user(&username, &deployment.db().pool).await {
        Ok(route) => route,
        Err(e) => {
            tracing::error!("Failed to route user '{}': {}", username, e);
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    tracing::info!(
        "Routing user '{}' to Topsi on device '{}' (fallback: {})",
        username,
        topsi_route.serving_device_id,
        topsi_route.using_fallback
    );

    // For now, we'll use the shared database pool
    // TODO: Connect to per-user Topsi database based on topsi_route.topsi_db_path
    let orcha_context = OrchaAccessContext {
        access: access_context,
        topsi_route,
        topsi_pool: None,  // Will be implemented in phase 2
    };

    req.extensions_mut().insert(orcha_context);
    Ok(next.run(req).await)
}

/// Get username from user ID
async fn get_username_from_id(pool: &SqlitePool, user_id: &uuid::Uuid) -> Result<String, ApiError> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT username FROM users WHERE id = ?"
    )
    .bind(user_id.as_bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    result
        .map(|(username,)| username)
        .ok_or_else(|| ApiError::InternalError("User not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
    // For now, testing is done via the ORCHA router tests
}

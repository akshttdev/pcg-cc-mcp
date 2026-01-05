//! Social Account Management Routes
//!
//! Handles OAuth connections, account management, and platform integrations.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, delete, patch},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::social_account::{SocialAccount, UpdateSocialAccount};

#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    pub project_id: Option<Uuid>,
    pub platform: Option<String>,
    pub active_only: Option<bool>,
}

/// GET /social/accounts - List social accounts
async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<ApiResponse<Vec<SocialAccount>>>, ApiError> {
    let pool = &deployment.db().pool;

    let accounts = if let Some(project_id) = query.project_id {
        SocialAccount::find_by_project(pool, project_id).await?
    } else {
        SocialAccount::find_active(pool).await?
    };

    // Filter by platform if specified
    let accounts = if let Some(platform) = query.platform {
        accounts
            .into_iter()
            .filter(|a| a.platform == platform)
            .collect()
    } else {
        accounts
    };

    Ok(Json(ApiResponse::success(accounts)))
}

/// GET /social/accounts/:id - Get single account
async fn get_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SocialAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = SocialAccount::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// PATCH /social/accounts/:id - Update account
async fn update_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateSocialAccount>,
) -> Result<Json<ApiResponse<SocialAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let account = SocialAccount::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// DELETE /social/accounts/:id - Disconnect account
async fn delete_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    SocialAccount::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/accounts", get(list_accounts))
        .route("/social/accounts/{id}", get(get_account))
        .route("/social/accounts/{id}", patch(update_account))
        .route("/social/accounts/{id}", delete(delete_account))
}

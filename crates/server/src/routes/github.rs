//! GitHub integration HTTP routes.
//!
//! Mirrors `storage.rs`: OAuth begin/callback + entity CRUD. Tokens are
//! persisted through the unified `integration_connections` table (provider =
//! `"github"`). Identity of the linked repo lives on `github_repo_links` and
//! is keyed by `(project_id, github_repo_id)`.
//!
//! HTTP surface (under `/api`):
//! - `GET    /github/connect`             Start OAuth — redirects to GitHub
//! - `GET    /github/callback`            OAuth callback — exchange code, store tokens
//! - `GET    /github/repositories`        List repos visible to the connected user
//! - `POST   /github/links`               Link a GitHub repo to a project
//! - `GET    /github/links`               List links (by project or org)
//! - `DELETE /github/links/{id}`          Remove a link
//! - `GET    /github/links/{id}/commits`  Sync commits for the linked repo

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::{delete, get},
    Extension, Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use db::models::{
    github_repo_link::{CreateGitHubRepoLink, GitHubRepoLink},
    integration_connection::{IntegrationConnection, OAuthPendingState},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    github_service::{self, AuthenticatedUser, CommitSummary, GitHubService, RepositoryInfo},
    oauth_token_manager::{self, PlainTokens, StoreTokensInput},
};
use tracing::{info, warn};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

const PROVIDER: &str = "github";

// ─── Configuration ─────────────────────────────────────────────────────────

fn redirect_base() -> String {
    std::env::var("OAUTH_REDIRECT_BASE").unwrap_or_else(|_| "http://localhost:3002/api".into())
}

fn callback_redirect_uri() -> String {
    format!("{}/github/callback", redirect_base())
}

fn frontend_settings_url() -> String {
    std::env::var("GITHUB_OAUTH_SUCCESS_REDIRECT")
        .unwrap_or_else(|_| "/settings/integrations".into())
}

fn generate_state_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

async fn github_service_for_org(
    pool: &sqlx::SqlitePool,
    organization_id: Uuid,
) -> Result<(IntegrationConnection, GitHubService), ApiError> {
    let conns =
        IntegrationConnection::find_by_org_and_provider(pool, organization_id, PROVIDER).await?;
    let conn = conns
        .into_iter()
        .find(|c| c.status == "active")
        .ok_or_else(|| {
            ApiError::BadRequest(
                "No active GitHub connection for this organization. Connect first.".into(),
            )
        })?;
    let token = oauth_token_manager::get_access_token(pool, conn.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load GitHub token: {e}")))?;
    let service = GitHubService::new(&token)
        .map_err(|e| ApiError::InternalError(format!("GitHub service init failed: {e}")))?;
    Ok((conn, service))
}

// ─── Request / response shapes ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
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
pub struct ListReposQuery {
    pub organization_id: Uuid,
    pub page: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkBody {
    pub organization_id: Uuid,
    pub project_id: Uuid,
    pub github_repo_id: i64,
    pub owner: String,
    pub repo_name: String,
    pub full_name: String,
    pub default_branch: Option<String>,
    pub clone_url: Option<String>,
    pub ssh_url: Option<String>,
    pub private: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListLinksQuery {
    pub project_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CommitsQuery {
    pub since: Option<DateTime<Utc>>,
    pub branch: Option<String>,
    pub page: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionSummary {
    pub id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub status: String,
    pub scopes: Option<String>,
}

impl From<&IntegrationConnection> for ConnectionSummary {
    fn from(c: &IntegrationConnection) -> Self {
        Self {
            id: c.id,
            provider: c.provider.clone(),
            provider_account_id: c.provider_account_id.clone(),
            display_name: c.display_name.clone(),
            status: c.status.clone(),
            scopes: c.scopes.clone(),
        }
    }
}

// ─── Handlers ──────────────────────────────────────────────────────────────

/// `GET /github/connect?organization_id=...&project_id=...`
async fn connect(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ConnectQuery>,
) -> Result<Redirect, ApiError> {
    let pool = &deployment.db().pool;

    access_context
        .require_org_membership(pool, &query.organization_id.to_string())
        .await?;

    let state_token = generate_state_token();
    OAuthPendingState::create(
        pool,
        &state_token,
        PROVIDER,
        query.organization_id,
        query.project_id,
        query.redirect_after.as_deref(),
        None,
        Duration::minutes(10),
    )
    .await?;

    let auth_url = github_service::oauth_authorize_url(&callback_redirect_uri(), &state_token)
        .map_err(|e| ApiError::InternalError(format!("Failed to build auth URL: {e}")))?;

    Ok(Redirect::temporary(&auth_url))
}

/// `GET /github/callback?code=...&state=...`
async fn callback(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, ApiError> {
    let pool = &deployment.db().pool;

    if let Some(err) = query.error {
        warn!(
            "GitHub OAuth returned error: {} ({})",
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

    if pending.provider != PROVIDER {
        return Err(ApiError::BadRequest(
            "State token does not match callback provider".into(),
        ));
    }

    access_context
        .require_org_membership(pool, &pending.organization_id.to_string())
        .await?;

    let tokens = github_service::oauth_exchange_code(&code, &callback_redirect_uri())
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {e}")))?;

    let service = GitHubService::new(&tokens.access_token)
        .map_err(|e| ApiError::InternalError(format!("GitHub service init failed: {e}")))?;
    let me: AuthenticatedUser = service
        .get_authenticated_user()
        .await
        .map_err(|e| ApiError::InternalError(format!("Account info fetch failed: {e}")))?;

    oauth_token_manager::store_tokens(
        pool,
        StoreTokensInput {
            organization_id: pending.organization_id,
            project_id: pending.project_id,
            provider: PROVIDER.to_string(),
            provider_account_id: me.id.to_string(),
            display_name: me.name.or(Some(me.login.clone())),
            avatar_url: me.avatar_url.clone(),
            tokens: PlainTokens {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_at: tokens.expires_at,
                scopes: tokens.scope,
            },
            metadata: Some(serde_json::json!({ "login": me.login, "email": me.email }).to_string()),
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to persist tokens: {e}")))?;

    let target = pending.redirect_after.unwrap_or_else(frontend_settings_url);
    Ok(Redirect::temporary(&format!(
        "{}?provider={}&status=connected",
        target, PROVIDER
    )))
}

/// `GET /github/repositories?organization_id=...&page=1`
async fn list_repositories(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListReposQuery>,
) -> Result<Json<ApiResponse<Vec<RepositoryInfo>>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, &query.organization_id.to_string())
        .await?;

    let (_, service) = github_service_for_org(pool, query.organization_id).await?;
    let repositories = service
        .list_repositories(query.page.unwrap_or(1))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list repositories: {e}")))?;

    Ok(Json(ApiResponse::success(repositories)))
}

/// `POST /github/links`
async fn create_link(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateLinkBody>,
) -> Result<Json<ApiResponse<GitHubRepoLink>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, &body.organization_id.to_string())
        .await?;
    access_context
        .require_editor(pool, &body.project_id.to_string())
        .await?;

    // Carry over the active integration connection so the link survives a
    // disconnect+reconnect cycle without losing audit lineage.
    let connection_id =
        IntegrationConnection::find_by_org_and_provider(pool, body.organization_id, PROVIDER)
            .await?
            .into_iter()
            .find(|c| c.status == "active")
            .map(|c| c.id);

    let link = GitHubRepoLink::upsert(
        pool,
        CreateGitHubRepoLink {
            organization_id: body.organization_id,
            project_id: body.project_id,
            integration_connection_id: connection_id,
            github_repo_id: body.github_repo_id,
            owner: body.owner,
            repo_name: body.repo_name,
            full_name: body.full_name,
            default_branch: body.default_branch,
            clone_url: body.clone_url,
            ssh_url: body.ssh_url,
            private: body.private,
            metadata: None,
        },
    )
    .await?;

    info!(
        "Linked GitHub repo {} to project {}",
        link.full_name, link.project_id
    );
    Ok(Json(ApiResponse::success(link)))
}

/// `GET /github/links?project_id=...` or `?organization_id=...`
async fn list_links(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListLinksQuery>,
) -> Result<Json<ApiResponse<Vec<GitHubRepoLink>>>, ApiError> {
    let pool = &deployment.db().pool;

    let links = match (query.project_id, query.organization_id) {
        (Some(project_id), _) => {
            access_context
                .require_viewer(pool, &project_id.to_string())
                .await?;
            GitHubRepoLink::find_by_project(pool, project_id).await?
        }
        (None, Some(org_id)) => {
            access_context
                .require_org_membership(pool, &org_id.to_string())
                .await?;
            GitHubRepoLink::find_by_org(pool, org_id).await?
        }
        _ => {
            return Err(ApiError::BadRequest(
                "Provide project_id or organization_id".into(),
            ));
        }
    };

    Ok(Json(ApiResponse::success(links)))
}

/// `DELETE /github/links/{id}`
async fn delete_link(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let pool = &deployment.db().pool;
    let link = GitHubRepoLink::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("GitHub repo link not found".into()))?;
    access_context
        .require_editor(pool, &link.project_id.to_string())
        .await?;

    let deleted = GitHubRepoLink::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(deleted)))
}

/// `GET /github/links/{id}/commits?since=...&branch=...&page=1`
async fn list_commits(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Query(query): Query<CommitsQuery>,
) -> Result<Json<ApiResponse<Vec<CommitSummary>>>, ApiError> {
    let pool = &deployment.db().pool;
    let link = GitHubRepoLink::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("GitHub repo link not found".into()))?;
    access_context
        .require_viewer(pool, &link.project_id.to_string())
        .await?;

    let (_, service) = github_service_for_org(pool, link.organization_id).await?;
    let branch = query
        .branch
        .as_deref()
        .or(Some(link.default_branch.as_str()));
    let commits = service
        .list_commits(
            &link.owner,
            &link.repo_name,
            branch,
            query.since,
            query.page.unwrap_or(1),
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list commits: {e}")))?;

    let latest_sha = commits.first().map(|c| c.sha.clone());
    if let Err(e) = GitHubRepoLink::touch_sync(pool, link.id, latest_sha.as_deref()).await {
        warn!("Failed to update last_sync_at for {}: {}", link.id, e);
    }

    Ok(Json(ApiResponse::success(commits)))
}

// ─── Router ────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/github/connect", get(connect))
        .route("/github/callback", get(callback))
        .route("/github/repositories", get(list_repositories))
        .route("/github/links", get(list_links).post(create_link))
        .route("/github/links/{id}", delete(delete_link))
        .route("/github/links/{id}/commits", get(list_commits))
        .with_state(deployment.clone())
}

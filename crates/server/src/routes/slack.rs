//! Slack integration routes.
//!
//! - `GET  /slack/connect?organization_id=...` — start workspace OAuth.
//! - `GET  /slack/oauth/callback`              — code → token, store as
//!                                                `integration_connections` row.
//! - `GET  /slack/channels?organization_id=...` — pull live channel list.
//! - `GET  /slack/routes?organization_id=...`   — list configured routes.
//! - `PUT  /slack/routes`                       — upsert a route.
//! - `DELETE /slack/routes/:id`                 — delete a route.
//! - `POST /slack/test`                         — send a one-off test message.

use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Redirect,
    routing::{delete, get, post, put},
};
use db::{
    db_uuid::DbUuid,
    models::{
        integration_connection::IntegrationConnection,
        slack_channel_route::{
            SlackChannelRoute, SlackEventType, SlackRouteError, UpsertSlackRoute,
        },
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    oauth_token_manager::{self, PlainTokens, StoreTokensInput},
    slack::{
        client::{SlackChannel, SlackClient, exchange_oauth_code},
        dispatcher::dispatch_test,
    },
};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, helpers::uuid_params::parse_db_uuid_param};

const PROVIDER_SLACK: &str = "slack";
const SLACK_AUTH_URL: &str = "https://slack.com/oauth/v2/authorize";
const SLACK_DEFAULT_SCOPES: &str = "chat:write,channels:read,groups:read";

// ─── Query / response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertRouteRequest {
    pub organization_id: Uuid,
    pub event_type: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TestMessageRequest {
    pub organization_id: Uuid,
    pub channel_id: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SlackConnectionDto {
    pub id: String,
    pub team_name: Option<String>,
    pub display_name: Option<String>,
    pub status: String,
    pub last_sync_at: Option<String>,
}

// ─── Env helpers ────────────────────────────────────────────────────────────

fn redirect_base() -> String {
    std::env::var("OAUTH_REDIRECT_BASE").unwrap_or_else(|_| "http://localhost:3000/api".to_string())
}
fn callback_url() -> String {
    format!("{}/slack/oauth/callback", redirect_base())
}
fn slack_credentials() -> Result<(String, String), ApiError> {
    let id = std::env::var("SLACK_CLIENT_ID")
        .map_err(|_| ApiError::InternalError("SLACK_CLIENT_ID not configured".into()))?;
    let secret = std::env::var("SLACK_CLIENT_SECRET")
        .map_err(|_| ApiError::InternalError("SLACK_CLIENT_SECRET not configured".into()))?;
    Ok((id, secret))
}

// ─── Connect / callback ─────────────────────────────────────────────────────

async fn connect(Query(q): Query<OrgQuery>) -> Result<Redirect, ApiError> {
    let (client_id, _) = slack_credentials()?;
    let scopes = std::env::var("SLACK_BOT_SCOPES").unwrap_or_else(|_| SLACK_DEFAULT_SCOPES.into());
    let url = format!(
        "{SLACK_AUTH_URL}?client_id={}&scope={}&redirect_uri={}&state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&scopes),
        urlencoding::encode(&callback_url()),
        urlencoding::encode(&q.organization_id.to_string()),
    );
    Ok(Redirect::temporary(&url))
}

async fn oauth_callback(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<CallbackQuery>,
) -> Result<Redirect, ApiError> {
    if let Some(err) = q.error {
        return Ok(Redirect::temporary(&format!(
            "/settings/integrations?slack_error={}",
            urlencoding::encode(&err)
        )));
    }
    let code = q
        .code
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".into()))?;
    let state = q
        .state
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".into()))?;
    let organization_id = Uuid::parse_str(&state)
        .map_err(|_| ApiError::BadRequest("Invalid state parameter".into()))?;

    let (client_id, client_secret) = slack_credentials()?;
    let exchanged = exchange_oauth_code(&client_id, &client_secret, &code, &callback_url())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let access_token = exchanged
        .access_token
        .ok_or_else(|| ApiError::InternalError("Slack returned no access token".into()))?;
    let team = exchanged.team.as_ref();
    let team_id = team
        .map(|t| t.id.clone())
        .unwrap_or_else(|| format!("org-{organization_id}"));
    let team_name = team.and_then(|t| t.name.clone());

    let pool = &deployment.db().pool;
    oauth_token_manager::store_tokens(
        pool,
        StoreTokensInput {
            organization_id,
            project_id: None,
            provider: PROVIDER_SLACK.into(),
            provider_account_id: team_id,
            display_name: team_name,
            avatar_url: None,
            tokens: PlainTokens {
                access_token,
                // Slack bot tokens don't have a refresh token by default —
                // only "token rotation"-enabled apps issue one.
                refresh_token: None,
                expires_at: None,
                scopes: exchanged.scope,
            },
            metadata: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Redirect::temporary(
        "/settings/integrations?slack_connected=true",
    ))
}

// ─── Workspaces / channels ──────────────────────────────────────────────────

async fn list_workspaces(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<OrgQuery>,
) -> Result<Json<ApiResponse<Vec<SlackConnectionDto>>>, ApiError> {
    let pool = &deployment.db().pool;
    let conns =
        IntegrationConnection::find_by_org_and_provider(pool, q.organization_id, PROVIDER_SLACK)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let dtos = conns
        .into_iter()
        .map(|c| SlackConnectionDto {
            id: c.id.to_string(),
            team_name: c.display_name.clone(),
            display_name: c.display_name,
            status: c.status,
            last_sync_at: c.last_sync_at.map(|t| t.to_rfc3339()),
        })
        .collect();
    Ok(Json(ApiResponse::success(dtos)))
}

async fn list_channels(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<OrgQuery>,
) -> Result<Json<ApiResponse<Vec<SlackChannel>>>, ApiError> {
    let pool = &deployment.db().pool;
    let mut conns =
        IntegrationConnection::find_by_org_and_provider(pool, q.organization_id, PROVIDER_SLACK)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let conn = conns
        .pop()
        .ok_or_else(|| ApiError::BadRequest("No Slack workspace connected for this org".into()))?;
    let token = oauth_token_manager::get_access_token(pool, conn.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let channels = SlackClient::new(token)
        .list_channels()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(channels)))
}

// ─── Routes table ───────────────────────────────────────────────────────────

async fn list_routes(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<OrgQuery>,
) -> Result<Json<ApiResponse<Vec<SlackChannelRoute>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_uuid = DbUuid::from_string(q.organization_id.to_string());
    let routes = SlackChannelRoute::list_for_org(pool, &org_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(routes)))
}

async fn upsert_route(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<UpsertRouteRequest>,
) -> Result<Json<ApiResponse<SlackChannelRoute>>, ApiError> {
    let pool = &deployment.db().pool;
    let event_type = SlackEventType::from_str(&req.event_type)
        .map_err(|e| ApiError::BadRequest(format!("invalid event_type: {e}")))?;
    let org_uuid = DbUuid::from_string(req.organization_id.to_string());
    let row = SlackChannelRoute::upsert(
        pool,
        UpsertSlackRoute {
            organization_id: org_uuid,
            event_type,
            channel_id: req.channel_id,
            channel_name: req.channel_name,
            enabled: req.enabled,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(row)))
}

async fn delete_route(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = parse_db_uuid_param(&id, "slack route id")?;
    SlackChannelRoute::delete(pool, &id_uuid)
        .await
        .map_err(|e| match e {
            SlackRouteError::NotFound => ApiError::NotFound("slack route not found".into()),
            other => ApiError::InternalError(other.to_string()),
        })?;
    Ok(Json(ApiResponse::success(())))
}

// ─── Test send ──────────────────────────────────────────────────────────────

async fn test_send(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<TestMessageRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_uuid = DbUuid::from_string(req.organization_id.to_string());
    let text = req
        .text
        .unwrap_or_else(|| "👋 Test message from ORCHA".to_string());
    let ts = dispatch_test(pool, &org_uuid, &req.channel_id, &text)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "channel_id": req.channel_id,
        "ts": ts,
    }))))
}

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/slack/connect", get(connect))
        .route("/slack/oauth/callback", get(oauth_callback))
        .route("/slack/workspaces", get(list_workspaces))
        .route("/slack/channels", get(list_channels))
        .route("/slack/routes", get(list_routes).put(upsert_route))
        .route("/slack/routes/{id}", delete(delete_route))
        .route("/slack/test", post(test_send))
}

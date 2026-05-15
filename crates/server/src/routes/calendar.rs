//! Calendar integration routes.
//!
//! - `GET  /calendar/connect/google?organization_id=...` — kick off Google OAuth.
//!   Hands back to `GET /calendar/oauth/callback` once the user consents.
//! - `GET  /calendar/connect/outlook?organization_id=...` — same for MS Graph.
//! - `GET  /calendar/oauth/callback` — receives `?code&state`, exchanges for
//!   tokens, persists via `oauth_token_manager`.
//! - `GET  /calendar/accounts?organization_id=...` — list connected calendars.
//! - `DELETE /calendar/accounts/:id` — disconnect (revokes via token manager).
//! - `POST /calendar/accounts/:id/sync` — trigger a sync now, returns stats.
//! - `GET  /calendar/events?organization_id=...&from=&to=&limit=` — list
//!   normalized events (works without any provider connection — empty list).
//!
//! Provider creds come from existing env vars (`GOOGLE_CLIENT_ID/SECRET`,
//! `MICROSOFT_CLIENT_ID/SECRET`); missing creds surface as 503-style errors.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Redirect,
    routing::{delete, get, post},
};
use chrono::{Duration, Utc};
use db::{
    db_uuid::DbUuid,
    models::{calendar_event::CalendarEvent, integration_connection::IntegrationConnection},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    calendar::{CalendarSyncStats, sync_account},
    oauth_token_manager::{self, PlainTokens, StoreTokensInput},
};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, helpers::uuid_params::parse_db_uuid_param};

const PROVIDER_GOOGLE: &str = "google_calendar";
const PROVIDER_OUTLOOK: &str = "outlook_calendar";

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_DEFAULT_SCOPES: &str = "https://www.googleapis.com/auth/calendar.readonly \
     https://www.googleapis.com/auth/userinfo.email \
     https://www.googleapis.com/auth/userinfo.profile \
     openid";

const MICROSOFT_AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MICROSOFT_DEFAULT_SCOPES: &str = "Calendars.Read offline_access User.Read openid";

// ─── Query types ────────────────────────────────────────────────────────────

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
pub struct ListEventsQuery {
    pub organization_id: Uuid,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CalendarConnectionDto {
    pub id: String,
    pub provider: String,
    pub display_name: Option<String>,
    pub status: String,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
}

// ─── OAuth: connect ─────────────────────────────────────────────────────────

fn redirect_base() -> String {
    std::env::var("OAUTH_REDIRECT_BASE").unwrap_or_else(|_| "http://localhost:3000/api".to_string())
}

fn callback_url() -> String {
    format!("{}/calendar/oauth/callback", redirect_base())
}

fn google_credentials() -> Result<(String, String), ApiError> {
    let id = std::env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| ApiError::InternalError("GOOGLE_CLIENT_ID not configured".into()))?;
    let secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .map_err(|_| ApiError::InternalError("GOOGLE_CLIENT_SECRET not configured".into()))?;
    Ok((id, secret))
}

fn microsoft_credentials() -> Result<(String, String), ApiError> {
    let id = std::env::var("MICROSOFT_CLIENT_ID")
        .map_err(|_| ApiError::InternalError("MICROSOFT_CLIENT_ID not configured".into()))?;
    let secret = std::env::var("MICROSOFT_CLIENT_SECRET")
        .map_err(|_| ApiError::InternalError("MICROSOFT_CLIENT_SECRET not configured".into()))?;
    Ok((id, secret))
}

/// State payload encoded into the `state=` query parameter so the callback
/// knows which org + provider this exchange is for.
fn make_state(provider: &str, organization_id: Uuid) -> String {
    format!("{provider}:{organization_id}")
}

fn parse_state(state: &str) -> Option<(&str, Uuid)> {
    let (prov, rest) = state.split_once(':')?;
    let org = Uuid::parse_str(rest).ok()?;
    Some((prov, org))
}

async fn connect_google(Query(q): Query<OrgQuery>) -> Result<Redirect, ApiError> {
    let (client_id, _) = google_credentials()?;
    let scopes =
        std::env::var("GOOGLE_CALENDAR_SCOPES").unwrap_or_else(|_| GOOGLE_DEFAULT_SCOPES.into());
    let url = format!(
        "{GOOGLE_AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&callback_url()),
        urlencoding::encode(&scopes),
        urlencoding::encode(&make_state(PROVIDER_GOOGLE, q.organization_id)),
    );
    Ok(Redirect::temporary(&url))
}

async fn connect_outlook(Query(q): Query<OrgQuery>) -> Result<Redirect, ApiError> {
    let (client_id, _) = microsoft_credentials()?;
    let scopes = std::env::var("MICROSOFT_CALENDAR_SCOPES")
        .unwrap_or_else(|_| MICROSOFT_DEFAULT_SCOPES.into());
    let url = format!(
        "{MICROSOFT_AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&response_mode=query&state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&callback_url()),
        urlencoding::encode(&scopes),
        urlencoding::encode(&make_state(PROVIDER_OUTLOOK, q.organization_id)),
    );
    Ok(Redirect::temporary(&url))
}

// ─── OAuth: callback ────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct TokenExchangeResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

async fn oauth_callback(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<CallbackQuery>,
) -> Result<Redirect, ApiError> {
    if let Some(err) = q.error {
        return Ok(Redirect::temporary(&format!(
            "/settings/integrations?calendar_error={}",
            urlencoding::encode(&err)
        )));
    }
    let code = q
        .code
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".into()))?;
    let state = q
        .state
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".into()))?;
    let (provider, organization_id) = parse_state(&state)
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter".into()))?;

    let pool = &deployment.db().pool;
    let (token_url, client_id, client_secret) = match provider {
        PROVIDER_GOOGLE => {
            let (id, secret) = google_credentials()?;
            (GOOGLE_TOKEN_URL, id, secret)
        }
        PROVIDER_OUTLOOK => {
            let (id, secret) = microsoft_credentials()?;
            (MICROSOFT_TOKEN_URL, id, secret)
        }
        other => {
            return Err(ApiError::BadRequest(format!(
                "Unknown calendar provider: {other}"
            )));
        }
    };

    let resp = reqwest::Client::new()
        .post(token_url)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", callback_url().as_str()),
        ])
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Token exchange failed: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::error!(provider = %provider, body = %body, "calendar token exchange failed");
        return Ok(Redirect::temporary(
            "/settings/integrations?calendar_error=token_exchange_failed",
        ));
    }
    let tokens: TokenExchangeResponse = resp
        .json()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse token response: {e}")))?;

    let expires_at = tokens.expires_in.map(|s| Utc::now() + Duration::seconds(s));
    // We don't have a stable provider account id from these scopes without
    // an extra userinfo call; use the org+provider tuple as a stable key.
    let provider_account_id = format!("org-{organization_id}");

    oauth_token_manager::store_tokens(
        pool,
        StoreTokensInput {
            organization_id,
            project_id: None,
            provider: provider.to_string(),
            provider_account_id,
            display_name: Some(provider.replace('_', " ")),
            avatar_url: None,
            tokens: PlainTokens {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_at,
                scopes: None,
            },
            metadata: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Redirect::temporary(
        "/settings/integrations?calendar_connected=true",
    ))
}

// ─── List / disconnect / sync / events ──────────────────────────────────────

async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<OrgQuery>,
) -> Result<Json<ApiResponse<Vec<CalendarConnectionDto>>>, ApiError> {
    let pool = &deployment.db().pool;
    let google =
        IntegrationConnection::find_by_org_and_provider(pool, q.organization_id, PROVIDER_GOOGLE)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let outlook =
        IntegrationConnection::find_by_org_and_provider(pool, q.organization_id, PROVIDER_OUTLOOK)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let dtos: Vec<CalendarConnectionDto> = google
        .into_iter()
        .chain(outlook)
        .map(|c| CalendarConnectionDto {
            id: c.id.to_string(),
            provider: c.provider,
            display_name: c.display_name,
            status: c.status,
            last_sync_at: c.last_sync_at.map(|t| t.to_rfc3339()),
            last_error: c.last_error,
        })
        .collect();

    Ok(Json(ApiResponse::success(dtos)))
}

async fn disconnect_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id_uuid = parse_db_uuid_param(&id, "calendar account id")?.to_uuid();
    let pool = &deployment.db().pool;
    let _ = oauth_token_manager::revoke(pool, id_uuid).await;
    IntegrationConnection::delete(pool, id_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

async fn trigger_sync(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CalendarSyncStats>>, ApiError> {
    let id_uuid = parse_db_uuid_param(&id, "calendar account id")?.to_uuid();
    let stats = sync_account(&deployment.db().pool, id_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(stats)))
}

async fn list_events(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<ListEventsQuery>,
) -> Result<Json<ApiResponse<Vec<CalendarEvent>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_id = DbUuid::from_string(q.organization_id.to_string());
    let limit = q.limit.unwrap_or(100).min(500);
    let events =
        CalendarEvent::list_for_org(pool, &org_id, q.from.as_deref(), q.to.as_deref(), limit)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(events)))
}

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/calendar/connect/google", get(connect_google))
        .route("/calendar/connect/outlook", get(connect_outlook))
        .route("/calendar/oauth/callback", get(oauth_callback))
        .route("/calendar/accounts", get(list_accounts))
        .route("/calendar/accounts/{id}", delete(disconnect_account))
        .route("/calendar/accounts/{id}/sync", post(trigger_sync))
        .route("/calendar/events", get(list_events))
}

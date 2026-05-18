//! Discord voice bot REST API
//!
//! Provides dashboard control over the Discord bot and SSE streaming
//! of live voice transcripts.

use std::{convert::Infallible, pin::Pin, time::Duration};

use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use db::models::meeting_session::MeetingSession;
use deployment::Deployment;
use discord_bot::{active_sessions, session_key, subscribe_transcript};
use futures_util::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

// ─── Response types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordSessionSummary {
    pub meeting_session_id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub channel_name: String,
    pub project_id: String,
    pub agent: String,
    pub started_at: String,
    pub elapsed_seconds: i64,
    pub segment_count: i32,
    pub participant_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub project_id: Option<String>,
    pub agent: Option<String>, // "nora" | "topsi" (default: "nora")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaveRequest {
    pub guild_id: String,
    pub agent: Option<String>, // "nora" | "topsi" | "all" (default: "all")
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ─── Route builder ───────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/discord/sessions", get(list_active_sessions))
        .route("/discord/join", post(join_channel))
        .route("/discord/leave", post(leave_channel))
        .route("/discord/archive", get(list_archived_sessions))
        .route("/discord/sessions/{id}", get(get_session))
        .route("/discord/sessions/{id}/transcript", get(get_transcript))
        .route("/discord/sessions/{id}/stream", get(stream_transcript))
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /discord/sessions — list currently active Discord voice sessions
async fn list_active_sessions() -> Json<ApiResponse<Vec<DiscordSessionSummary>>> {
    let sessions: Vec<DiscordSessionSummary> = active_sessions()
        .into_iter()
        .map(|s| DiscordSessionSummary {
            meeting_session_id: s.meeting_session_id.clone(),
            guild_id: s.guild_id.to_string(),
            channel_id: s.channel_id.to_string(),
            channel_name: s.channel_name.clone(),
            project_id: s.project_id.clone(),
            agent: s.agent.name().to_string(),
            started_at: s.started_at.to_rfc3339(),
            elapsed_seconds: s.elapsed_ms() / 1000,
            segment_count: s.segment_count,
            participant_count: s.user_names.len(),
        })
        .collect();

    Json(ApiResponse::success(sessions))
}

/// POST /discord/join — inform the dashboard about the correct slash commands to use.
/// Full programmatic join from the REST API would require a command channel into the
/// running bot task, which is not yet wired (bots join via /nora-join, /topsi-join).
async fn join_channel(
    State(_deployment): State<DeploymentImpl>,
    Json(body): Json<JoinRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let guild_id: u64 = body
        .guild_id
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid guild_id".into()))?;
    let _channel_id: u64 = body
        .channel_id
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid channel_id".into()))?;

    let agent = match body.agent.as_deref().unwrap_or("nora") {
        "topsi" => "topsi",
        _ => "nora",
    };

    // Check if there's already an active session for this guild+agent
    let key = session_key(guild_id, agent);
    if discord_bot::DISCORD_SESSIONS.contains_key(&key) {
        return Err(ApiError::BadRequest(format!(
            "{} is already in a voice session for this guild.",
            agent
        )));
    }

    Ok(Json(serde_json::json!({
        "status": "use_slash_command",
        "message": format!(
            "To have the bot join, use the Discord slash command: /{}-join #{}",
            agent,
            body.channel_name.as_deref().unwrap_or("channel")
        ),
        "guild_id": body.guild_id,
        "channel_id": body.channel_id,
        "agent": agent,
        "tip": "Bots join via Discord slash commands (/nora-join, /topsi-join).",
    })))
}

/// POST /discord/leave — end a guild's active voice session
async fn leave_channel(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<LeaveRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let guild_id: u64 = body
        .guild_id
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid guild_id".into()))?;

    let agent = body.agent.as_deref().unwrap_or("all");

    let agents_to_try: Vec<&str> = if agent == "all" {
        vec!["Nora", "Topsi"]
    } else if agent == "topsi" {
        vec!["Topsi"]
    } else {
        vec!["Nora"]
    };

    let mut ended = vec![];

    for a in agents_to_try {
        let key = session_key(guild_id, a);
        if let Some((_, session)) = discord_bot::DISCORD_SESSIONS.remove(&key) {
            let mid = session.meeting_session_id.clone();
            let _ = sqlx::query(
                r#"UPDATE meeting_sessions SET
                    status = 'ended',
                    ended_at = datetime('now', 'subsec'),
                    updated_at = datetime('now', 'subsec')
                   WHERE id = ?1"#,
            )
            .bind(&mid)
            .execute(&deployment.db().pool)
            .await;

            discord_bot::TRANSCRIPT_CHANNELS.remove(&mid);
            info!("Ended Discord session {} via REST API", mid);
            ended.push(serde_json::json!({ "agent": a, "meeting_session_id": mid }));
        }
    }

    if ended.is_empty() {
        return Err(ApiError::NotFound(
            "No active session for that guild".into(),
        ));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "ended": ended,
        "note": "Sessions archived. Bots will leave the voice channel on next gateway interaction."
    })))
}

/// GET /discord/archive — list past Discord meeting sessions from DB
async fn list_archived_sessions(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let sessions = sqlx::query_as::<_, MeetingSession>(
        r#"SELECT * FROM meeting_sessions
           WHERE source_type = 'discord'
           ORDER BY started_at DESC
           LIMIT ?1 OFFSET ?2"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&deployment.db().pool)
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(sessions))
}

/// GET /discord/sessions/{id} — get a single meeting session
async fn get_session(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session = MeetingSession::find_by_id(&deployment.db().pool, &id)
        .await
        .map_err(|_| ApiError::NotFound("Session not found".into()))?;

    Ok(Json(session))
}

/// GET /discord/sessions/{id}/transcript — get all segments for a session
async fn get_transcript(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    #[derive(sqlx::FromRow, Serialize)]
    struct Segment {
        id: String,
        segment_index: i32,
        speaker_label: Option<String>,
        text: String,
        confidence: Option<f64>,
        start_time_ms: i64,
        end_time_ms: i64,
        is_topsi_addressed: bool,
        metadata: Option<String>,
        created_at: String,
    }

    let segments = sqlx::query_as::<_, Segment>(
        r#"SELECT id, segment_index, speaker_label, text, confidence,
                  start_time_ms, end_time_ms, is_topsi_addressed, metadata, created_at
           FROM meeting_segments
           WHERE meeting_session_id = ?1
           ORDER BY segment_index ASC"#,
    )
    .bind(&id)
    .fetch_all(&deployment.db().pool)
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "meeting_session_id": id,
        "segment_count": segments.len(),
        "segments": segments,
    })))
}

/// GET /discord/sessions/{id}/stream — SSE stream of live transcript events
async fn stream_transcript(Path(id): Path<String>) -> impl IntoResponse {
    let stream: Pin<Box<dyn Stream<Item = Result<SseEvent, Infallible>> + Send>> =
        match subscribe_transcript(&id) {
            Some(rx) => {
                let s = BroadcastStream::new(rx).filter_map(|item| {
                    let result = match item {
                        Ok(event) => serde_json::to_string(&event)
                            .ok()
                            .map(|json| Ok(SseEvent::default().event("transcript").data(json))),
                        Err(_) => Some(Ok(SseEvent::default()
                            .event("done")
                            .data(r#"{"message":"Session ended"}"#))),
                    };
                    std::future::ready(result)
                });
                Box::pin(s)
            }
            None => {
                let event = SseEvent::default()
                    .event("done")
                    .data(r#"{"message":"Session is not currently active"}"#);
                Box::pin(futures_util::stream::once(async move { Ok(event) }))
            }
        };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

//! Meeting-related handlers for Topsi

use super::{
    voice::{get_or_init_voice_engine, sanitize_text_for_tts},
    *,
};

// ============================================================================
// Meeting Request/Response Types
// ============================================================================

/// Request to start a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartMeetingRequest {
    pub project_id: String,
    pub title: Option<String>,
    pub session_id: Option<String>,
}

/// Response from starting a meeting
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartMeetingResponse {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub message: String,
}

/// Request with an audio chunk from a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAudioChunkRequest {
    pub session_id: String,
    pub audio_data: String, // Base64 encoded audio
    pub chunk_index: u32,
    pub duration_ms: u32,
}

/// Response from processing a meeting audio chunk
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAudioChunkResponse {
    pub session_id: String,
    pub chunk_index: u32,
    pub text: String,
    pub speaker_label: Option<String>,
    pub is_topsi_addressed: bool,
    pub topsi_response: Option<String>,
    pub topsi_audio_response: Option<String>,
    pub segment_index: i32,
}

/// Request to end a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EndMeetingRequest {
    pub session_id: String,
    pub generate_notes: Option<bool>,
}

/// Response from ending a meeting
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EndMeetingResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: i32,
    pub participant_count: i32,
    pub segment_count: usize,
    pub notes: Option<topsi::MeetingNotes>,
}

/// Request to share a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShareMeetingRequest {
    pub user_ids: Vec<String>,
}

/// Meeting status response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingStatusResponse {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub duration_seconds: Option<i32>,
    pub participant_count: i32,
    pub segment_count: i64,
    pub is_active: bool,
}

/// Meeting transcript response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTranscriptResponse {
    pub session_id: String,
    pub segments: Vec<db::models::meeting_session::MeetingSegment>,
    pub total_count: i64,
}

/// Request to join an existing meeting session
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingRequest {
    pub session_id: String,
}

/// Response from joining a meeting
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingResponse {
    pub session_id: String,
    pub title: String,
    pub project_id: String,
    pub participant_count: i32,
}

/// Request to add a typed text message/link to an active meeting (no audio)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMessageRequest {
    pub session_id: String,
    pub text: String,
    pub speaker_label: Option<String>,
    pub is_link: Option<bool>,
}

/// Response from posting a meeting message
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMessageResponse {
    pub session_id: String,
    pub segment_index: i32,
    pub text: String,
}

/// Query params for listing meetings
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMeetingsQuery {
    pub project_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Response for listing meetings
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMeetingsResponse {
    pub meetings: Vec<MeetingSessionSummary>,
    pub total: usize,
}

/// Summary of a meeting session for list views
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSessionSummary {
    pub id: String,
    pub project_id: String,

    pub title: String,
    pub status: String,
    pub started_by: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<i32>,
    pub participant_count: Option<i32>,
    pub segment_count: i64,
    pub notes: Option<serde_json::Value>,
}

// ============================================================================
// Meeting Mode Handlers
// ============================================================================

// TODO: unused — comment out to suppress warning
// /// Helper: get accessible project IDs (as lowercase hex, 32 chars) for a user
// /// Returns None for admins (all projects accessible).
// async fn get_accessible_project_hex_ids(
//     pool: &sqlx::SqlitePool,
//     user_id: &str,
//     is_admin: bool,
// ) -> Option<std::collections::HashSet<String>> {
//     if is_admin {
//         return None;
//     }
//     let uid = match uuid::Uuid::parse_str(user_id) {
//         Ok(u) => u,
//         Err(_) => return Some(std::collections::HashSet::new()),
//     };
//     // project_members.project_id is BLOB; compare via hex
//     let ids: Vec<Vec<u8>> = sqlx::query_scalar(
//         "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?1",
//     )
//     .bind(uid.as_bytes().as_slice())
//     .fetch_all(pool)
//     .await
//     .unwrap_or_default();
//
//     Some(
//         ids.into_iter()
//             .map(|b| hex::encode(&b))
//             .collect(),
//     )
// }

// TODO: unused — comment out to suppress warning
// /// Helper: check if a project_id (UUID text with dashes) is accessible given a hex-id set
// fn project_is_accessible(
//     project_id_str: &str,
//     accessible: &Option<std::collections::HashSet<String>>,
// ) -> bool {
//     match accessible {
//         None => true, // admin
//         Some(set) => {
//             let hex = project_id_str.replace('-', "").to_lowercase();
//             set.contains(&hex)
//         }
//     }
// }

/// List meeting sessions — scoped to user's accessible projects
pub async fn list_meetings(
    State(state): State<DeploymentImpl>,

    Query(params): Query<ListMeetingsQuery>,
) -> Result<Json<ListMeetingsResponse>, ApiError> {
    let pool = &state.db().pool;
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let sessions = db::models::meeting_session::MeetingSession::list(
        pool,
        params.project_id.as_deref(),
        limit,
        offset,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to list meetings: {}", e)))?;

    let status_filter = params.status.as_deref();

    // Build a project_id -> (project_name, org_name) lookup via a single query
    // #[derive(sqlx::FromRow)]
    // struct ProjectRow {
    //     id_hex: String,
    //     name: String,
    // }
    // let project_rows: Vec<ProjectRow> = sqlx::query_as(
    //     "SELECT lower(hex(id)) as id_hex, name FROM projects",
    // )
    // .fetch_all(pool)
    // .await
    // .unwrap_or_default();

    // let project_map: std::collections::HashMap<String, String> = project_rows
    //     .into_iter()
    //     .map(|r| (r.id_hex, r.name))
    //     .collect();

    // Batch segment counts for all sessions
    #[derive(sqlx::FromRow)]
    struct SegCountRow {
        session_id: String,
        cnt: i64,
    }
    let seg_counts: Vec<SegCountRow> = sqlx::query_as(
        "SELECT meeting_session_id as session_id, COUNT(*) as cnt FROM meeting_segments GROUP BY meeting_session_id"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let seg_count_map: std::collections::HashMap<String, i64> = seg_counts
        .into_iter()
        .map(|r| (r.session_id, r.cnt))
        .collect();

    let meetings: Vec<MeetingSessionSummary> = sessions
        .into_iter()
        .filter(|s| status_filter.is_none_or(|f| s.status == f))
        .map(|s| {
            let notes_value = s
                .notes
                .as_ref()
                .and_then(|n| serde_json::from_str::<serde_json::Value>(n).ok());

            // TODO: project_name lookup was computed but never used in the response
            // let hex = s.project_id.replace('-', "").to_lowercase();
            // let _project_name = project_map.get(&hex).cloned();

            let segment_count = seg_count_map.get(&s.id).copied().unwrap_or(0);
            MeetingSessionSummary {
                id: s.id,
                project_id: s.project_id,

                title: s.title,
                status: s.status,
                started_by: s.started_by,
                started_at: s.started_at,
                ended_at: s.ended_at,
                duration_seconds: s.duration_seconds,
                participant_count: s.participant_count,
                segment_count,
                notes: notes_value,
            }
        })
        .collect();

    let total = meetings.len();
    Ok(Json(ListMeetingsResponse { meetings, total }))
}

/// Join an existing active meeting session (increments participant count)
pub async fn join_meeting(
    State(state): State<DeploymentImpl>,

    Json(request): Json<JoinMeetingRequest>,
) -> Result<Json<JoinMeetingResponse>, ApiError> {
    let pool = &state.db().pool;

    let session =
        db::models::meeting_session::MeetingSession::find_by_id(pool, &request.session_id)
            .await
            .map_err(|_| {
                ApiError::NotFound(format!("Meeting session not found: {}", request.session_id))
            })?;

    if session.status != "active" {
        return Err(ApiError::BadRequest("Meeting is not active".to_string()));
    }

    let new_count = session.participant_count.unwrap_or(0) + 1;
    let updated = db::models::meeting_session::MeetingSession::update(
        pool,
        &session.id,
        db::models::meeting_session::UpdateMeetingSession {
            participant_count: Some(new_count),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // TODO: project_name query was executed but never used in the response — commented to avoid dead DB call
    // let _project_name: Option<String> = sqlx::query_scalar(
    //     "SELECT name FROM projects WHERE lower(hex(id)) = lower(replace(?1, '-', '')) AND deleted_at IS NULL",
    // )
    // .bind(&session.project_id)
    // .fetch_optional(pool)
    // .await
    // .ok()
    // .flatten();

    tracing::info!(
        "[MEETING] Session {} (project: {}) joined — participants: {}",
        session.id,
        session.project_id,
        new_count
    );

    Ok(Json(JoinMeetingResponse {
        session_id: updated.id,
        title: updated.title,
        project_id: updated.project_id,
        participant_count: updated.participant_count.unwrap_or(new_count),
    }))
}

/// Add a typed text message or link to an active meeting without audio
pub async fn meeting_text_message(
    State(state): State<DeploymentImpl>,
    _headers: axum::http::HeaderMap,
    Json(request): Json<MeetingMessageRequest>,
) -> Result<Json<MeetingMessageResponse>, ApiError> {
    let pool = &state.db().pool;

    let session =
        db::models::meeting_session::MeetingSession::find_by_id(pool, &request.session_id)
            .await
            .map_err(|_| {
                ApiError::NotFound(format!("Meeting session not found: {}", request.session_id))
            })?;

    if session.status != "active" {
        return Err(ApiError::BadRequest("Meeting is not active".to_string()));
    }

    let count =
        db::models::meeting_session::MeetingSegment::count_by_session(pool, &request.session_id)
            .await
            .unwrap_or(0);

    let text = if request.is_link.unwrap_or(false) {
        format!("[SHARED LINK] {}", request.text)
    } else {
        request.text.clone()
    };

    let segment = db::models::meeting_session::MeetingSegment::create(
        pool,
        db::models::meeting_session::CreateMeetingSegment {
            meeting_session_id: request.session_id.clone(),
            segment_index: count as i32,
            speaker_label: request.speaker_label.clone(),
            text: text.clone(),
            confidence: Some(1.0),
            start_time_ms: 0,
            end_time_ms: 0,
            is_topsi_addressed: false,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let speaker = request.speaker_label.as_deref().unwrap_or("participant");
    tracing::info!(
        "[MEETING] Text message from {} in session {}: {}",
        speaker,
        request.session_id,
        &text[..text.len().min(80)]
    );

    Ok(Json(MeetingMessageResponse {
        session_id: request.session_id,
        segment_index: segment.segment_index,
        text,
    }))
}

/// Start a new meeting session
pub async fn start_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<StartMeetingRequest>,
) -> Result<Json<StartMeetingResponse>, ApiError> {
    tracing::info!("Starting meeting for project: {}", request.project_id);

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::StartMeeting {
        project_id: request.project_id,
        title: request.title,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to start meeting: {}", e)))?;

    // Parse the response message as JSON to extract fields
    let response_json: serde_json::Value = serde_json::from_str(&response.message)
        .unwrap_or_else(|_| serde_json::json!({"message": response.message}));

    Ok(Json(StartMeetingResponse {
        session_id: response_json["session_id"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        title: response_json["title"]
            .as_str()
            .unwrap_or("Untitled Meeting")
            .to_string(),
        status: "active".to_string(),
        started_at: response_json["started_at"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        message: response_json["message"]
            .as_str()
            .unwrap_or("Meeting started")
            .to_string(),
    }))
}

/// Process an audio chunk from a meeting
pub async fn meeting_audio_chunk(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<MeetingAudioChunkRequest>,
) -> Result<Json<MeetingAudioChunkResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Step 1: Transcribe the audio using voice engine
    let transcribed_text = {
        let engine_result = get_or_init_voice_engine().await;
        if let Ok(engine_lock) = engine_result {
            let engine_guard = engine_lock.read().await;
            if let Some(engine) = engine_guard.as_ref() {
                match engine.transcribe_speech(&request.audio_data).await {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::warn!("Meeting transcription failed, using empty: {}", e);
                        String::new()
                    }
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    };

    if transcribed_text.is_empty() {
        // Return acknowledgement for empty/silent chunks
        return Ok(Json(MeetingAudioChunkResponse {
            session_id: request.session_id,
            chunk_index: request.chunk_index,
            text: String::new(),
            speaker_label: None,
            is_topsi_addressed: false,
            topsi_response: None,
            topsi_audio_response: None,
            segment_index: request.chunk_index as i32,
        }));
    }

    // Step 2: Process through Topsi agent (stores segment, detects wake word)
    let topsi_request = TopsiRequest::new(TopsiRequestType::MeetingAudioChunk {
        session_id: request.session_id.clone(),
        audio_data: transcribed_text.clone(),
        chunk_index: request.chunk_index,
        duration_ms: request.duration_ms,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Meeting audio processing failed: {}", e)))?;

    // Parse response
    let response_json: serde_json::Value =
        serde_json::from_str(&response.message).unwrap_or_else(|_| serde_json::json!({}));

    let is_addressed = response_json["is_topsi_addressed"]
        .as_bool()
        .unwrap_or(false);
    let topsi_response_text = response_json["topsi_response"]
        .as_str()
        .map(|s| s.to_string());

    // Step 3: If Topsi responded, synthesize audio response
    let topsi_audio = if let Some(ref response_text) = topsi_response_text {
        let engine_result = get_or_init_voice_engine().await;
        if let Ok(engine_lock) = engine_result {
            let engine_guard = engine_lock.read().await;
            if let Some(engine) = engine_guard.as_ref() {
                let tts_text = sanitize_text_for_tts(response_text);
                if !tts_text.is_empty() {
                    match engine.synthesize_speech(&tts_text).await {
                        Ok(audio) => Some(audio),
                        Err(e) => {
                            tracing::warn!("Failed to synthesize meeting response: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(MeetingAudioChunkResponse {
        session_id: request.session_id,
        chunk_index: request.chunk_index,
        text: transcribed_text,
        speaker_label: response_json["speaker_label"]
            .as_str()
            .map(|s| s.to_string()),
        is_topsi_addressed: is_addressed,
        topsi_response: topsi_response_text,
        topsi_audio_response: topsi_audio,
        segment_index: response_json["segment_index"].as_i64().unwrap_or(0) as i32,
    }))
}

/// End a meeting session
pub async fn end_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<EndMeetingRequest>,
) -> Result<Json<EndMeetingResponse>, ApiError> {
    tracing::info!("Ending meeting: {}", request.session_id);

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::EndMeeting {
        session_id: request.session_id.clone(),
        generate_notes: request.generate_notes.unwrap_or(true),
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to end meeting: {}", e)))?;

    let response_json: serde_json::Value =
        serde_json::from_str(&response.message).unwrap_or_else(|_| serde_json::json!({}));

    let notes: Option<topsi::MeetingNotes> = response_json
        .get("notes")
        .and_then(|n| serde_json::from_value(n.clone()).ok());

    Ok(Json(EndMeetingResponse {
        session_id: request.session_id,
        status: "ended".to_string(),
        duration_seconds: response_json["duration_seconds"].as_i64().unwrap_or(0) as i32,
        participant_count: response_json["participant_count"].as_i64().unwrap_or(0) as i32,
        segment_count: response_json["segment_count"].as_u64().unwrap_or(0) as usize,
        notes,
    }))
}

/// Get meeting status
pub async fn meeting_status(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<MeetingStatusResponse>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    // Access control
    if !session.has_access(&user_context.user_id, user_context.is_admin) {
        return Err(ApiError::Forbidden(
            "Access denied to this meeting".to_string(),
        ));
    }

    let segment_count =
        db::models::meeting_session::MeetingSegment::count_by_session(&pool, &session_id)
            .await
            .unwrap_or(0);

    let is_active = session.status == "active";

    Ok(Json(MeetingStatusResponse {
        session_id: session.id,
        title: session.title,
        status: session.status,
        started_at: session.started_at,
        duration_seconds: session.duration_seconds,
        participant_count: session.participant_count.unwrap_or(0),
        segment_count,
        is_active,
    }))
}

/// Get meeting notes
pub async fn get_meeting_notes(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    if !session.has_access(&user_context.user_id, user_context.is_admin) {
        return Err(ApiError::Forbidden(
            "Access denied to this meeting".to_string(),
        ));
    }

    let notes = session
        .notes
        .and_then(|n| serde_json::from_str::<serde_json::Value>(&n).ok());

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "notes": notes,
    })))
}

/// POST /topsi/meeting/notes/:session_id — Regenerate meeting notes via AI
pub async fn regenerate_meeting_notes(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    if !session.has_access(&user_context.user_id, user_context.is_admin) {
        return Err(ApiError::Forbidden(
            "Access denied to this meeting".to_string(),
        ));
    }

    // Fetch transcript segments to regenerate notes from
    let segments = db::models::meeting_session::MeetingSegment::find_by_session(&pool, &session_id)
        .await
        .unwrap_or_default();

    if segments.is_empty() {
        return Err(ApiError::BadRequest(
            "No transcript segments available to generate notes from".to_string(),
        ));
    }

    let transcript_text = segments
        .iter()
        .map(|s| {
            format!(
                "[{}] {}",
                s.speaker_label.as_deref().unwrap_or("Speaker"),
                s.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Generate notes via Anthropic
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .unwrap_or_default();

    let notes = if api_key.is_empty() {
        serde_json::json!({
            "summary": "Notes regeneration unavailable — no API key configured.",
            "action_items": [],
            "key_decisions": []
        })
    } else {
        let client = reqwest::Client::new();
        let prompt = format!(
            "You are a meeting notes assistant. Summarize the following meeting transcript into structured notes.\n\nTranscript:\n{}\n\nReturn ONLY a JSON object with keys: summary (string), action_items (array of strings), key_decisions (array of strings), topics_discussed (array of strings).",
            &transcript_text[..transcript_text.len().min(8000)]
        );
        let body = serde_json::json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}]
        });
        match client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let val: serde_json::Value = resp.json().await.unwrap_or_default();
                let text = val
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|b| b.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("{}");
                // Try to parse as JSON, fall back to wrapping in summary
                serde_json::from_str(text).unwrap_or_else(|_| serde_json::json!({"summary": text, "action_items": [], "key_decisions": []}))
            }
            _ => serde_json::json!({
                "summary": "Notes generation failed — please try again.",
                "action_items": [],
                "key_decisions": []
            }),
        }
    };

    // Persist regenerated notes back to the session
    let notes_json = serde_json::to_string(&notes).unwrap_or_else(|_| "{}".to_string());
    let _ = db::models::meeting_session::MeetingSession::update(
        &pool,
        &session_id,
        db::models::meeting_session::UpdateMeetingSession {
            notes: Some(notes_json),
            ..Default::default()
        },
    )
    .await;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "notes": notes,
        "regenerated": true,
    })))
}

/// Get meeting transcript
pub async fn get_meeting_transcript(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<MeetingTranscriptResponse>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    if !session.has_access(&user_context.user_id, user_context.is_admin) {
        return Err(ApiError::Forbidden(
            "Access denied to this meeting".to_string(),
        ));
    }

    let segments = db::models::meeting_session::MeetingSegment::find_by_session(&pool, &session_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch transcript: {}", e)))?;

    let total_count = segments.len() as i64;

    Ok(Json(MeetingTranscriptResponse {
        session_id,
        segments,
        total_count,
    }))
}

/// Share a meeting with other users
pub async fn share_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
    Json(request): Json<ShareMeetingRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    // Only admin or meeting starter can share
    if !user_context.is_admin && session.started_by != user_context.user_id {
        return Err(ApiError::Forbidden(
            "Only the meeting creator or admin can share meetings".to_string(),
        ));
    }

    // Merge new user_ids with existing shared_with
    let mut shared_users: Vec<String> = session
        .shared_with
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    for user_id in &request.user_ids {
        if !shared_users.contains(user_id) {
            shared_users.push(user_id.clone());
        }
    }

    let shared_json = serde_json::to_string(&shared_users).unwrap_or_else(|_| "[]".to_string());

    db::models::meeting_session::MeetingSession::update(
        &pool,
        &session_id,
        db::models::meeting_session::UpdateMeetingSession {
            shared_with: Some(shared_json),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to update sharing: {}", e)))?;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "shared_with": shared_users,
        "message": format!("Meeting shared with {} users", request.user_ids.len()),
    })))
}

//! Nora Google Meet integration
//!
//! Spawns meet-bot.js (Playwright) to join a Google Meet as Nora,
//! routes participant audio through Whisper STT → Nora LLM → ElevenLabs/Chatterbox TTS,
//! and injects Nora's voice back into the meeting via PipeWire virtual audio devices.

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::process::Stdio;
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::{Event as SseEvent, KeepAlive, Sse}},
    routing::{get, post},
};
use futures::Stream;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, broadcast};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;

use db::models::meeting_session::{CreateMeetingSession, MeetingSession, UpdateMeetingSession, MeetingStatus, CreateMeetingSegment, MeetingSegment};
use db::models::project_knowledge_source::{ProjectKnowledgeSource, KnowledgeSourceType};
use db::models::data_source::{CreateDataSource, DataSource};
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use deployment::Deployment;
use crate::{DeploymentImpl, routes::nora::get_nora_instance};

// ── Global state ─────────────────────────────────────────────────────────────

#[derive(Debug)]
struct ActiveMeetSession {
    session_id: String,
    meet_url: String,
    project_id: String,
    process: Option<Child>,
    tx: broadcast::Sender<TranscriptEvent>,
    tts_queue: Vec<Vec<u8>>,
    started_at: Instant,
    segment_count: i32,
}

#[derive(Debug, Clone, Serialize)]
struct TranscriptEvent {
    speaker: String,
    text: String,
    segment_index: i32,
    is_nora: bool,
}

static ACTIVE_MEETS: Lazy<Arc<Mutex<HashMap<String, ActiveMeetSession>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetRequest {
    pub meet_url: String,
    pub project_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetSessionInfo {
    pub session_id: String,
    pub meet_url: String,
    pub project_id: String,
    pub elapsed_seconds: u64,
    pub segment_count: i32,
}

// ── Route handlers ────────────────────────────────────────────────────────────

/// POST /nora/join-meet
pub async fn join_meet(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<JoinMeetRequest>,
) -> Result<Json<JoinMeetResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = deployment.db().pool.clone();
    let meet_url = body.meet_url.trim().to_string();
    if !meet_url.contains("meet.google.com") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "URL must be a Google Meet link (meet.google.com)" })),
        ));
    }

    // Resolve project_id: use provided value if non-empty, otherwise fall back to
    // the first admin-owned project so sessions appear in the dashboard
    let project_id = if body.project_id.is_empty() || body.project_id == "00000000000000000000000000000001" {
        sqlx::query_scalar::<_, String>(
            "SELECT lower(hex(p.id)) FROM projects p \
             JOIN project_members pm ON pm.project_id = p.id \
             JOIN users u ON u.id = pm.user_id \
             WHERE u.is_admin = 1 AND u.is_active = 1 \
             ORDER BY p.created_at DESC LIMIT 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|_| body.project_id.clone())
    } else {
        body.project_id.clone()
    };

    let db_session = MeetingSession::create(&pool, CreateMeetingSession {
        project_id,
        title: Some(body.title.unwrap_or_else(|| "Google Meet with Nora".to_string())),
        started_by: "nora".to_string(),
    }).await.map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": format!("Failed to create meeting session: {}", e) })),
    ))?;

    // Use the DB-generated ID as the canonical session ID everywhere
    let session_id = db_session.id.clone();

    let (tx, _rx) = broadcast::channel::<TranscriptEvent>(128);
    let script_path = find_script("meet-bot.js");
    let server_url = std::env::var("BACKEND_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let profile_dir = std::env::var("NORA_CHROME_PROFILE")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/pythia".to_string());
            format!("{}/nora-chrome-profile", home)
        });

    info!("[MEET] Spawning meet-bot: session={} url={}", session_id, meet_url);

    let mut child = Command::new("node")
        .arg(&script_path)
        .arg(&meet_url)
        .arg(&session_id)
        .arg(&server_url)
        .arg(&profile_dir)
        .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":1".to_string()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to start meet bot: {}", e) })),
        ))?;

    // Log bot stdout events
    let sid_log = session_id.clone();
    let pool_log = pool.clone();
    let tx_log = tx.clone();
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!("[MEET-BOT {}] {}", &sid_log[..8], line);
                if let Ok(ev) = serde_json::from_str::<serde_json::Value>(&line) {
                    match ev.get("type").and_then(|t| t.as_str()) {
                        Some("joined") => {
                            // Auto-route Chrome's audio sink-input to Nora's virtual sink
                            let out_sink = format!("nora-meet-out-{}", &sid_log[..8]);
                            let sessions_intro = ACTIVE_MEETS.clone();
                            let sid_intro = sid_log.clone();
                            tokio::spawn(async move {
                                // Brief pause for Chrome to finish setting up audio streams
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                // Find Chrome sink-inputs and move them to our virtual sink
                                if let Ok(out) = std::process::Command::new("pactl")
                                    .args(["list", "short", "sink-inputs"])
                                    .output()
                                {
                                    let stdout = String::from_utf8_lossy(&out.stdout);
                                    for line in stdout.lines() {
                                        if let Some(sink_input_id) = line.split_whitespace().next() {
                                            let _ = std::process::Command::new("pactl")
                                                .args(["move-sink-input", sink_input_id, &out_sink])
                                                .output();
                                            info!("[MEET] Moved sink-input {} to {}", sink_input_id, out_sink);
                                        }
                                    }
                                }
                                // Give audio routing a moment to settle, then introduce Nora
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                let intro = "Hello everyone, I'm Nora. I'll be listening in — just say my name if you'd like my input.";
                                info!("[MEET] Synthesising intro for session {}", &sid_intro[..8]);
                                if let Some(audio) = synthesize_tts(intro).await {
                                    let mut sessions = sessions_intro.lock().await;
                                    if let Some(sess) = sessions.get_mut(&sid_intro) {
                                        sess.tts_queue.push(audio);
                                        info!("[MEET] Intro queued for session {}", &sid_intro[..8]);
                                    }
                                }
                            });
                        }
                        Some("transcript") => {
                            if let Some(text) = ev.get("text").and_then(|t| t.as_str()) {
                                let speaker = ev.get("speaker").and_then(|s| s.as_str()).unwrap_or("participant").to_string();
                                let _ = tx_log.send(TranscriptEvent {
                                    speaker, text: text.to_string(), segment_index: 0, is_nora: false,
                                });
                            }
                        }
                        Some("done") | Some("leaving") => {
                            let _ = MeetingSession::update(&pool_log, &sid_log, UpdateMeetingSession {
                                status: Some(MeetingStatus::Ended),
                                ended_at: Some(Utc::now().to_rfc3339()),
                                ..Default::default()
                            }).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    let sid_err = session_id.clone();
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!("[MEET-BOT-ERR {}] {}", &sid_err[..8], line);
            }
        });
    }

    {
        let mut sessions = ACTIVE_MEETS.lock().await;
        sessions.insert(session_id.clone(), ActiveMeetSession {
            session_id: session_id.clone(),
            meet_url: meet_url.clone(),
            project_id: body.project_id.clone(),
            process: Some(child),
            tx,
            tts_queue: Vec::new(),
            started_at: Instant::now(),
            segment_count: 0,
        });
    }

    Ok(Json(JoinMeetResponse {
        session_id,
        status: "joining".to_string(),
        message: "Nora is joining the Google Meet".to_string(),
    }))
}

/// POST /nora/meet/:id/audio  — receive WAV chunk, STT → Nora → TTS queue
pub async fn receive_audio(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<String>,
    body: Bytes,
) -> Response {
    let pool = deployment.db().pool.clone();

    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "empty audio" }))).into_response();
    }

    let whisper_url = std::env::var("WHISPER_URL")
        .unwrap_or_else(|_| "http://localhost:8101".to_string());

    let transcript = call_whisper(&whisper_url, &body).await;
    let transcript = match transcript {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => return Json(json!({ "transcript": null })).into_response(),
    };

    info!("[MEET {}] STT: {}", &session_id[..session_id.len().min(8)], transcript);

    // Only engage Nora when she's directly addressed by name
    let addressed = transcript.to_lowercase().contains("nora");

    // Store participant segment + broadcast
    let participant_idx = {
        let mut sessions = ACTIVE_MEETS.lock().await;
        let idx = if let Some(sess) = sessions.get_mut(&session_id) {
            sess.segment_count += 1;
            let _ = sess.tx.send(TranscriptEvent {
                speaker: "participant".to_string(),
                text: transcript.clone(),
                segment_index: sess.segment_count,
                is_nora: false,
            });
            sess.segment_count
        } else {
            // Session not in memory (e.g. after server restart) — use DB to determine index
            let count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM meeting_segments WHERE meeting_session_id = ?"
            )
            .bind(&session_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(0) as i32;
            count + 1
        };
        idx
    };
    let _ = MeetingSegment::create(&pool, CreateMeetingSegment {
        meeting_session_id: session_id.clone(),
        segment_index: participant_idx,
        speaker_label: Some("participant".to_string()),
        text: transcript.clone(),
        confidence: None,
        start_time_ms: 0,
        end_time_ms: 2000,
        is_topsi_addressed: false,
    }).await;

    // Only engage Nora when addressed by name — transcript is always stored above
    if !addressed {
        return Json(json!({ "transcript": transcript })).into_response();
    }

    // Call Nora
    let nora_text = {
        let nora_arc = match get_nora_instance().await {
            Ok(a) => a,
            Err(e) => {
                warn!("[MEET] get_nora_instance error: {:?}", e);
                return Json(json!({ "transcript": transcript })).into_response();
            }
        };
        let guard = nora_arc.read().await;
        if let Some(nora) = guard.as_ref() {
            let req = NoraRequest {
                request_id: Uuid::new_v4().to_string(),
                session_id: format!("meet-{}", session_id),
                request_type: NoraRequestType::TextInteraction,
                content: format!(
                    "[GOOGLE MEET — Audio only. Keep response to 2-3 sentences max. No markdown.]\n\n{}",
                    transcript
                ),
                context: None,
                voice_enabled: false,
                priority: RequestPriority::Normal,
                timestamp: Utc::now(),
            };
            match nora.process_request(req).await {
                Ok(resp) => Some(resp.content),
                Err(e) => { warn!("[MEET] Nora error: {}", e); None }
            }
        } else {
            warn!("[MEET] Nora not initialized");
            None
        }
    };

    let nora_text = match nora_text {
        Some(t) if !t.trim().is_empty() => t,
        _ => return Json(json!({ "transcript": transcript })).into_response(),
    };

    info!("[MEET {}] Nora: {}", &session_id[..session_id.len().min(8)], nora_text);

    // Store Nora segment + broadcast
    let nora_idx = {
        let mut sessions = ACTIVE_MEETS.lock().await;
        let idx = if let Some(sess) = sessions.get_mut(&session_id) {
            sess.segment_count += 1;
            let _ = sess.tx.send(TranscriptEvent {
                speaker: "Nora".to_string(),
                text: nora_text.clone(),
                segment_index: sess.segment_count,
                is_nora: true,
            });
            sess.segment_count
        } else {
            let count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM meeting_segments WHERE meeting_session_id = ?"
            )
            .bind(&session_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(0) as i32;
            count + 1
        };
        idx
    };
    let _ = MeetingSegment::create(&pool, CreateMeetingSegment {
        meeting_session_id: session_id.clone(),
        segment_index: nora_idx,
        speaker_label: Some("Nora".to_string()),
        text: nora_text.clone(),
        confidence: Some(1.0),
        start_time_ms: 0,
        end_time_ms: 0,
        is_topsi_addressed: false,
    }).await;

    // Synthesize TTS and queue
    if let Some(audio) = synthesize_tts(&nora_text).await {
        let mut sessions = ACTIVE_MEETS.lock().await;
        if let Some(sess) = sessions.get_mut(&session_id) {
            sess.tts_queue.push(audio);
        }
    }

    Json(json!({ "transcript": transcript, "nora_response": nora_text })).into_response()
}

/// GET /nora/meet/:id/next-tts — long-poll for next TTS WAV buffer
pub async fn next_tts(Path(session_id): Path<String>) -> Response {
    let deadline = Instant::now() + Duration::from_secs(25);

    loop {
        {
            let mut sessions = ACTIVE_MEETS.lock().await;
            if let Some(sess) = sessions.get_mut(&session_id) {
                if !sess.tts_queue.is_empty() {
                    let audio = sess.tts_queue.remove(0);
                    return (StatusCode::OK, [("Content-Type", "audio/wav")], audio).into_response();
                }
            } else {
                return StatusCode::NOT_FOUND.into_response();
            }
        }

        if Instant::now() >= deadline {
            return StatusCode::NO_CONTENT.into_response();
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// GET /nora/meet/sessions
pub async fn list_sessions() -> Json<Vec<MeetSessionInfo>> {
    let sessions = ACTIVE_MEETS.lock().await;
    Json(sessions.values().map(|s| MeetSessionInfo {
        session_id: s.session_id.clone(),
        meet_url: s.meet_url.clone(),
        project_id: s.project_id.clone(),
        elapsed_seconds: s.started_at.elapsed().as_secs(),
        segment_count: s.segment_count,
    }).collect())
}

/// POST /nora/meet/:id/leave
pub async fn leave_meet(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let pool = deployment.db().pool.clone();
    let mut sessions = ACTIVE_MEETS.lock().await;

    if let Some(mut sess) = sessions.remove(&session_id) {
        if let Some(mut child) = sess.process.take() {
            let _ = child.kill().await;
        }
        let _ = MeetingSession::update(&pool, &session_id, UpdateMeetingSession {
            status: Some(MeetingStatus::Ended),
            ended_at: Some(Utc::now().to_rfc3339()),
            ..Default::default()
        }).await;
        save_meeting_knowledge_source(&pool, &session_id).await;
        Json(json!({ "status": "left", "session_id": session_id }))
    } else {
        Json(json!({ "status": "not_found" }))
    }
}

/// Save completed meeting as a knowledge source (Conversation type)
async fn save_meeting_knowledge_source(pool: &sqlx::SqlitePool, session_id: &str) {
    // Fetch session metadata
    let session = match MeetingSession::find_by_id(pool, session_id).await {
        Ok(s) => s,
        Err(_) => return,
    };

    // Parse project UUID — stored as plain hex string in meeting_sessions
    let project_uuid = match uuid::Uuid::parse_str(&session.project_id) {
        Ok(u) => u,
        Err(_) => {
            // Try hex without dashes
            let hex = session.project_id.replace('-', "");
            match hex::decode(&hex).ok().and_then(|b| uuid::Uuid::from_slice(&b).ok()) {
                Some(u) => u,
                None => return,
            }
        }
    };

    let seg_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM meeting_segments WHERE meeting_session_id = ?"
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Fetch all transcript segments for full-text content
    #[derive(sqlx::FromRow)]
    struct SegRow { speaker_label: Option<String>, text: String }
    let all_segs: Vec<SegRow> = sqlx::query_as(
        "SELECT speaker_label, text FROM meeting_segments WHERE meeting_session_id = ? ORDER BY segment_index ASC"
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let summary = if seg_count > 0 {
        let preview: String = all_segs.iter().take(5).map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        let preview_trimmed = if preview.len() > 300 { &preview[..300] } else { &preview };
        format!("{} transcript segments captured. Preview: {}", seg_count, preview_trimmed)
    } else {
        "Nora attended this meeting. No transcript was captured (audio may not have been active).".to_string()
    };

    // Full transcript as plain text (for data_sources content)
    let transcript_text: String = all_segs.iter().map(|s| {
        let label = s.speaker_label.as_deref().unwrap_or("Speaker");
        format!("[{}] {}", label, s.text)
    }).collect::<Vec<_>>().join("\n");

    let title = format!("{} ({})", session.title, &session_id[..8]);
    let coverage = if seg_count > 0 { (seg_count as f64 / 100.0).min(1.0).max(0.3) } else { 0.1 };

    let full_content = if transcript_text.is_empty() {
        format!("Nora attended: {}\n\nNo audio transcript was captured.", session.title)
    } else {
        format!("Meeting: {}\nDate: {}\n\n{}", session.title, session.started_at, transcript_text)
    };

    // 1. Save to project_knowledge_sources (project knowledge graph)
    let _ = ProjectKnowledgeSource::upsert_source(
        pool,
        project_uuid,
        &KnowledgeSourceType::Conversation,
        session_id,
        &title,
        Some(&summary),
        coverage,
    ).await;

    // 2. Save to data_sources (organization dashboard)
    // Skip if already saved (idempotent check via metadata json containing session_id)
    let already_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM data_sources WHERE json_extract(metadata, '$.meeting_session_id') = ?"
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if already_exists == 0 {
        // Look up organization_id from the project (include soft-deleted projects)
        let org_id_bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT organization_id FROM projects WHERE lower(hex(id)) = ?"
        )
        .bind(&session.project_id.replace('-', "").to_lowercase())
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .flatten();

        // Fallback: use admin user's default org
        let org_uuid: Option<Uuid> = if let Some(bytes) = org_id_bytes {
            Uuid::from_slice(&bytes).ok()
        } else {
            // Look up admin's first organization
            let admin_org: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT om.organization_id FROM organization_members om \
                 JOIN users u ON u.id = om.user_id \
                 WHERE u.is_admin = 1 AND u.is_active = 1 \
                 ORDER BY om.created_at ASC LIMIT 1"
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .flatten();
            admin_org.and_then(|b| Uuid::from_slice(&b).ok())
        };

        let metadata = serde_json::json!({
            "meeting_session_id": session_id,
            "started_by": session.started_by,
            "segment_count": seg_count,
            "source": "nora_meet"
        }).to_string();

        let _ = DataSource::create(pool, CreateDataSource {
            organization_id: org_uuid.map(|u| u.to_string()),
            project_id: None,
            created_by: None,
            title: title.clone(),
            description: Some(summary.clone()),
            data_type: "conversation".to_string(),
            source_type: Some("text".to_string()),
            content: Some(full_content.clone()),
            file_name: None,
            file_type: None,
            file_path: None,
            file_size_bytes: None,
            file_hash: None,
            metadata: Some(metadata),
            folder: None,
        }).await;

        // Mark as ready immediately (no processing needed for text)
        let _ = sqlx::query(
            "UPDATE data_sources SET status='ready' \
             WHERE json_extract(metadata, '$.meeting_session_id') = ?"
        )
        .bind(session_id)
        .execute(pool)
        .await;
    }

    // Auto-detect attendees from title (e.g. "Meet with Sirak" → look up person "Sirak")
    // If found, link them on the session and also save to their org's data sources
    detect_and_link_attendees(pool, session_id, &session.title, &full_content, &title, &summary).await;

    info!("Saved meeting {} as knowledge source and data source ({} segments)", session_id, seg_count);
}

/// Extract names from meeting title (e.g. "Meet with Sirak", "Call with John & Amy"),
/// look up matching persons/users, link them on the session, and save to their org's data_sources.
async fn detect_and_link_attendees(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    title: &str,
    content: &str,
    ds_title: &str,
    ds_description: &str,
) {
    // Extract candidate names from common title patterns
    let lower = title.to_lowercase();
    let name_part = ["meet with ", "call with ", "chat with ", "meeting with ", "sync with "]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))
        .unwrap_or("");

    if name_part.is_empty() {
        return;
    }

    // Split on "&", "and", ","
    let raw_names: Vec<&str> = name_part
        .split(|c: char| c == '&' || c == ',')
        .flat_map(|s| s.split(" and "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if raw_names.is_empty() {
        return;
    }

    for name in raw_names {
        // Look up person or user by name (case-insensitive partial match)
        #[derive(sqlx::FromRow)]
        struct PersonRow { id: Vec<u8> }

        // Try persons table first
        let person_bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT id FROM persons WHERE lower(full_name) LIKE lower('%' || ? || '%') LIMIT 1"
        )
        .bind(name)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .flatten();

        // Fallback: users table
        let person_bytes = if person_bytes.is_some() { person_bytes } else {
            sqlx::query_scalar(
                "SELECT id FROM users WHERE lower(username) LIKE lower('%' || ? || '%') \
                 OR lower(full_name) LIKE lower('%' || ? || '%') LIMIT 1"
            )
            .bind(name)
            .bind(name)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .flatten()
        };

        let person_uuid = match person_bytes.and_then(|b| Uuid::from_slice(&b).ok()) {
            Some(u) => u,
            None => continue,
        };
        let person_hex = person_uuid.simple().to_string();

        // Update attendee_person_ids on the session (add if not already present)
        let current_ids: String = sqlx::query_scalar(
            "SELECT COALESCE(attendee_person_ids, '[]') FROM meeting_sessions WHERE id = ?"
        )
        .bind(session_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "[]".to_string());

        if !current_ids.contains(&person_hex) {
            let new_ids = {
                let mut ids: Vec<String> = serde_json::from_str(&current_ids).unwrap_or_default();
                ids.push(person_hex.clone());
                serde_json::to_string(&ids).unwrap_or_else(|_| format!("[\"{}\"]", person_hex))
            };
            let _ = sqlx::query(
                "UPDATE meeting_sessions SET attendee_person_ids = ?, linked_person_ids = ? WHERE id = ?"
            )
            .bind(&new_ids)
            .bind(&new_ids)
            .bind(session_id)
            .execute(pool)
            .await;
        }

        // Find the person's organization via organization_members
        let attendee_org: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT om.organization_id FROM organization_members om \
             WHERE om.user_id = ? \
             ORDER BY om.created_at ASC LIMIT 1"
        )
        .bind(person_uuid.as_bytes().as_slice())
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .flatten();

        let attendee_org_uuid = match attendee_org.and_then(|b| Uuid::from_slice(&b).ok()) {
            Some(u) => u,
            None => continue,
        };

        // Skip if this org already has a data_source for this session
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM data_sources \
             WHERE json_extract(metadata, '$.meeting_session_id') = ? \
             AND organization_id = ?"
        )
        .bind(session_id)
        .bind(attendee_org_uuid.as_bytes().as_slice())
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if exists > 0 {
            continue;
        }

        let metadata = serde_json::json!({
            "meeting_session_id": session_id,
            "attendee_person_id": person_hex,
            "source": "nora_meet"
        }).to_string();

        let _ = DataSource::create(pool, CreateDataSource {
            organization_id: Some(attendee_org_uuid.to_string()),
            project_id: None,
            created_by: None,
            title: ds_title.to_string(),
            description: Some(ds_description.to_string()),
            data_type: "conversation".to_string(),
            source_type: Some("text".to_string()),
            content: Some(content.to_string()),
            file_name: None,
            file_type: None,
            file_path: None,
            file_size_bytes: None,
            file_hash: None,
            metadata: Some(metadata),
            folder: None,
        }).await;

        let _ = sqlx::query(
            "UPDATE data_sources SET status='ready' \
             WHERE json_extract(metadata, '$.meeting_session_id') = ? \
             AND organization_id = ?"
        )
        .bind(session_id)
        .bind(attendee_org_uuid.as_bytes().as_slice())
        .execute(pool)
        .await;

        info!("Linked attendee {} (org {}) to meeting {}", person_hex, attendee_org_uuid, session_id);
    }
}

/// POST /nora/meet/:id/left  (called by meet-bot.js on natural exit)
pub async fn bot_left(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let pool = deployment.db().pool.clone();
    ACTIVE_MEETS.lock().await.remove(&session_id);
    let _ = MeetingSession::update(&pool, &session_id, UpdateMeetingSession {
        status: Some(MeetingStatus::Ended),
        ended_at: Some(Utc::now().to_rfc3339()),
        ..Default::default()
    }).await;
    save_meeting_knowledge_source(&pool, &session_id).await;
    StatusCode::OK
}

/// GET /nora/meet/:id/transcript  — SSE live transcript
pub async fn transcript_stream(
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let stream: Pin<Box<dyn Stream<Item = Result<SseEvent, Infallible>> + Send>> = {
        let sessions = ACTIVE_MEETS.lock().await;
        match sessions.get(&session_id) {
            Some(sess) => {
                let rx = sess.tx.subscribe();
                let s = BroadcastStream::new(rx).filter_map(|item| {
                    let result = match item {
                        Ok(ev) => serde_json::to_string(&ev).ok().map(|json| {
                            Ok(SseEvent::default().event("transcript").data(json))
                        }),
                        Err(_) => Some(Ok(SseEvent::default().event("done").data(r#"{"message":"session ended"}"#))),
                    };
                    std::future::ready(result)
                });
                Box::pin(s)
            }
            None => {
                let ev = SseEvent::default().event("error").data(r#"{"error":"session not found"}"#);
                Box::pin(futures_util::stream::once(async move { Ok(ev) }))
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_script(name: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/pythia".to_string());
    let candidates = [
        format!("scripts/{}", name),
        format!("./scripts/{}", name),
        format!("../scripts/{}", name),
        format!("{}/pcg-cc-mcp/scripts/{}", home, name),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() { return c.clone(); }
    }
    format!("scripts/{}", name)
}

async fn call_whisper(whisper_url: &str, wav: &[u8]) -> Option<String> {
    use reqwest::multipart;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build().ok()?;

    // Use /transcribe/file endpoint — sends WAV with correct filename so Whisper
    // saves it with .wav extension and ffmpeg decodes it correctly
    let part = multipart::Part::bytes(wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav").ok()?;

    let resp = client
        .post(format!("{}/transcribe/file", whisper_url))
        .multipart(multipart::Form::new().part("file", part))
        .send().await.ok()?;

    if !resp.status().is_success() {
        warn!("[MEET-WHISPER] Error {}: {}", resp.status(), resp.text().await.unwrap_or_default());
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("text")
        .or_else(|| json.get("transcript"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

async fn synthesize_tts(text: &str) -> Option<Vec<u8>> {
    if let Ok(key) = std::env::var("ELEVENLABS_API_KEY") {
        if !key.is_empty() {
            if let Some(audio) = elevenlabs_tts(text, &key).await { return Some(audio); }
        }
    }
    let chatterbox_url = std::env::var("CHATTERBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8102".to_string());
    chatterbox_tts(text, &chatterbox_url).await
}

async fn elevenlabs_tts(text: &str, api_key: &str) -> Option<Vec<u8>> {
    let voice_id = std::env::var("ELEVENLABS_VOICE_ID")
        .unwrap_or_else(|_| "ZtcPZrt9K4w8e1OB9M6w".to_string());
    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build().ok()?;
    let resp = client
        .post(format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id))
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": { "stability": 0.5, "similarity_boost": 0.75 }
        }))
        .send().await.ok()?;
    if !resp.status().is_success() { return None; }
    Some(resp.bytes().await.ok()?.to_vec())
}

async fn chatterbox_tts(text: &str, chatterbox_url: &str) -> Option<Vec<u8>> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build().ok()?;
    let resp = client
        .post(format!("{}/synthesize", chatterbox_url))
        .json(&json!({ "text": text, "voice": "p225" }))
        .send().await.ok()?;
    if !resp.status().is_success() { return None; }
    Some(resp.bytes().await.ok()?.to_vec())
}

// ── Inbox watcher ─────────────────────────────────────────────────────────────

/// Global set of message IDs already processed (persists across polls, resets on restart)
static SEEN_MESSAGE_IDS: Lazy<Arc<Mutex<HashSet<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

/// Zoho token cache: (access_token, refreshed_at)
static ZOHO_TOKEN_CACHE: Lazy<Arc<Mutex<(String, Instant)>>> =
    Lazy::new(|| Arc::new(Mutex::new((String::new(), Instant::now() - Duration::from_secs(9999)))));

async fn get_zoho_token(pool: &sqlx::SqlitePool) -> Option<String> {
    // Return cached token if < 50 minutes old
    {
        let cache = ZOHO_TOKEN_CACHE.lock().await;
        if cache.0.is_empty() == false && cache.1.elapsed() < Duration::from_secs(50 * 60) {
            return Some(cache.0.clone());
        }
    }

    // Fetch credentials from DB
    let row = sqlx::query!(
        "SELECT access_token, refresh_token FROM email_accounts WHERE email_address = 'nora@powerclubglobal.com' LIMIT 1"
    )
    .fetch_optional(pool).await.ok()??;

    let refresh_token = row.refresh_token?;
    let client_id  = std::env::var("ZOHO_CLIENT_ID").ok()?;
    let client_secret = std::env::var("ZOHO_CLIENT_SECRET").ok()?;

    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().ok()?;
    let resp = client
        .post("https://accounts.zoho.com/oauth/v2/token")
        .form(&[
            ("refresh_token", refresh_token.as_str()),
            ("client_id",     client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type",    "refresh_token"),
        ])
        .send().await.ok()?;

    if !resp.status().is_success() {
        warn!("[MEET-WATCHER] Zoho token refresh failed: {}", resp.status());
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let token = body.get("access_token")?.as_str()?.to_string();

    // Persist to DB
    let _ = sqlx::query!(
        "UPDATE email_accounts SET access_token = ? WHERE email_address = 'nora@powerclubglobal.com'",
        token
    ).execute(pool).await;

    // Update cache
    *ZOHO_TOKEN_CACHE.lock().await = (token.clone(), Instant::now());
    Some(token)
}

async fn fetch_inbox_meets(token: &str, account_id: &str) -> Vec<(String, String)> {
    // Returns Vec<(message_id, meet_url)> for new invites
    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap_or_default();
    let url = format!("https://mail.zoho.com/api/accounts/{}/messages/view?limit=10&sortorder=false", account_id);
    let resp = match client.get(&url).header("Authorization", format!("Zoho-oauthtoken {}", token)).send().await {
        Ok(r) => r,
        Err(e) => { warn!("[MEET-WATCHER] Inbox fetch error: {}", e); return vec![]; }
    };
    if !resp.status().is_success() {
        warn!("[MEET-WATCHER] Inbox fetch HTTP {}", resp.status());
        return vec![];
    }
    let body: serde_json::Value = match resp.json().await {
        Ok(b) => b,
        Err(_) => return vec![],
    };
    let msgs = match body.get("data").and_then(|d| d.as_array()) {
        Some(a) => a.clone(),
        None => return vec![],
    };

    let re = regex::Regex::new(r"meet\.google\.com/([a-z]{3}-[a-z]{4}-[a-z]{3})").unwrap();
    let mut results = vec![];
    for msg in &msgs {
        let msg_id = match msg.get("messageId").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        let summary = msg.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        let subject  = msg.get("subject").and_then(|v| v.as_str()).unwrap_or("");
        // Only process Google Meet invite emails
        if !summary.contains("meet.google.com") && !subject.to_lowercase().contains("video call") {
            continue;
        }
        if let Some(cap) = re.find(summary) {
            results.push((msg_id, format!("https://{}", cap.as_str())));
        }
    }
    results
}

/// Spawned once at startup — polls Nora's inbox every 45 seconds and auto-joins new invites
pub async fn start_meet_watcher(deployment: DeploymentImpl) {
    tokio::spawn(async move {
        // Stagger startup so server fully initialises first
        tokio::time::sleep(Duration::from_secs(20)).await;
        info!("[MEET-WATCHER] Started — polling Nora's inbox every 45s");

        // Pre-populate seen IDs with current inbox to avoid joining stale meetings on boot
        let pool = deployment.db().pool.clone();
        if let Some(account_id) = get_zoho_account_id(&pool).await {
            if let Some(token) = get_zoho_token(&pool).await {
                for (msg_id, _) in fetch_inbox_meets(&token, &account_id).await {
                    SEEN_MESSAGE_IDS.lock().await.insert(msg_id);
                }
                info!("[MEET-WATCHER] Pre-seeded seen message IDs (won't re-join old invites)");
            }
        }

        loop {
            tokio::time::sleep(Duration::from_secs(45)).await;

            let pool = deployment.db().pool.clone();
            let account_id = match get_zoho_account_id(&pool).await {
                Some(id) => id,
                None => { warn!("[MEET-WATCHER] No Zoho account_id found"); continue; }
            };

            let token = match get_zoho_token(&pool).await {
                Some(t) => t,
                None => { warn!("[MEET-WATCHER] Could not obtain Zoho token"); continue; }
            };

            let invites = fetch_inbox_meets(&token, &account_id).await;
            if invites.is_empty() { continue; }

            let mut seen = SEEN_MESSAGE_IDS.lock().await;
            let active_urls: HashSet<String> = {
                let active = ACTIVE_MEETS.lock().await;
                active.values().map(|s| s.meet_url.clone()).collect()
            };

            for (msg_id, meet_url) in invites {
                if seen.contains(&msg_id) { continue; }
                seen.insert(msg_id.clone());

                if active_urls.contains(&meet_url) {
                    info!("[MEET-WATCHER] Already in meeting {}", meet_url);
                    continue;
                }

                info!("[MEET-WATCHER] New invite detected — joining {}", meet_url);
                drop(seen); // release lock before async join

                // Resolve project_id
                let project_id = sqlx::query_scalar::<_, String>(
                    "SELECT lower(hex(p.id)) FROM projects p \
                     JOIN project_members pm ON pm.project_id = p.id \
                     JOIN users u ON u.id = pm.user_id \
                     WHERE u.is_admin = 1 AND u.is_active = 1 \
                     ORDER BY p.created_at DESC LIMIT 1"
                )
                .fetch_one(&pool).await.unwrap_or_default();

                let req = JoinMeetRequest {
                    meet_url: meet_url.clone(),
                    project_id,
                    title: Some("Nora — Auto-joined Meet".to_string()),
                };
                match join_meet(axum::extract::State(deployment.clone()), Json(req)).await {
                    Ok(Json(resp)) => info!("[MEET-WATCHER] Joined session {}", resp.session_id),
                    Err((_, Json(e))) => warn!("[MEET-WATCHER] Failed to join {}: {:?}", meet_url, e),
                }

                seen = SEEN_MESSAGE_IDS.lock().await;
            }
        }
    });
}

async fn get_zoho_account_id(pool: &sqlx::SqlitePool) -> Option<String> {
    let row = sqlx::query!(
        "SELECT metadata FROM email_accounts WHERE email_address = 'nora@powerclubglobal.com' LIMIT 1"
    )
    .fetch_optional(pool).await.ok()??;
    let meta: serde_json::Value = serde_json::from_str(row.metadata.as_deref().unwrap_or("{}")).ok()?;
    meta.get("zoho_account_id")?.as_str().map(str::to_string)
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn meet_routes(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Start inbox watcher in background
    let dep = deployment.clone();
    tokio::spawn(async move { start_meet_watcher(dep).await });

    Router::new()
        .route("/nora/join-meet", post(join_meet))
        .route("/nora/meet/sessions", get(list_sessions))
        .route("/nora/meet/{id}/audio", post(receive_audio))
        .route("/nora/meet/{id}/next-tts", get(next_tts))
        .route("/nora/meet/{id}/transcript", get(transcript_stream))
        .route("/nora/meet/{id}/leave", post(leave_meet))
        .route("/nora/meet/{id}/left", post(bot_left))
}

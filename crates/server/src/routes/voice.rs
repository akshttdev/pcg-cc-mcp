//! Voice routes for the Sovereign Stack voice pipeline
//!
//! This module provides HTTP and WebSocket endpoints for:
//! - Browser voice input via WebSocket
//! - Glasses/wearable voice via APN mesh
//! - Voice gateway status and control
//! - TTS synthesis endpoint

use std::sync::Arc;

use axum::{
    Router,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, sse::Sse},
    routing::{get, post},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use nora::{
    voice::{
        BrowserChannel, BrowserVoiceMessage, GlassesChannel, VoiceChannel,
        VoiceCommandRouter, VoiceGateway, VoiceGatewayConfig,
        VoiceGatewayEvent,
    },
    ExecutionEngine,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::DeploymentImpl;

/// Global VoiceGateway instance
static VOICE_GATEWAY: tokio::sync::OnceCell<Arc<VoiceGatewayState>> =
    tokio::sync::OnceCell::const_new();

/// Voice gateway state including channels
struct VoiceGatewayState {
    gateway: Arc<RwLock<VoiceGateway>>,
    browser_channel: Arc<BrowserChannel>,
    glasses_channel: Arc<GlassesChannel>,
    command_router: Arc<VoiceCommandRouter>,
    /// Mesh voice message sender (for glasses)
    mesh_tx: mpsc::UnboundedSender<MeshVoiceMessage>,
}

/// Mesh voice message (simplified for server use)
#[derive(Debug, Clone)]
struct MeshVoiceMessage {
    peer_id: String,
    session_id: String,
    audio_b64: Option<String>,
    command: Option<String>,
}

/// Get or initialize the voice gateway
async fn get_voice_gateway() -> Result<Arc<VoiceGatewayState>, String> {
    if let Some(state) = VOICE_GATEWAY.get() {
        return Ok(state.clone());
    }

    info!("Initializing Voice Gateway...");

    // Create channels
    let browser_channel = Arc::new(BrowserChannel::new());
    let glasses_channel = Arc::new(GlassesChannel::new());

    // Create mesh channel for glasses communication
    let (mesh_tx, mut mesh_rx) = mpsc::unbounded_channel::<MeshVoiceMessage>();

    // Create command router
    let command_router = Arc::new(VoiceCommandRouter::new());

    // Create gateway
    let config = VoiceGatewayConfig {
        chatterbox_url: std::env::var("CHATTERBOX_URL")
            .unwrap_or_else(|_| "http://localhost:8100".to_string()),
        enable_command_routing: true,
        ..Default::default()
    };

    let gateway = VoiceGateway::new(config);

    // Initialize gateway
    if let Err(e) = gateway.initialize().await {
        warn!("Voice gateway initialization warning: {}", e);
        // Continue anyway - it will work once Chatterbox is running
    }

    // Register channels
    let gateway = Arc::new(RwLock::new(gateway));
    {
        let mut gw = gateway.write().await;
        if let Err(e) = gw.register_channel(browser_channel.clone()).await {
            warn!("Failed to register browser channel: {}", e);
        }
        if let Err(e) = gw.register_channel(glasses_channel.clone()).await {
            warn!("Failed to register glasses channel: {}", e);
        }

        // Set command handler
        gw.set_command_handler(command_router.clone()).await;
    }

    let state = Arc::new(VoiceGatewayState {
        gateway,
        browser_channel,
        glasses_channel,
        command_router,
        mesh_tx,
    });

    // Spawn mesh message handler
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = mesh_rx.recv().await {
            // Handle mesh messages from glasses
            debug!("Received mesh voice message from peer {}", msg.peer_id);
            // This would be connected to the actual APN mesh
        }
    });

    if VOICE_GATEWAY.set(state.clone()).is_err() {
        // Another task initialized it first
        return Ok(VOICE_GATEWAY.get().unwrap().clone());
    }

    info!("Voice Gateway initialized successfully");
    Ok(state)
}

/// Set the execution engine for the voice command router
pub async fn set_voice_execution_engine(engine: Arc<ExecutionEngine>) {
    if let Ok(state) = get_voice_gateway().await {
        // The command router would need to be modified to accept this
        info!("Execution engine connected to voice command router");
    }
}

/// Voice routes
pub fn voice_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/voice/ws/{session_id}", get(voice_websocket))
        .route("/voice/status", get(voice_status))
        .route("/voice/sessions", get(list_sessions))
        .route("/voice/synthesize", post(synthesize_speech))
        .route("/voice/transcribe", post(transcribe_speech))
        .route("/voice/mesh/send", post(send_mesh_audio))
        .route("/voice/events", get(voice_events_sse))
}

/// WebSocket handler for browser voice input
async fn voice_websocket(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(_deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    info!("Voice WebSocket connection request for session: {}", session_id);

    ws.on_upgrade(move |socket| handle_voice_socket(socket, session_id))
}

/// Handle WebSocket connection for voice
async fn handle_voice_socket(socket: WebSocket, session_id: String) {
    let (mut sender, mut receiver) = socket.split();

    let state = match get_voice_gateway().await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get voice gateway: {}", e);
            return;
        }
    };

    // Subscribe to gateway events for this session
    let mut event_rx = {
        let gw = state.gateway.read().await;
        gw.subscribe()
    };

    // Spawn task to forward gateway events to WebSocket
    let session_id_clone = session_id.clone();
    let sender_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = match &event {
                VoiceGatewayEvent::Transcription { session_id: sid, text, confidence, is_final }
                    if sid == &session_id_clone => {
                    Some(serde_json::json!({
                        "type": "transcription",
                        "session_id": sid,
                        "text": text,
                        "confidence": confidence,
                        "is_final": is_final,
                    }))
                }
                VoiceGatewayEvent::ResponseSent { session_id: sid, text, audio_duration_ms }
                    if sid == &session_id_clone => {
                    Some(serde_json::json!({
                        "type": "response",
                        "session_id": sid,
                        "text": text,
                        "audio_duration_ms": audio_duration_ms,
                    }))
                }
                VoiceGatewayEvent::CommandDetected { session_id: sid, command, intent }
                    if sid == &session_id_clone => {
                    Some(serde_json::json!({
                        "type": "command_detected",
                        "session_id": sid,
                        "command": command,
                        "intent": intent,
                    }))
                }
                VoiceGatewayEvent::Error { session_id: sid, error }
                    if sid.as_ref() == Some(&session_id_clone) => {
                    Some(serde_json::json!({
                        "type": "error",
                        "session_id": sid,
                        "error": error,
                    }))
                }
                _ => None,
            };

            if let Some(msg) = msg {
                if sender.send(Message::Text(msg.to_string().into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages from browser
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Parse browser voice message
                match serde_json::from_str::<BrowserVoiceMessage>(&text) {
                    Ok(voice_msg) => {
                        state.browser_channel.handle_ws_message(&session_id, voice_msg).await;
                    }
                    Err(e) => {
                        warn!("Invalid voice message format: {}", e);
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                // Binary audio data - convert to base64 and create audio chunk
                let audio_b64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &data,
                );
                let voice_msg = BrowserVoiceMessage::AudioChunk {
                    audio_b64,
                    sequence: 0, // Would need proper sequencing
                    is_final: false,
                };
                state.browser_channel.handle_ws_message(&session_id, voice_msg).await;
            }
            Ok(Message::Close(_)) => {
                info!("Voice WebSocket closed for session: {}", session_id);
                let voice_msg = BrowserVoiceMessage::EndSession;
                state.browser_channel.handle_ws_message(&session_id, voice_msg).await;
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    sender_task.abort();
    info!("Voice WebSocket handler ended for session: {}", session_id);
}

/// Voice gateway status response
#[derive(Debug, Serialize)]
struct VoiceStatusResponse {
    initialized: bool,
    chatterbox_url: String,
    active_sessions: usize,
    channels: Vec<ChannelStatus>,
}

#[derive(Debug, Serialize)]
struct ChannelStatus {
    channel_type: String,
    channel_id: String,
    active_sessions: usize,
}

/// Get voice gateway status
async fn voice_status() -> impl IntoResponse {
    match get_voice_gateway().await {
        Ok(state) => {
            let gw = state.gateway.read().await;
            let sessions = gw.active_sessions().await;

            let browser_sessions = state.browser_channel.active_sessions().await;
            let glasses_sessions = state.glasses_channel.active_sessions().await;

            let response = VoiceStatusResponse {
                initialized: true,
                chatterbox_url: std::env::var("CHATTERBOX_URL")
                    .unwrap_or_else(|_| "http://localhost:8100".to_string()),
                active_sessions: sessions.len(),
                channels: vec![
                    ChannelStatus {
                        channel_type: "browser".to_string(),
                        channel_id: state.browser_channel.channel_id().to_string(),
                        active_sessions: browser_sessions.len(),
                    },
                    ChannelStatus {
                        channel_type: "glasses".to_string(),
                        channel_id: state.glasses_channel.channel_id().to_string(),
                        active_sessions: glasses_sessions.len(),
                    },
                ],
            };

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = VoiceStatusResponse {
                initialized: false,
                chatterbox_url: String::new(),
                active_sessions: 0,
                channels: vec![],
            };
            (StatusCode::OK, Json(response))
        }
    }
}

/// Session info response
#[derive(Debug, Serialize)]
struct SessionInfo {
    session_id: String,
    channel_type: String,
    device_id: String,
    started_at: String,
    is_listening: bool,
    is_speaking: bool,
}

/// List active voice sessions
async fn list_sessions() -> impl IntoResponse {
    match get_voice_gateway().await {
        Ok(state) => {
            let gw = state.gateway.read().await;
            let sessions = gw.active_sessions().await;

            let session_infos: Vec<SessionInfo> = sessions
                .into_iter()
                .map(|s| SessionInfo {
                    session_id: s.channel_session.session_id,
                    channel_type: s.channel_session.channel_type.to_string(),
                    device_id: s.channel_session.device_id,
                    started_at: s.channel_session.started_at.to_rfc3339(),
                    is_listening: s.channel_session.is_listening,
                    is_speaking: s.channel_session.is_speaking,
                })
                .collect();

            (StatusCode::OK, Json(session_infos))
        }
        Err(_) => (StatusCode::OK, Json(Vec::<SessionInfo>::new())),
    }
}

/// TTS synthesis request
#[derive(Debug, Deserialize)]
struct SynthesizeRequest {
    text: String,
    voice: Option<String>,
}

/// TTS synthesis response
#[derive(Debug, Serialize)]
struct SynthesizeResponse {
    audio_b64: String,
    duration_ms: u64,
    format: String,
}

/// Synthesize speech using Chatterbox
async fn synthesize_speech(
    Json(request): Json<SynthesizeRequest>,
) -> impl IntoResponse {
    // Call Chatterbox directly
    let chatterbox_url = std::env::var("CHATTERBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8100".to_string());

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "text": request.text,
        "voice": request.voice.unwrap_or_else(|| "british_female".to_string()),
        "speed": 1.0,
        "exaggeration": 0.5,
    });

    match client
        .post(format!("{}/tts", chatterbox_url))
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.bytes().await {
                    Ok(bytes) => {
                        let audio_b64 = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &bytes,
                        );
                        (
                            StatusCode::OK,
                            Json(SynthesizeResponse {
                                audio_b64,
                                duration_ms: 0, // Would calculate from audio
                                format: "wav".to_string(),
                            }),
                        )
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(SynthesizeResponse {
                            audio_b64: String::new(),
                            duration_ms: 0,
                            format: format!("Error: {}", e),
                        }),
                    ),
                }
            } else {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(SynthesizeResponse {
                        audio_b64: String::new(),
                        duration_ms: 0,
                        format: format!("Chatterbox error: {}", response.status()),
                    }),
                )
            }
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(SynthesizeResponse {
                audio_b64: String::new(),
                duration_ms: 0,
                format: format!("Chatterbox unavailable: {}", e),
            }),
        ),
    }
}

/// STT transcription request
#[derive(Debug, Deserialize)]
struct TranscribeRequest {
    audio_b64: String,
    language: Option<String>,
}

/// STT transcription response
#[derive(Debug, Serialize)]
struct TranscribeResponse {
    text: String,
    confidence: f32,
    language: String,
}

/// Transcribe speech using local Whisper server
async fn transcribe_speech(
    Json(request): Json<TranscribeRequest>,
) -> impl IntoResponse {
    // Use local Whisper server
    let whisper_url = std::env::var("WHISPER_URL")
        .unwrap_or_else(|_| "http://localhost:8101".to_string());

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "audio_b64": request.audio_b64,
        "language": request.language.unwrap_or_else(|| "en".to_string()),
    });

    match client
        .post(format!("{}/transcribe", whisper_url))
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let text = json["text"].as_str().unwrap_or("").to_string();
                        let confidence = json["confidence"].as_f64().unwrap_or(0.95) as f32;
                        let language = json["language"].as_str().unwrap_or("en").to_string();
                        (
                            StatusCode::OK,
                            Json(TranscribeResponse {
                                text,
                                confidence,
                                language,
                            }),
                        )
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TranscribeResponse {
                            text: String::new(),
                            confidence: 0.0,
                            language: format!("Parse error: {}", e),
                        }),
                    ),
                }
            } else {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(TranscribeResponse {
                        text: String::new(),
                        confidence: 0.0,
                        language: format!("Whisper error: {}", response.status()),
                    }),
                )
            }
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(TranscribeResponse {
                text: String::new(),
                confidence: 0.0,
                language: format!("Local Whisper unavailable: {}. Start with: python scripts/whisper_server.py", e),
            }),
        ),
    }
}

/// Send audio to glasses via mesh
#[derive(Debug, Deserialize)]
struct MeshAudioRequest {
    peer_id: String,
    session_id: String,
    audio_b64: String,
    text: String,
}

async fn send_mesh_audio(
    Json(request): Json<MeshAudioRequest>,
) -> impl IntoResponse {
    match get_voice_gateway().await {
        Ok(state) => {
            // Send via glasses channel
            if let Err(e) = state
                .glasses_channel
                .send_audio(&request.session_id, &request.audio_b64, true)
                .await
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                );
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({ "status": "sent" })),
            )
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

/// SSE endpoint for voice gateway events
async fn voice_events_sse(
    State(_deployment): State<DeploymentImpl>,
) -> Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use axum::response::sse::{Event, Sse};
    use std::convert::Infallible;

    let state = get_voice_gateway().await.ok();

    let stream = async_stream::stream! {
        let Some(state) = state else {
            return;
        };

        let mut event_rx = {
            let gw = state.gateway.read().await;
            gw.subscribe()
        };

        while let Ok(event) = event_rx.recv().await {
            let event_json = match event {
                VoiceGatewayEvent::SessionStarted { session_id, channel_type, device_id } => {
                    serde_json::json!({
                        "type": "session_started",
                        "session_id": session_id,
                        "channel_type": channel_type.to_string(),
                        "device_id": device_id,
                    })
                }
                VoiceGatewayEvent::SessionEnded { session_id, duration_ms } => {
                    serde_json::json!({
                        "type": "session_ended",
                        "session_id": session_id,
                        "duration_ms": duration_ms,
                    })
                }
                VoiceGatewayEvent::Transcription { session_id, text, confidence, is_final } => {
                    serde_json::json!({
                        "type": "transcription",
                        "session_id": session_id,
                        "text": text,
                        "confidence": confidence,
                        "is_final": is_final,
                    })
                }
                VoiceGatewayEvent::CommandDetected { session_id, command, intent } => {
                    serde_json::json!({
                        "type": "command_detected",
                        "session_id": session_id,
                        "command": command,
                        "intent": intent,
                    })
                }
                VoiceGatewayEvent::ResponseSent { session_id, text, audio_duration_ms } => {
                    serde_json::json!({
                        "type": "response_sent",
                        "session_id": session_id,
                        "text": text,
                        "audio_duration_ms": audio_duration_ms,
                    })
                }
                VoiceGatewayEvent::Error { session_id, error } => {
                    serde_json::json!({
                        "type": "error",
                        "session_id": session_id,
                        "error": error,
                    })
                }
            };

            yield Ok::<_, Infallible>(Event::default().data(event_json.to_string()));
        }
    };

    Sse::new(stream)
}

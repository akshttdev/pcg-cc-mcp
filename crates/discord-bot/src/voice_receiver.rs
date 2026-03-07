//! Songbird voice event handler — captures per-user audio, runs the full STT → agent → TTS pipeline

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use songbird::events::{CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::audio;
use crate::session::{ActiveAgent, DiscordVoiceSession, TranscriptEvent, UserAudioBuffer};

/// Shared state between the Receiver and the session manager
pub struct ReceiverState {
    pub session: Arc<Mutex<DiscordVoiceSession>>,
    /// Audio buffers per SSRC (Discord RTP stream ID)
    pub buffers: DashMap<u32, UserAudioBuffer>,
    /// Maps SSRC → Discord user ID (populated on SpeakingStateUpdate)
    pub ssrc_map: DashMap<u32, String>,
    /// Event broadcast for SSE streaming
    pub event_tx: broadcast::Sender<TranscriptEvent>,
    /// Sqlx pool for persisting segments
    pub pool: sqlx::SqlitePool,
    /// Songbird call handle — used to play TTS audio back into the channel
    pub call_lock: Arc<Mutex<songbird::Call>>,
}

/// Songbird event handler — processes both speaking state updates and audio ticks
pub struct VoiceReceiver {
    pub state: Arc<ReceiverState>,
}

#[async_trait]
impl VoiceEventHandler for VoiceReceiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            // Track SSRC → Discord user_id mapping
            EventContext::SpeakingStateUpdate(data) => {
                if let Some(user_id) = data.user_id {
                    let uid_str = user_id.0.to_string();
                    self.state.ssrc_map.insert(data.ssrc, uid_str.clone());
                    let mut session = self.state.session.lock().await;
                    session.ssrc_to_user.insert(data.ssrc, uid_str);
                    debug!("SSRC {} mapped to Discord user {}", data.ssrc, user_id.0);
                }
            }

            // Process audio: accumulate per-user and flush when silence detected
            EventContext::VoiceTick(tick) => {
                let elapsed_ms = {
                    let session = self.state.session.lock().await;
                    session.elapsed_ms()
                };

                for ssrc in &tick.silent {
                    if let Some(mut buf) = self.state.buffers.get_mut(ssrc) {
                        buf.silent_ticks += 1;
                    }
                }

                for (ssrc, voice_data) in &tick.speaking {
                    if let Some(decoded) = &voice_data.decoded_voice {
                        let mut buf = self.state.buffers.entry(*ssrc).or_default();
                        buf.push_samples(decoded, elapsed_ms);
                    }
                }

                let ready_ssrcs: Vec<u32> = self
                    .state
                    .buffers
                    .iter()
                    .filter(|e| e.value().is_ready_to_flush() && !e.value().samples.is_empty())
                    .map(|e| *e.key())
                    .collect();

                for ssrc in ready_ssrcs {
                    let (samples, start_time_ms) = {
                        let mut buf = match self.state.buffers.get_mut(&ssrc) {
                            Some(b) => b,
                            None => continue,
                        };
                        let start = buf.start_time_ms;
                        (buf.take(), start)
                    };

                    let user_id = self
                        .state
                        .ssrc_map
                        .get(&ssrc)
                        .map(|r| r.clone())
                        .unwrap_or_else(|| format!("unknown-{}", ssrc));

                    let state = Arc::clone(&self.state);
                    tokio::spawn(async move {
                        process_audio_chunk(state, ssrc, samples, user_id, start_time_ms).await;
                    });
                }
            }

            _ => {}
        }

        None
    }
}

/// Process a completed audio chunk: STT → wake word → agent → TTS playback → persist
async fn process_audio_chunk(
    state: Arc<ReceiverState>,
    ssrc: u32,
    samples: Vec<i16>,
    user_id: String,
    start_time_ms: i64,
) {
    let end_time_ms = {
        let session = state.session.lock().await;
        session.elapsed_ms()
    };

    let wav_bytes = match audio::encode_wav(&samples) {
        Ok(b) => b,
        Err(e) => {
            warn!("WAV encode failed for ssrc {}: {}", ssrc, e);
            return;
        }
    };

    let transcript = match audio::transcribe_wav(wav_bytes).await {
        Ok(Some(t)) if !t.trim().is_empty() => t,
        Ok(_) => return,
        Err(e) => {
            warn!("STT failed for ssrc {}: {}", ssrc, e);
            return;
        }
    };

    info!("Discord transcript [{}]: {}", user_id, &transcript[..transcript.len().min(80)]);

    let (meeting_session_id, segment_index, agent, project_id, server_port, speaker_label) = {
        let mut session = state.session.lock().await;
        session.segment_count += 1;
        let idx = session.segment_count;
        let label = session
            .user_names
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| format!("Speaker {}", &user_id[..user_id.len().min(6)]));
        (
            session.meeting_session_id.clone(),
            idx,
            session.agent,
            session.project_id.clone(),
            session.server_port,
            label,
        )
    };

    let addressed = audio::detect_wake_word(&transcript, agent);
    let is_addressed = addressed.is_some();

    // Call agent and get text response
    let agent_response = if let Some(command) = &addressed {
        let cmd = if command.is_empty() { &transcript } else { command.as_str() };
        match audio::call_agent(server_port, agent, cmd, &project_id, &meeting_session_id).await {
            Ok(response) => Some(response),
            Err(e) => {
                warn!("Agent call failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // TTS playback: synthesize the agent response and play it into the voice channel
    if let Some(ref response_text) = agent_response {
        let call_lock = Arc::clone(&state.call_lock);
        let text = response_text.clone();
        tokio::spawn(async move {
            match audio::synthesize_and_play(call_lock, &text).await {
                Ok(()) => info!("TTS played back into voice channel"),
                Err(e) => warn!("TTS playback failed: {}", e),
            }
        });
    }

    // Persist user segment to DB
    let segment_id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        r#"INSERT INTO meeting_segments (
            id, meeting_session_id, segment_index, speaker_label,
            text, confidence, start_time_ms, end_time_ms, is_topsi_addressed, metadata
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
    )
    .bind(&segment_id)
    .bind(&meeting_session_id)
    .bind(segment_index)
    .bind(&speaker_label)
    .bind(&transcript)
    .bind(0.9_f64)
    .bind(start_time_ms)
    .bind(end_time_ms)
    .bind(is_addressed)
    .bind(
        agent_response
            .as_ref()
            .map(|r| serde_json::json!({ "agent_response": r, "discord_user_id": user_id }).to_string()),
    )
    .execute(&state.pool)
    .await;

    // Also persist agent response as its own segment
    if let Some(ref response) = agent_response {
        let resp_segment_id = Uuid::new_v4().to_string();
        let agent_name = agent.name().to_string();
        let _ = sqlx::query(
            r#"INSERT INTO meeting_segments (
                id, meeting_session_id, segment_index, speaker_label,
                text, confidence, start_time_ms, end_time_ms, is_topsi_addressed, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
        )
        .bind(&resp_segment_id)
        .bind(&meeting_session_id)
        .bind(segment_index + 1)
        .bind(&agent_name)
        .bind(response)
        .bind(1.0_f64)
        .bind(end_time_ms)
        .bind(end_time_ms)
        .bind(false)
        .bind(serde_json::json!({ "is_agent_response": true }).to_string())
        .execute(&state.pool)
        .await;
    }

    // Broadcast transcript event for SSE streaming to dashboard
    let event = TranscriptEvent {
        meeting_session_id,
        segment_index,
        speaker_label,
        speaker_discord_id: user_id,
        text: transcript,
        confidence: 0.9,
        start_time_ms,
        end_time_ms,
        is_agent_addressed: is_addressed,
        agent_response,
        timestamp: Utc::now(),
    };
    let _ = state.event_tx.send(event);
}

/// Register all necessary event handlers on the given call handler
pub async fn register_handlers(handler: &mut songbird::Call, state: Arc<ReceiverState>) {
    handler.add_global_event(
        CoreEvent::SpeakingStateUpdate.into(),
        VoiceReceiver { state: Arc::clone(&state) },
    );
    handler.add_global_event(
        CoreEvent::VoiceTick.into(),
        VoiceReceiver { state },
    );
}

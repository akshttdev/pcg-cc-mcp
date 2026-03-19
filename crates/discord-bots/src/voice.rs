use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use byteorder::{LittleEndian, WriteBytesExt};
use serenity::all::*;
use songbird::{
    input::{Input, RawAdapter},
    Call, CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::backend::BackendClient;

/// Configuration for voice services
#[derive(Debug, Clone)]
pub struct VoiceServiceConfig {
    pub tts_url: String,
    pub stt_url: String,
    pub silence_threshold_secs: f64,
    pub tts_voice: String,
}

impl Default for VoiceServiceConfig {
    fn default() -> Self {
        Self {
            tts_url: std::env::var("CHATTERBOX_URL")
                .unwrap_or_else(|_| "http://localhost:8100".to_string()),
            stt_url: std::env::var("WHISPER_URL")
                .unwrap_or_else(|_| "http://localhost:8101".to_string()),
            silence_threshold_secs: 1.5,
            tts_voice: "british_female".to_string(),
        }
    }
}

/// Active meeting session state
#[derive(Debug, Clone)]
pub struct MeetingSession {
    pub session_id: String,
    pub title: String,
}

/// Manages voice connections with meeting mode
#[derive(Clone)]
pub struct VoiceManager {
    pub config: VoiceServiceConfig,
    pub backend: Arc<BackendClient>,
    pub agent_name: String,
    client: reqwest::Client,
    audio_buffers: Arc<RwLock<HashMap<u32, AudioBuffer>>>,
    ssrc_map: Arc<RwLock<HashMap<u32, UserId>>>,
    /// Active meeting session (set when in a voice channel)
    active_meeting: Arc<RwLock<Option<MeetingSession>>>,
    /// Chunk counter for meeting audio
    chunk_counter: Arc<AtomicU32>,
}

struct AudioBuffer {
    samples: Vec<i16>,
    last_packet_time: std::time::Instant,
}

impl VoiceManager {
    pub fn new(backend: Arc<BackendClient>, agent_name: &str, config: VoiceServiceConfig) -> Self {
        Self {
            config,
            backend,
            agent_name: agent_name.to_string(),
            client: reqwest::Client::new(),
            audio_buffers: Arc::new(RwLock::new(HashMap::new())),
            ssrc_map: Arc::new(RwLock::new(HashMap::new())),
            active_meeting: Arc::new(RwLock::new(None)),
            chunk_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Join a voice channel and start a meeting session
    pub async fn join_channel(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<Arc<Mutex<Call>>, String> {
        let manager = songbird::get(ctx)
            .await
            .ok_or("Songbird voice client not initialized")?;

        let call = manager
            .join(guild_id, channel_id)
            .await
            .map_err(|e| format!("Failed to join voice channel: {:?}", e))?;

        info!(
            "[{} VOICE] Joined voice channel {} — entering meeting mode",
            self.agent_name, channel_id
        );

        // Start a meeting session via the backend
        let meeting_title = format!(
            "Discord Voice - {} - {}",
            self.agent_name,
            chrono::Utc::now().format("%Y-%m-%d %H:%M")
        );

        match self
            .backend
            .start_meeting("default", Some(&meeting_title))
            .await
        {
            Ok(resp) => {
                info!(
                    "[{} VOICE] Meeting started: {} ({})",
                    self.agent_name, resp.session_id, resp.title
                );
                *self.active_meeting.write().await = Some(MeetingSession {
                    session_id: resp.session_id,
                    title: resp.title,
                });
                self.chunk_counter.store(0, Ordering::Relaxed);
            }
            Err(e) => {
                warn!(
                    "[{} VOICE] Could not start meeting session: {} — running in direct chat mode",
                    self.agent_name, e
                );
            }
        }

        // Register voice event handlers
        {
            let mut call_lock = call.lock().await;

            call_lock.add_global_event(
                CoreEvent::SpeakingStateUpdate.into(),
                SpeakingHandler {
                    agent_name: self.agent_name.clone(),
                    ssrc_map: self.ssrc_map.clone(),
                },
            );

            call_lock.add_global_event(
                CoreEvent::VoiceTick.into(),
                VoiceTickHandler {
                    voice_manager: self.clone(),
                    call: call.clone(),
                },
            );
        }

        Ok(call)
    }

    /// Leave voice channel and end the meeting session
    pub async fn leave_channel(
        &self,
        ctx: &Context,
        guild_id: GuildId,
    ) -> Result<Option<crate::backend::EndMeetingResponse>, String> {
        let manager = songbird::get(ctx).await.ok_or("Songbird not initialized")?;

        manager
            .leave(guild_id)
            .await
            .map_err(|e| format!("Failed to leave: {:?}", e))?;

        // End the meeting session
        let end_result = if let Some(meeting) = self.active_meeting.write().await.take() {
            info!(
                "[{} VOICE] Ending meeting session {}",
                self.agent_name, meeting.session_id
            );
            match self.backend.end_meeting(&meeting.session_id, true).await {
                Ok(resp) => {
                    info!(
                        "[{} VOICE] Meeting ended — {} segments, {} participants, {}s duration",
                        self.agent_name,
                        resp.segment_count,
                        resp.participant_count,
                        resp.duration_seconds
                    );
                    Some(resp)
                }
                Err(e) => {
                    warn!("[{} VOICE] Failed to end meeting: {}", self.agent_name, e);
                    None
                }
            }
        } else {
            None
        };

        self.audio_buffers.write().await.clear();
        self.ssrc_map.write().await.clear();

        info!("[{} VOICE] Left voice channel", self.agent_name);
        Ok(end_result)
    }

    /// Synthesize speech using Chatterbox TTS
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .post(format!("{}/tts", self.config.tts_url))
            .json(&serde_json::json!({
                "text": text,
                "voice": self.config.tts_voice,
                "speed": 1.0,
                "exaggeration": 0.5
            }))
            .send()
            .await
            .map_err(|e| format!("TTS request failed: {}", e))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("TTS failed: {}", body));
        }

        Ok(response
            .bytes()
            .await
            .map_err(|e| format!("TTS read error: {}", e))?
            .to_vec())
    }

    /// Meeting mode pipeline:
    /// 1. Send audio to backend for transcription + wake word detection
    /// 2. Backend logs to meeting transcript automatically
    /// 3. Only respond if the agent's wake word was detected
    async fn process_meeting_audio(
        &self,
        pcm_samples: Vec<i16>,
        call: Arc<Mutex<Call>>,
        user_id: Option<UserId>,
    ) {
        let meeting = {
            let m = self.active_meeting.read().await;
            m.clone()
        };

        // Convert stereo 48kHz to mono 16kHz WAV for the backend
        let mono_16k = downsample_stereo_to_mono(&pcm_samples, 48000, 16000);
        let wav_bytes = pcm_to_wav(&mono_16k, 16000, 1);
        let audio_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &wav_bytes);

        let chunk_index = self.chunk_counter.fetch_add(1, Ordering::Relaxed);
        let duration_ms = (mono_16k.len() as u32 * 1000) / 16000;

        if let Some(meeting) = meeting {
            // MEETING MODE: send to backend meeting API
            // Backend handles: STT, wake word detection, transcript storage
            match self
                .backend
                .meeting_audio_chunk(&meeting.session_id, &audio_b64, chunk_index, duration_ms)
                .await
            {
                Ok(resp) => {
                    let speaker = resp.speaker_label.as_deref().unwrap_or("Unknown");

                    if !resp.text.is_empty() {
                        info!(
                            "[{} VOICE] [{}] {}: \"{}\"{}",
                            self.agent_name,
                            meeting.session_id,
                            speaker,
                            resp.text,
                            if resp.is_topsi_addressed {
                                " [ADDRESSED]"
                            } else {
                                ""
                            }
                        );
                    }

                    // Only speak if the agent was addressed by wake word
                    if resp.is_topsi_addressed {
                        if let Some(response_text) = &resp.topsi_response {
                            info!(
                                "[{} VOICE] Responding: \"{}\"",
                                self.agent_name,
                                if response_text.len() > 100 {
                                    format!("{}...", &response_text[..97])
                                } else {
                                    response_text.clone()
                                }
                            );

                            // Check if backend already synthesized audio
                            if let Some(audio_b64) = &resp.topsi_audio_response {
                                if let Ok(audio) = base64::Engine::decode(
                                    &base64::engine::general_purpose::STANDARD,
                                    audio_b64,
                                ) {
                                    let input = Input::from(RawAdapter::new(
                                        std::io::Cursor::new(audio),
                                        48000,
                                        2,
                                    ));
                                    let mut call_lock = call.lock().await;
                                    call_lock.play_input(input);
                                    return;
                                }
                            }

                            // Otherwise synthesize locally
                            if let Ok(audio_bytes) = self.synthesize(response_text).await {
                                let input = Input::from(RawAdapter::new(
                                    std::io::Cursor::new(audio_bytes),
                                    48000,
                                    2,
                                ));
                                let mut call_lock = call.lock().await;
                                call_lock.play_input(input);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "[{} VOICE] Meeting audio chunk failed: {}",
                        self.agent_name, e
                    );
                }
            }
        } else {
            // FALLBACK: No meeting session — direct chat mode (old behavior)
            // Transcribe locally and chat directly
            self.process_direct_chat(pcm_samples, call, user_id).await;
        }
    }

    /// Direct chat mode (fallback when meeting API is unavailable)
    async fn process_direct_chat(
        &self,
        pcm_samples: Vec<i16>,
        call: Arc<Mutex<Call>>,
        user_id: Option<UserId>,
    ) {
        let mono_16k = downsample_stereo_to_mono(&pcm_samples, 48000, 16000);
        let wav_bytes = pcm_to_wav(&mono_16k, 16000, 1);
        let audio_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &wav_bytes);

        // Transcribe via local Whisper
        let response = match self
            .client
            .post(format!("{}/transcribe", self.config.stt_url))
            .json(&serde_json::json!({
                "audio_b64": audio_b64,
                "language": "en"
            }))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => return,
        };

        let result: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(_) => return,
        };

        let text = result["text"].as_str().unwrap_or("").trim().to_string();
        if text.is_empty() {
            return;
        }

        // In direct mode, still check for wake word locally
        let wake_patterns = if self.agent_name == "Nora" {
            vec!["nora", "nor a"]
        } else {
            vec!["topsi", "topsy", "top see", "topsie"]
        };

        let text_lower = text.to_lowercase();
        let is_addressed = wake_patterns.iter().any(|p| text_lower.contains(p));

        info!(
            "[{} VOICE] \"{}\"{}",
            self.agent_name,
            text,
            if is_addressed { " [ADDRESSED]" } else { "" }
        );

        if !is_addressed {
            return; // Silent observer — don't respond
        }

        let session_id = format!(
            "discord-voice-{}-{}",
            self.agent_name.to_lowercase(),
            user_id
                .map(|u| u.get().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );

        let response_text = if self.agent_name == "Nora" {
            self.backend
                .chat_nora(&text, &session_id)
                .await
                .ok()
                .map(|r| r.content)
        } else {
            self.backend
                .chat_topsi(&text, &session_id)
                .await
                .ok()
                .map(|r| r.content)
        };

        if let Some(response_text) = response_text {
            if let Ok(audio_bytes) = self.synthesize(&response_text).await {
                let input =
                    Input::from(RawAdapter::new(std::io::Cursor::new(audio_bytes), 48000, 2));
                let mut call_lock = call.lock().await;
                call_lock.play_input(input);
            }
        }
    }

    /// Get the active meeting session ID (for use by bot handlers)
    pub async fn active_session_id(&self) -> Option<String> {
        self.active_meeting
            .read()
            .await
            .as_ref()
            .map(|m| m.session_id.clone())
    }
}

// ============================================================================
// Event Handlers
// ============================================================================

struct SpeakingHandler {
    agent_name: String,
    ssrc_map: Arc<RwLock<HashMap<u32, UserId>>>,
}

#[async_trait::async_trait]
impl VoiceEventHandler for SpeakingHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::SpeakingStateUpdate(speaking) = ctx {
            if let Some(user_id) = speaking.user_id {
                let discord_user_id = UserId::new(user_id.0.into());
                self.ssrc_map
                    .write()
                    .await
                    .insert(speaking.ssrc, discord_user_id);
                info!(
                    "[{} VOICE] SSRC {} -> user {}",
                    self.agent_name, speaking.ssrc, discord_user_id
                );
            }
        }
        None
    }
}

struct VoiceTickHandler {
    voice_manager: VoiceManager,
    call: Arc<Mutex<Call>>,
}

#[async_trait::async_trait]
impl VoiceEventHandler for VoiceTickHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::VoiceTick(tick) = ctx {
            for (ssrc, data) in &tick.speaking {
                if let Some(decoded) = &data.decoded_voice {
                    let mut buffers = self.voice_manager.audio_buffers.write().await;
                    let buf = buffers.entry(*ssrc).or_insert_with(|| AudioBuffer {
                        samples: Vec::new(),
                        last_packet_time: std::time::Instant::now(),
                    });
                    buf.samples.extend_from_slice(decoded);
                    buf.last_packet_time = std::time::Instant::now();
                }
            }

            // Silence detection
            let ssrcs_to_process: Vec<u32> = {
                let buffers = self.voice_manager.audio_buffers.read().await;
                buffers
                    .iter()
                    .filter(|(ssrc, buf)| {
                        let silence = buf.last_packet_time.elapsed().as_secs_f64();
                        silence > self.voice_manager.config.silence_threshold_secs
                            && buf.samples.len() > 24000
                            && !tick.speaking.contains_key(ssrc)
                    })
                    .map(|(ssrc, _)| *ssrc)
                    .collect()
            };

            for ssrc in ssrcs_to_process {
                let samples = {
                    let mut buffers = self.voice_manager.audio_buffers.write().await;
                    if let Some(buf) = buffers.remove(&ssrc) {
                        buf.samples
                    } else {
                        continue;
                    }
                };

                let user_id = {
                    let map = self.voice_manager.ssrc_map.read().await;
                    map.get(&ssrc).copied()
                };

                let vm = self.voice_manager.clone();
                let call = self.call.clone();
                tokio::spawn(async move {
                    vm.process_meeting_audio(samples, call, user_id).await;
                });
            }
        }
        None
    }
}

// ============================================================================
// Audio utilities
// ============================================================================

fn downsample_stereo_to_mono(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    let ratio = from_rate / to_rate;
    let mut mono = Vec::with_capacity(samples.len() / (2 * ratio as usize));
    let mut i = 0;
    while i + 1 < samples.len() {
        let left = samples[i] as i32;
        let right = samples[i + 1] as i32;
        mono.push(((left + right) / 2) as i16);
        i += 2 * ratio as usize;
    }
    mono
}

fn pcm_to_wav(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;
    let mut buf = Vec::with_capacity(file_size as usize + 8);

    buf.extend_from_slice(b"RIFF");
    let _ = buf.write_u32::<LittleEndian>(file_size);
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    let _ = buf.write_u32::<LittleEndian>(16);
    let _ = buf.write_u16::<LittleEndian>(1);
    let _ = buf.write_u16::<LittleEndian>(channels);
    let _ = buf.write_u32::<LittleEndian>(sample_rate);
    let _ = buf.write_u32::<LittleEndian>(sample_rate * channels as u32 * 2);
    let _ = buf.write_u16::<LittleEndian>(channels * 2);
    let _ = buf.write_u16::<LittleEndian>(16);
    buf.extend_from_slice(b"data");
    let _ = buf.write_u32::<LittleEndian>(data_size);
    for sample in samples {
        let _ = buf.write_i16::<LittleEndian>(*sample);
    }
    buf
}

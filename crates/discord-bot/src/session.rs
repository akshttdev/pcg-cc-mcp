//! Discord voice session state and event types

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Which PCG agent is present in this session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveAgent {
    Nora,
    Topsi,
}

impl ActiveAgent {
    pub fn name(&self) -> &'static str {
        match self {
            ActiveAgent::Nora => "Nora",
            ActiveAgent::Topsi => "Topsi",
        }
    }

    /// Wake words that trigger this agent
    pub fn wake_words(&self) -> &'static [&'static str] {
        match self {
            ActiveAgent::Nora => &["nora", "nor a", "no ra"],
            ActiveAgent::Topsi => &["topsi", "topsy", "top see", "topsie", "topsey"],
        }
    }
}

/// A transcript event streamed to the dashboard via SSE
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEvent {
    pub meeting_session_id: String,
    pub segment_index: i32,
    pub speaker_label: String,
    pub speaker_discord_id: String,
    pub text: String,
    pub confidence: f64,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub is_agent_addressed: bool,
    pub agent_response: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// A bot command sent from the REST API to the running Discord bot task
#[derive(Debug)]
pub enum BotCommand {
    Join {
        guild_id: u64,
        channel_id: u64,
        channel_name: String,
        project_id: String,
        meeting_session_id: String,
        agent: ActiveAgent,
        server_port: u16,
    },
    Leave {
        guild_id: u64,
    },
}

/// In-memory state for an active Discord voice session
#[derive(Debug, Clone)]
pub struct DiscordVoiceSession {
    pub meeting_session_id: String,
    pub guild_id: u64,
    pub channel_id: u64,
    pub channel_name: String,
    pub project_id: String,
    pub agent: ActiveAgent,
    pub started_at: DateTime<Utc>,
    pub server_port: u16,
    /// Running segment count
    pub segment_count: i32,
    /// Maps Discord SSRC → Discord user ID string
    pub ssrc_to_user: HashMap<u32, String>,
    /// Maps Discord user ID string → display name
    pub user_names: HashMap<String, String>,
}

impl DiscordVoiceSession {
    pub fn new(
        meeting_session_id: String,
        guild_id: u64,
        channel_id: u64,
        channel_name: String,
        project_id: String,
        agent: ActiveAgent,
        server_port: u16,
    ) -> Self {
        Self {
            meeting_session_id,
            guild_id,
            channel_id,
            channel_name,
            project_id,
            agent,
            started_at: Utc::now(),
            server_port,
            segment_count: 0,
            ssrc_to_user: HashMap::new(),
            user_names: HashMap::new(),
        }
    }

    pub fn elapsed_ms(&self) -> i64 {
        (Utc::now() - self.started_at).num_milliseconds()
    }
}

/// Audio accumulation buffer for a single Discord user
#[derive(Debug, Default)]
pub struct UserAudioBuffer {
    /// 48kHz stereo i16 PCM samples, interleaved L/R
    pub samples: Vec<i16>,
    /// Timestamp of last received audio chunk
    pub last_audio_at: Option<std::time::Instant>,
    /// Number of silent ticks in a row
    pub silent_ticks: u32,
    /// When this buffer was started (ms from session start)
    pub start_time_ms: i64,
}

impl UserAudioBuffer {
    pub fn push_samples(&mut self, audio: &[i16], start_time_ms: i64) {
        if self.samples.is_empty() {
            self.start_time_ms = start_time_ms;
        }
        self.samples.extend_from_slice(audio);
        self.last_audio_at = Some(std::time::Instant::now());
        self.silent_ticks = 0;
    }

    pub fn duration_ms(&self) -> i64 {
        // 48000 samples/sec * 2 channels = 96000 i16 per second
        (self.samples.len() as i64 * 1000) / 96_000
    }

    pub fn is_ready_to_flush(&self) -> bool {
        // Flush after 1500ms of silence or 8 seconds of audio
        let silence_threshold = 75; // 75 ticks * 20ms = 1500ms
        let max_duration_ms = 8000;

        self.silent_ticks >= silence_threshold || self.duration_ms() >= max_duration_ms
    }

    pub fn take(&mut self) -> Vec<i16> {
        self.silent_ticks = 0;
        std::mem::take(&mut self.samples)
    }
}

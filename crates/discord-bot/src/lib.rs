//! Discord Voice Bot for PCG — Nora and Topsi join Discord voice channels
//!
//! ## Architecture
//!
//! ```text
//! Discord Voice Channel
//!        │ (Opus audio, per-user RTP packets)
//!        ▼
//! [Songbird VoiceReceiver]
//!   ├── VoiceTick: per-SSRC PCM accumulation
//!   ├── SpeakingStateUpdate: SSRC → user_id mapping
//!   └── Silence detection → flush → WAV encode
//!                                    │
//!                         [Whisper STT] (local or OpenAI)
//!                                    │
//!                         Wake word detection
//!                         ("Nora" / "Topsi")
//!                                    │
//!                         [Agent HTTP call to localhost]
//!                         (Nora chat / Topsi chat API)
//!                                    │
//!                         [TTS synthesis] → songbird playback
//!                                    │
//!                         [DB persist: meeting_segments]
//!                                    │
//!                         [SSE broadcast to dashboard]
//! ```

pub mod audio;
pub mod bot;
pub mod session;
pub mod voice_receiver;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use session::{ActiveAgent, DiscordVoiceSession, TranscriptEvent};
use tokio::sync::broadcast;
use tracing::{error, info};

/// Session map key: "{guild_id}:{agent_name}" — allows Nora and Topsi to
/// coexist in the same guild simultaneously.
pub fn session_key(guild_id: u64, agent: &str) -> String {
    format!("{}:{}", guild_id, agent)
}

/// Active Discord voice sessions: "{guild_id}:{agent}" → session
pub static DISCORD_SESSIONS: Lazy<DashMap<String, DiscordVoiceSession>> = Lazy::new(DashMap::new);

/// Per-session broadcast channels for SSE streaming: meeting_session_id → Sender
pub static TRANSCRIPT_CHANNELS: Lazy<DashMap<String, broadcast::Sender<TranscriptEvent>>> =
    Lazy::new(DashMap::new);

/// Spawn Discord bots as background Tokio tasks.
/// Reads DISCORD_BOT_TOKEN (Nora) and DISCORD_TOPSI_BOT_TOKEN (Topsi).
/// Safe to call unconditionally — silently skips any token not set.
pub async fn spawn_discord_bot(pool: sqlx::SqlitePool, server_port: u16) {
    let nora_token = std::env::var("DISCORD_BOT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    let topsi_token = std::env::var("DISCORD_TOPSI_BOT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    if nora_token.is_none() && topsi_token.is_none() {
        info!("No Discord bot tokens set — Discord bot disabled");
        return;
    }

    if let Some(token) = nora_token {
        info!("Spawning Nora Discord bot...");
        let pool = pool.clone();
        tokio::spawn(async move {
            if let Err(e) = bot::run_bot(token, pool, server_port, Some(ActiveAgent::Nora)).await {
                error!("Nora Discord bot stopped: {}", e);
            }
        });
    }

    if let Some(token) = topsi_token {
        info!("Spawning Topsi Discord bot...");
        let pool = pool.clone();
        tokio::spawn(async move {
            if let Err(e) = bot::run_bot(token, pool, server_port, Some(ActiveAgent::Topsi)).await {
                error!("Topsi Discord bot stopped: {}", e);
            }
        });
    }
}

/// Subscribe to live transcript events for a given meeting session
pub fn subscribe_transcript(
    meeting_session_id: &str,
) -> Option<broadcast::Receiver<TranscriptEvent>> {
    TRANSCRIPT_CHANNELS
        .get(meeting_session_id)
        .map(|tx| tx.subscribe())
}

/// List all active Discord voice sessions
pub fn active_sessions() -> Vec<DiscordVoiceSession> {
    DISCORD_SESSIONS.iter().map(|e| e.value().clone()).collect()
}

//! Voice processing capabilities for Nora
//!
//! Adapted from voice-agent-v2 for executive assistant functionality.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                    Voice Channels (Input)                        │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐ │
//! │  │ Glasses │  │ Twilio  │  │ Browser │  │ Desktop Microphone  │ │
//! │  │  (APN)  │  │ (Phone) │  │  (WS)   │  │      (Local)        │ │
//! │  └────┬────┘  └────┬────┘  └────┬────┘  └──────────┬──────────┘ │
//! │       │            │            │                   │           │
//! │       └────────────┴────────────┴───────────────────┘           │
//! │                              │                                   │
//! │                    ┌─────────▼─────────┐                        │
//! │                    │   VoiceGateway    │                        │
//! │                    │  (Orchestrator)   │                        │
//! │                    └────────┬──────────┘                        │
//! │                             │                                    │
//! │              ┌──────────────┼──────────────┐                    │
//! │              │              │              │                    │
//! │      ┌───────▼──────┐ ┌─────▼─────┐ ┌─────▼──────┐             │
//! │      │ Whisper STT  │ │  Router   │ │ Chatterbox │             │
//! │      │   (Local)    │ │           │ │    TTS     │             │
//! │      └──────────────┘ └─────┬─────┘ └────────────┘             │
//! │                             │                                    │
//! │                    ┌────────▼────────┐                          │
//! │                    │ ExecutionEngine │                          │
//! │                    │ (Nora/Topsi)    │                          │
//! │                    └─────────────────┘                          │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Sovereign Stack
//!
//! All voice processing can run locally without cloud dependencies:
//! - **STT**: Whisper (local or API)
//! - **TTS**: Chatterbox (local Python server)
//! - **Transport**: APN mesh network (libp2p)

// pub mod channels;  // Not yet implemented
pub mod config;
pub mod diarization;
pub mod engine;
// pub mod gateway;  // Not yet implemented
pub mod pcg_router_engine;
// pub mod router;  // Not yet implemented
pub mod stt;
pub mod tts;

use std::sync::Arc;

use chrono::{DateTime, Utc};
// pub use channels::{
//     AudioChunk, BrowserChannel, BrowserVoiceMessage, ChannelType, GlassesChannel,
//     MeshVoiceMessage, MeshVoicePayload, VoiceChannel, VoiceChannelEvent, VoiceChannelSession,
// };
pub use config::{AudioConfig, STTConfig, STTProvider, TTSConfig, TTSProvider, VoiceConfig};
pub use diarization::{AlignedWord, DiarizationConfig, DiarizationEngine, DiarizedSegment};
pub use engine::VoiceEngine;
pub use pcg_router_engine::PCGRouterVoiceEngine;
use sqlx::SqlitePool;

/// Unified voice engine that can use either the legacy config-based engine
/// or the PCG Router database-driven engine.
///
/// Prefer using `UnifiedVoiceEngine::from_pool()` when a database pool is available,
/// as it provides automatic provider fallback, cost tracking, and centralized
/// API key management.
pub enum UnifiedVoiceEngine {
    /// Legacy config-based voice engine (deprecated for new code)
    Legacy(VoiceEngine),
    /// PCG Router database-driven voice engine (preferred)
    Router(PCGRouterVoiceEngine),
}

impl UnifiedVoiceEngine {
    /// Create a unified voice engine using the PCG Router (preferred).
    ///
    /// This is the recommended constructor when a database pool is available.
    pub fn from_pool(pool: Arc<SqlitePool>) -> Self {
        Self::Router(PCGRouterVoiceEngine::new(pool))
    }

    /// Create a unified voice engine from a legacy VoiceConfig.
    ///
    /// Use this only when a database pool is not available.
    pub async fn from_config(config: VoiceConfig) -> VoiceResult<Self> {
        Ok(Self::Legacy(VoiceEngine::new(config).await?))
    }

    /// Synthesize speech from text.
    pub async fn synthesize_speech(&self, text: &str) -> VoiceResult<String> {
        match self {
            Self::Legacy(engine) => engine.synthesize_speech(text).await,
            Self::Router(engine) => engine.synthesize_speech(text).await,
        }
    }

    /// Synthesize speech with format info.
    pub async fn synthesize_speech_with_format(
        &self,
        text: &str,
    ) -> VoiceResult<(String, AudioFormat)> {
        match self {
            Self::Legacy(engine) => engine.synthesize_speech_with_format(text).await,
            Self::Router(engine) => engine.synthesize_speech_with_format(text).await,
        }
    }

    /// Transcribe speech to text.
    pub async fn transcribe_speech(&self, audio_data: &str) -> VoiceResult<String> {
        match self {
            Self::Legacy(engine) => engine.transcribe_speech(audio_data).await,
            Self::Router(engine) => engine.transcribe_speech(audio_data).await,
        }
    }

    /// Check if the engine is ready.
    pub async fn is_ready(&self) -> bool {
        match self {
            Self::Legacy(engine) => engine.is_ready().await,
            Self::Router(engine) => engine.is_ready(),
        }
    }
}
// pub use gateway::{
//     CommandContext, CommandHandler, CommandResponse, ConversationTurn, GatewaySession,
//     Speaker, VoiceGateway, VoiceGatewayConfig, VoiceGatewayEvent,
// };
// pub use router::{
//     ControlAction, IntentMatch, StatusTarget, VoiceCommandRouter, VoiceIntent,
// };
use serde::{Deserialize, Serialize};
pub use stt::{SpeechToText, TranscriptionResult};
use ts_rs::TS;
pub use tts::{TextToSpeech, VoiceProfile};

/// Request for speech synthesis
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpeechRequest {
    pub text: String,
    pub voice_profile: VoiceProfile,
    pub speed: f32,
    pub volume: f32,
    pub format: AudioFormat,
    pub british_accent: bool,
    pub executive_tone: bool,
}

/// Response from speech synthesis
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpeechResponse {
    pub audio_data: String, // Base64 encoded
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub format: AudioFormat,
    pub processing_time_ms: u64,
}

/// Audio format options
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
}

/// Voice processing errors
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("TTS error: {0}")]
    TTSError(String),

    #[error("STT error: {0}")]
    STTError(String),

    #[error("Audio processing error: {0}")]
    AudioError(String),

    #[error("Voice engine not initialized")]
    NotInitialized,

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type VoiceResult<T> = Result<T, VoiceError>;

/// Voice interaction event
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInteraction {
    pub interaction_id: String,
    pub session_id: String,
    pub interaction_type: VoiceInteractionType,
    // User/device tracking
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub device_type: Option<String>, // "glasses", "phone", "browser"
    // Audio data
    pub audio_input: Option<String>, // Base64 encoded
    pub transcription: Option<String>,
    pub response_text: String,
    pub audio_response: Option<String>, // Base64 encoded
    pub processing_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Types of voice interactions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum VoiceInteractionType {
    /// User spoke to Nora
    SpeechInput,
    /// Nora spoke to user
    SpeechOutput,
    /// Two-way conversation
    Conversation,
    /// Voice command
    Command,
    /// Executive briefing
    Briefing,
    /// Alert/notification
    Alert,
}

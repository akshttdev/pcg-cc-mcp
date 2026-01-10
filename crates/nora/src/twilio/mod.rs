//! Twilio phone integration for NORA
//!
//! Enables users to call a Twilio virtual number and interact with NORA via voice.
//! Supports inbound calls, speech recognition, and TTS responses using NORA's voice engine.

pub mod audio_cache;
pub mod call_handler;
pub mod twiml;

pub use audio_cache::{get_audio_cache, CachedAudio, TwilioAudioCache};
pub use call_handler::{TwilioCallHandler, TwilioCallState};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
pub use twiml::TwimlBuilder;

/// Configuration for Twilio integration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TwilioConfig {
    /// Twilio Account SID
    pub account_sid: String,
    /// Twilio Auth Token
    pub auth_token: String,
    /// Twilio phone number (the number users call)
    pub phone_number: String,
    /// Base URL for webhooks (your server's public URL)
    pub webhook_base_url: String,
    /// Maximum call duration in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_max_call_duration")]
    pub max_call_duration: u32,
    /// Speech recognition language (default: en-GB for British English)
    #[serde(default = "default_speech_language")]
    pub speech_language: String,
    /// TTS voice for responses (Twilio Polly voice)
    #[serde(default = "default_tts_voice")]
    pub tts_voice: String,
    /// Enable call recording
    #[serde(default)]
    pub recording_enabled: bool,
    /// Greeting message for incoming calls
    #[serde(default = "default_greeting")]
    pub greeting_message: String,
}

fn default_max_call_duration() -> u32 {
    3600 // 1 hour
}

fn default_speech_language() -> String {
    "en-GB".to_string() // British English
}

fn default_tts_voice() -> String {
    "Polly.Amy".to_string() // British female voice (Amy)
}

fn default_greeting() -> String {
    "Hello, this is Nora, your Executive AI Assistant. How may I assist you today?".to_string()
}

impl Default for TwilioConfig {
    fn default() -> Self {
        Self {
            account_sid: String::new(),
            auth_token: String::new(),
            phone_number: String::new(),
            webhook_base_url: String::new(),
            max_call_duration: default_max_call_duration(),
            speech_language: default_speech_language(),
            tts_voice: default_tts_voice(),
            recording_enabled: false,
            greeting_message: default_greeting(),
        }
    }
}

impl TwilioConfig {
    /// Check if Twilio is configured
    pub fn is_configured(&self) -> bool {
        !self.account_sid.is_empty()
            && !self.auth_token.is_empty()
            && !self.phone_number.is_empty()
            && !self.webhook_base_url.is_empty()
    }

    /// Create config from environment variables
    pub fn from_env() -> Option<Self> {
        let account_sid = std::env::var("TWILIO_ACCOUNT_SID").ok()?;
        let auth_token = std::env::var("TWILIO_AUTH_TOKEN").ok()?;
        let phone_number = std::env::var("TWILIO_PHONE_NUMBER").ok()?;
        let webhook_base_url = std::env::var("TWILIO_WEBHOOK_BASE_URL").ok()?;

        // Helper to get env var with fallback if empty
        let get_env_or_default = |key: &str, default: String| -> String {
            std::env::var(key)
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(default)
        };

        Some(Self {
            account_sid,
            auth_token,
            phone_number,
            webhook_base_url,
            max_call_duration: std::env::var("TWILIO_MAX_CALL_DURATION")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_call_duration),
            speech_language: get_env_or_default("TWILIO_SPEECH_LANGUAGE", default_speech_language()),
            tts_voice: get_env_or_default("TWILIO_TTS_VOICE", default_tts_voice()),
            recording_enabled: std::env::var("TWILIO_RECORDING_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            greeting_message: get_env_or_default("TWILIO_GREETING_MESSAGE", default_greeting()),
        })
    }
}

/// Twilio webhook request for incoming calls
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TwilioCallRequest {
    /// Unique identifier for the call
    pub call_sid: String,
    /// The Twilio account SID
    pub account_sid: String,
    /// The phone number that initiated the call
    pub from: String,
    /// The phone number that was called
    pub to: String,
    /// Call status
    pub call_status: String,
    /// API version
    pub api_version: Option<String>,
    /// Direction of the call
    pub direction: Option<String>,
    /// Caller's name (if available via caller ID)
    pub caller_name: Option<String>,
    /// Geographic location info
    pub from_city: Option<String>,
    pub from_state: Option<String>,
    pub from_country: Option<String>,
}

/// Twilio speech recognition result
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TwilioSpeechResult {
    /// The call SID
    pub call_sid: String,
    /// The transcribed speech
    pub speech_result: Option<String>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: Option<f64>,
    /// Speech language detected
    pub language: Option<String>,
    /// Unstable text (partial recognition)
    #[serde(rename = "UnstableSpeechResult")]
    pub unstable_speech_result: Option<String>,
}

/// Twilio call status callback
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TwilioStatusCallback {
    /// The call SID
    pub call_sid: String,
    /// Call status
    pub call_status: String,
    /// Call duration in seconds
    pub call_duration: Option<u32>,
    /// Recording URL (if recording was enabled)
    pub recording_url: Option<String>,
    /// Recording SID
    pub recording_sid: Option<String>,
}

/// Twilio error types
#[derive(Debug, thiserror::Error)]
pub enum TwilioError {
    #[error("Twilio not configured")]
    NotConfigured,

    #[error("Invalid request signature")]
    InvalidSignature,

    #[error("Call not found: {0}")]
    CallNotFound(String),

    #[error("Speech recognition failed: {0}")]
    SpeechRecognitionFailed(String),

    #[error("TTS generation failed: {0}")]
    TtsGenerationFailed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type TwilioResult<T> = Result<T, TwilioError>;

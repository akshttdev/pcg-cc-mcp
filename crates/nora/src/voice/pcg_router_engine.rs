//! PCG Router-aware voice engine for database-driven provider selection.
//!
//! This module provides a voice engine that routes TTS and STT requests through
//! the PCG Router system (`WorkflowTTSService` and `WorkflowSTTService`), enabling:
//!
//! - **Database-driven provider selection** via `pcg_router_tts_providers` and
//!   `pcg_router_stt_providers` tables
//! - **Automatic fallback** across providers based on priority
//! - **Cost tracking** via `service_usage_log` table
//! - **Centralized API key management** (env var or direct DB storage)
//!
//! # Usage
//!
//! ```rust,ignore
//! use nora::voice::PCGRouterVoiceEngine;
//! use sqlx::SqlitePool;
//!
//! let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
//! let engine = PCGRouterVoiceEngine::new(pool);
//!
//! // Synthesize speech (routes through enabled TTS providers)
//! let (audio_b64, format, metadata) = engine.synthesize("Hello, world!", None, None).await?;
//!
//! // Transcribe speech (routes through enabled STT providers)
//! let (response, metadata) = engine.transcribe(&audio_b64, Some("wav"), None, None).await?;
//! ```

use std::sync::Arc;

use services::services::{
    workflow_stt::{STTRequest, STTResponse, STTRoutingMetadata, WorkflowSTTService},
    workflow_tts::{TTSRequest, TTSRoutingMetadata, WorkflowTTSService},
};
use sqlx::SqlitePool;
use tracing::info;

use super::{AudioFormat, VoiceError, VoiceResult};

/// PCG Router-aware voice engine for database-driven provider selection.
///
/// Unlike the legacy `VoiceEngine`, this engine:
/// - Routes all requests through the PCG Router services
/// - Uses database-configured providers with priority-based fallback
/// - Automatically logs usage and costs to `service_usage_log`
/// - Does not require upfront provider initialization
#[derive(Clone)]
pub struct PCGRouterVoiceEngine {
    pool: Arc<SqlitePool>,
}

impl PCGRouterVoiceEngine {
    /// Create a new PCG Router voice engine.
    ///
    /// The engine is immediately ready to use; providers are resolved
    /// at request time from the database.
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        info!("[PCG_ROUTER_VOICE] Initialized with database-driven provider routing");
        Self { pool }
    }

    /// Synthesize speech from text.
    ///
    /// Routes to enabled TTS providers in priority order with automatic fallback.
    /// Returns the base64-encoded audio data, format, and routing metadata.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to synthesize
    /// * `voice_id` - Optional voice identifier (provider-specific)
    /// * `speed` - Optional playback speed multiplier (default: 1.0)
    ///
    /// # Returns
    ///
    /// A tuple of (audio_base64, audio_format, routing_metadata)
    pub async fn synthesize(
        &self,
        text: &str,
        voice_id: Option<&str>,
        speed: Option<f64>,
    ) -> VoiceResult<(String, AudioFormat, TTSRoutingMetadata)> {
        let request = TTSRequest {
            text: text.to_string(),
            voice_id: voice_id.map(String::from),
            speed,
            format: None, // Let provider choose default
        };

        let (response, metadata) = WorkflowTTSService::synthesize(&self.pool, request)
            .await
            .map_err(|e| VoiceError::TTSError(e.to_string()))?;

        let format = match response.format.as_str() {
            "mp3" => AudioFormat::Mp3,
            "wav" => AudioFormat::Wav,
            "ogg" => AudioFormat::Ogg,
            "flac" => AudioFormat::Flac,
            other => {
                return Err(VoiceError::UnsupportedFormat(format!(
                    "TTS returned unsupported format: {}",
                    other
                )))
            }
        };

        info!(
            "[PCG_ROUTER_VOICE] TTS completed via '{}' ({}) — {} chars, {}ms",
            metadata.provider_used,
            metadata.provider_type,
            metadata.input_chars,
            metadata.duration_ms
        );

        Ok((response.audio_data, format, metadata))
    }

    /// Synthesize speech with simplified interface.
    ///
    /// Convenience method that returns just the audio data and format,
    /// hiding the routing metadata.
    pub async fn synthesize_simple(&self, text: &str) -> VoiceResult<(String, AudioFormat)> {
        let (audio, format, _metadata) = self.synthesize(text, None, None).await?;
        Ok((audio, format))
    }

    /// Transcribe audio to text.
    ///
    /// Routes to enabled STT providers in priority order with automatic fallback.
    /// Returns the transcription response and routing metadata.
    ///
    /// # Arguments
    ///
    /// * `audio_data` - Base64-encoded audio data
    /// * `format` - Audio format (e.g., "wav", "mp3", "webm")
    /// * `language` - Optional language code (default: "en")
    /// * `word_timestamps` - Whether to include word-level timestamps
    ///
    /// # Returns
    ///
    /// A tuple of (STTResponse, routing_metadata)
    pub async fn transcribe(
        &self,
        audio_data: &str,
        format: Option<&str>,
        language: Option<&str>,
        word_timestamps: Option<bool>,
    ) -> VoiceResult<(STTResponse, STTRoutingMetadata)> {
        let request = STTRequest {
            audio_data: audio_data.to_string(),
            format: format.map(String::from),
            language: language.map(String::from),
            word_timestamps,
        };

        let (response, metadata) = WorkflowSTTService::transcribe(&self.pool, request)
            .await
            .map_err(|e| VoiceError::STTError(e.to_string()))?;

        info!(
            "[PCG_ROUTER_VOICE] STT completed via '{}' ({}) — '{}' ({}ms)",
            metadata.provider_used,
            metadata.provider_type,
            truncate_text(&response.text, 50),
            metadata.duration_ms
        );

        Ok((response, metadata))
    }

    /// Transcribe audio with simplified interface.
    ///
    /// Convenience method that returns just the transcription text,
    /// hiding the routing metadata and additional response fields.
    pub async fn transcribe_simple(
        &self,
        audio_data: &str,
        format: Option<&str>,
    ) -> VoiceResult<String> {
        let (response, _metadata) = self.transcribe(audio_data, format, None, None).await?;
        Ok(response.text)
    }

    /// Get the database pool for direct service access if needed.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // =========================================================================
    // VoiceEngine-compatible methods
    // =========================================================================
    // These methods provide API compatibility with the legacy VoiceEngine,
    // making it easier to migrate existing callers.

    /// Synthesize speech from text (VoiceEngine-compatible).
    ///
    /// Returns just the base64-encoded audio data (MP3 format by default).
    /// For access to format and routing metadata, use [`synthesize`] instead.
    pub async fn synthesize_speech(&self, text: &str) -> VoiceResult<String> {
        let (audio, _format, _metadata) = self.synthesize(text, None, None).await?;
        Ok(audio)
    }

    /// Synthesize speech with format info (VoiceEngine-compatible).
    ///
    /// Returns the base64-encoded audio data and format.
    /// For access to routing metadata, use [`synthesize`] instead.
    pub async fn synthesize_speech_with_format(
        &self,
        text: &str,
    ) -> VoiceResult<(String, AudioFormat)> {
        self.synthesize_simple(text).await
    }

    /// Transcribe speech to text (VoiceEngine-compatible).
    ///
    /// Returns just the transcription text.
    /// For access to routing metadata and word timestamps, use [`transcribe`] instead.
    pub async fn transcribe_speech(&self, audio_data: &str) -> VoiceResult<String> {
        self.transcribe_simple(audio_data, None).await
    }

    /// Check if the engine is ready (always true for PCG Router engine).
    ///
    /// Unlike the legacy `VoiceEngine` which requires provider initialization,
    /// this engine resolves providers at request time from the database.
    pub fn is_ready(&self) -> bool {
        true
    }
}

/// Truncate text for logging, adding ellipsis if needed.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 8), "hello...");
        assert_eq!(truncate_text("hi", 5), "hi");
    }
}

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

const ELEVENLABS_API_URL: &str = "https://api.elevenlabs.io/v1/text-to-speech";

#[derive(Debug, Clone, Deserialize)]
pub struct WordAlignment {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

pub struct TtsResult {
    /// Raw MP3 bytes
    pub audio_bytes: Vec<u8>,
    /// Word-level timing from ElevenLabs alignment data
    pub word_alignment: Vec<WordAlignment>,
}

/// ElevenLabs TTS client with connection reuse.
pub struct ElevenLabsClient {
    client: Client,
    api_key: String,
}

impl ElevenLabsClient {
    /// Create a new ElevenLabs client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            api_key: api_key.into(),
        })
    }

    /// Generate TTS audio via ElevenLabs with-timestamps endpoint.
    /// Returns raw MP3 bytes + word-level alignment for lip sync.
    pub async fn generate_tts(&self, voice_id: &str, text: &str) -> Result<TtsResult> {
        let url = format!("{}/{}/with-timestamps", ELEVENLABS_API_URL, voice_id);

        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_turbo_v2_5",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.75
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("ElevenLabs request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs error {}: {}", status, text);
        }

        #[derive(Deserialize)]
        struct ElevenLabsResponse {
            audio_base64: String,
            alignment: Option<Alignment>,
        }

        #[derive(Deserialize)]
        struct Alignment {
            characters: Vec<String>,
            character_start_times_seconds: Vec<f64>,
            character_end_times_seconds: Vec<f64>,
        }

        let parsed: ElevenLabsResponse =
            resp.json().await.context("parsing ElevenLabs response")?;

        let audio_bytes = base64_decode(&parsed.audio_base64)?;

        let word_alignment = if let Some(align) = parsed.alignment {
            chars_to_words(
                &align.characters,
                &align.character_start_times_seconds,
                &align.character_end_times_seconds,
            )
        } else {
            vec![]
        };

        tracing::info!(
            "ElevenLabs TTS: {} bytes audio, {} words aligned",
            audio_bytes.len(),
            word_alignment.len()
        );

        Ok(TtsResult {
            audio_bytes,
            word_alignment,
        })
    }
}

/// Collapse character-level alignment into word-level alignment.
fn chars_to_words(chars: &[String], starts: &[f64], ends: &[f64]) -> Vec<WordAlignment> {
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut word_start = 0.0f64;
    let mut word_end = 0.0f64;

    for (i, ch) in chars.iter().enumerate() {
        let start = starts.get(i).copied().unwrap_or(0.0);
        let end = ends.get(i).copied().unwrap_or(0.0);

        if ch == " " || ch == "\n" {
            if !current_word.is_empty() {
                words.push(WordAlignment {
                    word: current_word.clone(),
                    start: word_start,
                    end: word_end,
                });
                current_word.clear();
            }
        } else {
            if current_word.is_empty() {
                word_start = start;
            }
            current_word.push_str(ch);
            word_end = end;
        }
    }

    if !current_word.is_empty() {
        words.push(WordAlignment {
            word: current_word,
            start: word_start,
            end: word_end,
        });
    }

    words
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(s).context("base64 decode failed")
}

// ============================================================================
// Legacy standalone function (preserved for backward compatibility)
// Creates a new client per call - prefer using ElevenLabsClient directly
// ============================================================================

/// Generate TTS audio via ElevenLabs with-timestamps endpoint.
/// Returns raw MP3 bytes + word-level alignment for lip sync.
///
/// NOTE: Prefer using `ElevenLabsClient::generate_tts` for connection reuse.
pub async fn generate_tts(api_key: &str, voice_id: &str, text: &str) -> Result<TtsResult> {
    let client = ElevenLabsClient::new(api_key)?;
    client.generate_tts(voice_id, text).await
}

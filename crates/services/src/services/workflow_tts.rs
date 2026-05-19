//! Workflow TTS Service — provider-aware TTS routing for the workflow engine.
//!
//! Routes TTS requests through the PCG Router provider system with automatic
//! fallback, cost tracking, and usage logging.

use std::time::Instant;

use base64::engine::Engine;
use db::models::pcg_router_tts_provider::PcgRouterTTSProvider;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::services::editron::tracking::UsageTracker;

// ── Public types ────────────────────────────────────────────────────────────

/// TTS synthesis request.
#[derive(Debug, Clone)]
pub struct TTSRequest {
    pub text: String,
    pub voice_id: Option<String>,
    pub speed: Option<f64>,
    pub format: Option<String>,
}

/// TTS synthesis response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSResponse {
    /// Base64-encoded audio data.
    pub audio_data: String,
    /// Estimated audio duration in milliseconds.
    pub duration_ms: u64,
    /// Audio sample rate in Hz.
    pub sample_rate: u32,
    /// Audio format (mp3, wav, etc.).
    pub format: String,
}

/// Routing metadata for TTS operations.
#[derive(Debug, Clone, Serialize)]
pub struct TTSRoutingMetadata {
    pub provider_used: String,
    pub provider_type: String,
    pub input_chars: i64,
    pub duration_ms: i64,
    pub estimated_cost_micros: Option<i64>,
}

// ── Service ─────────────────────────────────────────────────────────────────

/// Stateless TTS service for workflow speech synthesis.
///
/// Routes TTS requests through the PCG Router provider system with automatic
/// fallback on provider errors.
pub struct WorkflowTTSService;

impl WorkflowTTSService {
    /// Synthesize speech from text.
    ///
    /// Routes to enabled TTS providers in priority order with automatic fallback.
    pub async fn synthesize(
        pool: &SqlitePool,
        request: TTSRequest,
    ) -> anyhow::Result<(TTSResponse, TTSRoutingMetadata)> {
        let candidates = Self::resolve_candidates(pool, request.voice_id.as_deref()).await?;

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        let input_chars = request.text.chars().count() as i64;

        for provider in &candidates {
            // Local providers don't need API keys
            if !provider.is_local {
                if provider.resolve_api_key().is_none() {
                    tracing::warn!(
                        "[WORKFLOW_TTS] No API key for provider '{}', skipping",
                        provider.name
                    );
                    continue;
                }
            }

            let start = Instant::now();

            let result = forward_to_provider(&http, provider, &request).await;
            let duration_ms = start.elapsed().as_millis() as i64;

            match result {
                Ok(response) => {
                    tracing::info!(
                        "[WORKFLOW_TTS] Synthesized with '{}' ({}) — {} chars in {}ms",
                        provider.name,
                        provider.provider_type,
                        input_chars,
                        duration_ms
                    );

                    let cost = calculate_cost(provider, input_chars);

                    // Log usage (fire-and-forget)
                    UsageTracker::log_tts_operation(
                        pool,
                        &provider.provider_type,
                        "synthesize",
                        Some(input_chars),
                        Some(response.duration_ms as i64),
                        duration_ms,
                        true,
                        None,
                        Some(serde_json::json!({
                            "voice_id": request.voice_id,
                            "format": response.format,
                            "cost_micros": cost,
                        })),
                    )
                    .await;

                    let metadata = TTSRoutingMetadata {
                        provider_used: provider.name.clone(),
                        provider_type: provider.provider_type.clone(),
                        input_chars,
                        duration_ms,
                        estimated_cost_micros: cost,
                    };

                    return Ok((response, metadata));
                }
                Err(e) => {
                    tracing::warn!(
                        "[WORKFLOW_TTS] '{}' failed: {}, trying next provider",
                        provider.name,
                        e
                    );

                    // Log failure (fire-and-forget)
                    UsageTracker::log_tts_operation(
                        pool,
                        &provider.provider_type,
                        "synthesize",
                        Some(input_chars),
                        None,
                        duration_ms,
                        false,
                        Some(&e.to_string()),
                        None,
                    )
                    .await;
                }
            }
        }

        anyhow::bail!("All TTS providers failed")
    }

    /// Build the candidate list (shared by synthesis methods).
    async fn resolve_candidates(
        pool: &SqlitePool,
        voice_hint: Option<&str>,
    ) -> anyhow::Result<Vec<PcgRouterTTSProvider>> {
        let mut candidates = PcgRouterTTSProvider::list_enabled(pool).await?;

        // Filter by voice support if a hint is provided
        if let Some(voice) = voice_hint {
            candidates.retain(|p| p.supports_voice(voice));
        }

        if candidates.is_empty() {
            anyhow::bail!("No enabled TTS providers in PCG Router");
        }

        Ok(candidates)
    }
}

// ── Provider dispatch ───────────────────────────────────────────────────────

async fn forward_to_provider(
    http: &reqwest::Client,
    provider: &PcgRouterTTSProvider,
    request: &TTSRequest,
) -> anyhow::Result<TTSResponse> {
    match provider.provider_type.as_str() {
        "elevenlabs" => forward_to_elevenlabs(http, provider, request).await,
        "openai" => forward_to_openai(http, provider, request).await,
        "azure" => forward_to_azure(http, provider, request).await,
        "chatterbox" => forward_to_chatterbox(http, provider, request).await,
        other => anyhow::bail!("Unknown TTS provider type: {}", other),
    }
}

// ── ElevenLabs ──────────────────────────────────────────────────────────────

async fn forward_to_elevenlabs(
    http: &reqwest::Client,
    provider: &PcgRouterTTSProvider,
    request: &TTSRequest,
) -> anyhow::Result<TTSResponse> {
    let api_key = provider
        .resolve_api_key()
        .ok_or_else(|| anyhow::anyhow!("ElevenLabs API key not configured"))?;

    let voice_id = request.voice_id.as_deref().unwrap_or("Rachel");
    let speed = request.speed.unwrap_or(1.0).clamp(0.7, 1.2);

    let payload = serde_json::json!({
        "text": request.text,
        "model_id": "eleven_monolingual_v1",
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.8,
            "speed": speed
        }
    });

    let url = format!("{}/v1/text-to-speech/{}", provider.base_url(), voice_id);

    let response = http
        .post(&url)
        .header("Accept", "audio/mpeg")
        .header("Content-Type", "application/json")
        .header("xi-api-key", &api_key)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("ElevenLabs {}: {}", status, text);
    }

    let audio_bytes = response.bytes().await?;
    let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

    Ok(TTSResponse {
        audio_data,
        duration_ms: estimate_duration(&request.text),
        sample_rate: 22050,
        format: "mp3".to_string(),
    })
}

// ── OpenAI ──────────────────────────────────────────────────────────────────

async fn forward_to_openai(
    http: &reqwest::Client,
    provider: &PcgRouterTTSProvider,
    request: &TTSRequest,
) -> anyhow::Result<TTSResponse> {
    let api_key = provider
        .resolve_api_key()
        .ok_or_else(|| anyhow::anyhow!("OpenAI API key not configured"))?;

    let voice = request.voice_id.as_deref().unwrap_or("fable");
    let valid_voices = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];
    let voice = if valid_voices.contains(&voice) {
        voice
    } else {
        "fable"
    };

    let speed = request.speed.unwrap_or(1.0).clamp(0.25, 4.0);
    let format = request.format.as_deref().unwrap_or("mp3");

    let payload = serde_json::json!({
        "model": "tts-1-hd",
        "input": request.text,
        "voice": voice,
        "response_format": format,
        "speed": speed
    });

    let url = format!("{}/v1/audio/speech", provider.base_url());

    let response = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI TTS {}: {}", status, text);
    }

    let audio_bytes = response.bytes().await?;
    let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

    Ok(TTSResponse {
        audio_data,
        duration_ms: estimate_duration(&request.text),
        sample_rate: 24000,
        format: format.to_string(),
    })
}

// ── Azure ───────────────────────────────────────────────────────────────────

async fn forward_to_azure(
    http: &reqwest::Client,
    provider: &PcgRouterTTSProvider,
    request: &TTSRequest,
) -> anyhow::Result<TTSResponse> {
    let api_key = provider
        .resolve_api_key()
        .ok_or_else(|| anyhow::anyhow!("Azure Speech key not configured"))?;

    let voice_name = request.voice_id.as_deref().unwrap_or("en-GB-SoniaNeural");

    let speed = match request.speed {
        Some(s) if s > 1.0 => "fast",
        Some(s) if s < 1.0 => "slow",
        _ => "medium",
    };

    let ssml = format!(
        r#"<speak version='1.0' xml:lang='en-GB'>
            <voice xml:lang='en-GB' name='{}'>
                <prosody rate='{}'>
                    {}
                </prosody>
            </voice>
        </speak>"#,
        voice_name, speed, request.text
    );

    let url = format!("{}/cognitiveservices/v1", provider.base_url());

    let response = http
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", &api_key)
        .header("Content-Type", "application/ssml+xml")
        .header("X-Microsoft-OutputFormat", "riff-24khz-16bit-mono-pcm")
        .body(ssml)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Azure TTS {}: {}", status, text);
    }

    let audio_bytes = response.bytes().await?;
    let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

    Ok(TTSResponse {
        audio_data,
        duration_ms: estimate_duration(&request.text),
        sample_rate: 24000,
        format: "wav".to_string(),
    })
}

// ── Chatterbox (local) ──────────────────────────────────────────────────────

async fn forward_to_chatterbox(
    http: &reqwest::Client,
    provider: &PcgRouterTTSProvider,
    request: &TTSRequest,
) -> anyhow::Result<TTSResponse> {
    let voice = request.voice_id.as_deref().unwrap_or("british_female");
    let speed = request.speed.unwrap_or(1.0);

    let payload = serde_json::json!({
        "text": request.text,
        "voice": voice,
        "speed": speed,
        "exaggeration": 0.5
    });

    let url = format!("{}/tts", provider.base_url());

    let response = http
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Chatterbox TTS {}: {}", status, text);
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/wav");

    let format = if content_type.contains("mpeg") || content_type.contains("mp3") {
        "mp3"
    } else {
        "wav"
    };

    let audio_bytes = response.bytes().await?;
    let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

    Ok(TTSResponse {
        audio_data,
        duration_ms: estimate_duration(&request.text),
        sample_rate: 22050,
        format: format.to_string(),
    })
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn estimate_duration(text: &str) -> u64 {
    // Rough estimation: average speaking rate is about 150 words per minute
    let word_count = text.split_whitespace().count();
    let minutes = word_count as f64 / 150.0;
    (minutes * 60.0 * 1000.0) as u64
}

fn calculate_cost(provider: &PcgRouterTTSProvider, input_chars: i64) -> Option<i64> {
    if provider.cost_per_1000_chars == 0 {
        return None;
    }
    let cost = (input_chars as f64 / 1000.0) * provider.cost_per_1000_chars as f64;
    Some(cost as i64)
}

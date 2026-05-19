//! Workflow STT Service — provider-aware STT routing for the workflow engine.
//!
//! Routes STT requests through the PCG Router provider system with automatic
//! fallback, cost tracking, and usage logging.

use std::time::Instant;

use base64::engine::Engine;
use db::models::pcg_router_stt_provider::PcgRouterSTTProvider;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::services::editron::tracking::UsageTracker;

// ── Public types ────────────────────────────────────────────────────────────

/// STT transcription request.
#[derive(Debug, Clone)]
pub struct STTRequest {
    /// Base64-encoded audio data.
    pub audio_data: String,
    /// Audio format (wav, mp3, webm, etc.).
    pub format: Option<String>,
    /// Language code (e.g., "en", "en-GB").
    pub language: Option<String>,
    /// Whether to include word-level timestamps.
    pub word_timestamps: Option<bool>,
}

/// Word-level timestamp information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub confidence: f32,
}

/// STT transcription response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTResponse {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub word_timestamps: Vec<WordTimestamp>,
}

/// Routing metadata for STT operations.
#[derive(Debug, Clone, Serialize)]
pub struct STTRoutingMetadata {
    pub provider_used: String,
    pub provider_type: String,
    pub audio_duration_ms: Option<i64>,
    pub duration_ms: i64,
    pub estimated_cost_micros: Option<i64>,
}

// ── Service ─────────────────────────────────────────────────────────────────

/// Stateless STT service for workflow speech recognition.
///
/// Routes STT requests through the PCG Router provider system with automatic
/// fallback on provider errors.
pub struct WorkflowSTTService;

impl WorkflowSTTService {
    /// Transcribe audio to text.
    ///
    /// Routes to enabled STT providers in priority order with automatic fallback.
    pub async fn transcribe(
        pool: &SqlitePool,
        request: STTRequest,
    ) -> anyhow::Result<(STTResponse, STTRoutingMetadata)> {
        let language = request.language.as_deref().unwrap_or("en");
        let candidates = Self::resolve_candidates(pool, language).await?;

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        for provider in &candidates {
            // Local providers don't need API keys
            if !provider.is_local {
                if provider.resolve_api_key().is_none() {
                    tracing::warn!(
                        "[WORKFLOW_STT] No API key for provider '{}', skipping",
                        provider.name
                    );
                    continue;
                }
            }

            let start = Instant::now();

            let result = forward_to_provider(&http, &provider, &request).await;
            let duration_ms = start.elapsed().as_millis() as i64;

            match result {
                Ok((response, audio_duration_ms)) => {
                    tracing::info!(
                        "[WORKFLOW_STT] Transcribed with '{}' ({}) — {} chars in {}ms",
                        provider.name,
                        provider.provider_type,
                        response.text.len(),
                        duration_ms
                    );

                    let cost = calculate_cost(&provider, audio_duration_ms);

                    // Log usage (fire-and-forget)
                    UsageTracker::log_stt_operation(
                        pool,
                        &provider.provider_type,
                        "transcribe",
                        audio_duration_ms,
                        Some(response.text.chars().count() as i64),
                        duration_ms,
                        true,
                        None,
                        Some(serde_json::json!({
                            "language": response.language,
                            "confidence": response.confidence,
                            "cost_micros": cost,
                        })),
                    )
                    .await;

                    let metadata = STTRoutingMetadata {
                        provider_used: provider.name.clone(),
                        provider_type: provider.provider_type.clone(),
                        audio_duration_ms,
                        duration_ms,
                        estimated_cost_micros: cost,
                    };

                    return Ok((response, metadata));
                }
                Err(e) => {
                    tracing::warn!(
                        "[WORKFLOW_STT] '{}' failed: {}, trying next provider",
                        provider.name,
                        e
                    );

                    // Log failure (fire-and-forget)
                    UsageTracker::log_stt_operation(
                        pool,
                        &provider.provider_type,
                        "transcribe",
                        None,
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

        anyhow::bail!("All STT providers failed")
    }

    /// Build the candidate list (shared by transcription methods).
    async fn resolve_candidates(
        pool: &SqlitePool,
        language: &str,
    ) -> anyhow::Result<Vec<PcgRouterSTTProvider>> {
        let mut candidates = PcgRouterSTTProvider::list_enabled(pool).await?;

        // Filter by language support
        candidates.retain(|p| p.supports_language(language));

        if candidates.is_empty() {
            anyhow::bail!(
                "No enabled STT providers in PCG Router for language '{}'",
                language
            );
        }

        Ok(candidates)
    }
}

// ── Provider dispatch ───────────────────────────────────────────────────────

async fn forward_to_provider(
    http: &reqwest::Client,
    provider: &PcgRouterSTTProvider,
    request: &STTRequest,
) -> anyhow::Result<(STTResponse, Option<i64>)> {
    match provider.provider_type.as_str() {
        "whisper" => forward_to_whisper(http, provider, request).await,
        "azure" => forward_to_azure(http, provider, request).await,
        "local_whisper" => forward_to_local_whisper(http, provider, request).await,
        other => anyhow::bail!("Unknown STT provider type: {}", other),
    }
}

// ── OpenAI Whisper ──────────────────────────────────────────────────────────

async fn forward_to_whisper(
    http: &reqwest::Client,
    provider: &PcgRouterSTTProvider,
    request: &STTRequest,
) -> anyhow::Result<(STTResponse, Option<i64>)> {
    let api_key = provider
        .resolve_api_key()
        .ok_or_else(|| anyhow::anyhow!("OpenAI API key not configured"))?;

    // Decode base64 audio data
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.audio_data)
        .map_err(|e| anyhow::anyhow!("Invalid base64 audio data: {}", e))?;

    let format = request.format.as_deref().unwrap_or("wav");
    let filename = format!("audio.{}", format);
    let mime_type = match format {
        "mp3" => "audio/mpeg",
        "webm" => "audio/webm",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        _ => "audio/wav",
    };

    let language = request
        .language
        .as_deref()
        .map(|l| if l == "en-GB" { "en" } else { l })
        .unwrap_or("en");

    // Build multipart form
    let mut form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(audio_bytes)
                .file_name(filename)
                .mime_str(mime_type)?,
        )
        .text("model", "whisper-1")
        .text("language", language.to_string())
        .text("response_format", "verbose_json");

    if request.word_timestamps.unwrap_or(false) {
        form = form.text("timestamp_granularities[]", "word");
    }

    let url = format!("{}/v1/audio/transcriptions", provider.base_url());

    let response = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI Whisper {}: {}", status, text);
    }

    let body: serde_json::Value = response.json().await?;

    let text = body["text"].as_str().unwrap_or("").to_string();
    let duration_sec = body["duration"].as_f64();
    let audio_duration_ms = duration_sec.map(|d| (d * 1000.0) as i64);

    let word_timestamps = body["words"]
        .as_array()
        .map(|words| {
            words
                .iter()
                .map(|w| WordTimestamp {
                    word: w["word"].as_str().unwrap_or("").to_string(),
                    start_time_ms: (w["start"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
                    end_time_ms: (w["end"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
                    confidence: 1.0, // OpenAI doesn't provide word-level confidence
                })
                .collect()
        })
        .unwrap_or_default();

    Ok((
        STTResponse {
            text,
            confidence: 1.0,
            language: language.to_string(),
            word_timestamps,
        },
        audio_duration_ms,
    ))
}

// ── Azure STT ───────────────────────────────────────────────────────────────

async fn forward_to_azure(
    http: &reqwest::Client,
    provider: &PcgRouterSTTProvider,
    request: &STTRequest,
) -> anyhow::Result<(STTResponse, Option<i64>)> {
    let api_key = provider
        .resolve_api_key()
        .ok_or_else(|| anyhow::anyhow!("Azure Speech key not configured"))?;

    // Decode base64 audio data
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.audio_data)
        .map_err(|e| anyhow::anyhow!("Invalid base64 audio data: {}", e))?;

    let language = request.language.as_deref().unwrap_or("en-US");

    let url = format!(
        "{}/speech/recognition/conversation/cognitiveservices/v1?language={}",
        provider.base_url(),
        language
    );

    let response = http
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", &api_key)
        .header("Content-Type", "audio/wav")
        .body(audio_bytes)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Azure STT {}: {}", status, text);
    }

    let body: serde_json::Value = response.json().await?;

    let text = body["DisplayText"]
        .as_str()
        .or_else(|| body["RecognitionStatus"].as_str())
        .unwrap_or("")
        .to_string();

    let duration_ticks = body["Duration"].as_i64();
    // Azure duration is in 100-nanosecond ticks
    let audio_duration_ms = duration_ticks.map(|t| t / 10_000);

    let confidence = body["NBest"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|best| best["Confidence"].as_f64())
        .map(|c| c as f32)
        .unwrap_or(1.0);

    Ok((
        STTResponse {
            text,
            confidence,
            language: language.to_string(),
            word_timestamps: Vec::new(), // Azure word timestamps require additional parsing
        },
        audio_duration_ms,
    ))
}

// ── Local Whisper ───────────────────────────────────────────────────────────

async fn forward_to_local_whisper(
    http: &reqwest::Client,
    provider: &PcgRouterSTTProvider,
    request: &STTRequest,
) -> anyhow::Result<(STTResponse, Option<i64>)> {
    // Decode base64 audio data
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.audio_data)
        .map_err(|e| anyhow::anyhow!("Invalid base64 audio data: {}", e))?;

    let format = request.format.as_deref().unwrap_or("wav");
    let filename = format!("audio.{}", format);
    let mime_type = match format {
        "mp3" => "audio/mpeg",
        "webm" => "audio/webm",
        _ => "audio/wav",
    };

    let language = request.language.as_deref().unwrap_or("en");

    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(audio_bytes)
                .file_name(filename)
                .mime_str(mime_type)?,
        )
        .text("language", language.to_string());

    let url = format!("{}/transcribe", provider.base_url());

    let response = http.post(&url).multipart(form).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("LocalWhisper {}: {}", status, text);
    }

    let body: serde_json::Value = response.json().await?;

    let text = body["text"].as_str().unwrap_or("").to_string();
    let duration_sec = body["duration"].as_f64();
    let audio_duration_ms = duration_sec.map(|d| (d * 1000.0) as i64);

    let word_timestamps = body["words"]
        .as_array()
        .map(|words| {
            words
                .iter()
                .map(|w| WordTimestamp {
                    word: w["word"].as_str().unwrap_or("").to_string(),
                    start_time_ms: (w["start"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
                    end_time_ms: (w["end"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
                    confidence: w["confidence"].as_f64().unwrap_or(1.0) as f32,
                })
                .collect()
        })
        .unwrap_or_default();

    Ok((
        STTResponse {
            text,
            confidence: 1.0,
            language: language.to_string(),
            word_timestamps,
        },
        audio_duration_ms,
    ))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn calculate_cost(provider: &PcgRouterSTTProvider, audio_duration_ms: Option<i64>) -> Option<i64> {
    let duration_ms = audio_duration_ms?;
    if provider.cost_per_minute == 0 {
        return None;
    }
    let minutes = duration_ms as f64 / 60_000.0;
    let cost = minutes * provider.cost_per_minute as f64;
    Some(cost as i64)
}

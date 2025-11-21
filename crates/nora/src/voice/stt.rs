//! Speech-to-Text implementations adapted from voice-agent-v2

use std::time::Instant;

use async_trait::async_trait;
use base64::engine::Engine;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use ts_rs::TS;

use super::{config::STTConfig, VoiceError, VoiceResult};

/// Transcription result from STT
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub processing_time_ms: u64,
    pub word_timestamps: Vec<WordTimestamp>,
}

/// Word-level timestamp information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WordTimestamp {
    pub word: String,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub confidence: f32,
}

/// Speech-to-Text trait
#[async_trait]
pub trait SpeechToText {
    async fn transcribe_audio(&self, audio_data: &str) -> VoiceResult<TranscriptionResult>;
    async fn transcribe_streaming(
        &self,
        audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> VoiceResult<tokio::sync::mpsc::Receiver<TranscriptionResult>>;
    async fn is_ready(&self) -> bool;
    async fn get_supported_languages(&self) -> VoiceResult<Vec<String>>;
}

/// Whisper STT implementation (adapted from voice-agent-v2)
#[derive(Debug)]
pub struct WhisperSTT {
    config: STTConfig,
    client: reqwest::Client,
}

impl WhisperSTT {
    pub async fn new(config: &STTConfig) -> VoiceResult<Self> {
        info!("Initializing Whisper STT with model: {}", config.model);

        Ok(Self {
            config: config.clone(),
            client: reqwest::Client::new(),
        })
    }

    async fn transcribe_with_openai_whisper(
        &self,
        audio_data: &str,
    ) -> VoiceResult<TranscriptionResult> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| VoiceError::STTError("OpenAI API key not configured".to_string()))?;

        // Decode base64 audio data
        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(audio_data)
            .map_err(|e| VoiceError::STTError(format!("Invalid base64 audio data: {}", e)))?;

        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_bytes)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| {
                        VoiceError::STTError(format!("Failed to create form part: {}", e))
                    })?,
            )
            .text("model", "whisper-1")
            .text(
                "language",
                if self.config.language == "en-GB" {
                    "en".to_string()
                } else {
                    self.config.language.clone()
                },
            )
            .text("response_format", "verbose_json");

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::STTError(format!(
                "OpenAI Whisper API error: {}",
                error_text
            )));
        }

        let whisper_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        let text = whisper_response["text"].as_str().unwrap_or("").to_string();

        let word_timestamps = whisper_response["words"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|word| WordTimestamp {
                word: word["word"].as_str().unwrap_or("").to_string(),
                start_time_ms: (word["start"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
                end_time_ms: (word["end"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
                confidence: 1.0, // OpenAI doesn't provide word-level confidence
            })
            .collect();

        Ok(TranscriptionResult {
            text,
            confidence: 1.0, // OpenAI doesn't provide overall confidence
            language: self.config.language.clone(),
            processing_time_ms: 0, // Will be set by caller
            word_timestamps,
        })
    }

    fn apply_british_vocabulary_corrections(&self, text: &str) -> String {
        if !self.config.british_dialect_support {
            return text.to_string();
        }

        let mut corrected = text.to_string();

        // Common British pronunciation corrections
        let british_corrections = vec![
            ("aluminum", "aluminium"),
            ("program", "programme"),
            ("theater", "theatre"),
            ("center", "centre"),
            ("color", "colour"),
            ("favor", "favour"),
            ("neighbor", "neighbour"),
            ("labor", "labour"),
            ("honor", "honour"),
            ("defense", "defence"),
        ];

        for (american, british) in british_corrections {
            corrected = corrected.replace(american, british);
        }

        // Executive vocabulary enhancements
        if self.config.executive_vocabulary {
            corrected = self.apply_executive_vocabulary_corrections(&corrected);
        }

        corrected
    }

    fn apply_executive_vocabulary_corrections(&self, text: &str) -> String {
        let mut corrected = text.to_string();

        // Common executive terminology corrections
        let executive_corrections = vec![
            ("meeting", "conference"),
            ("talk about", "discuss"),
            ("look at", "review"),
            ("check", "analyse"),
            ("fix", "address"),
            ("problem", "challenge"),
            ("issue", "matter"),
        ];

        for (casual, executive) in executive_corrections {
            // Only replace if it's a whole word
            let pattern = format!(r"\b{}\b", regex::escape(casual));
            let re = regex::Regex::new(&pattern).unwrap();
            corrected = re.replace_all(&corrected, executive).to_string();
        }

        corrected
    }
}

#[async_trait]
impl SpeechToText for WhisperSTT {
    async fn transcribe_audio(&self, audio_data: &str) -> VoiceResult<TranscriptionResult> {
        let start_time = Instant::now();

        info!(
            "Transcribing audio with Whisper (model: {})",
            self.config.model
        );

        let mut result = self.transcribe_with_openai_whisper(audio_data).await?;

        // Apply British vocabulary corrections
        result.text = self.apply_british_vocabulary_corrections(&result.text);

        result.processing_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "Transcription completed in {}ms: '{}'",
            result.processing_time_ms, result.text
        );

        Ok(result)
    }

    async fn transcribe_streaming(
        &self,
        mut audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> VoiceResult<tokio::sync::mpsc::Receiver<TranscriptionResult>> {
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        // Spawn task to handle streaming transcription
        tokio::spawn(async move {
            let mut audio_buffer = Vec::new();

            while let Some(audio_chunk) = audio_stream.recv().await {
                audio_buffer.extend_from_slice(&audio_chunk);

                // Process when we have enough audio data (e.g., 2 seconds worth)
                if audio_buffer.len() >= 32000 {
                    // Assuming 16kHz sample rate, 16-bit
                    let audio_data =
                        base64::engine::general_purpose::STANDARD.encode(&audio_buffer);

                    // This is a simplified implementation - in reality you'd want
                    // to use a proper streaming STT service or local Whisper
                    if let Ok(result) = Self::dummy_transcribe(&audio_data).await {
                        let _ = tx.send(result).await;
                    }

                    audio_buffer.clear();
                }
            }
        });

        Ok(rx)
    }

    async fn is_ready(&self) -> bool {
        std::env::var("OPENAI_API_KEY").is_ok()
    }

    async fn get_supported_languages(&self) -> VoiceResult<Vec<String>> {
        Ok(vec![
            "en".to_string(),
            "en-GB".to_string(),
            "es".to_string(),
            "fr".to_string(),
            "de".to_string(),
            "it".to_string(),
            "pt".to_string(),
            "ru".to_string(),
            "ja".to_string(),
            "ko".to_string(),
            "zh".to_string(),
        ])
    }
}

impl WhisperSTT {
    async fn dummy_transcribe(_audio_data: &str) -> VoiceResult<TranscriptionResult> {
        // Placeholder for streaming transcription
        Ok(TranscriptionResult {
            text: "Streaming transcription result".to_string(),
            confidence: 0.9,
            language: "en-GB".to_string(),
            processing_time_ms: 100,
            word_timestamps: vec![],
        })
    }
}

/// Azure STT implementation
#[derive(Debug)]
pub struct AzureSTT {
    config: STTConfig,
    client: reqwest::Client,
    subscription_key: Option<String>,
    region: String,
}

impl AzureSTT {
    pub async fn new(config: &STTConfig) -> VoiceResult<Self> {
        let subscription_key = std::env::var("AZURE_SPEECH_KEY").ok();
        let region = std::env::var("AZURE_SPEECH_REGION").unwrap_or_else(|_| "eastus".to_string());

        if subscription_key.is_none() {
            warn!("Azure Speech key not found, STT may not work");
        }

        Ok(Self {
            config: config.clone(),
            client: reqwest::Client::new(),
            subscription_key,
            region,
        })
    }
}

#[async_trait]
impl SpeechToText for AzureSTT {
    async fn transcribe_audio(&self, audio_data: &str) -> VoiceResult<TranscriptionResult> {
        let start_time = Instant::now();

        let subscription_key = self.subscription_key.as_ref().ok_or_else(|| {
            VoiceError::STTError("Azure subscription key not configured".to_string())
        })?;

        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(audio_data)
            .map_err(|e| VoiceError::STTError(format!("Invalid base64 audio data: {}", e)))?;

        info!("Transcribing audio with Azure Speech");

        let response = self.client
            .post(&format!("https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1", self.region))
            .header("Ocp-Apim-Subscription-Key", subscription_key)
            .header("Content-Type", "audio/wav")
            .query(&[("language", &self.config.language)])
            .body(audio_bytes)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::STTError(format!(
                "Azure STT error: {}",
                error_text
            )));
        }

        let azure_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        let text = azure_response["DisplayText"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let confidence = azure_response["Confidence"].as_f64().unwrap_or(0.0) as f32;

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text,
            confidence,
            language: self.config.language.clone(),
            processing_time_ms: processing_time,
            word_timestamps: vec![], // Azure doesn't provide detailed timestamps in basic API
        })
    }

    async fn transcribe_streaming(
        &self,
        _audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> VoiceResult<tokio::sync::mpsc::Receiver<TranscriptionResult>> {
        // Azure supports streaming but requires WebSocket implementation
        Err(VoiceError::STTError(
            "Azure streaming STT not implemented yet".to_string(),
        ))
    }

    async fn is_ready(&self) -> bool {
        self.subscription_key.is_some()
    }

    async fn get_supported_languages(&self) -> VoiceResult<Vec<String>> {
        Ok(vec![
            "en-US".to_string(),
            "en-GB".to_string(),
            "es-ES".to_string(),
            "fr-FR".to_string(),
            "de-DE".to_string(),
            "it-IT".to_string(),
            "pt-BR".to_string(),
            "ru-RU".to_string(),
            "ja-JP".to_string(),
            "ko-KR".to_string(),
            "zh-CN".to_string(),
        ])
    }
}

/// Google STT implementation
#[derive(Debug)]
#[allow(dead_code)]
pub struct GoogleSTT {
    config: STTConfig,
    client: reqwest::Client,
}

impl GoogleSTT {
    pub async fn new(config: &STTConfig) -> VoiceResult<Self> {
        info!("Initializing Google STT");

        Ok(Self {
            config: config.clone(),
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl SpeechToText for GoogleSTT {
    async fn transcribe_audio(&self, _audio_data: &str) -> VoiceResult<TranscriptionResult> {
        // Google Cloud Speech-to-Text implementation would go here
        Err(VoiceError::STTError(
            "Google STT not implemented yet".to_string(),
        ))
    }

    async fn transcribe_streaming(
        &self,
        _audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> VoiceResult<tokio::sync::mpsc::Receiver<TranscriptionResult>> {
        Err(VoiceError::STTError(
            "Google streaming STT not implemented yet".to_string(),
        ))
    }

    async fn is_ready(&self) -> bool {
        false // Not implemented yet
    }

    async fn get_supported_languages(&self) -> VoiceResult<Vec<String>> {
        Ok(vec!["en-GB".to_string()])
    }
}

/// System STT implementation (fallback)
#[derive(Debug)]
pub struct SystemSTT {
    config: STTConfig,
}

impl SystemSTT {
    pub async fn new(config: &STTConfig) -> VoiceResult<Self> {
        info!("Initializing system STT (fallback)");
        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait]
impl SpeechToText for SystemSTT {
    async fn transcribe_audio(&self, _audio_data: &str) -> VoiceResult<TranscriptionResult> {
        let start_time = Instant::now();

        warn!("System STT is a placeholder implementation - no real transcription performed");
        warn!("To enable real speech-to-text, configure one of these:");
        warn!("  - OpenAI Whisper: Set OPENAI_API_KEY environment variable");
        warn!("  - Azure Speech: Set AZURE_SPEECH_KEY and AZURE_SPEECH_REGION");
        warn!("  - Google Cloud: Configure Google Cloud credentials");

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text: "This is a dummy transcription from system STT. Configure OPENAI_API_KEY to enable real speech-to-text.".to_string(),
            confidence: 0.0,
            language: self.config.language.clone(),
            processing_time_ms: processing_time,
            word_timestamps: vec![],
        })
    }

    async fn transcribe_streaming(
        &self,
        _audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> VoiceResult<tokio::sync::mpsc::Receiver<TranscriptionResult>> {
        Err(VoiceError::STTError(
            "System streaming STT not implemented".to_string(),
        ))
    }

    async fn is_ready(&self) -> bool {
        true // System STT is always available as fallback
    }

    async fn get_supported_languages(&self) -> VoiceResult<Vec<String>> {
        Ok(vec!["en".to_string(), "en-GB".to_string()])
    }
}

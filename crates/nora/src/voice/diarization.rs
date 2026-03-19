//! Speaker diarization engine using WhisperX
//!
//! Provides speaker identification and segmentation for meeting transcription.
//! Uses WhisperX (installed at `.venv-whisperx/`) via subprocess for
//! word-level aligned transcription with speaker diarization.

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{VoiceError, VoiceResult};

/// A segment of speech with speaker identification
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiarizedSegment {
    /// Transcribed text for this segment
    pub text: String,
    /// Speaker identifier (e.g., "SPEAKER_00", "SPEAKER_01")
    pub speaker_id: String,
    /// Mapped speaker label (e.g., "Speaker 1", "Speaker 2")
    pub speaker_label: String,
    /// Start time in seconds
    pub start: f64,
    /// End time in seconds
    pub end: f64,
    /// Word-level alignment (if available)
    pub words: Vec<AlignedWord>,
}

/// A single word with timing alignment
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AlignedWord {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub speaker: Option<String>,
}

/// WhisperX JSON output format
#[derive(Debug, Deserialize)]
struct WhisperXOutput {
    segments: Vec<WhisperXSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperXSegment {
    text: String,
    start: f64,
    end: f64,
    speaker: Option<String>,
    words: Option<Vec<WhisperXWord>>,
}

#[derive(Debug, Deserialize)]
struct WhisperXWord {
    word: String,
    start: Option<f64>,
    end: Option<f64>,
    speaker: Option<String>,
}

/// Configuration for the diarization engine
#[derive(Debug, Clone)]
pub struct DiarizationConfig {
    /// Path to the WhisperX virtual environment
    pub venv_path: PathBuf,
    /// HuggingFace token for speaker diarization model
    pub hf_token: Option<String>,
    /// Whisper model size (e.g., "base", "small", "medium", "large-v3")
    pub model_size: String,
    /// Language for transcription
    pub language: String,
    /// Number of speakers (if known)
    pub num_speakers: Option<u32>,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self {
            venv_path: PathBuf::from(".venv-whisperx"),
            hf_token: std::env::var("HF_TOKEN").ok(),
            model_size: "base".to_string(),
            language: "en".to_string(),
            num_speakers: None,
        }
    }
}

/// Speaker diarization engine that calls WhisperX via subprocess
pub struct DiarizationEngine {
    config: DiarizationConfig,
    /// Cross-chunk speaker mapping: raw speaker ID -> consistent label
    speaker_map: tokio::sync::RwLock<HashMap<String, String>>,
    /// Counter for assigning speaker labels
    speaker_counter: tokio::sync::RwLock<u32>,
}

impl DiarizationEngine {
    pub fn new(config: DiarizationConfig) -> Self {
        Self {
            config,
            speaker_map: tokio::sync::RwLock::new(HashMap::new()),
            speaker_counter: tokio::sync::RwLock::new(0),
        }
    }

    /// Check if the WhisperX environment is available
    pub async fn is_available(&self) -> bool {
        let python_path = self.config.venv_path.join("bin").join("python");
        python_path.exists()
    }

    /// Diarize an audio file and return speaker-segmented transcript
    pub async fn diarize(&self, audio_data: &[u8]) -> VoiceResult<Vec<DiarizedSegment>> {
        // Write audio to temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("meeting_audio_{}.wav", uuid::Uuid::new_v4()));

        tokio::fs::write(&temp_file, audio_data)
            .await
            .map_err(|e| VoiceError::IoError(e))?;

        let result = self.run_whisperx(&temp_file).await;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_file).await;

        result
    }

    /// Diarize from base64-encoded audio data
    pub async fn diarize_base64(&self, base64_audio: &str) -> VoiceResult<Vec<DiarizedSegment>> {
        use base64::Engine;
        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(base64_audio)
            .map_err(|e| VoiceError::AudioError(format!("Invalid base64 audio: {}", e)))?;

        self.diarize(&audio_bytes).await
    }

    /// Run WhisperX subprocess on an audio file
    async fn run_whisperx(
        &self,
        audio_path: &std::path::Path,
    ) -> VoiceResult<Vec<DiarizedSegment>> {
        let python_path = self.config.venv_path.join("bin").join("python");

        if !python_path.exists() {
            return Err(VoiceError::STTError(format!(
                "WhisperX venv not found at {}. Install with: python -m venv .venv-whisperx && pip install whisperx",
                self.config.venv_path.display()
            )));
        }

        let output_dir = std::env::temp_dir().join("whisperx_output");
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| VoiceError::IoError(e))?;

        let mut cmd_args = vec![
            "-m".to_string(),
            "whisperx".to_string(),
            audio_path.to_string_lossy().to_string(),
            "--diarize".to_string(),
            "--output_format".to_string(),
            "json".to_string(),
            "--output_dir".to_string(),
            output_dir.to_string_lossy().to_string(),
            "--model".to_string(),
            self.config.model_size.clone(),
            "--language".to_string(),
            self.config.language.clone(),
        ];

        if let Some(ref token) = self.config.hf_token {
            cmd_args.push("--hf_token".to_string());
            cmd_args.push(token.clone());
        }

        if let Some(num) = self.config.num_speakers {
            cmd_args.push("--min_speakers".to_string());
            cmd_args.push(num.to_string());
            cmd_args.push("--max_speakers".to_string());
            cmd_args.push(num.to_string());
        }

        let output = tokio::process::Command::new(&python_path)
            .args(&cmd_args)
            .output()
            .await
            .map_err(|e| VoiceError::STTError(format!("Failed to run WhisperX: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VoiceError::STTError(format!("WhisperX failed: {}", stderr)));
        }

        // Find the output JSON file
        let audio_stem = audio_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let json_path = output_dir.join(format!("{}.json", audio_stem));

        let json_content = tokio::fs::read_to_string(&json_path)
            .await
            .map_err(|e| VoiceError::STTError(format!("Failed to read WhisperX output: {}", e)))?;

        // Clean up output
        let _ = tokio::fs::remove_file(&json_path).await;

        // Parse WhisperX JSON output
        let whisperx_output: WhisperXOutput = serde_json::from_str(&json_content)
            .map_err(|e| VoiceError::STTError(format!("Failed to parse WhisperX JSON: {}", e)))?;

        // Convert to DiarizedSegments with consistent speaker mapping
        let mut segments = Vec::new();
        for seg in whisperx_output.segments {
            let speaker_id = seg.speaker.unwrap_or_else(|| "UNKNOWN".to_string());
            let speaker_label = self.map_speaker(&speaker_id).await;

            let words = seg
                .words
                .unwrap_or_default()
                .into_iter()
                .map(|w| AlignedWord {
                    word: w.word,
                    start: w.start.unwrap_or(0.0),
                    end: w.end.unwrap_or(0.0),
                    speaker: w.speaker,
                })
                .collect();

            segments.push(DiarizedSegment {
                text: seg.text.trim().to_string(),
                speaker_id,
                speaker_label,
                start: seg.start,
                end: seg.end,
                words,
            });
        }

        Ok(segments)
    }

    /// Map a raw speaker ID (e.g., "SPEAKER_00") to a consistent label
    async fn map_speaker(&self, speaker_id: &str) -> String {
        let map = self.speaker_map.read().await;
        if let Some(label) = map.get(speaker_id) {
            return label.clone();
        }
        drop(map);

        let mut map = self.speaker_map.write().await;
        // Double-check after acquiring write lock
        if let Some(label) = map.get(speaker_id) {
            return label.clone();
        }

        let mut counter = self.speaker_counter.write().await;
        *counter += 1;
        let label = format!("Speaker {}", *counter);
        map.insert(speaker_id.to_string(), label.clone());
        label
    }

    /// Reset speaker mapping (for new meeting sessions)
    pub async fn reset_speakers(&self) {
        let mut map = self.speaker_map.write().await;
        map.clear();
        let mut counter = self.speaker_counter.write().await;
        *counter = 0;
    }

    /// Get current speaker count
    pub async fn speaker_count(&self) -> u32 {
        *self.speaker_counter.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_speaker_mapping() {
        let config = DiarizationConfig::default();
        let engine = DiarizationEngine::new(config);

        let label1 = engine.map_speaker("SPEAKER_00").await;
        let label2 = engine.map_speaker("SPEAKER_01").await;
        let label1_again = engine.map_speaker("SPEAKER_00").await;

        assert_eq!(label1, "Speaker 1");
        assert_eq!(label2, "Speaker 2");
        assert_eq!(label1_again, "Speaker 1"); // Same speaker, same label

        assert_eq!(engine.speaker_count().await, 2);

        engine.reset_speakers().await;
        assert_eq!(engine.speaker_count().await, 0);
    }

    #[tokio::test]
    async fn test_availability() {
        let config = DiarizationConfig {
            venv_path: PathBuf::from("/nonexistent/path"),
            ..Default::default()
        };
        let engine = DiarizationEngine::new(config);
        assert!(!engine.is_available().await);
    }
}

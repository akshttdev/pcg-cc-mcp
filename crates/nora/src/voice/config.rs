//! Voice configuration adapted from voice-agent-v2

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Comprehensive voice configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceConfig {
    pub tts: TTSConfig,
    pub stt: STTConfig,
    pub audio: AudioConfig,
    pub british_accent: BritishAccentConfig,
    pub executive_mode: ExecutiveModeConfig,
}

impl VoiceConfig {
    /// Default configuration for British executive assistant
    pub fn british_executive() -> Self {
        Self {
            tts: TTSConfig::british_executive(),
            stt: STTConfig::high_accuracy(),
            audio: AudioConfig::high_quality(),
            british_accent: BritishAccentConfig::professional(),
            executive_mode: ExecutiveModeConfig::enabled(),
        }
    }

    /// Configuration for development/testing
    pub fn development() -> Self {
        Self {
            tts: TTSConfig::development(),
            stt: STTConfig::fast(),
            audio: AudioConfig::standard(),
            british_accent: BritishAccentConfig::standard(),
            executive_mode: ExecutiveModeConfig::disabled(),
        }
    }
}

/// Text-to-Speech configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TTSConfig {
    pub provider: TTSProvider,
    pub voice_id: String,
    pub speed: f32,
    pub volume: f32,
    pub pitch: f32,
    pub quality: TTSQuality,
    pub british_voice_preferences: Vec<String>,
    pub fallback_providers: Vec<TTSProvider>,
}

impl TTSConfig {
    pub fn british_executive() -> Self {
        Self {
            provider: TTSProvider::Chatterbox, // Use local Chatterbox as primary
            voice_id: "british_female".to_string(), // British voice reference
            speed: 1.0,
            volume: 0.85,
            pitch: 1.0,
            quality: TTSQuality::High,
            british_voice_preferences: vec![
                "british_female".to_string(),    // Chatterbox British voice
                "fable".to_string(),             // OpenAI British-leaning female
                "nova".to_string(),              // OpenAI warm female
                "echo".to_string(),              // OpenAI clear male
                "Rachel".to_string(),            // ElevenLabs British voice
                "en-GB-SoniaNeural".to_string(), // Azure British
            ],
            fallback_providers: vec![
                TTSProvider::OpenAI,
                TTSProvider::ElevenLabs,
                TTSProvider::Azure,
                TTSProvider::System,
            ],
        }
    }

    pub fn development() -> Self {
        Self {
            provider: TTSProvider::Chatterbox, // Use local Chatterbox for dev too
            voice_id: "british_female".to_string(), // British voice reference
            speed: 1.0,                    // Normal speed for better comprehension
            volume: 0.8,
            pitch: 1.0,
            quality: TTSQuality::High, // Use high quality even in dev
            british_voice_preferences: vec![
                "british_female".to_string(),
                "fable".to_string(),
                "nova".to_string(),
                "echo".to_string(),
            ],
            fallback_providers: vec![TTSProvider::System],
        }
    }
}

/// Speech-to-Text configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct STTConfig {
    pub provider: STTProvider,
    pub model: String,
    pub language: String,
    pub british_dialect_support: bool,
    pub executive_vocabulary: bool,
    pub real_time: bool,
    pub noise_reduction: bool,
}

impl STTConfig {
    pub fn high_accuracy() -> Self {
        Self {
            provider: STTProvider::LocalWhisper,  // Force local Whisper (Sovereign Stack)
            model: "base".to_string(),  // Use base model for local Whisper
            language: "en-GB".to_string(),
            british_dialect_support: true,
            executive_vocabulary: true,
            real_time: false,
            noise_reduction: true,
        }
    }

    pub fn fast() -> Self {
        Self {
            provider: STTProvider::Whisper,
            model: "base".to_string(),
            language: "en-GB".to_string(),
            british_dialect_support: true,
            executive_vocabulary: false,
            real_time: true,
            noise_reduction: false,
        }
    }
}

/// Audio processing configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub buffer_size: usize,
    pub noise_suppression: bool,
    pub echo_cancellation: bool,
    pub auto_gain_control: bool,
}

impl AudioConfig {
    pub fn high_quality() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            bit_depth: 16,
            buffer_size: 2048,
            noise_suppression: true,
            echo_cancellation: true,
            auto_gain_control: true,
        }
    }

    pub fn standard() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            bit_depth: 16,
            buffer_size: 1024,
            noise_suppression: false,
            echo_cancellation: false,
            auto_gain_control: false,
        }
    }
}

/// British accent configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BritishAccentConfig {
    pub accent_strength: f32,
    pub regional_variant: BritishRegion,
    pub formality_level: FormalityLevel,
    pub vocabulary_preferences: BritishVocabulary,
}

impl BritishAccentConfig {
    pub fn professional() -> Self {
        Self {
            accent_strength: 0.8,
            regional_variant: BritishRegion::ReceivedPronunciation,
            formality_level: FormalityLevel::Professional,
            vocabulary_preferences: BritishVocabulary::Executive,
        }
    }

    pub fn standard() -> Self {
        Self {
            accent_strength: 0.6,
            regional_variant: BritishRegion::GeneralBritish,
            formality_level: FormalityLevel::Neutral,
            vocabulary_preferences: BritishVocabulary::Standard,
        }
    }
}

/// Executive mode configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutiveModeConfig {
    pub enabled: bool,
    pub proactive_communication: bool,
    pub executive_summary_style: bool,
    pub formal_address: bool,
    pub business_vocabulary: bool,
}

impl ExecutiveModeConfig {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            proactive_communication: true,
            executive_summary_style: true,
            formal_address: true,
            business_vocabulary: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            proactive_communication: false,
            executive_summary_style: false,
            formal_address: false,
            business_vocabulary: false,
        }
    }
}

/// TTS provider options
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TTSProvider {
    ElevenLabs,
    Azure,
    OpenAI,
    System,
    Google,
    Amazon,
    /// Chatterbox local TTS (resemble-ai/chatterbox)
    Chatterbox,
}

/// STT provider options
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum STTProvider {
    Whisper,
    /// Local Whisper server (no API key needed)
    LocalWhisper,
    Azure,
    Google,
    System,
}

/// TTS quality levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TTSQuality {
    Low,
    Medium,
    High,
    Premium,
}

/// British regional variants
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum BritishRegion {
    ReceivedPronunciation,
    GeneralBritish,
    London,
    Scottish,
    Welsh,
    NorthernEnglish,
}

/// Formality levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FormalityLevel {
    Casual,
    Neutral,
    Professional,
    VeryFormal,
}

/// British vocabulary preferences
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum BritishVocabulary {
    Standard,
    Executive,
    Academic,
    Legal,
    Technical,
}

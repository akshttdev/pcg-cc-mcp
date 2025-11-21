//! Main voice engine implementation adapted from voice-agent-v2

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{
    config::{STTProvider, TTSProvider, VoiceConfig},
    stt::SpeechToText,
    tts::{TextToSpeech, VoiceProfile},
    AudioFormat, SpeechRequest, VoiceError, VoiceResult,
};

/// Main voice processing engine for Nora
pub struct VoiceEngine {
    config: VoiceConfig,
    tts: Arc<dyn TextToSpeech + Send + Sync>,
    stt: Arc<dyn SpeechToText + Send + Sync>,
    is_initialized: Arc<RwLock<bool>>,
    active_sessions: Arc<RwLock<std::collections::HashMap<String, VoiceSession>>>,
}

/// Voice session state
#[derive(Debug, Clone)]
pub struct VoiceSession {
    pub session_id: String,
    pub voice_profile: VoiceProfile,
    pub is_listening: bool,
    pub is_speaking: bool,
    pub last_interaction: chrono::DateTime<chrono::Utc>,
}

impl VoiceEngine {
    /// Create a new voice engine with configuration
    pub async fn new(config: VoiceConfig) -> VoiceResult<Self> {
        info!("Initializing Nora voice engine...");

        // Initialize TTS provider
        let tts = Self::create_tts_provider(&config).await?;

        // Initialize STT provider
        let stt = Self::create_stt_provider(&config).await?;

        let engine = Self {
            config,
            tts,
            stt,
            is_initialized: Arc::new(RwLock::new(false)),
            active_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        };

        // Mark as initialized
        *engine.is_initialized.write().await = true;

        info!("Voice engine initialized successfully");
        Ok(engine)
    }

    /// Synthesize speech from text with British executive personality
    pub async fn synthesize_speech(&self, text: &str) -> VoiceResult<String> {
        if !*self.is_initialized.read().await {
            return Err(VoiceError::NotInitialized);
        }

        // Use the configured voice_id to determine the profile
        // This allows dynamic voice switching based on user preferences
        let voice_profile = Self::voice_id_to_profile(&self.config.tts.voice_id);

        let request = SpeechRequest {
            text: text.to_string(),
            voice_profile,
            speed: self.config.tts.speed,
            volume: self.config.tts.volume,
            format: AudioFormat::Wav,
            british_accent: true,
            executive_tone: true,
        };

        let start_time = std::time::Instant::now();

        info!("Synthesizing speech: {} chars", text.len());

        // Apply British executive personality to text before synthesis
        let processed_text = self.apply_british_executive_style(&request.text);

        let mut modified_request = request;
        modified_request.text = processed_text;

        let response = self.tts.synthesize_speech(modified_request).await?;

        let processing_time = start_time.elapsed().as_millis();
        info!("Speech synthesis completed in {}ms", processing_time);

        Ok(response.audio_data)
    }

    /// Transcribe speech to text with British dialect support
    pub async fn transcribe_speech(&self, audio_data: &str) -> VoiceResult<String> {
        if !*self.is_initialized.read().await {
            return Err(VoiceError::NotInitialized);
        }

        let start_time = std::time::Instant::now();

        info!("Transcribing speech...");

        let result = self.stt.transcribe_audio(audio_data).await?;

        let processing_time = start_time.elapsed().as_millis();
        info!(
            "Speech transcription completed in {}ms: '{}'",
            processing_time, result.text
        );

        // Apply British vocabulary normalization if needed
        let normalized_text = self.normalize_british_vocabulary(&result.text);

        Ok(normalized_text)
    }

    /// Start a new voice session
    pub async fn start_session(
        &self,
        session_id: String,
        voice_profile: VoiceProfile,
    ) -> VoiceResult<()> {
        let session = VoiceSession {
            session_id: session_id.clone(),
            voice_profile,
            is_listening: false,
            is_speaking: false,
            last_interaction: chrono::Utc::now(),
        };

        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session_id.clone(), session);

        info!("Voice session started: {}", session_id);
        Ok(())
    }

    /// End a voice session
    pub async fn end_session(&self, session_id: &str) -> VoiceResult<()> {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(session_id);

        info!("Voice session ended: {}", session_id);
        Ok(())
    }

    /// Get active voice sessions
    pub async fn get_active_sessions(&self) -> VoiceResult<Vec<VoiceSession>> {
        let sessions = self.active_sessions.read().await;
        Ok(sessions.values().cloned().collect())
    }

    /// Check if voice engine is ready
    pub async fn is_ready(&self) -> bool {
        *self.is_initialized.read().await
    }

    // Private helper methods

    /// Convert voice_id string to VoiceProfile enum
    fn voice_id_to_profile(voice_id: &str) -> VoiceProfile {
        match voice_id {
            "fable" | "nova" | "shimmer" => VoiceProfile::BritishExecutiveFemale,
            "echo" | "onyx" => VoiceProfile::BritishExecutiveMale,
            "alloy" => VoiceProfile::SystemDefault,
            // ElevenLabs voices
            "Rachel" | "british_executive_female" => VoiceProfile::BritishExecutiveFemale,
            "british_executive_male" => VoiceProfile::BritishExecutiveMale,
            // Azure voices
            "en-GB-SoniaNeural" | "en-GB-LibbyNeural" => VoiceProfile::BritishProfessionalFemale,
            "en-GB-RyanNeural" | "en-GB-ThomasNeural" => VoiceProfile::BritishProfessionalMale,
            // Default to British executive female for unknown voices
            _ => VoiceProfile::BritishExecutiveFemale,
        }
    }

    async fn create_tts_provider(
        config: &VoiceConfig,
    ) -> VoiceResult<Arc<dyn TextToSpeech + Send + Sync>> {
        match config.tts.provider {
            TTSProvider::ElevenLabs => {
                info!("Creating ElevenLabs TTS provider");
                Ok(Arc::new(super::tts::ElevenLabsTTS::new(&config.tts).await?))
            }
            TTSProvider::Azure => {
                info!("Creating Azure TTS provider");
                Ok(Arc::new(super::tts::AzureTTS::new(&config.tts).await?))
            }
            TTSProvider::OpenAI => {
                info!("Creating OpenAI TTS provider");
                Ok(Arc::new(super::tts::OpenAITTS::new(&config.tts).await?))
            }
            TTSProvider::System => {
                info!("Creating System TTS provider");
                Ok(Arc::new(super::tts::SystemTTS::new(&config.tts).await?))
            }
            _ => {
                warn!("Unsupported TTS provider, falling back to System TTS");
                Ok(Arc::new(super::tts::SystemTTS::new(&config.tts).await?))
            }
        }
    }

    async fn create_stt_provider(
        config: &VoiceConfig,
    ) -> VoiceResult<Arc<dyn SpeechToText + Send + Sync>> {
        match config.stt.provider {
            STTProvider::Whisper => {
                // Check if OpenAI API key is available
                if std::env::var("OPENAI_API_KEY").is_ok() {
                    info!("Creating Whisper STT provider with OpenAI API key");
                    Ok(Arc::new(super::stt::WhisperSTT::new(&config.stt).await?))
                } else {
                    warn!("OpenAI API key not found, falling back to System STT");
                    warn!("Set OPENAI_API_KEY environment variable to enable Whisper STT");
                    Ok(Arc::new(super::stt::SystemSTT::new(&config.stt).await?))
                }
            }
            STTProvider::Azure => {
                // Check if Azure credentials are available
                if std::env::var("AZURE_SPEECH_KEY").is_ok() {
                    info!("Creating Azure STT provider");
                    Ok(Arc::new(super::stt::AzureSTT::new(&config.stt).await?))
                } else {
                    warn!("Azure Speech credentials not found, falling back to System STT");
                    warn!("Set AZURE_SPEECH_KEY and AZURE_SPEECH_REGION to enable Azure STT");
                    Ok(Arc::new(super::stt::SystemSTT::new(&config.stt).await?))
                }
            }
            STTProvider::Google => {
                info!("Creating Google STT provider");
                warn!("Google STT not fully implemented, falling back to System STT");
                Ok(Arc::new(super::stt::SystemSTT::new(&config.stt).await?))
            }
            STTProvider::System => {
                info!("Creating System STT provider (placeholder)");
                warn!("System STT returns dummy transcriptions - configure a real STT provider");
                Ok(Arc::new(super::stt::SystemSTT::new(&config.stt).await?))
            }
        }
    }

    fn apply_british_executive_style(&self, text: &str) -> String {
        if !self
            .config
            .british_accent
            .vocabulary_preferences
            .is_executive()
        {
            return text.to_string();
        }

        let mut processed = text.to_string();

        // Apply British vocabulary substitutions
        let british_replacements = vec![
            ("elevator", "lift"),
            ("apartment", "flat"),
            ("vacation", "holiday"),
            ("schedule", "programme"),
            ("analyze", "analyse"),
            ("realize", "realise"),
            ("organize", "organise"),
            ("color", "colour"),
            ("favor", "favour"),
            ("center", "centre"),
            ("theater", "theatre"),
            ("defense", "defence"),
        ];

        for (american, british) in british_replacements {
            processed = processed.replace(american, british);
        }

        // Apply executive formality
        if self.config.executive_mode.formal_address {
            processed = self.apply_executive_formality(&processed);
        }

        processed
    }

    fn apply_executive_formality(&self, text: &str) -> String {
        let mut processed = text.to_string();

        // Add executive courtesy phrases
        if !processed.starts_with("I")
            && !processed.starts_with("May I")
            && !processed.starts_with("Allow me")
        {
            if processed.contains("suggest") || processed.contains("recommend") {
                processed = format!("May I suggest that {}", processed.to_lowercase());
            } else if processed.contains("think") || processed.contains("believe") {
                processed = format!("I believe {}", processed);
            }
        }

        // Add executive closing phrases for longer responses
        if processed.len() > 100
            && !processed.contains("Please")
            && !processed.contains("Thank you")
        {
            processed = format!(
                "{}. Please let me know if you require any further assistance.",
                processed
            );
        }

        processed
    }

    fn normalize_british_vocabulary(&self, text: &str) -> String {
        // Normalize British pronunciation variations that might be transcribed differently
        let mut normalized = text.to_string();

        // Common British pronunciation normalizations
        let normalizations = vec![
            ("aluminium", "aluminum"), // Reverse for consistency with international usage
            ("colour", "color"),       // Keep American spelling for data consistency
            ("programme", "program"),  // Technical context
        ];

        for (british, normalized_form) in normalizations {
            normalized = normalized.replace(british, normalized_form);
        }

        normalized
    }
}

// Trait implementations for British vocabulary
trait BritishVocabularyExt {
    fn is_executive(&self) -> bool;
}

impl BritishVocabularyExt for super::config::BritishVocabulary {
    fn is_executive(&self) -> bool {
        matches!(self, super::config::BritishVocabulary::Executive)
    }
}

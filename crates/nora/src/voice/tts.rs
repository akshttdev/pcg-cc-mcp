//! Text-to-Speech implementations adapted from voice-agent-v2

use std::{collections::HashMap, time::Instant};

use async_trait::async_trait;
use base64::engine::Engine;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use ts_rs::TS;

use super::{
    config::TTSConfig, AudioFormat, SpeechRequest, SpeechResponse, VoiceError, VoiceResult,
};

/// Voice profile options for Nora
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum VoiceProfile {
    BritishExecutiveFemale,
    BritishExecutiveMale,
    BritishProfessionalFemale,
    BritishProfessionalMale,
    SystemDefault,
}

impl VoiceProfile {
    pub fn is_british(&self) -> bool {
        matches!(
            self,
            VoiceProfile::BritishExecutiveFemale
                | VoiceProfile::BritishExecutiveMale
                | VoiceProfile::BritishProfessionalFemale
                | VoiceProfile::BritishProfessionalMale
        )
    }

    pub fn is_executive(&self) -> bool {
        matches!(
            self,
            VoiceProfile::BritishExecutiveFemale | VoiceProfile::BritishExecutiveMale
        )
    }
}

/// Text-to-Speech trait
#[async_trait]
pub trait TextToSpeech {
    async fn synthesize_speech(&self, request: SpeechRequest) -> VoiceResult<SpeechResponse>;
    async fn get_available_voices(&self) -> VoiceResult<Vec<String>>;
    async fn is_ready(&self) -> bool;
}

/// ElevenLabs TTS implementation (premium British voices)
#[derive(Debug)]
pub struct ElevenLabsTTS {
    config: TTSConfig,
    client: reqwest::Client,
    api_key: Option<String>,
    voice_map: HashMap<String, String>,
}

impl ElevenLabsTTS {
    pub async fn new(config: &TTSConfig) -> VoiceResult<Self> {
        let api_key = std::env::var("ELEVENLABS_API_KEY").ok();

        if api_key.is_none() {
            warn!("ElevenLabs API key not found, TTS may not work");
        }

        let client = reqwest::Client::new();
        let voice_map = if let Some(ref key) = api_key {
            Self::load_voice_catalog(&client, key).await
        } else {
            HashMap::new()
        };

        Ok(Self {
            config: config.clone(),
            client,
            api_key,
            voice_map,
        })
    }

    fn get_voice_id_for_profile(&self, profile: &VoiceProfile) -> String {
        let configured = self.config.voice_id.trim();

        let preferred =
            if configured.is_empty() || configured.eq_ignore_ascii_case("system_british") {
                self.default_voice_alias(profile)
            } else {
                configured
            };

        let canonical = self.alias_to_voice_name(preferred, profile);

        if let Some(resolved) = self.lookup_voice_identifier(&canonical) {
            return resolved;
        }

        if let Some(resolved) = self.lookup_voice_identifier(preferred) {
            return resolved;
        }

        if let Some(fallback) = self.voice_map.values().next() {
            warn!(
                "ElevenLabs voice '{}' could not be resolved; falling back to '{}'",
                preferred, fallback
            );
            return fallback.clone();
        }

        warn!(
            "ElevenLabs voice '{}' is not recognized and no catalog is cached; the API may reject the request", preferred
        );

        canonical
    }

    fn default_voice_alias(&self, profile: &VoiceProfile) -> &str {
        match profile {
            VoiceProfile::BritishExecutiveFemale => "british_executive_female",
            VoiceProfile::BritishExecutiveMale => "british_executive_male",
            VoiceProfile::BritishProfessionalFemale => "british_professional_female",
            VoiceProfile::BritishProfessionalMale => "british_professional_male",
            VoiceProfile::SystemDefault => "system_default",
        }
    }

    fn alias_to_voice_name(&self, alias: &str, profile: &VoiceProfile) -> String {
        if alias.trim().is_empty() {
            return self.default_voice_alias(profile).to_string();
        }

        match alias {
            "british_executive_female" => "Rachel".to_string(),
            "british_executive_male" => "Brian".to_string(),
            "british_professional_female" => "Bella".to_string(),
            "british_professional_male" => "Charlie".to_string(),
            other => other.to_string(),
        }
    }

    fn lookup_voice_identifier(&self, key: &str) -> Option<String> {
        if key.is_empty() {
            return None;
        }

        if let Some(resolved) = self.voice_map.get(key) {
            return Some(resolved.clone());
        }

        let lower = key.to_lowercase();
        if let Some(resolved) = self.voice_map.get(&lower) {
            return Some(resolved.clone());
        }

        None
    }

    async fn load_voice_catalog(
        client: &reqwest::Client,
        api_key: &str,
    ) -> HashMap<String, String> {
        let mut voice_map = HashMap::new();

        match client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", api_key)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(payload) => {
                            if let Some(voices) = payload["voices"].as_array() {
                                for voice in voices {
                                    if let Some(id) = voice["voice_id"].as_str() {
                                        let id = id.to_string();
                                        voice_map.insert(id.clone(), id.clone());

                                        if let Some(name) = voice["name"].as_str() {
                                            voice_map.insert(name.to_string(), id.clone());
                                            voice_map.insert(name.to_lowercase(), id.clone());
                                        }
                                    }
                                }

                                let alias_plan: Vec<(&str, Vec<(Option<&str>, Option<&str>)>)> = vec![
                                    (
                                        "british_executive_female",
                                        vec![
                                            (Some("british"), Some("female")),
                                            (Some("british"), None),
                                        ],
                                    ),
                                    (
                                        "british_executive_male",
                                        vec![
                                            (Some("british"), Some("male")),
                                            (Some("british"), None),
                                        ],
                                    ),
                                    (
                                        "british_professional_female",
                                        vec![
                                            (Some("british"), Some("female")),
                                            (Some("british"), None),
                                        ],
                                    ),
                                    (
                                        "british_professional_male",
                                        vec![
                                            (Some("british"), Some("male")),
                                            (Some("british"), None),
                                        ],
                                    ),
                                    (
                                        "system_default",
                                        vec![(Some("british"), None), (None, None)],
                                    ),
                                ];

                                for (alias, criteria_list) in alias_plan {
                                    if voice_map.contains_key(alias) {
                                        continue;
                                    }

                                    for (accent, gender) in criteria_list {
                                        if let Some(id) = Self::select_voice(voices, accent, gender)
                                        {
                                            voice_map.insert(alias.to_string(), id.clone());
                                            break;
                                        }
                                    }

                                    if !voice_map.contains_key(alias) {
                                        warn!(
                                            "No ElevenLabs voice matched alias '{}' with preferred criteria; voice selection will fall back",
                                            alias
                                        );
                                    }
                                }
                            }
                        }
                        Err(err) => warn!("Failed to parse ElevenLabs voice catalog: {}", err),
                    }
                } else {
                    let status = response.status();
                    match response.text().await {
                        Ok(body) => warn!(
                            "Failed to load ElevenLabs voice catalog (status {}): {}",
                            status,
                            body
                        ),
                        Err(err) => warn!(
                            "Failed to load ElevenLabs voice catalog (status {}): could not read body: {}",
                            status,
                            err
                        ),
                    }
                }
            }
            Err(err) => warn!("Failed to fetch ElevenLabs voice catalog: {}", err),
        }

        voice_map
    }

    fn select_voice(
        voices: &[serde_json::Value],
        accent: Option<&str>,
        gender: Option<&str>,
    ) -> Option<String> {
        voices.iter().find_map(|voice| {
            let accent_matches = match (accent, voice["labels"]["accent"].as_str()) {
                (Some(target), Some(actual)) => actual.eq_ignore_ascii_case(target),
                (Some(_), None) => false,
                _ => true,
            };

            let gender_matches = match (gender, voice["labels"]["gender"].as_str()) {
                (Some(target), Some(actual)) => actual.eq_ignore_ascii_case(target),
                (Some(_), None) => false,
                _ => true,
            };

            if accent_matches && gender_matches {
                voice["voice_id"].as_str().map(|id| id.to_string())
            } else {
                None
            }
        })
    }
}

#[async_trait]
impl TextToSpeech for ElevenLabsTTS {
    async fn synthesize_speech(&self, request: SpeechRequest) -> VoiceResult<SpeechResponse> {
        let start_time = Instant::now();

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| VoiceError::TTSError("ElevenLabs API key not configured".to_string()))?;

        let voice_id = self.get_voice_id_for_profile(&request.voice_profile);

        info!("Synthesizing speech with ElevenLabs voice: {}", voice_id);

        // Map speed to ElevenLabs speed parameter (0.7 to 1.2)
        let eleven_labs_speed = (request.speed * 1.0).max(0.7).min(1.2);

        let payload = serde_json::json!({
            "text": request.text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.8,
                "style": if request.executive_tone { 0.8 } else { 0.5 },
                "use_speaker_boost": request.executive_tone,
                "speed": eleven_labs_speed
            }
        });

        let response = self
            .client
            .post(&format!(
                "https://api.elevenlabs.io/v1/text-to-speech/{}",
                voice_id
            ))
            .header("Accept", "audio/mpeg")
            .header("Content-Type", "application/json")
            .header("xi-api-key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::TTSError(format!(
                "ElevenLabs API error: {}",
                error_text
            )));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(SpeechResponse {
            audio_data,
            duration_ms: self.estimate_duration(&request.text),
            sample_rate: 22050, // ElevenLabs default
            format: AudioFormat::Mp3,
            processing_time_ms: processing_time,
        })
    }

    async fn get_available_voices(&self) -> VoiceResult<Vec<String>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| VoiceError::TTSError("ElevenLabs API key not configured".to_string()))?;

        let response = self
            .client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", api_key)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            return Err(VoiceError::TTSError("Failed to fetch voices".to_string()));
        }

        let voices: serde_json::Value = response
            .json()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        let voice_names = voices["voices"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(voice_names)
    }

    async fn is_ready(&self) -> bool {
        self.api_key.is_some()
    }
}

impl ElevenLabsTTS {
    fn estimate_duration(&self, text: &str) -> u64 {
        // Rough estimation: average speaking rate is about 150 words per minute
        let word_count = text.split_whitespace().count();
        let minutes = word_count as f64 / 150.0;
        (minutes * 60.0 * 1000.0) as u64 // Convert to milliseconds
    }
}

/// Azure Cognitive Services TTS implementation
#[derive(Debug)]
pub struct AzureTTS {
    #[allow(dead_code)]
    config: TTSConfig,
    client: reqwest::Client,
    subscription_key: Option<String>,
    region: String,
}

impl AzureTTS {
    pub async fn new(config: &TTSConfig) -> VoiceResult<Self> {
        let subscription_key = std::env::var("AZURE_SPEECH_KEY").ok();
        let region = std::env::var("AZURE_SPEECH_REGION").unwrap_or_else(|_| "eastus".to_string());

        if subscription_key.is_none() {
            warn!("Azure Speech key not found, TTS may not work");
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
impl TextToSpeech for AzureTTS {
    async fn synthesize_speech(&self, request: SpeechRequest) -> VoiceResult<SpeechResponse> {
        let start_time = Instant::now();

        let subscription_key = self.subscription_key.as_ref().ok_or_else(|| {
            VoiceError::TTSError("Azure subscription key not configured".to_string())
        })?;

        let voice_name = match request.voice_profile {
            VoiceProfile::BritishExecutiveFemale => "en-GB-SoniaNeural",
            VoiceProfile::BritishExecutiveMale => "en-GB-RyanNeural",
            VoiceProfile::BritishProfessionalFemale => "en-GB-LibbyNeural",
            VoiceProfile::BritishProfessionalMale => "en-GB-ThomasNeural",
            VoiceProfile::SystemDefault => "en-GB-SoniaNeural",
        };

        let ssml = format!(
            r#"<speak version='1.0' xml:lang='en-GB'>
                <voice xml:lang='en-GB' name='{}'>
                    <prosody rate='{}' volume='{}'>
                        {}
                    </prosody>
                </voice>
            </speak>"#,
            voice_name,
            if request.speed > 1.0 {
                "fast"
            } else if request.speed < 1.0 {
                "slow"
            } else {
                "medium"
            },
            if request.volume > 0.8 {
                "loud"
            } else if request.volume < 0.5 {
                "soft"
            } else {
                "medium"
            },
            request.text
        );

        info!("Synthesizing speech with Azure voice: {}", voice_name);

        let response = self
            .client
            .post(&format!(
                "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
                self.region
            ))
            .header("Ocp-Apim-Subscription-Key", subscription_key)
            .header("Content-Type", "application/ssml+xml")
            .header("X-Microsoft-OutputFormat", "riff-24khz-16bit-mono-pcm")
            .body(ssml)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::TTSError(format!(
                "Azure TTS error: {}",
                error_text
            )));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(SpeechResponse {
            audio_data,
            duration_ms: self.estimate_duration(&request.text),
            sample_rate: 24000,
            format: AudioFormat::Wav,
            processing_time_ms: processing_time,
        })
    }

    async fn get_available_voices(&self) -> VoiceResult<Vec<String>> {
        // Return known British voices for Azure
        Ok(vec![
            "en-GB-SoniaNeural".to_string(),
            "en-GB-RyanNeural".to_string(),
            "en-GB-LibbyNeural".to_string(),
            "en-GB-ThomasNeural".to_string(),
        ])
    }

    async fn is_ready(&self) -> bool {
        self.subscription_key.is_some()
    }
}

impl AzureTTS {
    fn estimate_duration(&self, text: &str) -> u64 {
        let word_count = text.split_whitespace().count();
        let minutes = word_count as f64 / 150.0;
        (minutes * 60.0 * 1000.0) as u64
    }
}

/// OpenAI TTS implementation
#[derive(Debug)]
pub struct OpenAITTS {
    config: TTSConfig,
    client: reqwest::Client,
    api_key: Option<String>,
}

impl OpenAITTS {
    pub async fn new(config: &TTSConfig) -> VoiceResult<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").ok();

        if api_key.is_none() {
            warn!("OpenAI API key not found, TTS may not work");
        }

        Ok(Self {
            config: config.clone(),
            client: reqwest::Client::new(),
            api_key,
        })
    }
}

#[async_trait]
impl TextToSpeech for OpenAITTS {
    async fn synthesize_speech(&self, request: SpeechRequest) -> VoiceResult<SpeechResponse> {
        let start_time = Instant::now();

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| VoiceError::TTSError("OpenAI API key not configured".to_string()))?;

        // Use the configured voice_id if it's a valid OpenAI voice, otherwise map from profile
        let valid_voices = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];
        let voice = if valid_voices.contains(&self.config.voice_id.as_str()) {
            self.config.voice_id.as_str()
        } else {
            // Fall back to profile-based mapping for non-OpenAI voice IDs
            match request.voice_profile {
                VoiceProfile::BritishExecutiveFemale => "fable",  // Most British-sounding female
                VoiceProfile::BritishProfessionalFemale => "nova", // Warm professional female
                VoiceProfile::BritishExecutiveMale => "echo",     // Clear, authoritative male
                VoiceProfile::BritishProfessionalMale => "onyx",  // Deep, professional male
                VoiceProfile::SystemDefault => "fable",           // Default to British-leaning voice
            }
        };

        info!("Synthesizing speech with OpenAI voice: {} (config voice_id: {}, profile: {:?})", 
              voice, self.config.voice_id, request.voice_profile);

        let payload = serde_json::json!({
            "model": "tts-1-hd",
            "input": request.text,
            "voice": voice,
            "response_format": "mp3",
            "speed": request.speed
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::TTSError(format!(
                "OpenAI TTS error: {}",
                error_text
            )));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        let audio_data = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(SpeechResponse {
            audio_data,
            duration_ms: self.estimate_duration(&request.text),
            sample_rate: 24000,
            format: AudioFormat::Mp3,
            processing_time_ms: processing_time,
        })
    }

    async fn get_available_voices(&self) -> VoiceResult<Vec<String>> {
        Ok(vec![
            "alloy".to_string(),
            "echo".to_string(),
            "fable".to_string(),
            "onyx".to_string(),
            "nova".to_string(),
            "shimmer".to_string(),
        ])
    }

    async fn is_ready(&self) -> bool {
        self.api_key.is_some()
    }
}

impl OpenAITTS {
    fn estimate_duration(&self, text: &str) -> u64 {
        let word_count = text.split_whitespace().count();
        let minutes = word_count as f64 / 150.0;
        (minutes * 60.0 * 1000.0) as u64
    }
}

/// System TTS implementation (fallback)
#[derive(Debug)]
pub struct SystemTTS {
    #[allow(dead_code)]
    config: TTSConfig,
}

impl SystemTTS {
    pub async fn new(config: &TTSConfig) -> VoiceResult<Self> {
        info!("Initializing system TTS (fallback)");
        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait]
impl TextToSpeech for SystemTTS {
    async fn synthesize_speech(&self, request: SpeechRequest) -> VoiceResult<SpeechResponse> {
        let start_time = Instant::now();

        info!("Synthesizing speech with system TTS");

        // This is a placeholder implementation
        // In a real implementation, you would use system TTS libraries
        // For now, we'll return a dummy response
        let dummy_audio = vec![0u8; 1024]; // 1KB of silence
        let audio_data = base64::engine::general_purpose::STANDARD.encode(&dummy_audio);

        let processing_time = start_time.elapsed().as_millis() as u64;

        warn!("System TTS returning dummy audio data - implement actual TTS integration");

        Ok(SpeechResponse {
            audio_data,
            duration_ms: self.estimate_duration(&request.text),
            sample_rate: 16000,
            format: AudioFormat::Wav,
            processing_time_ms: processing_time,
        })
    }

    async fn get_available_voices(&self) -> VoiceResult<Vec<String>> {
        Ok(vec!["system_default".to_string()])
    }

    async fn is_ready(&self) -> bool {
        true // System TTS is always available as fallback
    }
}

impl SystemTTS {
    fn estimate_duration(&self, text: &str) -> u64 {
        let word_count = text.split_whitespace().count();
        let minutes = word_count as f64 / 150.0;
        (minutes * 60.0 * 1000.0) as u64
    }
}

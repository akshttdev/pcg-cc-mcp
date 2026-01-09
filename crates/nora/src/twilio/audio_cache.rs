//! Audio cache for storing NORA-generated speech for Twilio playback
//!
//! When Twilio needs to play audio, it fetches from a URL. This cache stores
//! generated audio temporarily so Twilio can retrieve it via the audio endpoint.

use std::{collections::HashMap, sync::Arc, time::Duration};

use base64::engine::Engine;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::voice::AudioFormat;

/// Cached audio entry
#[derive(Debug, Clone)]
pub struct CachedAudio {
    /// Unique identifier for this audio
    pub id: String,
    /// Raw audio bytes (decoded from base64)
    pub audio_bytes: Vec<u8>,
    /// Audio format (mp3, wav, etc.)
    pub format: AudioFormat,
    /// When this audio was created
    pub created_at: DateTime<Utc>,
    /// Optional call SID this audio is associated with
    pub call_sid: Option<String>,
    /// Text that was synthesized (for debugging)
    pub source_text: String,
}

impl CachedAudio {
    /// Get the content type for this audio format
    pub fn content_type(&self) -> &'static str {
        match self.format {
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Ogg => "audio/ogg",
        }
    }

    /// Get the file extension for this audio format
    pub fn extension(&self) -> &'static str {
        match self.format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
        }
    }
}

/// Audio cache for Twilio TTS responses
#[derive(Debug)]
pub struct TwilioAudioCache {
    /// Map of audio ID -> cached audio
    cache: Arc<RwLock<HashMap<String, CachedAudio>>>,
    /// Maximum age before audio is considered stale (default: 5 minutes)
    max_age: Duration,
    /// Maximum number of cached items
    max_items: usize,
}

impl Default for TwilioAudioCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TwilioAudioCache {
    /// Create a new audio cache with default settings
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_age: Duration::from_secs(300), // 5 minutes
            max_items: 100,
        }
    }

    /// Create a new audio cache with custom settings
    pub fn with_settings(max_age_secs: u64, max_items: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_age: Duration::from_secs(max_age_secs),
            max_items,
        }
    }

    /// Store audio in the cache and return the ID
    pub async fn store(
        &self,
        audio_base64: &str,
        format: AudioFormat,
        source_text: &str,
        call_sid: Option<String>,
    ) -> Result<String, String> {
        // Decode base64 audio
        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(audio_base64)
            .map_err(|e| format!("Failed to decode base64 audio: {}", e))?;

        let id = Uuid::new_v4().to_string();
        let cached = CachedAudio {
            id: id.clone(),
            audio_bytes,
            format,
            created_at: Utc::now(),
            call_sid,
            source_text: source_text.to_string(),
        };

        let mut cache = self.cache.write().await;

        // Clean up old entries if we're at capacity
        if cache.len() >= self.max_items {
            self.cleanup_stale_entries(&mut cache);
        }

        // If still at capacity after cleanup, remove oldest
        if cache.len() >= self.max_items {
            if let Some(oldest_id) = self.find_oldest_entry(&cache) {
                cache.remove(&oldest_id);
            }
        }

        info!(
            "Stored audio {} ({} bytes, format: {:?}) for text: '{}'",
            id,
            cached.audio_bytes.len(),
            cached.format,
            if source_text.len() > 50 {
                format!("{}...", &source_text[..50])
            } else {
                source_text.to_string()
            }
        );

        cache.insert(id.clone(), cached);

        Ok(id)
    }

    /// Retrieve audio from the cache
    pub async fn get(&self, id: &str) -> Option<CachedAudio> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    /// Remove audio from the cache
    pub async fn remove(&self, id: &str) -> Option<CachedAudio> {
        let mut cache = self.cache.write().await;
        cache.remove(id)
    }

    /// Get number of cached items
    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.cache.read().await.is_empty()
    }

    /// Clean up stale entries (older than max_age)
    pub async fn cleanup(&self) {
        let mut cache = self.cache.write().await;
        self.cleanup_stale_entries(&mut cache);
    }

    fn cleanup_stale_entries(&self, cache: &mut HashMap<String, CachedAudio>) {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.max_age).unwrap_or_default();
        let stale_ids: Vec<String> = cache
            .iter()
            .filter(|(_, audio)| audio.created_at < cutoff)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &stale_ids {
            cache.remove(id);
        }

        if !stale_ids.is_empty() {
            info!("Cleaned up {} stale audio entries", stale_ids.len());
        }
    }

    fn find_oldest_entry(&self, cache: &HashMap<String, CachedAudio>) -> Option<String> {
        cache
            .iter()
            .min_by_key(|(_, audio)| audio.created_at)
            .map(|(id, _)| id.clone())
    }

    /// Remove all audio associated with a specific call
    pub async fn cleanup_call(&self, call_sid: &str) {
        let mut cache = self.cache.write().await;
        let call_ids: Vec<String> = cache
            .iter()
            .filter(|(_, audio)| audio.call_sid.as_deref() == Some(call_sid))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &call_ids {
            cache.remove(id);
        }

        if !call_ids.is_empty() {
            info!(
                "Cleaned up {} audio entries for call {}",
                call_ids.len(),
                call_sid
            );
        }
    }
}

/// Global audio cache instance
static AUDIO_CACHE: tokio::sync::OnceCell<TwilioAudioCache> = tokio::sync::OnceCell::const_new();

/// Get or initialize the global audio cache
pub async fn get_audio_cache() -> &'static TwilioAudioCache {
    AUDIO_CACHE
        .get_or_init(|| async { TwilioAudioCache::new() })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_cache_store_and_retrieve() {
        let cache = TwilioAudioCache::new();

        // Create some dummy audio data
        let audio_data = vec![0u8; 1024];
        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&audio_data);

        // Store it
        let id = cache
            .store(&audio_base64, AudioFormat::Mp3, "Hello world", None)
            .await
            .expect("Failed to store audio");

        // Retrieve it
        let retrieved = cache.get(&id).await.expect("Failed to get audio");
        assert_eq!(retrieved.audio_bytes.len(), 1024);
        assert_eq!(retrieved.source_text, "Hello world");

        // Remove it
        let removed = cache.remove(&id).await.expect("Failed to remove audio");
        assert_eq!(removed.id, id);

        // Should be gone now
        assert!(cache.get(&id).await.is_none());
    }
}

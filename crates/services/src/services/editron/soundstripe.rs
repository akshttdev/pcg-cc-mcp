//! Soundstripe API Client
//!
//! Provides access to Soundstripe's music catalog through their REST API.
//! Uses a simple Bearer API key authentication (no token refresh needed).
//! Responses use JSON:API format with included resources for audio files.

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use super::{
    EditronError,
    music::{
        LicenseInfo, LicenseType, MusicGenre, MusicMood, MusicPlatform, MusicSearchCriteria,
        MusicTrack,
    },
};

/// Soundstripe API base URL
const SS_API_BASE: &str = "https://api.soundstripe.com/v1";

/// Soundstripe service error types
#[derive(Debug, Error, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SoundstripeError {
    #[error("Soundstripe credentials not configured")]
    NotConfigured,
    #[error("Soundstripe authentication failed - invalid API key")]
    AuthFailed,
    #[error("Soundstripe rate limit exceeded")]
    RateLimited,
    #[error("Soundstripe track not found: {0}")]
    TrackNotFound(String),
    #[error("Soundstripe API error: {0}")]
    ApiError(String),
    #[ts(skip)]
    #[serde(skip)]
    #[error("HTTP request error: {0}")]
    Request(String),
}

impl SoundstripeError {
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            SoundstripeError::RateLimited | SoundstripeError::Request(_)
        )
    }
}

impl From<reqwest::Error> for SoundstripeError {
    fn from(err: reqwest::Error) -> Self {
        SoundstripeError::Request(err.to_string())
    }
}

impl From<SoundstripeError> for EditronError {
    fn from(err: SoundstripeError) -> Self {
        EditronError::Process(format!("Soundstripe error: {}", err))
    }
}

/// Soundstripe configuration
#[derive(Clone, Debug, Serialize, Deserialize, TS, Default)]
pub struct SoundstripeConfig {
    /// API key for Bearer authentication
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// Whether Soundstripe integration is enabled
    #[serde(default)]
    pub enabled: bool,
}

impl SoundstripeConfig {
    pub fn is_configured(&self) -> bool {
        self.enabled && self.api_key.is_some()
    }
}

// === JSON:API Response Types ===

/// Top-level JSON:API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonApiResponse {
    #[serde(default)]
    data: Vec<SongData>,
    #[serde(default)]
    included: Vec<IncludedResource>,
    #[serde(default)]
    meta: Option<JsonApiMeta>,
}

/// Single-resource JSON:API response
#[derive(Debug, Deserialize)]
struct JsonApiSingleResponse {
    data: SongData,
    #[serde(default)]
    included: Vec<IncludedResource>,
}

/// Song data in JSON:API format
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SongData {
    id: String,
    #[serde(rename = "type")]
    resource_type: String,
    attributes: SongAttributes,
    #[serde(default)]
    relationships: Option<SongRelationships>,
}

/// Song attributes
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SongAttributes {
    #[serde(default)]
    title: String,
    #[serde(default)]
    artist_name: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    bpm: Option<u32>,
    #[serde(default)]
    key_signature: Option<String>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    moods: Vec<String>,
    #[serde(default)]
    energy: Option<String>,
    #[serde(default)]
    instrumental: Option<bool>,
    #[serde(default)]
    preview_url: Option<String>,
    #[serde(default)]
    image_url: Option<String>,
}

/// Song relationships for linking to included resources
#[derive(Debug, Deserialize)]
struct SongRelationships {
    #[serde(default)]
    audio_files: Option<RelationshipData>,
}

#[derive(Debug, Deserialize)]
struct RelationshipData {
    #[serde(default)]
    data: Vec<ResourceIdentifier>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResourceIdentifier {
    id: String,
    #[serde(rename = "type")]
    resource_type: String,
}

/// Included resources (audio_files, tags, etc.)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IncludedResource {
    id: String,
    #[serde(rename = "type")]
    resource_type: String,
    #[serde(default)]
    attributes: serde_json::Value,
}

/// Pagination metadata
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonApiMeta {
    #[serde(default)]
    total: Option<u32>,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    per_page: Option<u32>,
}

/// Tag data for genre/mood filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundstripeTag {
    pub id: String,
    pub name: String,
    pub category: String,
}

/// Soundstripe API client
pub struct SoundstripeClient {
    client: Client,
    api_key: String,
}

impl SoundstripeClient {
    /// Create a new Soundstripe client
    pub fn new(api_key: &str) -> Result<Self, SoundstripeError> {
        if api_key.is_empty() {
            return Err(SoundstripeError::NotConfigured);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| SoundstripeError::Request(e.to_string()))?;

        Ok(Self {
            client,
            api_key: api_key.to_string(),
        })
    }

    /// Create from config
    pub fn from_config(config: &SoundstripeConfig) -> Result<Self, SoundstripeError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or(SoundstripeError::NotConfigured)?;
        Self::new(api_key)
    }

    /// Build a request with Soundstripe auth headers
    fn authed_get(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("api_key", &self.api_key)
    }

    /// Search tracks with retry logic
    pub async fn search_tracks(
        &self,
        criteria: &MusicSearchCriteria,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<MusicTrack>, SoundstripeError> {
        (|| async { self.search_tracks_internal(criteria, page, per_page).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .notify(|err: &SoundstripeError, dur: Duration| {
                tracing::warn!(
                    "Soundstripe API call failed, retrying after {:.2}s: {}",
                    dur.as_secs_f64(),
                    err
                );
            })
            .await
    }

    async fn search_tracks_internal(
        &self,
        criteria: &MusicSearchCriteria,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<MusicTrack>, SoundstripeError> {
        let mut params: Vec<(String, String)> = vec![
            ("page[number]".to_string(), page.to_string()),
            ("page[size]".to_string(), per_page.to_string()),
            ("include".to_string(), "audio_files".to_string()),
        ];

        if let Some(ref query) = criteria.query {
            params.push(("filter[q]".to_string(), query.clone()));
        }

        for genre in &criteria.genres {
            params.push((
                "filter[tags][genre]".to_string(),
                genre.soundstripe_term().to_string(),
            ));
        }

        for mood in &criteria.moods {
            params.push((
                "filter[tags][mood]".to_string(),
                mood.soundstripe_term().to_string(),
            ));
        }

        if let Some(min_bpm) = criteria.min_bpm {
            params.push(("filter[bpm][min]".to_string(), min_bpm.to_string()));
        }
        if let Some(max_bpm) = criteria.max_bpm {
            params.push(("filter[bpm][max]".to_string(), max_bpm.to_string()));
        }

        if criteria.instrumental == Some(true) {
            params.push(("filter[instrumental]".to_string(), "true".to_string()));
        }

        let url = format!("{}/songs", SS_API_BASE);

        let response = self.authed_get(&url).query(&params).send().await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(SoundstripeError::AuthFailed);
        }
        if response.status() == 429 {
            return Err(SoundstripeError::RateLimited);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SoundstripeError::ApiError(error_text));
        }

        let api_response: JsonApiResponse = response
            .json()
            .await
            .map_err(|e| SoundstripeError::ApiError(e.to_string()))?;

        let tracks = api_response
            .data
            .into_iter()
            .map(|song| Self::song_to_music_track(song, &api_response.included))
            .collect();

        Ok(tracks)
    }

    /// Get a specific track by ID
    pub async fn get_track(&self, track_id: &str) -> Result<MusicTrack, SoundstripeError> {
        let url = format!("{}/songs/{}?include=audio_files", SS_API_BASE, track_id);

        let response = self.authed_get(&url).send().await?;

        if response.status() == 404 {
            return Err(SoundstripeError::TrackNotFound(track_id.to_string()));
        }
        if response.status() == 401 || response.status() == 403 {
            return Err(SoundstripeError::AuthFailed);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SoundstripeError::ApiError(error_text));
        }

        let api_response: JsonApiSingleResponse = response
            .json()
            .await
            .map_err(|e| SoundstripeError::ApiError(e.to_string()))?;

        Ok(Self::song_to_music_track(
            api_response.data,
            &api_response.included,
        ))
    }

    /// Get available tags for filtering (genre or mood)
    pub async fn get_tags(&self, category: &str) -> Result<Vec<SoundstripeTag>, SoundstripeError> {
        let url = format!("{}/tags", SS_API_BASE);

        let response = self
            .authed_get(&url)
            .query(&[("filter[category]", category)])
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(SoundstripeError::AuthFailed);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SoundstripeError::ApiError(error_text));
        }

        // Tags don't use the same SongData shape, parse as raw JSON:API
        let raw: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SoundstripeError::ApiError(e.to_string()))?;

        let tags = raw
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let id = item.get("id")?.as_str()?.to_string();
                        let attrs = item.get("attributes")?;
                        let name = attrs
                            .get("title")
                            .or_else(|| attrs.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        if name.is_empty() {
                            None
                        } else {
                            Some(SoundstripeTag {
                                id,
                                name,
                                category: category.to_string(),
                            })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(tags)
    }

    /// Verify credentials by making a minimal API call
    pub async fn verify_credentials(&self) -> Result<(), SoundstripeError> {
        let url = format!("{}/songs", SS_API_BASE);

        let response = self
            .authed_get(&url)
            .query(&[("page[size]", "1")])
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(SoundstripeError::AuthFailed);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SoundstripeError::ApiError(error_text));
        }

        Ok(())
    }

    /// Convert a JSON:API song resource to MusicTrack
    fn song_to_music_track(song: SongData, included: &[IncludedResource]) -> MusicTrack {
        let genre = song
            .attributes
            .genre
            .as_deref()
            .and_then(MusicGenre::from_soundstripe_term)
            .unwrap_or(MusicGenre::Pop);

        let moods: Vec<MusicMood> = song
            .attributes
            .moods
            .iter()
            .filter_map(|s| MusicMood::from_soundstripe_term(s))
            .collect();

        // Extract download URL from included audio_files
        let download_url = Self::extract_download_url(&song, included);

        MusicTrack {
            id: format!("soundstripe:{}", song.id),
            title: song.attributes.title,
            artist: song
                .attributes
                .artist_name
                .unwrap_or_else(|| "Unknown Artist".to_string()),
            duration: song.attributes.duration.unwrap_or(0.0),
            bpm: song.attributes.bpm,
            key: song.attributes.key_signature,
            genre,
            moods,
            tags: vec![],
            platform: MusicPlatform::Soundstripe,
            url: Some(format!("https://app.soundstripe.com/songs/{}", song.id)),
            local_path: None,
            preview_url: song.attributes.preview_url.or(download_url),
            license: LicenseInfo {
                license_type: LicenseType::Subscription,
                platform: MusicPlatform::Soundstripe,
                subscription_id: None,
                download_date: None,
                project_name: None,
                usage_notes: None,
            },
            waveform: None,
        }
    }

    /// Extract download URL from included audio_files resources
    fn extract_download_url(song: &SongData, included: &[IncludedResource]) -> Option<String> {
        // Get audio_file relationship IDs
        let audio_file_ids: Vec<&str> = song
            .relationships
            .as_ref()
            .and_then(|r| r.audio_files.as_ref())
            .map(|af| af.data.iter().map(|ri| ri.id.as_str()).collect())
            .unwrap_or_default();

        // Find matching included resource and extract mp3 URL
        for resource in included {
            if resource.resource_type == "audio_files"
                && audio_file_ids.contains(&resource.id.as_str())
            {
                // Try versions.mp3 first, then versions.wav
                if let Some(versions) = resource.attributes.get("versions") {
                    if let Some(mp3_url) = versions.get("mp3").and_then(|v| v.as_str()) {
                        return Some(mp3_url.to_string());
                    }
                    if let Some(wav_url) = versions.get("wav").and_then(|v| v.as_str()) {
                        return Some(wav_url.to_string());
                    }
                }
                // Fallback: check for direct url attribute
                if let Some(url) = resource.attributes.get("url").and_then(|v| v.as_str()) {
                    return Some(url.to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_should_retry() {
        assert!(SoundstripeError::RateLimited.should_retry());
        assert!(SoundstripeError::Request("timeout".to_string()).should_retry());
        assert!(!SoundstripeError::AuthFailed.should_retry());
        assert!(!SoundstripeError::NotConfigured.should_retry());
    }

    #[test]
    fn test_create_client_empty_key() {
        let result = SoundstripeClient::new("");
        assert!(matches!(result, Err(SoundstripeError::NotConfigured)));
    }

    #[test]
    fn test_config_is_configured() {
        let config = SoundstripeConfig::default();
        assert!(!config.is_configured());

        let config = SoundstripeConfig {
            api_key: Some("key".to_string()),
            enabled: true,
        };
        assert!(config.is_configured());

        let config = SoundstripeConfig {
            api_key: Some("key".to_string()),
            enabled: false,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_song_to_music_track() {
        let song = SongData {
            id: "abc123".to_string(),
            resource_type: "songs".to_string(),
            attributes: SongAttributes {
                title: "Test Song".to_string(),
                artist_name: Some("Test Artist".to_string()),
                duration: Some(200.0),
                bpm: Some(128),
                key_signature: Some("C Major".to_string()),
                genre: Some("Electronic".to_string()),
                moods: vec!["Energetic".to_string()],
                energy: Some("High".to_string()),
                instrumental: Some(true),
                preview_url: Some("https://example.com/preview.mp3".to_string()),
                image_url: None,
            },
            relationships: None,
        };

        let track = SoundstripeClient::song_to_music_track(song, &[]);
        assert_eq!(track.id, "soundstripe:abc123");
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.artist, "Test Artist");
        assert_eq!(track.duration, 200.0);
        assert_eq!(track.bpm, Some(128));
        assert!(matches!(track.genre, MusicGenre::Electronic));
    }
}

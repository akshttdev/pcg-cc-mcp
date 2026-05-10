//! Epidemic Sound Partner API Client
//!
//! Provides access to Epidemic Sound's music catalog through their Partner API.
//! Uses a two-step token exchange:
//!   1. Partner token (from access key credentials, 1-day TTL)
//!   2. User token (from partner token + user ID, 7-day TTL)

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use backon::{ExponentialBuilder, Retryable};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use ts_rs::TS;

use super::{
    music::{
        LicenseInfo, LicenseType, MusicGenre, MusicMood, MusicPlatform, MusicSearchCriteria,
        MusicTrack,
    },
    EditronError,
};

/// Epidemic Sound API base URL
const ES_API_BASE: &str = "https://api.epidemicsound.com/v0";

/// Epidemic Sound service error types
#[derive(Debug, Error, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpidemicSoundError {
    #[error("Epidemic Sound credentials not configured")]
    NotConfigured,
    #[error("Epidemic Sound authentication failed - invalid credentials")]
    AuthFailed,
    #[error("Epidemic Sound rate limit exceeded")]
    RateLimited,
    #[error("Epidemic Sound track not found: {0}")]
    TrackNotFound(String),
    #[error("Epidemic Sound API error: {0}")]
    ApiError(String),
    #[ts(skip)]
    #[serde(skip)]
    #[error("HTTP request error: {0}")]
    Request(String),
    #[error("Partner token expired")]
    PartnerTokenExpired,
    #[error("User token expired")]
    UserTokenExpired,
}

impl EpidemicSoundError {
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            EpidemicSoundError::RateLimited
                | EpidemicSoundError::Request(_)
                | EpidemicSoundError::PartnerTokenExpired
                | EpidemicSoundError::UserTokenExpired
        )
    }
}

impl From<reqwest::Error> for EpidemicSoundError {
    fn from(err: reqwest::Error) -> Self {
        EpidemicSoundError::Request(err.to_string())
    }
}

impl From<EpidemicSoundError> for EditronError {
    fn from(err: EpidemicSoundError) -> Self {
        EditronError::Process(format!("Epidemic Sound error: {}", err))
    }
}

/// Epidemic Sound configuration
#[derive(Clone, Debug, Serialize, Deserialize, TS, Default)]
pub struct EpidemicSoundConfig {
    /// Partner API access key ID
    pub access_key_id: Option<String>,
    /// Partner API access key secret
    #[serde(skip_serializing)]
    pub access_key_secret: Option<String>,
    /// Whether Epidemic Sound integration is enabled
    #[serde(default)]
    pub enabled: bool,
}

impl EpidemicSoundConfig {
    pub fn is_configured(&self) -> bool {
        self.enabled && self.access_key_id.is_some() && self.access_key_secret.is_some()
    }
}

/// Cached access token with expiry tracking
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

impl CachedToken {
    fn is_expired(&self) -> bool {
        // Consider expired 5 minutes before actual expiry for safety
        Instant::now() > self.expires_at - Duration::from_secs(300)
    }
}

/// Partner token response
#[derive(Debug, Deserialize)]
struct PartnerTokenResponse {
    token: String,
    #[serde(default = "default_partner_ttl")]
    expires_in: u64,
}

fn default_partner_ttl() -> u64 {
    86400 // 1 day
}

/// User token response
#[derive(Debug, Deserialize)]
struct UserTokenResponse {
    token: String,
    #[serde(default = "default_user_ttl")]
    expires_in: u64,
}

fn default_user_ttl() -> u64 {
    604800 // 7 days
}

/// Epidemic Sound track from API
#[derive(Debug, Clone, Deserialize)]
pub struct EpidemicTrack {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub artist: Option<EpidemicArtist>,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub bpm: Option<u32>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub moods: Vec<String>,
    #[serde(default)]
    pub energy: Option<String>,
    #[serde(rename = "previewUrl")]
    pub preview_url: Option<String>,
    #[serde(rename = "hasVocals")]
    #[serde(default)]
    pub has_vocals: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EpidemicArtist {
    pub id: Option<u64>,
    pub name: String,
}

/// Track search response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TrackSearchResponse {
    #[serde(default)]
    tracks: Vec<EpidemicTrack>,
    #[serde(default)]
    total: u32,
}

/// Track metadata batch response
#[derive(Debug, Deserialize)]
struct TrackMetadataResponse {
    #[serde(default)]
    tracks: Vec<EpidemicTrack>,
}

/// Download URL response
#[derive(Debug, Deserialize)]
struct DownloadResponse {
    url: String,
}

/// Highlight segment from ML analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackHighlight {
    pub start: f64,
    pub end: f64,
    pub duration: f64,
}

/// Highlights response
#[derive(Debug, Deserialize)]
struct HighlightsResponse {
    #[serde(default)]
    highlights: Vec<TrackHighlight>,
}

/// Similar tracks response
#[derive(Debug, Deserialize)]
struct SimilarResponse {
    #[serde(default)]
    tracks: Vec<EpidemicTrack>,
}

/// Beat timestamps response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackBeats {
    pub bpm: f64,
    #[serde(default)]
    pub beats: Vec<f64>,
    #[serde(default)]
    pub downbeats: Vec<f64>,
}

/// Epidemic Sound API client
pub struct EpidemicSoundClient {
    client: Client,
    access_key_id: String,
    access_key_secret: String,
    user_id: String,
    partner_token: Arc<RwLock<Option<CachedToken>>>,
    user_token: Arc<RwLock<Option<CachedToken>>>,
}

impl EpidemicSoundClient {
    /// Create a new Epidemic Sound client
    pub fn new(
        access_key_id: &str,
        access_key_secret: &str,
        user_id: &str,
    ) -> Result<Self, EpidemicSoundError> {
        if access_key_id.is_empty() || access_key_secret.is_empty() {
            return Err(EpidemicSoundError::NotConfigured);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| EpidemicSoundError::Request(e.to_string()))?;

        Ok(Self {
            client,
            access_key_id: access_key_id.to_string(),
            access_key_secret: access_key_secret.to_string(),
            user_id: user_id.to_string(),
            partner_token: Arc::new(RwLock::new(None)),
            user_token: Arc::new(RwLock::new(None)),
        })
    }

    /// Create from config (uses "default" as user ID)
    pub fn from_config(config: &EpidemicSoundConfig) -> Result<Self, EpidemicSoundError> {
        let key_id = config
            .access_key_id
            .as_ref()
            .ok_or(EpidemicSoundError::NotConfigured)?;
        let key_secret = config
            .access_key_secret
            .as_ref()
            .ok_or(EpidemicSoundError::NotConfigured)?;
        Self::new(key_id, key_secret, "default")
    }

    /// Step 1: Get partner token
    async fn get_partner_token(&self) -> Result<String, EpidemicSoundError> {
        {
            let guard = self.partner_token.read().await;
            if let Some(ref cached) = *guard {
                if !cached.is_expired() {
                    return Ok(cached.access_token.clone());
                }
            }
        }
        self.refresh_partner_token().await
    }

    async fn refresh_partner_token(&self) -> Result<String, EpidemicSoundError> {
        let url = format!("{}/partner-token", ES_API_BASE);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "accessKeyId": self.access_key_id,
                "accessKeySecret": self.access_key_secret
            }))
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(EpidemicSoundError::AuthFailed);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(format!(
                "Partner token request failed: {}",
                error_text
            )));
        }

        let token_response: PartnerTokenResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        let cached = CachedToken {
            access_token: token_response.token.clone(),
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        };

        {
            let mut guard = self.partner_token.write().await;
            *guard = Some(cached);
        }

        Ok(token_response.token)
    }

    /// Step 2: Get user token (requires partner token)
    async fn get_user_token(&self) -> Result<String, EpidemicSoundError> {
        {
            let guard = self.user_token.read().await;
            if let Some(ref cached) = *guard {
                if !cached.is_expired() {
                    return Ok(cached.access_token.clone());
                }
            }
        }
        self.refresh_user_token().await
    }

    async fn refresh_user_token(&self) -> Result<String, EpidemicSoundError> {
        let partner_token = self.get_partner_token().await?;
        let url = format!("{}/token", ES_API_BASE);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", partner_token))
            .json(&serde_json::json!({
                "userId": self.user_id
            }))
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            // Partner token may be stale, clear it
            {
                let mut guard = self.partner_token.write().await;
                *guard = None;
            }
            return Err(EpidemicSoundError::PartnerTokenExpired);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(format!(
                "User token request failed: {}",
                error_text
            )));
        }

        let token_response: UserTokenResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        let cached = CachedToken {
            access_token: token_response.token.clone(),
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        };

        {
            let mut guard = self.user_token.write().await;
            *guard = Some(cached);
        }

        Ok(token_response.token)
    }

    /// Build authorization header with user token
    async fn auth_header(&self) -> Result<String, EpidemicSoundError> {
        let token = self.get_user_token().await?;
        Ok(format!("Bearer {}", token))
    }

    /// Handle common response errors and clear token caches as needed
    fn handle_response_status(status: reqwest::StatusCode) -> Option<EpidemicSoundError> {
        if status == 401 || status == 403 {
            Some(EpidemicSoundError::UserTokenExpired)
        } else if status == 429 {
            Some(EpidemicSoundError::RateLimited)
        } else {
            None
        }
    }

    /// Search for tracks with retry logic
    pub async fn search_tracks(
        &self,
        criteria: &MusicSearchCriteria,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<MusicTrack>, EpidemicSoundError> {
        (|| async { self.search_tracks_internal(criteria, page, per_page).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .notify(|err: &EpidemicSoundError, dur: Duration| {
                tracing::warn!(
                    "Epidemic Sound API call failed, retrying after {:.2}s: {}",
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
    ) -> Result<Vec<MusicTrack>, EpidemicSoundError> {
        let mut params: Vec<(String, String)> = vec![
            ("page".to_string(), page.to_string()),
            ("limit".to_string(), per_page.to_string()),
        ];

        if let Some(ref query) = criteria.query {
            params.push(("term".to_string(), query.clone()));
        }

        for mood in &criteria.moods {
            params.push(("mood[]".to_string(), mood.epidemic_term().to_string()));
        }

        for genre in &criteria.genres {
            params.push(("genre[]".to_string(), genre.epidemic_term().to_string()));
        }

        if let Some(min_bpm) = criteria.min_bpm {
            params.push(("bpmMin".to_string(), min_bpm.to_string()));
        }
        if let Some(max_bpm) = criteria.max_bpm {
            params.push(("bpmMax".to_string(), max_bpm.to_string()));
        }

        let url = format!("{}/tracks/search", ES_API_BASE);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .query(&params)
            .send()
            .await?;

        if let Some(err) = Self::handle_response_status(response.status()) {
            if matches!(err, EpidemicSoundError::UserTokenExpired) {
                let mut guard = self.user_token.write().await;
                *guard = None;
            }
            return Err(err);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let search_response: TrackSearchResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        let tracks = search_response
            .tracks
            .into_iter()
            .map(|t| t.into_music_track())
            .collect();

        Ok(tracks)
    }

    /// Browse tracks with filters only (no search term)
    pub async fn browse_tracks(
        &self,
        criteria: &MusicSearchCriteria,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<MusicTrack>, EpidemicSoundError> {
        let mut params: Vec<(String, String)> = vec![
            ("page".to_string(), page.to_string()),
            ("limit".to_string(), per_page.to_string()),
        ];

        for mood in &criteria.moods {
            params.push(("mood[]".to_string(), mood.epidemic_term().to_string()));
        }
        for genre in &criteria.genres {
            params.push(("genre[]".to_string(), genre.epidemic_term().to_string()));
        }
        if let Some(min_bpm) = criteria.min_bpm {
            params.push(("bpmMin".to_string(), min_bpm.to_string()));
        }
        if let Some(max_bpm) = criteria.max_bpm {
            params.push(("bpmMax".to_string(), max_bpm.to_string()));
        }

        let url = format!("{}/tracks", ES_API_BASE);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .query(&params)
            .send()
            .await?;

        if let Some(err) = Self::handle_response_status(response.status()) {
            return Err(err);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let search_response: TrackSearchResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        Ok(search_response
            .tracks
            .into_iter()
            .map(|t| t.into_music_track())
            .collect())
    }

    /// Get track metadata by ID (batch endpoint)
    pub async fn get_track(&self, track_id: &str) -> Result<MusicTrack, EpidemicSoundError> {
        let url = format!("{}/tracks/metadata", ES_API_BASE);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .query(&[("trackId[]", track_id)])
            .send()
            .await?;

        if response.status() == 404 {
            return Err(EpidemicSoundError::TrackNotFound(track_id.to_string()));
        }
        if let Some(err) = Self::handle_response_status(response.status()) {
            return Err(err);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let metadata: TrackMetadataResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        metadata
            .tracks
            .into_iter()
            .next()
            .map(|t| t.into_music_track())
            .ok_or_else(|| EpidemicSoundError::TrackNotFound(track_id.to_string()))
    }

    /// Get download URL for a track
    pub async fn get_download_url(
        &self,
        track_id: &str,
        format: &str,
        quality: &str,
    ) -> Result<String, EpidemicSoundError> {
        let url = format!("{}/tracks/{}/download", ES_API_BASE, track_id);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .query(&[("format", format), ("quality", quality)])
            .send()
            .await?;

        if response.status() == 404 {
            return Err(EpidemicSoundError::TrackNotFound(track_id.to_string()));
        }
        if let Some(err) = Self::handle_response_status(response.status()) {
            return Err(err);
        }
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let download: DownloadResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        Ok(download.url)
    }

    /// Get ML-detected highlight segments for a track
    pub async fn get_highlights(
        &self,
        track_id: &str,
        durations: &[u32],
    ) -> Result<Vec<TrackHighlight>, EpidemicSoundError> {
        let url = format!("{}/tracks/{}/highlights", ES_API_BASE, track_id);
        let auth = self.auth_header().await?;

        let duration_str = durations
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .query(&[("duration", &duration_str)])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let highlights: HighlightsResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        Ok(highlights.highlights)
    }

    /// Get similar tracks
    pub async fn get_similar(&self, track_id: &str) -> Result<Vec<MusicTrack>, EpidemicSoundError> {
        let url = format!("{}/tracks/{}/similar", ES_API_BASE, track_id);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let similar: SimilarResponse = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        Ok(similar
            .tracks
            .into_iter()
            .map(|t| t.into_music_track())
            .collect())
    }

    /// Get native beat timestamps for a track
    pub async fn get_beats(&self, track_id: &str) -> Result<TrackBeats, EpidemicSoundError> {
        let url = format!("{}/tracks/{}/beats", ES_API_BASE, track_id);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EpidemicSoundError::ApiError(error_text));
        }

        let beats: TrackBeats = response
            .json()
            .await
            .map_err(|e| EpidemicSoundError::ApiError(e.to_string()))?;

        Ok(beats)
    }

    /// Verify credentials by attempting the two-step token exchange
    pub async fn verify_credentials(&self) -> Result<(), EpidemicSoundError> {
        self.get_user_token().await?;
        Ok(())
    }
}

impl EpidemicTrack {
    /// Convert Epidemic Sound track to common MusicTrack format
    fn into_music_track(self) -> MusicTrack {
        let genre = self
            .genres
            .first()
            .and_then(|s| MusicGenre::from_epidemic_term(s))
            .unwrap_or(MusicGenre::Pop);

        let moods: Vec<MusicMood> = self
            .moods
            .iter()
            .filter_map(|s| MusicMood::from_epidemic_term(s))
            .collect();

        let artist_name = self
            .artist
            .map(|a| a.name)
            .unwrap_or_else(|| "Unknown Artist".to_string());

        MusicTrack {
            id: format!("epidemic:{}", self.id),
            title: self.title,
            artist: artist_name,
            duration: self.duration,
            bpm: self.bpm,
            key: None,
            genre,
            moods,
            tags: vec![],
            platform: MusicPlatform::Epidemic,
            url: Some(format!("https://www.epidemicsound.com/track/{}", self.id)),
            local_path: None,
            preview_url: self.preview_url,
            license: LicenseInfo {
                license_type: LicenseType::Subscription,
                platform: MusicPlatform::Epidemic,
                subscription_id: None,
                download_date: None,
                project_name: None,
                usage_notes: None,
            },
            waveform: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_should_retry() {
        assert!(EpidemicSoundError::RateLimited.should_retry());
        assert!(EpidemicSoundError::Request("timeout".to_string()).should_retry());
        assert!(EpidemicSoundError::PartnerTokenExpired.should_retry());
        assert!(EpidemicSoundError::UserTokenExpired.should_retry());
        assert!(!EpidemicSoundError::AuthFailed.should_retry());
        assert!(!EpidemicSoundError::NotConfigured.should_retry());
    }

    #[test]
    fn test_create_client_empty_credentials() {
        let result = EpidemicSoundClient::new("", "secret", "user");
        assert!(matches!(result, Err(EpidemicSoundError::NotConfigured)));

        let result = EpidemicSoundClient::new("key_id", "", "user");
        assert!(matches!(result, Err(EpidemicSoundError::NotConfigured)));
    }

    #[test]
    fn test_config_is_configured() {
        let config = EpidemicSoundConfig::default();
        assert!(!config.is_configured());

        let config = EpidemicSoundConfig {
            access_key_id: Some("id".to_string()),
            access_key_secret: Some("secret".to_string()),
            enabled: true,
        };
        assert!(config.is_configured());

        let config = EpidemicSoundConfig {
            access_key_id: Some("id".to_string()),
            access_key_secret: Some("secret".to_string()),
            enabled: false,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_epidemic_track_to_music_track() {
        let track = EpidemicTrack {
            id: 12345,
            title: "Test Track".to_string(),
            artist: Some(EpidemicArtist {
                id: Some(1),
                name: "Test Artist".to_string(),
            }),
            duration: 180.0,
            bpm: Some(120),
            genres: vec!["pop".to_string()],
            moods: vec!["happy".to_string(), "uplifting".to_string()],
            energy: Some("high".to_string()),
            preview_url: Some("https://example.com/preview.mp3".to_string()),
            has_vocals: false,
        };

        let music_track = track.into_music_track();
        assert_eq!(music_track.id, "epidemic:12345");
        assert_eq!(music_track.title, "Test Track");
        assert_eq!(music_track.artist, "Test Artist");
        assert_eq!(music_track.duration, 180.0);
        assert_eq!(music_track.bpm, Some(120));
        assert!(matches!(music_track.genre, MusicGenre::Pop));
        assert!(!music_track.moods.is_empty());
    }
}

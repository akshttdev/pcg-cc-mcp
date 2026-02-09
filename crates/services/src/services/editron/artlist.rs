//! Artlist Music Library Integration
//!
//! Provides access to Artlist's music catalog through their Business API.
//! Uses OAuth 2.0 Client Credentials flow for authentication.

use std::sync::Arc;
use std::time::{Duration, Instant};

use backon::{ExponentialBuilder, Retryable};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use ts_rs::TS;

use super::music::{
    LicenseInfo, LicenseType, MusicGenre, MusicMood, MusicPlatform, MusicSearchCriteria,
    MusicTrack,
};
use super::{EditronError, EditronResult};

/// Artlist API endpoints
const ARTLIST_TOKEN_URL: &str =
    "https://artlist-business-api-prod-cognito.artlist.io/oauth2/token";
const ARTLIST_API_BASE: &str = "https://api.artlist.io/business/v1";

/// Artlist service error types
#[derive(Debug, Error, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtlistError {
    #[error("Artlist credentials not configured")]
    NotConfigured,
    #[error("Artlist authentication failed - invalid credentials")]
    AuthFailed,
    #[error("Artlist rate limit exceeded")]
    RateLimited,
    #[error("Artlist track not found: {0}")]
    TrackNotFound(String),
    #[error("Artlist API error: {0}")]
    ApiError(String),
    #[ts(skip)]
    #[serde(skip)]
    #[error("HTTP request error: {0}")]
    Request(String),
    #[error("Token expired")]
    TokenExpired,
}

impl ArtlistError {
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            ArtlistError::RateLimited | ArtlistError::Request(_) | ArtlistError::TokenExpired
        )
    }
}

impl From<reqwest::Error> for ArtlistError {
    fn from(err: reqwest::Error) -> Self {
        ArtlistError::Request(err.to_string())
    }
}

impl From<ArtlistError> for EditronError {
    fn from(err: ArtlistError) -> Self {
        EditronError::Process(format!("Artlist error: {}", err))
    }
}

/// Artlist configuration
#[derive(Clone, Debug, Serialize, Deserialize, TS, Default)]
pub struct ArtlistConfig {
    /// OAuth client ID (from account manager)
    pub client_id: Option<String>,
    /// OAuth client secret (keep secure!)
    #[serde(skip_serializing)]
    pub client_secret: Option<String>,
    /// Whether Artlist integration is enabled
    #[serde(default)]
    pub enabled: bool,
}

impl ArtlistConfig {
    pub fn is_configured(&self) -> bool {
        self.enabled && self.client_id.is_some() && self.client_secret.is_some()
    }
}

/// OAuth token response from Artlist
#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
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

/// Artlist API track response
#[derive(Debug, Clone, Deserialize)]
pub struct ArtlistTrack {
    pub id: String,
    pub title: String,
    pub artist: ArtlistArtist,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub bpm: Option<u32>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub moods: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub has_vocals: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtlistArtist {
    pub id: String,
    pub name: String,
}

/// Search response wrapper
#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    tracks: Vec<ArtlistTrack>,
    #[serde(default)]
    total: u32,
    #[serde(default)]
    page: u32,
    #[serde(default)]
    per_page: u32,
}

/// Artlist API client
pub struct ArtlistClient {
    client: Client,
    client_id: String,
    client_secret: String,
    token: Arc<RwLock<Option<CachedToken>>>,
}

impl ArtlistClient {
    /// Create a new Artlist client with credentials
    pub fn new(client_id: &str, client_secret: &str) -> Result<Self, ArtlistError> {
        if client_id.is_empty() || client_secret.is_empty() {
            return Err(ArtlistError::NotConfigured);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ArtlistError::Request(e.to_string()))?;

        Ok(Self {
            client,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            token: Arc::new(RwLock::new(None)),
        })
    }

    /// Create from config
    pub fn from_config(config: &ArtlistConfig) -> Result<Self, ArtlistError> {
        let client_id = config.client_id.as_ref().ok_or(ArtlistError::NotConfigured)?;
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or(ArtlistError::NotConfigured)?;
        Self::new(client_id, client_secret)
    }

    /// Get a valid access token, refreshing if necessary
    async fn get_access_token(&self) -> Result<String, ArtlistError> {
        // Check if we have a valid cached token
        {
            let token_guard = self.token.read().await;
            if let Some(ref cached) = *token_guard {
                if !cached.is_expired() {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        // Need to refresh token
        self.refresh_token().await
    }

    /// Request a new access token using client credentials
    async fn refresh_token(&self) -> Result<String, ArtlistError> {
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        let encoded_credentials = BASE64.encode(credentials.as_bytes());

        let response = self
            .client
            .post(ARTLIST_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Authorization", format!("Basic {}", encoded_credentials))
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(ArtlistError::AuthFailed);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ArtlistError::ApiError(format!(
                "Token request failed: {}",
                error_text
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| ArtlistError::ApiError(e.to_string()))?;

        let cached = CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        };

        // Cache the token
        {
            let mut token_guard = self.token.write().await;
            *token_guard = Some(cached);
        }

        Ok(token_response.access_token)
    }

    /// Build authorization header
    async fn auth_header(&self) -> Result<String, ArtlistError> {
        let token = self.get_access_token().await?;
        Ok(format!("Bearer {}", token))
    }

    /// Search for tracks with retry logic
    pub async fn search_tracks(
        &self,
        criteria: &MusicSearchCriteria,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<MusicTrack>, ArtlistError> {
        (|| async { self.search_tracks_internal(criteria, page, per_page).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .notify(|err: &ArtlistError, dur: Duration| {
                tracing::warn!(
                    "Artlist API call failed, retrying after {:.2}s: {}",
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
    ) -> Result<Vec<MusicTrack>, ArtlistError> {
        let mut params = vec![
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];

        // Add search query if present
        if let Some(ref query) = criteria.query {
            params.push(("q".to_string(), query.clone()));
        }

        // Add mood filters
        for mood in &criteria.moods {
            params.push(("moods[]".to_string(), mood.artlist_term().to_string()));
        }

        // Add genre filters
        for genre in &criteria.genres {
            params.push(("genres[]".to_string(), genre.artlist_term().to_string()));
        }

        // Add BPM range
        if let Some(min_bpm) = criteria.min_bpm {
            params.push(("bpm_min".to_string(), min_bpm.to_string()));
        }
        if let Some(max_bpm) = criteria.max_bpm {
            params.push(("bpm_max".to_string(), max_bpm.to_string()));
        }

        // Add duration range
        if let Some(min_duration) = criteria.min_duration {
            params.push(("duration_min".to_string(), (min_duration as u32).to_string()));
        }
        if let Some(max_duration) = criteria.max_duration {
            params.push(("duration_max".to_string(), (max_duration as u32).to_string()));
        }

        // Add vocal filter
        if criteria.instrumental == Some(true) {
            params.push(("vocals".to_string(), "instrumental".to_string()));
        } else if criteria.has_vocals == Some(true) {
            params.push(("vocals".to_string(), "vocals".to_string()));
        }

        let url = format!("{}/tracks/search", ARTLIST_API_BASE);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", auth)
            .query(&params)
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            // Token might have expired, clear cache and retry
            {
                let mut token_guard = self.token.write().await;
                *token_guard = None;
            }
            return Err(ArtlistError::TokenExpired);
        }

        if response.status() == 429 {
            return Err(ArtlistError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ArtlistError::ApiError(error_text));
        }

        let search_response: SearchResponse = response
            .json()
            .await
            .map_err(|e| ArtlistError::ApiError(e.to_string()))?;

        // Convert Artlist tracks to MusicTrack
        let tracks = search_response
            .tracks
            .into_iter()
            .map(|t| t.into_music_track())
            .collect();

        Ok(tracks)
    }

    /// Get a specific track by ID
    pub async fn get_track(&self, track_id: &str) -> Result<MusicTrack, ArtlistError> {
        (|| async { self.get_track_internal(track_id).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .await
    }

    async fn get_track_internal(&self, track_id: &str) -> Result<MusicTrack, ArtlistError> {
        let url = format!("{}/tracks/{}", ARTLIST_API_BASE, track_id);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", auth)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(ArtlistError::TrackNotFound(track_id.to_string()));
        }

        if response.status() == 401 || response.status() == 403 {
            {
                let mut token_guard = self.token.write().await;
                *token_guard = None;
            }
            return Err(ArtlistError::TokenExpired);
        }

        if response.status() == 429 {
            return Err(ArtlistError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ArtlistError::ApiError(error_text));
        }

        let track: ArtlistTrack = response
            .json()
            .await
            .map_err(|e| ArtlistError::ApiError(e.to_string()))?;

        Ok(track.into_music_track())
    }

    /// Get download URL for a track (requires appropriate license)
    pub async fn get_download_url(&self, track_id: &str) -> Result<String, ArtlistError> {
        let url = format!("{}/tracks/{}/download", ARTLIST_API_BASE, track_id);
        let auth = self.auth_header().await?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", auth)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(ArtlistError::TrackNotFound(track_id.to_string()));
        }

        if response.status() == 401 || response.status() == 403 {
            return Err(ArtlistError::AuthFailed);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ArtlistError::ApiError(error_text));
        }

        #[derive(Deserialize)]
        struct DownloadResponse {
            url: String,
        }

        let download: DownloadResponse = response
            .json()
            .await
            .map_err(|e| ArtlistError::ApiError(e.to_string()))?;

        Ok(download.url)
    }

    /// Verify credentials by attempting to get a token
    pub async fn verify_credentials(&self) -> Result<(), ArtlistError> {
        self.refresh_token().await?;
        Ok(())
    }
}

impl ArtlistTrack {
    /// Convert Artlist track to the common MusicTrack format
    fn into_music_track(self) -> MusicTrack {
        let genre = self
            .genres
            .first()
            .and_then(|g| MusicGenre::from_artlist_term(g))
            .unwrap_or(MusicGenre::Pop);

        let moods: Vec<MusicMood> = self
            .moods
            .iter()
            .filter_map(|m| MusicMood::from_artlist_term(m))
            .collect();

        MusicTrack {
            id: format!("artlist:{}", self.id),
            title: self.title,
            artist: self.artist.name,
            duration: self.duration,
            bpm: self.bpm,
            key: self.key,
            genre,
            moods,
            tags: self.tags,
            platform: MusicPlatform::Artlist,
            url: self.url.or_else(|| Some(format!("https://artlist.io/song/{}", self.id))),
            local_path: None,
            preview_url: self.preview_url,
            license: LicenseInfo {
                license_type: LicenseType::Subscription,
                platform: MusicPlatform::Artlist,
                subscription_id: None,
                download_date: None,
                project_name: None,
                usage_notes: None,
            },
            waveform: None,
        }
    }
}

// Extend MusicMood with Artlist-specific term mapping
impl MusicMood {
    /// Get Artlist API search term for this mood
    pub fn artlist_term(&self) -> &'static str {
        match self {
            MusicMood::Uplifting => "uplifting",
            MusicMood::Inspirational => "inspirational",
            MusicMood::Happy => "happy",
            MusicMood::Energetic => "energetic",
            MusicMood::Powerful => "powerful",
            MusicMood::Epic => "epic",
            MusicMood::Dramatic => "dramatic",
            MusicMood::Emotional => "emotional",
            MusicMood::Sad => "sad",
            MusicMood::Melancholic => "melancholic",
            MusicMood::Peaceful => "peaceful",
            MusicMood::Relaxing => "relaxing",
            MusicMood::Ambient => "ambient",
            MusicMood::Mysterious => "mysterious",
            MusicMood::Suspenseful => "suspenseful",
            MusicMood::Dark => "dark",
            MusicMood::Aggressive => "aggressive",
            MusicMood::Playful => "playful",
            MusicMood::Romantic => "romantic",
            MusicMood::Nostalgic => "nostalgic",
            MusicMood::Corporate => "corporate",
            MusicMood::Modern => "modern",
            MusicMood::Cinematic => "cinematic",
            MusicMood::Documentary => "documentary",
        }
    }

    /// Parse Artlist mood term to MusicMood
    pub fn from_artlist_term(term: &str) -> Option<Self> {
        match term.to_lowercase().as_str() {
            "uplifting" | "upbeat" => Some(MusicMood::Uplifting),
            "inspirational" | "inspiring" => Some(MusicMood::Inspirational),
            "happy" | "joyful" => Some(MusicMood::Happy),
            "energetic" | "energy" => Some(MusicMood::Energetic),
            "powerful" | "strong" => Some(MusicMood::Powerful),
            "epic" => Some(MusicMood::Epic),
            "dramatic" => Some(MusicMood::Dramatic),
            "emotional" | "emotive" => Some(MusicMood::Emotional),
            "sad" | "somber" => Some(MusicMood::Sad),
            "melancholic" | "melancholy" => Some(MusicMood::Melancholic),
            "peaceful" | "serene" => Some(MusicMood::Peaceful),
            "relaxing" | "calm" | "chill" => Some(MusicMood::Relaxing),
            "ambient" | "atmospheric" => Some(MusicMood::Ambient),
            "mysterious" | "mystery" => Some(MusicMood::Mysterious),
            "suspenseful" | "tense" | "tension" => Some(MusicMood::Suspenseful),
            "dark" | "moody" => Some(MusicMood::Dark),
            "aggressive" | "intense" => Some(MusicMood::Aggressive),
            "playful" | "fun" => Some(MusicMood::Playful),
            "romantic" | "love" => Some(MusicMood::Romantic),
            "nostalgic" | "retro" => Some(MusicMood::Nostalgic),
            "corporate" | "business" => Some(MusicMood::Corporate),
            "modern" | "contemporary" => Some(MusicMood::Modern),
            "cinematic" | "film" => Some(MusicMood::Cinematic),
            "documentary" => Some(MusicMood::Documentary),
            _ => None,
        }
    }
}

// Extend MusicGenre with Artlist-specific term mapping
impl MusicGenre {
    /// Get Artlist API search term for this genre
    pub fn artlist_term(&self) -> &'static str {
        match self {
            MusicGenre::Pop => "pop",
            MusicGenre::Rock => "rock",
            MusicGenre::Electronic => "electronic",
            MusicGenre::HipHop => "hip-hop",
            MusicGenre::RnB => "r&b",
            MusicGenre::Jazz => "jazz",
            MusicGenre::Classical => "classical",
            MusicGenre::Orchestral => "orchestral",
            MusicGenre::Acoustic => "acoustic",
            MusicGenre::Folk => "folk",
            MusicGenre::Country => "country",
            MusicGenre::Blues => "blues",
            MusicGenre::Funk => "funk",
            MusicGenre::Soul => "soul",
            MusicGenre::Reggae => "reggae",
            MusicGenre::Latin => "latin",
            MusicGenre::World => "world",
            MusicGenre::Indie => "indie",
            MusicGenre::Alternative => "alternative",
            MusicGenre::Metal => "metal",
            MusicGenre::Punk => "punk",
            MusicGenre::LoFi => "lo-fi",
            MusicGenre::Chillhop => "chillhop",
            MusicGenre::House => "house",
            MusicGenre::Techno => "techno",
            MusicGenre::Trap => "trap",
            MusicGenre::DrumAndBass => "drum-and-bass",
            MusicGenre::Dubstep => "dubstep",
            MusicGenre::Ambient => "ambient",
            MusicGenre::NewAge => "new-age",
            MusicGenre::Soundtrack => "soundtrack",
            MusicGenre::Trailer => "trailer",
        }
    }

    /// Parse Artlist genre term to MusicGenre
    pub fn from_artlist_term(term: &str) -> Option<Self> {
        match term.to_lowercase().replace(['-', ' '], "").as_str() {
            "pop" => Some(MusicGenre::Pop),
            "rock" => Some(MusicGenre::Rock),
            "electronic" | "edm" => Some(MusicGenre::Electronic),
            "hiphop" | "rap" => Some(MusicGenre::HipHop),
            "rnb" | "r&b" | "randb" => Some(MusicGenre::RnB),
            "jazz" => Some(MusicGenre::Jazz),
            "classical" => Some(MusicGenre::Classical),
            "orchestral" | "orchestra" => Some(MusicGenre::Orchestral),
            "acoustic" => Some(MusicGenre::Acoustic),
            "folk" => Some(MusicGenre::Folk),
            "country" => Some(MusicGenre::Country),
            "blues" => Some(MusicGenre::Blues),
            "funk" => Some(MusicGenre::Funk),
            "soul" => Some(MusicGenre::Soul),
            "reggae" => Some(MusicGenre::Reggae),
            "latin" => Some(MusicGenre::Latin),
            "world" => Some(MusicGenre::World),
            "indie" => Some(MusicGenre::Indie),
            "alternative" | "alt" => Some(MusicGenre::Alternative),
            "metal" => Some(MusicGenre::Metal),
            "punk" => Some(MusicGenre::Punk),
            "lofi" | "lo-fi" => Some(MusicGenre::LoFi),
            "chillhop" => Some(MusicGenre::Chillhop),
            "house" => Some(MusicGenre::House),
            "techno" => Some(MusicGenre::Techno),
            "trap" => Some(MusicGenre::Trap),
            "drumandbass" | "dnb" => Some(MusicGenre::DrumAndBass),
            "dubstep" => Some(MusicGenre::Dubstep),
            "ambient" => Some(MusicGenre::Ambient),
            "newage" => Some(MusicGenre::NewAge),
            "soundtrack" | "score" => Some(MusicGenre::Soundtrack),
            "trailer" | "cinematic" => Some(MusicGenre::Trailer),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_should_retry() {
        assert!(ArtlistError::RateLimited.should_retry());
        assert!(ArtlistError::Request("timeout".to_string()).should_retry());
        assert!(ArtlistError::TokenExpired.should_retry());
        assert!(!ArtlistError::AuthFailed.should_retry());
        assert!(!ArtlistError::NotConfigured.should_retry());
    }

    #[test]
    fn test_create_client_empty_credentials() {
        let result = ArtlistClient::new("", "secret");
        assert!(matches!(result, Err(ArtlistError::NotConfigured)));

        let result = ArtlistClient::new("client_id", "");
        assert!(matches!(result, Err(ArtlistError::NotConfigured)));
    }

    #[test]
    fn test_mood_term_mapping() {
        assert_eq!(MusicMood::Uplifting.artlist_term(), "uplifting");
        assert_eq!(MusicMood::Cinematic.artlist_term(), "cinematic");

        assert_eq!(
            MusicMood::from_artlist_term("uplifting"),
            Some(MusicMood::Uplifting)
        );
        assert_eq!(
            MusicMood::from_artlist_term("upbeat"),
            Some(MusicMood::Uplifting)
        );
        assert_eq!(MusicMood::from_artlist_term("unknown"), None);
    }

    #[test]
    fn test_genre_term_mapping() {
        assert_eq!(MusicGenre::HipHop.artlist_term(), "hip-hop");
        assert_eq!(MusicGenre::DrumAndBass.artlist_term(), "drum-and-bass");

        assert_eq!(
            MusicGenre::from_artlist_term("hip-hop"),
            Some(MusicGenre::HipHop)
        );
        assert_eq!(
            MusicGenre::from_artlist_term("hiphop"),
            Some(MusicGenre::HipHop)
        );
        assert_eq!(MusicGenre::from_artlist_term("unknown"), None);
    }

    #[test]
    fn test_config_is_configured() {
        let config = ArtlistConfig::default();
        assert!(!config.is_configured());

        let config = ArtlistConfig {
            client_id: Some("id".to_string()),
            client_secret: Some("secret".to_string()),
            enabled: true,
        };
        assert!(config.is_configured());

        let config = ArtlistConfig {
            client_id: Some("id".to_string()),
            client_secret: Some("secret".to_string()),
            enabled: false,
        };
        assert!(!config.is_configured());
    }
}

//! YouTube Platform Connector
//!
//! Stub implementation. OAuth uses the same Google Cloud project as
//! Calendar/Drive — verify which client ID we're reusing once approved
//! for `youtube.upload`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const YOUTUBE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

pub struct YouTubeConnector {
    _client: Client,
    client_id: String,
    _client_secret: String,
}

impl YouTubeConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            _client: Client::new(),
            client_id: std::env::var("GOOGLE_CLIENT_ID")
                .or_else(|_| std::env::var("YOUTUBE_CLIENT_ID"))
                .unwrap_or_default(),
            _client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                .or_else(|_| std::env::var("YOUTUBE_CLIENT_SECRET"))
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PlatformConnector for YouTubeConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::YouTube
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "https://www.googleapis.com/auth/youtube.upload \
                      https://www.googleapis.com/auth/youtube.readonly";
        Ok(format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}\
             &access_type=offline&prompt=consent",
            YOUTUBE_AUTH_URL,
            self.client_id,
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(scopes),
        ))
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
        _code_verifier: Option<&str>,
    ) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "YouTube OAuth exchange not yet implemented".into(),
        ))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "YouTube token refresh not yet implemented".into(),
        ))
    }

    async fn get_profile(&self, _access_token: &str) -> Result<ProfileInfo, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "YouTube profile fetch not yet implemented".into(),
        ))
    }

    async fn publish(
        &self,
        _access_token: &str,
        _content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "YouTube video upload not yet implemented (resumable upload required)".into(),
        ))
    }

    async fn get_metrics(
        &self,
        _access_token: &str,
        _platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError> {
        Ok(EngagementMetrics::default())
    }

    async fn fetch_mentions(
        &self,
        _access_token: &str,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError> {
        Ok(vec![])
    }

    async fn reply_to_mention(
        &self,
        _access_token: &str,
        _mention_id: &str,
        _content: &str,
    ) -> Result<String, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "YouTube comment reply not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        if content.media_urls.is_empty() {
            return Err(SocialError::ValidationError(
                "YouTube requires a video file".into(),
            ));
        }
        let limits = self.get_limits();
        if content.caption.len() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Description exceeds {} character limit",
                limits.max_caption_length
            )));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 5_000, // description
            max_hashtags: 15,
            max_mentions: 0,
            max_images: 0,
            max_video_length_seconds: 12 * 60 * 60, // 12h verified accounts
            max_image_size_bytes: 0,
            max_video_size_bytes: 256 * 1024 * 1024 * 1024, // 256GB
            supported_media_types: vec![
                "video/mp4".into(),
                "video/quicktime".into(),
                "video/x-msvideo".into(),
                "video/webm".into(),
            ],
        }
    }
}

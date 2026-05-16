//! Bluesky Platform Connector
//!
//! Bluesky uses the **AT Protocol** rather than OAuth: the user creates an
//! "app password" in their Bluesky settings and pastes it into ORCHA. We
//! exchange that for a session JWT via `com.atproto.server.createSession`,
//! then post via `com.atproto.repo.createRecord`.
//!
//! For now this is a stub — `get_auth_url`/`exchange_code` are not the right
//! shape for AT Proto, so the UI will need a separate "paste app password"
//! connect path that calls a Bluesky-specific service method.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

pub struct BlueskyConnector {
    _client: Client,
}

impl BlueskyConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            _client: Client::new(),
        }
    }
}

#[async_trait]
impl PlatformConnector for BlueskyConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Bluesky
    }

    async fn get_auth_url(&self, _redirect_uri: &str, _state: &str) -> Result<String, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Bluesky uses app passwords, not OAuth — use the dedicated connect endpoint".into(),
        ))
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
        _code_verifier: Option<&str>,
    ) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Bluesky uses app passwords, not OAuth code exchange".into(),
        ))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Bluesky session refresh not yet implemented".into(),
        ))
    }

    async fn get_profile(&self, _access_token: &str) -> Result<ProfileInfo, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Bluesky profile fetch not yet implemented".into(),
        ))
    }

    async fn publish(
        &self,
        _access_token: &str,
        _content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Bluesky publishing not yet implemented".into(),
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
            "Bluesky reply not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        let limits = self.get_limits();
        if content.caption.chars().count() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Bluesky post exceeds {} graphemes",
                limits.max_caption_length
            )));
        }
        if content.media_urls.len() > limits.max_images {
            return Err(SocialError::ValidationError(format!(
                "Too many images (max {})",
                limits.max_images
            )));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 300,
            max_hashtags: 0, // hashtags inline, no separate field
            max_mentions: 0,
            max_images: 4,
            max_video_length_seconds: 60,
            max_image_size_bytes: 1_000_000, // 1MB
            max_video_size_bytes: 100 * 1024 * 1024,
            supported_media_types: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/webp".into(),
            ],
        }
    }
}

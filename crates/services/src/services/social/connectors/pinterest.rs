//! Pinterest Platform Connector
//!
//! Stub implementation. Pinterest API v5 uses standard OAuth 2.0.
//! Endpoints land here once the Pinterest developer app is approved
//! for `pins:write` and `boards:read`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const PINTEREST_AUTH_URL: &str = "https://www.pinterest.com/oauth/";

pub struct PinterestConnector {
    _client: Client,
    client_id: String,
    _client_secret: String,
}

impl PinterestConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            _client: Client::new(),
            client_id: std::env::var("PINTEREST_CLIENT_ID").unwrap_or_default(),
            _client_secret: std::env::var("PINTEREST_CLIENT_SECRET").unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PlatformConnector for PinterestConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Pinterest
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "boards:read,pins:read,pins:write,user_accounts:read";
        Ok(format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}",
            PINTEREST_AUTH_URL,
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
            "Pinterest OAuth exchange not yet implemented".into(),
        ))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Pinterest token refresh not yet implemented".into(),
        ))
    }

    async fn get_profile(&self, _access_token: &str) -> Result<ProfileInfo, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Pinterest profile fetch not yet implemented".into(),
        ))
    }

    async fn publish(
        &self,
        _access_token: &str,
        _content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Pinterest pin creation not yet implemented".into(),
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
            "Pinterest does not support standard reply API".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        if content.media_urls.is_empty() {
            return Err(SocialError::ValidationError(
                "Pinterest requires at least one image".into(),
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
            max_caption_length: 800,
            max_hashtags: 20,
            max_mentions: 0,
            max_images: 1,
            max_video_length_seconds: 15 * 60,
            max_image_size_bytes: 32 * 1024 * 1024, // 32MB
            max_video_size_bytes: 2 * 1024 * 1024 * 1024,
            supported_media_types: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/webp".into(),
                "video/mp4".into(),
            ],
        }
    }
}

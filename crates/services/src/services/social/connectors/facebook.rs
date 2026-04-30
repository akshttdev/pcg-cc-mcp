//! Facebook Platform Connector
//!
//! Stub implementation. Shares Meta App credentials with Instagram/Threads.
//! Full publish/metrics/mentions wiring lands once Meta App Review is approved
//! for `pages_manage_posts` + `pages_read_engagement`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const FACEBOOK_AUTH_URL: &str = "https://www.facebook.com/v19.0/dialog/oauth";

pub struct FacebookConnector {
    _client: Client,
    app_id: String,
    _app_secret: String,
}

impl FacebookConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            _client: Client::new(),
            app_id: std::env::var("FACEBOOK_APP_ID")
                .or_else(|_| std::env::var("INSTAGRAM_APP_ID"))
                .unwrap_or_default(),
            _app_secret: std::env::var("FACEBOOK_APP_SECRET")
                .or_else(|_| std::env::var("INSTAGRAM_APP_SECRET"))
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PlatformConnector for FacebookConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Facebook
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "pages_manage_posts,pages_read_engagement,pages_show_list";
        Ok(format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}",
            FACEBOOK_AUTH_URL,
            self.app_id,
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
            "Facebook OAuth exchange not yet implemented (pending Meta App Review)".into(),
        ))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Facebook uses long-lived page tokens; no refresh flow".into(),
        ))
    }

    async fn get_profile(&self, _access_token: &str) -> Result<ProfileInfo, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Facebook profile fetch not yet implemented".into(),
        ))
    }

    async fn publish(
        &self,
        _access_token: &str,
        _content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Facebook publishing not yet implemented".into(),
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
            "Facebook reply not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        let limits = self.get_limits();
        if content.caption.len() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Caption exceeds {} character limit",
                limits.max_caption_length
            )));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 63_206,
            max_hashtags: 30,
            max_mentions: 50,
            max_images: 10,
            max_video_length_seconds: 240 * 60, // 240 min
            max_image_size_bytes: 4 * 1024 * 1024,
            max_video_size_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            supported_media_types: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/gif".into(),
                "video/mp4".into(),
            ],
        }
    }
}

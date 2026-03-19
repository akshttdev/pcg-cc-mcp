//! TikTok Platform Connector
//!
//! Implements OAuth 2.0 and video publishing for TikTok.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;
use serde::Deserialize;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const TIKTOK_AUTH_URL: &str = "https://www.tiktok.com/v2/auth/authorize/";
const TIKTOK_TOKEN_URL: &str = "https://open.tiktokapis.com/v2/oauth/token/";
const TIKTOK_API_BASE: &str = "https://open.tiktokapis.com/v2";

pub struct TikTokConnector {
    client: Client,
    client_key: String,
    client_secret: String,
}

impl TikTokConnector {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_key: std::env::var("TIKTOK_CLIENT_KEY").unwrap_or_default(),
            client_secret: std::env::var("TIKTOK_CLIENT_SECRET").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TikTokTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
    token_type: String,
    scope: Option<String>,
    open_id: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TikTokUserInfo {
    open_id: String,
    display_name: String,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TikTokPublishResponse {
    data: TikTokPublishData,
}

#[derive(Debug, Deserialize)]
struct TikTokPublishData {
    publish_id: String,
}

#[async_trait]
impl PlatformConnector for TikTokConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::TikTok
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "user.info.basic,video.publish,video.upload";
        let url = format!(
            "{}?client_key={}&response_type=code&scope={}&redirect_uri={}&state={}",
            TIKTOK_AUTH_URL,
            urlencoding::encode(&self.client_key),
            urlencoding::encode(scopes),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state)
        );
        Ok(url)
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("client_key", self.client_key.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];

        let response = self
            .client
            .post(TIKTOK_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!(
                "TikTok token exchange failed: {}",
                err
            )));
        }

        let tok: TikTokTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = Utc::now() + chrono::Duration::seconds(tok.expires_in);

        Ok(OAuthTokens {
            access_token: tok.access_token,
            refresh_token: tok.refresh_token,
            expires_at: Some(expires_at),
            token_type: tok.token_type,
            scope: tok.scope,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("client_key", self.client_key.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        let response = self
            .client
            .post(TIKTOK_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::AuthError("TikTok token refresh failed".into()));
        }

        let tok: TikTokTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = Utc::now() + chrono::Duration::seconds(tok.expires_in);

        Ok(OAuthTokens {
            access_token: tok.access_token,
            refresh_token: tok.refresh_token,
            expires_at: Some(expires_at),
            token_type: tok.token_type,
            scope: tok.scope,
        })
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        let response = self
            .client
            .get(format!("{}/user/info/", TIKTOK_API_BASE))
            .bearer_auth(access_token)
            .query(&[("fields", "open_id,display_name,avatar_url")])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::PlatformError(
                "Failed to fetch TikTok profile".into(),
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let user = &data["data"]["user"];
        let open_id = user["open_id"].as_str().unwrap_or("unknown").to_string();
        let display_name = user["display_name"].as_str().map(|s| s.to_string());
        let avatar_url = user["avatar_url"].as_str().map(|s| s.to_string());

        Ok(ProfileInfo {
            platform_account_id: open_id.clone(),
            username: open_id,
            display_name,
            profile_url: None,
            avatar_url,
            follower_count: None,
            following_count: None,
            post_count: None,
            is_verified: false,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        self.validate_content(content)?;

        // TikTok requires video URL — use first media URL if provided
        let video_url =
            content.media_urls.first().cloned().ok_or_else(|| {
                SocialError::ValidationError("TikTok requires a video URL".into())
            })?;

        let body = serde_json::json!({
            "post_info": {
                "title": &content.caption[..content.caption.len().min(150)],
                "privacy_level": "SELF_ONLY",
                "disable_duet": false,
                "disable_comment": false,
                "disable_stitch": false
            },
            "source_info": {
                "source": "PULL_FROM_URL",
                "video_url": video_url
            }
        });

        let response = self
            .client
            .post(format!("{}/post/publish/video/init/", TIKTOK_API_BASE))
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!(
                "TikTok publish failed: {}",
                err
            )));
        }

        let result: TikTokPublishResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(PublishResult {
            platform: SocialPlatform::TikTok,
            platform_post_id: result.data.publish_id.clone(),
            platform_url: None,
            published_at: Utc::now(),
        })
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
            "TikTok replies not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        if content.caption.len() > 2200 {
            return Err(SocialError::ValidationError(
                "TikTok caption exceeds 2200 character limit".into(),
            ));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 2200,
            max_hashtags: 30,
            max_mentions: 0,
            max_images: 0,
            max_video_length_seconds: 600,
            max_image_size_bytes: 0,
            max_video_size_bytes: 4 * 1024 * 1024 * 1024,
            supported_media_types: vec!["video/mp4".to_string()],
        }
    }
}

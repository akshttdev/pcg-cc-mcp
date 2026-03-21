//! Threads Platform Connector
//!
//! Implements Meta's Threads API (Instagram Graph API threads endpoint).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;
use serde::Deserialize;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const THREADS_AUTH_URL: &str = "https://threads.net/oauth/authorize";
const THREADS_TOKEN_URL: &str = "https://graph.threads.net/oauth/access_token";
const THREADS_API_BASE: &str = "https://graph.threads.net/v1.0";

pub struct ThreadsConnector {
    client: Client,
    app_id: String,
    app_secret: String,
}

impl ThreadsConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            app_id: std::env::var("INSTAGRAM_APP_ID").unwrap_or_default(),
            app_secret: std::env::var("INSTAGRAM_APP_SECRET").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ThreadsTokenResponse {
    access_token: String,
    token_type: String,
}

#[async_trait]
impl PlatformConnector for ThreadsConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Threads
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "threads_basic,threads_content_publish";
        let url = format!(
            "{}?client_id={}&redirect_uri={}&scope={}&response_type=code&state={}",
            THREADS_AUTH_URL,
            urlencoding::encode(&self.app_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(scopes),
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
            ("client_id", self.app_id.as_str()),
            ("client_secret", self.app_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
            ("code", code),
        ];

        let response = self
            .client
            .post(THREADS_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!(
                "Threads token exchange failed: {}",
                err
            )));
        }

        let tok: ThreadsTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(OAuthTokens {
            access_token: tok.access_token,
            refresh_token: None,
            expires_at: None, // Threads tokens are long-lived
            token_type: tok.token_type,
            scope: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Threads uses long-lived tokens — no refresh needed".into(),
        ))
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        let response = self
            .client
            .get(format!("{}/me", THREADS_API_BASE))
            .query(&[
                ("fields", "id,username,name,threads_profile_picture_url"),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::PlatformError(
                "Failed to fetch Threads profile".into(),
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let id = data["id"].as_str().unwrap_or("unknown").to_string();
        let username = data["username"].as_str().unwrap_or(&id).to_string();
        let display_name = data["name"].as_str().map(|s| s.to_string());
        let avatar_url = data["threads_profile_picture_url"]
            .as_str()
            .map(|s| s.to_string());

        Ok(ProfileInfo {
            platform_account_id: id.clone(),
            username: username.clone(),
            display_name,
            profile_url: Some(format!("https://www.threads.net/@{}", username)),
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

        // Step 1: Create media container
        let mut params = vec![
            ("text".to_string(), content.caption.clone()),
            ("media_type".to_string(), "TEXT".to_string()),
            ("access_token".to_string(), access_token.to_string()),
        ];

        if let Some(url) = content.media_urls.first() {
            params.push(("media_type".to_string(), "IMAGE".to_string()));
            params.push(("image_url".to_string(), url.clone()));
        }

        // Get user ID first
        let me_resp = self
            .client
            .get(format!("{}/me", THREADS_API_BASE))
            .query(&[("fields", "id"), ("access_token", access_token)])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        let me_data: serde_json::Value = me_resp
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;
        let user_id = me_data["id"].as_str().unwrap_or("").to_string();

        // Create container
        let container_resp = self
            .client
            .post(format!("{}/{}/threads", THREADS_API_BASE, user_id))
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !container_resp.status().is_success() {
            let err = container_resp.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!(
                "Threads container creation failed: {}",
                err
            )));
        }

        let container_data: serde_json::Value = container_resp
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;
        let container_id = container_data["id"].as_str().unwrap_or("").to_string();

        // Step 2: Publish container
        let publish_resp = self
            .client
            .post(format!("{}/{}/threads_publish", THREADS_API_BASE, user_id))
            .form(&[
                ("creation_id", container_id.as_str()),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !publish_resp.status().is_success() {
            let err = publish_resp.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!(
                "Threads publish failed: {}",
                err
            )));
        }

        let publish_data: serde_json::Value = publish_resp
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;
        let post_id = publish_data["id"].as_str().unwrap_or("").to_string();

        Ok(PublishResult {
            platform: SocialPlatform::Threads,
            platform_post_id: post_id,
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
            "Threads replies not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        if content.caption.len() > 500 {
            return Err(SocialError::ValidationError(
                "Threads post exceeds 500 character limit".into(),
            ));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 500,
            max_hashtags: 10,
            max_mentions: 10,
            max_images: 10,
            max_video_length_seconds: 300,
            max_image_size_bytes: 8 * 1024 * 1024,
            max_video_size_bytes: 1024 * 1024 * 1024,
            supported_media_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "video/mp4".to_string(),
            ],
        }
    }
}

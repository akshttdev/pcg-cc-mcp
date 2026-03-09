//! Twitter/X Platform Connector
//!
//! Implements OAuth 2.0 PKCE and content publishing for Twitter/X.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};
use db::models::social_account::SocialPlatform;

const TWITTER_AUTH_URL: &str = "https://twitter.com/i/oauth2/authorize";
const TWITTER_TOKEN_URL: &str = "https://api.twitter.com/2/oauth2/token";
const TWITTER_API_BASE: &str = "https://api.twitter.com/2";

pub struct TwitterConnector {
    client: Client,
    client_id: String,
    client_secret: String,
    bearer_token: String,
}

impl TwitterConnector {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("TWITTER_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("TWITTER_CLIENT_SECRET").unwrap_or_default(),
            bearer_token: std::env::var("TWITTER_BEARER_TOKEN").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TwitterTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwitterUser {
    id: String,
    name: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct TwitterUserResponse {
    data: TwitterUser,
}

#[derive(Debug, Serialize)]
struct TweetRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct TweetData {
    id: String,
}

#[derive(Debug, Deserialize)]
struct TweetResponse {
    data: TweetData,
}

#[async_trait]
impl PlatformConnector for TwitterConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Twitter
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "tweet.read tweet.write users.read offline.access";
        let url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}&code_challenge=challenge&code_challenge_method=plain",
            TWITTER_AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(scopes)
        );
        Ok(url)
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", "challenge"),
        ];

        let response = self
            .client
            .post(TWITTER_TOKEN_URL)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!("Twitter token exchange failed: {}", err)));
        }

        let tok: TwitterTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = tok
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s));

        Ok(OAuthTokens {
            access_token: tok.access_token,
            refresh_token: tok.refresh_token,
            expires_at,
            token_type: tok.token_type,
            scope: tok.scope,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        let response = self
            .client
            .post(TWITTER_TOKEN_URL)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::AuthError("Twitter token refresh failed".into()));
        }

        let tok: TwitterTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = tok
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s));

        Ok(OAuthTokens {
            access_token: tok.access_token,
            refresh_token: tok.refresh_token,
            expires_at,
            token_type: tok.token_type,
            scope: tok.scope,
        })
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        let response = self
            .client
            .get(format!("{}/users/me", TWITTER_API_BASE))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::PlatformError("Failed to fetch Twitter profile".into()));
        }

        let data: TwitterUserResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(ProfileInfo {
            platform_account_id: data.data.id.clone(),
            username: data.data.username.clone(),
            display_name: Some(data.data.name),
            profile_url: Some(format!("https://twitter.com/{}", data.data.username)),
            avatar_url: None,
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

        let mut text = content.caption.clone();
        if !content.hashtags.is_empty() {
            text.push(' ');
            text.push_str(
                &content
                    .hashtags
                    .iter()
                    .map(|h| format!("#{}", h))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }

        let tweet = TweetRequest { text };

        let response = self
            .client
            .post(format!("{}/tweets", TWITTER_API_BASE))
            .bearer_auth(access_token)
            .json(&tweet)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!("Twitter publish failed: {}", err)));
        }

        let result: TweetResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(PublishResult {
            platform: SocialPlatform::Twitter,
            platform_post_id: result.data.id.clone(),
            platform_url: Some(format!("https://twitter.com/i/web/status/{}", result.data.id)),
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
            "Twitter replies not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        let limits = self.get_limits();
        if content.caption.len() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Tweet exceeds {} character limit",
                limits.max_caption_length
            )));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 280,
            max_hashtags: 30,
            max_mentions: 50,
            max_images: 4,
            max_video_length_seconds: 140,
            max_image_size_bytes: 5 * 1024 * 1024,
            max_video_size_bytes: 512 * 1024 * 1024,
            supported_media_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "video/mp4".to_string(),
            ],
        }
    }
}

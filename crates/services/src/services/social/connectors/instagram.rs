//! Instagram Platform Connector
//!
//! Implements Instagram Graph API for business/creator accounts.
//! Uses Meta's OAuth 2.0 and requires a Facebook page connection.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};
use db::models::social_account::SocialPlatform;

const META_AUTH_URL: &str = "https://www.facebook.com/v18.0/dialog/oauth";
const META_TOKEN_URL: &str = "https://graph.facebook.com/v18.0/oauth/access_token";
const GRAPH_API_BASE: &str = "https://graph.facebook.com/v18.0";

pub struct InstagramConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl InstagramConnector {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("META_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("META_CLIENT_SECRET").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MetaTokenResponse {
    access_token: String,
    token_type: Option<String>,
    expires_in: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct InstagramAccountsResponse {
    data: Vec<InstagramAccountData>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct InstagramAccountData {
    id: String,
    username: Option<String>,
    name: Option<String>,
    profile_picture_url: Option<String>,
    followers_count: Option<i64>,
    follows_count: Option<i64>,
    media_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MediaContainerResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct PublishResponse {
    id: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MediaInsightsResponse {
    data: Vec<InsightData>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct InsightData {
    name: String,
    values: Vec<InsightValue>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct InsightValue {
    value: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CommentsResponse {
    data: Vec<CommentData>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CommentData {
    id: String,
    text: String,
    timestamp: String,
    from: Option<CommentAuthor>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CommentAuthor {
    id: String,
    username: Option<String>,
}

#[async_trait]
impl PlatformConnector for InstagramConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Instagram
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        // Instagram Business API requires Facebook login with Instagram permissions
        let scopes = "instagram_basic,instagram_content_publish,instagram_manage_comments,instagram_manage_insights,pages_show_list,pages_read_engagement";

        let url = format!(
            "{}?client_id={}&redirect_uri={}&state={}&scope={}&response_type=code",
            META_AUTH_URL,
            &self.client_id,
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
        // Exchange code for short-lived token
        let url = format!(
            "{}?client_id={}&client_secret={}&redirect_uri={}&code={}",
            META_TOKEN_URL,
            &self.client_id,
            &self.client_secret,
            urlencoding::encode(redirect_uri),
            code
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!(
                "Meta token exchange failed: {}",
                error_text
            )));
        }

        let short_lived: MetaTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        // Exchange for long-lived token
        let long_lived_url = format!(
            "{}/oauth/access_token?grant_type=fb_exchange_token&client_id={}&client_secret={}&fb_exchange_token={}",
            GRAPH_API_BASE,
            &self.client_id,
            &self.client_secret,
            &short_lived.access_token
        );

        let long_lived_response = self
            .client
            .get(&long_lived_url)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        let long_lived: MetaTokenResponse = long_lived_response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = long_lived
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs));

        Ok(OAuthTokens {
            access_token: long_lived.access_token,
            refresh_token: None, // Meta uses long-lived tokens instead
            expires_at,
            token_type: long_lived.token_type.unwrap_or_else(|| "Bearer".to_string()),
            scope: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        // Meta doesn't use traditional refresh tokens
        // Long-lived tokens can be refreshed by exchanging them again
        Err(SocialError::AuthError(
            "Use exchange_code with new authorization for Instagram".to_string(),
        ))
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        // First, get connected Instagram accounts via Facebook pages
        let url = format!(
            "{}/me/accounts?fields=instagram_business_account{{id,username,name,profile_picture_url,followers_count,follows_count,media_count}}&access_token={}",
            GRAPH_API_BASE,
            access_token
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::PlatformError("Failed to fetch Instagram account".to_string()));
        }

        // Parse the response to find Instagram account
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        // Navigate to the Instagram business account
        let ig_account = body["data"]
            .as_array()
            .and_then(|pages| pages.first())
            .and_then(|page| page.get("instagram_business_account"))
            .ok_or_else(|| SocialError::PlatformError("No Instagram business account found".to_string()))?;

        Ok(ProfileInfo {
            platform_account_id: ig_account["id"].as_str().unwrap_or_default().to_string(),
            username: ig_account["username"].as_str().unwrap_or_default().to_string(),
            display_name: ig_account["name"].as_str().map(|s| s.to_string()),
            profile_url: Some(format!(
                "https://instagram.com/{}",
                ig_account["username"].as_str().unwrap_or_default()
            )),
            avatar_url: ig_account["profile_picture_url"].as_str().map(|s| s.to_string()),
            follower_count: ig_account["followers_count"].as_i64(),
            following_count: ig_account["follows_count"].as_i64(),
            post_count: ig_account["media_count"].as_i64(),
            is_verified: false, // Would need additional API call
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        self.validate_content(content)?;

        // Get Instagram account ID
        let profile = self.get_profile(access_token).await?;
        let ig_account_id = &profile.platform_account_id;

        // Build caption with hashtags
        let mut caption = content.caption.clone();
        if !content.hashtags.is_empty() {
            caption.push_str("\n\n");
            caption.push_str(&content.hashtags.iter().map(|h| format!("#{}", h)).collect::<Vec<_>>().join(" "));
        }

        // Step 1: Create media container
        let container_id = if content.media_urls.is_empty() {
            return Err(SocialError::ValidationError(
                "Instagram requires at least one image".to_string(),
            ));
        } else if content.media_urls.len() == 1 {
            // Single image/video post
            let media_url = &content.media_urls[0];
            let is_video = media_url.contains(".mp4") || media_url.contains("video");

            let mut params = vec![
                ("caption", caption.as_str()),
                ("access_token", access_token),
            ];

            if is_video {
                params.push(("media_type", "VIDEO"));
                params.push(("video_url", media_url));
            } else {
                params.push(("image_url", media_url));
            }

            let response = self
                .client
                .post(format!("{}/{}/media", GRAPH_API_BASE, ig_account_id))
                .form(&params)
                .send()
                .await
                .map_err(|e| SocialError::NetworkError(e.to_string()))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(SocialError::PlatformError(format!(
                    "Failed to create media container: {}",
                    error_text
                )));
            }

            let container: MediaContainerResponse = response
                .json()
                .await
                .map_err(|e| SocialError::PlatformError(e.to_string()))?;

            container.id
        } else {
            // Carousel post
            let mut children_ids = Vec::new();

            // Create containers for each media item
            for media_url in &content.media_urls {
                let is_video = media_url.contains(".mp4") || media_url.contains("video");

                let mut params = vec![
                    ("is_carousel_item", "true"),
                    ("access_token", access_token),
                ];

                if is_video {
                    params.push(("media_type", "VIDEO"));
                    params.push(("video_url", media_url));
                } else {
                    params.push(("image_url", media_url));
                }

                let response = self
                    .client
                    .post(format!("{}/{}/media", GRAPH_API_BASE, ig_account_id))
                    .form(&params)
                    .send()
                    .await
                    .map_err(|e| SocialError::NetworkError(e.to_string()))?;

                let container: MediaContainerResponse = response
                    .json()
                    .await
                    .map_err(|e| SocialError::PlatformError(e.to_string()))?;

                children_ids.push(container.id);
            }

            // Create carousel container
            let children_str = children_ids.join(",");
            let params = [
                ("media_type", "CAROUSEL"),
                ("caption", &caption),
                ("children", &children_str),
                ("access_token", access_token),
            ];

            let response = self
                .client
                .post(format!("{}/{}/media", GRAPH_API_BASE, ig_account_id))
                .form(&params)
                .send()
                .await
                .map_err(|e| SocialError::NetworkError(e.to_string()))?;

            let container: MediaContainerResponse = response
                .json()
                .await
                .map_err(|e| SocialError::PlatformError(e.to_string()))?;

            container.id
        };

        // Step 2: Publish the container
        let publish_params = [
            ("creation_id", container_id.as_str()),
            ("access_token", access_token),
        ];

        let publish_response = self
            .client
            .post(format!("{}/{}/media_publish", GRAPH_API_BASE, ig_account_id))
            .form(&publish_params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !publish_response.status().is_success() {
            let error_text = publish_response.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!(
                "Failed to publish: {}",
                error_text
            )));
        }

        let published: PublishResponse = publish_response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(PublishResult {
            platform: SocialPlatform::Instagram,
            platform_post_id: published.id.clone(),
            platform_url: Some(format!(
                "https://instagram.com/p/{}",
                published.id
            )),
            published_at: Utc::now(),
        })
    }

    async fn get_metrics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError> {
        let url = format!(
            "{}/{}?fields=insights.metric(impressions,reach,saved,likes,comments,shares)&access_token={}",
            GRAPH_API_BASE,
            platform_post_id,
            access_token
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(EngagementMetrics::default());
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let mut metrics = EngagementMetrics::default();

        if let Some(data) = body["insights"]["data"].as_array() {
            for insight in data {
                let name = insight["name"].as_str().unwrap_or_default();
                let value = insight["values"][0]["value"].as_i64().unwrap_or(0);

                match name {
                    "impressions" => metrics.impressions = value,
                    "reach" => metrics.reach = value,
                    "saved" => metrics.saves = value,
                    "likes" => metrics.likes = value,
                    "comments" => metrics.comments = value,
                    "shares" => metrics.shares = value,
                    _ => {}
                }
            }
        }

        Ok(metrics)
    }

    async fn fetch_mentions(
        &self,
        _access_token: &str,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError> {
        // This would need to iterate over recent posts and fetch their comments
        // For now, return empty
        Ok(vec![])
    }

    async fn reply_to_mention(
        &self,
        access_token: &str,
        mention_id: &str,
        content: &str,
    ) -> Result<String, SocialError> {
        let params = [
            ("message", content),
            ("access_token", access_token),
        ];

        let response = self
            .client
            .post(format!("{}/{}/replies", GRAPH_API_BASE, mention_id))
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::PlatformError("Failed to reply".to_string()));
        }

        let reply: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(reply["id"].as_str().unwrap_or_default().to_string())
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        let limits = self.get_limits();

        if content.caption.len() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Caption exceeds {} character limit",
                limits.max_caption_length
            )));
        }

        if content.hashtags.len() > limits.max_hashtags {
            return Err(SocialError::ValidationError(format!(
                "Too many hashtags (max {})",
                limits.max_hashtags
            )));
        }

        if content.media_urls.is_empty() {
            return Err(SocialError::ValidationError(
                "Instagram requires at least one media item".to_string(),
            ));
        }

        if content.media_urls.len() > limits.max_images {
            return Err(SocialError::ValidationError(format!(
                "Too many media items (max {})",
                limits.max_images
            )));
        }

        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 2200,
            max_hashtags: 30,
            max_mentions: 20,
            max_images: 10, // Carousel limit
            max_video_length_seconds: 90, // Reels can be longer
            max_image_size_bytes: 8 * 1024 * 1024, // 8MB
            max_video_size_bytes: 100 * 1024 * 1024, // 100MB
            supported_media_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "video/mp4".to_string(),
            ],
        }
    }
}

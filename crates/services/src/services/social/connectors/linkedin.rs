//! LinkedIn Platform Connector
//!
//! Implements OAuth 2.0 authentication and content publishing for LinkedIn.
//! Supports personal profiles and company pages.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};
use db::models::social_account::SocialPlatform;

const LINKEDIN_AUTH_URL: &str = "https://www.linkedin.com/oauth/v2/authorization";
const LINKEDIN_TOKEN_URL: &str = "https://www.linkedin.com/oauth/v2/accessToken";
const LINKEDIN_API_BASE: &str = "https://api.linkedin.com/v2";

pub struct LinkedInConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl LinkedInConnector {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("LINKEDIN_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("LINKEDIN_CLIENT_SECRET").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LinkedInTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinkedInProfile {
    id: String,
    #[serde(rename = "localizedFirstName")]
    first_name: Option<String>,
    #[serde(rename = "localizedLastName")]
    last_name: Option<String>,
    #[serde(rename = "profilePicture")]
    profile_picture: Option<LinkedInProfilePicture>,
}

#[derive(Debug, Deserialize)]
struct LinkedInProfilePicture {
    #[serde(rename = "displayImage~")]
    display_image: Option<LinkedInDisplayImage>,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkedInDisplayImage {
    elements: Vec<LinkedInImageElement>,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkedInImageElement {
    identifiers: Vec<LinkedInImageIdentifier>,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkedInImageIdentifier {
    identifier: String,
}

#[derive(Debug, Serialize)]
struct LinkedInShareContent {
    author: String,
    #[serde(rename = "lifecycleState")]
    lifecycle_state: String,
    #[serde(rename = "specificContent")]
    specific_content: LinkedInSpecificContent,
    visibility: LinkedInVisibility,
}

#[derive(Debug, Serialize)]
struct LinkedInSpecificContent {
    #[serde(rename = "com.linkedin.ugc.ShareContent")]
    share_content: LinkedInShareBody,
}

#[derive(Debug, Serialize)]
struct LinkedInShareBody {
    #[serde(rename = "shareCommentary")]
    share_commentary: LinkedInText,
    #[serde(rename = "shareMediaCategory")]
    share_media_category: String,
    media: Option<Vec<LinkedInMedia>>,
}

#[derive(Debug, Serialize)]
struct LinkedInText {
    text: String,
}

#[derive(Debug, Serialize)]
struct LinkedInMedia {
    status: String,
    #[serde(rename = "originalUrl")]
    original_url: Option<String>,
    media: Option<String>,
    title: Option<LinkedInText>,
}

#[derive(Debug, Serialize)]
struct LinkedInVisibility {
    #[serde(rename = "com.linkedin.ugc.MemberNetworkVisibility")]
    visibility: String,
}

#[derive(Debug, Deserialize)]
struct LinkedInPostResponse {
    id: String,
}

#[async_trait]
impl PlatformConnector for LinkedInConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::LinkedIn
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "r_liteprofile w_member_social";

        let url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}",
            LINKEDIN_AUTH_URL,
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
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = self
            .client
            .post(LINKEDIN_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!(
                "LinkedIn token exchange failed: {}",
                error_text
            )));
        }

        let token_response: LinkedInTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = Utc::now() + chrono::Duration::seconds(token_response.expires_in);

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Some(expires_at),
            token_type: "Bearer".to_string(),
            scope: token_response.scope,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = self
            .client
            .post(LINKEDIN_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::AuthError("Token refresh failed".to_string()));
        }

        let token_response: LinkedInTokenResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let expires_at = Utc::now() + chrono::Duration::seconds(token_response.expires_in);

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Some(expires_at),
            token_type: "Bearer".to_string(),
            scope: token_response.scope,
        })
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        let response = self
            .client
            .get(format!("{}/me", LINKEDIN_API_BASE))
            .bearer_auth(access_token)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SocialError::PlatformError("Failed to fetch profile".to_string()));
        }

        let profile: LinkedInProfile = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        let display_name = match (&profile.first_name, &profile.last_name) {
            (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
            (Some(first), None) => Some(first.clone()),
            (None, Some(last)) => Some(last.clone()),
            _ => None,
        };

        let avatar_url = profile
            .profile_picture
            .and_then(|pp| pp.display_image)
            .and_then(|di| di.elements.first().cloned())
            .and_then(|e| e.identifiers.first().cloned())
            .map(|i| i.identifier);

        Ok(ProfileInfo {
            platform_account_id: profile.id.clone(),
            username: profile.id,
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

        // Get user profile to get the URN
        let profile = self.get_profile(access_token).await?;
        let author_urn = format!("urn:li:person:{}", profile.platform_account_id);

        // Build the caption with hashtags
        let mut caption = content.caption.clone();
        if !content.hashtags.is_empty() {
            caption.push_str("\n\n");
            caption.push_str(&content.hashtags.iter().map(|h| format!("#{}", h)).collect::<Vec<_>>().join(" "));
        }

        // Determine media category
        let (media_category, media) = if content.media_urls.is_empty() {
            ("NONE".to_string(), None)
        } else {
            let media_items: Vec<LinkedInMedia> = content
                .media_urls
                .iter()
                .map(|url| LinkedInMedia {
                    status: "READY".to_string(),
                    original_url: Some(url.clone()),
                    media: None,
                    title: None,
                })
                .collect();
            ("ARTICLE".to_string(), Some(media_items))
        };

        let share_content = LinkedInShareContent {
            author: author_urn,
            lifecycle_state: "PUBLISHED".to_string(),
            specific_content: LinkedInSpecificContent {
                share_content: LinkedInShareBody {
                    share_commentary: LinkedInText { text: caption },
                    share_media_category: media_category,
                    media,
                },
            },
            visibility: LinkedInVisibility {
                visibility: "PUBLIC".to_string(),
            },
        };

        let response = self
            .client
            .post(format!("{}/ugcPosts", LINKEDIN_API_BASE))
            .bearer_auth(access_token)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&share_content)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!(
                "LinkedIn publish failed: {}",
                error_text
            )));
        }

        let post_response: LinkedInPostResponse = response
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;

        Ok(PublishResult {
            platform: SocialPlatform::LinkedIn,
            platform_post_id: post_response.id.clone(),
            platform_url: Some(format!(
                "https://www.linkedin.com/feed/update/{}",
                post_response.id
            )),
            published_at: Utc::now(),
        })
    }

    async fn get_metrics(
        &self,
        _access_token: &str,
        _platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError> {
        // LinkedIn analytics API requires additional permissions
        // Return empty metrics for now
        Ok(EngagementMetrics::default())
    }

    async fn fetch_mentions(
        &self,
        _access_token: &str,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError> {
        // LinkedIn doesn't have a standard mentions API
        // Would need to poll comments on posts
        Ok(vec![])
    }

    async fn reply_to_mention(
        &self,
        _access_token: &str,
        _mention_id: &str,
        _content: &str,
    ) -> Result<String, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "LinkedIn comment replies not yet implemented".to_string(),
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
            max_caption_length: 3000,
            max_hashtags: 30,
            max_mentions: 50,
            max_images: 9,
            max_video_length_seconds: 600, // 10 minutes
            max_image_size_bytes: 8 * 1024 * 1024, // 8MB
            max_video_size_bytes: 200 * 1024 * 1024, // 200MB
            supported_media_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "video/mp4".to_string(),
            ],
        }
    }
}

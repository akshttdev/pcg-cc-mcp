//! Social Media Platform Services
//!
//! Provides unified interface for social media platform operations including:
//! - Account connection and OAuth handling
//! - Content publishing across platforms
//! - Engagement monitoring and inbox management
//! - Analytics collection

pub mod connectors;
pub mod publisher;
pub mod scheduler;

pub use publisher::Publisher;
pub use scheduler::Scheduler;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SocialError {
    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),
    #[error("Authentication failed: {0}")]
    AuthError(String),
    #[error("Rate limited by platform")]
    RateLimited,
    #[error("Content validation failed: {0}")]
    ValidationError(String),
    #[error("Platform API error: {0}")]
    PlatformError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Social account error: {0}")]
    AccountError(String),
    #[error("Social post error: {0}")]
    PostError(String),
    #[error("Social mention error: {0}")]
    MentionError(String),
}

impl From<db::models::social_account::SocialAccountError> for SocialError {
    fn from(err: db::models::social_account::SocialAccountError) -> Self {
        SocialError::AccountError(err.to_string())
    }
}

impl From<db::models::social_post::SocialPostError> for SocialError {
    fn from(err: db::models::social_post::SocialPostError) -> Self {
        SocialError::PostError(err.to_string())
    }
}

impl From<db::models::social_mention::SocialMentionError> for SocialError {
    fn from(err: db::models::social_mention::SocialMentionError) -> Self {
        SocialError::MentionError(err.to_string())
    }
}

/// Result of publishing a post to a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub platform: SocialPlatform,
    pub platform_post_id: String,
    pub platform_url: Option<String>,
    pub published_at: DateTime<Utc>,
}

/// Engagement metrics from a platform
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngagementMetrics {
    pub impressions: i64,
    pub reach: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub saves: i64,
    pub clicks: i64,
}

/// Profile information from a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub platform_account_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub profile_url: Option<String>,
    pub avatar_url: Option<String>,
    pub follower_count: Option<i64>,
    pub following_count: Option<i64>,
    pub post_count: Option<i64>,
    pub is_verified: bool,
}

/// Content to be published
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishContent {
    pub caption: String,
    pub media_urls: Vec<String>,
    pub hashtags: Vec<String>,
    pub mentions: Vec<String>,
    pub link: Option<String>,
    pub scheduled_for: Option<DateTime<Utc>>,
}

/// Platform-specific content adaptations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformContent {
    pub platform: SocialPlatform,
    pub caption: Option<String>,
    pub hashtags: Option<Vec<String>>,
    pub media_urls: Option<Vec<String>>,
    pub extra: Option<serde_json::Value>,
}

/// Trait for platform-specific connectors
#[async_trait]
pub trait PlatformConnector: Send + Sync {
    /// Get the platform this connector handles
    fn platform(&self) -> SocialPlatform;

    /// Generate OAuth authorization URL
    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError>;

    /// Exchange OAuth code for tokens
    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, SocialError>;

    /// Refresh access token
    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, SocialError>;

    /// Get profile information for connected account
    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError>;

    /// Publish content to the platform
    async fn publish(
        &self,
        access_token: &str,
        content: &PublishContent,
    ) -> Result<PublishResult, SocialError>;

    /// Get engagement metrics for a post
    async fn get_metrics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError>;

    /// Fetch recent mentions/comments/DMs
    async fn fetch_mentions(
        &self,
        access_token: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError>;

    /// Reply to a mention/comment
    async fn reply_to_mention(
        &self,
        access_token: &str,
        mention_id: &str,
        content: &str,
    ) -> Result<String, SocialError>;

    /// Validate content before publishing
    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError>;

    /// Get platform-specific character limits
    fn get_limits(&self) -> PlatformLimits;
}

/// OAuth tokens returned from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub token_type: String,
    pub scope: Option<String>,
}

/// Mention/engagement from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMention {
    pub platform_mention_id: String,
    pub mention_type: String,
    pub author_username: String,
    pub author_display_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub author_follower_count: Option<i64>,
    pub author_is_verified: bool,
    pub content: String,
    pub media_urls: Vec<String>,
    pub parent_post_id: Option<String>,
    pub received_at: DateTime<Utc>,
}

/// Platform content limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformLimits {
    pub max_caption_length: usize,
    pub max_hashtags: usize,
    pub max_mentions: usize,
    pub max_images: usize,
    pub max_video_length_seconds: usize,
    pub max_image_size_bytes: usize,
    pub max_video_size_bytes: usize,
    pub supported_media_types: Vec<String>,
}

/// Get connector for a specific platform
pub fn get_connector(platform: SocialPlatform) -> Result<Box<dyn PlatformConnector>, SocialError> {
    match platform {
        SocialPlatform::LinkedIn => Ok(Box::new(connectors::linkedin::LinkedInConnector::new())),
        SocialPlatform::Instagram => Ok(Box::new(connectors::instagram::InstagramConnector::new())),
        _ => Err(SocialError::UnsupportedPlatform(platform.to_string())),
    }
}

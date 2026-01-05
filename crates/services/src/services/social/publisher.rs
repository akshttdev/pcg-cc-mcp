//! Social Media Publisher Service
//!
//! Handles publishing content to multiple platforms with retry logic,
//! rate limiting, and error handling.

use sqlx::SqlitePool;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{get_connector, PublishContent, PublishResult, SocialError};
use db::models::social_account::{SocialAccount, SocialPlatform};
use db::models::social_post::{SocialPost, UpdateSocialPost, PostStatus};

/// Publisher configuration
pub struct PublisherConfig {
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_seconds: 30,
        }
    }
}

/// Social Media Publisher
pub struct Publisher {
    pool: SqlitePool,
    config: PublisherConfig,
}

impl Publisher {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            config: PublisherConfig::default(),
        }
    }

    pub fn with_config(pool: SqlitePool, config: PublisherConfig) -> Self {
        Self { pool, config }
    }

    /// Publish a single post to its target platforms
    pub async fn publish_post(&self, post_id: Uuid) -> Result<Vec<PublishResult>, SocialError> {
        let post = SocialPost::find_by_id(&self.pool, post_id)
            .await
            .map_err(|_| SocialError::PlatformError("Post not found".to_string()))?;

        // Parse target platform account IDs
        let platform_ids: Vec<Uuid> = serde_json::from_str(&post.platforms)
            .map_err(|e| SocialError::PlatformError(format!("Invalid platforms JSON: {}", e)))?;

        let mut results = Vec::new();
        let mut errors = Vec::new();

        for account_id in platform_ids {
            match self.publish_to_account(&post, account_id).await {
                Ok(result) => {
                    info!(
                        "Successfully published post {} to account {}",
                        post_id, account_id
                    );
                    results.push(result);
                }
                Err(e) => {
                    error!(
                        "Failed to publish post {} to account {}: {}",
                        post_id, account_id, e
                    );
                    errors.push(e);
                }
            }
        }

        // Update post status based on results
        if results.is_empty() {
            // All failed
            if let Some(error) = errors.first() {
                SocialPost::mark_failed(&self.pool, post_id, &error.to_string()).await?;
            }
        } else if errors.is_empty() {
            // All succeeded
            if let Some(result) = results.first() {
                SocialPost::mark_published(
                    &self.pool,
                    post_id,
                    &result.platform_post_id,
                    result.platform_url.as_deref(),
                )
                .await?;
            }
        } else {
            // Partial success - mark as published but log errors
            if let Some(result) = results.first() {
                SocialPost::mark_published(
                    &self.pool,
                    post_id,
                    &result.platform_post_id,
                    result.platform_url.as_deref(),
                )
                .await?;
            }
            warn!(
                "Post {} published with {} successes and {} failures",
                post_id,
                results.len(),
                errors.len()
            );
        }

        Ok(results)
    }

    /// Publish to a specific account with retry logic
    async fn publish_to_account(
        &self,
        post: &SocialPost,
        account_id: Uuid,
    ) -> Result<PublishResult, SocialError> {
        let account = SocialAccount::find_by_id(&self.pool, account_id)
            .await
            .map_err(|_| SocialError::PlatformError("Account not found".to_string()))?;

        let access_token = account
            .access_token
            .ok_or_else(|| SocialError::AuthError("No access token".to_string()))?;

        // Parse platform
        let platform: SocialPlatform = match account.platform.as_str() {
            "instagram" => SocialPlatform::Instagram,
            "linkedin" => SocialPlatform::LinkedIn,
            "twitter" => SocialPlatform::Twitter,
            "tiktok" => SocialPlatform::TikTok,
            "youtube" => SocialPlatform::YouTube,
            "facebook" => SocialPlatform::Facebook,
            "threads" => SocialPlatform::Threads,
            "bluesky" => SocialPlatform::Bluesky,
            "pinterest" => SocialPlatform::Pinterest,
            _ => return Err(SocialError::UnsupportedPlatform(account.platform)),
        };

        let connector = get_connector(platform)?;

        // Build content
        let content = PublishContent {
            caption: post.caption.clone().unwrap_or_default(),
            media_urls: post
                .media_urls
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            hashtags: post
                .hashtags
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            mentions: post
                .mentions
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            link: None,
            scheduled_for: post.scheduled_for,
        };

        // Retry logic
        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                info!(
                    "Retrying publish (attempt {}/{}) for post {}",
                    attempt + 1,
                    self.config.max_retries + 1,
                    post.id
                );
                tokio::time::sleep(std::time::Duration::from_secs(
                    self.config.retry_delay_seconds * (attempt as u64),
                ))
                .await;
            }

            match connector.publish(&access_token, &content).await {
                Ok(result) => return Ok(result),
                Err(SocialError::RateLimited) => {
                    warn!("Rate limited, will retry after delay");
                    last_error = Some(SocialError::RateLimited);
                }
                Err(e) => {
                    last_error = Some(e);
                    // Don't retry for non-retryable errors
                    break;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SocialError::PlatformError("Unknown error".to_string())))
    }

    /// Process all posts that are due for publishing
    pub async fn process_scheduled_posts(&self) -> Result<usize, SocialError> {
        let due_posts = SocialPost::find_due_for_publish(&self.pool).await?;

        let mut published_count = 0;

        for post in due_posts {
            info!("Processing scheduled post {}", post.id);

            // Mark as publishing
            SocialPost::update(
                &self.pool,
                post.id,
                UpdateSocialPost {
                    status: Some(PostStatus::Publishing),
                    ..Default::default()
                },
            )
            .await?;

            match self.publish_post(post.id).await {
                Ok(results) if !results.is_empty() => {
                    published_count += 1;
                }
                Ok(_) => {
                    warn!("Post {} had no successful publishes", post.id);
                }
                Err(e) => {
                    error!("Failed to publish post {}: {}", post.id, e);
                }
            }
        }

        Ok(published_count)
    }
}

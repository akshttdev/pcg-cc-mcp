//! Retry wrapper for social platform connectors
//!
//! Wraps any PlatformConnector to add exponential backoff retry logic
//! for transient errors like rate limiting and network failures.

use std::time::Duration;

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

/// A wrapper that adds retry logic to any PlatformConnector.
///
/// Uses exponential backoff with jitter for rate limiting and network errors.
/// Non-retryable errors (auth, validation, platform) are passed through immediately.
pub struct RetryingConnector<C: PlatformConnector> {
    inner: C,
    max_retries: usize,
    min_delay: Duration,
    max_delay: Duration,
}

impl<C: PlatformConnector> RetryingConnector<C> {
    /// Create a new retrying connector with default settings.
    ///
    /// Defaults: 3 retries, 1s min delay, 30s max delay
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            max_retries: 3,
            min_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }

    /// Create a retrying connector with custom settings.
    pub fn with_settings(
        inner: C,
        max_retries: usize,
        min_delay: Duration,
        max_delay: Duration,
    ) -> Self {
        Self {
            inner,
            max_retries,
            min_delay,
            max_delay,
        }
    }

    fn retry_builder(&self) -> ExponentialBuilder {
        ExponentialBuilder::default()
            .with_min_delay(self.min_delay)
            .with_max_delay(self.max_delay)
            .with_max_times(self.max_retries)
            .with_jitter()
    }

    fn log_retry(err: &SocialError, dur: Duration, platform: SocialPlatform) {
        tracing::warn!(
            platform = %platform,
            delay_secs = dur.as_secs_f64(),
            error = %err,
            "Retrying social API call after transient error"
        );
    }
}

#[async_trait]
impl<C: PlatformConnector + 'static> PlatformConnector for RetryingConnector<C> {
    fn platform(&self) -> SocialPlatform {
        self.inner.platform()
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        // Auth URL generation is local - no retry needed
        self.inner.get_auth_url(redirect_uri, state).await
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, SocialError> {
        let platform = self.inner.platform();
        let code = code.to_string();
        let redirect_uri = redirect_uri.to_string();

        (|| async { self.inner.exchange_code(&code, &redirect_uri).await })
            .retry(&self.retry_builder())
            .when(|e| e.should_retry())
            .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
            .await
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        let platform = self.inner.platform();
        let refresh_token = refresh_token.to_string();

        (|| async { self.inner.refresh_token(&refresh_token).await })
            .retry(&self.retry_builder())
            .when(|e| e.should_retry())
            .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
            .await
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        let platform = self.inner.platform();
        let access_token = access_token.to_string();

        (|| async { self.inner.get_profile(&access_token).await })
            .retry(&self.retry_builder())
            .when(|e| e.should_retry())
            .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
            .await
    }

    async fn publish(
        &self,
        access_token: &str,
        content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        let platform = self.inner.platform();
        let access_token = access_token.to_string();
        let content = content.clone();

        (|| async { self.inner.publish(&access_token, &content).await })
            .retry(&self.retry_builder())
            .when(|e| e.should_retry())
            .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
            .await
    }

    async fn get_metrics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError> {
        let platform = self.inner.platform();
        let access_token = access_token.to_string();
        let platform_post_id = platform_post_id.to_string();

        (|| async {
            self.inner
                .get_metrics(&access_token, &platform_post_id)
                .await
        })
        .retry(&self.retry_builder())
        .when(|e| e.should_retry())
        .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
        .await
    }

    async fn fetch_mentions(
        &self,
        access_token: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError> {
        let platform = self.inner.platform();
        let access_token = access_token.to_string();

        (|| async { self.inner.fetch_mentions(&access_token, since).await })
            .retry(&self.retry_builder())
            .when(|e| e.should_retry())
            .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
            .await
    }

    async fn reply_to_mention(
        &self,
        access_token: &str,
        mention_id: &str,
        content: &str,
    ) -> Result<String, SocialError> {
        let platform = self.inner.platform();
        let access_token = access_token.to_string();
        let mention_id = mention_id.to_string();
        let content = content.to_string();

        (|| async {
            self.inner
                .reply_to_mention(&access_token, &mention_id, &content)
                .await
        })
        .retry(&self.retry_builder())
        .when(|e| e.should_retry())
        .notify(|err: &SocialError, dur: Duration| Self::log_retry(err, dur, platform))
        .await
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        // Validation is local - no retry needed
        self.inner.validate_content(content)
    }

    fn get_limits(&self) -> PlatformLimits {
        self.inner.get_limits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_retry_rate_limited() {
        assert!(SocialError::RateLimited.should_retry());
    }

    #[test]
    fn test_should_retry_network_error() {
        assert!(SocialError::NetworkError("connection reset".into()).should_retry());
    }

    #[test]
    fn test_should_not_retry_auth_error() {
        assert!(!SocialError::AuthError("invalid token".into()).should_retry());
    }

    #[test]
    fn test_should_not_retry_validation_error() {
        assert!(!SocialError::ValidationError("caption too long".into()).should_retry());
    }
}

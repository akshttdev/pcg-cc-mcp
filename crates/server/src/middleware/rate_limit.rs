use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Rate limiting configuration for Nora endpoints
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum requests per period
    pub max_requests: u32,
    /// Time period for rate limiting (in seconds)
    pub period_seconds: u64,
    /// Burst size (how many requests can be made at once)
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,   // 60 requests
            period_seconds: 60, // per minute
            burst_size: 10,     // burst of 10
        }
    }
}

impl RateLimitConfig {
    /// Create a permissive rate limit (for development/testing)
    pub fn permissive() -> Self {
        Self {
            max_requests: 1000,
            period_seconds: 60,
            burst_size: 100,
        }
    }

    /// Create a strict rate limit (for production with quota concerns)
    pub fn strict() -> Self {
        Self {
            max_requests: 20,
            period_seconds: 60,
            burst_size: 5,
        }
    }

    /// Create a conservative rate limit (balanced approach)
    pub fn conservative() -> Self {
        Self {
            max_requests: 40,
            period_seconds: 60,
            burst_size: 8,
        }
    }
}

/// Custom rate limit error response
pub struct RateLimitExceeded;

impl IntoResponse for RateLimitExceeded {
    fn into_response(self) -> Response {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please slow down your requests to Nora.",
        )
            .into_response()
    }
}

/// Simple token bucket rate limiter for in-handler use
pub struct TokenBucket {
    tokens: Arc<tokio::sync::Mutex<f64>>,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Arc<tokio::sync::Mutex<std::time::Instant>>,
}

impl TokenBucket {
    /// Create a new token bucket
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(tokio::sync::Mutex::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: Arc::new(tokio::sync::Mutex::new(std::time::Instant::now())),
        }
    }

    /// Try to consume a token, returns true if successful
    pub async fn try_consume(&self) -> bool {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;

        // Refill tokens based on time elapsed
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        *tokens = (*tokens + new_tokens).min(self.max_tokens);
        *last_refill = now;

        // Try to consume a token
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get current token count
    pub async fn available_tokens(&self) -> f64 {
        let tokens = self.tokens.lock().await;
        *tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 60);
        assert_eq!(config.period_seconds, 60);
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn test_rate_limit_config_presets() {
        let permissive = RateLimitConfig::permissive();
        assert_eq!(permissive.max_requests, 1000);

        let strict = RateLimitConfig::strict();
        assert_eq!(strict.max_requests, 20);

        let conservative = RateLimitConfig::conservative();
        assert_eq!(conservative.max_requests, 40);
    }

    #[tokio::test]
    async fn test_token_bucket_consumption() {
        let bucket = TokenBucket::new(5.0, 1.0);

        // Should be able to consume 5 tokens
        for _ in 0..5 {
            assert!(bucket.try_consume().await);
        }

        // 6th token should fail
        assert!(!bucket.try_consume().await);
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(2.0, 10.0); // 10 tokens per second

        // Consume all tokens
        assert!(bucket.try_consume().await);
        assert!(bucket.try_consume().await);
        assert!(!bucket.try_consume().await);

        // Wait for refill (100ms should refill 1 token at 10/sec)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should be able to consume again
        assert!(bucket.try_consume().await);
    }

    #[tokio::test]
    async fn test_token_bucket_available() {
        let bucket = TokenBucket::new(10.0, 1.0);
        let available = bucket.available_tokens().await;
        assert_eq!(available, 10.0);

        bucket.try_consume().await;
        let available = bucket.available_tokens().await;
        assert_eq!(available, 9.0);
    }
}

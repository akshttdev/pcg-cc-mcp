//! Chat and voice rate limiters for Nora endpoints.

use std::sync::Arc;
use crate::middleware::rate_limit::TokenBucket;

/// Global rate limiter for chat endpoints (20 req/min, refill 1 per 3 seconds)
pub(crate) static CHAT_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

/// Global rate limiter for voice synthesis (30 req/min, refill 1 per 2 seconds)
pub(crate) static VOICE_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

/// Get or initialize chat rate limiter
pub(crate) async fn get_chat_rate_limiter() -> &'static Arc<TokenBucket> {
    CHAT_RATE_LIMITER
        .get_or_init(|| async {
            // 20 tokens max, refill at 1 token per 3 seconds (20/min)
            Arc::new(TokenBucket::new(20.0, 1.0 / 3.0))
        })
        .await
}

/// Get or initialize voice rate limiter
pub(crate) async fn get_voice_rate_limiter() -> &'static Arc<TokenBucket> {
    VOICE_RATE_LIMITER
        .get_or_init(|| async {
            // 30 tokens max, refill at 1 token per 2 seconds (30/min)
            Arc::new(TokenBucket::new(30.0, 0.5))
        })
        .await
}

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};

use moka::future::Cache;
use serde::{Deserialize, Serialize};

/// Cache key for LLM responses
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    /// Hash of the request content
    content_hash: u64,
    /// Request type identifier
    request_type: String,
}

impl CacheKey {
    /// Create a new cache key from request content and type
    pub fn new(content: &str, request_type: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let content_hash = hasher.finish();

        Self {
            content_hash,
            request_type: request_type.to_string(),
        }
    }
}

/// Cached LLM response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The LLM response content
    pub content: String,
    /// Timestamp when cached
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// Original request metadata
    pub metadata: ResponseMetadata,
}

/// Metadata about the cached response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// LLM provider used
    pub provider: String,
    /// Model used
    pub model: String,
    /// Token count (if available)
    pub tokens: Option<usize>,
}

/// LRU cache for LLM responses
#[derive(Debug)]
pub struct LlmCache {
    cache: Cache<CacheKey, Arc<CachedResponse>>,
    hits: Arc<std::sync::atomic::AtomicU64>,
    misses: Arc<std::sync::atomic::AtomicU64>,
}

impl LlmCache {
    /// Create a new LLM cache with the specified capacity and TTL
    ///
    /// # Arguments
    /// * `max_capacity` - Maximum number of entries to cache
    /// * `ttl_seconds` - Time-to-live in seconds for cache entries
    pub fn new(max_capacity: u64, ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        Self {
            cache,
            hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Create a default cache with reasonable settings
    /// - 1000 entries max
    /// - 1 hour TTL
    pub fn default() -> Self {
        Self::new(1000, 3600)
    }

    /// Get a cached response if available
    pub async fn get(&self, key: &CacheKey) -> Option<Arc<CachedResponse>> {
        let result = self.cache.get(key).await;
        if result.is_some() {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }

    /// Store a response in the cache
    pub async fn put(&self, key: CacheKey, response: CachedResponse) {
        self.cache.insert(key, Arc::new(response)).await;
    }

    /// Check if a response is cached
    pub async fn contains(&self, key: &CacheKey) -> bool {
        self.cache.get(key).await.is_some()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
            hits,
            misses,
            hit_rate,
        }
    }

    /// Clear all cache entries
    pub async fn invalidate_all(&self) {
        self.cache.invalidate_all();
        // Wait for invalidation to complete
        self.cache.run_pending_tasks().await;
    }

    /// Remove a specific cache entry
    pub async fn invalidate(&self, key: &CacheKey) {
        self.cache.invalidate(key).await;
    }
}

/// Cache statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    /// Number of entries in the cache
    pub entry_count: u64,
    /// Total weighted size of the cache
    pub weighted_size: u64,
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Hit rate (hits / (hits + misses))
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_key_generation() {
        let key1 = CacheKey::new("Hello world", "text");
        let key2 = CacheKey::new("Hello world", "text");
        let key3 = CacheKey::new("Different content", "text");
        let key4 = CacheKey::new("Hello world", "voice");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key1, key4);
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let cache = LlmCache::new(10, 60);
        let key = CacheKey::new("test content", "text");
        let response = CachedResponse {
            content: "Test response".to_string(),
            cached_at: chrono::Utc::now(),
            metadata: ResponseMetadata {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                tokens: Some(100),
            },
        };

        cache.put(key.clone(), response.clone()).await;
        let cached = cache.get(&key).await;

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.content, "Test response");
        assert_eq!(cached.metadata.provider, "openai");
    }

    #[tokio::test]
    async fn test_cache_contains() {
        let cache = LlmCache::new(10, 60);
        let key = CacheKey::new("test", "text");

        assert!(!cache.contains(&key).await);

        let response = CachedResponse {
            content: "Test".to_string(),
            cached_at: chrono::Utc::now(),
            metadata: ResponseMetadata {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                tokens: None,
            },
        };
        cache.put(key.clone(), response).await;

        assert!(cache.contains(&key).await);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = LlmCache::new(10, 60);
        let key = CacheKey::new("test", "text");
        let response = CachedResponse {
            content: "Test".to_string(),
            cached_at: chrono::Utc::now(),
            metadata: ResponseMetadata {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                tokens: None,
            },
        };

        cache.put(key.clone(), response).await;
        assert!(cache.contains(&key).await);

        cache.invalidate(&key).await;
        assert!(!cache.contains(&key).await);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = LlmCache::new(10, 60);
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);

        let response = CachedResponse {
            content: "Test".to_string(),
            cached_at: chrono::Utc::now(),
            metadata: ResponseMetadata {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                tokens: None,
            },
        };

        cache
            .put(CacheKey::new("test1", "text"), response.clone())
            .await;
        cache
            .put(CacheKey::new("test2", "text"), response.clone())
            .await;

        // Run pending tasks to ensure cache is updated
        cache.cache.run_pending_tasks().await;

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 2);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        // Create cache with 1 second TTL
        let cache = LlmCache::new(10, 1);
        let key = CacheKey::new("test", "text");
        let response = CachedResponse {
            content: "Test".to_string(),
            cached_at: chrono::Utc::now(),
            metadata: ResponseMetadata {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                tokens: None,
            },
        };

        cache.put(key.clone(), response).await;
        assert!(cache.contains(&key).await);

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Entry should be evicted
        assert!(!cache.contains(&key).await);
    }
}

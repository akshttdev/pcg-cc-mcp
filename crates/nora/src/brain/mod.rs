use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ts_rs::TS;
use futures::stream::Stream;
use std::pin::Pin;

use crate::{cache::{CacheKey, CachedResponse, LlmCache, ResponseMetadata}, NoraError, Result};

/// Supported LLM providers for Nora
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LLMProvider {
    OpenAI,
}

/// Configuration for Nora's reasoning engine
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LLMConfig {
    pub provider: LLMProvider,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
    #[serde(default)]
    pub endpoint: Option<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::OpenAI,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.2,
            max_tokens: 600,
            system_prompt: "You are Nora, the executive AI assistant for PowerClub Global. Respond in confident British English, provide concise executive summaries, and surface relevant projects, stakeholders, or next actions. Offer follow-up suggestions only when useful.\n\nYou have access to the Master_Cinematographer agent, which can create cinematic content using Stable Diffusion via ComfyUI. When users ask about video generation, cinematic content, or the Master_Cinematographer agent, inform them that you can create cinematic briefs that will be processed by this specialized agent. The Master_Cinematographer uses ComfyUI with AnimateDiff and VideoHelperSuite for video generation.".to_string(),
            endpoint: None,
        }
    }
}

/// Thin wrapper around the configured LLM provider
#[derive(Debug, Clone)]
pub struct LLMClient {
    config: LLMConfig,
    client: Client,
    api_key: Option<String>,
    cache: Arc<LlmCache>,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let api_key = match config.provider {
            LLMProvider::OpenAI => std::env::var("OPENAI_API_KEY").ok(),
        };

        Self {
            config,
            client: Client::new(),
            api_key,
            cache: Arc::new(LlmCache::default()),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.api_key.is_some() || self.config.endpoint.is_some()
    }

    /// Check if LLM is configured and operational
    pub fn is_configured(&self) -> bool {
        self.config.endpoint.is_some() || self.api_key.is_some()
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }

    /// Clear the LLM cache
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all().await;
    }

    pub async fn generate(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<String> {
        // Generate cache key from content and request type
        let cache_content = format!("{}\n{}\n{}", system_prompt, user_query, context);
        let cache_key = CacheKey::new(&cache_content, "llm_generate");

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key).await {
            tracing::info!("LLM cache hit for request");
            return Ok(cached.content.clone());
        }

        // Cache miss - generate from LLM
        tracing::info!("LLM cache miss - generating from provider");
        let start = std::time::Instant::now();
        let content = match self.config.provider {
            LLMProvider::OpenAI => {
                self.generate_openai(system_prompt, user_query, context)
                    .await?
            }
        };
        let duration = start.elapsed();

        // Cache the response
        let cached_response = CachedResponse {
            content: content.clone(),
            cached_at: chrono::Utc::now(),
            metadata: ResponseMetadata {
                provider: format!("{:?}", self.config.provider),
                model: self.config.model.clone(),
                tokens: None, // TODO: Extract from response
            },
        };
        self.cache.put(cache_key, cached_response).await;

        tracing::debug!("LLM response generated in {:?}", duration);
        Ok(content)
    }

    /// Generate a streaming response from the LLM
    /// Returns a stream of text chunks as they arrive
    pub async fn generate_stream(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // Note: Streaming responses bypass the cache
        // We could cache the full response after streaming completes, but that's TODO
        tracing::info!("LLM streaming request (cache bypassed)");
        
        match self.config.provider {
            LLMProvider::OpenAI => {
                self.generate_openai_stream(system_prompt, user_query, context)
                    .await
            }
        }
    }

    async fn generate_openai(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<String> {
        let endpoint = self
            .config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.config.endpoint.is_none() {
            return Err(NoraError::ConfigError(
                "LLM configured without OPENAI_API_KEY or custom endpoint".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        let payload = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "messages": [
                { "role": "system", "content": system },
                {
                    "role": "user",
                    "content": format!(
                        "Context:\n{context}\n\nExecutive request:\n{user_query}",
                        context = context,
                        user_query = user_query
                    )
                }
            ]
        });

        let client = self
            .client
            .post(endpoint)
            .header("Content-Type", "application/json");

        let client = if let Some(auth) = auth_header {
            client.header("Authorization", auth)
        } else {
            client
        };

        let response = client.json(&payload).send().await.map_err(|e| {
            tracing::warn!("LLM request failed: {}", e);
            NoraError::LLMError(format!("Failed to send request: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("LLM API error ({}): {}", status, body);
            return Err(NoraError::LLMError(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            tracing::warn!("Failed to parse LLM response: {}", e);
            NoraError::LLMError(format!("Failed to parse response: {}", e))
        })?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            tracing::warn!("LLM returned empty content");
            return Err(NoraError::LLMError(
                "OpenAI returned an empty response".to_string(),
            ));
        }

        tracing::debug!("LLM response received: {} chars", content.len());
        Ok(content)
    }

    async fn generate_openai_stream(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        use futures::stream::StreamExt;

        let endpoint = self
            .config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.config.endpoint.is_none() {
            return Err(NoraError::ConfigError(
                "LLM configured without OPENAI_API_KEY or custom endpoint".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        let payload = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "stream": true,
            "messages": [
                { "role": "system", "content": system },
                {
                    "role": "user",
                    "content": format!(
                        "Context:\n{context}\n\nExecutive request:\n{user_query}",
                        context = context,
                        user_query = user_query
                    )
                }
            ]
        });

        let client = self
            .client
            .post(endpoint)
            .header("Content-Type", "application/json");

        let client = if let Some(auth) = auth_header {
            client.header("Authorization", auth)
        } else {
            client
        };

        let response = client.json(&payload).send().await.map_err(|e| {
            tracing::warn!("LLM stream request failed: {}", e);
            NoraError::LLMError(format!("Failed to send streaming request: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("LLM API error ({}): {}", status, body);
            return Err(NoraError::LLMError(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        // Convert response bytes stream to text chunks
        let byte_stream = response.bytes_stream();
        
        let text_stream = byte_stream.map(move |chunk_result| {
            let chunk = chunk_result.map_err(|e| {
                NoraError::LLMError(format!("Stream chunk error: {}", e))
            })?;
            
            // Parse SSE format: "data: {...}\n\n"
            let text = String::from_utf8_lossy(&chunk);
            let mut content_chunks = Vec::new();
            
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let json_str = line.strip_prefix("data: ").unwrap_or("");
                    
                    // OpenAI sends "[DONE]" as the final message
                    if json_str == "[DONE]" {
                        continue;
                    }
                    
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(delta_content) = json["choices"][0]["delta"]["content"].as_str() {
                            content_chunks.push(delta_content.to_string());
                        }
                    }
                }
            }
            
            Ok(content_chunks.join(""))
        });

        Ok(Box::pin(text_stream))
    }
}

use std::{pin::Pin, sync::Arc};

use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    cache::{CacheKey, CachedResponse, LlmCache, ResponseMetadata},
    NoraError, Result,
};

// Multi-provider abstraction layer
pub mod providers;
pub use providers::{
    AnthropicProvider, LLMProviderTrait, OpenAIProvider, ProviderError, ProviderType, TokenUsage,
};

// Agent-specific LLM client configuration
pub mod agent_client;
pub use agent_client::{
    create_client_for_agent, infer_provider_from_model, AgentModelConfig,
};

/// A message in the conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
}

/// Role of a message in conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl ConversationMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Tool call request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of a tool execution to pass back to LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub success: bool,
    pub result: String,
}

/// Response from LLM that may include tool calls
#[derive(Debug, Clone)]
pub enum LLMResponse {
    /// Direct text response
    Text {
        content: String,
        usage: Option<TokenUsage>,
    },
    /// LLM wants to call tools
    ToolCalls {
        calls: Vec<ToolCall>,
        usage: Option<TokenUsage>,
    },
}

impl LLMResponse {
    /// Get token usage if available
    pub fn usage(&self) -> Option<&TokenUsage> {
        match self {
            LLMResponse::Text { usage, .. } => usage.as_ref(),
            LLMResponse::ToolCalls { usage, .. } => usage.as_ref(),
        }
    }

    /// Get text content if this is a text response
    pub fn content(&self) -> Option<&str> {
        match self {
            LLMResponse::Text { content, .. } => Some(content),
            LLMResponse::ToolCalls { .. } => None,
        }
    }

    /// Get tool calls if this is a tool calls response
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        match self {
            LLMResponse::Text { .. } => None,
            LLMResponse::ToolCalls { calls, .. } => Some(calls),
        }
    }
}

/// Supported LLM providers for Nora
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    /// Ollama local LLM server (OpenAI-compatible API)
    Ollama,
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
            provider: LLMProvider::Ollama,
            model: "gpt-oss".to_string(),
            temperature: 0.2,
            max_tokens: 600,
            system_prompt: r#"You are Nora, the executive AI assistant for PowerClub Global. Respond in confident British English.

CRITICAL - TOOL USAGE REQUIREMENT:
When the user mentions ANY of these agents or requests creative content, you MUST call the execute_workflow tool immediately. Do NOT just respond with text about "initiating" or "processing" - actually CALL THE TOOL:

AGENT ALIASES (all map to execute_workflow):
- "Maci", "Master Cinematographer", "Spectra" → agent_id='master-cinematographer', workflow_id='ai-cinematic-suite'
- "Editron" → agent_id='editron-post', workflow_id='event-recap-forge'
- "Astra" → agent_id='astra-strategy', workflow_id='roadmap-compression'

SOCIAL COMMAND AGENT ALIASES:
- "Scout" → agent_id='scout-research', workflow_id='competitor-deep-dive'
- "Oracle" → agent_id='oracle-strategy', workflow_id='content-calendar-30day'
- "Muse" → agent_id='muse-creative', workflow_id='content-creation'
- "Herald" → agent_id='herald-distribution', workflow_id='content-publishing'
- "Echo" → agent_id='echo-engagement', workflow_id='engagement-response'

TRIGGER PHRASES that require tool calls:
- "tell [agent] to...", "have [agent] generate...", "ask [agent] to..."
- "generate an image", "create a video", "make a cinematic"
- Any request mentioning image generation, video editing, or content creation
- "research competitors", "analyze competition" → Scout
- "create content calendar", "plan content strategy" → Oracle
- "write a post", "create content", "draft copy" → Muse
- "publish", "schedule post", "distribute content" → Herald
- "check mentions", "respond to comments", "monitor engagement" → Echo

Example: "tell Maci to generate an image of a casino" → CALL execute_workflow with agent_id='master-cinematographer', workflow_id='ai-cinematic-suite', inputs={prompt: 'casino...'}

CRITICAL WORKFLOW EXECUTION RULES:
1. For SOCIAL MEDIA work (strategy, content, research), ALWAYS start with Scout, NOT Astra:
   - Social strategy/research → Scout (agent_id='scout-research', workflow_id='competitor-deep-dive')
   - Content planning → Oracle (agent_id='oracle-strategy', workflow_id='content-calendar-30day')
   - Astra is for PROJECT ROADMAPS only, NOT social media!

2. When executing workflows:
   - FIRST: Call get_project_details to get the project's UUID (the "id" field in the response)
   - Use the UUID from the response, NOT the project name! Example: "a1b2c3d4-..." not "jungleverse"
   - IMMEDIATELY call execute_workflow with that UUID as project_id

3. Project ID MUST be a UUID like "a1b2c3d4-5678-90ab-cdef-123456789abc", NOT the project name!

CORRECT EXAMPLE:
User: "develop social strategy for Jungleverse"
Step 1: Call get_project_details(project_name="jungleverse") → Response includes: {"id": "abc123-def456-..."}
Step 2: Call execute_workflow(agent_id='scout-research', workflow_id='competitor-deep-dive', project_id='abc123-def456-...')
         ↑ Use the UUID from step 1, NOT "jungleverse"!

DO NOT respond with text about initiating workflows - CALL execute_workflow INSTEAD.

You orchestrate a team of specialized agents:

SOCIAL COMMAND TEAM (Full Social Media Suite):
- Scout (Social Intelligence Analyst): Competitor research, trend detection, hashtag analysis, audience profiling
- Oracle (Content Strategy Architect): Content calendar planning, campaign architecture, posting optimization
- Muse (Content Creation Specialist): Copywriting, platform adaptation, visual briefs, hook generation
- Maci (Master Cinematographer): AI image/video generation via ComfyUI - creates visual content for social posts
- Editron (Post-Production): Video editing, reels, stories, event recaps, social hooks
- Herald (Content Distribution Manager): Multi-platform publishing, schedule management, queue rotation
- Echo (Community Engagement Specialist): Mention monitoring, sentiment analysis, response drafting, engagement analytics

STRATEGY:
- Astra: Strategic planning and roadmaps

The Social Command workflow: Scout researches → Oracle plans strategy → Muse writes copy → Maci/Editron create visuals → Herald publishes → Echo monitors engagement.

When asked about social media, content strategy, or the Social Command team, explain these agents and their workflows.

Provide concise executive summaries and surface actionable next steps."#.to_string(),
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
    /// Fallback to local Ollama when primary provider fails
    fallback_endpoint: Option<String>,
    fallback_model: Option<String>,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let api_key = match config.provider {
            LLMProvider::OpenAI => std::env::var("OPENAI_API_KEY").ok(),
            LLMProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok(),
            LLMProvider::Ollama => None, // Ollama doesn't require an API key
        };

        let provider_name = match config.provider {
            LLMProvider::OpenAI => "OpenAI",
            LLMProvider::Anthropic => "Anthropic",
            LLMProvider::Ollama => "Ollama",
        };

        match config.provider {
            LLMProvider::Ollama => {
                tracing::info!("LLMClient initialized with Ollama (local, no API key required)");
            }
            _ if api_key.is_some() => {
                tracing::info!("LLMClient initialized with {} API key", provider_name);
            }
            _ => {
                tracing::warn!(
                    "LLMClient created without API key - {} env var not found",
                    match config.provider {
                        LLMProvider::OpenAI => "OPENAI_API_KEY",
                        LLMProvider::Anthropic => "ANTHROPIC_API_KEY",
                        LLMProvider::Ollama => "N/A",
                    }
                );
            }
        }

        // Check for local Ollama fallback (local LLM models)
        let fallback_endpoint = std::env::var("OLLAMA_ENDPOINT")
            .ok()
            .or_else(|| {
                // Check if Ollama is running on default port
                if std::net::TcpStream::connect("127.0.0.1:11434").is_ok() {
                    Some("http://127.0.0.1:11434/v1/chat/completions".to_string())
                } else {
                    None
                }
            });

        let fallback_model = std::env::var("OLLAMA_MODEL")
            .ok()
            .or_else(|| fallback_endpoint.as_ref().map(|_| "gpt-oss:20b".to_string()));

        if fallback_endpoint.is_some() {
            tracing::info!("LLMClient has Ollama fallback configured: {:?}", fallback_model);
        }

        Self {
            config,
            client: Client::new(),
            api_key,
            cache: Arc::new(LlmCache::default()),
            fallback_endpoint,
            fallback_model,
        }
    }

    pub fn is_ready(&self) -> bool {
        // Ollama is always ready (local server, no API key needed)
        matches!(self.config.provider, LLMProvider::Ollama)
            || self.api_key.is_some()
            || self.config.endpoint.is_some()
    }

    /// Check if LLM is configured and operational
    pub fn is_configured(&self) -> bool {
        // Ollama is always configured (just needs the local server running)
        matches!(self.config.provider, LLMProvider::Ollama)
            || self.config.endpoint.is_some()
            || self.api_key.is_some()
    }

    /// Get the appropriate API endpoint based on provider
    fn get_endpoint(&self) -> String {
        // Custom endpoint takes priority
        if let Some(ref endpoint) = self.config.endpoint {
            return endpoint.clone();
        }

        // Provider-specific default endpoints
        match self.config.provider {
            LLMProvider::Ollama => {
                // Ollama's OpenAI-compatible endpoint
                std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string())
                    + "/v1/chat/completions"
            }
            LLMProvider::OpenAI | LLMProvider::Anthropic => {
                // Default OpenAI endpoint
                "https://api.openai.com/v1/chat/completions".to_string()
            }
        }
    }

    /// Check if authentication is required for the current provider
    fn requires_auth(&self) -> bool {
        // Ollama doesn't require authentication
        !matches!(self.config.provider, LLMProvider::Ollama)
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
            LLMProvider::OpenAI | LLMProvider::Ollama => {
                self.generate_openai(system_prompt, user_query, context)
                    .await?
            }
            LLMProvider::Anthropic => {
                self.generate_anthropic(system_prompt, user_query, context)
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
            LLMProvider::OpenAI | LLMProvider::Ollama => {
                self.generate_openai_stream(system_prompt, user_query, context)
                    .await
            }
            LLMProvider::Anthropic => {
                self.generate_anthropic_stream(system_prompt, user_query, context)
                    .await
            }
        }
    }

    /// Generate a response with function calling support
    /// The LLM may return either a text response or request to call tools
    pub async fn generate_with_tools(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tools: &[serde_json::Value],
    ) -> Result<LLMResponse> {
        // Call the conversation-aware version with empty history for backwards compatibility
        self.generate_with_tools_and_history(system_prompt, user_query, context, tools, &[])
            .await
    }

    /// Generate a response with function calling support AND conversation history
    /// This enables true conversational mode where the LLM remembers previous messages
    pub async fn generate_with_tools_and_history(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tools: &[serde_json::Value],
        conversation_history: &[ConversationMessage],
    ) -> Result<LLMResponse> {
        match self.config.provider {
            LLMProvider::OpenAI | LLMProvider::Ollama => {
                self.generate_openai_with_tools_and_history(
                    system_prompt,
                    user_query,
                    context,
                    tools,
                    conversation_history,
                )
                .await
            }
            LLMProvider::Anthropic => {
                self.generate_anthropic_with_tools_and_history(
                    system_prompt,
                    user_query,
                    context,
                    tools,
                    conversation_history,
                )
                .await
            }
        }
    }

    /// Continue conversation after tool execution, passing results back to LLM
    /// Note: This legacy function doesn't support tool chaining. Use continue_with_tool_results_and_history for that.
    pub async fn continue_with_tool_results(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
    ) -> Result<String> {
        // Call the conversation-aware version with empty history and no tools (no chaining)
        match self.continue_with_tool_results_and_history(
            system_prompt,
            user_query,
            context,
            tool_calls,
            tool_results,
            &[],
            &[], // Empty tools = no chaining support in legacy function
        )
        .await?
        {
            LLMResponse::Text { content, .. } => Ok(content),
            LLMResponse::ToolCalls { .. } => {
                // Legacy function doesn't support chaining, return a message
                Ok("Action completed. Use the conversation-aware API for tool chaining.".to_string())
            }
        }
    }

    /// Continue conversation after tool execution with conversation history
    /// Returns LLMResponse which can be Text or ToolCalls (for chaining)
    pub async fn continue_with_tool_results_and_history(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
        conversation_history: &[ConversationMessage],
        tools: &[serde_json::Value],
    ) -> Result<LLMResponse> {
        match self.config.provider {
            LLMProvider::OpenAI | LLMProvider::Ollama => {
                self.continue_openai_with_tool_results_and_history(
                    system_prompt,
                    user_query,
                    context,
                    tool_calls,
                    tool_results,
                    conversation_history,
                    tools,
                )
                .await
            }
            LLMProvider::Anthropic => {
                self.continue_anthropic_with_tool_results_and_history(
                    system_prompt,
                    user_query,
                    context,
                    tool_calls,
                    tool_results,
                    conversation_history,
                    tools,
                )
                .await
            }
        }
    }

    async fn generate_openai_with_tools_and_history(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tools: &[serde_json::Value],
        conversation_history: &[ConversationMessage],
    ) -> Result<LLMResponse> {
        tracing::debug!("[LLM_API] generate_openai_with_tools_and_history called");
        tracing::debug!(
            "[LLM_API] Model: {}, Tools: {}, History: {} messages",
            self.config.model,
            tools.len(),
            conversation_history.len()
        );

        let endpoint = self.get_endpoint();

        tracing::debug!("[LLM_API] Endpoint: {}", endpoint);

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.requires_auth() {
            tracing::error!("[LLM_API] No API key configured for provider that requires auth");
            return Err(NoraError::ConfigError(
                "LLM configured without required API key".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        // Build messages array with conversation history
        let mut messages: Vec<serde_json::Value> =
            vec![serde_json::json!({ "role": "system", "content": system })];

        // Add conversation history (limit to last 10 messages to control tokens)
        let history_limit = 10;
        let history_start = conversation_history.len().saturating_sub(history_limit);
        for msg in &conversation_history[history_start..] {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => continue, // Skip system messages in history
            };
            messages.push(serde_json::json!({
                "role": role,
                "content": msg.content
            }));
        }

        // Add current request with context
        messages.push(serde_json::json!({
            "role": "user",
            "content": format!(
                "Context:\n{context}\n\nRequest:\n{user_query}",
                context = context,
                user_query = user_query
            )
        }));

        let mut payload = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "messages": messages
        });

        // Add tools if provided
        if !tools.is_empty() {
            payload["tools"] = serde_json::json!(tools);
            payload["tool_choice"] = serde_json::json!("required");
            tracing::debug!(
                "[LLM_API] Added {} tools to payload with tool_choice=auto",
                tools.len()
            );
        }

        tracing::info!(
            "[LLM_API] Sending request to OpenAI API with {} total messages...",
            messages.len()
        );
        eprintln!("[DEBUG] Sending to OpenAI: model={}, tools={}, tool_choice={:?}",
            self.config.model,
            tools.len(),
            payload.get("tool_choice"));

        // Retry loop with exponential backoff for rate limiting (429)
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay_ms = 1000 * (1 << (attempt - 1));
                tracing::info!("[LLM_API] Rate limited, retrying in {}ms (attempt {}/{})", delay_ms, attempt + 1, max_retries);
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            let client = self
                .client
                .post(&endpoint)
                .header("Content-Type", "application/json");

            let client = if let Some(ref auth) = auth_header {
                client.header("Authorization", auth.clone())
            } else {
                client
            };

            let response = match client.json(&payload).send().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("[LLM_API] Request failed: {}", e);
                    last_error = Some(NoraError::LLMError(format!("Failed to send request: {}", e)));
                    continue;
                }
            };

            tracing::debug!("[LLM_API] Response status: {}", response.status());

            if response.status().is_success() {
                // Success - break out of retry loop and continue processing
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    tracing::error!("[LLM_API] Failed to parse response: {}", e);
                    NoraError::LLMError(format!("Failed to parse response: {}", e))
                })?;

                // Return early with the successful response processing
                return self.process_openai_tool_response(json).await;
            }

            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Only retry on 429 (rate limit) or 5xx (server errors)
            if status.as_u16() == 429 || status.is_server_error() {
                tracing::warn!("[LLM_API] Retryable error ({}): {}", status, &body[..body.len().min(200)]);
                last_error = Some(NoraError::LLMError(format!("OpenAI API error ({}): {}", status, body)));
                continue;
            }

            // Non-retryable error
            tracing::error!("[LLM_API] API error ({}): {}", status, body);
            return Err(NoraError::LLMError(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        // All retries exhausted - try Ollama fallback if available
        if let (Some(endpoint), Some(model)) = (&self.fallback_endpoint, &self.fallback_model) {
            tracing::info!("[LLM_API] Primary provider failed, trying Ollama fallback: {}", model);
            match self.call_ollama_fallback(endpoint, model, &messages, tools).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::warn!("[LLM_API] Ollama fallback also failed: {}", e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| NoraError::LLMError("Max retries exceeded".to_string())))
    }

    /// Try Ollama/local LLM as fallback when primary provider fails
    async fn call_ollama_fallback(
        &self,
        endpoint: &str,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
    ) -> Result<LLMResponse> {
        let mut payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": self.config.temperature,
            "stream": false
        });

        // Add tools if provided (Ollama supports OpenAI-compatible tool calling)
        if !tools.is_empty() {
            payload["tools"] = serde_json::json!(tools);
            payload["tool_choice"] = serde_json::json!("auto");
        }

        tracing::info!("[LLM_API] Calling Ollama at {} with model {}", endpoint, model);

        let response = self
            .client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NoraError::LLMError(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NoraError::LLMError(format!("Ollama error ({}): {}", status, body)));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            NoraError::LLMError(format!("Failed to parse Ollama response: {}", e))
        })?;

        tracing::info!("[LLM_API] Ollama fallback succeeded");
        self.process_openai_tool_response(json).await
    }

    /// Helper to process OpenAI tool response JSON
    async fn process_openai_tool_response(&self, json: serde_json::Value) -> Result<LLMResponse> {
        tracing::debug!("[LLM_API] Response parsed successfully");
        eprintln!("[DEBUG] tool_calls in response: {:?}", json["choices"][0]["message"]["tool_calls"]);

        // Extract token usage from response
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                input_tokens: u["prompt_tokens"].as_u64()? as u32,
                output_tokens: u["completion_tokens"].as_u64()? as u32,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });

        if let Some(ref u) = usage {
            tracing::info!("[LLM_API] Token usage: {} input, {} output", u.input_tokens, u.output_tokens);
        }

        // Check if LLM wants to call tools
        if let Some(tool_calls) = json["choices"][0]["message"]["tool_calls"].as_array() {
            tracing::info!(
                "[LLM_API] LLM returned tool_calls array with {} items",
                tool_calls.len()
            );
            let calls: Vec<ToolCall> = tool_calls
                .iter()
                .filter_map(|tc| {
                    let id = tc["id"].as_str()?.to_string();
                    let name = tc["function"]["name"].as_str()?.to_string();
                    tracing::debug!("[LLM_API] Parsing tool call: id={}, name={}", id, name);
                    let arguments: serde_json::Value =
                        serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}"))
                            .unwrap_or(serde_json::json!({}));
                    tracing::debug!("[LLM_API] Tool arguments: {}", arguments);
                    Some(ToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect();

            if !calls.is_empty() {
                tracing::info!("[LLM_API] Returning {} tool calls to caller", calls.len());
                return Ok(LLMResponse::ToolCalls { calls, usage });
            }
        } else {
            tracing::debug!("[LLM_API] No tool_calls in response");
        }

        // No tool calls, return text content
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        tracing::info!(
            "[LLM_API] Returning text response ({} chars)",
            content.len()
        );
        Ok(LLMResponse::Text { content, usage })
    }

    async fn continue_openai_with_tool_results_and_history(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
        conversation_history: &[ConversationMessage],
        tools: &[serde_json::Value],
    ) -> Result<LLMResponse> {
        tracing::debug!("[LLM_API] continue_openai_with_tool_results_and_history called");
        tracing::debug!(
            "[LLM_API] Tool calls: {}, Tool results: {}, History: {} messages, Tools: {}",
            tool_calls.len(),
            tool_results.len(),
            conversation_history.len(),
            tools.len()
        );

        let endpoint = self.get_endpoint();

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.requires_auth() {
            return Err(NoraError::ConfigError(
                "LLM configured without required API key".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        // Build the conversation with history, tool call, and results
        let mut messages: Vec<serde_json::Value> =
            vec![serde_json::json!({ "role": "system", "content": system })];

        // Add conversation history (limit to last 10 messages)
        let history_limit = 10;
        let history_start = conversation_history.len().saturating_sub(history_limit);
        for msg in &conversation_history[history_start..] {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => continue,
            };
            messages.push(serde_json::json!({
                "role": role,
                "content": msg.content
            }));
        }

        // Add the current user request
        messages.push(serde_json::json!({
            "role": "user",
            "content": format!(
                "Context:\n{context}\n\nRequest:\n{user_query}",
                context = context,
                user_query = user_query
            )
        }));

        // Add assistant message with tool calls
        let assistant_tool_calls: Vec<serde_json::Value> = tool_calls
            .iter()
            .map(|tc| {
                serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments.to_string()
                    }
                })
            })
            .collect();

        messages.push(serde_json::json!({
            "role": "assistant",
            "tool_calls": assistant_tool_calls
        }));

        // Add tool results
        for result in tool_results {
            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": result.result
            }));
        }

        let mut payload = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "messages": messages
        });

        // Add tools so LLM can chain tool calls (e.g., get_project_details -> execute_workflow)
        if !tools.is_empty() {
            payload["tools"] = serde_json::json!(tools);
            payload["tool_choice"] = serde_json::json!("auto");
            tracing::debug!(
                "[LLM_API] Added {} tools to continuation payload with tool_choice=auto",
                tools.len()
            );
        }

        tracing::debug!(
            "[LLM_API] Constructed continuation payload with {} messages",
            messages.len()
        );
        tracing::info!("[LLM_API] Sending continuation request to OpenAI API...");

        // Retry logic with exponential backoff for 429/5xx errors
        let max_retries = 3;
        let mut last_error = String::new();
        let mut json: Option<serde_json::Value> = None;

        for attempt in 1..=max_retries {
            let client = self
                .client
                .post(&endpoint)
                .header("Content-Type", "application/json");

            let client = if let Some(ref auth) = auth_header {
                client.header("Authorization", auth.clone())
            } else {
                client
            };

            let response = match client.json(&payload).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("Failed to send request: {}", e);
                    tracing::error!("[LLM_API] Continuation request failed: {}", e);
                    continue;
                }
            };

            let status = response.status();
            if status.is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(j) => {
                        json = Some(j);
                        break;
                    }
                    Err(e) => {
                        last_error = format!("Failed to parse response: {}", e);
                        continue;
                    }
                }
            }

            let body = response.text().await.unwrap_or_default();
            last_error = format!("OpenAI API error ({}): {}", status, body);

            // Retry on 429 or 5xx
            if status.as_u16() == 429 || status.is_server_error() {
                if attempt < max_retries {
                    let delay = 1000 * (1 << (attempt - 1)); // 1s, 2s, 4s
                    tracing::warn!(
                        "[LLM_API] Continuation got {}, retrying in {}ms (attempt {}/{})",
                        status, delay, attempt + 1, max_retries
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
            } else {
                break;
            }
        }

        // If OpenAI failed, try Ollama fallback
        if json.is_none() {
            if let Some(ref fallback_endpoint) = self.fallback_endpoint {
                if let Some(ref fallback_model) = self.fallback_model {
                    tracing::info!(
                        "[LLM_API] Primary provider failed for continuation, trying Ollama fallback: {}",
                        fallback_model
                    );

                    let mut fallback_payload = payload.clone();
                    fallback_payload["model"] = serde_json::json!(fallback_model);

                    match self.client
                        .post(fallback_endpoint)
                        .header("Content-Type", "application/json")
                        .json(&fallback_payload)
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(j) = resp.json::<serde_json::Value>().await {
                                tracing::info!("[LLM_API] Ollama fallback succeeded for continuation");
                                json = Some(j);
                            }
                        }
                        Ok(resp) => {
                            let body = resp.text().await.unwrap_or_default();
                            tracing::error!("[LLM_API] Ollama fallback failed: {}", body);
                        }
                        Err(e) => {
                            tracing::error!("[LLM_API] Ollama fallback request failed: {}", e);
                        }
                    }
                }
            }
        }

        let json = json.ok_or_else(|| {
            tracing::error!("[LLM_API] All continuation attempts failed: {}", last_error);
            NoraError::LLMError(last_error)
        })?;

        // Extract token usage
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                input_tokens: u["prompt_tokens"].as_u64()? as u32,
                output_tokens: u["completion_tokens"].as_u64()? as u32,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });

        let message = &json["choices"][0]["message"];

        // Check if LLM wants to make more tool calls (chaining)
        if let Some(new_tool_calls) = message["tool_calls"].as_array() {
            if !new_tool_calls.is_empty() {
                tracing::info!(
                    "[LLM_API] Continuation returned {} more tool calls (chaining)",
                    new_tool_calls.len()
                );

                let parsed_calls: Vec<ToolCall> = new_tool_calls
                    .iter()
                    .filter_map(|tc| {
                        let id = tc["id"].as_str()?.to_string();
                        let name = tc["function"]["name"].as_str()?.to_string();
                        let arguments: serde_json::Value =
                            serde_json::from_str(tc["function"]["arguments"].as_str()?)
                                .unwrap_or(serde_json::Value::Null);
                        Some(ToolCall {
                            id,
                            name,
                            arguments,
                        })
                    })
                    .collect();

                for tc in &parsed_calls {
                    tracing::debug!("[LLM_API] Chained tool call: {} ({})", tc.name, tc.id);
                }

                return Ok(LLMResponse::ToolCalls { calls: parsed_calls, usage });
            }
        }

        // No tool calls - return the text content
        let content = message["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        tracing::info!(
            "[LLM_API] Continuation response received ({} chars)",
            content.len()
        );
        tracing::debug!(
            "[LLM_API] Final content: {}",
            &content[..content.len().min(200)]
        );

        Ok(LLMResponse::Text { content, usage })
    }

    #[allow(dead_code)]
    async fn generate_openai_with_tools(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tools: &[serde_json::Value],
    ) -> Result<LLMResponse> {
        tracing::debug!("[LLM_API] generate_openai_with_tools called");
        tracing::debug!(
            "[LLM_API] Model: {}, Tools count: {}",
            self.config.model,
            tools.len()
        );

        let endpoint = self.get_endpoint();

        tracing::debug!("[LLM_API] Endpoint: {}", endpoint);

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.requires_auth() {
            tracing::error!("[LLM_API] No API key configured for provider that requires auth");
            return Err(NoraError::ConfigError(
                "LLM configured without required API key".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        let mut payload = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "messages": [
                { "role": "system", "content": system },
                {
                    "role": "user",
                    "content": format!(
                        "Context:\n{context}\n\nRequest:\n{user_query}",
                        context = context,
                        user_query = user_query
                    )
                }
            ]
        });

        // Add tools if provided
        if !tools.is_empty() {
            payload["tools"] = serde_json::json!(tools);
            payload["tool_choice"] = serde_json::json!("required");
            tracing::debug!(
                "[LLM_API] Added {} tools to payload with tool_choice=auto",
                tools.len()
            );
        }

        tracing::info!("[LLM_API] Sending request to OpenAI API...");

        let client = self
            .client
            .post(&endpoint)
            .header("Content-Type", "application/json");

        let client = if let Some(auth) = auth_header {
            client.header("Authorization", auth)
        } else {
            client
        };

        let response = client.json(&payload).send().await.map_err(|e| {
            tracing::error!("[LLM_API] Request failed: {}", e);
            NoraError::LLMError(format!("Failed to send request: {}", e))
        })?;

        tracing::debug!("[LLM_API] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("[LLM_API] API error ({}): {}", status, body);
            return Err(NoraError::LLMError(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!("[LLM_API] Failed to parse response: {}", e);
            NoraError::LLMError(format!("Failed to parse response: {}", e))
        })?;

        tracing::debug!("[LLM_API] Response parsed successfully");
        eprintln!("[DEBUG] tool_calls in response: {:?}", json["choices"][0]["message"]["tool_calls"]);

        // Extract token usage
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                input_tokens: u["prompt_tokens"].as_u64()? as u32,
                output_tokens: u["completion_tokens"].as_u64()? as u32,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });

        // Check if LLM wants to call tools
        if let Some(tool_calls) = json["choices"][0]["message"]["tool_calls"].as_array() {
            tracing::info!(
                "[LLM_API] LLM returned tool_calls array with {} items",
                tool_calls.len()
            );
            let calls: Vec<ToolCall> = tool_calls
                .iter()
                .filter_map(|tc| {
                    let id = tc["id"].as_str()?.to_string();
                    let name = tc["function"]["name"].as_str()?.to_string();
                    tracing::debug!("[LLM_API] Parsing tool call: id={}, name={}", id, name);
                    let arguments: serde_json::Value =
                        serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}"))
                            .unwrap_or(serde_json::json!({}));
                    tracing::debug!("[LLM_API] Tool arguments: {}", arguments);
                    Some(ToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect();

            if !calls.is_empty() {
                tracing::info!("[LLM_API] Returning {} tool calls to caller", calls.len());
                return Ok(LLMResponse::ToolCalls { calls, usage });
            }
        } else {
            tracing::debug!("[LLM_API] No tool_calls in response");
        }

        // No tool calls, return text content
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        tracing::info!(
            "[LLM_API] Returning text response ({} chars)",
            content.len()
        );
        Ok(LLMResponse::Text { content, usage })
    }

    #[allow(dead_code)]
    async fn continue_openai_with_tool_results(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
    ) -> Result<String> {
        tracing::debug!("[LLM_API] continue_openai_with_tool_results called");
        tracing::debug!(
            "[LLM_API] Tool calls: {}, Tool results: {}",
            tool_calls.len(),
            tool_results.len()
        );

        let endpoint = self.get_endpoint();

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.requires_auth() {
            return Err(NoraError::ConfigError(
                "LLM configured without required API key".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        // Build the conversation with tool call and results
        let mut messages = vec![
            serde_json::json!({ "role": "system", "content": system }),
            serde_json::json!({
                "role": "user",
                "content": format!(
                    "Context:\n{context}\n\nRequest:\n{user_query}",
                    context = context,
                    user_query = user_query
                )
            }),
        ];

        // Add assistant message with tool calls
        let assistant_tool_calls: Vec<serde_json::Value> = tool_calls
            .iter()
            .map(|tc| {
                serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments.to_string()
                    }
                })
            })
            .collect();

        messages.push(serde_json::json!({
            "role": "assistant",
            "tool_calls": assistant_tool_calls
        }));

        // Add tool results
        for result in tool_results {
            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": result.result
            }));
        }

        let payload = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "messages": messages
        });

        tracing::debug!(
            "[LLM_API] Constructed continuation payload with {} messages",
            messages.len()
        );
        tracing::info!("[LLM_API] Sending continuation request to OpenAI API...");

        let client = self
            .client
            .post(&endpoint)
            .header("Content-Type", "application/json");

        let client = if let Some(auth) = auth_header {
            client.header("Authorization", auth)
        } else {
            client
        };

        let response = client.json(&payload).send().await.map_err(|e| {
            tracing::error!("[LLM_API] Continuation request failed: {}", e);
            NoraError::LLMError(format!("Failed to send request: {}", e))
        })?;

        tracing::debug!(
            "[LLM_API] Continuation response status: {}",
            response.status()
        );

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("[LLM_API] Continuation API error ({}): {}", status, body);
            return Err(NoraError::LLMError(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!("[LLM_API] Failed to parse continuation response: {}", e);
            NoraError::LLMError(format!("Failed to parse response: {}", e))
        })?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        tracing::info!(
            "[LLM_API] Continuation response received ({} chars)",
            content.len()
        );
        tracing::debug!(
            "[LLM_API] Final content: {}",
            &content[..content.len().min(200)]
        );

        Ok(content)
    }

    async fn generate_openai(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<String> {
        let endpoint = self.get_endpoint();

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.requires_auth() {
            return Err(NoraError::ConfigError(
                "LLM configured without required API key".to_string(),
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

        let endpoint = self.get_endpoint();

        let auth_header = self.api_key.as_ref().map(|k| format!("Bearer {}", k));

        if auth_header.is_none() && self.requires_auth() {
            return Err(NoraError::ConfigError(
                "LLM configured without required API key".to_string(),
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
            let chunk = chunk_result
                .map_err(|e| NoraError::LLMError(format!("Stream chunk error: {}", e)))?;

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
                        if let Some(delta_content) = json["choices"][0]["delta"]["content"].as_str()
                        {
                            content_chunks.push(delta_content.to_string());
                        }
                    }
                }
            }

            Ok(content_chunks.join(""))
        });

        Ok(Box::pin(text_stream))
    }

    // ============================================================================
    // ANTHROPIC CLAUDE IMPLEMENTATION METHODS
    // ============================================================================

    async fn generate_anthropic(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<String> {
        use providers::{ChatConfig, ChatMessage, ChatRequest};

        let provider = AnthropicProvider::new();
        if !provider.is_configured() {
            return Err(NoraError::ConfigError(
                "Anthropic provider not configured - ANTHROPIC_API_KEY not found".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        let user_content = format!(
            "Context:\n{context}\n\nRequest:\n{user_query}",
            context = context,
            user_query = user_query
        );

        let request = ChatRequest {
            messages: vec![
                ChatMessage::system(system),
                ChatMessage::user(user_content),
            ],
            tools: None,
            config: ChatConfig {
                model: self.config.model.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                tool_choice: None,
            },
        };

        let response = provider.chat(request).await.map_err(|e| {
            NoraError::LLMError(format!("Anthropic API error: {}", e))
        })?;

        match response {
            providers::ProviderResponse::Text { content, .. } => Ok(content),
            providers::ProviderResponse::ToolCalls { .. } => {
                Err(NoraError::LLMError("Unexpected tool calls in simple generate".to_string()))
            }
        }
    }

    async fn generate_anthropic_stream(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        use futures::StreamExt;
        use providers::{ChatConfig, ChatMessage, ChatRequest};

        let provider = AnthropicProvider::new();
        if !provider.is_configured() {
            return Err(NoraError::ConfigError(
                "Anthropic provider not configured - ANTHROPIC_API_KEY not found".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        let user_content = format!(
            "Context:\n{context}\n\nRequest:\n{user_query}",
            context = context,
            user_query = user_query
        );

        let request = ChatRequest {
            messages: vec![
                ChatMessage::system(system),
                ChatMessage::user(user_content),
            ],
            tools: None,
            config: ChatConfig {
                model: self.config.model.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                tool_choice: None,
            },
        };

        let stream = provider.chat_stream(request).await.map_err(|e| {
            NoraError::LLMError(format!("Anthropic streaming error: {}", e))
        })?;

        // Convert provider stream to the expected format
        let mapped_stream = stream.map(|result| {
            result
                .map(|chunk| chunk.content)
                .map_err(|e| NoraError::LLMError(format!("Stream chunk error: {}", e)))
        });

        Ok(Box::pin(mapped_stream))
    }

    async fn generate_anthropic_with_tools_and_history(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tools: &[serde_json::Value],
        conversation_history: &[ConversationMessage],
    ) -> Result<LLMResponse> {
        use providers::{ChatConfig, ChatMessage, ChatRequest, ToolDefinition};

        let provider = AnthropicProvider::new();
        if !provider.is_configured() {
            return Err(NoraError::ConfigError(
                "Anthropic provider not configured - ANTHROPIC_API_KEY not found".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        // Build messages
        let mut messages = vec![ChatMessage::system(system)];

        // Add conversation history (limit to last 10 messages)
        let history_limit = 10;
        let history_start = conversation_history.len().saturating_sub(history_limit);
        for msg in &conversation_history[history_start..] {
            match msg.role {
                MessageRole::User => messages.push(ChatMessage::user(&msg.content)),
                MessageRole::Assistant => messages.push(ChatMessage::assistant(&msg.content)),
                MessageRole::System => {} // Skip system messages in history
            }
        }

        // Add current request with context
        let user_content = format!(
            "Context:\n{context}\n\nRequest:\n{user_query}",
            context = context,
            user_query = user_query
        );
        messages.push(ChatMessage::user(user_content));

        // Convert tools to provider format
        let provider_tools: Option<Vec<ToolDefinition>> = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .filter_map(|t| {
                        let func = t.get("function")?;
                        Some(ToolDefinition {
                            name: func["name"].as_str()?.to_string(),
                            description: func["description"].as_str().unwrap_or("").to_string(),
                            parameters: func["parameters"].clone(),
                        })
                    })
                    .collect(),
            )
        };

        let request = ChatRequest {
            messages,
            tools: provider_tools,
            config: ChatConfig {
                model: self.config.model.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                tool_choice: if tools.is_empty() { None } else { Some("auto".to_string()) },
            },
        };

        let response = provider.chat(request).await.map_err(|e| {
            NoraError::LLMError(format!("Anthropic API error: {}", e))
        })?;

        // Convert provider response to LLMResponse, preserving usage
        match response {
            providers::ProviderResponse::Text { content, usage } => {
                Ok(LLMResponse::Text { content, usage })
            }
            providers::ProviderResponse::ToolCalls { calls, usage } => {
                let tool_calls: Vec<ToolCall> = calls
                    .into_iter()
                    .map(|c| ToolCall {
                        id: c.id,
                        name: c.name,
                        arguments: c.arguments,
                    })
                    .collect();
                Ok(LLMResponse::ToolCalls { calls: tool_calls, usage })
            }
        }
    }

    async fn continue_anthropic_with_tool_results_and_history(
        &self,
        system_prompt: &str,
        user_query: &str,
        context: &str,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
        conversation_history: &[ConversationMessage],
        tools: &[serde_json::Value],
    ) -> Result<LLMResponse> {
        use providers::{ChatConfig, ChatMessage, ChatRequest, ToolCallRequest, ToolDefinition};

        let provider = AnthropicProvider::new();
        if !provider.is_configured() {
            return Err(NoraError::ConfigError(
                "Anthropic provider not configured - ANTHROPIC_API_KEY not found".to_string(),
            ));
        }

        let system = if system_prompt.is_empty() {
            self.config.system_prompt.as_str()
        } else {
            system_prompt
        };

        // Build messages
        let mut messages = vec![ChatMessage::system(system)];

        // Add conversation history
        let history_limit = 10;
        let history_start = conversation_history.len().saturating_sub(history_limit);
        for msg in &conversation_history[history_start..] {
            match msg.role {
                MessageRole::User => messages.push(ChatMessage::user(&msg.content)),
                MessageRole::Assistant => messages.push(ChatMessage::assistant(&msg.content)),
                MessageRole::System => {}
            }
        }

        // Add the original user request
        let user_content = format!(
            "Context:\n{context}\n\nRequest:\n{user_query}",
            context = context,
            user_query = user_query
        );
        messages.push(ChatMessage::user(user_content));

        // Add assistant message with tool calls
        let provider_tool_calls: Vec<ToolCallRequest> = tool_calls
            .iter()
            .map(|tc| ToolCallRequest {
                id: tc.id.clone(),
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            })
            .collect();
        messages.push(ChatMessage::assistant_with_tools(provider_tool_calls));

        // Add tool results
        for result in tool_results {
            messages.push(ChatMessage::tool_result(&result.tool_call_id, &result.result));
        }

        // Convert tools to provider format
        let provider_tools: Option<Vec<ToolDefinition>> = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .filter_map(|t| {
                        let func = t.get("function")?;
                        Some(ToolDefinition {
                            name: func["name"].as_str()?.to_string(),
                            description: func["description"].as_str().unwrap_or("").to_string(),
                            parameters: func["parameters"].clone(),
                        })
                    })
                    .collect(),
            )
        };

        let request = ChatRequest {
            messages,
            tools: provider_tools,
            config: ChatConfig {
                model: self.config.model.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                tool_choice: if tools.is_empty() { None } else { Some("auto".to_string()) },
            },
        };

        let response = provider.chat(request).await.map_err(|e| {
            NoraError::LLMError(format!("Anthropic API error: {}", e))
        })?;

        // Convert provider response to LLMResponse, preserving usage
        match response {
            providers::ProviderResponse::Text { content, usage } => {
                Ok(LLMResponse::Text { content, usage })
            }
            providers::ProviderResponse::ToolCalls { calls, usage } => {
                let tool_calls: Vec<ToolCall> = calls
                    .into_iter()
                    .map(|c| ToolCall {
                        id: c.id,
                        name: c.name,
                        arguments: c.arguments,
                    })
                    .collect();
                Ok(LLMResponse::ToolCalls { calls: tool_calls, usage })
            }
        }
    }
}

//! Anthropic Claude provider implementation

use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;

use super::provider_trait::{
    ChatConfig, ChatMessage, ChatRequest, ContentBlock, LLMProviderTrait, MessageRole,
    ProviderError, ProviderResponse, ProviderType, StreamChunk, TokenUsage, ToolCallRequest,
    ToolDefinition,
};

/// Anthropic Claude API provider
pub struct AnthropicProvider {
    client: Client,
    api_key: Option<String>,
    endpoint: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        if api_key.is_some() {
            tracing::info!("Anthropic provider initialized with API key");
        } else {
            tracing::warn!(
                "Anthropic provider created without API key - ANTHROPIC_API_KEY env var not found"
            );
        }

        Self {
            client: Client::new(),
            api_key,
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
        }
    }

    /// Create with a custom endpoint
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        let mut provider = Self::new();
        provider.endpoint = endpoint.into();
        provider
    }

    /// Convert our ChatMessage to Anthropic API format
    /// Note: Anthropic uses a different format - system is separate, and tool results are special
    fn messages_to_anthropic(
        &self,
        messages: &[ChatMessage],
    ) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system_prompt: Option<String> = None;
        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    // Anthropic takes system as a separate parameter
                    if system_prompt.is_none() {
                        system_prompt = Some(msg.content.clone());
                    } else {
                        // Append additional system messages
                        if let Some(ref mut s) = system_prompt {
                            s.push_str("\n\n");
                            s.push_str(&msg.content);
                        }
                    }
                }
                MessageRole::User => {
                    if let Some(ref blocks) = msg.content_blocks {
                        // Multi-modal: serialize content blocks in Anthropic format
                        let content_array: Vec<serde_json::Value> = blocks
                            .iter()
                            .map(|block| match block {
                                ContentBlock::Text { text } => {
                                    serde_json::json!({ "type": "text", "text": text })
                                }
                                ContentBlock::ImageBase64 { media_type, data } => {
                                    serde_json::json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": media_type,
                                            "data": data
                                        }
                                    })
                                }
                            })
                            .collect();
                        api_messages.push(serde_json::json!({
                            "role": "user",
                            "content": content_array
                        }));
                    } else {
                        api_messages.push(serde_json::json!({
                            "role": "user",
                            "content": msg.content
                        }));
                    }
                }
                MessageRole::Assistant => {
                    if let Some(ref tool_calls) = msg.tool_calls {
                        // Assistant message with tool use
                        let tool_use_blocks: Vec<serde_json::Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                serde_json::json!({
                                    "type": "tool_use",
                                    "id": tc.id,
                                    "name": tc.name,
                                    "input": tc.arguments
                                })
                            })
                            .collect();

                        api_messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": tool_use_blocks
                        }));
                    } else {
                        api_messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": msg.content
                        }));
                    }
                }
                MessageRole::Tool => {
                    // Tool results in Anthropic format
                    if let Some(ref tool_call_id) = msg.tool_call_id {
                        api_messages.push(serde_json::json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": msg.content
                            }]
                        }));
                    }
                }
            }
        }

        (system_prompt, api_messages)
    }

    /// Convert ToolDefinition to Anthropic tool format
    fn tool_to_anthropic(&self, tool: &ToolDefinition) -> serde_json::Value {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.parameters
        })
    }

    /// Parse Anthropic response into ProviderResponse
    fn parse_response(&self, json: &serde_json::Value) -> Result<ProviderResponse, ProviderError> {
        // Extract usage
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                input_tokens: u["input_tokens"].as_u64()? as u32,
                output_tokens: u["output_tokens"].as_u64()? as u32,
                total_tokens: (u["input_tokens"].as_u64()? + u["output_tokens"].as_u64()?) as u32,
            })
        });

        // Check stop reason for tool use
        let stop_reason = json["stop_reason"].as_str().unwrap_or("");

        // Parse content blocks
        let content_blocks = json["content"].as_array();

        if stop_reason == "tool_use" {
            // Extract tool calls from content blocks
            if let Some(blocks) = content_blocks {
                let calls: Vec<ToolCallRequest> = blocks
                    .iter()
                    .filter_map(|block| {
                        if block["type"].as_str()? == "tool_use" {
                            Some(ToolCallRequest {
                                id: block["id"].as_str()?.to_string(),
                                name: block["name"].as_str()?.to_string(),
                                arguments: block["input"].clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                if !calls.is_empty() {
                    return Ok(ProviderResponse::ToolCalls { calls, usage });
                }
            }
        }

        // Text response - extract from content blocks
        let content = if let Some(blocks) = content_blocks {
            blocks
                .iter()
                .filter_map(|block| {
                    if block["type"].as_str()? == "text" {
                        block["text"].as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .join("")
        } else {
            String::new()
        };

        Ok(ProviderResponse::Text { content, usage })
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProviderTrait for AnthropicProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    fn name(&self) -> &'static str {
        "Anthropic"
    }

    fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn default_model(&self) -> &str {
        "claude-sonnet-4-20250514"
    }

    fn validate_model(&self, model: &str) -> bool {
        // Current Claude models
        matches!(
            model,
            "claude-sonnet-4-20250514"
                | "claude-opus-4-20250514"
                | "claude-3-5-sonnet-20241022"
                | "claude-3-5-haiku-20241022"
                | "claude-3-opus-20240229"
                | "claude-3-sonnet-20240229"
                | "claude-3-haiku-20240307"
        ) || model.starts_with("claude-")
    }

    async fn chat(&self, request: ChatRequest) -> Result<ProviderResponse, ProviderError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| ProviderError::AuthError("No Anthropic API key configured".to_string()))?;

        // Convert messages
        let (system_prompt, messages) = self.messages_to_anthropic(&request.messages);

        // Ensure we have at least one message
        if messages.is_empty() {
            return Err(ProviderError::ConfigError(
                "At least one non-system message is required".to_string(),
            ));
        }

        // Build payload
        let mut payload = serde_json::json!({
            "model": request.config.model,
            "max_tokens": request.config.max_tokens,
            "messages": messages
        });

        // Add system prompt if present
        if let Some(system) = system_prompt {
            payload["system"] = serde_json::json!(system);
        }

        // Add temperature (Anthropic accepts 0.0 to 1.0)
        // Note: Anthropic default is 1.0, so we set it explicitly
        payload["temperature"] = serde_json::json!(request.config.temperature);

        // Add tools if provided
        if let Some(ref tools) = request.tools {
            if !tools.is_empty() {
                let anthropic_tools: Vec<serde_json::Value> =
                    tools.iter().map(|t| self.tool_to_anthropic(t)).collect();
                payload["tools"] = serde_json::json!(anthropic_tools);

                // Set tool_choice if specified
                if let Some(ref choice) = request.config.tool_choice {
                    payload["tool_choice"] = match choice.as_str() {
                        "auto" => serde_json::json!({"type": "auto"}),
                        "required" | "any" => serde_json::json!({"type": "any"}),
                        "none" => serde_json::json!({"type": "none"}), // Note: Anthropic doesn't have "none", we just don't send tools
                        specific => serde_json::json!({"type": "tool", "name": specific}),
                    };
                }
            }
        }

        tracing::debug!(
            "[Anthropic] Sending request: model={}, messages={}, tools={}",
            request.config.model,
            messages.len(),
            request.tools.as_ref().map(|t| t.len()).unwrap_or(0)
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            // Check for rate limiting
            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimited {
                    retry_after_ms: None,
                });
            }

            // Check for overloaded
            if status.as_u16() == 529 {
                return Err(ProviderError::NotAvailable(
                    "Anthropic API is temporarily overloaded".to_string(),
                ));
            }

            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        self.parse_response(&json)
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>, ProviderError>
    {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| ProviderError::AuthError("No Anthropic API key configured".to_string()))?
            .clone();

        // Convert messages
        let (system_prompt, messages) = self.messages_to_anthropic(&request.messages);

        // Build payload with stream enabled
        let mut payload = serde_json::json!({
            "model": request.config.model,
            "max_tokens": request.config.max_tokens,
            "messages": messages,
            "stream": true
        });

        if let Some(system) = system_prompt {
            payload["system"] = serde_json::json!(system);
        }

        payload["temperature"] = serde_json::json!(request.config.temperature);

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        // Convert response to a stream of chunks
        let byte_stream = response.bytes_stream();

        let stream = byte_stream.filter_map(|result| async move {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();
                    let mut is_done = false;

                    // Parse Anthropic SSE format
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                let event_type = json["type"].as_str().unwrap_or("");

                                match event_type {
                                    "content_block_delta" => {
                                        if let Some(delta) = json["delta"]["text"].as_str() {
                                            content.push_str(delta);
                                        }
                                    }
                                    "message_stop" => {
                                        is_done = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    if content.is_empty() && !is_done {
                        None
                    } else {
                        Some(Ok(StreamChunk { content, is_done }))
                    }
                }
                Err(e) => Some(Err(ProviderError::RequestFailed(e.to_string()))),
            }
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_validation() {
        let provider = AnthropicProvider::new();
        assert!(provider.validate_model("claude-sonnet-4-20250514"));
        assert!(provider.validate_model("claude-3-5-sonnet-20241022"));
        assert!(provider.validate_model("claude-3-opus-20240229"));
        assert!(!provider.validate_model("gpt-4o"));
    }

    #[test]
    fn test_message_conversion() {
        let provider = AnthropicProvider::new();

        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let (system, api_messages) = provider.messages_to_anthropic(&messages);

        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(api_messages.len(), 2); // System is extracted, user + assistant remain
        assert_eq!(api_messages[0]["role"], "user");
        assert_eq!(api_messages[1]["role"], "assistant");
    }

    #[test]
    fn test_vision_content_blocks_serialization() {
        let provider = AnthropicProvider::new();

        let msg = ChatMessage::user_with_images(
            "Analyze this frame",
            vec![("dGVzdA==".to_string(), "image/jpeg".to_string())],
        );
        let messages = vec![msg];
        let (_, api_messages) = provider.messages_to_anthropic(&messages);

        assert_eq!(api_messages.len(), 1);
        let content = &api_messages[0]["content"];
        assert!(content.is_array());
        let blocks = content.as_array().unwrap();
        assert_eq!(blocks.len(), 2); // 1 image + 1 text

        // First block should be the image
        assert_eq!(blocks[0]["type"], "image");
        assert_eq!(blocks[0]["source"]["type"], "base64");
        assert_eq!(blocks[0]["source"]["media_type"], "image/jpeg");
        assert_eq!(blocks[0]["source"]["data"], "dGVzdA==");

        // Second block should be the text
        assert_eq!(blocks[1]["type"], "text");
        assert_eq!(blocks[1]["text"], "Analyze this frame");
    }

    #[test]
    fn test_plain_text_message_no_content_blocks() {
        let provider = AnthropicProvider::new();

        let msg = ChatMessage::user("Just text");
        let messages = vec![msg];
        let (_, api_messages) = provider.messages_to_anthropic(&messages);

        // Plain text should be a string, not an array
        assert_eq!(api_messages[0]["content"], "Just text");
    }
}

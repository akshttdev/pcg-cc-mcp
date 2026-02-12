//! OpenAI provider implementation

use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;

use super::provider_trait::{
    ChatConfig, ChatMessage, ChatRequest, ContentBlock, LLMProviderTrait, MessageRole,
    ProviderError, ProviderResponse, ProviderType, StreamChunk, TokenUsage, ToolCallRequest,
    ToolDefinition,
};

/// OpenAI API provider
pub struct OpenAIProvider {
    client: Client,
    api_key: Option<String>,
    endpoint: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").ok();

        if api_key.is_some() {
            tracing::info!("OpenAI provider initialized with API key");
        } else {
            tracing::warn!("OpenAI provider created without API key - OPENAI_API_KEY env var not found");
        }

        Self {
            client: Client::new(),
            api_key,
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
        }
    }

    /// Create with a custom endpoint (e.g., for Azure OpenAI or local proxies)
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        let mut provider = Self::new();
        provider.endpoint = endpoint.into();
        provider
    }

    /// Convert our ChatMessage to OpenAI API format
    fn message_to_openai(&self, msg: &ChatMessage) -> serde_json::Value {
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };

        // Multi-modal: serialize content blocks in OpenAI format (image_url with data URI)
        let content_value = if let Some(ref blocks) = msg.content_blocks {
            let content_array: Vec<serde_json::Value> = blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => {
                        serde_json::json!({ "type": "text", "text": text })
                    }
                    ContentBlock::ImageBase64 { media_type, data } => {
                        serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", media_type, data)
                            }
                        })
                    }
                })
                .collect();
            serde_json::json!(content_array)
        } else {
            serde_json::json!(msg.content)
        };

        let mut obj = serde_json::json!({
            "role": role,
            "content": content_value
        });

        // Add tool_call_id for tool responses
        if let Some(ref tool_call_id) = msg.tool_call_id {
            obj["tool_call_id"] = serde_json::json!(tool_call_id);
        }

        // Add tool_calls for assistant messages that called tools
        if let Some(ref tool_calls) = msg.tool_calls {
            let calls: Vec<serde_json::Value> = tool_calls
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
            obj["tool_calls"] = serde_json::json!(calls);
            // When there are tool_calls, content might be null
            if msg.content.is_empty() {
                obj["content"] = serde_json::Value::Null;
            }
        }

        obj
    }

    /// Convert ToolDefinition to OpenAI tool format
    fn tool_to_openai(&self, tool: &ToolDefinition) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters
            }
        })
    }

    /// Parse OpenAI response into ProviderResponse
    fn parse_response(&self, json: &serde_json::Value) -> Result<ProviderResponse, ProviderError> {
        let message = &json["choices"][0]["message"];

        // Extract usage if present
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                input_tokens: u["prompt_tokens"].as_u64()? as u32,
                output_tokens: u["completion_tokens"].as_u64()? as u32,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });

        // Check for tool calls
        if let Some(tool_calls) = message["tool_calls"].as_array() {
            if !tool_calls.is_empty() {
                let calls: Vec<ToolCallRequest> = tool_calls
                    .iter()
                    .filter_map(|tc| {
                        let id = tc["id"].as_str()?.to_string();
                        let name = tc["function"]["name"].as_str()?.to_string();
                        let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                        let arguments: serde_json::Value =
                            serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));

                        Some(ToolCallRequest {
                            id,
                            name,
                            arguments,
                        })
                    })
                    .collect();

                if !calls.is_empty() {
                    return Ok(ProviderResponse::ToolCalls { calls, usage });
                }
            }
        }

        // Text response
        let content = message["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(ProviderResponse::Text { content, usage })
    }
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProviderTrait for OpenAIProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn default_model(&self) -> &str {
        "gpt-4o"
    }

    fn validate_model(&self, model: &str) -> bool {
        // Common OpenAI models
        matches!(
            model,
            "gpt-4o"
                | "gpt-4o-mini"
                | "gpt-4-turbo"
                | "gpt-4-turbo-preview"
                | "gpt-4"
                | "gpt-3.5-turbo"
                | "o1"
                | "o1-mini"
                | "o1-preview"
        ) || model.starts_with("gpt-")
            || model.starts_with("o1")
    }

    async fn chat(&self, request: ChatRequest) -> Result<ProviderResponse, ProviderError> {
        let auth_header = self
            .api_key
            .as_ref()
            .map(|k| format!("Bearer {}", k))
            .ok_or_else(|| ProviderError::AuthError("No OpenAI API key configured".to_string()))?;

        // Build messages array
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| self.message_to_openai(m))
            .collect();

        // Build payload
        let mut payload = serde_json::json!({
            "model": request.config.model,
            "temperature": request.config.temperature,
            "max_tokens": request.config.max_tokens,
            "messages": messages
        });

        // Add tools if provided
        if let Some(ref tools) = request.tools {
            if !tools.is_empty() {
                let openai_tools: Vec<serde_json::Value> =
                    tools.iter().map(|t| self.tool_to_openai(t)).collect();
                payload["tools"] = serde_json::json!(openai_tools);

                // Set tool_choice
                if let Some(ref choice) = request.config.tool_choice {
                    payload["tool_choice"] = match choice.as_str() {
                        "auto" => serde_json::json!("auto"),
                        "required" => serde_json::json!("required"),
                        "none" => serde_json::json!("none"),
                        specific => serde_json::json!({"type": "function", "function": {"name": specific}}),
                    };
                } else {
                    payload["tool_choice"] = serde_json::json!("auto");
                }
            }
        }

        tracing::debug!(
            "[OpenAI] Sending request: model={}, messages={}, tools={}",
            request.config.model,
            messages.len(),
            request.tools.as_ref().map(|t| t.len()).unwrap_or(0)
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", auth_header)
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
        let auth_header = self
            .api_key
            .as_ref()
            .map(|k| format!("Bearer {}", k))
            .ok_or_else(|| ProviderError::AuthError("No OpenAI API key configured".to_string()))?;

        // Build messages array
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| self.message_to_openai(m))
            .collect();

        // Build payload with stream enabled
        let payload = serde_json::json!({
            "model": request.config.model,
            "temperature": request.config.temperature,
            "max_tokens": request.config.max_tokens,
            "messages": messages,
            "stream": true
        });

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", auth_header)
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
                    // Parse SSE format: data: {...}
                    let mut content = String::new();
                    let mut is_done = false;

                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                is_done = true;
                                continue;
                            }

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str()
                                {
                                    content.push_str(delta);
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
    fn test_message_conversion() {
        let provider = OpenAIProvider::new();

        let msg = ChatMessage::user("Hello");
        let json = provider.message_to_openai(&msg);
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello");

        let msg = ChatMessage::system("You are helpful");
        let json = provider.message_to_openai(&msg);
        assert_eq!(json["role"], "system");
    }

    #[test]
    fn test_model_validation() {
        let provider = OpenAIProvider::new();
        assert!(provider.validate_model("gpt-4o"));
        assert!(provider.validate_model("gpt-4o-mini"));
        assert!(provider.validate_model("gpt-4-turbo"));
        assert!(!provider.validate_model("claude-3-opus"));
    }

    #[test]
    fn test_vision_content_blocks_serialization() {
        let provider = OpenAIProvider::new();

        let msg = ChatMessage::user_with_images(
            "Score this frame",
            vec![("dGVzdA==".to_string(), "image/jpeg".to_string())],
        );
        let json = provider.message_to_openai(&msg);

        assert_eq!(json["role"], "user");
        let content = &json["content"];
        assert!(content.is_array());
        let blocks = content.as_array().unwrap();
        assert_eq!(blocks.len(), 2); // 1 image + 1 text

        // First block should be the image_url
        assert_eq!(blocks[0]["type"], "image_url");
        assert_eq!(
            blocks[0]["image_url"]["url"],
            "data:image/jpeg;base64,dGVzdA=="
        );

        // Second block should be the text
        assert_eq!(blocks[1]["type"], "text");
        assert_eq!(blocks[1]["text"], "Score this frame");
    }

    #[test]
    fn test_plain_text_message_no_content_blocks() {
        let provider = OpenAIProvider::new();

        let msg = ChatMessage::user("Just text");
        let json = provider.message_to_openai(&msg);

        // Plain text should be a simple string
        assert_eq!(json["content"], "Just text");
    }
}

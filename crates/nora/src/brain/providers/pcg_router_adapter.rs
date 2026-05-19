//! PCG Router adapter — routes Nora LLM calls through WorkflowLLMService
//!
//! This adapter implements `LLMProviderTrait` and forwards all calls to
//! `WorkflowLLMService`, providing database-driven provider selection,
//! unified cost tracking, and automatic fallback.

use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use futures::Stream;
use services::services::workflow_llm::{
    LLMResponse as RouterLLMResponse, LLMStreamResponse, RoutingMetadata,
    TokenUsage as RouterTokenUsage, ToolDefinition as RouterToolDefinition, WorkflowLLMService,
};
use sqlx::SqlitePool;

use super::provider_trait::{
    ChatMessage, ChatRequest, ContentBlock, LLMProviderTrait, MessageRole, ProviderError,
    ProviderResponse, ProviderType, StreamChunk, TokenUsage, ToolCallRequest, ToolDefinition,
};

/// Adapter that routes Nora brain LLM calls through PCG Router.
///
/// Uses `WorkflowLLMService` for all completions, gaining:
/// - Database-driven model selection
/// - Automatic provider fallback
/// - Unified cost tracking
/// - Support for all PCG Router providers (Anthropic, OpenAI, Gemini, etc.)
pub struct PcgRouterAdapter {
    pool: Arc<SqlitePool>,
    model_hint: Option<String>,
}

impl PcgRouterAdapter {
    /// Create a new adapter with default model selection.
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            pool,
            model_hint: None,
        }
    }

    /// Create an adapter with a specific model preference.
    ///
    /// The model hint is used to select a specific model from PCG Router.
    /// Falls back to priority list if the hinted model is unavailable.
    pub fn with_model(pool: Arc<SqlitePool>, model: impl Into<String>) -> Self {
        Self {
            pool,
            model_hint: Some(model.into()),
        }
    }

    /// Convert Nora ChatMessage format to WorkflowLLMService JSON format.
    fn convert_messages(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                };

                let mut json = serde_json::json!({
                    "role": role,
                    "content": msg.content,
                });

                // Handle tool call ID for tool result messages
                if let Some(ref tool_call_id) = msg.tool_call_id {
                    json["tool_call_id"] = serde_json::json!(tool_call_id);
                }

                // Handle assistant tool calls
                if let Some(ref tool_calls) = msg.tool_calls {
                    let tc_json: Vec<serde_json::Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default(),
                                }
                            })
                        })
                        .collect();
                    json["tool_calls"] = serde_json::json!(tc_json);
                }

                // Handle multi-modal content blocks
                if let Some(ref blocks) = msg.content_blocks {
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
                    json["content"] = serde_json::json!(content_array);
                }

                json
            })
            .collect()
    }

    /// Convert Nora tool definitions to WorkflowLLMService format.
    fn convert_tools(tools: &[ToolDefinition]) -> Vec<RouterToolDefinition> {
        tools
            .iter()
            .map(|t| RouterToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            })
            .collect()
    }

    /// Convert WorkflowLLMService response to Nora ProviderResponse.
    fn convert_response(
        response: RouterLLMResponse,
        metadata: &RoutingMetadata,
    ) -> ProviderResponse {
        match response {
            RouterLLMResponse::Text { content, usage } => {
                let converted_usage = Self::convert_usage(&usage, metadata);
                ProviderResponse::Text {
                    content,
                    usage: Some(converted_usage),
                }
            }
            RouterLLMResponse::ToolCalls { calls, usage } => {
                let converted_calls: Vec<ToolCallRequest> = calls
                    .into_iter()
                    .map(|c| ToolCallRequest {
                        id: c.id,
                        name: c.name,
                        arguments: c.arguments,
                    })
                    .collect();
                let converted_usage = Self::convert_usage(&usage, metadata);
                ProviderResponse::ToolCalls {
                    calls: converted_calls,
                    usage: Some(converted_usage),
                }
            }
        }
    }

    /// Convert WorkflowLLMService token usage to Nora format.
    fn convert_usage(usage: &RouterTokenUsage, metadata: &RoutingMetadata) -> TokenUsage {
        let input = usage.input_tokens.or(metadata.input_tokens).unwrap_or(0) as u32;
        let output = usage.output_tokens.or(metadata.output_tokens).unwrap_or(0) as u32;
        TokenUsage {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }
}

#[async_trait]
impl LLMProviderTrait for PcgRouterAdapter {
    fn provider_type(&self) -> ProviderType {
        // PCG Router is a meta-provider that can route to any backend
        // Return Anthropic as the default since that's the most common backend
        ProviderType::Anthropic
    }

    fn name(&self) -> &'static str {
        "PCG Router"
    }

    fn is_configured(&self) -> bool {
        // PCG Router is always configured if we have a database connection
        true
    }

    fn supports_tools(&self) -> bool {
        // PCG Router supports tools through WorkflowLLMService
        true
    }

    fn default_model(&self) -> &str {
        self.model_hint
            .as_deref()
            .unwrap_or("claude-sonnet-4-20250514")
    }

    fn validate_model(&self, _model: &str) -> bool {
        // PCG Router can validate models against the database, but for now
        // we accept any model and let the router handle fallback
        true
    }

    async fn chat(&self, request: ChatRequest) -> Result<ProviderResponse, ProviderError> {
        let messages = Self::convert_messages(&request.messages);
        let tools = request
            .tools
            .as_ref()
            .map(|t| Self::convert_tools(t))
            .unwrap_or_default();

        let (response, metadata) = WorkflowLLMService::completion_with_tools(
            &self.pool,
            messages,
            &tools,
            self.model_hint.as_deref(),
            Some(request.config.max_tokens as i64),
            Some(request.config.temperature as f64),
        )
        .await
        .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        tracing::info!(
            model = %metadata.model_used,
            provider = %metadata.provider,
            input_tokens = ?metadata.input_tokens,
            output_tokens = ?metadata.output_tokens,
            cost_micros = ?metadata.estimated_cost_micros,
            "[PCG_ROUTER] LLM call completed"
        );

        Ok(Self::convert_response(response, &metadata))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>, ProviderError>
    {
        let messages = Self::convert_messages(&request.messages);

        let stream_response = WorkflowLLMService::completion_stream(
            &self.pool,
            messages,
            self.model_hint.as_deref(),
            Some(request.config.max_tokens as i64),
            Some(request.config.temperature as f64),
        )
        .await
        .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        match stream_response {
            LLMStreamResponse::TextStream { stream, metadata } => {
                tracing::info!(
                    model = %metadata.model_used,
                    provider = %metadata.provider,
                    "[PCG_ROUTER] Streaming started"
                );

                // Convert the WorkflowLLMService stream to Nora's StreamChunk format
                use futures::StreamExt;
                let converted_stream = stream.map(|result| {
                    result
                        .map(|chunk| StreamChunk {
                            content: chunk.delta,
                            is_done: chunk.is_done,
                        })
                        .map_err(|e| ProviderError::RequestFailed(e.to_string()))
                });

                Ok(Box::pin(converted_stream))
            }
            LLMStreamResponse::ToolCalls { .. } => {
                // Tool calls can't be streamed in the current architecture
                // Return an error - caller should use chat() for tool-enabled requests
                Err(ProviderError::ConfigError(
                    "Tool calls received during streaming - use chat() instead".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let converted = PcgRouterAdapter::convert_messages(&messages);

        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0]["role"], "system");
        assert_eq!(converted[0]["content"], "You are helpful");
        assert_eq!(converted[1]["role"], "user");
        assert_eq!(converted[2]["role"], "assistant");
    }

    #[test]
    fn test_tool_conversion() {
        let tools = vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get the weather".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                }
            }),
        }];

        let converted = PcgRouterAdapter::convert_tools(&tools);

        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "get_weather");
    }
}

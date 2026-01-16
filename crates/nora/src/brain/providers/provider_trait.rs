//! Provider trait and common types for multi-provider LLM support

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Supported LLM provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    /// Ollama local LLM (OpenAI-compatible)
    Ollama,
}

impl Default for ProviderType {
    fn default() -> Self {
        ProviderType::Ollama
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" | "claude" => Ok(ProviderType::Anthropic),
            "ollama" => Ok(ProviderType::Ollama),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

/// Error type for provider operations
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Response parse error: {0}")]
    ParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Rate limited: retry after {retry_after_ms:?}ms")]
    RateLimited { retry_after_ms: Option<u64> },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Provider not available: {0}")]
    NotAvailable(String),
}

/// Role of a message in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    /// For tool role messages - the ID of the tool call this is responding to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// For assistant messages that include tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRequest>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }

    pub fn assistant_with_tools(tool_calls: Vec<ToolCallRequest>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
        }
    }
}

/// A tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Configuration for a chat request
#[derive(Debug, Clone)]
pub struct ChatConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    /// Force tool usage: "auto", "required", "none", or specific tool name
    pub tool_choice: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            temperature: 0.2,
            max_tokens: 4096,
            tool_choice: None,
        }
    }
}

/// A complete chat request
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub config: ChatConfig,
}

/// Response from an LLM provider
#[derive(Debug, Clone)]
pub enum ProviderResponse {
    /// Text response from the LLM
    Text {
        content: String,
        usage: Option<TokenUsage>,
    },
    /// LLM wants to call tools
    ToolCalls {
        calls: Vec<ToolCallRequest>,
        usage: Option<TokenUsage>,
    },
}

impl ProviderResponse {
    /// Get the text content if this is a text response
    pub fn text(&self) -> Option<&str> {
        match self {
            ProviderResponse::Text { content, .. } => Some(content),
            ProviderResponse::ToolCalls { .. } => None,
        }
    }

    /// Get the tool calls if this is a tool call response
    pub fn tool_calls(&self) -> Option<&[ToolCallRequest]> {
        match self {
            ProviderResponse::Text { .. } => None,
            ProviderResponse::ToolCalls { calls, .. } => Some(calls),
        }
    }

    /// Get token usage if available
    pub fn usage(&self) -> Option<&TokenUsage> {
        match self {
            ProviderResponse::Text { usage, .. } => usage.as_ref(),
            ProviderResponse::ToolCalls { usage, .. } => usage.as_ref(),
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// A chunk of streamed response
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub is_done: bool,
}

/// Trait that all LLM providers must implement
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    /// Get the provider type
    fn provider_type(&self) -> ProviderType;

    /// Get the provider name for logging/display
    fn name(&self) -> &'static str;

    /// Check if this provider is properly configured and ready
    fn is_configured(&self) -> bool;

    /// Check if this provider supports tool/function calling
    fn supports_tools(&self) -> bool;

    /// Send a chat request and get a response
    async fn chat(&self, request: ChatRequest) -> Result<ProviderResponse, ProviderError>;

    /// Send a chat request and get a streaming response
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>, ProviderError>;

    /// Get the default model for this provider
    fn default_model(&self) -> &str;

    /// Validate that a model name is supported
    fn validate_model(&self, model: &str) -> bool;
}

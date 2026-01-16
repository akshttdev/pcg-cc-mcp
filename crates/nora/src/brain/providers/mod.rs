//! Multi-provider LLM abstraction layer
//!
//! This module provides a trait-based abstraction for different LLM providers,
//! enabling agents to use the optimal model for their specific use case.

mod anthropic;
mod openai;
mod provider_trait;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use provider_trait::{
    ChatConfig, ChatMessage, ChatRequest, LLMProviderTrait, ProviderError, ProviderResponse,
    ProviderType, StreamChunk, TokenUsage, ToolCallRequest, ToolDefinition,
};

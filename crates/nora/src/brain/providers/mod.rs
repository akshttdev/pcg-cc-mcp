//! Multi-provider LLM abstraction layer
//!
//! This module provides a trait-based abstraction for different LLM providers,
//! enabling agents to use the optimal model for their specific use case.

mod anthropic;
mod openai;
pub mod pcg_router_adapter;
mod provider_trait;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use pcg_router_adapter::PcgRouterAdapter;
pub use provider_trait::{
    ChatConfig, ChatMessage, ChatRequest, ContentBlock, LLMProviderTrait, ProviderError,
    ProviderResponse, ProviderType, StreamChunk, TokenUsage, ToolCallRequest, ToolDefinition,
};

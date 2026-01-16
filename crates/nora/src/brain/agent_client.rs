//! Agent-specific LLM client configuration
//!
//! This module provides utilities for creating LLM clients based on agent
//! database configurations, enabling each agent to use their optimal LLM provider.

use db::models::agent::Agent;
use serde::{Deserialize, Serialize};

use super::{LLMClient, LLMConfig, LLMProvider};

/// Parsed agent model configuration from database JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModelConfig {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub system_prompt_prefix: String,
    /// Optional provider override (if not specified, inferred from model name)
    #[serde(default)]
    pub provider: Option<String>,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for AgentModelConfig {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            system_prompt_prefix: String::new(),
            provider: None,
        }
    }
}

/// Infer the LLM provider from a model name
///
/// # Examples
/// - "claude-sonnet-4" → Anthropic
/// - "claude-3-5-sonnet-20241022" → Anthropic
/// - "gpt-4o" → OpenAI
/// - "gpt-4-turbo" → OpenAI
/// - "gpt-oss" → Ollama (local)
/// - "llama3.3" → Ollama (local)
pub fn infer_provider_from_model(model: &str) -> LLMProvider {
    let model_lower = model.to_lowercase();

    // Check Ollama models FIRST (before generic gpt check)
    if model_lower.starts_with("deepseek")
        || model_lower.starts_with("gpt-oss")
        || model_lower.starts_with("gptoss")
        || model_lower.starts_with("llama")
        || model_lower.starts_with("qwen")
        || model_lower.starts_with("mistral")
        || model_lower.starts_with("phi")
    {
        LLMProvider::Ollama
    } else if model_lower.starts_with("claude") || model_lower.contains("anthropic") {
        LLMProvider::Anthropic
    } else if model_lower.starts_with("gpt-4")
        || model_lower.starts_with("gpt-3")
        || model_lower.starts_with("o1")
        || model_lower.contains("openai")
    {
        LLMProvider::OpenAI
    } else {
        // Default to Ollama for unknown models (local-first approach)
        // This includes: gpt-oss, llama, mistral, codellama, deepseek, etc.
        tracing::info!(
            "Model '{}' not recognized as cloud provider, using Ollama (local)",
            model
        );
        LLMProvider::Ollama
    }
}

/// Normalize model name to the format expected by each provider
///
/// Handles common aliases and ensures model names are valid for the target provider.
pub fn normalize_model_name(model: &str, provider: &LLMProvider) -> String {
    let model_lower = model.to_lowercase();

    match provider {
        LLMProvider::Anthropic => {
            // Handle common Claude aliases
            match model_lower.as_str() {
                "claude-sonnet-4" | "sonnet-4" | "claude-4-sonnet" => {
                    "claude-sonnet-4-20250514".to_string()
                }
                "claude-opus-4" | "opus-4" | "claude-4-opus" => {
                    "claude-opus-4-20250514".to_string()
                }
                "claude-3.5-sonnet" | "claude-3-5-sonnet" => {
                    "claude-3-5-sonnet-20241022".to_string()
                }
                "claude-3.5-haiku" | "claude-3-5-haiku" => {
                    "claude-3-5-haiku-20241022".to_string()
                }
                "claude-3-opus" => "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet" => "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku" => "claude-3-haiku-20240307".to_string(),
                _ => model.to_string(),
            }
        }
        LLMProvider::OpenAI => {
            // Handle common GPT aliases
            match model_lower.as_str() {
                "gpt4" | "gpt-4" => "gpt-4o".to_string(),
                "gpt4o" => "gpt-4o".to_string(),
                "gpt4-turbo" => "gpt-4-turbo".to_string(),
                "gpt35" | "gpt-3.5" => "gpt-3.5-turbo".to_string(),
                _ => model.to_string(),
            }
        }
        LLMProvider::Ollama => {
            // Ollama model names are passed through as-is
            // Common models: gpt-oss, llama3.3, codellama, mistral, etc.
            model.to_string()
        }
    }
}

/// Create an LLMClient configured for a specific agent
///
/// Uses the agent's default_model to determine the provider, and applies
/// the agent's model_config settings (temperature, max_tokens, etc.).
pub fn create_client_for_agent(agent: &Agent) -> LLMClient {
    // Get model name (default to Claude sonnet if not specified)
    let model = agent
        .default_model
        .as_deref()
        .unwrap_or("claude-sonnet-4-20250514");

    // Parse model config from JSON
    let config: AgentModelConfig = agent
        .model_config
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Determine provider (from config override or inferred from model)
    let provider = config
        .provider
        .as_ref()
        .map(|p| match p.to_lowercase().as_str() {
            "anthropic" | "claude" => LLMProvider::Anthropic,
            "openai" | "gpt" => LLMProvider::OpenAI,
            "ollama" | "local" => LLMProvider::Ollama,
            _ => infer_provider_from_model(model),
        })
        .unwrap_or_else(|| infer_provider_from_model(model));

    // Normalize model name for the provider
    let normalized_model = normalize_model_name(model, &provider);

    // Build system prompt with agent prefix
    let system_prompt = if config.system_prompt_prefix.is_empty() {
        format!("You are {}, {}.", agent.short_name, agent.designation)
    } else {
        config.system_prompt_prefix.clone()
    };

    // Determine endpoint based on provider
    let endpoint = match provider {
        LLMProvider::Ollama => {
            // Check for custom endpoint first, then default to Ollama's port
            std::env::var("OLLAMA_ENDPOINT").ok().or_else(|| {
                // Check if Ollama is running on default port
                if std::net::TcpStream::connect("127.0.0.1:11434").is_ok() {
                    Some("http://127.0.0.1:11434/v1/chat/completions".to_string())
                } else {
                    None
                }
            })
        }
        LLMProvider::OpenAI => None, // Uses default OpenAI endpoint
        LLMProvider::Anthropic => None, // Uses Anthropic SDK
    };

    tracing::info!(
        "Creating LLM client for agent '{}': provider={:?}, model={}, endpoint={:?}",
        agent.short_name,
        provider,
        normalized_model,
        endpoint
    );

    let llm_config = LLMConfig {
        provider,
        model: normalized_model,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        system_prompt,
        endpoint,
    };

    LLMClient::new(llm_config)
}

/// Get the recommended provider for each agent based on their specialization
///
/// This provides hints for optimal provider selection based on agent capabilities.
pub fn get_recommended_provider(agent_name: &str) -> LLMProvider {
    match agent_name.to_lowercase().as_str() {
        // Coding agents → Anthropic (Claude excels at coding)
        "auri" | "aura" | "developer" | "coder" => LLMProvider::Anthropic,

        // Research agents → Anthropic (Claude is better at analysis)
        "scout" | "researcher" | "oracle" => LLMProvider::Anthropic,

        // Orchestration → OpenAI (good at coordination)
        "nora" | "orchestrator" => LLMProvider::OpenAI,

        // Visual/Creative → OpenAI (GPT-4o has vision)
        "maci" | "editron" | "cinematographer" => LLMProvider::OpenAI,

        // Strategy → Either works well
        "astra" | "genesis" => LLMProvider::OpenAI,

        // Default to Anthropic for unknown agents
        _ => LLMProvider::Anthropic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_inference() {
        assert!(matches!(
            infer_provider_from_model("claude-sonnet-4"),
            LLMProvider::Anthropic
        ));
        assert!(matches!(
            infer_provider_from_model("claude-3-5-sonnet-20241022"),
            LLMProvider::Anthropic
        ));
        assert!(matches!(
            infer_provider_from_model("gpt-4o"),
            LLMProvider::OpenAI
        ));
        assert!(matches!(
            infer_provider_from_model("gpt-4-turbo"),
            LLMProvider::OpenAI
        ));
        assert!(matches!(
            infer_provider_from_model("o1-preview"),
            LLMProvider::OpenAI
        ));
    }

    #[test]
    fn test_model_normalization() {
        assert_eq!(
            normalize_model_name("claude-sonnet-4", &LLMProvider::Anthropic),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(
            normalize_model_name("gpt4", &LLMProvider::OpenAI),
            "gpt-4o"
        );
    }
}

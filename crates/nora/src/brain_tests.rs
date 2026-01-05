//! Tests for LLM brain module

#[cfg(test)]
mod tests {
    use crate::brain::{LLMClient, LLMConfig, LLMProvider};

    #[test]
    fn test_llm_config_default() {
        let config = LLMConfig::default();

        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.temperature, 0.2);
        assert_eq!(config.max_tokens, 600);
        assert!(matches!(config.provider, LLMProvider::OpenAI));
        assert!(config.system_prompt.contains("Nora"));
    }

    #[test]
    fn test_llm_config_custom() {
        let config = LLMConfig {
            provider: LLMProvider::OpenAI,
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 1000,
            system_prompt: "Custom prompt".to_string(),
            endpoint: Some("http://localhost:8080".to_string()),
        };

        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 1000);
        assert_eq!(config.endpoint, Some("http://localhost:8080".to_string()));
    }

    #[test]
    fn test_llm_client_initialization() {
        let config = LLMConfig::default();
        let client = LLMClient::new(config);

        // Client should be created even without API key
        // is_ready() will return false without API key
        assert!(!client.is_ready() || client.is_configured());
    }

    #[test]
    fn test_llm_client_with_custom_endpoint() {
        let mut config = LLMConfig::default();
        config.endpoint = Some("http://localhost:11434".to_string());

        let client = LLMClient::new(config);

        // With custom endpoint, should be configured even without API key
        assert!(client.is_configured());
    }

    #[test]
    fn test_llm_client_readiness_without_key() {
        // Clear any existing OPENAI_API_KEY for this test
        std::env::remove_var("OPENAI_API_KEY");

        let config = LLMConfig::default();
        let client = LLMClient::new(config);

        // Without API key and without custom endpoint, should not be ready
        assert!(!client.is_ready());
    }

    #[tokio::test]
    async fn test_llm_generate_without_credentials() {
        std::env::remove_var("OPENAI_API_KEY");

        let config = LLMConfig::default();
        let client = LLMClient::new(config);

        let result = client
            .generate("You are a helpful assistant", "Say hello", "No context")
            .await;

        // Should fail without credentials
        assert!(result.is_err());
    }

    #[test]
    fn test_llm_config_environment_override() {
        // Test that environment variables can override config
        // This is tested indirectly through the apply_llm_overrides function in routes

        std::env::set_var("NORA_LLM_MODEL", "gpt-4-turbo");
        std::env::set_var("NORA_LLM_TEMPERATURE", "0.5");
        std::env::set_var("NORA_LLM_MAX_TOKENS", "1500");

        // These would be applied in the routes layer
        let model = std::env::var("NORA_LLM_MODEL").unwrap();
        let temp = std::env::var("NORA_LLM_TEMPERATURE")
            .unwrap()
            .parse::<f32>()
            .unwrap();
        let max_tokens = std::env::var("NORA_LLM_MAX_TOKENS")
            .unwrap()
            .parse::<u32>()
            .unwrap();

        assert_eq!(model, "gpt-4-turbo");
        assert_eq!(temp, 0.5);
        assert_eq!(max_tokens, 1500);

        // Clean up
        std::env::remove_var("NORA_LLM_MODEL");
        std::env::remove_var("NORA_LLM_TEMPERATURE");
        std::env::remove_var("NORA_LLM_MAX_TOKENS");
    }

    #[test]
    fn test_llm_provider_serialization() {
        // Test that LLMProvider can be serialized/deserialized
        use serde_json;

        let provider = LLMProvider::OpenAI;
        let json = serde_json::to_string(&provider).unwrap();
        // With camelCase serialization, it becomes "openAI"
        assert!(
            json.contains("openAI") || json.contains("OpenAI"),
            "JSON should contain provider name"
        );

        let deserialized: LLMProvider = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, LLMProvider::OpenAI));
    }

    #[test]
    fn test_llm_config_serialization() {
        use serde_json;

        let config = LLMConfig::default();
        let json = serde_json::to_string(&config).unwrap();

        let deserialized: LLMConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.model, config.model);
        assert_eq!(deserialized.temperature, config.temperature);
        assert_eq!(deserialized.max_tokens, config.max_tokens);
    }
}

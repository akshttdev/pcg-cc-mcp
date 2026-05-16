//! Nora mode presets and LLM configuration overrides.

use nora::{
    brain::{infer_provider_from_model, LLMConfig},
    personality::PersonalityConfig,
    voice::VoiceConfig,
    LLMProvider, NoraConfig,
};
use once_cell::sync::Lazy;
use serde::Serialize;

#[derive(Clone)]
pub(crate) struct NoraModePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub config: NoraConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoraModeSummary {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub(crate) static NORA_MODE_PRESETS: Lazy<Vec<NoraModePreset>> = Lazy::new(|| {
    let default_cfg = NoraConfig::default();
    let mut rapid_cfg = NoraConfig::default();
    rapid_cfg.voice = VoiceConfig::development();
    rapid_cfg.personality = PersonalityConfig::casual_british();

    let mut boardroom_cfg = NoraConfig::default();
    boardroom_cfg.voice = VoiceConfig::british_executive();
    boardroom_cfg.personality = PersonalityConfig::british_executive_assistant();

    vec![
        NoraModePreset {
            id: "rapid-builder",
            label: "Rapid Builder",
            description: "Fast prototyping mode with casual tone and lightweight voice stack",
            config: rapid_cfg,
        },
        NoraModePreset {
            id: "boardroom",
            label: "Boardroom",
            description: "High-formality executive briefing mode",
            config: boardroom_cfg,
        },
        NoraModePreset {
            id: "standard",
            label: "Standard",
            description: "Balanced configuration used by default",
            config: default_cfg,
        },
    ]
});

/// Apply environment variable overrides to Nora's LLM configuration.
pub(crate) fn apply_llm_overrides(config: &mut NoraConfig) {
    // If OPENAI_API_KEY is set, ensure we have an LLM config
    if std::env::var("OPENAI_API_KEY").is_ok() && config.llm.is_none() {
        config.llm = Some(LLMConfig::default());
        tracing::info!("LLM enabled via OPENAI_API_KEY");
    }

    if let Ok(model) = std::env::var("NORA_LLM_MODEL") {
        let llm = config.llm.get_or_insert_with(LLMConfig::default);
        llm.model = model.clone();
        llm.provider = infer_provider_from_model(&model);
        tracing::info!(
            "Nora LLM model set to: {} (provider: {:?})",
            model,
            llm.provider
        );
    }

    // Explicit provider override (takes precedence over inference)
    if let Ok(provider) = std::env::var("NORA_LLM_PROVIDER") {
        let llm = config.llm.get_or_insert_with(LLMConfig::default);
        llm.provider = match provider.to_lowercase().as_str() {
            "anthropic" | "claude" => LLMProvider::Anthropic,
            "openai" | "gpt" => LLMProvider::OpenAI,
            _ => LLMProvider::Ollama,
        };
    }

    if let Ok(endpoint) = std::env::var("NORA_LLM_ENDPOINT") {
        config.llm.get_or_insert_with(LLMConfig::default).endpoint = Some(endpoint);
    }

    if let Ok(temp) = std::env::var("NORA_LLM_TEMPERATURE") {
        if let Ok(value) = temp.parse::<f32>() {
            config
                .llm
                .get_or_insert_with(LLMConfig::default)
                .temperature = value;
        }
    }

    if let Ok(max_tokens) = std::env::var("NORA_LLM_MAX_TOKENS") {
        if let Ok(value) = max_tokens.parse::<u32>() {
            config.llm.get_or_insert_with(LLMConfig::default).max_tokens = value;
        }
    }

    if let Ok(prompt) = std::env::var("NORA_LLM_SYSTEM_PROMPT") {
        config
            .llm
            .get_or_insert_with(LLMConfig::default)
            .system_prompt = prompt;
    }
}

/// Default capabilities list shown after initialization.
pub(crate) fn default_capabilities() -> Vec<String> {
    vec![
        "Voice Interaction".to_string(),
        "Task Coordination".to_string(),
        "Strategic Planning".to_string(),
        "Performance Analysis".to_string(),
        "Decision Support".to_string(),
        "Communication Management".to_string(),
    ]
}

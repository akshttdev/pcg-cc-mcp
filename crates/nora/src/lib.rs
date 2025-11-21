//! # Nora - Executive AI Assistant
//!
//! Advanced AI assistant designed to serve as Executive Assistant/COO for PowerClub Global.
//! Features sophisticated voice capabilities with British accent and executive-level coordination.

pub mod agent;
pub mod brain;
pub mod cache;
pub mod coordination;
pub mod executor;
pub mod integrations;
pub mod memory;
pub mod personality;
pub mod tools;
pub mod voice;

#[cfg(test)]
mod agent_tests;
#[cfg(test)]
mod brain_tests;
#[cfg(test)]
mod executor_tests;
#[cfg(test)]
mod personality_tests;

pub use agent::NoraAgent;
pub use brain::{LLMConfig, LLMProvider};
pub use cache::{CacheKey, CachedResponse, LlmCache, ResponseMetadata};
pub use coordination::{CoordinationEvent, CoordinationManager};
pub use executor::{
    BoardInfo, PodInfo, ProjectDetails, ProjectInfo, ProjectStats, TaskDefinition, TaskExecutor,
    TaskInfo,
};
pub use memory::{ConversationMemory, ExecutiveContext};
pub use personality::{BritishPersonality, PersonalityConfig};
use serde::{Deserialize, Serialize};
pub use tools::{ExecutiveTools, NoraExecutiveTool};
use ts_rs::TS;
pub use voice::{SpeechRequest, SpeechResponse, VoiceConfig, VoiceEngine};

/// Core configuration for Nora
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraConfig {
    pub voice: VoiceConfig,
    pub personality: PersonalityConfig,
    pub executive_mode: bool,
    pub proactive_notifications: bool,
    pub context_awareness: bool,
    pub multi_agent_coordination: bool,
    #[serde(default)]
    pub llm: Option<LLMConfig>,
}

impl Default for NoraConfig {
    fn default() -> Self {
        Self {
            voice: VoiceConfig::british_executive(),
            personality: PersonalityConfig::british_executive_assistant(),
            executive_mode: true,
            proactive_notifications: true,
            context_awareness: true,
            multi_agent_coordination: true,
            llm: Some(LLMConfig::default()),
        }
    }
}

/// Main error types for Nora operations
#[derive(Debug, thiserror::Error)]
pub enum NoraError {
    #[error("Voice processing error: {0}")]
    VoiceError(String),

    #[error("Coordination error: {0}")]
    CoordinationError(String),

    #[error("Memory error: {0}")]
    MemoryError(String),

    #[error("Tools error: {0}")]
    ToolsError(String),

    #[error("Tool execution error: {0}")]
    ToolExecutionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Voice error: {0}")]
    VoiceEngineError(#[from] crate::voice::VoiceError),

    #[error("Execution error: {0}")]
    ExecutionError(String),
}

pub type Result<T> = std::result::Result<T, NoraError>;

/// Initialize Nora with configuration
pub async fn initialize_nora(config: NoraConfig) -> Result<NoraAgent> {
    tracing::info!("Initializing Nora Executive Assistant...");

    let agent = NoraAgent::new(config).await?;

    tracing::info!("Nora initialized successfully");
    Ok(agent)
}

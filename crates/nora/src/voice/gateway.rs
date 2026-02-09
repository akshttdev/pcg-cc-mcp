//! Voice Gateway - Orchestrates voice interactions
//!
//! Stub module for voice gateway functionality.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use ts_rs::TS;

/// Voice gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceGatewayConfig {
    pub enable_stt: bool,
    pub enable_tts: bool,
    pub default_voice: String,
}

impl Default for VoiceGatewayConfig {
    fn default() -> Self {
        Self {
            enable_stt: true,
            enable_tts: true,
            default_voice: "nora".to_string(),
        }
    }
}

/// Voice gateway for orchestrating voice interactions
pub struct VoiceGateway {
    config: VoiceGatewayConfig,
}

impl VoiceGateway {
    pub fn new(config: VoiceGatewayConfig) -> Self {
        Self { config }
    }
}

/// Gateway session
#[derive(Debug, Clone)]
pub struct GatewaySession {
    pub session_id: String,
    pub active: bool,
}

/// Voice gateway event
#[derive(Debug, Clone)]
pub enum VoiceGatewayEvent {
    SessionStarted(String),
    SessionEnded(String),
    TranscriptionReceived(String),
    ResponseGenerated(String),
}

/// Speaker identification
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Speaker {
    User,
    Assistant,
    System,
}

/// Conversation turn
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConversationTurn {
    pub speaker: Speaker,
    pub text: String,
    pub timestamp: u64,
}

/// Command context for voice commands
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub session_id: String,
    pub user_id: Option<String>,
}

/// Command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub text: String,
    pub success: bool,
}

/// Command handler trait
pub trait CommandHandler: Send + Sync {
    fn handle<'a>(
        &'a self,
        context: &'a CommandContext,
        command: &'a str,
    ) -> Pin<Box<dyn Future<Output = CommandResponse> + Send + 'a>>;
}

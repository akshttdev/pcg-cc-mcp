//! Voice Command Router
//!
//! Routes voice commands to appropriate handlers.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Voice intent classification
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum VoiceIntent {
    /// Query for information
    Query,
    /// Execute a command
    Command,
    /// Status check
    Status,
    /// Navigation
    Navigate,
    /// Control action
    Control,
    /// Unknown/unclear intent
    Unknown,
}

/// Intent match result
#[derive(Debug, Clone)]
pub struct IntentMatch {
    pub intent: VoiceIntent,
    pub confidence: f32,
    pub entities: Vec<String>,
}

/// Control action types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ControlAction {
    Start,
    Stop,
    Pause,
    Resume,
    Cancel,
}

/// Status target
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StatusTarget {
    Task,
    Project,
    System,
    Network,
}

/// Voice command router
pub struct VoiceCommandRouter {
    // Router state
}

impl VoiceCommandRouter {
    pub fn new() -> Self {
        Self {}
    }

    /// Route a voice command to the appropriate handler
    pub fn route(&self, text: &str) -> IntentMatch {
        // Simple keyword-based routing for now
        let text_lower = text.to_lowercase();

        if text_lower.contains("status") {
            IntentMatch {
                intent: VoiceIntent::Status,
                confidence: 0.8,
                entities: vec![],
            }
        } else if text_lower.contains("start") || text_lower.contains("run") {
            IntentMatch {
                intent: VoiceIntent::Command,
                confidence: 0.8,
                entities: vec![],
            }
        } else if text_lower.contains("what") || text_lower.contains("how") {
            IntentMatch {
                intent: VoiceIntent::Query,
                confidence: 0.7,
                entities: vec![],
            }
        } else {
            IntentMatch {
                intent: VoiceIntent::Unknown,
                confidence: 0.3,
                entities: vec![],
            }
        }
    }
}

impl Default for VoiceCommandRouter {
    fn default() -> Self {
        Self::new()
    }
}

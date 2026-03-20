//! Agent Response Protocol
//!
//! Structured response envelope for agent flow outputs.
//! Enables the orchestration engine to parse agent results,
//! detect clarification requests, and route next actions.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Structured envelope wrapping all agent flow outputs.
/// The orchestration engine inspects `status` to decide next steps.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentResponseEnvelope {
    /// Outcome of this agent step
    pub status: AgentResponseStatus,
    /// Human-readable summary of what happened
    pub summary: String,
    /// Structured output data (step-specific)
    #[ts(type = "Record<string, unknown> | null")]
    pub data: Option<serde_json::Value>,
    /// If status is NeedsClarification, the clarification request details
    pub clarification: Option<ClarificationRequest>,
    /// Suggested next action for the orchestration engine
    pub suggested_next: Option<String>,
    /// Confidence score (0.0 - 1.0) for the agent's output
    pub confidence: Option<f64>,
}

/// Outcome status of an agent step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AgentResponseStatus {
    /// Step completed successfully
    Success,
    /// Step failed with an error
    Error,
    /// Agent needs human input before proceeding
    NeedsClarification,
    /// Partial progress — more work needed
    InProgress,
    /// Step was skipped (e.g., preconditions not met)
    Skipped,
}

/// Details of a clarification request from an agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClarificationRequest {
    /// What the agent needs to know
    pub question: String,
    /// Why this information is needed
    pub context: Option<String>,
    /// Suggested options (if applicable)
    pub options: Option<Vec<String>>,
    /// Whether this blocks all progress or just this step
    pub blocking: bool,
}

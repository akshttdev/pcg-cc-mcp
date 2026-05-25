//! Sub-agent trait and concrete skeleton implementations.
//!
//! The trait is intentionally narrow: an agent receives a `LoopEvent` and
//! optionally returns more events. Anything heavier (LLM calls, shelling
//! out, file I/O) lives inside the implementation, hidden behind this
//! interface so the orchestrator stays test-friendly.

pub mod auri;
pub mod fake_user;
pub mod playwright;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::events::LoopEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentId {
    Auri,
    Playwright,
    FakeUser,
    /// External trigger (CI webhook, manual operator), not a registered agent.
    External,
}

impl AgentId {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentId::Auri => "auri",
            AgentId::Playwright => "playwright",
            AgentId::FakeUser => "fake_user",
            AgentId::External => "external",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent timed out")]
    Timeout,
    #[error("agent rejected the event")]
    Rejected,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AgentResult = Result<Vec<LoopEvent>, AgentError>;

/// What the orchestrator sees of a sub-agent.
///
/// Implementations must be `Send + Sync` because the orchestrator holds an
/// `Arc<dyn Agent>` and dispatches `handle` from a `tokio::spawn`'d driver.
#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> AgentId;

    /// Whether this agent wants to react to a given event. Default: react
    /// to everything. Implementations can narrow this down to avoid waking
    /// up for irrelevant traffic.
    fn interested_in(&self, _event: &LoopEvent) -> bool {
        true
    }

    /// Handle an event. The returned events are republished on the bus.
    async fn handle(&self, event: LoopEvent) -> AgentResult;
}

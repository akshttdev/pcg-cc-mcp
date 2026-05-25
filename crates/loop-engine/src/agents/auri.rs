//! Auri — the coding sub-agent skeleton.
//!
//! In the production wiring this delegates to whichever back-end the
//! operator chose (Hermes, Claude, Gemini) via the `executors` crate. For
//! this initial skeleton the implementation is intentionally inert: it
//! records what it would have done and emits no events, so the
//! orchestrator and circuit breaker can be exercised end-to-end without
//! touching git or any LLM.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::agents::{Agent, AgentError, AgentId, AgentResult};
use crate::events::LoopEvent;

#[derive(Debug, Clone)]
pub struct AuriRecord {
    pub event_kind: &'static str,
    pub note: String,
}

#[derive(Default)]
pub struct AuriAgent {
    log: Arc<Mutex<Vec<AuriRecord>>>,
}

impl AuriAgent {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn history(&self) -> Vec<AuriRecord> {
        self.log.lock().await.clone()
    }
}

#[async_trait]
impl Agent for AuriAgent {
    fn id(&self) -> AgentId {
        AgentId::Auri
    }

    fn interested_in(&self, event: &LoopEvent) -> bool {
        matches!(
            event,
            LoopEvent::FailureDetected { .. } | LoopEvent::UxReport { .. }
        )
    }

    async fn handle(&self, event: LoopEvent) -> AgentResult {
        let note = match &event {
            LoopEvent::FailureDetected { detail, .. } => {
                format!("would investigate and fix: {detail}")
            }
            LoopEvent::UxReport { blockers, .. } => {
                format!("would address {} UX blocker(s)", blockers.len())
            }
            _ => "no-op".to_string(),
        };
        self.log
            .lock()
            .await
            .push(AuriRecord {
                event_kind: event.kind(),
                note,
            });
        // Real implementation will return a CommitCompleted event after
        // the back-end finishes. Skeleton returns nothing so the loop is
        // a clean closed system in tests.
        Ok(Vec::new())
    }
}

#[allow(dead_code)]
fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<AuriAgent>();
    // Stub stays generic-friendly even if we add fields later.
    let _ = std::marker::PhantomData::<AgentError>;
}

//! Playwright — the automated test sub-agent skeleton.
//!
//! On a real wiring, this agent invokes `npx playwright test` against the
//! commit just produced by Auri. The skeleton records the commit it would
//! have tested and returns no follow-up events.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::agents::{Agent, AgentId, AgentResult};
use crate::events::LoopEvent;

#[derive(Default)]
pub struct PlaywrightAgent {
    runs: Arc<Mutex<Vec<String>>>, // shas observed
}

impl PlaywrightAgent {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn runs(&self) -> Vec<String> {
        self.runs.lock().await.clone()
    }
}

#[async_trait]
impl Agent for PlaywrightAgent {
    fn id(&self) -> AgentId {
        AgentId::Playwright
    }

    fn interested_in(&self, event: &LoopEvent) -> bool {
        matches!(event, LoopEvent::CommitCompleted { .. })
    }

    async fn handle(&self, event: LoopEvent) -> AgentResult {
        if let LoopEvent::CommitCompleted { sha, .. } = &event {
            self.runs.lock().await.push(sha.clone());
        }
        Ok(Vec::new())
    }
}

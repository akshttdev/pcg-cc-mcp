//! Fake User — UX stress sub-agent skeleton.
//!
//! Real wiring: drives the running app through Playwright the same way a
//! confused first-time user would (mash buttons, navigate in odd orders,
//! provide weird input). Surfaces dead clicks, missing affordances and
//! console errors as `UxReport` events. Skeleton just counts invocations.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use crate::agents::{Agent, AgentId, AgentResult};
use crate::events::LoopEvent;

#[derive(Default)]
pub struct FakeUserAgent {
    invocations: Arc<AtomicU32>,
}

impl FakeUserAgent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn invocation_count(&self) -> u32 {
        self.invocations.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Agent for FakeUserAgent {
    fn id(&self) -> AgentId {
        AgentId::FakeUser
    }

    fn interested_in(&self, event: &LoopEvent) -> bool {
        matches!(event, LoopEvent::CommitCompleted { .. })
    }

    async fn handle(&self, _event: LoopEvent) -> AgentResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

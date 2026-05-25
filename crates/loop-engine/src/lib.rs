//! Autonomous dev loop engine.
//!
//! Wires three sub-agents (Auri, Playwright, Fake User) into a reactive
//! feedback loop with a circuit breaker that halts the system on repeated
//! same-cause failures or high overall failure rate. See
//! `LOOP_ENGINE_ARCHITECTURE.md` for the full design.

pub mod agents;
pub mod circuit_breaker;
pub mod events;
pub mod orchestrator;
pub mod report;
pub mod sustained;

pub use agents::{Agent, AgentId, AgentResult};
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitSnapshot, Decision, TripReason,
};
pub use events::{CauseHash, EventBus, LoopEvent, TestFailure, UxBlocker};
pub use orchestrator::{Orchestrator, OrchestratorConfig};
pub use report::HaltReport;
pub use sustained::sustained_loop;

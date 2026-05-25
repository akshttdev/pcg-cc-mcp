//! Reactive orchestration harness.
//!
//! Owns the event bus, the circuit breaker, and one driver task per
//! registered agent. External callers publish via `bus().publish(event)`
//! and run the orchestrator via `run_reactive()`, which returns when the
//! breaker trips or the cancellation token fires.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::agents::{Agent, AgentError, AgentId};
use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, Decision};
use crate::events::{EventBus, LoopEvent};
use crate::report::HaltReport;

/// Default cap on a single agent's `handle` call. Anything slower is
/// almost certainly hung — abort rather than accumulate dead work.
const DEFAULT_AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Default in-memory capacity of the event bus.
const DEFAULT_BUS_CAPACITY: usize = 256;

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub bus_capacity: usize,
    pub agent_timeout: Duration,
    pub circuit_breaker: CircuitBreakerConfig,
    /// Where halt reports are written. Created on demand.
    pub report_dir: PathBuf,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            bus_capacity: DEFAULT_BUS_CAPACITY,
            agent_timeout: DEFAULT_AGENT_TIMEOUT,
            circuit_breaker: CircuitBreakerConfig::default(),
            report_dir: PathBuf::from("planning/reports"),
        }
    }
}

pub struct Orchestrator {
    bus: EventBus,
    breaker: Arc<Mutex<CircuitBreaker>>,
    agents: Vec<Arc<dyn Agent>>,
    config: OrchestratorConfig,
    cancel: CancellationToken,
    history: Arc<Mutex<Vec<LoopEvent>>>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let bus = EventBus::new(config.bus_capacity);
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(
            config.circuit_breaker.clone(),
        )));
        Self {
            bus,
            breaker,
            agents: Vec::new(),
            config,
            cancel: CancellationToken::new(),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn bus(&self) -> &EventBus {
        &self.bus
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    pub fn register(&mut self, agent: Arc<dyn Agent>) {
        self.agents.push(agent);
    }

    /// Run the reactive harness until the circuit trips or `cancel_token`
    /// is triggered. Returns the final report path on a clean halt.
    pub async fn run_reactive(&self) -> Result<PathBuf, anyhow::Error> {
        // One driver task per agent.
        let mut driver_handles = Vec::with_capacity(self.agents.len());
        for agent in &self.agents {
            driver_handles.push(self.spawn_agent_driver(agent.clone()));
        }

        // Plus one task that observes every event through the breaker.
        let breaker_handle = self.spawn_breaker_driver();

        // Wait for either cancellation or any driver to exit on its own.
        // The breaker driver triggers cancellation on trip, so this also
        // catches the "tripped" path.
        self.cancel.cancelled().await;

        for h in driver_handles {
            h.abort();
        }
        breaker_handle.abort();

        let (snapshot, trip_reason) = {
            let br = self.breaker.lock().await;
            (br.snapshot(), br.tripped_reason())
        };
        let history = self.history.lock().await.clone();

        let report = HaltReport::build(trip_reason, snapshot, history);
        let path = report.write(&self.config.report_dir).await?;
        self.bus.publish(LoopEvent::LoopHalted {
            at: Utc::now(),
            final_report_path: path.clone(),
        });
        Ok(path)
    }

    fn spawn_breaker_driver(&self) -> tokio::task::JoinHandle<()> {
        let mut rx = self.bus.subscribe();
        let breaker = self.breaker.clone();
        let cancel = self.cancel.clone();
        let bus = self.bus.clone();
        let history = self.history.clone();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        history.lock().await.push(event.clone());
                        let mut br = breaker.lock().await;
                        let decision = br.observe(&event);
                        if let Decision::Tripped(reason) = decision {
                            let snapshot = br.snapshot();
                            drop(br);
                            warn!(?reason, "circuit breaker tripped — halting loop");
                            bus.publish(LoopEvent::CircuitTripped {
                                at: Utc::now(),
                                reason,
                                snapshot,
                            });
                            cancel.cancel();
                            return;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "breaker driver lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                }
            }
        })
    }

    fn spawn_agent_driver(&self, agent: Arc<dyn Agent>) -> tokio::task::JoinHandle<()> {
        let mut rx = self.bus.subscribe();
        let bus = self.bus.clone();
        let cancel = self.cancel.clone();
        let agent_timeout = self.config.agent_timeout;
        let agent_id = agent.id();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    msg = rx.recv() => match msg {
                        Ok(event) => {
                            if !agent.interested_in(&event) {
                                continue;
                            }
                            dispatch_one(&agent, agent_id, event, agent_timeout, &bus).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(agent = agent_id.as_str(), skipped, "agent driver lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        })
    }
}

async fn dispatch_one(
    agent: &Arc<dyn Agent>,
    id: AgentId,
    event: LoopEvent,
    agent_timeout: Duration,
    bus: &EventBus,
) {
    let result = timeout(agent_timeout, agent.handle(event)).await;
    match result {
        Err(_) => {
            warn!(agent = id.as_str(), "agent handle timed out");
            bus.publish(LoopEvent::FailureDetected {
                at: Utc::now(),
                sha: None,
                cause: crate::events::CauseHash::from_normalised(&format!(
                    "agent:{}:timeout",
                    id.as_str()
                )),
                detail: format!("agent {} timed out", id.as_str()),
                source: id,
            });
        }
        Ok(Err(AgentError::Timeout)) => {
            warn!(agent = id.as_str(), "agent reported timeout");
        }
        Ok(Err(AgentError::Rejected)) => {
            debug!(agent = id.as_str(), "agent rejected event");
        }
        Ok(Err(AgentError::Other(err))) => {
            error!(agent = id.as_str(), error = %err, "agent failed");
            bus.publish(LoopEvent::FailureDetected {
                at: Utc::now(),
                sha: None,
                cause: crate::events::CauseHash::from_normalised(&format!(
                    "agent:{}:error:{}",
                    id.as_str(),
                    err
                )),
                detail: format!("agent {} errored: {err}", id.as_str()),
                source: id,
            });
        }
        Ok(Ok(emitted)) => {
            for ev in emitted {
                bus.publish(ev);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::auri::AuriAgent;
    use crate::agents::fake_user::FakeUserAgent;
    use crate::agents::playwright::PlaywrightAgent;
    use chrono::Utc;
    use tempfile::TempDir;

    fn commit_event(sha: &str) -> LoopEvent {
        LoopEvent::CommitCompleted {
            at: Utc::now(),
            sha: sha.into(),
            author: AgentId::External,
            summary: "test".into(),
            loop_generated: false,
        }
    }

    fn failure_event(cause: &str) -> LoopEvent {
        LoopEvent::FailureDetected {
            at: Utc::now(),
            sha: None,
            cause: crate::events::CauseHash::from_normalised(cause),
            detail: cause.into(),
            source: AgentId::External,
        }
    }

    #[tokio::test]
    async fn agents_react_to_commit_and_trip_on_repeated_failure() {
        let tmp = TempDir::new().expect("tmp");
        let mut config = OrchestratorConfig::default();
        config.report_dir = tmp.path().to_path_buf();
        let mut orch = Orchestrator::new(config);

        let auri = Arc::new(AuriAgent::new());
        let pw = Arc::new(PlaywrightAgent::new());
        let fu = Arc::new(FakeUserAgent::new());
        orch.register(auri.clone());
        orch.register(pw.clone());
        orch.register(fu.clone());

        let cancel = orch.cancel_token();
        let bus = orch.bus().clone();

        let run_handle = tokio::spawn(async move { orch.run_reactive().await });

        // Give driver tasks a beat to subscribe before publishing.
        tokio::time::sleep(Duration::from_millis(50)).await;

        bus.publish(commit_event("aaa"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        bus.publish(failure_event("E1"));
        bus.publish(failure_event("E1"));
        bus.publish(failure_event("E1"));

        // Should auto-trip within a short window.
        let path = tokio::time::timeout(Duration::from_secs(5), run_handle)
            .await
            .expect("run did not finish in time")
            .expect("join")
            .expect("report path");

        assert!(path.exists(), "report should be written");
        assert!(cancel.is_cancelled());
        assert_eq!(pw.runs().await, vec!["aaa".to_string()]);
        assert_eq!(fu.invocation_count(), 1);
    }
}

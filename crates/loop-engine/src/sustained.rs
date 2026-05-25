//! Sustained-loop mode.
//!
//! Thin wrapper over the reactive orchestrator that runs for a bounded
//! wall-clock duration. The pacing is intentionally external: callers
//! provide a `tick_interval` and a closure that returns the next
//! "synthetic trigger" event (e.g. an external `CommitCompleted` from
//! a polled CI feed). The reactive harness handles the rest.

use std::sync::Arc;
use std::time::Duration;

use tokio::time;

use crate::events::LoopEvent;
use crate::orchestrator::Orchestrator;

#[derive(Debug, Clone)]
pub struct SustainedLoopConfig {
    /// How long the loop is allowed to run before forcing a halt.
    /// Default is 6 hours; the hard cap is 8 hours.
    pub duration: Duration,
    /// How often to publish the next synthetic trigger.
    pub tick_interval: Duration,
}

impl Default for SustainedLoopConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(6 * 60 * 60),
            tick_interval: Duration::from_secs(30),
        }
    }
}

/// Run the sustained loop. `next_event` is polled every `tick_interval`
/// for a fresh event to publish; if it returns `None` we skip that tick.
///
/// Returns the path to the final halt report.
pub async fn sustained_loop<F>(
    orchestrator: Arc<Orchestrator>,
    config: SustainedLoopConfig,
    mut next_event: F,
) -> Result<std::path::PathBuf, anyhow::Error>
where
    F: FnMut() -> Option<LoopEvent> + Send + 'static,
{
    let cap = Duration::from_secs(8 * 60 * 60);
    let duration = config.duration.min(cap);

    let orch_for_run = orchestrator.clone();
    let run_handle = tokio::spawn(async move { orch_for_run.run_reactive().await });

    let cancel = orchestrator.cancel_token();
    let bus = orchestrator.bus().clone();

    let pacer = {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            let mut ticker = time::interval(config.tick_interval);
            ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = ticker.tick() => {
                        if let Some(event) = next_event() {
                            bus.publish(event);
                        }
                    }
                }
            }
        })
    };

    let deadline_canceller = {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = cancel.cancelled() => {}
                _ = time::sleep(duration) => {
                    tracing::info!("sustained loop hit wall-clock cap, cancelling");
                    cancel.cancel();
                }
            }
        })
    };

    let path = run_handle.await??;
    pacer.abort();
    deadline_canceller.abort();
    Ok(path)
}

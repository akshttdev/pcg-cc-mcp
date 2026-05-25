//! Events flowing through the loop engine and the in-memory event bus.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::agents::AgentId;
use crate::circuit_breaker::{CircuitSnapshot, TripReason};

/// 16-byte BLAKE3 prefix of a normalised failure cause.
///
/// Built from `(file, error_code, top_stack_frame)` so the noisy parts of a
/// failure (timestamps, PIDs, full stack) don't fragment the bucket.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CauseHash(pub [u8; 16]);

impl CauseHash {
    /// Hash a normalised cause string. The caller is responsible for
    /// stripping volatile content (timestamps, PIDs) before calling this.
    pub fn from_normalised(s: &str) -> Self {
        let full = blake3::hash(s.as_bytes());
        let bytes = full.as_bytes();
        let mut out = [0u8; 16];
        out.copy_from_slice(&bytes[..16]);
        CauseHash(out)
    }

    pub fn as_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub name: String,
    pub message: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UxBlocker {
    pub url: String,
    pub description: String,
    pub severity: UxSeverity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UxSeverity {
    Info,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoopEvent {
    CommitCompleted {
        at: DateTime<Utc>,
        sha: String,
        author: AgentId,
        summary: String,
        loop_generated: bool,
    },
    TestsCompleted {
        at: DateTime<Utc>,
        sha: String,
        passed: bool,
        failures: Vec<TestFailure>,
    },
    UxReport {
        at: DateTime<Utc>,
        sha: String,
        blockers: Vec<UxBlocker>,
    },
    FailureDetected {
        at: DateTime<Utc>,
        sha: Option<String>,
        cause: CauseHash,
        detail: String,
        source: AgentId,
    },
    CircuitTripped {
        at: DateTime<Utc>,
        reason: TripReason,
        snapshot: CircuitSnapshot,
    },
    LoopHalted {
        at: DateTime<Utc>,
        final_report_path: PathBuf,
    },
}

impl LoopEvent {
    pub fn at(&self) -> DateTime<Utc> {
        match self {
            LoopEvent::CommitCompleted { at, .. }
            | LoopEvent::TestsCompleted { at, .. }
            | LoopEvent::UxReport { at, .. }
            | LoopEvent::FailureDetected { at, .. }
            | LoopEvent::CircuitTripped { at, .. }
            | LoopEvent::LoopHalted { at, .. } => *at,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            LoopEvent::CommitCompleted { .. } => "commit_completed",
            LoopEvent::TestsCompleted { .. } => "tests_completed",
            LoopEvent::UxReport { .. } => "ux_report",
            LoopEvent::FailureDetected { .. } => "failure_detected",
            LoopEvent::CircuitTripped { .. } => "circuit_tripped",
            LoopEvent::LoopHalted { .. } => "loop_halted",
        }
    }
}

/// Thin wrapper around `tokio::sync::broadcast` so callers don't need to
/// import the channel type directly.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<LoopEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    pub fn publish(&self, event: LoopEvent) {
        // A send error means there are no active subscribers — that's fine,
        // we don't want to block the publisher.
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LoopEvent> {
        self.tx.subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cause_hash_is_stable_and_short() {
        let a = CauseHash::from_normalised("E0432: cannot find function `foo` in `bar`");
        let b = CauseHash::from_normalised("E0432: cannot find function `foo` in `bar`");
        let c = CauseHash::from_normalised("E0277: trait `Send` not implemented");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.as_hex().len(), 32);
    }

    #[tokio::test]
    async fn bus_delivers_to_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut r1 = bus.subscribe();
        let mut r2 = bus.subscribe();

        let event = LoopEvent::CommitCompleted {
            at: Utc::now(),
            sha: "abc".into(),
            author: AgentId::Auri,
            summary: "x".into(),
            loop_generated: false,
        };
        bus.publish(event);

        let got1 = r1.recv().await.expect("r1 should receive");
        let got2 = r2.recv().await.expect("r2 should receive");
        assert_eq!(got1.kind(), "commit_completed");
        assert_eq!(got2.kind(), "commit_completed");
    }
}

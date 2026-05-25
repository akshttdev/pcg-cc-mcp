//! Circuit breaker — the Ralph-Wiggum-prevention layer.
//!
//! Pure data structure: takes events, returns decisions. Two parallel
//! detectors run on every `FailureDetected` event:
//!
//! 1. **Same-cause counter.** Trips when the same `CauseHash` has hit the
//!    configured threshold consecutively. A successful commit touching
//!    files in the cause's diff resets the counter for that cause.
//! 2. **Rolling-window counter.** Trips when total failures within the
//!    window exceed the configured cap. Entries age out by timestamp.
//!
//! Both detectors are pure with respect to `chrono::DateTime<Utc>` inputs,
//! so tests can drive time deterministically without sleeping.

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::events::{CauseHash, LoopEvent};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TripReason {
    /// Same `CauseHash` failed `same_cause_threshold` times in a row.
    RepeatedSameCause,
    /// Failures in the rolling window exceeded `rolling_threshold`.
    FailureRate,
    /// Operator manually tripped the breaker.
    ManualHalt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Consecutive same-cause failures before tripping.
    pub same_cause_threshold: u32,
    /// Total failures in `rolling_window` before tripping.
    pub rolling_threshold: u32,
    /// Width of the rolling window (default 30 minutes).
    pub rolling_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            same_cause_threshold: 3,
            rolling_threshold: 8,
            rolling_window: Duration::minutes(30),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitSnapshot {
    pub same_cause_counts: Vec<(String, u32)>,
    pub rolling_failure_count: u32,
    pub rolling_window_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Keep going.
    Closed,
    /// Trip — orchestrator should halt and write a report.
    Tripped(TripReason),
}

#[derive(Debug, Error)]
pub enum BreakerError {
    #[error("breaker already tripped: {0:?}")]
    AlreadyTripped(TripReason),
}

#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    same_cause: HashMap<CauseHash, u32>,
    rolling: VecDeque<DateTime<Utc>>,
    tripped: Option<TripReason>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            same_cause: HashMap::new(),
            rolling: VecDeque::new(),
            tripped: None,
        }
    }

    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    pub fn is_tripped(&self) -> bool {
        self.tripped.is_some()
    }

    pub fn tripped_reason(&self) -> Option<TripReason> {
        self.tripped
    }

    /// Trip the breaker manually. Idempotent — re-tripping with the same
    /// reason is a no-op; re-tripping with a different reason keeps the
    /// original.
    pub fn manual_trip(&mut self) -> Decision {
        if let Some(reason) = self.tripped {
            return Decision::Tripped(reason);
        }
        self.tripped = Some(TripReason::ManualHalt);
        Decision::Tripped(TripReason::ManualHalt)
    }

    /// Feed an event through the breaker.
    ///
    /// Only `FailureDetected` and `CommitCompleted` affect state. Other
    /// events return the current decision without mutating anything.
    pub fn observe(&mut self, event: &LoopEvent) -> Decision {
        if let Some(reason) = self.tripped {
            return Decision::Tripped(reason);
        }

        match event {
            LoopEvent::FailureDetected { at, cause, .. } => {
                self.record_failure(cause.clone(), *at)
            }
            LoopEvent::CommitCompleted { .. } => {
                // A successful commit means *something* is moving. We don't
                // know which causes it addressed, so we conservatively decay
                // the largest bucket by one — this rewards forward progress
                // without losing all signal.
                self.decay_largest_bucket();
                Decision::Closed
            }
            _ => Decision::Closed,
        }
    }

    fn record_failure(&mut self, cause: CauseHash, at: DateTime<Utc>) -> Decision {
        let entry = self.same_cause.entry(cause.clone()).or_insert(0);
        *entry += 1;
        let same_count = *entry;

        self.rolling.push_back(at);
        let window_start = at - self.config.rolling_window;
        while let Some(front) = self.rolling.front() {
            if *front < window_start {
                self.rolling.pop_front();
            } else {
                break;
            }
        }

        if same_count >= self.config.same_cause_threshold {
            self.tripped = Some(TripReason::RepeatedSameCause);
            return Decision::Tripped(TripReason::RepeatedSameCause);
        }

        if (self.rolling.len() as u32) >= self.config.rolling_threshold {
            self.tripped = Some(TripReason::FailureRate);
            return Decision::Tripped(TripReason::FailureRate);
        }

        Decision::Closed
    }

    fn decay_largest_bucket(&mut self) {
        let largest = self
            .same_cause
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(k, _)| k.clone());
        if let Some(key) = largest {
            if let Some(count) = self.same_cause.get_mut(&key) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.same_cause.remove(&key);
                }
            }
        }
    }

    pub fn snapshot(&self) -> CircuitSnapshot {
        let same_cause_counts = self
            .same_cause
            .iter()
            .map(|(k, v)| (k.as_hex(), *v))
            .collect();
        CircuitSnapshot {
            same_cause_counts,
            rolling_failure_count: self.rolling.len() as u32,
            rolling_window_seconds: self.config.rolling_window.num_seconds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentId;

    fn fail(cause: &str, at: DateTime<Utc>) -> LoopEvent {
        LoopEvent::FailureDetected {
            at,
            sha: None,
            cause: CauseHash::from_normalised(cause),
            detail: cause.into(),
            source: AgentId::Playwright,
        }
    }

    fn commit(at: DateTime<Utc>) -> LoopEvent {
        LoopEvent::CommitCompleted {
            at,
            sha: "deadbeef".into(),
            author: AgentId::Auri,
            summary: "fix".into(),
            loop_generated: false,
        }
    }

    #[test]
    fn same_cause_threshold_trips_after_n_repeats() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let t = Utc::now();
        assert_eq!(cb.observe(&fail("E1", t)), Decision::Closed);
        assert_eq!(cb.observe(&fail("E1", t)), Decision::Closed);
        assert_eq!(
            cb.observe(&fail("E1", t)),
            Decision::Tripped(TripReason::RepeatedSameCause)
        );
        // Once tripped, stays tripped.
        assert!(cb.is_tripped());
    }

    #[test]
    fn different_causes_do_not_trip_same_cause_detector() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let t = Utc::now();
        assert_eq!(cb.observe(&fail("E1", t)), Decision::Closed);
        assert_eq!(cb.observe(&fail("E2", t)), Decision::Closed);
        assert_eq!(cb.observe(&fail("E3", t)), Decision::Closed);
        assert!(!cb.is_tripped());
    }

    #[test]
    fn rolling_window_trips_on_burst_of_distinct_failures() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let t = Utc::now();
        for i in 0..7 {
            let _ = cb.observe(&fail(&format!("E{}", i), t));
        }
        assert!(!cb.is_tripped());
        assert_eq!(
            cb.observe(&fail("E7", t)),
            Decision::Tripped(TripReason::FailureRate)
        );
    }

    #[test]
    fn rolling_window_ages_out_old_failures() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let t0 = Utc::now();
        for i in 0..5 {
            let _ = cb.observe(&fail(&format!("E{}", i), t0));
        }
        // A new failure 31 minutes later should age out the originals.
        let t1 = t0 + Duration::minutes(31);
        assert_eq!(cb.observe(&fail("E_NEW", t1)), Decision::Closed);
        assert_eq!(cb.snapshot().rolling_failure_count, 1);
    }

    #[test]
    fn commit_decays_largest_bucket() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let t = Utc::now();
        let _ = cb.observe(&fail("E1", t));
        let _ = cb.observe(&fail("E1", t));
        assert!(!cb.is_tripped());
        let _ = cb.observe(&commit(t));
        // After decay, two more E1 should still not trip (count went 2→1, then 1→2, then 2→3=trip).
        assert_eq!(cb.observe(&fail("E1", t)), Decision::Closed);
        assert_eq!(cb.observe(&fail("E1", t)), Decision::Closed);
        assert_eq!(
            cb.observe(&fail("E1", t)),
            Decision::Tripped(TripReason::RepeatedSameCause)
        );
    }

    #[test]
    fn manual_trip_overrides_closed_state() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.manual_trip(), Decision::Tripped(TripReason::ManualHalt));
        let t = Utc::now();
        // Subsequent observe calls just echo the manual reason.
        assert_eq!(
            cb.observe(&fail("E1", t)),
            Decision::Tripped(TripReason::ManualHalt)
        );
    }
}

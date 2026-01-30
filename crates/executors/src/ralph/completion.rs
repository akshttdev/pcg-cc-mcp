//! Completion Detection for Ralph Loops
//!
//! Implements dual-gate completion detection:
//! 1. Completion promise (e.g., "<promise>TASK_COMPLETE</promise>")
//! 2. Exit signal (e.g., "EXIT_SIGNAL: true")
//!
//! Both must be present for true completion, preventing premature exits.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Status of completion detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum CompletionStatus {
    /// Both completion promise and exit signal found
    Complete,
    /// Only completion promise found (continue loop)
    PromiseOnly,
    /// Only exit signal found (continue loop)
    SignalOnly,
    /// Neither found (continue loop)
    Incomplete,
}

impl CompletionStatus {
    /// Returns true if the task is fully complete
    pub fn is_complete(&self) -> bool {
        matches!(self, CompletionStatus::Complete)
    }

    /// Returns true if any completion indicator was found
    pub fn has_any_indicator(&self) -> bool {
        !matches!(self, CompletionStatus::Incomplete)
    }
}

/// Configuration for completion detection
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CompletionConfig {
    /// The completion promise string to look for
    pub completion_promise: String,
    /// The exit signal string to look for
    pub exit_signal_key: String,
    /// Whether to require both signals (dual-gate) or just the promise
    pub require_dual_gate: bool,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            completion_promise: "<promise>TASK_COMPLETE</promise>".to_string(),
            exit_signal_key: "EXIT_SIGNAL: true".to_string(),
            require_dual_gate: true,
        }
    }
}

/// Detects completion signals in Claude's output
#[derive(Debug, Clone)]
pub struct CompletionDetector {
    config: CompletionConfig,
}

impl CompletionDetector {
    /// Create a new completion detector with default config
    pub fn new() -> Self {
        Self {
            config: CompletionConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: CompletionConfig) -> Self {
        Self { config }
    }

    /// Create with custom completion promise
    pub fn with_promise(promise: &str) -> Self {
        Self {
            config: CompletionConfig {
                completion_promise: promise.to_string(),
                ..Default::default()
            },
        }
    }

    /// Check if output indicates completion
    pub fn check(&self, output: &str) -> CompletionStatus {
        let has_promise = output.contains(&self.config.completion_promise);
        let has_exit_signal = output.contains(&self.config.exit_signal_key);

        match (has_promise, has_exit_signal) {
            (true, true) => CompletionStatus::Complete,
            (true, false) => {
                if self.config.require_dual_gate {
                    CompletionStatus::PromiseOnly
                } else {
                    CompletionStatus::Complete
                }
            }
            (false, true) => CompletionStatus::SignalOnly,
            (false, false) => CompletionStatus::Incomplete,
        }
    }

    /// Check if output indicates completion (simple boolean)
    pub fn is_complete(&self, output: &str) -> bool {
        self.check(output).is_complete()
    }

    /// Extract any completion-related text from output for logging
    pub fn extract_completion_context(&self, output: &str) -> Option<String> {
        // Find lines containing completion signals
        let relevant_lines: Vec<&str> = output
            .lines()
            .filter(|line| {
                line.contains(&self.config.completion_promise)
                    || line.contains(&self.config.exit_signal_key)
                    || line.to_lowercase().contains("complete")
                    || line.to_lowercase().contains("finished")
                    || line.to_lowercase().contains("done")
            })
            .take(5) // Limit to 5 lines
            .collect();

        if relevant_lines.is_empty() {
            None
        } else {
            Some(relevant_lines.join("\n"))
        }
    }
}

impl Default for CompletionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_with_both_signals() {
        let detector = CompletionDetector::new();
        let output = r#"
            Task completed successfully.
            <promise>TASK_COMPLETE</promise>
            EXIT_SIGNAL: true
        "#;
        assert_eq!(detector.check(output), CompletionStatus::Complete);
    }

    #[test]
    fn test_promise_only() {
        let detector = CompletionDetector::new();
        let output = r#"
            Task completed.
            <promise>TASK_COMPLETE</promise>
        "#;
        assert_eq!(detector.check(output), CompletionStatus::PromiseOnly);
    }

    #[test]
    fn test_signal_only() {
        let detector = CompletionDetector::new();
        let output = r#"
            EXIT_SIGNAL: true
        "#;
        assert_eq!(detector.check(output), CompletionStatus::SignalOnly);
    }

    #[test]
    fn test_incomplete() {
        let detector = CompletionDetector::new();
        let output = "Still working on the task...";
        assert_eq!(detector.check(output), CompletionStatus::Incomplete);
    }

    #[test]
    fn test_single_gate_mode() {
        let detector = CompletionDetector::with_config(CompletionConfig {
            require_dual_gate: false,
            ..Default::default()
        });
        let output = "<promise>TASK_COMPLETE</promise>";
        assert_eq!(detector.check(output), CompletionStatus::Complete);
    }

    #[test]
    fn test_custom_promise() {
        let detector = CompletionDetector::with_promise("<done>FINISHED</done>");
        let output = "<done>FINISHED</done>\nEXIT_SIGNAL: true";
        assert_eq!(detector.check(output), CompletionStatus::Complete);
    }
}

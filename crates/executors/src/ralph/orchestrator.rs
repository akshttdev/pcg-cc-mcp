//! Ralph Loop Orchestrator
//!
//! The main execution engine for Ralph Wiggum methodology.
//! Coordinates iterations, completion detection, and backpressure validation.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use ts_rs::TS;

use super::backpressure::{
    BackpressureConfig, BackpressureValidationResult, BackpressureValidator,
};
use super::completion::{CompletionConfig, CompletionDetector, CompletionStatus};
use super::prompt::{PromptConfig, RalphPromptBuilder};

use crate::executors::{ExecutorError, SpawnedChild, StandardCodingAgentExecutor};

#[derive(Debug, Error)]
pub enum RalphError {
    #[error("Executor error: {0}")]
    Executor(#[from] ExecutorError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Max iterations ({0}) reached without completion")]
    MaxIterationsReached(u32),
    #[error("Loop was cancelled")]
    Cancelled,
    #[error("Consecutive failures exceeded threshold: {0}")]
    ConsecutiveFailures(u32),
    #[error("Total timeout exceeded: {0}ms")]
    TotalTimeout(u64),
    #[error("No executor configured")]
    NoExecutor,
}

/// Result of a Ralph loop execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum RalphResult {
    /// Loop completed successfully
    Complete {
        iterations: u32,
        final_validation: Option<BackpressureValidationResult>,
    },
    /// Max iterations reached without completion
    MaxIterationsReached { iterations: u32 },
    /// Loop was cancelled
    Cancelled { iterations: u32 },
    /// Failed due to consecutive errors
    Failed {
        iterations: u32,
        error: String,
    },
}

impl RalphResult {
    pub fn is_success(&self) -> bool {
        matches!(self, RalphResult::Complete { .. })
    }

    pub fn iterations(&self) -> u32 {
        match self {
            RalphResult::Complete { iterations, .. } => *iterations,
            RalphResult::MaxIterationsReached { iterations } => *iterations,
            RalphResult::Cancelled { iterations } => *iterations,
            RalphResult::Failed { iterations, .. } => *iterations,
        }
    }
}

/// Events emitted during Ralph loop execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum RalphEvent {
    /// Loop started
    Started { max_iterations: u32 },
    /// Iteration started
    IterationStarted { iteration: u32 },
    /// Iteration completed
    IterationCompleted {
        iteration: u32,
        completion_status: CompletionStatus,
    },
    /// Backpressure validation started
    ValidationStarted { iteration: u32 },
    /// Backpressure validation completed
    ValidationCompleted {
        iteration: u32,
        all_passed: bool,
        summary: String,
    },
    /// Session ID captured
    SessionCaptured {
        iteration: u32,
        session_id: String,
    },
    /// Loop completed
    Completed { result: RalphResult },
    /// Error occurred
    Error { iteration: u32, error: String },
}

/// Configuration for Ralph loop execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RalphConfig {
    /// Maximum number of iterations
    pub max_iterations: u32,
    /// Completion detection config
    pub completion: CompletionConfig,
    /// Backpressure validation config
    pub backpressure: BackpressureConfig,
    /// Prompt building config
    pub prompt: PromptConfig,
    /// Delay between iterations in milliseconds
    pub iteration_delay_ms: u64,
    /// Maximum consecutive failures before stopping
    pub max_consecutive_failures: u32,
    /// Total timeout in milliseconds (0 = no timeout)
    pub total_timeout_ms: u64,
    /// Whether to run backpressure after each iteration
    pub validate_each_iteration: bool,
    /// Whether to run backpressure before completion
    pub validate_before_completion: bool,
}

impl Default for RalphConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            completion: CompletionConfig::default(),
            backpressure: BackpressureConfig::default(),
            prompt: PromptConfig::default(),
            iteration_delay_ms: 2000,
            max_consecutive_failures: 5,
            total_timeout_ms: 0, // No timeout by default
            validate_each_iteration: false, // Only validate on completion by default
            validate_before_completion: true,
        }
    }
}

impl RalphConfig {
    /// Create config for standard Ralph execution
    pub fn standard() -> Self {
        Self::default()
    }

    /// Create config for aggressive Ralph execution (more iterations, stricter validation)
    pub fn aggressive() -> Self {
        Self {
            max_iterations: 100,
            validate_each_iteration: true,
            ..Default::default()
        }
    }

    /// Create config for quick tasks (fewer iterations)
    pub fn quick() -> Self {
        Self {
            max_iterations: 20,
            ..Default::default()
        }
    }

    /// Set max iterations
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set backpressure commands
    pub fn with_backpressure_commands(mut self, commands: Vec<String>) -> Self {
        self.backpressure.commands = commands;
        self
    }

    /// Set agent info for prompts
    pub fn with_agent(mut self, name: &str, designation: &str) -> Self {
        self.prompt.agent_name = name.to_string();
        self.prompt.agent_designation = designation.to_string();
        self
    }

    /// Set completion promise
    pub fn with_completion_promise(mut self, promise: &str) -> Self {
        self.completion.completion_promise = promise.to_string();
        self.prompt.completion_promise = promise.to_string();
        self
    }
}

/// State tracked during Ralph loop execution
struct RalphState {
    iteration: u32,
    session_id: Option<String>,
    consecutive_failures: u32,
    last_validation: Option<BackpressureValidationResult>,
}

/// Ralph Loop Orchestrator
///
/// Coordinates the execution of tasks using the Ralph Wiggum methodology.
/// Works with any executor that implements StandardCodingAgentExecutor.
pub struct RalphOrchestrator<E: StandardCodingAgentExecutor> {
    executor: E,
    config: RalphConfig,
    completion_detector: CompletionDetector,
    backpressure_validator: BackpressureValidator,
    prompt_builder: RalphPromptBuilder,
    event_sender: Option<mpsc::Sender<RalphEvent>>,
}

impl<E: StandardCodingAgentExecutor> RalphOrchestrator<E> {
    /// Create a new orchestrator
    pub fn new(executor: E, config: RalphConfig) -> Self {
        let completion_detector = CompletionDetector::with_config(config.completion.clone());
        let backpressure_validator = BackpressureValidator::new(config.backpressure.clone());
        let prompt_builder = RalphPromptBuilder::with_config(config.prompt.clone());

        Self {
            executor,
            config,
            completion_detector,
            backpressure_validator,
            prompt_builder,
            event_sender: None,
        }
    }

    /// Set event sender for progress updates
    pub fn with_event_sender(mut self, sender: mpsc::Sender<RalphEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    /// Send an event if a sender is configured
    async fn emit(&self, event: RalphEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event).await;
        }
    }

    /// Execute the Ralph loop
    pub async fn execute(
        &self,
        working_dir: &Path,
        task_description: &str,
    ) -> Result<RalphResult, RalphError> {
        let start_time = std::time::Instant::now();

        let mut state = RalphState {
            iteration: 0,
            session_id: None,
            consecutive_failures: 0,
            last_validation: None,
        };

        self.emit(RalphEvent::Started {
            max_iterations: self.config.max_iterations,
        })
        .await;

        loop {
            // Check total timeout
            if self.config.total_timeout_ms > 0
                && start_time.elapsed().as_millis() as u64 > self.config.total_timeout_ms
            {
                return Err(RalphError::TotalTimeout(self.config.total_timeout_ms));
            }

            // Check max iterations
            if state.iteration >= self.config.max_iterations {
                let result = RalphResult::MaxIterationsReached {
                    iterations: state.iteration,
                };
                self.emit(RalphEvent::Completed {
                    result: result.clone(),
                })
                .await;
                return Ok(result);
            }

            // Check consecutive failures
            if state.consecutive_failures >= self.config.max_consecutive_failures {
                return Err(RalphError::ConsecutiveFailures(state.consecutive_failures));
            }

            state.iteration += 1;
            self.emit(RalphEvent::IterationStarted {
                iteration: state.iteration,
            })
            .await;

            // Build prompt
            let prompt = if state.iteration == 1 {
                self.prompt_builder.build_initial(task_description)
            } else if let Some(ref validation) = state.last_validation {
                if !validation.all_passed {
                    if let Some(failure_summary) = validation.failure_summary() {
                        self.prompt_builder.build_with_feedback(
                            task_description,
                            state.iteration as i32,
                            &failure_summary,
                        )
                    } else {
                        self.prompt_builder
                            .build_followup(task_description, state.iteration as i32)
                    }
                } else {
                    self.prompt_builder
                        .build_followup(task_description, state.iteration as i32)
                }
            } else {
                self.prompt_builder
                    .build_followup(task_description, state.iteration as i32)
            };

            // Execute iteration
            let output = match self.run_iteration(working_dir, &prompt, &state).await {
                Ok(output) => {
                    state.consecutive_failures = 0;
                    output
                }
                Err(e) => {
                    state.consecutive_failures += 1;
                    self.emit(RalphEvent::Error {
                        iteration: state.iteration,
                        error: e.to_string(),
                    })
                    .await;

                    // Short delay before retry
                    tokio::time::sleep(Duration::from_millis(self.config.iteration_delay_ms)).await;
                    continue;
                }
            };

            // Extract session ID for next iteration
            if let Some(session_id) = self.extract_session_id(&output) {
                self.emit(RalphEvent::SessionCaptured {
                    iteration: state.iteration,
                    session_id: session_id.clone(),
                })
                .await;
                state.session_id = Some(session_id);
            }

            // Check completion
            let completion_status = self.completion_detector.check(&output);
            self.emit(RalphEvent::IterationCompleted {
                iteration: state.iteration,
                completion_status: completion_status.clone(),
            })
            .await;

            if completion_status.is_complete() {
                // Run final validation if configured
                if self.config.validate_before_completion
                    && self.backpressure_validator.has_commands()
                {
                    self.emit(RalphEvent::ValidationStarted {
                        iteration: state.iteration,
                    })
                    .await;

                    let validation = self
                        .backpressure_validator
                        .validate(working_dir)
                        .await
                        .unwrap_or_else(|e| BackpressureValidationResult {
                            results: Default::default(),
                            all_passed: false,
                            passed_count: 0,
                            failed_count: 0,
                            total_duration_ms: 0,
                            summary: e.to_string(),
                        });

                    self.emit(RalphEvent::ValidationCompleted {
                        iteration: state.iteration,
                        all_passed: validation.all_passed,
                        summary: validation.summary.clone(),
                    })
                    .await;

                    if validation.all_passed {
                        let result = RalphResult::Complete {
                            iterations: state.iteration,
                            final_validation: Some(validation),
                        };
                        self.emit(RalphEvent::Completed {
                            result: result.clone(),
                        })
                        .await;
                        return Ok(result);
                    } else {
                        // Validation failed - continue loop
                        state.last_validation = Some(validation);
                    }
                } else {
                    // No validation configured - complete
                    let result = RalphResult::Complete {
                        iterations: state.iteration,
                        final_validation: None,
                    };
                    self.emit(RalphEvent::Completed {
                        result: result.clone(),
                    })
                    .await;
                    return Ok(result);
                }
            } else if self.config.validate_each_iteration
                && self.backpressure_validator.has_commands()
            {
                // Run validation between iterations if configured
                self.emit(RalphEvent::ValidationStarted {
                    iteration: state.iteration,
                })
                .await;

                let validation = self
                    .backpressure_validator
                    .validate(working_dir)
                    .await
                    .unwrap_or_else(|e| BackpressureValidationResult {
                        results: Default::default(),
                        all_passed: false,
                        passed_count: 0,
                        failed_count: 0,
                        total_duration_ms: 0,
                        summary: e.to_string(),
                    });

                self.emit(RalphEvent::ValidationCompleted {
                    iteration: state.iteration,
                    all_passed: validation.all_passed,
                    summary: validation.summary.clone(),
                })
                .await;

                state.last_validation = Some(validation);
            }

            // Delay between iterations
            if self.config.iteration_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.iteration_delay_ms)).await;
            }
        }
    }

    /// Run a single iteration
    async fn run_iteration(
        &self,
        working_dir: &Path,
        prompt: &str,
        state: &RalphState,
    ) -> Result<String, RalphError> {
        let child = if let Some(session_id) = &state.session_id {
            self.executor
                .spawn_follow_up(working_dir, prompt, session_id)
                .await?
        } else {
            self.executor.spawn(working_dir, prompt).await?
        };

        // Wait for completion and capture output
        let output = self.wait_for_output(child).await?;
        Ok(output)
    }

    /// Wait for child process and capture output
    async fn wait_for_output(&self, mut child: SpawnedChild) -> Result<String, RalphError> {
        use tokio::io::AsyncReadExt;

        let mut output = String::new();

        // Read stdout
        if let Some(mut stdout) = child.child.inner().stdout.take() {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).await?;
            output.push_str(&String::from_utf8_lossy(&buf));
        }

        // Wait for process to complete
        let _ = child.child.wait().await;

        Ok(output)
    }

    /// Extract session ID from Claude Code output
    fn extract_session_id(&self, output: &str) -> Option<String> {
        // Look for session ID in JSON output
        // Claude Code outputs: {"type":"system","subtype":"session_id","session_id":"..."}
        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if json.get("subtype").and_then(|v| v.as_str()) == Some("session_id") {
                    if let Some(session_id) = json.get("session_id").and_then(|v| v.as_str()) {
                        return Some(session_id.to_string());
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = RalphConfig::default();
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.iteration_delay_ms, 2000);
        assert!(config.validate_before_completion);
    }

    #[test]
    fn test_config_aggressive() {
        let config = RalphConfig::aggressive();
        assert_eq!(config.max_iterations, 100);
        assert!(config.validate_each_iteration);
    }

    #[test]
    fn test_config_quick() {
        let config = RalphConfig::quick();
        assert_eq!(config.max_iterations, 20);
    }

    #[test]
    fn test_config_builder() {
        let config = RalphConfig::default()
            .with_max_iterations(30)
            .with_agent("TestAgent", "Test Role")
            .with_completion_promise("<done>FINISHED</done>");

        assert_eq!(config.max_iterations, 30);
        assert_eq!(config.prompt.agent_name, "TestAgent");
        assert_eq!(config.prompt.agent_designation, "Test Role");
        assert_eq!(config.completion.completion_promise, "<done>FINISHED</done>");
    }

    #[test]
    fn test_ralph_result_is_success() {
        let complete = RalphResult::Complete {
            iterations: 5,
            final_validation: None,
        };
        assert!(complete.is_success());

        let max_reached = RalphResult::MaxIterationsReached { iterations: 50 };
        assert!(!max_reached.is_success());
    }
}

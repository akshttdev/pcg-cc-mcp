//! Backpressure Validation for Ralph Loops
//!
//! Runs validation commands (tests, lints, type checks) between iterations
//! to ensure code quality and provide feedback to the next iteration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::process::Command;
use ts_rs::TS;

#[derive(Debug, Error)]
pub enum BackpressureError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command timed out after {0}ms")]
    Timeout(u64),
    #[error("All commands failed")]
    AllFailed,
}

/// Result of a single validation command
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CommandResult {
    pub command: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Result of all backpressure validation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BackpressureValidationResult {
    /// All validation results by command
    pub results: HashMap<String, CommandResult>,
    /// Whether all commands passed
    pub all_passed: bool,
    /// Number of commands that passed
    pub passed_count: usize,
    /// Number of commands that failed
    pub failed_count: usize,
    /// Total time for all validations
    pub total_duration_ms: u64,
    /// Summary for logging
    pub summary: String,
}

impl BackpressureValidationResult {
    /// Get a brief status string
    pub fn status_string(&self) -> String {
        if self.all_passed {
            format!("PASS ({}/{} commands)", self.passed_count, self.results.len())
        } else {
            format!(
                "FAIL ({}/{} passed)",
                self.passed_count,
                self.results.len()
            )
        }
    }

    /// Get failed command names
    pub fn failed_commands(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|(_, r)| !r.passed)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get a summary of failures for feedback
    pub fn failure_summary(&self) -> Option<String> {
        if self.all_passed {
            return None;
        }

        let failures: Vec<String> = self
            .results
            .iter()
            .filter(|(_, r)| !r.passed)
            .map(|(cmd, r)| {
                let output = if !r.stderr.is_empty() {
                    // Truncate stderr to reasonable length
                    let stderr = if r.stderr.len() > 500 {
                        format!("{}...[truncated]", &r.stderr[..500])
                    } else {
                        r.stderr.clone()
                    };
                    format!("  stderr: {}", stderr.trim())
                } else if !r.stdout.is_empty() {
                    let stdout = if r.stdout.len() > 500 {
                        format!("{}...[truncated]", &r.stdout[..500])
                    } else {
                        r.stdout.clone()
                    };
                    format!("  stdout: {}", stdout.trim())
                } else {
                    "  (no output)".to_string()
                };
                format!("- {} (exit code: {:?})\n{}", cmd, r.exit_code, output)
            })
            .collect();

        Some(failures.join("\n\n"))
    }
}

/// Configuration for backpressure validation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BackpressureConfig {
    /// Commands to run for validation
    pub commands: Vec<String>,
    /// Timeout per command in milliseconds
    pub timeout_ms: u64,
    /// Whether to fail if any command fails (vs. threshold)
    pub fail_on_any: bool,
    /// Minimum number of commands that must pass (if fail_on_any is false)
    pub min_pass_count: usize,
    /// Whether to run commands in parallel
    pub run_parallel: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            commands: vec![],
            timeout_ms: 300_000, // 5 minutes
            fail_on_any: true,
            min_pass_count: 0,
            run_parallel: false,
        }
    }
}

impl BackpressureConfig {
    /// Create config for Rust projects
    pub fn rust_standard() -> Self {
        Self {
            commands: vec![
                "cargo test --quiet".to_string(),
                "cargo clippy --quiet -- -D warnings".to_string(),
            ],
            ..Default::default()
        }
    }

    /// Create config for Node.js projects
    pub fn node_standard() -> Self {
        Self {
            commands: vec!["npm test".to_string(), "npm run lint".to_string()],
            ..Default::default()
        }
    }

    /// Create config for Python projects
    pub fn python_standard() -> Self {
        Self {
            commands: vec!["pytest".to_string(), "ruff check .".to_string()],
            ..Default::default()
        }
    }
}

/// Backpressure validator that runs validation commands
pub struct BackpressureValidator {
    config: BackpressureConfig,
}

impl BackpressureValidator {
    /// Create a new validator with the given config
    pub fn new(config: BackpressureConfig) -> Self {
        Self { config }
    }

    /// Create with default config
    pub fn with_commands(commands: Vec<String>) -> Self {
        Self {
            config: BackpressureConfig {
                commands,
                ..Default::default()
            },
        }
    }

    /// Run all validation commands
    pub async fn validate(
        &self,
        working_dir: &Path,
    ) -> Result<BackpressureValidationResult, BackpressureError> {
        let start = Instant::now();
        let mut results = HashMap::new();

        if self.config.run_parallel {
            // Run commands in parallel
            let futures: Vec<_> = self
                .config
                .commands
                .iter()
                .map(|cmd| self.run_command(cmd, working_dir))
                .collect();

            let outputs = futures::future::join_all(futures).await;

            for (cmd, result) in self.config.commands.iter().zip(outputs) {
                match result {
                    Ok(r) => {
                        results.insert(cmd.clone(), r);
                    }
                    Err(e) => {
                        results.insert(
                            cmd.clone(),
                            CommandResult {
                                command: cmd.clone(),
                                passed: false,
                                exit_code: None,
                                stdout: String::new(),
                                stderr: e.to_string(),
                                duration_ms: 0,
                            },
                        );
                    }
                }
            }
        } else {
            // Run commands sequentially
            for cmd in &self.config.commands {
                match self.run_command(cmd, working_dir).await {
                    Ok(result) => {
                        results.insert(cmd.clone(), result);
                    }
                    Err(e) => {
                        results.insert(
                            cmd.clone(),
                            CommandResult {
                                command: cmd.clone(),
                                passed: false,
                                exit_code: None,
                                stdout: String::new(),
                                stderr: e.to_string(),
                                duration_ms: 0,
                            },
                        );
                    }
                }
            }
        }

        let passed_count = results.values().filter(|r| r.passed).count();
        let failed_count = results.len() - passed_count;

        let all_passed = if self.config.fail_on_any {
            failed_count == 0
        } else {
            passed_count >= self.config.min_pass_count
        };

        let summary = if all_passed {
            format!(
                "All {} validation commands passed",
                results.len()
            )
        } else {
            let failed_cmds: Vec<_> = results
                .iter()
                .filter(|(_, r)| !r.passed)
                .map(|(cmd, _)| cmd.as_str())
                .collect();
            format!(
                "{} of {} commands failed: {}",
                failed_count,
                results.len(),
                failed_cmds.join(", ")
            )
        };

        Ok(BackpressureValidationResult {
            results,
            all_passed,
            passed_count,
            failed_count,
            total_duration_ms: start.elapsed().as_millis() as u64,
            summary,
        })
    }

    /// Run a single command
    async fn run_command(
        &self,
        cmd: &str,
        working_dir: &Path,
    ) -> Result<CommandResult, BackpressureError> {
        let start = Instant::now();

        let output = tokio::time::timeout(
            Duration::from_millis(self.config.timeout_ms),
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| BackpressureError::Timeout(self.config.timeout_ms))?
        .map_err(BackpressureError::Io)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(CommandResult {
            command: cmd.to_string(),
            passed: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
        })
    }

    /// Check if any commands are configured
    pub fn has_commands(&self) -> bool {
        !self.config.commands.is_empty()
    }
}

/// Detect project type from working directory
pub fn detect_project_type(working_dir: &Path) -> Option<&'static str> {
    if working_dir.join("Cargo.toml").exists() {
        Some("rust")
    } else if working_dir.join("package.json").exists() {
        Some("node")
    } else if working_dir.join("pyproject.toml").exists()
        || working_dir.join("setup.py").exists()
        || working_dir.join("requirements.txt").exists()
    {
        Some("python")
    } else if working_dir.join("go.mod").exists() {
        Some("go")
    } else {
        None
    }
}

/// Get default backpressure commands for a project type
pub fn default_commands_for_project_type(project_type: &str) -> Vec<String> {
    match project_type {
        "rust" => vec![
            "cargo test --quiet".to_string(),
            "cargo clippy --quiet -- -D warnings".to_string(),
        ],
        "node" => vec!["npm test".to_string()],
        "python" => vec!["pytest".to_string()],
        "go" => vec!["go test ./...".to_string()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_simple_command() {
        let validator = BackpressureValidator::with_commands(vec!["echo hello".to_string()]);
        let result = validator.validate(Path::new(".")).await.unwrap();
        assert!(result.all_passed);
        assert_eq!(result.passed_count, 1);
    }

    #[tokio::test]
    async fn test_failing_command() {
        let validator = BackpressureValidator::with_commands(vec!["exit 1".to_string()]);
        let result = validator.validate(Path::new(".")).await.unwrap();
        assert!(!result.all_passed);
        assert_eq!(result.failed_count, 1);
    }

    #[tokio::test]
    async fn test_mixed_commands() {
        let validator = BackpressureValidator::with_commands(vec![
            "echo pass".to_string(),
            "exit 1".to_string(),
        ]);
        let result = validator.validate(Path::new(".")).await.unwrap();
        assert!(!result.all_passed);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.failed_count, 1);
    }

    #[test]
    fn test_detect_project_type() {
        // This test depends on the current directory having a Cargo.toml
        let cwd = env::current_dir().unwrap();
        if cwd.join("Cargo.toml").exists() {
            assert_eq!(detect_project_type(&cwd), Some("rust"));
        }
    }
}

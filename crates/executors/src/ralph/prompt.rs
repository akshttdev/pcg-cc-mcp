//! Prompt Builder for Ralph Loops
//!
//! Constructs prompts for different phases of Ralph loop execution:
//! - Initial prompt with full task context
//! - Follow-up prompts for subsequent iterations
//! - Agent-specific customization

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Configuration for prompt building
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PromptConfig {
    /// Agent name for personalization
    pub agent_name: String,
    /// Agent designation/role
    pub agent_designation: String,
    /// Completion promise to include in prompt
    pub completion_promise: String,
    /// Exit signal key
    pub exit_signal_key: String,
    /// Custom system prompt prefix
    pub system_prefix: Option<String>,
    /// Custom system prompt suffix
    pub system_suffix: Option<String>,
    /// Backpressure commands to mention
    pub backpressure_commands: Vec<String>,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            agent_name: "AURI".to_string(),
            agent_designation: "Master Developer Architect".to_string(),
            completion_promise: "<promise>TASK_COMPLETE</promise>".to_string(),
            exit_signal_key: "EXIT_SIGNAL: true".to_string(),
            system_prefix: None,
            system_suffix: None,
            backpressure_commands: vec![],
        }
    }
}

/// Builder for Ralph loop prompts
pub struct RalphPromptBuilder {
    config: PromptConfig,
}

impl RalphPromptBuilder {
    /// Create a new prompt builder with default config
    pub fn new() -> Self {
        Self {
            config: PromptConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: PromptConfig) -> Self {
        Self { config }
    }

    /// Set agent name
    pub fn agent_name(mut self, name: &str) -> Self {
        self.config.agent_name = name.to_string();
        self
    }

    /// Set agent designation
    pub fn agent_designation(mut self, designation: &str) -> Self {
        self.config.agent_designation = designation.to_string();
        self
    }

    /// Set completion promise
    pub fn completion_promise(mut self, promise: &str) -> Self {
        self.config.completion_promise = promise.to_string();
        self
    }

    /// Set backpressure commands
    pub fn backpressure_commands(mut self, commands: Vec<String>) -> Self {
        self.config.backpressure_commands = commands;
        self
    }

    /// Set system prefix
    pub fn system_prefix(mut self, prefix: &str) -> Self {
        self.config.system_prefix = Some(prefix.to_string());
        self
    }

    /// Set system suffix
    pub fn system_suffix(mut self, suffix: &str) -> Self {
        self.config.system_suffix = Some(suffix.to_string());
        self
    }

    /// Build the initial prompt for the first iteration
    pub fn build_initial(&self, task_description: &str) -> String {
        let backpressure_section = if !self.config.backpressure_commands.is_empty() {
            format!(
                r#"
## Validation Commands
The following commands will be run to validate your work:
{}

Ensure these pass before signaling completion.
"#,
                self.config
                    .backpressure_commands
                    .iter()
                    .map(|c| format!("- `{}`", c))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            String::new()
        };

        let prefix = self
            .config
            .system_prefix
            .as_ref()
            .map(|p| format!("{}\n\n", p))
            .unwrap_or_default();

        let suffix = self
            .config
            .system_suffix
            .as_ref()
            .map(|s| format!("\n\n{}", s))
            .unwrap_or_default();

        format!(
            r#"{prefix}You are {agent_name}, {agent_designation}.

## Execution Protocol
You are operating in Ralph Wiggum loop mode - an autonomous, iterative execution pattern.
Each iteration builds on the previous. Your changes persist between iterations.

## Current Task
{task_description}
{backpressure_section}
## Workflow
1. Analyze the current state of the codebase
2. Plan your approach (create/update IMPLEMENTATION_PLAN.md for complex tasks)
3. Implement incrementally with tests
4. Run validation if possible
5. Commit working changes with clear messages

## Completion Protocol
When the task is FULLY COMPLETE with all tests passing:
1. Output: {completion_promise}
2. Output: {exit_signal_key}

Both signals are required to end the loop.

## If Stuck or Encountering Errors
- Document blockers clearly
- Do NOT output completion signals
- The loop will restart with fresh context but your file changes persist

## Guidelines
- Architecture First - understand before implementing
- Write tests before implementation when feasible
- Keep changes atomic and focused
- Commit frequently with descriptive messages
- No improvisational tech debt
- Modular by default{suffix}"#,
            prefix = prefix,
            suffix = suffix,
            agent_name = self.config.agent_name,
            agent_designation = self.config.agent_designation,
            task_description = task_description,
            backpressure_section = backpressure_section,
            completion_promise = self.config.completion_promise,
            exit_signal_key = self.config.exit_signal_key,
        )
    }

    /// Build a follow-up prompt for subsequent iterations
    pub fn build_followup(&self, task_description: &str, iteration: i32) -> String {
        format!(
            r#"You are {agent_name}, {agent_designation}.

## Ralph Loop - Iteration {iteration}
You are continuing work on this task. Your previous changes are saved.

## Task
{task_description}

## Continue From Where You Left Off
1. Check the current state: `git status`, `git log -3`
2. Review IMPLEMENTATION_PLAN.md if it exists
3. Continue implementing the next step
4. Run tests to verify your changes

## Completion Protocol
When FULLY COMPLETE:
1. Output: {completion_promise}
2. Output: {exit_signal_key}

If not complete, continue working. The loop will restart if needed."#,
            agent_name = self.config.agent_name,
            agent_designation = self.config.agent_designation,
            iteration = iteration,
            task_description = task_description,
            completion_promise = self.config.completion_promise,
            exit_signal_key = self.config.exit_signal_key,
        )
    }

    /// Build a prompt that includes failure feedback
    pub fn build_with_feedback(
        &self,
        task_description: &str,
        iteration: i32,
        failure_summary: &str,
    ) -> String {
        format!(
            r#"You are {agent_name}, {agent_designation}.

## Ralph Loop - Iteration {iteration}
The previous iteration's validation failed. Please address the issues below.

## Task
{task_description}

## Validation Failures
```
{failure_summary}
```

## Your Mission
1. Review the failures above
2. Fix the issues
3. Run validation to verify fixes
4. Continue with remaining work if any

## Completion Protocol
When FULLY COMPLETE with ALL validations passing:
1. Output: {completion_promise}
2. Output: {exit_signal_key}

Do NOT signal completion if there are still failing tests or validations."#,
            agent_name = self.config.agent_name,
            agent_designation = self.config.agent_designation,
            iteration = iteration,
            task_description = task_description,
            failure_summary = failure_summary,
            completion_promise = self.config.completion_promise,
            exit_signal_key = self.config.exit_signal_key,
        )
    }
}

impl Default for RalphPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_initial_prompt() {
        let builder = RalphPromptBuilder::new()
            .agent_name("TestAgent")
            .agent_designation("Test Role");

        let prompt = builder.build_initial("Implement feature X");

        assert!(prompt.contains("TestAgent"));
        assert!(prompt.contains("Test Role"));
        assert!(prompt.contains("Implement feature X"));
        assert!(prompt.contains("<promise>TASK_COMPLETE</promise>"));
        assert!(prompt.contains("EXIT_SIGNAL: true"));
    }

    #[test]
    fn test_build_with_backpressure() {
        let builder = RalphPromptBuilder::new()
            .backpressure_commands(vec!["cargo test".to_string(), "cargo clippy".to_string()]);

        let prompt = builder.build_initial("Task");

        assert!(prompt.contains("cargo test"));
        assert!(prompt.contains("cargo clippy"));
        assert!(prompt.contains("Validation Commands"));
    }

    #[test]
    fn test_build_followup() {
        let builder = RalphPromptBuilder::new();
        let prompt = builder.build_followup("Task", 3);

        assert!(prompt.contains("Iteration 3"));
        assert!(prompt.contains("Continue From Where You Left Off"));
    }

    #[test]
    fn test_build_with_feedback() {
        let builder = RalphPromptBuilder::new();
        let prompt = builder.build_with_feedback("Task", 2, "Test failed: expected 5, got 3");

        assert!(prompt.contains("Iteration 2"));
        assert!(prompt.contains("Validation Failures"));
        assert!(prompt.contains("expected 5, got 3"));
    }
}

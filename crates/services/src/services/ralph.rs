//! Ralph Wiggum Service
//!
//! Manages Ralph loop executions for autonomous task completion.
//! Integrates with the container service and agent execution configs.

use std::path::PathBuf;

use db::{
    DBService,
    models::{
        agent::Agent,
        agent_execution_config::{
            AgentExecutionConfig, AgentExecutionConfigError, AgentExecutionProfile,
            BackpressureDefinition, CreateRalphLoopState, ExecutionMode, RalphIteration,
            RalphLoopState,
        },
        task::Task,
        task_attempt::TaskAttempt,
    },
};
use executors::{
    executors::claude::ClaudeCode,
    ralph::{
        BackpressureConfig, CompletionConfig,
        RalphConfig, RalphOrchestrator, RalphResult,
    },
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use ts_rs::TS;
use uuid::Uuid;

use super::git::GitService;

#[derive(Debug, Error)]
pub enum RalphServiceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Agent execution config error: {0}")]
    AgentExecutionConfig(#[from] AgentExecutionConfigError),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),
    #[error("Task attempt not found: {0}")]
    TaskAttemptNotFound(Uuid),
    #[error("Working directory not found")]
    WorkingDirNotFound,
    #[error("Ralph execution error: {0}")]
    ExecutionError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

/// Event emitted during Ralph execution for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum RalphServiceEvent {
    /// Ralph loop started
    Started {
        ralph_loop_id: String,
        task_attempt_id: Uuid,
        max_iterations: u32,
    },
    /// Iteration started
    IterationStarted {
        ralph_loop_id: String,
        iteration: u32,
    },
    /// Iteration completed
    IterationCompleted {
        ralph_loop_id: String,
        iteration: u32,
        completion_detected: bool,
    },
    /// Validation running
    ValidationStarted {
        ralph_loop_id: String,
        iteration: u32,
    },
    /// Validation completed
    ValidationCompleted {
        ralph_loop_id: String,
        iteration: u32,
        all_passed: bool,
    },
    /// Loop completed
    Completed {
        ralph_loop_id: String,
        result: RalphResult,
    },
    /// Error occurred
    Error {
        ralph_loop_id: String,
        iteration: u32,
        error: String,
    },
}

/// Request to start a Ralph loop execution
#[derive(Debug, Deserialize, TS)]
pub struct StartRalphRequest {
    pub task_attempt_id: Uuid,
    /// Override max iterations (uses agent config default if not provided)
    pub max_iterations: Option<u32>,
    /// Override backpressure commands
    pub backpressure_commands: Option<Vec<String>>,
    /// Override completion promise
    pub completion_promise: Option<String>,
}

/// Ralph Service for managing loop executions
#[derive(Clone)]
pub struct RalphService {
    db: DBService,
    git: GitService,
}

impl RalphService {
    /// Create a new Ralph service
    pub fn new(db: DBService, git: GitService) -> Self {
        Self { db, git }
    }

    /// Resolve execution config for a task
    pub async fn resolve_config(
        &self,
        task: &Task,
    ) -> Result<ResolvedRalphConfig, RalphServiceError> {
        // 1. Check if task has agent assigned
        let agent = if let Some(agent_id) = task.agent_id {
            Agent::find_by_id(&self.db.pool, agent_id).await?
        } else if let Some(agent_name) = &task.assigned_agent {
            Agent::find_by_short_name(&self.db.pool, agent_name).await?
        } else {
            None
        };

        // 2. Get agent execution config if agent exists
        let (agent_config, profile) = if let Some(ref agent) = agent {
            let config = AgentExecutionConfig::find_by_agent_id(&self.db.pool, agent.id).await?;
            let profile = if let Some(ref c) = config {
                if let Some(ref profile_id) = c.execution_profile_id {
                    AgentExecutionProfile::find_by_id(&self.db.pool, profile_id).await?
                } else {
                    None
                }
            } else {
                None
            };
            (config, profile)
        } else {
            (None, None)
        };

        // 3. Determine execution mode
        let execution_mode = agent_config
            .as_ref()
            .and_then(|c| c.execution_mode_override.clone())
            .or_else(|| profile.as_ref().map(|p| p.execution_mode.clone()))
            .unwrap_or(ExecutionMode::Standard);

        // 4. Build resolved config
        let max_iterations = agent_config
            .as_ref()
            .and_then(|c| c.max_iterations_override)
            .or_else(|| profile.as_ref().and_then(|p| p.max_iterations))
            .unwrap_or(50) as u32;

        let completion_promise = profile
            .as_ref()
            .and_then(|p| p.completion_promise.clone())
            .unwrap_or_else(|| "<promise>TASK_COMPLETE</promise>".to_string());

        let backpressure_commands = agent_config
            .as_ref()
            .and_then(|c| {
                c.backpressure_commands_override
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
            })
            .or_else(|| profile.as_ref().map(|p| p.get_backpressure_commands()))
            .unwrap_or_default();

        let system_prefix = agent_config
            .as_ref()
            .and_then(|c| c.system_prompt_prefix.clone());

        let system_suffix = agent_config
            .as_ref()
            .and_then(|c| c.system_prompt_suffix.clone());

        Ok(ResolvedRalphConfig {
            execution_mode,
            agent_name: agent.as_ref().map(|a| a.short_name.clone()),
            agent_designation: agent.as_ref().map(|a| a.designation.clone()),
            max_iterations,
            completion_promise,
            backpressure_commands,
            system_prefix,
            system_suffix,
        })
    }

    /// Check if a task should use Ralph mode
    pub async fn should_use_ralph(&self, task: &Task) -> Result<bool, RalphServiceError> {
        let config = self.resolve_config(task).await?;
        Ok(matches!(config.execution_mode, ExecutionMode::Ralph))
    }

    /// Start a Ralph loop execution
    pub async fn start_ralph_loop(
        &self,
        request: StartRalphRequest,
        event_sender: Option<mpsc::Sender<RalphServiceEvent>>,
    ) -> Result<RalphLoopState, RalphServiceError> {
        // Get task attempt
        let task_attempt = TaskAttempt::find_by_id(&self.db.pool, request.task_attempt_id)
            .await?
            .ok_or(RalphServiceError::TaskAttemptNotFound(request.task_attempt_id))?;

        // Get task
        let task = task_attempt
            .parent_task(&self.db.pool)
            .await?
            .ok_or(RalphServiceError::TaskNotFound(request.task_attempt_id))?;

        // Resolve config
        let mut resolved = self.resolve_config(&task).await?;

        // Apply overrides from request
        if let Some(max_iter) = request.max_iterations {
            resolved.max_iterations = max_iter;
        }
        if let Some(commands) = request.backpressure_commands {
            resolved.backpressure_commands = commands;
        }
        if let Some(promise) = request.completion_promise {
            resolved.completion_promise = promise;
        }

        // Get agent ID if available
        let agent_id = if let Some(agent_name) = &resolved.agent_name {
            Agent::find_by_short_name(&self.db.pool, agent_name)
                .await?
                .map(|a| a.id)
        } else {
            None
        };

        // Create Ralph loop state record
        let loop_state = RalphLoopState::create(
            &self.db.pool,
            CreateRalphLoopState {
                task_attempt_id: task_attempt.id,
                agent_id,
                max_iterations: resolved.max_iterations as i32,
                completion_promise: Some(resolved.completion_promise.clone()),
            },
        )
        .await
        .map_err(|e| RalphServiceError::Database(sqlx::Error::Protocol(e.to_string())))?;

        // Emit started event
        if let Some(ref sender) = event_sender {
            let _ = sender
                .send(RalphServiceEvent::Started {
                    ralph_loop_id: loop_state.id.clone(),
                    task_attempt_id: task_attempt.id,
                    max_iterations: resolved.max_iterations,
                })
                .await;
        }

        // Get working directory
        let working_dir = task_attempt
            .container_ref
            .as_ref()
            .map(PathBuf::from)
            .ok_or(RalphServiceError::WorkingDirNotFound)?;

        // Spawn the Ralph loop execution in background
        let loop_id = loop_state.id.clone();
        let db = self.db.clone();
        let task_prompt = task.to_prompt();

        tokio::spawn(async move {
            let result =
                Self::run_ralph_loop(db, loop_id, working_dir, task_prompt, resolved, event_sender)
                    .await;

            if let Err(e) = result {
                tracing::error!("Ralph loop failed: {}", e);
            }
        });

        Ok(loop_state)
    }

    /// Run the Ralph loop (internal)
    async fn run_ralph_loop(
        db: DBService,
        loop_id: String,
        working_dir: PathBuf,
        task_prompt: String,
        config: ResolvedRalphConfig,
        event_sender: Option<mpsc::Sender<RalphServiceEvent>>,
    ) -> Result<RalphResult, RalphServiceError> {
        // Build Ralph config
        let ralph_config = RalphConfig {
            max_iterations: config.max_iterations,
            completion: CompletionConfig {
                completion_promise: config.completion_promise.clone(),
                exit_signal_key: "EXIT_SIGNAL: true".to_string(),
                require_dual_gate: true,
            },
            backpressure: BackpressureConfig {
                commands: config.backpressure_commands.clone(),
                timeout_ms: 300_000,
                fail_on_any: true,
                min_pass_count: 0,
                run_parallel: false,
            },
            prompt: executors::ralph::prompt::PromptConfig {
                agent_name: config.agent_name.clone().unwrap_or_else(|| "AURI".to_string()),
                agent_designation: config
                    .agent_designation
                    .clone()
                    .unwrap_or_else(|| "Master Developer Architect".to_string()),
                completion_promise: config.completion_promise.clone(),
                exit_signal_key: "EXIT_SIGNAL: true".to_string(),
                system_prefix: config.system_prefix.clone(),
                system_suffix: config.system_suffix.clone(),
                backpressure_commands: config.backpressure_commands.clone(),
            },
            iteration_delay_ms: 2000,
            max_consecutive_failures: 5,
            total_timeout_ms: 0,
            validate_each_iteration: false,
            validate_before_completion: !config.backpressure_commands.is_empty(),
        };

        // Get Claude Code executor
        let executor = ClaudeCode {
            append_prompt: Default::default(),
            claude_code_router: None,
            plan: None,
            approvals: None,
            model: None,
            dangerously_skip_permissions: Some(true), // Ralph runs autonomously
            cmd: Default::default(),
        };

        // Create orchestrator
        let orchestrator = RalphOrchestrator::new(executor, ralph_config);

        // Run the loop
        let result = orchestrator.execute(&working_dir, &task_prompt).await;

        // Update loop state based on result
        match &result {
            Ok(r) => {
                match r {
                    RalphResult::Complete { .. } => {
                        let _ =
                            RalphLoopState::mark_complete(&db.pool, &loop_id, true).await;
                    }
                    RalphResult::MaxIterationsReached { .. } => {
                        let _ = RalphLoopState::mark_max_reached(&db.pool, &loop_id).await;
                    }
                    RalphResult::Failed { error, .. } => {
                        let _ = RalphLoopState::record_error(&db.pool, &loop_id, error).await;
                        let _ =
                            RalphLoopState::mark_complete(&db.pool, &loop_id, false).await;
                    }
                    RalphResult::Cancelled { .. } => {
                        // Mark as cancelled in DB
                        let _ =
                            RalphLoopState::mark_complete(&db.pool, &loop_id, false).await;
                    }
                }

                // Emit completed event
                if let Some(sender) = event_sender {
                    let _ = sender
                        .send(RalphServiceEvent::Completed {
                            ralph_loop_id: loop_id,
                            result: r.clone(),
                        })
                        .await;
                }
            }
            Err(e) => {
                let _ = RalphLoopState::record_error(&db.pool, &loop_id, &e.to_string()).await;
                let _ = RalphLoopState::mark_complete(&db.pool, &loop_id, false).await;

                if let Some(sender) = event_sender {
                    let _ = sender
                        .send(RalphServiceEvent::Error {
                            ralph_loop_id: loop_id,
                            iteration: 0,
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        }

        result.map_err(|e| RalphServiceError::ExecutionError(e.to_string()))
    }

    /// Get Ralph loop state by ID
    pub async fn get_loop_state(
        &self,
        loop_id: &str,
    ) -> Result<Option<RalphLoopState>, RalphServiceError> {
        RalphLoopState::find_by_id(&self.db.pool, loop_id)
            .await
            .map_err(|e| RalphServiceError::Database(sqlx::Error::Protocol(e.to_string())))
    }

    /// Get Ralph loop state by task attempt
    pub async fn get_loop_state_by_attempt(
        &self,
        task_attempt_id: Uuid,
    ) -> Result<Option<RalphLoopState>, RalphServiceError> {
        RalphLoopState::find_by_task_attempt(&self.db.pool, task_attempt_id)
            .await
            .map_err(|e| RalphServiceError::Database(sqlx::Error::Protocol(e.to_string())))
    }

    /// Get iterations for a Ralph loop
    pub async fn get_iterations(
        &self,
        loop_id: &str,
    ) -> Result<Vec<RalphIteration>, RalphServiceError> {
        RalphIteration::find_by_loop(&self.db.pool, loop_id)
            .await
            .map_err(|e| RalphServiceError::Database(sqlx::Error::Protocol(e.to_string())))
    }

    /// Cancel a running Ralph loop
    pub async fn cancel_loop(&self, loop_id: &str) -> Result<(), RalphServiceError> {
        // Update status to cancelled
        sqlx::query(
            "UPDATE ralph_loop_state SET status = 'cancelled', completed_at = datetime('now', 'subsec') WHERE id = ?1 AND status = 'running'",
        )
        .bind(loop_id)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Get all execution profiles
    pub async fn get_execution_profiles(
        &self,
    ) -> Result<Vec<AgentExecutionProfile>, RalphServiceError> {
        AgentExecutionProfile::list_all(&self.db.pool)
            .await
            .map_err(|e| RalphServiceError::Database(sqlx::Error::Protocol(e.to_string())))
    }

    /// Get backpressure definitions for a project type
    pub async fn get_backpressure_definitions(
        &self,
        project_type: &str,
    ) -> Result<Vec<BackpressureDefinition>, RalphServiceError> {
        BackpressureDefinition::find_for_project_type(&self.db.pool, project_type)
            .await
            .map_err(|e| RalphServiceError::Database(sqlx::Error::Protocol(e.to_string())))
    }
}

/// Resolved configuration for Ralph execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ResolvedRalphConfig {
    pub execution_mode: ExecutionMode,
    pub agent_name: Option<String>,
    pub agent_designation: Option<String>,
    pub max_iterations: u32,
    pub completion_promise: String,
    pub backpressure_commands: Vec<String>,
    pub system_prefix: Option<String>,
    pub system_suffix: Option<String>,
}

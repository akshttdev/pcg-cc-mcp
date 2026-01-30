//! Agent Execution Configuration Models
//!
//! Scalable system for configuring execution behavior for any agent.
//! Supports Ralph Wiggum loop methodology and other execution patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

// ============================================================================
// ENUMS
// ============================================================================

/// Execution mode for agents
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, Default)]
#[sqlx(type_name = "execution_mode", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Single execution without looping (current default behavior)
    #[default]
    Standard,
    /// Ralph Wiggum loop - iterative execution until completion
    Ralph,
    /// Parallel execution of multiple sub-tasks
    Parallel,
    /// Pipeline execution - sequential stages
    Pipeline,
}

/// Context window strategy for iterative execution
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, Default)]
#[sqlx(type_name = "context_window_strategy", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ContextWindowStrategy {
    /// Fresh context each iteration (uses session follow-up)
    #[default]
    Fresh,
    /// Cumulative context (grows with each iteration)
    Cumulative,
    /// Sliding window (keeps last N iterations)
    Sliding,
}

/// Ralph loop status
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, Default)]
#[sqlx(type_name = "ralph_loop_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RalphLoopStatus {
    #[default]
    Initializing,
    Running,
    Validating,
    Complete,
    MaxReached,
    Failed,
    Cancelled,
}

/// Ralph iteration status
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, Default)]
#[sqlx(type_name = "ralph_iteration_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RalphIterationStatus {
    #[default]
    Running,
    Completed,
    Failed,
    Timeout,
}

// ============================================================================
// ERRORS
// ============================================================================

#[derive(Debug, Error)]
pub enum AgentExecutionConfigError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),
    #[error("Config not found for agent: {0}")]
    ConfigNotFound(String),
    #[error("Ralph loop not found: {0}")]
    RalphLoopNotFound(Uuid),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================================
// AGENT EXECUTION PROFILE
// ============================================================================

/// Prompt templates for different execution phases
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
pub struct PromptTemplates {
    /// System prompt for initial execution
    #[serde(default)]
    pub system: Option<String>,
    /// Prompt for planning phase
    #[serde(default)]
    pub planning: Option<String>,
    /// Prompt for building/implementation phase
    #[serde(default)]
    pub building: Option<String>,
    /// Prompt for follow-up iterations
    #[serde(default)]
    pub followup: Option<String>,
}

/// Reusable execution configuration template
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AgentExecutionProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,

    // Execution Mode
    pub execution_mode: ExecutionMode,

    // Ralph Loop Configuration
    pub max_iterations: Option<i32>,
    pub completion_promise: Option<String>,
    pub exit_signal_key: Option<String>,

    // Backpressure Configuration (JSON array)
    pub backpressure_commands: Option<String>,
    pub backpressure_fail_threshold: Option<i32>,

    // Timing Configuration
    pub iteration_delay_ms: Option<i32>,
    pub iteration_timeout_ms: Option<i32>,
    pub total_timeout_ms: Option<i32>,

    // Context Management
    pub preserve_session: Option<bool>,
    pub context_window_strategy: Option<ContextWindowStrategy>,

    // Prompt Templates (JSON)
    pub prompt_templates: Option<String>,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

impl AgentExecutionProfile {
    /// Get prompt templates as struct
    pub fn get_prompt_templates(&self) -> PromptTemplates {
        self.prompt_templates
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Get backpressure commands as Vec
    pub fn get_backpressure_commands(&self) -> Vec<String> {
        self.backpressure_commands
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Find profile by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<Option<Self>, AgentExecutionConfigError> {
        let profile = sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_execution_profiles WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(profile)
    }

    /// Find profile by name
    pub async fn find_by_name(
        pool: &SqlitePool,
        name: &str,
    ) -> Result<Option<Self>, AgentExecutionConfigError> {
        let profile = sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_execution_profiles WHERE name = ?1",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        Ok(profile)
    }

    /// List all profiles
    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Self>, AgentExecutionConfigError> {
        let profiles = sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_execution_profiles ORDER BY name",
        )
        .fetch_all(pool)
        .await?;

        Ok(profiles)
    }
}

// ============================================================================
// AGENT EXECUTION CONFIG
// ============================================================================

/// Project-type specific backpressure configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
pub struct ProjectTypeBackpressure {
    #[serde(default)]
    pub rust: Vec<String>,
    #[serde(default)]
    pub node: Vec<String>,
    #[serde(default)]
    pub python: Vec<String>,
    #[serde(default)]
    pub go: Vec<String>,
}

/// Agent-specific execution configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AgentExecutionConfig {
    pub id: String,
    pub agent_id: Uuid,

    // Base profile
    pub execution_profile_id: Option<String>,

    // Override values
    pub execution_mode_override: Option<ExecutionMode>,
    pub max_iterations_override: Option<i32>,
    pub backpressure_commands_override: Option<String>,

    // Agent-specific prompt customization
    pub system_prompt_prefix: Option<String>,
    pub system_prompt_suffix: Option<String>,

    // Project-type specific backpressure (JSON)
    pub project_type_backpressure: Option<String>,

    // Feature flags
    pub auto_commit_on_success: Option<bool>,
    pub auto_create_pr_on_complete: Option<bool>,
    pub require_tests_pass: Option<bool>,

    // Metadata
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentExecutionConfig {
    /// Get effective execution mode (override or profile default)
    pub fn get_execution_mode(&self, profile: Option<&AgentExecutionProfile>) -> ExecutionMode {
        self.execution_mode_override
            .clone()
            .or_else(|| profile.map(|p| p.execution_mode.clone()))
            .unwrap_or_default()
    }

    /// Get effective max iterations
    pub fn get_max_iterations(&self, profile: Option<&AgentExecutionProfile>) -> i32 {
        self.max_iterations_override
            .or_else(|| profile.and_then(|p| p.max_iterations))
            .unwrap_or(50)
    }

    /// Get project-type backpressure config
    pub fn get_project_type_backpressure(&self) -> ProjectTypeBackpressure {
        self.project_type_backpressure
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Find config by agent ID
    pub async fn find_by_agent_id(
        pool: &SqlitePool,
        agent_id: Uuid,
    ) -> Result<Option<Self>, AgentExecutionConfigError> {
        let config = sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_execution_config WHERE agent_id = ?1 AND is_active = TRUE",
        )
        .bind(agent_id)
        .fetch_optional(pool)
        .await?;

        Ok(config)
    }

    /// Find config by agent short name
    pub async fn find_by_agent_name(
        pool: &SqlitePool,
        agent_name: &str,
    ) -> Result<Option<Self>, AgentExecutionConfigError> {
        let config = sqlx::query_as::<_, Self>(
            r#"
            SELECT aec.* FROM agent_execution_config aec
            JOIN agents a ON a.id = aec.agent_id
            WHERE LOWER(a.short_name) = LOWER(?1) AND aec.is_active = TRUE
            "#,
        )
        .bind(agent_name)
        .fetch_optional(pool)
        .await?;

        Ok(config)
    }

    /// Create new config for an agent
    pub async fn create(
        pool: &SqlitePool,
        data: CreateAgentExecutionConfig,
    ) -> Result<Self, AgentExecutionConfigError> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO agent_execution_config (
                id, agent_id, execution_profile_id, execution_mode_override,
                max_iterations_override, backpressure_commands_override,
                system_prompt_prefix, system_prompt_suffix, project_type_backpressure,
                auto_commit_on_success, auto_create_pr_on_complete, require_tests_pass
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&id)
        .bind(data.agent_id)
        .bind(&data.execution_profile_id)
        .bind(&data.execution_mode_override)
        .bind(data.max_iterations_override)
        .bind(&data.backpressure_commands_override)
        .bind(&data.system_prompt_prefix)
        .bind(&data.system_prompt_suffix)
        .bind(&data.project_type_backpressure)
        .bind(data.auto_commit_on_success)
        .bind(data.auto_create_pr_on_complete)
        .bind(data.require_tests_pass)
        .execute(pool)
        .await?;

        Self::find_by_agent_id(pool, data.agent_id)
            .await?
            .ok_or(AgentExecutionConfigError::ConfigNotFound(data.agent_id.to_string()))
    }

    /// Update config
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        data: UpdateAgentExecutionConfig,
    ) -> Result<Self, AgentExecutionConfigError> {
        sqlx::query(
            r#"
            UPDATE agent_execution_config SET
                execution_profile_id = COALESCE(?2, execution_profile_id),
                execution_mode_override = COALESCE(?3, execution_mode_override),
                max_iterations_override = COALESCE(?4, max_iterations_override),
                backpressure_commands_override = COALESCE(?5, backpressure_commands_override),
                system_prompt_prefix = COALESCE(?6, system_prompt_prefix),
                system_prompt_suffix = COALESCE(?7, system_prompt_suffix),
                project_type_backpressure = COALESCE(?8, project_type_backpressure),
                auto_commit_on_success = COALESCE(?9, auto_commit_on_success),
                auto_create_pr_on_complete = COALESCE(?10, auto_create_pr_on_complete),
                require_tests_pass = COALESCE(?11, require_tests_pass),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(&data.execution_profile_id)
        .bind(&data.execution_mode_override)
        .bind(data.max_iterations_override)
        .bind(&data.backpressure_commands_override)
        .bind(&data.system_prompt_prefix)
        .bind(&data.system_prompt_suffix)
        .bind(&data.project_type_backpressure)
        .bind(data.auto_commit_on_success)
        .bind(data.auto_create_pr_on_complete)
        .bind(data.require_tests_pass)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, Self>("SELECT * FROM agent_execution_config WHERE id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAgentExecutionConfig {
    pub agent_id: Uuid,
    pub execution_profile_id: Option<String>,
    pub execution_mode_override: Option<ExecutionMode>,
    pub max_iterations_override: Option<i32>,
    pub backpressure_commands_override: Option<String>,
    pub system_prompt_prefix: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub project_type_backpressure: Option<String>,
    pub auto_commit_on_success: Option<bool>,
    pub auto_create_pr_on_complete: Option<bool>,
    pub require_tests_pass: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateAgentExecutionConfig {
    pub execution_profile_id: Option<String>,
    pub execution_mode_override: Option<ExecutionMode>,
    pub max_iterations_override: Option<i32>,
    pub backpressure_commands_override: Option<String>,
    pub system_prompt_prefix: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub project_type_backpressure: Option<String>,
    pub auto_commit_on_success: Option<bool>,
    pub auto_create_pr_on_complete: Option<bool>,
    pub require_tests_pass: Option<bool>,
}

// ============================================================================
// RALPH LOOP STATE
// ============================================================================

/// State of a Ralph loop execution
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct RalphLoopState {
    pub id: String,
    pub task_attempt_id: Uuid,
    pub agent_id: Option<Uuid>,

    // Loop State
    pub current_iteration: i32,
    pub max_iterations: i32,
    pub session_id: Option<String>,

    // Status
    pub status: RalphLoopStatus,

    // Completion Tracking
    pub completion_promise: Option<String>,
    pub completion_detected_at: Option<DateTime<Utc>>,
    pub final_validation_passed: Option<bool>,

    // Metrics
    pub total_tokens_used: Option<i32>,
    pub total_cost_cents: Option<i32>,

    // Timing
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_iteration_at: Option<DateTime<Utc>>,

    // Error tracking
    pub last_error: Option<String>,
    pub consecutive_failures: Option<i32>,
}

impl RalphLoopState {
    /// Create a new Ralph loop state
    pub async fn create(
        pool: &SqlitePool,
        data: CreateRalphLoopState,
    ) -> Result<Self, AgentExecutionConfigError> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO ralph_loop_state (
                id, task_attempt_id, agent_id, max_iterations, completion_promise
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(&id)
        .bind(data.task_attempt_id)
        .bind(data.agent_id)
        .bind(data.max_iterations)
        .bind(&data.completion_promise)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or(AgentExecutionConfigError::RalphLoopNotFound(data.task_attempt_id))
    }

    /// Find by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<Option<Self>, AgentExecutionConfigError> {
        sqlx::query_as::<_, Self>("SELECT * FROM ralph_loop_state WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find by task attempt ID
    pub async fn find_by_task_attempt(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Option<Self>, AgentExecutionConfigError> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM ralph_loop_state WHERE task_attempt_id = ?1",
        )
        .bind(task_attempt_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Update loop state for next iteration
    pub async fn increment_iteration(
        pool: &SqlitePool,
        id: &str,
        session_id: Option<&str>,
    ) -> Result<Self, AgentExecutionConfigError> {
        sqlx::query(
            r#"
            UPDATE ralph_loop_state SET
                current_iteration = current_iteration + 1,
                session_id = COALESCE(?2, session_id),
                status = 'running',
                last_iteration_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(session_id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(AgentExecutionConfigError::RalphLoopNotFound(
                Uuid::parse_str(id).unwrap_or_default(),
            ))
    }

    /// Mark loop as complete
    pub async fn mark_complete(
        pool: &SqlitePool,
        id: &str,
        validation_passed: bool,
    ) -> Result<Self, AgentExecutionConfigError> {
        let status = if validation_passed {
            RalphLoopStatus::Complete
        } else {
            RalphLoopStatus::Failed
        };

        sqlx::query(
            r#"
            UPDATE ralph_loop_state SET
                status = ?2,
                final_validation_passed = ?3,
                completion_detected_at = datetime('now', 'subsec'),
                completed_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(&status)
        .bind(validation_passed)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(AgentExecutionConfigError::RalphLoopNotFound(
                Uuid::parse_str(id).unwrap_or_default(),
            ))
    }

    /// Mark loop as max iterations reached
    pub async fn mark_max_reached(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<Self, AgentExecutionConfigError> {
        sqlx::query(
            r#"
            UPDATE ralph_loop_state SET
                status = 'max_reached',
                completed_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(AgentExecutionConfigError::RalphLoopNotFound(
                Uuid::parse_str(id).unwrap_or_default(),
            ))
    }

    /// Record an error
    pub async fn record_error(
        pool: &SqlitePool,
        id: &str,
        error: &str,
    ) -> Result<(), AgentExecutionConfigError> {
        sqlx::query(
            r#"
            UPDATE ralph_loop_state SET
                last_error = ?2,
                consecutive_failures = COALESCE(consecutive_failures, 0) + 1
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Reset consecutive failures (on successful iteration)
    pub async fn reset_failures(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<(), AgentExecutionConfigError> {
        sqlx::query(
            "UPDATE ralph_loop_state SET consecutive_failures = 0 WHERE id = ?1",
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateRalphLoopState {
    pub task_attempt_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub max_iterations: i32,
    pub completion_promise: Option<String>,
}

// ============================================================================
// RALPH ITERATION
// ============================================================================

/// Backpressure validation results
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
pub struct BackpressureResults {
    #[serde(flatten)]
    pub results: std::collections::HashMap<String, BackpressureCommandResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BackpressureCommandResult {
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub duration_ms: Option<i64>,
}

/// Log of a single Ralph iteration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct RalphIteration {
    pub id: String,
    pub ralph_loop_id: String,
    pub execution_process_id: Option<Uuid>,

    pub iteration_number: i32,

    // Iteration Status
    pub status: RalphIterationStatus,

    // Completion Detection
    pub completion_signal_found: Option<bool>,
    pub exit_signal_found: Option<bool>,

    // Backpressure Results (JSON)
    pub backpressure_results: Option<String>,
    pub all_backpressure_passed: Option<bool>,

    // Metrics
    pub tokens_used: Option<i32>,
    pub cost_cents: Option<i32>,
    pub duration_ms: Option<i32>,

    // Timing
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,

    // Output summary
    pub output_summary: Option<String>,
    pub files_modified: Option<i32>,
    pub commits_made: Option<i32>,
}

impl RalphIteration {
    /// Create a new iteration record
    pub async fn create(
        pool: &SqlitePool,
        data: CreateRalphIteration,
    ) -> Result<Self, AgentExecutionConfigError> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO ralph_iterations (
                id, ralph_loop_id, execution_process_id, iteration_number
            ) VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(&id)
        .bind(&data.ralph_loop_id)
        .bind(data.execution_process_id)
        .bind(data.iteration_number)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, Self>("SELECT * FROM ralph_iterations WHERE id = ?1")
            .bind(&id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Mark iteration complete with results
    pub async fn mark_complete(
        pool: &SqlitePool,
        id: &str,
        completion_found: bool,
        exit_signal_found: bool,
        backpressure_results: Option<&BackpressureResults>,
        all_passed: bool,
    ) -> Result<Self, AgentExecutionConfigError> {
        let bp_json = backpressure_results
            .map(|r| serde_json::to_string(r).ok())
            .flatten();

        sqlx::query(
            r#"
            UPDATE ralph_iterations SET
                status = 'completed',
                completion_signal_found = ?2,
                exit_signal_found = ?3,
                backpressure_results = ?4,
                all_backpressure_passed = ?5,
                completed_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(completion_found)
        .bind(exit_signal_found)
        .bind(&bp_json)
        .bind(all_passed)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, Self>("SELECT * FROM ralph_iterations WHERE id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Find iterations for a loop
    pub async fn find_by_loop(
        pool: &SqlitePool,
        ralph_loop_id: &str,
    ) -> Result<Vec<Self>, AgentExecutionConfigError> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM ralph_iterations WHERE ralph_loop_id = ?1 ORDER BY iteration_number",
        )
        .bind(ralph_loop_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateRalphIteration {
    pub ralph_loop_id: String,
    pub execution_process_id: Option<Uuid>,
    pub iteration_number: i32,
}

// ============================================================================
// BACKPRESSURE DEFINITION
// ============================================================================

/// Reusable backpressure validation configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct BackpressureDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,

    // Project type this applies to (NULL = universal)
    pub project_type: Option<String>,

    // Commands to run (JSON array)
    pub commands: String,

    // Behavior
    pub fail_on_any: Option<bool>,
    pub timeout_ms: Option<i32>,
    pub run_in_parallel: Option<bool>,

    // Priority (higher = run first)
    pub priority: Option<i32>,

    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
}

impl BackpressureDefinition {
    /// Get commands as Vec
    pub fn get_commands(&self) -> Vec<String> {
        serde_json::from_str(&self.commands).unwrap_or_default()
    }

    /// Find definitions for a project type
    pub async fn find_for_project_type(
        pool: &SqlitePool,
        project_type: &str,
    ) -> Result<Vec<Self>, AgentExecutionConfigError> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM backpressure_definitions
            WHERE (project_type = ?1 OR project_type IS NULL)
            AND is_active = TRUE
            ORDER BY priority DESC
            "#,
        )
        .bind(project_type)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all active definitions
    pub async fn find_all_active(
        pool: &SqlitePool,
    ) -> Result<Vec<Self>, AgentExecutionConfigError> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM backpressure_definitions WHERE is_active = TRUE ORDER BY priority DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

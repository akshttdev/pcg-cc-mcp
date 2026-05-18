//! Orchestration Engine
//!
//! Multi-task execution engine for agent flows. Executes orchestration tasks
//! concurrently based on their type and concurrency safety flags.
//!
//! This module provides:
//! - `OrchestrationEngine`: Executes tasks within a single flow
//! - `OrchestrationCoordinator`: Background worker that polls for flows with pending tasks

use std::{sync::Arc, time::Duration};

use db::models::orchestration_task::{
    CreateOrchestrationTask, OrchestrationTask, OrchestrationTaskStatus, OrchestrationTaskType,
};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    orchestration_events::OrchestrationEvent,
    orchestration_state::{OrchestrationStateStore, TaskState},
    workers::BackgroundWorker,
};

/// Configuration for the orchestration engine
#[derive(Debug, Clone)]
pub struct OrchestrationConfig {
    /// Maximum concurrent tasks per flow (default: 5)
    pub max_concurrent: usize,
    /// Task execution timeout in seconds (default: 300 = 5 minutes)
    pub task_timeout_secs: u64,
    /// Polling interval for the coordinator in seconds (default: 5)
    pub poll_interval_secs: u64,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent: std::env::var("ORCHESTRATION_MAX_CONCURRENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            task_timeout_secs: std::env::var("ORCHESTRATION_TASK_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            poll_interval_secs: std::env::var("ORCHESTRATION_POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }
}

// ── Orchestration Engine ──────────────────────────────────────────────────────

/// Executes orchestration tasks within a single agent flow.
/// Manages concurrency, emits events, and tracks state.
pub struct OrchestrationEngine {
    pool: SqlitePool,
    flow_id: String,
    config: OrchestrationConfig,
    state: OrchestrationStateStore,
    event_tx: mpsc::Sender<OrchestrationEvent>,
}

impl OrchestrationEngine {
    /// Create a new engine for a specific flow
    pub fn new(
        pool: SqlitePool,
        flow_id: impl Into<String>,
        config: OrchestrationConfig,
        event_tx: mpsc::Sender<OrchestrationEvent>,
    ) -> Self {
        let flow_id = flow_id.into();
        Self {
            pool,
            state: OrchestrationStateStore::new(&flow_id, config.max_concurrent),
            flow_id,
            config,
            event_tx,
        }
    }

    /// Load tasks from database and populate state
    pub async fn load_tasks(&self) -> Result<(), sqlx::Error> {
        let tasks = OrchestrationTask::find_by_flow_id(&self.pool, &self.flow_id).await?;

        for task in &tasks {
            self.state.upsert_task(TaskState::from(task)).await;
        }

        tracing::info!(
            "[OrchestrationEngine] Loaded {} tasks for flow {}",
            tasks.len(),
            self.flow_id
        );

        Ok(())
    }

    /// Run all pending tasks with concurrency control.
    /// Returns when all tasks are complete (or failed/killed).
    pub async fn run(&self, shutdown: CancellationToken) -> Result<(), anyhow::Error> {
        self.state.mark_started().await;
        let start_time = std::time::Instant::now();

        loop {
            // Check for shutdown
            if shutdown.is_cancelled() {
                tracing::info!(
                    "[OrchestrationEngine] Shutdown requested for flow {}",
                    self.flow_id
                );
                break;
            }

            // Check if all tasks are complete
            if self.state.all_complete().await {
                break;
            }

            // Get startable tasks
            let startable_ids = self.state.get_startable_task_ids().await;
            if startable_ids.is_empty() {
                // No tasks to start, but not all complete — wait a bit
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Start tasks concurrently
            let mut handles = Vec::new();
            for task_id in startable_ids {
                let pool = self.pool.clone();
                let flow_id = self.flow_id.clone();
                let event_tx = self.event_tx.clone();
                let timeout_secs = self.config.task_timeout_secs;
                let shutdown_clone = shutdown.clone();
                let task_id_for_spawn = task_id.clone();

                let handle = tokio::spawn(async move {
                    execute_single_task(
                        pool,
                        &flow_id,
                        &task_id_for_spawn,
                        event_tx,
                        timeout_secs,
                        shutdown_clone,
                    )
                    .await
                });
                handles.push((task_id, handle));
            }

            // Wait for all spawned tasks in this batch
            for (task_id, handle) in handles {
                match handle.await {
                    Ok(Ok(())) => {
                        // Task completed, update state
                        let task = OrchestrationTask::find_by_id(&self.pool, &task_id).await;
                        if let Ok(Some(t)) = task {
                            self.state.upsert_task(TaskState::from(&t)).await;
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!("[OrchestrationEngine] Task {} failed: {}", task_id, e);
                        self.state
                            .update_task_status(&task_id, OrchestrationTaskStatus::Failed)
                            .await;
                    }
                    Err(e) => {
                        tracing::error!("[OrchestrationEngine] Task {} panicked: {}", task_id, e);
                        self.state
                            .update_task_status(&task_id, OrchestrationTaskStatus::Failed)
                            .await;
                    }
                }
            }
        }

        self.state.mark_ended().await;

        // Emit flow completed event
        let final_state = self.state.get().await;
        let completed_count =
            final_state.count_by_status(OrchestrationTaskStatus::Completed) as i32;
        let failed_count = final_state.count_by_status(OrchestrationTaskStatus::Failed) as i32;

        let _ = self
            .event_tx
            .send(OrchestrationEvent::flow_completed(
                &self.flow_id,
                start_time.elapsed().as_millis() as i64,
                completed_count,
                failed_count,
            ))
            .await;

        tracing::info!(
            "[OrchestrationEngine] Flow {} completed: {} succeeded, {} failed",
            self.flow_id,
            completed_count,
            failed_count
        );

        Ok(())
    }

    /// Add a new task to the flow
    pub async fn add_task(
        &self,
        task_type: OrchestrationTaskType,
        description: &str,
        is_concurrency_safe: bool,
    ) -> Result<OrchestrationTask, sqlx::Error> {
        let position = self.state.get().await.tasks.len() as i32;

        let task = OrchestrationTask::create(
            &self.pool,
            CreateOrchestrationTask {
                agent_flow_id: self.flow_id.clone(),
                task_type,
                description: description.to_string(),
                tool_use_id: None,
                is_concurrency_safe,
                position,
            },
        )
        .await?;

        // Update state
        self.state.upsert_task(TaskState::from(&task)).await;

        // Emit event
        let _ = self
            .event_tx
            .send(OrchestrationEvent::task_queued(
                &task.id,
                task_type,
                description,
                position,
            ))
            .await;

        Ok(task)
    }

    /// Kill a running task
    pub async fn kill_task(&self, task_id: &str) -> Result<(), anyhow::Error> {
        let task = OrchestrationTask::kill(&self.pool, task_id).await?;

        self.state.upsert_task(TaskState::from(&task)).await;

        let _ = self
            .event_tx
            .send(OrchestrationEvent::task_killed(task_id, task.duration_ms()))
            .await;

        Ok(())
    }
}

/// Execute a single task based on its type
async fn execute_single_task(
    pool: SqlitePool,
    flow_id: &str,
    task_id: &str,
    event_tx: mpsc::Sender<OrchestrationEvent>,
    timeout_secs: u64,
    shutdown: CancellationToken,
) -> Result<(), anyhow::Error> {
    let task = OrchestrationTask::find_by_id(&pool, task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

    // Mark as running
    OrchestrationTask::update_status(&pool, task_id, OrchestrationTaskStatus::Running).await?;

    let _ = event_tx
        .send(OrchestrationEvent::task_started(task_id, task.task_type))
        .await;

    let start_time = std::time::Instant::now();

    // Execute with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        execute_task_by_type(&pool, &task, flow_id, &event_tx, shutdown),
    )
    .await;

    let duration_ms = start_time.elapsed().as_millis() as i64;

    match result {
        Ok(Ok(output)) => {
            OrchestrationTask::complete(&pool, task_id, &output).await?;
            let _ = event_tx
                .send(OrchestrationEvent::task_completed(
                    task_id,
                    output,
                    duration_ms,
                ))
                .await;
        }
        Ok(Err(e)) => {
            let error = e.to_string();
            OrchestrationTask::set_error(&pool, task_id, &error).await?;
            let _ = event_tx
                .send(OrchestrationEvent::task_failed(
                    task_id,
                    error,
                    Some(duration_ms),
                ))
                .await;
        }
        Err(_) => {
            let error = format!("Task timed out after {} seconds", timeout_secs);
            OrchestrationTask::set_error(&pool, task_id, &error).await?;
            let _ = event_tx
                .send(OrchestrationEvent::task_failed(
                    task_id,
                    error,
                    Some(duration_ms),
                ))
                .await;
        }
    }

    Ok(())
}

/// Execute task based on its type
async fn execute_task_by_type(
    pool: &SqlitePool,
    task: &OrchestrationTask,
    _flow_id: &str,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
    shutdown: CancellationToken,
) -> Result<String, anyhow::Error> {
    match task.task_type {
        OrchestrationTaskType::LocalBash => execute_bash(task, event_tx).await,
        OrchestrationTaskType::LocalAgent => {
            execute_local_agent(pool, task, event_tx, shutdown).await
        }
        OrchestrationTaskType::RemoteAgent => execute_remote_agent(task, event_tx).await,
        OrchestrationTaskType::InProcessTeammate => {
            execute_in_process_teammate(task, event_tx).await
        }
        OrchestrationTaskType::LocalWorkflow => execute_local_workflow(pool, task, event_tx).await,
        OrchestrationTaskType::MonitorMcp => execute_monitor_mcp(task, event_tx, shutdown).await,
        OrchestrationTaskType::Dream => execute_dream(task, event_tx, shutdown).await,
    }
}

// ── Task Type Executors ───────────────────────────────────────────────────────

async fn execute_bash(
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
) -> Result<String, anyhow::Error> {
    let command = &task.description;

    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            format!("Executing: {}", command),
            Some(10),
        ))
        .await;

    // Execute bash command
    let output = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(anyhow::anyhow!(
            "Command failed with exit code {:?}: {}",
            output.status.code(),
            stderr
        ))
    }
}

async fn execute_local_agent(
    _pool: &SqlitePool,
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
    _shutdown: CancellationToken,
) -> Result<String, anyhow::Error> {
    // Local agent execution would integrate with AgentFlowExecutor
    // For now, return a placeholder indicating this needs implementation
    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            "Local agent execution starting...",
            Some(20),
        ))
        .await;

    // TODO: Integrate with AgentFlowExecutor or spawn a sub-agent
    // This would call the LLM with the task description as the prompt
    Ok(format!(
        "Local agent task '{}' completed (stub implementation)",
        task.description
    ))
}

async fn execute_remote_agent(
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
) -> Result<String, anyhow::Error> {
    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            "Remote agent delegation not yet implemented",
            Some(50),
        ))
        .await;

    // TODO: Make HTTP call to remote agent endpoint
    // Parse endpoint from task description or flow_config
    Ok(format!(
        "Remote agent task '{}' completed (stub implementation)",
        task.description
    ))
}

async fn execute_in_process_teammate(
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
) -> Result<String, anyhow::Error> {
    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            "In-process teammate execution",
            Some(50),
        ))
        .await;

    // In-process teammates share context with the main agent
    // This is similar to local_agent but with shared memory
    Ok(format!(
        "In-process teammate task '{}' completed (stub implementation)",
        task.description
    ))
}

async fn execute_local_workflow(
    _pool: &SqlitePool,
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
) -> Result<String, anyhow::Error> {
    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            "Executing local workflow",
            Some(30),
        ))
        .await;

    // TODO: Look up workflow by name in task description
    // Execute the workflow steps
    Ok(format!(
        "Local workflow task '{}' completed (stub implementation)",
        task.description
    ))
}

async fn execute_monitor_mcp(
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
    shutdown: CancellationToken,
) -> Result<String, anyhow::Error> {
    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            "MCP monitor starting...",
            Some(10),
        ))
        .await;

    // Monitor mode: wait for events from MCP server
    // This is a long-running task that only completes on shutdown
    let mut iteration = 0;
    loop {
        if shutdown.is_cancelled() {
            break;
        }

        // Simulate monitoring
        tokio::time::sleep(Duration::from_secs(5)).await;
        iteration += 1;

        let _ = event_tx
            .send(OrchestrationEvent::task_progress(
                &task.id,
                format!("Monitoring... (iteration {})", iteration),
                None,
            ))
            .await;
    }

    Ok(format!(
        "MCP monitor task '{}' completed after {} iterations",
        task.description, iteration
    ))
}

async fn execute_dream(
    task: &OrchestrationTask,
    event_tx: &mpsc::Sender<OrchestrationEvent>,
    shutdown: CancellationToken,
) -> Result<String, anyhow::Error> {
    let _ = event_tx
        .send(OrchestrationEvent::task_progress(
            &task.id,
            "Dream mode: background processing",
            Some(5),
        ))
        .await;

    // Dream tasks run in the background when no other tasks are active
    // They can be preempted by higher-priority tasks
    loop {
        if shutdown.is_cancelled() {
            break;
        }

        // Background processing
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    Ok(format!("Dream task '{}' completed", task.description))
}

// ── Orchestration Coordinator ─────────────────────────────────────────────────

/// Background worker that polls for flows with pending orchestration tasks
/// and spawns OrchestrationEngine instances to execute them.
pub struct OrchestrationCoordinator {
    pool: SqlitePool,
    config: OrchestrationConfig,
}

impl OrchestrationCoordinator {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            config: OrchestrationConfig::default(),
        }
    }

    pub fn with_config(pool: SqlitePool, config: OrchestrationConfig) -> Self {
        Self { pool, config }
    }

    /// Find flows that have pending orchestration tasks
    async fn find_flows_with_pending_tasks(&self) -> Vec<String> {
        let result: Result<Vec<(String,)>, _> = sqlx::query_as(
            r#"
            SELECT DISTINCT agent_flow_id
            FROM orchestration_tasks
            WHERE status = 'pending'
            LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await;

        result
            .unwrap_or_default()
            .into_iter()
            .map(|(id,)| id)
            .collect()
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for OrchestrationCoordinator {
    fn name(&self) -> &str {
        "OrchestrationCoordinator"
    }

    async fn run(&self, shutdown: CancellationToken) {
        tracing::info!(
            "[OrchestrationCoordinator] Starting with poll_interval={}s, max_concurrent={}",
            self.config.poll_interval_secs,
            self.config.max_concurrent
        );

        let poll_interval = Duration::from_secs(self.config.poll_interval_secs);
        let active_engines: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>> =
            Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new()));

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("[OrchestrationCoordinator] Shutting down");
                    break;
                }
                _ = tokio::time::sleep(poll_interval) => {
                    // Poll for flows with pending tasks
                    let flows = self.find_flows_with_pending_tasks().await;

                    for flow_id in flows {
                        // Skip if already running
                        {
                            let active = active_engines.read().await;
                            if active.contains(&flow_id) {
                                continue;
                            }
                        }

                        // Mark as active
                        {
                            let mut active = active_engines.write().await;
                            active.insert(flow_id.clone());
                        }

                        // Spawn engine for this flow
                        let pool = self.pool.clone();
                        let config = self.config.clone();
                        let shutdown_clone = shutdown.clone();
                        let active_engines_clone = active_engines.clone();
                        let flow_id_clone = flow_id.clone();

                        tokio::spawn(async move {
                            let (tx, mut rx) = mpsc::channel(100);

                            // Log events in background
                            let flow_id_for_log = flow_id_clone.clone();
                            tokio::spawn(async move {
                                while let Some(event) = rx.recv().await {
                                    tracing::debug!(
                                        "[OrchestrationEngine] Flow {} event: {:?}",
                                        flow_id_for_log,
                                        event
                                    );
                                }
                            });

                            let engine = OrchestrationEngine::new(
                                pool,
                                &flow_id_clone,
                                config,
                                tx,
                            );

                            if let Err(e) = engine.load_tasks().await {
                                tracing::error!(
                                    "[OrchestrationCoordinator] Failed to load tasks for flow {}: {}",
                                    flow_id_clone,
                                    e
                                );
                            } else if let Err(e) = engine.run(shutdown_clone).await {
                                tracing::error!(
                                    "[OrchestrationCoordinator] Flow {} execution failed: {}",
                                    flow_id_clone,
                                    e
                                );
                            }

                            // Remove from active set
                            let mut active = active_engines_clone.write().await;
                            active.remove(&flow_id_clone);
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = OrchestrationConfig::default();
        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.task_timeout_secs, 300);
        assert_eq!(config.poll_interval_secs, 5);
    }
}

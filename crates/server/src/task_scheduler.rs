//! Task Scheduler - Automatic task execution based on assigned agents
//!
//! This module provides automatic task execution when tasks are created with
//! an assigned agent. Tasks will be executed:
//! - Immediately if no scheduled_start is set
//! - At scheduled_start time if set
//!
//! The scheduler polls the database for eligible tasks and triggers execution
//! by calling the /run API endpoint.

use chrono::Utc;
use deployment::Deployment;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

/// Configuration for the task scheduler
#[derive(Debug, Clone)]
pub struct TaskSchedulerConfig {
    /// How often to check for new tasks (in seconds)
    pub poll_interval_secs: u64,
    /// Maximum concurrent task executions
    pub max_concurrent_executions: usize,
    /// Whether to auto-execute tasks immediately when assigned
    pub auto_execute_on_assign: bool,
    /// Whether to respect scheduled_start times
    pub respect_scheduled_start: bool,
    /// Backend port for API calls
    pub backend_port: u16,
}

impl Default for TaskSchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 30,
            max_concurrent_executions: 3,
            auto_execute_on_assign: true,
            respect_scheduled_start: true,
            backend_port: 3000,
        }
    }
}

/// Task scheduler that monitors and auto-executes tasks
pub struct TaskScheduler<D: Deployment> {
    deployment: D,
    config: TaskSchedulerConfig,
    is_running: Arc<RwLock<bool>>,
    active_executions: Arc<RwLock<usize>>,
}

impl<D: Deployment + Clone + Send + Sync + 'static> TaskScheduler<D> {
    pub fn new(deployment: D, config: TaskSchedulerConfig) -> Self {
        Self {
            deployment,
            config,
            is_running: Arc::new(RwLock::new(false)),
            active_executions: Arc::new(RwLock::new(0)),
        }
    }

    /// Start the scheduler background task
    pub async fn start(&self) -> anyhow::Result<()> {
        let mut running = self.is_running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        let pool = self.deployment.db().pool.clone();
        let config = self.config.clone();
        let is_running = self.is_running.clone();
        let active_executions = self.active_executions.clone();

        tokio::spawn(async move {
            info!("[TASK_SCHEDULER] Started - polling every {}s", config.poll_interval_secs);

            loop {
                // Check if still running
                let running = is_running.read().await;
                if !*running {
                    break;
                }
                drop(running);

                // Process eligible tasks
                if let Err(e) = Self::process_eligible_tasks(
                    &pool,
                    &config,
                    &active_executions,
                ).await {
                    error!("[TASK_SCHEDULER] Error processing tasks: {}", e);
                }

                // Sleep before next poll
                tokio::time::sleep(tokio::time::Duration::from_secs(config.poll_interval_secs)).await;
            }

            info!("[TASK_SCHEDULER] Stopped");
        });

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
    }

    /// Process all eligible tasks
    async fn process_eligible_tasks(
        pool: &sqlx::SqlitePool,
        config: &TaskSchedulerConfig,
        active_executions: &Arc<RwLock<usize>>,
    ) -> anyhow::Result<()> {

        // Find tasks that are eligible for execution:
        // - status = 'todo'
        // - assigned_agent is not null
        // - no existing task_attempt
        // - scheduled_start is null or in the past
        // - workflow_stage_required check: if task has a required workflow stage,
        //   only execute if the workflow has completed that stage
        let now = Utc::now();

        let eligible_tasks: Vec<EligibleTask> = sqlx::query_as(
            r#"
            SELECT
                t.id,
                t.project_id,
                t.assigned_agent,
                t.scheduled_start
            FROM tasks t
            LEFT JOIN project_boards pb ON t.board_id = pb.id
            LEFT JOIN conference_workflows cw ON pb.id = cw.conference_board_id
            WHERE t.status = 'todo'
              AND t.assigned_agent IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM task_attempts ta WHERE ta.task_id = t.id
              )
              AND (t.scheduled_start IS NULL OR t.scheduled_start <= ?)
              -- Workflow stage gating: check if required stage has been completed
              AND (
                  -- No workflow requirement specified - execute normally
                  json_extract(t.custom_properties, '$.workflow_stage_required') IS NULL
                  OR
                  -- Workflow doesn't exist for this board - execute normally
                  cw.id IS NULL
                  OR
                  -- Check if workflow has progressed past the required stage
                  -- Stage order: conference_intel(1) < speaker_research(2) < brand_research(3) < production_team(4) < competitive_intel(5) < side_events(6) < content_creation(7)
                  (
                      CASE json_extract(t.custom_properties, '$.workflow_stage_required')
                          WHEN 'intake' THEN 0
                          WHEN 'conference_intel' THEN 1
                          WHEN 'speaker_research' THEN 2
                          WHEN 'brand_research' THEN 3
                          WHEN 'production_team' THEN 4
                          WHEN 'competitive_intel' THEN 5
                          WHEN 'side_events' THEN 6
                          WHEN 'research_complete' THEN 7
                          WHEN 'content_creation' THEN 8
                          ELSE 99
                      END
                  ) <= (
                      CASE cw.current_stage
                          WHEN 'conference_intel' THEN 1
                          WHEN 'speaker_research' THEN 2
                          WHEN 'brand_research' THEN 3
                          WHEN 'production_team' THEN 4
                          WHEN 'competitive_intel' THEN 5
                          WHEN 'side_events' THEN 6
                          ELSE 0
                      END
                  )
                  OR
                  -- Workflow is completed or in content_creation - all research tasks can run
                  cw.status IN ('ResearchComplete', 'ContentCreation', 'Scheduling', 'Completed')
              )
            ORDER BY
                CASE t.priority
                    WHEN 'critical' THEN 0
                    WHEN 'high' THEN 1
                    WHEN 'medium' THEN 2
                    WHEN 'low' THEN 3
                    ELSE 4
                END,
                t.scheduled_start ASC NULLS LAST,
                t.created_at ASC
            LIMIT 10
            "#
        )
        .bind(now)
        .fetch_all(pool)
        .await?;

        if eligible_tasks.is_empty() {
            return Ok(());
        }

        info!(
            "[TASK_SCHEDULER] Found {} eligible tasks for execution",
            eligible_tasks.len()
        );

        for task in eligible_tasks {
            // Check if we're at max concurrent executions
            let current_executions = *active_executions.read().await;
            if current_executions >= config.max_concurrent_executions {
                info!(
                    "[TASK_SCHEDULER] At max concurrent executions ({}), deferring remaining tasks",
                    config.max_concurrent_executions
                );
                break;
            }

            // Execute the task
            if let Err(e) = Self::execute_task(pool, &task, active_executions, config.backend_port).await {
                error!(
                    "[TASK_SCHEDULER] Failed to execute task {}: {}",
                    task.id, e
                );
            }
        }

        Ok(())
    }

    /// Execute a single task by calling the /run API endpoint
    async fn execute_task(
        _pool: &sqlx::SqlitePool, // Reserved for future use (e.g., updating task status)
        task: &EligibleTask,
        active_executions: &Arc<RwLock<usize>>,
        backend_port: u16,
    ) -> anyhow::Result<()> {
        // Increment active executions
        {
            let mut count = active_executions.write().await;
            *count += 1;
        }

        info!(
            "[TASK_SCHEDULER] Executing task {} with agent '{}'",
            task.id, task.assigned_agent
        );

        // Call the /run endpoint via HTTP
        let client = reqwest::Client::new();
        let url = format!(
            "http://127.0.0.1:{}/api/tasks/{}/run",
            backend_port, task.id
        );

        let payload = serde_json::json!({
            "base_branch": "main"
        });

        info!("[TASK_SCHEDULER] Triggering execution at {}", url);

        match client
            .post(&url)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    info!(
                        "[TASK_SCHEDULER] Successfully triggered execution for task {}",
                        task.id
                    );
                } else {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    error!(
                        "[TASK_SCHEDULER] Failed to execute task {}: {}",
                        task.id, error_text
                    );
                    // Decrement on failure
                    let mut count = active_executions.write().await;
                    *count = count.saturating_sub(1);
                }
            }
            Err(e) => {
                error!(
                    "[TASK_SCHEDULER] HTTP request failed for task {}: {}",
                    task.id, e
                );
                // Decrement on failure
                let mut count = active_executions.write().await;
                *count = count.saturating_sub(1);
                return Err(e.into());
            }
        }

        Ok(())
    }
}

/// Eligible task for execution
#[derive(Debug, sqlx::FromRow)]
struct EligibleTask {
    id: Uuid,
    project_id: Uuid,
    assigned_agent: String,
    #[allow(dead_code)]
    scheduled_start: Option<chrono::DateTime<Utc>>,
}

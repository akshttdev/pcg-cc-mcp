//! Workflow orchestration - manages multiple concurrent workflow executions

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    coordination::CoordinationManager,
    executor::TaskExecutor,
    profiles::{AgentProfile, AgentWorkflow},
    Result,
};
use cinematics::CinematicsService;

use super::{
    router::WorkflowRouter,
    types::{WorkflowContext, WorkflowInstance, WorkflowResult, WorkflowState},
};

/// Events emitted during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowEvent {
    WorkflowStarted {
        workflow_id: Uuid,
        agent_id: String,
        workflow_name: String,
    },
    StageStarted {
        workflow_id: Uuid,
        stage_index: usize,
        stage_name: String,
    },
    StageCompleted {
        workflow_id: Uuid,
        stage_index: usize,
        stage_name: String,
        task_id: Option<Uuid>,
    },
    StageFailed {
        workflow_id: Uuid,
        stage_index: usize,
        stage_name: String,
        error: String,
    },
    WorkflowCompleted {
        workflow_id: Uuid,
        agent_id: String,
        execution_time_ms: u64,
    },
    WorkflowFailed {
        workflow_id: Uuid,
        agent_id: String,
        error: String,
        stage: usize,
    },
}

/// Central orchestrator for managing workflow executions
/// Nora remains in control - the orchestrator tracks workflow state and progress
pub struct WorkflowOrchestrator {
    router: WorkflowRouter,
    active_workflows: Arc<RwLock<HashMap<Uuid, WorkflowInstance>>>,
    coordination: Arc<CoordinationManager>,
    event_sender: broadcast::Sender<WorkflowEvent>,
    task_executor: RwLock<Option<Arc<TaskExecutor>>>,
    db: RwLock<Option<SqlitePool>>,
    cinematics: RwLock<Option<Arc<CinematicsService>>>,
}

impl WorkflowOrchestrator {
    pub fn new(
        agents: Vec<AgentProfile>,
        coordination: Arc<CoordinationManager>,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(1000);

        Self {
            router: WorkflowRouter::new(agents),
            active_workflows: Arc::new(RwLock::new(HashMap::new())),
            coordination,
            event_sender,
            task_executor: RwLock::new(None),
            db: RwLock::new(None),
            cinematics: RwLock::new(None),
        }
    }

    pub async fn set_task_executor(&self, executor: Arc<TaskExecutor>) {
        *self.task_executor.write().await = Some(executor);
    }

    pub async fn set_database(&self, db: SqlitePool) {
        *self.db.write().await = Some(db);
    }

    /// Get all agent profiles with their workflows
    pub fn get_all_agent_workflows(&self) -> Vec<(String, String, Vec<(String, String, String)>)> {
        self.router
            .get_agents()
            .iter()
            .map(|agent| {
                let workflows = agent
                    .workflows
                    .iter()
                    .map(|w| (w.workflow_id.clone(), w.name.clone(), w.objective.clone()))
                    .collect();
                (agent.agent_id.clone(), agent.codename.clone(), workflows)
            })
            .collect()
    }

    /// Get workflows for a specific agent
    pub fn get_workflows_for_agent(&self, agent_id: &str) -> Vec<(String, String, String)> {
        let workflows = self.router.get_agent_workflows(agent_id);
        workflows
            .iter()
            .map(|w| (w.workflow_id.clone(), w.name.clone(), w.objective.clone()))
            .collect()
    }

    pub async fn set_cinematics(&self, cinematics: Arc<CinematicsService>) {
        *self.cinematics.write().await = Some(cinematics);
    }

    /// Start a workflow execution by agent and workflow ID
    pub async fn start_workflow(
        &self,
        agent_id: &str,
        workflow_id: &str,
        context: WorkflowContext,
    ) -> Result<Uuid> {
        let (agent, workflow) = self
            .router
            .find_workflow(agent_id, workflow_id)
            .ok_or_else(|| crate::NoraError::CoordinationError(format!(
                "Workflow not found: agent={}, workflow={}",
                agent_id, workflow_id
            )))?;

        self.execute_workflow_internal(agent, workflow, context).await
    }

    /// Route a user request to the appropriate workflow and execute it
    pub async fn route_and_execute(
        &self,
        user_request: &str,
        context: WorkflowContext,
    ) -> Result<Uuid> {
        // Get current agent states for routing
        let agent_states = self.coordination.get_all_agents().await?;

        let (agent, workflow) = self
            .router
            .route_request(user_request, &agent_states)
            .ok_or_else(|| crate::NoraError::CoordinationError(format!(
                "No workflow found for request: {}",
                user_request
            )))?;

        tracing::info!(
            "[WORKFLOW_ORCHESTRATOR] Routed request to agent '{}' workflow '{}'",
            agent.codename,
            workflow.name
        );

        self.execute_workflow_internal(agent, workflow, context).await
    }

    /// Execute a workflow
    async fn execute_workflow_internal(
        &self,
        agent: AgentProfile,
        workflow: AgentWorkflow,
        context: WorkflowContext,
    ) -> Result<Uuid> {
        let workflow_instance_id = Uuid::new_v4();
        let now = Utc::now();

        // Create workflow instance
        let instance = WorkflowInstance {
            id: workflow_instance_id,
            agent_id: agent.agent_id.clone(),
            workflow_id: workflow.workflow_id.clone(),
            workflow: workflow.clone(),
            current_stage: 0,
            state: WorkflowState::Running {
                stage: 0,
                stage_name: workflow.stages.first().map(|s| s.name.clone()).unwrap_or_default(),
                progress: 0.0,
            },
            context: context.clone(),
            created_tasks: Vec::new(),
            deliverables: Vec::new(),
            started_at: now,
            updated_at: now,
            completed_at: None,
        };

        // Store instance
        {
            let mut workflows = self.active_workflows.write().await;
            workflows.insert(workflow_instance_id, instance);
        }

        // Emit start event (internal broadcast)
        let _ = self.event_sender.send(WorkflowEvent::WorkflowStarted {
            workflow_id: workflow_instance_id,
            agent_id: agent.agent_id.clone(),
            workflow_name: workflow.name.clone(),
        });

        // Emit coordination event for Mission Control visibility
        let first_stage_name = workflow.stages.first()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Starting".to_string());
        let _ = self.coordination.emit_event(
            crate::coordination::CoordinationEvent::WorkflowProgress {
                workflow_instance_id: workflow_instance_id.to_string(),
                agent_id: agent.agent_id.clone(),
                agent_codename: agent.codename.clone(),
                workflow_name: workflow.name.clone(),
                current_stage: 1,
                total_stages: workflow.stages.len() as u32,
                stage_name: first_stage_name,
                status: "running".to_string(),
                project_id: context.project_id.map(|id| id.to_string()),
                timestamp: now,
            }
        ).await;

        // Create tracking tasks for each workflow stage (Nora will execute the tools)
        let task_ids = if let Some(task_executor) = self.task_executor.read().await.as_ref() {
            self.create_workflow_tasks(&agent, &workflow, &context, task_executor).await
        } else {
            Vec::new()
        };

        // Update instance with created tasks
        {
            let mut workflows = self.active_workflows.write().await;
            if let Some(instance) = workflows.get_mut(&workflow_instance_id) {
                instance.created_tasks = task_ids.clone();
            }
        }

        // Note: Workflow tasks are now created and tracked
        // Nora will execute the actual tools and update task status
        // The orchestrator simply creates the task plan

        tracing::info!(
            "[WORKFLOW_ORCHESTRATOR] Workflow {} initialized with {} tasks",
            workflow_instance_id,
            task_ids.len()
        );

        Ok(workflow_instance_id)
    }

    /// Get the status of a running workflow
    pub async fn get_workflow_status(&self, workflow_id: Uuid) -> Option<WorkflowInstance> {
        let workflows = self.active_workflows.read().await;
        workflows.get(&workflow_id).cloned()
    }

    /// Get all active workflows
    pub async fn get_active_workflows(&self) -> Vec<WorkflowInstance> {
        let workflows = self.active_workflows.read().await;
        workflows.values().cloned().collect()
    }

    /// Subscribe to workflow events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<WorkflowEvent> {
        self.event_sender.subscribe()
    }

    /// Get workflow result after completion
    pub async fn get_workflow_result(&self, workflow_id: Uuid) -> Option<WorkflowResult> {
        let workflows = self.active_workflows.read().await;
        workflows.get(&workflow_id).map(|instance| WorkflowResult {
            workflow_id: instance.id,
            agent_id: instance.agent_id.clone(),
            workflow_name: instance.workflow.name.clone(),
            state: instance.state.clone(),
            created_tasks: instance.created_tasks.clone(),
            deliverables: instance.deliverables.clone(),
            execution_time_ms: instance.completed_at
                .map(|completed| {
                    (completed.timestamp_millis() - instance.started_at.timestamp_millis()) as u64
                })
                .unwrap_or(0),
        })
    }

    /// Cancel a running workflow
    pub async fn cancel_workflow(&self, workflow_id: Uuid) -> Result<()> {
        let mut workflows = self.active_workflows.write().await;
        if let Some(instance) = workflows.get_mut(&workflow_id) {
            instance.state = WorkflowState::Paused {
                reason: "Cancelled by user".to_string(),
                stage: instance.current_stage,
            };
            instance.updated_at = Utc::now();

            tracing::info!("[WORKFLOW_ORCHESTRATOR] Workflow {} cancelled", workflow_id);
        }

        Ok(())
    }

    /// Update workflow stage progress
    pub async fn advance_workflow_stage(
        &self,
        workflow_id: Uuid,
        stage_output: serde_json::Value,
    ) -> Result<()> {
        let mut workflows = self.active_workflows.write().await;

        if let Some(instance) = workflows.get_mut(&workflow_id) {
            let stage_name = instance.workflow.stages[instance.current_stage].name.clone();

            // Store stage output
            instance.context.set_stage_output(stage_name.clone(), stage_output);

            // Advance to next stage
            instance.current_stage += 1;
            instance.updated_at = Utc::now();

            // Check if workflow is complete
            let status = if instance.current_stage >= instance.workflow.stages.len() {
                tracing::info!(
                    "[WORKFLOW_ORCHESTRATOR] Workflow {} completed successfully",
                    workflow_id
                );
                instance.completed_at = Some(Utc::now());
                instance.state = WorkflowState::Completed {
                    total_stages: instance.workflow.stages.len(),
                    execution_time_ms: (Utc::now() - instance.started_at).num_milliseconds() as u64,
                };
                "completed"
            } else {
                tracing::info!(
                    "[WORKFLOW_ORCHESTRATOR] Workflow {} advanced to stage {}/{}",
                    workflow_id,
                    instance.current_stage + 1,
                    instance.workflow.stages.len()
                );
                "running"
            };

            // Emit coordination event for Mission Control visibility
            let agent_codename = self.get_agent_codename_sync(&instance.agent_id);
            let _ = self.coordination.emit_event(
                crate::coordination::CoordinationEvent::WorkflowProgress {
                    workflow_instance_id: workflow_id.to_string(),
                    agent_id: instance.agent_id.clone(),
                    agent_codename,
                    workflow_name: instance.workflow.name.clone(),
                    current_stage: instance.current_stage as u32,
                    total_stages: instance.workflow.stages.len() as u32,
                    stage_name,
                    status: status.to_string(),
                    project_id: instance.context.project_id.map(|id| id.to_string()),
                    timestamp: Utc::now(),
                }
            ).await;
        }

        Ok(())
    }

    /// Helper to get agent codename from agent_id
    fn get_agent_codename_sync(&self, agent_id: &str) -> String {
        // Try to find the agent in the router
        for agent in self.router.get_agents() {
            if agent.agent_id == agent_id {
                return agent.codename.clone();
            }
        }
        agent_id.to_string()
    }

    /// Mark workflow stage as failed
    pub async fn fail_workflow_stage(
        &self,
        workflow_id: Uuid,
        error: String,
    ) -> Result<()> {
        let mut workflows = self.active_workflows.write().await;

        if let Some(instance) = workflows.get_mut(&workflow_id) {
            let stage_name = instance.workflow.stages[instance.current_stage].name.clone();

            instance.state = WorkflowState::Failed {
                error: error.clone(),
                stage: instance.current_stage,
                stage_name,
            };
            instance.updated_at = Utc::now();

            tracing::error!(
                "[WORKFLOW_ORCHESTRATOR] Workflow {} failed at stage {}: {}",
                workflow_id,
                instance.current_stage,
                error
            );
        }

        Ok(())
    }

    /// Clean up completed workflows older than a threshold
    pub async fn cleanup_old_workflows(&self, max_age_hours: i64) {
        let mut workflows = self.active_workflows.write().await;
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);

        workflows.retain(|id, instance| {
            if let Some(completed) = instance.completed_at {
                if completed < cutoff {
                    tracing::debug!("[WORKFLOW_ORCHESTRATOR] Cleaning up old workflow {}", id);
                    return false;
                }
            }
            true
        });
    }

    /// Create tracking tasks for each workflow stage
    async fn create_workflow_tasks(
        &self,
        agent: &crate::profiles::AgentProfile,
        workflow: &crate::profiles::AgentWorkflow,
        context: &WorkflowContext,
        executor: &TaskExecutor,
    ) -> Vec<Uuid> {
        use crate::executor::TaskDefinition;
        use db::models::task::Priority;

        let project_id = match context.project_id {
            Some(id) => id,
            None => {
                tracing::warn!("[WORKFLOW_ORCHESTRATOR] No project_id in context, skipping task creation");
                return Vec::new();
            }
        };

        let mut task_ids = Vec::new();

        for stage in &workflow.stages {
            let task_def = TaskDefinition {
                title: format!("{} - {}", agent.codename, stage.name),
                description: Some(format!("{}\n\nExpected output: {}", stage.description, stage.output)),
                priority: Some(Priority::High),
                tags: Some(vec![
                    agent.agent_id.clone(),
                    "workflow".to_string(),
                    workflow.workflow_id.clone(),
                ]),
                assignee_id: None,
                board_id: None,
                pod_id: None,
            };

            match executor.create_task(project_id, task_def).await {
                Ok(task) => {
                    tracing::info!(
                        "[WORKFLOW_ORCHESTRATOR] Created task '{}' for stage '{}'",
                        task.title,
                        stage.name
                    );
                    task_ids.push(task.id);
                }
                Err(e) => {
                    tracing::error!(
                        "[WORKFLOW_ORCHESTRATOR] Failed to create task for stage '{}': {}",
                        stage.name,
                        e
                    );
                }
            }
        }

        task_ids
    }
}

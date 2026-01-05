//! Execution Engine - Unified orchestration for all agent workflows
//!
//! Implements the complete Router-Executor-Observer loop:
//! 1. Router: Analyze request, select agent/workflow, create plan
//! 2. Executor: Run stages, manage artifacts, create tasks
//! 3. Observer: Verify completion, handle failures, broadcast events

use super::{
    artifact::{Artifact, ArtifactStore, ArtifactType},
    events::{AgentStatusType, EventBroadcaster},
    router::{AgentMatch, ExecutionRouter},
};
use crate::profiles::{AgentProfile, AgentWorkflow, WorkflowStage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

/// Request to execute a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Project UUID (required for task creation)
    pub project_id: Option<Uuid>,
    /// Agent to use (by ID or codename)
    pub agent: Option<String>,
    /// Specific workflow to run (optional, will use default if not specified)
    pub workflow_id: Option<String>,
    /// User's request text (used for routing if agent not specified)
    pub request: Option<String>,
    /// Input data for the workflow
    pub inputs: HashMap<String, serde_json::Value>,
}

/// Status of an execution
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStatus {
    Pending,
    Planning,
    Executing { stage: u32, stage_name: String },
    Verifying,
    Completed,
    Failed { error: String, stage: Option<u32> },
    Cancelled,
}

/// Result of an execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub execution_id: Uuid,
    pub agent_id: String,
    pub agent_name: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: ExecutionStatus,
    pub stages_completed: u32,
    pub total_stages: u32,
    pub artifacts: Vec<Uuid>,
    pub tasks_created: Vec<Uuid>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Active execution instance
#[derive(Debug)]
struct ExecutionInstance {
    id: Uuid,
    agent: AgentProfile,
    workflow: AgentWorkflow,
    project_id: Option<Uuid>,
    inputs: HashMap<String, serde_json::Value>,
    status: ExecutionStatus,
    current_stage: usize,
    tasks_created: Vec<Uuid>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

/// Unified Execution Engine
///
/// Replaces:
/// - WorkflowOrchestrator (workflow/orchestrator.rs)
/// - GraphOrchestrator (graph.rs)
/// - CoordinationManager (coordination.rs)
/// - TaskExecutor (executor.rs) - partially, for workflow-related execution
pub struct ExecutionEngine {
    router: Arc<ExecutionRouter>,
    artifacts: Arc<ArtifactStore>,
    events: Arc<EventBroadcaster>,
    executions: RwLock<HashMap<Uuid, ExecutionInstance>>,
    /// Callback for creating tasks on the board
    task_creator: RwLock<Option<Arc<dyn TaskCreator + Send + Sync>>>,
    /// Optional coordination manager for SSE event bridging
    coordination: RwLock<Option<Arc<crate::coordination::CoordinationManager>>>,
    /// Research executor for research agents (Scout, etc.)
    research_executor: super::research::ResearchExecutor,
    /// Research context cache for multi-stage workflows
    research_contexts: RwLock<HashMap<Uuid, super::research::ResearchContext>>,
}

/// Trait for creating tasks on the board
#[async_trait::async_trait]
pub trait TaskCreator {
    async fn create_task(
        &self,
        project_id: Uuid,
        title: String,
        description: Option<String>,
        agent_id: Option<String>,
    ) -> Result<Uuid, String>;
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(agents: Vec<AgentProfile>) -> Arc<Self> {
        Arc::new(Self {
            router: ExecutionRouter::new(agents),
            artifacts: ArtifactStore::new(),
            events: Arc::new(EventBroadcaster::new()),
            executions: RwLock::new(HashMap::new()),
            task_creator: RwLock::new(None),
            coordination: RwLock::new(None),
            research_executor: super::research::ResearchExecutor::new(),
            research_contexts: RwLock::new(HashMap::new()),
        })
    }

    /// Set the task creator callback
    pub async fn set_task_creator(&self, creator: Arc<dyn TaskCreator + Send + Sync>) {
        let mut tc = self.task_creator.write().await;
        *tc = Some(creator);
    }

    /// Set the coordination manager for SSE event bridging
    pub async fn set_coordination_manager(&self, manager: Arc<crate::coordination::CoordinationManager>) {
        let mut coord = self.coordination.write().await;
        *coord = Some(manager);
    }

    /// Emit an event through the coordination manager (for SSE streaming)
    async fn emit_coordination_event(&self, event: crate::coordination::CoordinationEvent) {
        let coord = self.coordination.read().await;
        if let Some(manager) = coord.as_ref() {
            tracing::info!("[EXECUTION_ENGINE] Emitting coordination event to subscribers");
            if let Err(e) = manager.emit_event(event).await {
                tracing::debug!("[EXECUTION_ENGINE] Failed to emit coordination event: {}", e);
            }
        } else {
            tracing::warn!("[EXECUTION_ENGINE] No coordination manager set - cannot emit event");
        }
    }

    /// Get event broadcaster for subscribers
    pub fn events(&self) -> Arc<EventBroadcaster> {
        self.events.clone()
    }

    /// Get artifact store
    pub fn artifacts(&self) -> Arc<ArtifactStore> {
        self.artifacts.clone()
    }

    /// Get router for agent lookups
    pub fn router(&self) -> Arc<ExecutionRouter> {
        self.router.clone()
    }

    /// Execute a workflow request
    pub async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult, String> {
        // Phase 1: ROUTER - Find agent and workflow
        let agent_match = self.route_request(&request)?;

        let execution_id = Uuid::new_v4();
        let agent = agent_match.agent;
        let workflow = agent_match.workflow;

        tracing::info!(
            "[EXECUTION_ENGINE] Starting execution {} - Agent: {} ({}) - Workflow: {}",
            execution_id,
            agent.codename,
            agent.agent_id,
            workflow.name
        );

        // Create execution instance
        let instance = ExecutionInstance {
            id: execution_id,
            agent: agent.clone(),
            workflow: workflow.clone(),
            project_id: request.project_id,
            inputs: request.inputs.clone(),
            status: ExecutionStatus::Planning,
            current_stage: 0,
            tasks_created: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
        };

        // Store instance
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id, instance);
        }

        // Broadcast start event
        self.events.execution_started(
            execution_id,
            &agent.agent_id,
            &agent.codename,
            &workflow.name,
            request.project_id,
            workflow.stages.len() as u32,
        );

        // Also emit to coordination manager for SSE streaming
        self.emit_coordination_event(crate::coordination::CoordinationEvent::ExecutionStarted {
            execution_id: execution_id.to_string(),
            project_id: request.project_id.map(|id| id.to_string()),
            agent_codename: agent.codename.clone(),
            workflow_name: Some(workflow.name.clone()),
            timestamp: Utc::now(),
        }).await;

        // Update agent status
        self.events.agent_status(
            &agent.agent_id,
            &agent.codename,
            AgentStatusType::Planning,
            Some(&workflow.name),
        );

        // Create plan artifact
        let plan_artifact = Artifact::new(
            execution_id,
            ArtifactType::Plan,
            format!("{} Execution Plan", workflow.name),
            serde_json::json!({
                "workflow": workflow.name,
                "stages": workflow.stages.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "description": s.description,
                        "output": s.output,
                    })
                }).collect::<Vec<_>>(),
                "inputs": request.inputs,
            }),
        ).with_agent(&agent.agent_id);

        self.artifacts.store(plan_artifact).await;

        // Phase 2: EXECUTOR - Run stages
        // Merge the original request text into inputs for research context
        // BUT don't overwrite if inputs already has a request (user-provided takes priority)
        let mut enriched_inputs = request.inputs.clone();
        if !enriched_inputs.contains_key("request") {
            if let Some(req_text) = &request.request {
                enriched_inputs.insert("request".to_string(), serde_json::Value::String(req_text.clone()));
            }
        }
        if let Some(pid) = &request.project_id {
            enriched_inputs.insert("project_id".to_string(), serde_json::Value::String(pid.to_string()));
        }

        let result = self.execute_stages(execution_id, &enriched_inputs).await;

        // Phase 3: OBSERVER - Finalize and report
        self.finalize_execution(execution_id, result).await
    }

    /// Route request to agent/workflow
    fn route_request(&self, request: &ExecutionRequest) -> Result<AgentMatch, String> {
        // If agent specified directly
        if let Some(agent_ref) = &request.agent {
            // Try by ID first
            if let Some((agent, workflow)) = request.workflow_id.as_ref()
                .and_then(|wid| self.router.get_workflow(agent_ref, wid))
            {
                return Ok(AgentMatch {
                    agent: agent.clone(),
                    workflow: workflow.clone(),
                    confidence: 1.0,
                    match_reasons: vec!["Direct agent/workflow specification".to_string()],
                });
            }

            // Try by codename
            if let Some(agent) = self.router.get_agent_by_codename(agent_ref) {
                let workflow = if let Some(wid) = &request.workflow_id {
                    agent.workflows.iter().find(|w| w.workflow_id == *wid)
                } else {
                    agent.workflows.first()
                };

                if let Some(workflow) = workflow {
                    return Ok(AgentMatch {
                        agent: agent.clone(),
                        workflow: workflow.clone(),
                        confidence: 1.0,
                        match_reasons: vec![format!("Agent codename: {}", agent_ref)],
                    });
                }
            }

            // Try by agent_id
            if let Some(agent) = self.router.get_agent(agent_ref) {
                let workflow = if let Some(wid) = &request.workflow_id {
                    agent.workflows.iter().find(|w| w.workflow_id == *wid)
                } else {
                    agent.workflows.first()
                };

                if let Some(workflow) = workflow {
                    return Ok(AgentMatch {
                        agent: agent.clone(),
                        workflow: workflow.clone(),
                        confidence: 1.0,
                        match_reasons: vec![format!("Agent ID: {}", agent_ref)],
                    });
                }
            }
        }

        // Route by request text
        if let Some(request_text) = &request.request {
            if let Some(agent_match) = self.router.route(request_text) {
                return Ok(agent_match);
            }
        }

        Err("Could not route request to any agent. Please specify an agent or provide a clearer request.".to_string())
    }

    /// Execute workflow stages
    async fn execute_stages(
        &self,
        execution_id: Uuid,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let (agent, workflow, project_id) = {
            let executions = self.executions.read().await;
            let instance = executions.get(&execution_id)
                .ok_or("Execution not found")?;
            (instance.agent.clone(), instance.workflow.clone(), instance.project_id)
        };

        // Create tasks on board if we have a project
        if let Some(pid) = project_id {
            self.create_workflow_tasks(execution_id, pid, &agent, &workflow).await;
        }

        // Update status to executing
        self.update_status(execution_id, ExecutionStatus::Executing {
            stage: 0,
            stage_name: workflow.stages.first()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
        }).await;

        self.events.agent_status(
            &agent.agent_id,
            &agent.codename,
            AgentStatusType::Executing,
            Some(&workflow.name),
        );

        // Execute each stage
        for (stage_index, stage) in workflow.stages.iter().enumerate() {
            let stage_start = Utc::now();

            tracing::info!(
                "[EXECUTION_ENGINE] Executing stage {}/{}: {}",
                stage_index + 1,
                workflow.stages.len(),
                stage.name
            );

            // Broadcast stage start
            self.events.stage_started(
                execution_id,
                &agent.agent_id,
                &agent.codename,
                stage_index as u32,
                &stage.name,
                workflow.stages.len() as u32,
            );

            // Emit to coordination manager
            self.emit_coordination_event(crate::coordination::CoordinationEvent::ExecutionStageStarted {
                execution_id: execution_id.to_string(),
                stage_index: stage_index as u32,
                stage_name: stage.name.clone(),
                agent_codename: agent.codename.clone(),
                timestamp: Utc::now(),
            }).await;

            // Update status
            self.update_status(execution_id, ExecutionStatus::Executing {
                stage: stage_index as u32,
                stage_name: stage.name.clone(),
            }).await;

            // Get outputs from previous stages
            let previous_outputs = self.artifacts.get_all_stage_outputs(execution_id).await;

            // Execute stage (this is where actual work happens)
            let result = self.execute_stage(
                execution_id,
                stage_index,
                stage,
                &agent,
                inputs,
                &previous_outputs,
            ).await;

            let duration_ms = (Utc::now() - stage_start).num_milliseconds() as u64;

            match result {
                Ok(output) => {
                    // Store stage output artifact
                    let artifact = Artifact::new(
                        execution_id,
                        ArtifactType::StageOutput,
                        format!("{} Output", stage.name),
                        output,
                    )
                    .with_stage(stage_index as u32, &stage.name)
                    .with_agent(&agent.agent_id);

                    self.artifacts.store(artifact).await;

                    // Update current stage
                    {
                        let mut executions = self.executions.write().await;
                        if let Some(instance) = executions.get_mut(&execution_id) {
                            instance.current_stage = stage_index + 1;
                        }
                    }

                    // Broadcast completion
                    let artifact_count = self.artifacts.count(execution_id).await as u32;
                    self.events.stage_completed(
                        execution_id,
                        &agent.agent_id,
                        stage_index as u32,
                        &stage.name,
                        duration_ms,
                        artifact_count,
                    );

                    // Emit to coordination manager
                    self.emit_coordination_event(crate::coordination::CoordinationEvent::ExecutionStageCompleted {
                        execution_id: execution_id.to_string(),
                        stage_index: stage_index as u32,
                        stage_name: stage.name.clone(),
                        output_summary: Some(format!("Completed in {}ms", duration_ms)),
                        timestamp: Utc::now(),
                    }).await;

                    tracing::info!(
                        "[EXECUTION_ENGINE] Stage {} completed in {}ms",
                        stage.name,
                        duration_ms
                    );
                }
                Err(error) => {
                    tracing::error!(
                        "[EXECUTION_ENGINE] Stage {} failed: {}",
                        stage.name,
                        error
                    );

                    // Store error artifact
                    let artifact = Artifact::new(
                        execution_id,
                        ArtifactType::Error,
                        format!("{} Error", stage.name),
                        serde_json::json!({"error": error}),
                    )
                    .with_stage(stage_index as u32, &stage.name)
                    .with_agent(&agent.agent_id);

                    self.artifacts.store(artifact).await;

                    // Broadcast failure
                    self.events.stage_failed(
                        execution_id,
                        &agent.agent_id,
                        stage_index as u32,
                        &stage.name,
                        &error,
                    );

                    // Emit to coordination manager
                    self.emit_coordination_event(crate::coordination::CoordinationEvent::ExecutionFailed {
                        execution_id: execution_id.to_string(),
                        error: error.clone(),
                        stage: Some(stage_index as u32),
                        timestamp: Utc::now(),
                    }).await;

                    return Err(error);
                }
            }
        }

        Ok(())
    }

    /// Execute a single stage
    async fn execute_stage(
        &self,
        execution_id: Uuid,
        stage_index: usize,
        stage: &WorkflowStage,
        agent: &AgentProfile,
        inputs: &HashMap<String, serde_json::Value>,
        _previous_outputs: &HashMap<u32, serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        // Check if this is a research agent
        let is_research_agent = matches!(
            agent.agent_id.as_str(),
            "scout-research" | "oracle-strategy"
        );

        if is_research_agent {
            // Use the ResearchExecutor for real LLM-powered execution
            tracing::info!(
                "[EXECUTION_ENGINE] Using ResearchExecutor for {} stage '{}'",
                agent.codename,
                stage.name
            );

            // Get or create research context for this execution
            let mut contexts = self.research_contexts.write().await;
            let context = contexts.entry(execution_id).or_insert_with(|| {
                // Create initial research context from inputs
                let request = inputs
                    .get("request")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Research request")
                    .to_string();

                let project_name = inputs
                    .get("project_name")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                super::research::ResearchContext {
                    original_request: request.clone(),
                    research_brief: request,
                    project_name,
                    target: inputs
                        .get("target")
                        .and_then(|v| v.as_str())
                        .unwrap_or("General research")
                        .to_string(),
                    findings: std::collections::HashMap::new(),
                }
            });

            // If this is the first stage, enhance the request into a research brief
            if stage_index == 0 {
                let enhanced = self.research_executor
                    .create_research_brief(
                        &context.original_request,
                        context.project_name.as_deref(),
                    )
                    .await;

                if let Ok(new_context) = enhanced {
                    *context = new_context;
                    tracing::info!(
                        "[EXECUTION_ENGINE] Created research brief for: {}",
                        context.target
                    );
                }
            }

            // Execute the stage with the research executor
            let result = self.research_executor
                .execute_stage(
                    execution_id,
                    &stage.name,
                    &stage.description,
                    &stage.output,
                    context,
                )
                .await?;

            return Ok(result);
        }

        // For non-research agents, use the original simulated output (for now)
        // TODO: Add executors for other agent types (Maci -> ComfyUI, Editron -> video tools, etc.)
        tracing::debug!(
            "[EXECUTION_ENGINE] Stage context: agent={}, stage={}",
            agent.codename,
            stage.name
        );

        Ok(serde_json::json!({
            "stage": stage.name,
            "status": "completed",
            "output": stage.output,
            "simulated": true,
            "note": "Non-research agent - using simulated output"
        }))
    }

    /// Create tasks on the board for each workflow stage
    async fn create_workflow_tasks(
        &self,
        execution_id: Uuid,
        project_id: Uuid,
        agent: &AgentProfile,
        workflow: &AgentWorkflow,
    ) {
        let task_creator = self.task_creator.read().await;
        let Some(creator) = task_creator.as_ref() else {
            tracing::warn!("[EXECUTION_ENGINE] No task creator configured, skipping task creation");
            return;
        };

        let mut tasks_created = Vec::new();

        for stage in &workflow.stages {
            let title = format!("{} - {}", agent.codename, stage.name);
            let description = Some(format!(
                "{}\n\nExpected output: {}",
                stage.description, stage.output
            ));

            match creator.create_task(project_id, title.clone(), description, Some(agent.agent_id.clone())).await {
                Ok(task_id) => {
                    tasks_created.push(task_id);
                    self.events.task_created(
                        execution_id,
                        task_id,
                        project_id,
                        &title,
                        &agent.agent_id,
                    );
                    tracing::info!(
                        "[EXECUTION_ENGINE] Created task {} for stage '{}'",
                        task_id,
                        stage.name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "[EXECUTION_ENGINE] Failed to create task for stage '{}': {}",
                        stage.name,
                        e
                    );
                }
            }
        }

        // Update instance with created tasks
        let mut executions = self.executions.write().await;
        if let Some(instance) = executions.get_mut(&execution_id) {
            instance.tasks_created = tasks_created;
        }
    }

    /// Update execution status
    async fn update_status(&self, execution_id: Uuid, status: ExecutionStatus) {
        let mut executions = self.executions.write().await;
        if let Some(instance) = executions.get_mut(&execution_id) {
            instance.status = status;
        }
    }

    /// Finalize execution and return result
    async fn finalize_execution(
        &self,
        execution_id: Uuid,
        result: Result<(), String>,
    ) -> Result<ExecutionResult, String> {
        // Gather data for coordination event before taking lock
        let (project_id, tasks_count, artifact_count, current_stage, duration_ms, status, error, exec_result) = {
            let mut executions = self.executions.write().await;
            let instance = executions.get_mut(&execution_id)
                .ok_or("Execution not found")?;

            instance.completed_at = Some(Utc::now());
            let duration_ms = (instance.completed_at.unwrap() - instance.started_at).num_milliseconds() as u64;
            let artifact_count = self.artifacts.count(execution_id).await as u32;
            let project_id = instance.project_id;
            let tasks_count = instance.tasks_created.len() as u32;
            let current_stage = instance.current_stage;

            let (status, error) = match &result {
                Ok(()) => {
                    instance.status = ExecutionStatus::Completed;

                    // Broadcast completion
                    self.events.execution_completed(
                        execution_id,
                        &instance.agent.agent_id,
                        &instance.agent.codename,
                        &instance.workflow.name,
                        instance.workflow.stages.len() as u32,
                        duration_ms,
                        artifact_count,
                    );

                    (ExecutionStatus::Completed, None)
                }
                Err(e) => {
                    let failed_status = ExecutionStatus::Failed {
                        error: e.clone(),
                        stage: Some(instance.current_stage as u32),
                    };
                    instance.status = failed_status.clone();

                    // Broadcast failure
                    self.events.execution_failed(
                        execution_id,
                        &instance.agent.agent_id,
                        &instance.agent.codename,
                        &instance.workflow.name,
                        instance.current_stage as u32,
                        e,
                    );

                    (failed_status, Some(e.clone()))
                }
            };

            // Update agent status to idle
            self.events.agent_status(
                &instance.agent.agent_id,
                &instance.agent.codename,
                AgentStatusType::Idle,
                None,
            );

            let artifacts = self.artifacts.get_by_execution(execution_id).await;

            let exec_result = ExecutionResult {
                execution_id,
                agent_id: instance.agent.agent_id.clone(),
                agent_name: instance.agent.codename.clone(),
                workflow_id: instance.workflow.workflow_id.clone(),
                workflow_name: instance.workflow.name.clone(),
                status: status.clone(),
                stages_completed: instance.current_stage as u32,
                total_stages: instance.workflow.stages.len() as u32,
                artifacts: artifacts.iter().map(|a| a.id).collect(),
                tasks_created: instance.tasks_created.clone(),
                started_at: instance.started_at,
                completed_at: instance.completed_at,
                duration_ms: Some(duration_ms),
                error: error.clone(),
            };

            (project_id, tasks_count, artifact_count, current_stage, duration_ms, status, error, exec_result)
        };

        // Emit coordination events after releasing the lock
        match &status {
            ExecutionStatus::Completed => {
                self.emit_coordination_event(crate::coordination::CoordinationEvent::ExecutionCompleted {
                    execution_id: execution_id.to_string(),
                    project_id: project_id.map(|id| id.to_string()),
                    tasks_created: tasks_count,
                    artifacts_count: artifact_count,
                    duration_ms,
                    timestamp: Utc::now(),
                }).await;
            }
            ExecutionStatus::Failed { error: e, .. } => {
                self.emit_coordination_event(crate::coordination::CoordinationEvent::ExecutionFailed {
                    execution_id: execution_id.to_string(),
                    error: e.clone(),
                    stage: Some(current_stage as u32),
                    timestamp: Utc::now(),
                }).await;
            }
            _ => {}
        }

        Ok(exec_result)
    }

    /// Get active executions
    pub async fn get_active_executions(&self) -> Vec<ExecutionResult> {
        let executions = self.executions.read().await;
        let mut results = Vec::new();

        for instance in executions.values() {
            if !matches!(instance.status, ExecutionStatus::Completed | ExecutionStatus::Failed { .. } | ExecutionStatus::Cancelled) {
                let artifacts = self.artifacts.get_by_execution(instance.id).await;
                results.push(ExecutionResult {
                    execution_id: instance.id,
                    agent_id: instance.agent.agent_id.clone(),
                    agent_name: instance.agent.codename.clone(),
                    workflow_id: instance.workflow.workflow_id.clone(),
                    workflow_name: instance.workflow.name.clone(),
                    status: instance.status.clone(),
                    stages_completed: instance.current_stage as u32,
                    total_stages: instance.workflow.stages.len() as u32,
                    artifacts: artifacts.iter().map(|a| a.id).collect(),
                    tasks_created: instance.tasks_created.clone(),
                    started_at: instance.started_at,
                    completed_at: instance.completed_at,
                    duration_ms: None,
                    error: None,
                });
            }
        }

        results
    }

    /// Get execution by ID
    pub async fn get_execution(&self, execution_id: Uuid) -> Option<ExecutionResult> {
        let executions = self.executions.read().await;
        let instance = executions.get(&execution_id)?;
        let artifacts = self.artifacts.get_by_execution(execution_id).await;

        Some(ExecutionResult {
            execution_id: instance.id,
            agent_id: instance.agent.agent_id.clone(),
            agent_name: instance.agent.codename.clone(),
            workflow_id: instance.workflow.workflow_id.clone(),
            workflow_name: instance.workflow.name.clone(),
            status: instance.status.clone(),
            stages_completed: instance.current_stage as u32,
            total_stages: instance.workflow.stages.len() as u32,
            artifacts: artifacts.iter().map(|a| a.id).collect(),
            tasks_created: instance.tasks_created.clone(),
            started_at: instance.started_at,
            completed_at: instance.completed_at,
            duration_ms: instance.completed_at.map(|c| (c - instance.started_at).num_milliseconds() as u64),
            error: match &instance.status {
                ExecutionStatus::Failed { error, .. } => Some(error.clone()),
                _ => None,
            },
        })
    }
}

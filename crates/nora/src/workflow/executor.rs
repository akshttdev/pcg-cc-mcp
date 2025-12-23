//! Workflow execution engine - executes workflow stages sequentially

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    executor::TaskExecutor,
    profiles::{AgentWorkflow, WorkflowStage},
    tools::ExecutiveTools,
    NoraError, Result,
};

use super::types::{WorkflowContext, WorkflowStageResult};

/// Executes agent workflows stage by stage
pub struct AgentWorkflowExecutor {
    pub agent_id: String,
    pub agent_name: String,
    tools: Arc<ExecutiveTools>,
    task_executor: Option<Arc<TaskExecutor>>,
    db: Option<SqlitePool>,
}

impl AgentWorkflowExecutor {
    pub fn new(
        agent_id: String,
        agent_name: String,
        tools: Arc<ExecutiveTools>,
    ) -> Self {
        Self {
            agent_id,
            agent_name,
            tools,
            task_executor: None,
            db: None,
        }
    }

    pub fn with_task_executor(mut self, executor: Arc<TaskExecutor>) -> Self {
        self.task_executor = Some(executor);
        self
    }

    pub fn with_database(mut self, db: SqlitePool) -> Self {
        self.db = Some(db);
        self
    }

    /// Execute a complete workflow
    pub async fn execute_workflow(
        &self,
        workflow: &AgentWorkflow,
        context: &mut WorkflowContext,
    ) -> Result<Vec<WorkflowStageResult>> {
        tracing::info!(
            "[WORKFLOW] Starting workflow '{}' for agent '{}'",
            workflow.name,
            self.agent_id
        );

        let mut results = Vec::new();

        for (index, stage) in workflow.stages.iter().enumerate() {
            tracing::info!(
                "[WORKFLOW] Executing stage {}/{}: {}",
                index + 1,
                workflow.stages.len(),
                stage.name
            );

            match self.execute_stage(stage, context, &workflow.workflow_id).await {
                Ok(result) => {
                    tracing::info!(
                        "[WORKFLOW] Stage '{}' completed successfully in {}ms",
                        stage.name,
                        result.execution_time_ms
                    );

                    // Store stage output in context for next stages
                    context.set_stage_output(stage.name.clone(), result.output.clone());
                    results.push(result);
                }
                Err(err) => {
                    tracing::error!(
                        "[WORKFLOW] Stage '{}' failed: {}",
                        stage.name,
                        err
                    );

                    let error_result = WorkflowStageResult {
                        stage_name: stage.name.clone(),
                        success: false,
                        output: serde_json::json!({}),
                        task_id: None,
                        error: Some(err.to_string()),
                        execution_time_ms: 0,
                    };
                    results.push(error_result);

                    // Stop execution on first failure
                    return Err(err);
                }
            }
        }

        tracing::info!(
            "[WORKFLOW] Workflow '{}' completed successfully",
            workflow.name
        );

        Ok(results)
    }

    /// Execute a single workflow stage
    async fn execute_stage(
        &self,
        stage: &WorkflowStage,
        context: &WorkflowContext,
        workflow_id: &str,
    ) -> Result<WorkflowStageResult> {
        let start = Instant::now();

        // Create a tracking task for this stage
        let task_id = if let Some(executor) = &self.task_executor {
            self.create_stage_task(stage, context, executor, workflow_id).await.ok()
        } else {
            None
        };

        // Execute the stage logic based on the stage name
        // This maps stage names to actual tool executions
        let output = self.execute_stage_logic(stage, context).await?;

        // Mark task as complete if created
        if let (Some(executor), Some(task_id)) = (&self.task_executor, task_id) {
            self.complete_stage_task(executor, task_id).await;
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(WorkflowStageResult {
            stage_name: stage.name.clone(),
            success: true,
            output,
            task_id,
            error: None,
            execution_time_ms,
        })
    }

    /// Execute the actual logic for a workflow stage
    async fn execute_stage_logic(
        &self,
        stage: &WorkflowStage,
        context: &WorkflowContext,
    ) -> Result<serde_json::Value> {
        // Map stage names/descriptions to tool executions
        // This is a simple pattern matcher - could be made more sophisticated

        let stage_lower = stage.name.to_lowercase();
        let desc_lower = stage.description.to_lowercase();

        // Editron-specific stage mappings
        if desc_lower.contains("ingest") || desc_lower.contains("download") || desc_lower.contains("batch intake") {
            return self.execute_ingest_stage(context).await;
        }

        if desc_lower.contains("analyze") || desc_lower.contains("storyboard") || stage_lower.contains("analysis") {
            return self.execute_analysis_stage(context).await;
        }

        if desc_lower.contains("assembly") || desc_lower.contains("edit") || desc_lower.contains("timeline") {
            return self.execute_assembly_stage(context).await;
        }

        if desc_lower.contains("render") || desc_lower.contains("export") || desc_lower.contains("delivery") {
            return self.execute_render_stage(context).await;
        }

        // Generic execution - just log the stage
        tracing::info!(
            "[WORKFLOW] Generic stage execution: {} - {}",
            stage.name,
            stage.description
        );

        Ok(serde_json::json!({
            "stage": stage.name,
            "output": stage.output,
            "executed_at": Utc::now().to_rfc3339(),
        }))
    }

    /// Execute media ingest stage
    async fn execute_ingest_stage(&self, context: &WorkflowContext) -> Result<serde_json::Value> {
        let source_url = context
            .inputs
            .get("source_url")
            .or_else(|| context.inputs.get("dropbox_url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NoraError::ToolExecutionError("Missing source_url in context".to_string()))?;

        tracing::info!("[WORKFLOW] Ingesting media from: {}", source_url);

        // Call the ingest_media_batch tool
        let tool = crate::tools::NoraExecutiveTool::IngestMediaBatch {
            source_url: source_url.to_string(),
            reference_name: context.inputs.get("reference_name").and_then(|v| v.as_str()).map(String::from),
            storage_tier: context.inputs.get("storage_tier")
                .and_then(|v| v.as_str())
                .unwrap_or("hot")
                .to_string(),
            checksum_required: true,
            project_id: context.project_id.map(|id| id.to_string()),
        };

        let result = self.tools.execute_tool_implementation(tool).await?;

        Ok(result)
    }

    /// Execute media analysis stage
    async fn execute_analysis_stage(&self, context: &WorkflowContext) -> Result<serde_json::Value> {
        let batch_id = context
            .get_stage_output("Batch Intake")
            .or_else(|| context.get_stage_output("ingest"))
            .and_then(|v| v.get("batch"))
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NoraError::ToolExecutionError("No batch_id from previous stage".to_string()))?;

        tracing::info!("[WORKFLOW] Analyzing media batch: {}", batch_id);

        let tool = crate::tools::NoraExecutiveTool::AnalyzeMediaBatch {
            batch_id: batch_id.to_string(),
            brief: context.inputs.get("brief")
                .and_then(|v| v.as_str())
                .unwrap_or("Identify hero shots, crowd moments, and key narrative beats")
                .to_string(),
            passes: context.inputs.get("passes")
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as u32,
            deliverable_targets: context.inputs.get("deliverable_targets")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| vec!["recap".to_string()]),
            project_id: context.project_id.map(|id| id.to_string()),
        };

        let result = self.tools.execute_tool_implementation(tool).await?;

        Ok(result)
    }

    /// Execute video assembly stage
    async fn execute_assembly_stage(&self, context: &WorkflowContext) -> Result<serde_json::Value> {
        let batch_id = context
            .get_stage_output("analysis")
            .or_else(|| context.get_stage_output("Storyboard Pass"))
            .and_then(|v| v.get("batch_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NoraError::ToolExecutionError("No batch_id from analysis stage".to_string()))?;

        tracing::info!("[WORKFLOW] Generating video edits for batch: {}", batch_id);

        let tool = crate::tools::NoraExecutiveTool::GenerateVideoEdits {
            batch_id: batch_id.to_string(),
            deliverable_type: context.inputs.get("deliverable_type")
                .and_then(|v| v.as_str())
                .unwrap_or("recap")
                .to_string(),
            aspect_ratios: context.inputs.get("aspect_ratios")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| vec!["16:9".to_string(), "9:16".to_string()]),
            reference_style: context.inputs.get("reference_style")
                .and_then(|v| v.as_str())
                .map(String::from),
            include_captions: context.inputs.get("include_captions")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            project_id: context.project_id.map(|id| id.to_string()),
        };

        let result = self.tools.execute_tool_implementation(tool).await?;

        Ok(result)
    }

    /// Execute render/export stage
    async fn execute_render_stage(&self, context: &WorkflowContext) -> Result<serde_json::Value> {
        let edit_session_id = context
            .get_stage_output("assembly")
            .or_else(|| context.get_stage_output("Assembly + Color"))
            .and_then(|v| v.get("edit_session_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NoraError::ToolExecutionError("No edit_session_id from assembly stage".to_string()))?;

        tracing::info!("[WORKFLOW] Rendering deliverables for session: {}", edit_session_id);

        use crate::tools::VideoRenderPriority;

        let priority = match context.inputs.get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("standard") {
            "rush" | "urgent" | "high" => VideoRenderPriority::Rush,
            "low" => VideoRenderPriority::Low,
            _ => VideoRenderPriority::Standard,
        };

        let tool = crate::tools::NoraExecutiveTool::RenderVideoDeliverables {
            edit_session_id: edit_session_id.to_string(),
            destinations: context.inputs.get("destinations")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| vec!["local".to_string()]),
            formats: context.inputs.get("formats")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| vec!["mp4".to_string()]),
            priority,
            project_id: context.project_id.map(|id| id.to_string()),
        };

        let result = self.tools.execute_tool_implementation(tool).await?;

        Ok(result)
    }

    /// Create a task for tracking workflow stage progress
    async fn create_stage_task(
        &self,
        stage: &WorkflowStage,
        context: &WorkflowContext,
        executor: &TaskExecutor,
        workflow_id: &str,
    ) -> Result<Uuid> {
        use crate::executor::TaskDefinition;
        use db::models::task::Priority;

        let project_id = context.project_id.ok_or_else(||
            NoraError::ConfigError("Project ID required for task creation".to_string())
        )?;

        let task_def = TaskDefinition {
            title: format!("{} - {}", self.agent_name, stage.name),
            description: Some(format!("{}\n\nExpected output: {}", stage.description, stage.output)),
            priority: Some(Priority::High),
            tags: Some(vec![
                self.agent_id.clone(),
                "workflow".to_string(),
                workflow_id.to_string(),
            ]),
            assignee_id: None,
            board_id: None,
            pod_id: None,
        };

        executor.create_task(project_id, task_def).await.map(|task| task.id)
    }

    /// Mark a stage task as complete
    async fn complete_stage_task(&self, executor: &TaskExecutor, task_id: Uuid) {
        use db::models::task::TaskStatus;
        if let Err(e) = executor.update_task_status(task_id, TaskStatus::Done).await {
            tracing::warn!("[WORKFLOW] Failed to complete task {}: {}", task_id, e);
        }
    }
}

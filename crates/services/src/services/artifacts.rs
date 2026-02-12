use db::{
    models::{
        agent_task_plan::{AgentTaskPlan, AgentTaskPlanError, CreateAgentTaskPlan, PlanStatus, PlanStep},
        execution_artifact::{ArtifactType, CreateExecutionArtifact, ExecutionArtifact, ExecutionArtifactError},
    },
    DBService,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ArtifactServiceError {
    #[error(transparent)]
    ArtifactError(#[from] ExecutionArtifactError),
    #[error(transparent)]
    PlanError(#[from] AgentTaskPlanError),
    #[error("Failed to extract artifact: {0}")]
    ExtractionFailed(String),
}

/// Response containing all artifacts for an execution
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ExecutionArtifactsResponse {
    pub execution_process_id: Uuid,
    pub artifacts: Vec<ExecutionArtifact>,
    pub plan: Option<AgentTaskPlan>,
}

/// Summary of artifacts for Mission Control view
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ArtifactsSummary {
    pub total_count: usize,
    pub plans_count: usize,
    pub screenshots_count: usize,
    pub test_results_count: usize,
    pub errors_count: usize,
}

/// Service for managing execution artifacts and task plans
#[derive(Clone)]
pub struct ArtifactService {
    db: DBService,
}

impl ArtifactService {
    pub fn new(db: DBService) -> Self {
        Self { db }
    }

    /// Create a new artifact
    pub async fn create_artifact(
        &self,
        data: CreateExecutionArtifact,
    ) -> Result<ExecutionArtifact, ArtifactServiceError> {
        let artifact = ExecutionArtifact::create(&self.db.pool, data).await?;
        Ok(artifact)
    }

    /// Get all artifacts for an execution process
    pub async fn get_artifacts(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ExecutionArtifact>, ArtifactServiceError> {
        let artifacts =
            ExecutionArtifact::find_by_execution_process(&self.db.pool, execution_process_id)
                .await?;
        Ok(artifacts)
    }

    /// Get artifacts with plan for Mission Control
    pub async fn get_execution_artifacts(
        &self,
        execution_process_id: Uuid,
    ) -> Result<ExecutionArtifactsResponse, ArtifactServiceError> {
        let artifacts =
            ExecutionArtifact::find_by_execution_process(&self.db.pool, execution_process_id)
                .await?;
        let plan =
            AgentTaskPlan::find_by_execution_process(&self.db.pool, execution_process_id).await?;

        Ok(ExecutionArtifactsResponse {
            execution_process_id,
            artifacts,
            plan,
        })
    }

    /// Create a task plan for an execution
    pub async fn create_plan(
        &self,
        data: CreateAgentTaskPlan,
    ) -> Result<AgentTaskPlan, ArtifactServiceError> {
        let plan = AgentTaskPlan::create(&self.db.pool, data).await?;
        Ok(plan)
    }

    /// Get task plan for an execution
    pub async fn get_plan(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Option<AgentTaskPlan>, ArtifactServiceError> {
        let plan =
            AgentTaskPlan::find_by_execution_process(&self.db.pool, execution_process_id).await?;
        Ok(plan)
    }

    /// Update plan status
    pub async fn update_plan_status(
        &self,
        plan_id: Uuid,
        status: PlanStatus,
    ) -> Result<AgentTaskPlan, ArtifactServiceError> {
        let plan = AgentTaskPlan::update_status(&self.db.pool, plan_id, status).await?;
        Ok(plan)
    }

    /// Update plan current step
    pub async fn update_plan_step(
        &self,
        plan_id: Uuid,
        step: i32,
    ) -> Result<AgentTaskPlan, ArtifactServiceError> {
        let plan = AgentTaskPlan::update_current_step(&self.db.pool, plan_id, step).await?;
        Ok(plan)
    }

    /// Get all active plans across all executions
    pub async fn get_active_plans(&self) -> Result<Vec<AgentTaskPlan>, ArtifactServiceError> {
        let plans = AgentTaskPlan::find_active(&self.db.pool).await?;
        Ok(plans)
    }

    /// Get artifact summary for an execution
    pub async fn get_artifacts_summary(
        &self,
        execution_process_id: Uuid,
    ) -> Result<ArtifactsSummary, ArtifactServiceError> {
        let artifacts =
            ExecutionArtifact::find_by_execution_process(&self.db.pool, execution_process_id)
                .await?;

        let plans_count = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::Plan)
            .count();
        let screenshots_count = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::Screenshot)
            .count();
        let test_results_count = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::TestResult)
            .count();
        let errors_count = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::ErrorReport)
            .count();

        Ok(ArtifactsSummary {
            total_count: artifacts.len(),
            plans_count,
            screenshots_count,
            test_results_count,
            errors_count,
        })
    }

    /// Store a plan artifact from agent output
    pub async fn store_plan_artifact(
        &self,
        execution_process_id: Uuid,
        title: String,
        plan_content: String,
        steps: Option<Vec<PlanStep>>,
    ) -> Result<ExecutionArtifact, ArtifactServiceError> {
        // Create the artifact
        let artifact = self
            .create_artifact(CreateExecutionArtifact {
                execution_process_id: Some(execution_process_id),
                artifact_type: ArtifactType::Plan,
                title,
                content: Some(plan_content),
                file_path: None,
                metadata: steps.as_ref().map(|s| serde_json::to_value(s).unwrap()),
            })
            .await?;

        // If steps provided, also create/update the task plan
        if let Some(steps) = steps {
            let existing = self.get_plan(execution_process_id).await?;
            if existing.is_some() {
                // Update existing plan
                if let Some(plan) = existing {
                    AgentTaskPlan::update_steps(&self.db.pool, plan.id, steps).await?;
                }
            } else {
                // Create new plan
                self.create_plan(CreateAgentTaskPlan {
                    execution_process_id,
                    steps,
                })
                .await?;
            }
        }

        Ok(artifact)
    }

    /// Store a screenshot artifact
    pub async fn store_screenshot(
        &self,
        execution_process_id: Uuid,
        title: String,
        file_path: String,
        metadata: Option<Value>,
    ) -> Result<ExecutionArtifact, ArtifactServiceError> {
        let artifact = self
            .create_artifact(CreateExecutionArtifact {
                execution_process_id: Some(execution_process_id),
                artifact_type: ArtifactType::Screenshot,
                title,
                content: None,
                file_path: Some(file_path),
                metadata,
            })
            .await?;

        Ok(artifact)
    }

    /// Store an error report artifact
    pub async fn store_error_report(
        &self,
        execution_process_id: Uuid,
        title: String,
        error_content: String,
        metadata: Option<Value>,
    ) -> Result<ExecutionArtifact, ArtifactServiceError> {
        let artifact = self
            .create_artifact(CreateExecutionArtifact {
                execution_process_id: Some(execution_process_id),
                artifact_type: ArtifactType::ErrorReport,
                title,
                content: Some(error_content),
                file_path: None,
                metadata,
            })
            .await?;

        Ok(artifact)
    }
}

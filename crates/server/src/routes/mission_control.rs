use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    agent_task_plan::AgentTaskPlan,
    execution_artifact::ExecutionArtifact,
    execution_process::ExecutionProcess,
    execution_slot::ProjectCapacity,
};
use deployment::Deployment;
use serde::Serialize;
use services::services::container::ContainerService;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Active execution with associated data for Mission Control
#[derive(Debug, Serialize, TS)]
pub struct ActiveExecutionInfo {
    pub process: ExecutionProcess,
    pub task_id: Uuid,
    pub task_title: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub executor: String,
    pub plan: Option<AgentTaskPlan>,
    pub artifacts_count: usize,
}

/// Mission Control dashboard data
#[derive(Debug, Serialize, TS)]
pub struct MissionControlDashboard {
    pub active_executions: Vec<ActiveExecutionInfo>,
    pub total_active: usize,
    pub by_project: Vec<ProjectExecutionSummary>,
}

#[derive(Debug, Serialize, TS)]
pub struct ProjectExecutionSummary {
    pub project_id: Uuid,
    pub project_name: String,
    pub active_count: usize,
    pub capacity: ProjectCapacity,
}

/// Get Mission Control dashboard with all active executions
pub async fn get_dashboard(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MissionControlDashboard>>, ApiError> {
    let pool = &deployment.db().pool;

    // Get all running execution processes
    let running_processes = ExecutionProcess::find_running(pool).await?;

    let mut active_executions = Vec::new();
    let mut project_map: std::collections::HashMap<Uuid, (String, usize, Option<ProjectCapacity>)> =
        std::collections::HashMap::new();

    for process in running_processes {
        // Load execution context
        if let Ok(ctx) = ExecutionProcess::load_context(pool, process.id).await {
            // Get plan if exists
            let plan = AgentTaskPlan::find_by_execution_process(pool, process.id)
                .await
                .ok()
                .flatten();

            // Get artifacts count
            let artifacts = ExecutionArtifact::find_by_execution_process(pool, process.id)
                .await
                .unwrap_or_default();

            let project_id = ctx.task.project_id;

            // Update project summary
            let entry = project_map.entry(project_id).or_insert_with(|| {
                let project_name = String::new(); // Will be populated below
                (project_name, 0, None)
            });
            entry.1 += 1;

            // Get project name if not already fetched
            if entry.0.is_empty() {
                if let Ok(Some(project)) =
                    db::models::project::Project::find_by_id(pool, project_id).await
                {
                    entry.0 = project.name;
                    // Get capacity for this project
                    if let Ok(capacity) = deployment.container().get_project_capacity(project_id).await
                    {
                        entry.2 = Some(capacity);
                    }
                }
            }

            active_executions.push(ActiveExecutionInfo {
                process,
                task_id: ctx.task.id,
                task_title: ctx.task.title,
                project_id,
                project_name: entry.0.clone(),
                executor: ctx.task_attempt.executor,
                plan,
                artifacts_count: artifacts.len(),
            });
        }
    }

    // Build project summaries
    let by_project: Vec<ProjectExecutionSummary> = project_map
        .into_iter()
        .map(|(project_id, (project_name, active_count, capacity))| ProjectExecutionSummary {
            project_id,
            project_name,
            active_count,
            capacity: capacity.unwrap_or(ProjectCapacity {
                project_id,
                max_concurrent_agents: 3,
                max_concurrent_browser_agents: 1,
                active_agent_slots: active_count as i32,
                active_browser_slots: 0,
                available_agent_slots: 3 - active_count as i32,
                available_browser_slots: 1,
            }),
        })
        .collect();

    let total_active = active_executions.len();

    Ok(ResponseJson(ApiResponse::success(MissionControlDashboard {
        active_executions,
        total_active,
        by_project,
    })))
}

/// Get artifacts for a specific execution process
pub async fn get_execution_artifacts(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionArtifact>>>, ApiError> {
    let pool = &deployment.db().pool;

    let artifacts = ExecutionArtifact::find_by_execution_process(pool, execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(artifacts)))
}

/// Get task plan for a specific execution process
pub async fn get_execution_plan(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Option<AgentTaskPlan>>>, ApiError> {
    let pool = &deployment.db().pool;

    let plan = AgentTaskPlan::find_by_execution_process(pool, execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(plan)))
}

/// Get all active task plans
pub async fn get_active_plans(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AgentTaskPlan>>>, ApiError> {
    let pool = &deployment.db().pool;

    let plans = AgentTaskPlan::find_active(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(plans)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/mission-control", get(get_dashboard))
        .route(
            "/mission-control/executions/{execution_id}/artifacts",
            get(get_execution_artifacts),
        )
        .route(
            "/mission-control/executions/{execution_id}/plan",
            get(get_execution_plan),
        )
        .route("/mission-control/plans/active", get(get_active_plans))
}

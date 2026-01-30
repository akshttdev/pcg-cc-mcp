//! Agent Configuration API Routes
//!
//! Manages agent execution configurations, execution profiles, and Ralph loop settings.
//! This is a scalable system that works for all agents.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use db::models::agent_execution_config::{
    AgentExecutionConfig, AgentExecutionProfile, BackpressureDefinition,
    CreateAgentExecutionConfig, UpdateAgentExecutionConfig, RalphLoopState,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::ralph::{RalphService, StartRalphRequest};
use ts_rs::TS;
use uuid::Uuid;

use crate::DeploymentImpl;

// ============================================================================
// ROUTES
// ============================================================================

/// Agent configuration and Ralph loop routes
pub fn routes() -> Router<DeploymentImpl> {
    Router::new()
        // Execution Profiles (templates)
        .route("/execution-profiles", get(list_execution_profiles))
        .route("/execution-profiles/{id}", get(get_execution_profile))

        // Agent Execution Config
        .route("/agents/{agent_id}/execution-config",
            get(get_agent_execution_config)
            .post(create_agent_execution_config)
            .put(update_agent_execution_config))

        // Backpressure Definitions
        .route("/backpressure-definitions", get(list_backpressure_definitions))
        .route("/backpressure-definitions/by-type/{project_type}",
            get(get_backpressure_by_project_type))

        // Ralph Loop Management
        .route("/ralph/start", post(start_ralph_loop))
        .route("/ralph/loops/{loop_id}", get(get_ralph_loop_state))
        .route("/ralph/loops/{loop_id}/cancel", post(cancel_ralph_loop))
        .route("/ralph/loops/{loop_id}/iterations", get(get_ralph_iterations))
        .route("/ralph/by-attempt/{task_attempt_id}", get(get_ralph_by_attempt))

        // Resolve config for a task (preview what config would be used)
        .route("/tasks/{task_id}/resolved-ralph-config", get(resolve_task_ralph_config))
}

// ============================================================================
// EXECUTION PROFILES
// ============================================================================

/// List all execution profiles
async fn list_execution_profiles(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let profiles = AgentExecutionProfile::list_all(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(profiles))
}

/// Get a specific execution profile
async fn get_execution_profile(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let profile = AgentExecutionProfile::find_by_id(&deployment.db().pool, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Profile not found".to_string()))?;

    Ok(Json(profile))
}

// ============================================================================
// AGENT EXECUTION CONFIG
// ============================================================================

/// Get execution config for an agent
async fn get_agent_execution_config(
    State(deployment): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let config = AgentExecutionConfig::find_by_agent_id(&deployment.db().pool, agent_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match config {
        Some(c) => Ok(Json(AgentConfigResponse::Found(c))),
        None => Ok(Json(AgentConfigResponse::NotConfigured { agent_id })),
    }
}

/// Create execution config for an agent
async fn create_agent_execution_config(
    State(deployment): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
    Json(mut data): Json<CreateAgentExecutionConfig>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Ensure agent_id matches path
    data.agent_id = agent_id;

    let config = AgentExecutionConfig::create(&deployment.db().pool, data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(config)))
}

/// Update execution config for an agent
async fn update_agent_execution_config(
    State(deployment): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
    Json(data): Json<UpdateAgentExecutionConfig>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Find existing config
    let existing = AgentExecutionConfig::find_by_agent_id(&deployment.db().pool, agent_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Config not found".to_string()))?;

    let config = AgentExecutionConfig::update(&deployment.db().pool, &existing.id, data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(config))
}

#[derive(Serialize, Deserialize, TS)]
#[serde(tag = "type")]
pub enum AgentConfigResponse {
    Found(AgentExecutionConfig),
    NotConfigured { agent_id: Uuid },
}

// ============================================================================
// BACKPRESSURE DEFINITIONS
// ============================================================================

/// List all backpressure definitions
async fn list_backpressure_definitions(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let definitions = BackpressureDefinition::find_all_active(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(definitions))
}

/// Get backpressure definitions for a project type
async fn get_backpressure_by_project_type(
    State(deployment): State<DeploymentImpl>,
    Path(project_type): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let definitions = BackpressureDefinition::find_for_project_type(&deployment.db().pool, &project_type)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(definitions))
}

// ============================================================================
// RALPH LOOP MANAGEMENT
// ============================================================================

/// Start a new Ralph loop execution
async fn start_ralph_loop(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<StartRalphRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ralph_service = RalphService::new(deployment.db().clone(), deployment.git().clone());

    let loop_state = ralph_service
        .start_ralph_loop(request, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(loop_state)))
}

/// Get Ralph loop state by ID
async fn get_ralph_loop_state(
    State(deployment): State<DeploymentImpl>,
    Path(loop_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ralph_service = RalphService::new(deployment.db().clone(), deployment.git().clone());

    let state = ralph_service
        .get_loop_state(&loop_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Loop not found".to_string()))?;

    Ok(Json(state))
}

/// Cancel a running Ralph loop
async fn cancel_ralph_loop(
    State(deployment): State<DeploymentImpl>,
    Path(loop_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ralph_service = RalphService::new(deployment.db().clone(), deployment.git().clone());

    ralph_service
        .cancel_loop(&loop_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get iterations for a Ralph loop
async fn get_ralph_iterations(
    State(deployment): State<DeploymentImpl>,
    Path(loop_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ralph_service = RalphService::new(deployment.db().clone(), deployment.git().clone());

    let iterations = ralph_service
        .get_iterations(&loop_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(iterations))
}

/// Get Ralph loop by task attempt ID
async fn get_ralph_by_attempt(
    State(deployment): State<DeploymentImpl>,
    Path(task_attempt_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ralph_service = RalphService::new(deployment.db().clone(), deployment.git().clone());

    let state = ralph_service
        .get_loop_state_by_attempt(task_attempt_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match state {
        Some(s) => Ok(Json(RalphByAttemptResponse::Found(s))),
        None => Ok(Json(RalphByAttemptResponse::NotFound { task_attempt_id })),
    }
}

#[derive(Serialize, Deserialize, TS)]
#[serde(tag = "type")]
pub enum RalphByAttemptResponse {
    Found(RalphLoopState),
    NotFound { task_attempt_id: Uuid },
}

// ============================================================================
// RESOLVED CONFIG
// ============================================================================

/// Resolve Ralph config for a task (preview what would be used)
async fn resolve_task_ralph_config(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use db::models::task::Task;

    let task = Task::find_by_id(&deployment.db().pool, task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    let ralph_service = RalphService::new(deployment.db().clone(), deployment.git().clone());

    let config = ralph_service
        .resolve_config(&task)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(config))
}

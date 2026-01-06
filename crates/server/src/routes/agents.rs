use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use db::models::agent::{
    Agent, AgentStatus, AgentWithParsedFields, CreateAgent, UpdateAgent,
};
use deployment::Deployment;
use serde::Deserialize;
use services::services::agent_registry::AgentRegistryService;
use uuid::Uuid;

use crate::DeploymentImpl;

/// Agent management routes
pub fn routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/active", get(list_active_agents))
        .route("/agents/seed", post(seed_agents))
        .route("/agents/{id}", get(get_agent).put(update_agent).delete(delete_agent))
        .route("/agents/by-name/{name}", get(get_agent_by_name))
        .route("/agents/{id}/wallet", put(assign_wallet))
        .route("/agents/{id}/status", put(update_status))
}

/// List all agents
async fn list_agents(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agents = Agent::find_all(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: Vec<AgentWithParsedFields> = agents.into_iter().map(|a| a.into()).collect();
    Ok(Json(parsed))
}

/// List only active agents
async fn list_active_agents(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agents = AgentRegistryService::get_active_agents(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: Vec<AgentWithParsedFields> = agents.into_iter().map(|a| a.into()).collect();
    Ok(Json(parsed))
}

/// Seed core agents (Nora, Maci, Editron)
async fn seed_agents(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agents = AgentRegistryService::seed_core_agents(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: Vec<AgentWithParsedFields> = agents.into_iter().map(|a| a.into()).collect();
    Ok((StatusCode::CREATED, Json(parsed)))
}

/// Get agent by ID
async fn get_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = Agent::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match agent {
        Some(a) => {
            let parsed: AgentWithParsedFields = a.into();
            Ok(Json(parsed))
        }
        None => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
    }
}

/// Get agent by short name
async fn get_agent_by_name(
    State(deployment): State<DeploymentImpl>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = AgentRegistryService::get_agent_by_name(&deployment.db().pool, &name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match agent {
        Some(a) => {
            let parsed: AgentWithParsedFields = a.into();
            Ok(Json(parsed))
        }
        None => Err((StatusCode::NOT_FOUND, format!("Agent '{}' not found", name))),
    }
}

/// Create a new agent
async fn create_agent(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateAgent>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = Agent::create(&deployment.db().pool, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();
    Ok((StatusCode::CREATED, Json(parsed)))
}

/// Update an existing agent
async fn update_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateAgent>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if agent exists
    let existing = Agent::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    let agent = Agent::update(&deployment.db().pool, id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();
    Ok(Json(parsed))
}

/// Delete an agent
async fn delete_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rows = Agent::delete(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if rows == 0 {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct AssignWalletRequest {
    wallet_address: String,
}

/// Assign Aptos wallet address to an agent
async fn assign_wallet(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(request): Json<AssignWalletRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = AgentRegistryService::assign_wallet(&deployment.db().pool, id, &request.wallet_address)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();
    Ok(Json(parsed))
}

#[derive(Debug, Deserialize)]
struct UpdateStatusRequest {
    status: AgentStatus,
}

/// Update agent status
async fn update_status(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateStatusRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if agent exists
    let existing = Agent::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    Agent::update_status(&deployment.db().pool, id, request.status)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return updated agent
    let agent = Agent::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();
    Ok(Json(parsed))
}

use axum::{
    extract::{Path, Query, State},
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
use ts_rs::TS;
use uuid::Uuid;

use crate::DeploymentImpl;

/// Query params for agent search/filter
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AgentSearchQuery {
    /// Search term for name, designation, or description
    pub q: Option<String>,
    /// Filter by status
    pub status: Option<AgentStatus>,
    /// Filter by capability
    pub capability: Option<String>,
    /// Sort by field
    pub sort_by: Option<String>,
    /// Sort direction (asc/desc)
    pub sort_dir: Option<String>,
}

/// Agent management routes
pub fn routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/search", get(search_agents))
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

/// Search and filter agents
async fn search_agents(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<AgentSearchQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    // Get all agents first, then filter in memory
    // (For a larger dataset, we'd want to use SQL filtering)
    let all_agents = Agent::find_all(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut parsed: Vec<AgentWithParsedFields> = all_agents.into_iter().map(|a| a.into()).collect();

    // Apply search filter
    if let Some(ref search) = query.q {
        let search_lower = search.to_lowercase();
        parsed.retain(|agent| {
            agent.short_name.to_lowercase().contains(&search_lower)
                || agent.designation.to_lowercase().contains(&search_lower)
                || agent.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&search_lower))
        });
    }

    // Apply status filter
    if let Some(ref status) = query.status {
        parsed.retain(|agent| &agent.status == status);
    }

    // Apply capability filter
    if let Some(ref capability) = query.capability {
        let cap_lower = capability.to_lowercase();
        parsed.retain(|agent| {
            agent.capabilities.as_ref().map_or(false, |caps| {
                caps.iter().any(|c| c.to_lowercase().contains(&cap_lower))
            })
        });
    }

    // Note: team filter not currently supported on AgentWithParsedFields
    // Would need to add team_id to the parsed struct if needed

    // Apply sorting
    let sort_by = query.sort_by.as_deref().unwrap_or("short_name");
    let sort_asc = query.sort_dir.as_deref() != Some("desc");

    match sort_by {
        "short_name" | "name" => {
            parsed.sort_by(|a, b| {
                let cmp = a.short_name.to_lowercase().cmp(&b.short_name.to_lowercase());
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        "designation" => {
            parsed.sort_by(|a, b| {
                let cmp = a.designation.to_lowercase().cmp(&b.designation.to_lowercase());
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        "status" => {
            parsed.sort_by(|a, b| {
                let cmp = format!("{:?}", a.status).cmp(&format!("{:?}", b.status));
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        "priority" | "priority_weight" => {
            parsed.sort_by(|a, b| {
                let cmp = a.priority_weight.unwrap_or(0).cmp(&b.priority_weight.unwrap_or(0));
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        "tasks_completed" => {
            parsed.sort_by(|a, b| {
                let cmp = a.tasks_completed.unwrap_or(0).cmp(&b.tasks_completed.unwrap_or(0));
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        _ => {}
    }

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

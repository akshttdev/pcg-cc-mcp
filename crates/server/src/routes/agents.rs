use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use db::models::agent::{Agent, AgentStatus, AgentWithParsedFields, CreateAgent, UpdateAgent};
use deployment::Deployment;
use serde::Deserialize;
use services::services::agent_registry::AgentRegistryService;
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, middleware::access_control::AccessContext};

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
        .route(
            "/agents/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
        .route("/agents/{id}/profile", get(get_agent_profile))
        .route("/agents/by-name/{name}", get(get_agent_by_name))
        .route("/agents/{id}/wallet", put(assign_wallet))
        .route("/agents/{id}/status", put(update_status))
}

/// List agents visible to the current user
/// Admin users see all agents; regular users see only system-tier + their own agents
async fn list_agents(
    State(deployment): State<DeploymentImpl>,
    access_ctx: Option<axum::Extension<AccessContext>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    let agents = match access_ctx {
        Some(axum::Extension(ctx)) if ctx.is_admin => {
            // Admin sees all agents
            Agent::find_all(pool).await
        }
        Some(axum::Extension(ctx)) => {
            // Regular user sees system-tier + own agents
            Agent::find_visible_for_user(pool, ctx.user_id.as_ref()).await
        }
        None => {
            // No auth context (shouldn't happen behind protected routes, but fallback)
            Agent::find_all(pool).await
        }
    }
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: Vec<AgentWithParsedFields> = agents.into_iter().map(|a| a.into()).collect();
    Ok(Json(ApiResponse::<_, ()>::success(parsed)))
}

/// List only active agents
async fn list_active_agents(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agents = AgentRegistryService::get_active_agents(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: Vec<AgentWithParsedFields> = agents.into_iter().map(|a| a.into()).collect();
    Ok(Json(ApiResponse::<_, ()>::success(parsed)))
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
                || agent
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&search_lower))
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
            agent
                .capabilities
                .as_ref()
                .is_some_and(|caps| caps.iter().any(|c| c.to_lowercase().contains(&cap_lower)))
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
                let cmp = a
                    .short_name
                    .to_lowercase()
                    .cmp(&b.short_name.to_lowercase());
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        "designation" => {
            parsed.sort_by(|a, b| {
                let cmp = a
                    .designation
                    .to_lowercase()
                    .cmp(&b.designation.to_lowercase());
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
                let cmp = a
                    .priority_weight
                    .unwrap_or(0)
                    .cmp(&b.priority_weight.unwrap_or(0));
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        "tasks_completed" => {
            parsed.sort_by(|a, b| {
                let cmp = a
                    .tasks_completed
                    .unwrap_or(0)
                    .cmp(&b.tasks_completed.unwrap_or(0));
                if sort_asc { cmp } else { cmp.reverse() }
            });
        }
        _ => {}
    }

    Ok(Json(ApiResponse::<_, ()>::success(parsed)))
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
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = Agent::find_by_id(&deployment.db().pool, &id)
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
    Path(id): Path<String>,
    Json(data): Json<UpdateAgent>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if agent exists
    let existing = Agent::find_by_id(&deployment.db().pool, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    let agent = Agent::update(&deployment.db().pool, &id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();
    Ok(Json(parsed))
}

/// Delete an agent
async fn delete_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rows = Agent::delete(&deployment.db().pool, &id)
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
    Path(id): Path<String>,
    Json(request): Json<AssignWalletRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent =
        AgentRegistryService::assign_wallet(&deployment.db().pool, &id, &request.wallet_address)
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
    Path(id): Path<String>,
    Json(request): Json<UpdateStatusRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if agent exists
    let existing = Agent::find_by_id(&deployment.db().pool, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    Agent::update_status(&deployment.db().pool, &id, request.status)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return updated agent
    let agent = Agent::find_by_id(&deployment.db().pool, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();
    Ok(Json(parsed))
}

/// GET /api/agents/:id/profile — Agent capability profile with execution stats
async fn get_agent_profile(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    let agent = Agent::find_by_id(pool, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    let parsed: AgentWithParsedFields = agent.into();

    // Query execution stats from task_attempts
    let agent_name = &parsed.short_name;

    #[derive(sqlx::FromRow)]
    struct Stats {
        total: i64,
        completed: i64,
        failed: i64,
        in_progress: i64,
    }

    let stats = sqlx::query_as::<_, Stats>(
        r#"SELECT
            COUNT(*) as total,
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
            SUM(CASE WHEN status = 'in_progress' THEN 1 ELSE 0 END) as in_progress
        FROM task_attempts
        WHERE agent_name = ?1"#,
    )
    .bind(agent_name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (total, completed, failed, in_progress) = stats
        .map(|s| (s.total, s.completed, s.failed, s.in_progress))
        .unwrap_or((0, 0, 0, 0));

    let success_rate = if total > 0 {
        (completed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Get recent task attempt history
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct RecentAttempt {
        id: String,
        task_id: String,
        status: String,
        created_at: String,
    }

    let recent = sqlx::query_as::<_, RecentAttempt>(
        r#"SELECT id, task_id, status, created_at
        FROM task_attempts
        WHERE agent_name = ?1
        ORDER BY created_at DESC
        LIMIT 10"#,
    )
    .bind(agent_name)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Load platform MCP servers available to this agent
    let mcp_tools: Vec<String> = if let Some(tools) = &parsed.tools {
        tools.iter().map(|t| t.to_string()).collect()
    } else {
        vec![]
    };

    Ok(Json(serde_json::json!({
        "agent": parsed,
        "execution_stats": {
            "total_attempts": total,
            "completed": completed,
            "failed": failed,
            "in_progress": in_progress,
            "success_rate": format!("{:.1}%", success_rate),
        },
        "recent_attempts": recent,
        "mcp_tools": mcp_tools,
        "platform_mcp_servers": ["orcha_task_server", "duck_kanban", "context7", "playwright"],
    })))
}

//! ORCHA — User Orchestrator Agent routes
//!
//! Each user gets a personal orchestrator agent:
//! - Admin → NORA (the executive assistant)
//! - Regular users → ORCHA-{username}
//!
//! The orchestrator delegates to sub-agents found via `parent_agent_id`.

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use crate::middleware::access_control::get_current_user;
use db::models::agent::{Agent, AgentBrief, AgentStatus, AutonomyLevel, CreateAgent};
use deployment::Deployment;

/// Response for GET /orcha/status
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrchaStatusResponse {
    /// Orchestrator display name (e.g. "NORA" or "ORCHA-alice")
    pub orchestrator_name: String,
    /// Agent ID in the database
    pub agent_id: Uuid,
    /// Device / session info
    pub device: String,
    /// Whether this is the admin orchestrator
    pub is_admin: bool,
    /// Sub-agents under this orchestrator
    pub sub_agents: Vec<SubAgentBrief>,
}

/// Brief sub-agent info
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SubAgentBrief {
    pub id: Uuid,
    pub short_name: String,
    pub designation: String,
    pub status: String,
}

/// Register ORCHA routes
pub fn orcha_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/orcha/status", get(get_orcha_status))
        .layer(axum::middleware::from_fn(
            crate::middleware::request_id_middleware,
        ))
}

/// GET /orcha/status — return the calling user's orchestrator identity
pub async fn get_orcha_status(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<OrchaStatusResponse>, ApiError> {
    let pool = &state.db().pool;

    // Authenticate caller
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let access_ctx = get_current_user(&state, auth_header, cookie_header).await?;

    // Resolve username from the users table
    let user_id_bytes = access_ctx.user_id.as_bytes().to_vec();
    let username: String = sqlx::query_scalar(
        "SELECT username FROM users WHERE id = ?"
    )
    .bind(&user_id_bytes)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?
    .unwrap_or_else(|| "unknown".to_string());

    // Determine which orchestrator this user gets
    let (orchestrator_name, agent) = if access_ctx.is_admin {
        // Admin → NORA
        let nora = Agent::find_by_short_name(pool, "NORA")
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
        ("NORA".to_string(), nora)
    } else {
        // Regular user → ORCHA-{username}
        let orcha_name = format!("ORCHA-{}", username);
        let agent = Agent::find_by_short_name(pool, &orcha_name)
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
        (orcha_name, agent)
    };

    let agent = agent.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Orchestrator '{}' not found. Server may need to restart to seed agents.",
            orchestrator_name
        ))
    })?;

    // Fetch sub-agents (agents whose parent_agent_id matches this orchestrator)
    let sub_agents: Vec<SubAgentBrief> = sqlx::query_as::<_, (Vec<u8>, String, String, String)>(
        r#"SELECT id, short_name, designation, status FROM agents
           WHERE parent_agent_id = ? AND status != 'inactive'
           ORDER BY short_name"#,
    )
    .bind(agent.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter_map(|(id_bytes, short_name, designation, status)| {
        let id = Uuid::from_slice(&id_bytes).ok()?;
        Some(SubAgentBrief {
            id,
            short_name,
            designation,
            status,
        })
    })
    .collect();

    // Device info — hostname
    let device = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-device".to_string());

    Ok(Json(OrchaStatusResponse {
        orchestrator_name,
        agent_id: agent.id,
        device,
        is_admin: access_ctx.is_admin,
        sub_agents,
    }))
}

/// Ensure every active user has an orchestrator agent record.
///
/// Called once on server boot:
/// - NORA already exists (seeded as core agent)
/// - For each non-admin active user, create ORCHA-{username} if missing
pub async fn ensure_orcha_agents(pool: &SqlitePool) -> Result<usize, String> {
    // Fetch all active users
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct UserRow {
        id: Vec<u8>,
        username: String,
        is_admin: bool,
    }

    let users: Vec<UserRow> = sqlx::query_as(
        "SELECT id, username, is_admin FROM users WHERE is_active = 1"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch users: {}", e))?;

    let mut created = 0usize;

    for user in &users {
        if user.is_admin {
            // NORA is seeded by AgentRegistryService::seed_core_agents — skip
            continue;
        }

        let orcha_name = format!("ORCHA-{}", user.username);

        // Check if already exists
        match Agent::find_by_short_name(pool, &orcha_name).await {
            Ok(Some(_)) => continue, // already exists
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("Failed to lookup agent '{}': {}", orcha_name, e);
                continue;
            }
        }

        // Resolve parent (NORA) so sub-agent hierarchy works
        let nora_id = Agent::find_by_short_name(pool, "NORA")
            .await
            .ok()
            .flatten()
            .map(|a| a.id);

        let create = CreateAgent {
            wallet_address: None,
            short_name: orcha_name.clone(),
            designation: "User Orchestrator".to_string(),
            description: Some(format!(
                "Personal orchestrator for {}. Delegates tasks to sub-agents.",
                user.username
            )),
            personality: None,
            voice_style: None,
            avatar_url: None,
            capabilities: Some(vec![
                "task-delegation".to_string(),
                "scheduling".to_string(),
                "monitoring".to_string(),
            ]),
            tools: None,
            functions: None,
            default_model: None,
            fallback_models: None,
            model_config: None,
            status: Some(AgentStatus::Active),
            autonomy_level: Some(AutonomyLevel::Supervised),
            max_concurrent_tasks: Some(5),
            priority_weight: None,
            parent_agent_id: nora_id,
            team_id: None,
            created_by: Some("system".to_string()),
            agent_tier: Some("system".to_string()),
            owner_id: None,
        };

        match Agent::create(pool, &create).await {
            Ok(agent) => {
                tracing::info!("Created orchestrator agent '{}' (ID: {})", orcha_name, agent.id);
                created += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to create orchestrator '{}': {}", orcha_name, e);
            }
        }
    }

    Ok(created)
}

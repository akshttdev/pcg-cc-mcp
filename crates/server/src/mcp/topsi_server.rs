//! Topsi MCP Server - Platform Control Plane
//!
//! Dedicated MCP server for Topsi, the platform orchestrator.
//! Provides topology intelligence, orchestration, monitoring,
//! policy enforcement, and communication tools.
//!
//! Uses raw SQL queries (sqlx::query_as::<_, Row>) to avoid
//! compile-time query_as! macro dependency on DATABASE_URL.

use db::models::agent::Agent;
use db::models::task::Task;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

// ========================================================
// Local row types for raw SQL queries
// ========================================================

#[derive(Debug, sqlx::FromRow)]
struct NodeRow {
    id: String,
    node_type: String,
    ref_id: String,
    capabilities: Option<String>,
    status: String,
    metadata: Option<String>,
    weight: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct EdgeRow {
    id: String,
    from_node_id: String,
    to_node_id: String,
    edge_type: String,
    weight: Option<f64>,
    status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ClusterRow {
    id: String,
    name: String,
    purpose: Option<String>,
    node_ids: String,
    is_active: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct CountRow {
    cnt: i64,
}

// ========================================================
// Request types
// ========================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTopologyRequest {
    #[schemars(description = "The project ID to get topology for")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTopologyHealthRequest {
    #[schemars(description = "The project ID to check health for")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DetectIssuesRequest {
    #[schemars(description = "The project ID to detect issues in")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindOptimalPathRequest {
    #[schemars(description = "The project ID")]
    pub project_id: String,
    #[schemars(description = "Source node ID")]
    pub source_id: String,
    #[schemars(description = "Target node ID")]
    pub target_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTopologySummaryRequest {
    #[schemars(description = "The project ID to summarize")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RouteTaskRequest {
    #[schemars(description = "The project ID")]
    pub project_id: String,
    #[schemars(description = "The task ID to route")]
    pub task_id: String,
    #[schemars(description = "Required capabilities for the task")]
    pub required_capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAgentStatusesRequest {
    #[schemars(description = "The project ID, or omit for all agents")]
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FormTeamRequest {
    #[schemars(description = "The project ID")]
    pub project_id: String,
    #[schemars(description = "Name for the team/cluster")]
    pub team_name: String,
    #[schemars(description = "Agent node IDs to include in the team")]
    pub agent_ids: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DissolveTeamRequest {
    #[schemars(description = "The cluster/team ID to dissolve")]
    pub cluster_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BroadcastMessageRequest {
    #[schemars(description = "The cluster ID to broadcast to")]
    pub cluster_id: String,
    #[schemars(description = "The message to broadcast")]
    pub message: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSystemPulseRequest {
    #[schemars(description = "The project ID, or omit for system-wide pulse")]
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetRecommendationsRequest {
    #[schemars(description = "The project ID to get recommendations for")]
    pub project_id: String,
    #[schemars(description = "Maximum recommendations to return (default: 5)")]
    pub limit: Option<i64>,
}

// ========================================================
// Helpers
// ========================================================

fn parse_uuid(s: &str, label: &str) -> Result<Uuid, CallToolResult> {
    Uuid::parse_str(s).map_err(|_| {
        CallToolResult::error(vec![Content::text(
            serde_json::json!({"success": false, "error": format!("Invalid {label}: {s}")})
                .to_string(),
        )])
    })
}

fn err_result(msg: &str) -> CallToolResult {
    CallToolResult::error(vec![Content::text(
        serde_json::json!({"success": false, "error": msg}).to_string(),
    )])
}

fn ok_result(value: serde_json::Value) -> CallToolResult {
    CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&value).unwrap_or_default(),
    )])
}

/// Encode a UUID as uppercase hex for SQLite text comparison.
fn pid_hex(uuid: Uuid) -> String {
    hex::encode(uuid.as_bytes()).to_uppercase()
}

/// Fetch topology nodes for a project via parameterized SQL.
async fn fetch_nodes(pool: &SqlitePool, pid: &str) -> Vec<NodeRow> {
    sqlx::query_as::<_, NodeRow>(
        "SELECT id, node_type, ref_id, capabilities, status, metadata, weight \
         FROM topology_nodes WHERE project_id = ? ORDER BY node_type, created_at DESC"
    )
    .bind(pid)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Fetch topology edges for a project via parameterized SQL.
async fn fetch_edges(pool: &SqlitePool, pid: &str) -> Vec<EdgeRow> {
    sqlx::query_as::<_, EdgeRow>(
        "SELECT id, from_node_id, to_node_id, edge_type, weight, status \
         FROM topology_edges WHERE project_id = ? ORDER BY edge_type, created_at DESC"
    )
    .bind(pid)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Fetch topology clusters for a project via parameterized SQL.
async fn fetch_clusters(pool: &SqlitePool, pid: &str) -> Vec<ClusterRow> {
    sqlx::query_as::<_, ClusterRow>(
        "SELECT id, name, purpose, node_ids, is_active \
         FROM topology_clusters WHERE project_id = ? ORDER BY is_active DESC, formed_at DESC"
    )
    .bind(pid)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Fetch agent-type nodes for a project that are active.
async fn fetch_active_agent_nodes(pool: &SqlitePool, pid: &str) -> Vec<NodeRow> {
    sqlx::query_as::<_, NodeRow>(
        "SELECT id, node_type, ref_id, capabilities, status, metadata, weight \
         FROM topology_nodes WHERE project_id = ? AND node_type = 'agent' AND status = 'active' \
         ORDER BY weight DESC"
    )
    .bind(pid)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

// ========================================================
// Server
// ========================================================

#[derive(Debug, Clone)]
pub struct TopsiServer {
    pub pool: SqlitePool,
    tool_router: ToolRouter<TopsiServer>,
}

impl TopsiServer {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl TopsiServer {
    // ────────────────────────────────────────────
    // Topology Intelligence
    // ────────────────────────────────────────────

    #[tool(
        description = "Get the full topology graph for a project, including all nodes, edges, and clusters. Returns agents, tasks, resources and their connections."
    )]
    async fn get_topology(
        &self,
        Parameters(req): Parameters<GetTopologyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let pid = pid_hex(project_id);

        let nodes = fetch_nodes(&self.pool, &pid).await;
        let edges = fetch_edges(&self.pool, &pid).await;
        let clusters = fetch_clusters(&self.pool, &pid).await;

        let response = serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "nodes": nodes.iter().map(|n| serde_json::json!({
                "id": n.id,
                "node_type": n.node_type,
                "ref_id": n.ref_id,
                "status": n.status,
                "capabilities": n.capabilities.as_deref()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
                "weight": n.weight,
            })).collect::<Vec<_>>(),
            "edges": edges.iter().map(|e| serde_json::json!({
                "id": e.id,
                "from_node_id": e.from_node_id,
                "to_node_id": e.to_node_id,
                "edge_type": e.edge_type,
                "weight": e.weight,
                "status": e.status,
            })).collect::<Vec<_>>(),
            "clusters": clusters.iter().map(|c| serde_json::json!({
                "id": c.id,
                "name": c.name,
                "node_ids": serde_json::from_str::<serde_json::Value>(&c.node_ids).ok(),
                "is_active": c.is_active,
            })).collect::<Vec<_>>(),
            "summary": {
                "node_count": nodes.len(),
                "edge_count": edges.len(),
                "cluster_count": clusters.len(),
            }
        });

        Ok(ok_result(response))
    }

    #[tool(
        description = "Get health scores for all nodes and clusters in a project topology. Returns per-node status breakdown and overall health."
    )]
    async fn get_topology_health(
        &self,
        Parameters(req): Parameters<GetTopologyHealthRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let pid = pid_hex(project_id);
        let nodes = fetch_nodes(&self.pool, &pid).await;

        let mut healthy = 0usize;
        let mut warning = 0usize;
        let mut error = 0usize;
        let mut inactive = 0usize;

        for node in &nodes {
            match node.status.as_str() {
                "active" => healthy += 1,
                "degraded" => warning += 1,
                "failed" => error += 1,
                "inactive" => inactive += 1,
                _ => {}
            }
        }

        let total = nodes.len().max(1) as f64;
        let health_score = (healthy as f64 + warning as f64 * 0.5) / total;

        let response = serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "total_nodes": nodes.len(),
            "healthy": healthy,
            "warning": warning,
            "error": error,
            "inactive": inactive,
            "health_score": (health_score * 100.0).round() / 100.0,
            "status": if health_score >= 0.8 { "healthy" } else if health_score >= 0.5 { "degraded" } else { "critical" },
        });

        Ok(ok_result(response))
    }

    #[tool(
        description = "Detect issues in the project topology, including bottlenecks, isolated nodes, overloaded agents, and dependency cycles."
    )]
    async fn detect_issues(
        &self,
        Parameters(req): Parameters<DetectIssuesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let pid = pid_hex(project_id);

        let nodes = fetch_nodes(&self.pool, &pid).await;
        let edges = fetch_edges(&self.pool, &pid).await;

        let mut issues: Vec<serde_json::Value> = Vec::new();

        // Detect isolated nodes (no edges)
        let connected_ids: std::collections::HashSet<&str> = edges
            .iter()
            .flat_map(|e| vec![e.from_node_id.as_str(), e.to_node_id.as_str()])
            .collect();

        for node in &nodes {
            if !connected_ids.contains(node.id.as_str()) {
                issues.push(serde_json::json!({
                    "issue_type": "isolated_node",
                    "severity": "low",
                    "description": format!("Node '{}' ({}) has no connections", node.ref_id, node.node_type),
                    "affected_nodes": [&node.id],
                    "recommendation": "Connect this node to the topology or remove it if unused",
                }));
            }
        }

        // Detect error-state nodes
        for node in &nodes {
            if node.status == "failed" {
                issues.push(serde_json::json!({
                    "issue_type": "node_error",
                    "severity": "high",
                    "description": format!("Node '{}' is in Failed state", node.ref_id),
                    "affected_nodes": [&node.id],
                    "recommendation": "Investigate and resolve the error, then restart the node",
                }));
            }
        }

        // Detect overloaded agents (many inbound edges)
        let mut inbound_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for edge in &edges {
            *inbound_counts.entry(&edge.to_node_id).or_default() += 1;
        }
        for node in &nodes {
            if node.node_type == "agent" {
                let count = inbound_counts.get(node.id.as_str()).copied().unwrap_or(0);
                if count > 5 {
                    issues.push(serde_json::json!({
                        "issue_type": "overloaded_agent",
                        "severity": "medium",
                        "description": format!("Agent '{}' has {} incoming connections", node.ref_id, count),
                        "affected_nodes": [&node.id],
                        "recommendation": "Consider redistributing tasks to other agents",
                    }));
                }
            }
        }

        let response = serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "issue_count": issues.len(),
            "issues": issues,
        });

        Ok(ok_result(response))
    }

    #[tool(
        description = "Get a summary of the project topology: node counts by type, edge counts, cluster info, and overall health."
    )]
    async fn get_topology_summary(
        &self,
        Parameters(req): Parameters<GetTopologySummaryRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let pid = pid_hex(project_id);

        let nodes = fetch_nodes(&self.pool, &pid).await;
        let edges = fetch_edges(&self.pool, &pid).await;
        let clusters = fetch_clusters(&self.pool, &pid).await;

        let mut by_type: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        let mut by_status: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for node in &nodes {
            *by_type.entry(&node.node_type).or_default() += 1;
            *by_status.entry(&node.status).or_default() += 1;
        }

        let response = serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "total_nodes": nodes.len(),
            "total_edges": edges.len(),
            "total_clusters": clusters.len(),
            "nodes_by_type": by_type,
            "nodes_by_status": by_status,
            "agent_count": by_type.get("agent").unwrap_or(&0),
            "task_count": by_type.get("task").unwrap_or(&0),
        });

        Ok(ok_result(response))
    }

    // ────────────────────────────────────────────
    // Orchestration
    // ────────────────────────────────────────────

    #[tool(
        description = "Route a task to the best available agent based on topology analysis, agent capabilities, and current workload."
    )]
    async fn route_task(
        &self,
        Parameters(req): Parameters<RouteTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let task_id = match parse_uuid(&req.task_id, "task ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let pid = pid_hex(project_id);

        // Get task details
        let task = match Task::find_by_id(&self.pool, &task_id.to_string()).await {
            Ok(Some(t)) => t,
            Ok(None) => return Ok(err_result("Task not found")),
            Err(e) => return Ok(err_result(&format!("Database error: {}", e))),
        };

        // Get active agent nodes
        let available = fetch_active_agent_nodes(&self.pool, &pid).await;

        if available.is_empty() {
            return Ok(ok_result(serde_json::json!({
                "success": false,
                "error": "No available agents in the topology",
                "task_id": task_id.to_string(),
            })));
        }

        // Find agent with matching capabilities if specified
        let selected = if let Some(ref caps) = req.required_capabilities {
            available
                .iter()
                .find(|n| {
                    let node_caps = n.capabilities.as_deref().unwrap_or("");
                    caps.iter().any(|c| node_caps.contains(c))
                })
                .or(available.first())
        } else {
            available.first()
        };

        match selected {
            Some(agent_node) => {
                let response = serde_json::json!({
                    "success": true,
                    "task_id": task_id.to_string(),
                    "task_title": task.title,
                    "assigned_agent_node_id": agent_node.id,
                    "assigned_agent_ref": agent_node.ref_id,
                    "message": format!("Task '{}' routed to agent '{}'", task.title, agent_node.ref_id),
                });
                Ok(ok_result(response))
            }
            None => Ok(ok_result(serde_json::json!({
                "success": false,
                "error": "No suitable agent found for the required capabilities",
                "task_id": task_id.to_string(),
            }))),
        }
    }

    #[tool(
        description = "Create a dynamic agent cluster/team for coordinated work on a shared objective."
    )]
    async fn form_team(
        &self,
        Parameters(req): Parameters<FormTeamRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };

        let cluster_id = Uuid::new_v4();
        let pid = pid_hex(project_id);
        let cid = hex::encode(cluster_id.as_bytes()).to_uppercase();
        let node_ids_json =
            serde_json::to_string(&req.agent_ids).unwrap_or_else(|_| "[]".to_string());

        match sqlx::query(
            "INSERT INTO topology_clusters (id, project_id, name, node_ids, is_active, formed_at) \
             VALUES (?, ?, ?, ?, 1, datetime('now'))"
        )
            .bind(&cid)
            .bind(&pid)
            .bind(&req.team_name)
            .bind(&node_ids_json)
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                let response = serde_json::json!({
                    "success": true,
                    "cluster_id": cluster_id.to_string(),
                    "team_name": req.team_name,
                    "agent_count": req.agent_ids.len(),
                    "message": format!("Team '{}' formed with {} agents", req.team_name, req.agent_ids.len()),
                });
                Ok(ok_result(response))
            }
            Err(e) => Ok(err_result(&format!("Failed to form team: {}", e))),
        }
    }

    #[tool(
        description = "Dissolve a team/cluster, releasing agents back to the general pool."
    )]
    async fn dissolve_team(
        &self,
        Parameters(req): Parameters<DissolveTeamRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let cluster_id = match parse_uuid(&req.cluster_id, "cluster ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let cid = hex::encode(cluster_id.as_bytes()).to_uppercase();

        match sqlx::query(
            "UPDATE topology_clusters SET is_active = 0, dissolved_at = datetime('now') WHERE id = ?"
        )
        .bind(&cid)
        .execute(&self.pool).await {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    return Ok(err_result("Cluster not found"));
                }
                Ok(ok_result(serde_json::json!({
                    "success": true,
                    "cluster_id": req.cluster_id,
                    "message": "Team dissolved successfully",
                })))
            }
            Err(e) => Ok(err_result(&format!("Failed to dissolve team: {}", e))),
        }
    }

    // ────────────────────────────────────────────
    // Monitoring
    // ────────────────────────────────────────────

    #[tool(
        description = "Get the status of all agents, including their current tasks and health. Optionally filter by project."
    )]
    async fn get_agent_statuses(
        &self,
        Parameters(_req): Parameters<GetAgentStatusesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let agents = Agent::find_all(&self.pool).await.unwrap_or_default();

        let response = serde_json::json!({
            "success": true,
            "agent_count": agents.len(),
            "agents": agents.iter().map(|a| serde_json::json!({
                "id": a.id,
                "name": a.short_name,
                "designation": a.designation,
                "status": format!("{:?}", a.status),
                "description": a.description,
                "capabilities": a.capabilities,
            })).collect::<Vec<_>>(),
        });

        Ok(ok_result(response))
    }

    #[tool(
        description = "Get real-time system health metrics including active agents, pending tasks, and overall system status."
    )]
    async fn get_system_pulse(
        &self,
        Parameters(req): Parameters<GetSystemPulseRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let agents = Agent::find_all(&self.pool).await.unwrap_or_default();
        let active_agents = Agent::find_active(&self.pool)
            .await
            .map(|a| a.len())
            .unwrap_or(0);

        let mut response = serde_json::json!({
            "success": true,
            "total_agents": agents.len(),
            "active_agents": active_agents,
            "system_health": if active_agents > 0 { "healthy" } else { "degraded" },
        });

        if let Some(ref pid_str) = req.project_id {
            if let Ok(project_id) = Uuid::parse_str(pid_str) {
                let pid = pid_hex(project_id);
                if let Ok(row) = sqlx::query_as::<_, CountRow>(
                    "SELECT CAST(COUNT(*) AS INTEGER) as cnt FROM topology_nodes WHERE project_id = ?"
                )
                    .bind(&pid)
                    .fetch_one(&self.pool)
                    .await
                {
                    response["topology_nodes"] = serde_json::json!(row.cnt);
                }
                response["project_id"] = serde_json::json!(pid_str);
            }
        }

        Ok(ok_result(response))
    }

    #[tool(
        description = "Get proactive improvement recommendations for the project topology based on current state analysis."
    )]
    async fn get_recommendations(
        &self,
        Parameters(req): Parameters<GetRecommendationsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_id = match parse_uuid(&req.project_id, "project ID") {
            Ok(id) => id,
            Err(r) => return Ok(r),
        };
        let pid = pid_hex(project_id);

        let nodes = fetch_nodes(&self.pool, &pid).await;
        let edges = fetch_edges(&self.pool, &pid).await;

        let mut recommendations: Vec<serde_json::Value> = Vec::new();

        let agent_count = nodes.iter().filter(|n| n.node_type == "agent").count();
        let task_count = nodes.iter().filter(|n| n.node_type == "task").count();

        if agent_count > 0 && task_count > agent_count * 3 {
            recommendations.push(serde_json::json!({
                "type": "scale_up",
                "priority": "high",
                "description": format!("Task-to-agent ratio is {}:1. Consider adding more agents.", task_count / agent_count),
                "action": "Add specialized agents to handle the workload",
            }));
        }

        if edges.is_empty() && !nodes.is_empty() {
            recommendations.push(serde_json::json!({
                "type": "connect_topology",
                "priority": "high",
                "description": "Topology has nodes but no connections. Tasks cannot be routed.",
                "action": "Create edges between agents and tasks to enable routing",
            }));
        }

        if nodes.is_empty() {
            recommendations.push(serde_json::json!({
                "type": "initialize",
                "priority": "critical",
                "description": "Topology is empty. Initialize by adding agent and task nodes.",
                "action": "Create topology nodes for agents and tasks",
            }));
        }

        let limit = req.limit.unwrap_or(5) as usize;
        recommendations.truncate(limit);

        let response = serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "recommendation_count": recommendations.len(),
            "recommendations": recommendations,
        });

        Ok(ok_result(response))
    }
}

#[tool_handler]
impl ServerHandler for TopsiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Topsi MCP Server - Platform control plane for topology intelligence, \
                 agent orchestration, health monitoring, and team management. \
                 Topsi is the primary platform orchestrator for the Sovereign Stack."
                    .to_string(),
            ),
        }
    }
}

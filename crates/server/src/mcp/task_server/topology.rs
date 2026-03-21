#![allow(dead_code)]
use std::collections::HashMap;

use db::models::{
    agent::{Agent, AgentStatus},
    project::Project,
};
use rmcp::{ErrorData, handler::server::tool::Parameters, model::CallToolResult, tool};
use serde_json::Value;

use super::{TaskServer, helpers::*, types::*};

impl TaskServer {
    #[tool(
        description = "Get the topology graph (nodes, edges, clusters) for a project. Optionally filter by node/edge type."
    )]
    pub(super) async fn get_topology(
        &self,
        Parameters(req): Parameters<GetTopologyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let active_only = req.active_only.unwrap_or(true);
        let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

        // Fetch nodes
        let nodes_query = if let Some(ref nt) = req.node_type {
            format!(
                "SELECT id, project_id, node_type, ref_id, label, capabilities, status, metadata, weight, created_at, updated_at \
                 FROM topology_nodes WHERE project_id = '{}' AND node_type = '{}' {} ORDER BY created_at",
                pid_hex,
                nt,
                if active_only {
                    "AND status = 'active'"
                } else {
                    ""
                }
            )
        } else {
            format!(
                "SELECT id, project_id, node_type, ref_id, label, capabilities, status, metadata, weight, created_at, updated_at \
                 FROM topology_nodes WHERE project_id = '{}' {} ORDER BY created_at",
                pid_hex,
                if active_only {
                    "AND status = 'active'"
                } else {
                    ""
                }
            )
        };

        #[derive(Debug, sqlx::FromRow)]
        struct NodeRow {
            id: String,
            node_type: String,
            ref_id: Option<String>,
            label: String,
            capabilities: Option<String>,
            status: String,
            weight: Option<f64>,
        }

        let node_rows = sqlx::query_as::<_, NodeRow>(&nodes_query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let nodes_json: Vec<Value> = node_rows
            .iter()
            .map(|n| serde_json::json!({
                "id": n.id,
                "node_type": n.node_type,
                "ref_id": n.ref_id,
                "label": n.label,
                "capabilities": n.capabilities.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "status": n.status,
                "weight": n.weight,
            }))
            .collect();

        // Fetch edges
        let edges_query = if let Some(ref et) = req.edge_type {
            format!(
                "SELECT id, from_node_id, to_node_id, edge_type, weight, status, metadata \
                 FROM topology_edges WHERE project_id = '{}' AND edge_type = '{}' {} ORDER BY created_at",
                pid_hex,
                et,
                if active_only {
                    "AND status = 'active'"
                } else {
                    ""
                }
            )
        } else {
            format!(
                "SELECT id, from_node_id, to_node_id, edge_type, weight, status, metadata \
                 FROM topology_edges WHERE project_id = '{}' {} ORDER BY created_at",
                pid_hex,
                if active_only {
                    "AND status = 'active'"
                } else {
                    ""
                }
            )
        };

        #[derive(Debug, sqlx::FromRow)]
        struct EdgeRow {
            id: String,
            from_node_id: String,
            to_node_id: String,
            edge_type: String,
            weight: Option<f64>,
            status: String,
        }

        let edge_rows = sqlx::query_as::<_, EdgeRow>(&edges_query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let edges_json: Vec<Value> = edge_rows
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "from_node_id": e.from_node_id,
                    "to_node_id": e.to_node_id,
                    "edge_type": e.edge_type,
                    "weight": e.weight,
                    "status": e.status,
                })
            })
            .collect();

        // Fetch clusters
        let clusters_query = format!(
            "SELECT id, name, purpose, node_ids, leader_node_id, is_active \
             FROM topology_clusters WHERE project_id = '{}' {}",
            pid_hex,
            if active_only { "AND is_active = 1" } else { "" }
        );

        #[derive(Debug, sqlx::FromRow)]
        struct ClusterRow {
            id: String,
            name: String,
            purpose: Option<String>,
            node_ids: Option<String>,
            leader_node_id: Option<String>,
            is_active: bool,
        }

        let cluster_rows = sqlx::query_as::<_, ClusterRow>(&clusters_query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let clusters_json: Vec<Value> = cluster_rows
            .iter()
            .map(|c| serde_json::json!({
                "id": c.id,
                "name": c.name,
                "purpose": c.purpose,
                "node_ids": c.node_ids.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "leader_node_id": c.leader_node_id,
                "is_active": c.is_active,
            }))
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "node_count": nodes_json.len(),
            "edge_count": edges_json.len(),
            "cluster_count": clusters_json.len(),
            "nodes": nodes_json,
            "edges": edges_json,
            "clusters": clusters_json,
        })))
    }

    #[tool(
        description = "Get topology issues (bottlenecks, holes, orphans, etc.) for a project. Optionally filter by severity."
    )]
    pub(super) async fn get_topology_issues(
        &self,
        Parameters(req): Parameters<GetTopologyIssuesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let include_resolved = req.include_resolved.unwrap_or(false);
        let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

        let mut query = format!(
            "SELECT id, issue_type, severity, affected_nodes, affected_edges, description, \
             suggested_action, resolved_at, resolution_notes, created_at \
             FROM topology_issues WHERE project_id = '{}'",
            pid_hex
        );

        if !include_resolved {
            query.push_str(" AND resolved_at IS NULL");
        }
        if let Some(ref sev) = req.severity {
            query.push_str(&format!(" AND severity = '{}'", sev));
        }
        query.push_str(" ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'error' THEN 1 WHEN 'warning' THEN 2 ELSE 3 END, created_at DESC");

        #[derive(Debug, sqlx::FromRow)]
        struct IssueRow {
            id: String,
            issue_type: String,
            severity: String,
            affected_nodes: Option<String>,
            affected_edges: Option<String>,
            description: String,
            suggested_action: Option<String>,
            resolved_at: Option<String>,
            created_at: String,
        }

        let rows = sqlx::query_as::<_, IssueRow>(&query)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let issues: Vec<Value> = rows
            .iter()
            .map(|i| serde_json::json!({
                "id": i.id,
                "issue_type": i.issue_type,
                "severity": i.severity,
                "affected_nodes": i.affected_nodes.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "affected_edges": i.affected_edges.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                "description": i.description,
                "suggested_action": i.suggested_action,
                "resolved_at": i.resolved_at,
                "created_at": i.created_at,
            }))
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "project_id": req.project_id,
            "count": issues.len(),
            "issues": issues,
        })))
    }

    #[tool(
        description = "Find the shortest path between two topology nodes using BFS. Returns the path nodes, edges, and total weight."
    )]
    pub(super) async fn find_topology_path(
        &self,
        Parameters(req): Parameters<FindTopologyPathRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

        // Load active edges
        #[derive(Debug, sqlx::FromRow, Clone)]
        #[allow(dead_code)]
        struct EdgeRow {
            id: String,
            from_node_id: String,
            to_node_id: String,
            edge_type: String,
            weight: Option<f64>,
        }

        let edges = sqlx::query_as::<_, EdgeRow>(&format!(
            "SELECT id, from_node_id, to_node_id, edge_type, weight \
             FROM topology_edges WHERE project_id = '{}' AND status = 'active'",
            pid_hex
        ))
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Build adjacency list
        let mut adj: HashMap<String, Vec<(String, &EdgeRow)>> = HashMap::new();
        for e in &edges {
            adj.entry(e.from_node_id.clone())
                .or_default()
                .push((e.to_node_id.clone(), e));
            // Treat as undirected for path-finding
            adj.entry(e.to_node_id.clone())
                .or_default()
                .push((e.from_node_id.clone(), e));
        }

        // BFS
        let from = &req.from_node_id;
        let to = &req.to_node_id;

        let mut visited: HashMap<String, (String, String)> = HashMap::new(); // node -> (prev_node, edge_id)
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        queue.push_back(from.clone());
        visited.insert(from.clone(), ("".to_string(), "".to_string()));

        let mut found = false;
        while let Some(current) = queue.pop_front() {
            if current == *to {
                found = true;
                break;
            }
            if let Some(neighbors) = adj.get(&current) {
                for (next, edge) in neighbors {
                    if !visited.contains_key(next) {
                        visited.insert(next.clone(), (current.clone(), edge.id.clone()));
                        queue.push_back(next.clone());
                    }
                }
            }
        }

        if !found {
            return Ok(success_json(&serde_json::json!({
                "success": true,
                "path_found": false,
                "message": "No path found between the specified nodes",
                "from_node_id": from,
                "to_node_id": to,
            })));
        }

        // Reconstruct path
        let mut path_nodes: Vec<String> = Vec::new();
        let mut path_edges: Vec<String> = Vec::new();
        let mut total_weight: f64 = 0.0;
        let mut current = to.clone();

        while current != *from {
            path_nodes.push(current.clone());
            let (prev, edge_id) = &visited[&current];
            path_edges.push(edge_id.clone());
            if let Some(edge) = edges.iter().find(|e| e.id == *edge_id) {
                total_weight += edge.weight.unwrap_or(1.0);
            }
            current = prev.clone();
        }
        path_nodes.push(from.clone());
        path_nodes.reverse();
        path_edges.reverse();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "path_found": true,
            "from_node_id": from,
            "to_node_id": to,
            "path_length": path_nodes.len(),
            "total_weight": total_weight,
            "path_nodes": path_nodes,
            "path_edges": path_edges,
        })))
    }

    #[tool(
        description = "Get VIBE budget information for a project — limit, spent, remaining, and whether budget is available."
    )]
    pub(super) async fn get_vibe_budget(
        &self,
        Parameters(req): Parameters<GetVibeBudgetRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Project::find_by_id(&self.pool, &project_uuid.to_string()).await {
            Ok(Some(project)) => {
                let remaining = project.remaining_vibe();
                let has_budget = project.has_vibe_budget(0);

                Ok(success_json(&serde_json::json!({
                    "success": true,
                    "project_id": req.project_id,
                    "project_name": project.name,
                    "vibe_budget_limit": project.vibe_budget_limit,
                    "vibe_spent_amount": project.vibe_spent_amount,
                    "vibe_remaining": remaining,
                    "has_budget": has_budget,
                    "is_unlimited": project.vibe_budget_limit.is_none(),
                    "currency_note": "1 VIBE = $0.01 USD",
                })))
            }
            Ok(None) => Ok(error_result("Project not found", None)),
            Err(e) => Ok(error_result(
                "Failed to retrieve project",
                Some(&e.to_string()),
            )),
        }
    }

    #[tool(description = "List registered AI agents. Optionally filter by status.")]
    pub(super) async fn list_agents(
        &self,
        Parameters(req): Parameters<ListAgentsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let agents = if let Some(ref status_str) = req.status {
            let status = match status_str.to_lowercase().as_str() {
                "active" => AgentStatus::Active,
                "inactive" => AgentStatus::Inactive,
                "maintenance" => AgentStatus::Maintenance,
                "training" => AgentStatus::Training,
                _ => {
                    return Ok(error_result(
                        "Invalid status. Valid: active, inactive, maintenance, training",
                        None,
                    ));
                }
            };
            match if status == AgentStatus::Active {
                Agent::find_active(&self.pool).await
            } else {
                Agent::find_all(&self.pool)
                    .await
                    .map(|agents| agents.into_iter().filter(|a| a.status == status).collect())
            } {
                Ok(a) => a,
                Err(e) => return Ok(error_result("Failed to list agents", Some(&e.to_string()))),
            }
        } else {
            match Agent::find_all(&self.pool).await {
                Ok(a) => a,
                Err(e) => return Ok(error_result("Failed to list agents", Some(&e.to_string()))),
            }
        };

        let agent_list: Vec<Value> = agents
            .iter()
            .map(|a| {
                let capabilities: Option<Vec<String>> = a
                    .capabilities
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok());
                let tools: Option<Vec<String>> = a
                    .tools
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok());

                serde_json::json!({
                    "id": a.id.to_string(),
                    "short_name": a.short_name,
                    "designation": a.designation,
                    "description": a.description,
                    "status": format!("{:?}", a.status).to_lowercase(),
                    "autonomy_level": format!("{:?}", a.autonomy_level).to_lowercase(),
                    "default_model": a.default_model,
                    "capabilities": capabilities,
                    "tools": tools,
                    "tasks_completed": a.tasks_completed,
                    "tasks_failed": a.tasks_failed,
                    "average_rating": a.average_rating,
                    "avatar_url": a.avatar_url,
                    "agent_tier": a.agent_tier,
                })
            })
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "count": agent_list.len(),
            "agents": agent_list,
        })))
    }
}

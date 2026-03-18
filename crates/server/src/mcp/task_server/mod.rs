mod helpers;
mod types;
mod task_ops;
mod project_ops;
mod knowledge;
mod topology;
mod deps;
mod policy;

use std::future::Future;

use db::models::{
    project::Project,
    project_knowledge_source::ProjectKnowledgeSource,
    task::Task,
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::ToolRouter,
    model::{
        Annotated, Implementation, ProtocolVersion,
        ReadResourceRequestParam, ReadResourceResult, ResourceContents,
        ResourceTemplate, RawResourceTemplate, ServerCapabilities, ServerInfo,
        ListResourceTemplatesResult,
    },
    tool_handler, tool_router,
};
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use helpers::*;

// Re-export all public types for external consumers
pub use types::*;

// ─── Server ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TaskServer {
    pub(super) pool: SqlitePool,
    /// User ID for scoping operations (None = admin/unscoped for backward compat)
    pub(super) user_id: Option<Uuid>,
    /// Whether this user is admin (bypasses project membership checks)
    pub(super) is_admin: bool,
    tool_router: ToolRouter<TaskServer>,
}

impl TaskServer {
    #[allow(dead_code)]
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            user_id: None,
            is_admin: true,
            tool_router: Self::tool_router(),
        }
    }

    /// Create a user-scoped MCP task server
    pub fn new_for_user(pool: SqlitePool, user_id: Uuid, is_admin: bool) -> Self {
        Self {
            pool,
            user_id: Some(user_id),
            is_admin,
            tool_router: Self::tool_router(),
        }
    }

    /// Resolve project IDs accessible to the current user
    pub(super) async fn accessible_project_ids(&self) -> Result<Option<Vec<Uuid>>, sqlx::Error> {
        if self.is_admin || self.user_id.is_none() {
            return Ok(None); // None = all projects accessible
        }
        // Safety: checked self.user_id.is_none() above, so this is guaranteed Some
        let user_id = self.user_id.unwrap_or_default();
        let user_id_bytes = user_id.to_string();

        #[derive(sqlx::FromRow)]
        struct ProjId {
            project_id: String,
        }

        let rows: Vec<ProjId> = sqlx::query_as(
            "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?",
        )
        .bind(&user_id_bytes)
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<Uuid> = rows
            .into_iter()
            .filter_map(|r| Uuid::parse_str(&r.project_id).ok())
            .collect();
        Ok(Some(ids))
    }
}

#[tool_router]
impl TaskServer {
    // All tool methods are defined in submodules via separate `impl TaskServer` blocks.
    // The #[tool_router] macro collects them from all impl blocks in the crate.

    // Phase 1 — task_ops.rs: create_task, list_tasks, update_task, delete_task, get_task, assign_task
    // Phase 2 — task_ops.rs: bulk_create_tasks, bulk_update_tasks, search_tasks
    // Phase 2 — project_ops.rs: list_projects, list_project_members, scaffold_project
    // Phase 2 — deps.rs: manage_task_dependencies, check_dependencies
    // Phase 3 — knowledge.rs: add_knowledge, list_knowledge, get_knowledge_completeness, manage_knowledge
    // Phase 4 — topology.rs: get_topology, get_topology_issues, find_topology_path, get_vibe_budget, list_agents
    // Policy — policy.rs: evaluate_policy, add_comment, unified_search
}

// ─── MCP Resources ──────────────────────────────────────────────────────────

/// Build the 6 MCP resource templates
fn orcha_resource_templates() -> Vec<ResourceTemplate> {
    let templates = vec![
        ("orcha://projects/{project_id}", "Project Detail", "Get project details including VIBE budget and configuration"),
        ("orcha://projects/{project_id}/tasks", "Project Tasks", "List all tasks in a project"),
        ("orcha://projects/{project_id}/tasks/{task_id}", "Task Detail", "Get detailed information about a specific task"),
        ("orcha://projects/{project_id}/knowledge", "Project Knowledge", "List knowledge sources for a project"),
        ("orcha://projects/{project_id}/topology", "Project Topology", "Get the topology graph (nodes, edges, clusters)"),
        ("orcha://projects/{project_id}/health", "Project Health", "Get health summary including issues and knowledge completeness"),
    ];

    templates
        .into_iter()
        .map(|(uri, name, desc)| {
            Annotated::new(RawResourceTemplate {
                uri_template: uri.to_string(),
                name: name.to_string(),
                description: Some(desc.to_string()),
                mime_type: Some("application/json".to_string()),
            }, None)
        })
        .collect()
}

#[tool_handler]
impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "pcg-dashboard-mcp".to_string(),
                version: "2.0.0".to_string(),
            },
            instructions: Some(
                "PCG Dashboard MCP v2 — Atlas-level project management for AI agents. \
                 26 tools + 6 MCP resources. Use `list_projects` to discover project IDs. \
                 Tools: list_projects, list_tasks, create_task, get_task, update_task, delete_task, \
                 assign_task, add_comment, list_project_members, \
                 evaluate_policy, bulk_create_tasks, bulk_update_tasks, search_tasks, \
                 manage_task_dependencies, check_dependencies, unified_search, scaffold_project, \
                 add_knowledge, list_knowledge, get_knowledge_completeness, \
                 manage_knowledge, get_topology, get_topology_issues, find_topology_path, \
                 get_vibe_budget, list_agents. \
                 Resources: orcha://projects/{project_id}[/tasks|/knowledge|/topology|/health]"
                    .to_string(),
            ),
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, ErrorData>> + Send + '_
    {
        std::future::ready(Ok(ListResourceTemplatesResult {
            resource_templates: orcha_resource_templates(),
            next_cursor: None,
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_
    {
        let pool = self.pool.clone();
        let uri = request.uri.clone();

        async move {
            let content = match resolve_resource(&pool, &uri).await {
                Ok(json) => json,
                Err(msg) => serde_json::json!({ "error": msg }),
            };

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string_pretty(&content).unwrap_or_default(),
                    uri,
                )],
            })
        }
    }
}

/// Resolve an orcha:// resource URI to JSON
async fn resolve_resource(pool: &SqlitePool, uri: &str) -> Result<Value, String> {
    // Parse: orcha://projects/{project_id}[/tasks[/{task_id}]|/knowledge|/topology|/health]
    let path = uri
        .strip_prefix("orcha://projects/")
        .ok_or_else(|| format!("Unknown resource URI: {}", uri))?;

    let parts: Vec<&str> = path.splitn(3, '/').collect();
    let project_uuid = Uuid::parse_str(parts[0])
        .map_err(|_| "Invalid project_id in URI".to_string())?;

    let project = Project::find_by_id(pool, &project_uuid.to_string())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    if parts.len() == 1 {
        // Project detail
        return Ok(serde_json::json!({
            "id": project.id.to_string(),
            "name": project.name,
            "git_repo_path": project.git_repo_path.to_string_lossy(),
            "vibe_budget_limit": project.vibe_budget_limit,
            "vibe_spent_amount": project.vibe_spent_amount,
            "vibe_remaining": project.remaining_vibe(),
            "setup_script": project.setup_script,
            "dev_script": project.dev_script,
            "cleanup_script": project.cleanup_script,
            "created_at": project.created_at.to_rfc3339(),
            "updated_at": project.updated_at.to_rfc3339(),
        }));
    }

    match parts[1] {
        "tasks" => {
            if parts.len() == 3 {
                // Single task detail
                let task_uuid = Uuid::parse_str(parts[2])
                    .map_err(|_| "Invalid task_id in URI".to_string())?;
                let task = Task::find_by_id_and_project_id(pool, &task_uuid.to_string(), &project_uuid.to_string())
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "Task not found".to_string())?;
                Ok(to_json_value(&task_to_summary(&task)))
            } else {
                // Task list
                let tasks = Task::find_by_project_id_with_attempt_status(pool, &project_uuid.to_string())
                    .await
                    .map_err(|e| e.to_string())?;
                let summaries: Vec<Value> = tasks
                    .iter()
                    .map(|t| to_json_value(&task_with_status_to_summary(t)))
                    .collect();
                Ok(serde_json::json!({
                    "project_id": project.id.to_string(),
                    "project_name": project.name,
                    "count": summaries.len(),
                    "tasks": summaries,
                }))
            }
        }
        "knowledge" => {
            let sources = ProjectKnowledgeSource::find_by_project(pool, project_uuid)
                .await
                .map_err(|e| e.to_string())?;
            let items: Vec<Value> = sources
                .iter()
                .map(|s| serde_json::json!({
                    "id": s.id.to_string(),
                    "source_type": s.source_type,
                    "source_id": s.source_id,
                    "source_title": s.source_title,
                    "source_summary": s.source_summary,
                    "coverage_score": s.coverage_score,
                    "is_stale": s.is_stale,
                }))
                .collect();
            Ok(serde_json::json!({
                "project_id": project.id.to_string(),
                "count": items.len(),
                "sources": items,
            }))
        }
        "topology" => {
            let pid_hex = hex::encode(project_uuid.as_bytes()).to_uppercase();

            #[derive(sqlx::FromRow)]
            struct CountRow { cnt: i64 }

            let node_count: i64 = sqlx::query_as::<_, CountRow>(
                &format!("SELECT COUNT(*) as cnt FROM topology_nodes WHERE project_id = '{}' AND status = 'active'", pid_hex)
            )
            .fetch_one(pool)
            .await
            .map(|r| r.cnt)
            .unwrap_or(0);

            let edge_count: i64 = sqlx::query_as::<_, CountRow>(
                &format!("SELECT COUNT(*) as cnt FROM topology_edges WHERE project_id = '{}' AND status = 'active'", pid_hex)
            )
            .fetch_one(pool)
            .await
            .map(|r| r.cnt)
            .unwrap_or(0);

            let cluster_count: i64 = sqlx::query_as::<_, CountRow>(
                &format!("SELECT COUNT(*) as cnt FROM topology_clusters WHERE project_id = '{}' AND is_active = 1", pid_hex)
            )
            .fetch_one(pool)
            .await
            .map(|r| r.cnt)
            .unwrap_or(0);

            Ok(serde_json::json!({
                "project_id": project.id.to_string(),
                "active_nodes": node_count,
                "active_edges": edge_count,
                "active_clusters": cluster_count,
                "note": "Use get_topology tool for full graph data",
            }))
        }
        "health" => {
            let health_map = ProjectKnowledgeSource::get_health_batch(pool, &[project_uuid.to_string()])
                .await
                .unwrap_or_default();

            let health = health_map
                .get(&project.id.to_string())
                .cloned();

            match health {
                Some(h) => Ok(to_json_value(&h)),
                None => Ok(serde_json::json!({
                    "project_id": project.id.to_string(),
                    "health_status": "unknown",
                    "active_issues_count": 0,
                    "critical_issues": 0,
                    "warning_issues": 0,
                    "knowledge_completeness": 0.0,
                })),
            }
        }
        _ => Err(format!("Unknown resource path segment: {}", parts[1])),
    }
}

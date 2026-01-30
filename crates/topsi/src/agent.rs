//! TopsiAgent - Platform Agent with containerized access control
//!
//! Topsi is the platform orchestrator that manages all projects and users
//! with strict data isolation between clients.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::config::TopsiConfig;
use crate::topology::graph::TopologyGraph;
use crate::tools::get_tool_schemas;
use crate::{DetectedIssue, Result, TopsiError, TopsiResponse, TopologySummary, ToolCallResult};

// Database models for querying real data
use db::models::project::Project;
use db::models::agent::Agent;
use db::models::task::Task;

// Import Nora's LLM infrastructure
use nora::brain::{
    infer_provider_from_model, LLMClient, LLMConfig as NoraLLMConfig, LLMProvider, LLMResponse,
};

/// Topsi's system prompt - defines its role as platform orchestrator
const TOPSI_SYSTEM_PROMPT: &str = r#"You are Topsi, the Topological Super Intelligence - the central platform orchestrator for the PowerClub Global ecosystem.

## Your Role
You are the master controller who reasons about the entire project ecosystem as a living topology of agents, tasks, resources, and connections. You don't just manage tasks - you maintain and sculpt the topology of projects.

## Core Capabilities
- **Global Awareness**: You maintain a live topology graph of all nodes (agents, tasks, projects, resources), edges (dependencies, assignments), and clusters (teams, workflows)
- **Pattern Detection**: You identify bottlenecks, holes, isolated nodes, and emerging patterns in the topology
- **Dynamic Re-Routing**: When routes degrade, you find alternate paths to keep work flowing
- **Collective Intelligence**: You form and dissolve agent teams dynamically based on task requirements
- **Access Control**: You enforce strict client data isolation - users only see projects they have access to
- **System Health**: You monitor and report on the overall health of the platform

## Access Scope
You have visibility based on the user's access level:
- **Admin users**: Full visibility across all projects and the entire platform topology
- **Regular users**: Containerized view limited to their assigned projects

## Available Tools
When users ask about topology, projects, or system state, use the appropriate tools:
- `list_nodes` - List nodes filtered by type or status
- `list_edges` - List connections between nodes
- `find_path` - Find optimal paths between nodes
- `detect_issues` - Identify topology problems (bottlenecks, cycles, isolated nodes)
- `get_topology_summary` - Get overall topology statistics
- `create_cluster` - Form a new team or cluster of nodes
- `verify_access` - Check if a user can access a resource

## Response Style
- Be concise but informative
- Use topology-aware language (nodes, edges, clusters, routes, paths)
- When reporting issues, suggest actionable next steps
- Format complex data as tables or structured lists
- For system health, provide a brief summary with key metrics

## Example Interactions
User: "What projects do I have access to?"
→ Use your access scope to list accessible projects

User: "Show me the topology overview"
→ Use `get_topology_summary` to provide system statistics

User: "Are there any bottlenecks in the system?"
→ Use `detect_issues` with issue_types=['bottleneck']

User: "Find a path from task X to agent Y"
→ Use `find_path` to compute and explain the optimal route"#;

pub mod access_control;
pub use access_control::{AccessControl, AccessScope, UserContext, ProjectAccess};

/// TopsiAgent - The Platform Intelligence Agent
///
/// Topsi serves as the central platform agent with:
/// - Master credential for admin users (full ecosystem visibility)
/// - Containerized access for regular users (project-scoped visibility)
/// - Strict client data isolation
/// - System-wide optimization capabilities
/// - LLM-powered intelligent conversations
pub struct TopsiAgent {
    /// Unique identifier for this Topsi instance
    pub id: Uuid,
    /// Configuration
    pub config: TopsiConfig,
    /// Access control manager
    pub access_control: Arc<AccessControl>,
    /// Database connection
    pub db: Option<SqlitePool>,
    /// Project topologies (indexed by project_id)
    topologies: Arc<RwLock<indexmap::IndexMap<Uuid, TopologyGraph>>>,
    /// Initialization timestamp
    pub initialized_at: DateTime<Utc>,
    /// Active status
    active: Arc<RwLock<bool>>,
    /// LLM client for intelligent conversations
    llm: Option<LLMClient>,
}

impl TopsiAgent {
    /// Create a new TopsiAgent with configuration
    pub async fn new(config: TopsiConfig) -> Result<Self> {
        tracing::info!("Initializing TopsiAgent: {}", config.name);

        let access_control = Arc::new(AccessControl::new());

        // Initialize LLM client if provider is configured
        let llm = if !config.llm.provider.is_empty() {
            let provider = infer_provider_from_model(&config.llm.model);
            let system_prompt = config
                .system_prompt
                .clone()
                .unwrap_or_else(|| TOPSI_SYSTEM_PROMPT.to_string());

            // Determine endpoint based on provider
            let endpoint = match provider {
                LLMProvider::Ollama => std::env::var("OLLAMA_ENDPOINT").ok().or_else(|| {
                    if std::net::TcpStream::connect("127.0.0.1:11434").is_ok() {
                        Some("http://127.0.0.1:11434/v1/chat/completions".to_string())
                    } else {
                        None
                    }
                }),
                _ => None,
            };

            let nora_config = NoraLLMConfig {
                provider,
                model: config.llm.model.clone(),
                temperature: config.llm.temperature,
                max_tokens: config.llm.max_tokens,
                system_prompt,
                endpoint,
            };

            tracing::info!(
                "Topsi LLM configured: provider={:?}, model={}",
                nora_config.provider,
                nora_config.model
            );

            Some(LLMClient::new(nora_config))
        } else {
            tracing::warn!("Topsi LLM not configured - chat will return stub responses");
            None
        };

        Ok(Self {
            id: Uuid::new_v4(),
            config,
            access_control,
            db: None,
            topologies: Arc::new(RwLock::new(indexmap::IndexMap::new())),
            initialized_at: Utc::now(),
            active: Arc::new(RwLock::new(false)),
            llm,
        })
    }

    /// Attach database connection
    pub fn with_database(mut self, pool: SqlitePool) -> Self {
        self.db = Some(pool);
        self
    }

    /// Set active state
    pub async fn set_active(&self, active: bool) -> Result<()> {
        let mut state = self.active.write().await;
        *state = active;
        tracing::info!("Topsi active state set to: {}", active);
        Ok(())
    }

    /// Check if Topsi is active
    pub async fn is_active(&self) -> bool {
        *self.active.read().await
    }

    /// Get uptime in milliseconds
    pub fn uptime_ms(&self) -> i64 {
        (Utc::now() - self.initialized_at).num_milliseconds()
    }

    /// Get the current access scope for a user
    pub async fn get_user_scope(&self, user_context: &UserContext) -> AccessScope {
        self.access_control.get_scope(user_context).await
    }

    /// Process a request with access control
    pub async fn process_request(
        &self,
        request: TopsiRequest,
        user_context: &UserContext,
    ) -> Result<TopsiResponse> {
        // Verify user has appropriate access
        let scope = self.access_control.get_scope(user_context).await;

        // Log access for audit
        self.access_control.log_access(
            user_context,
            &format!("{:?}", request.request_type),
            true,
        ).await;

        match request.request_type {
            TopsiRequestType::Chat { message } => {
                self.handle_chat(&message, user_context, &scope).await
            }
            TopsiRequestType::GetTopology { project_id } => {
                self.handle_get_topology(project_id, user_context, &scope).await
            }
            TopsiRequestType::DetectIssues { project_id } => {
                self.handle_detect_issues(project_id, user_context, &scope).await
            }
            TopsiRequestType::ListProjects => {
                self.handle_list_projects(user_context, &scope).await
            }
            TopsiRequestType::ExecuteCommand { command } => {
                self.handle_command(&command, user_context, &scope).await
            }
        }
    }

    /// Handle chat messages with LLM integration
    async fn handle_chat(
        &self,
        message: &str,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // If LLM is not configured, return a helpful message
        let Some(llm) = &self.llm else {
            let scope_description = match scope {
                AccessScope::Admin => "full platform access".to_string(),
                AccessScope::Projects(ids) => format!("access to {} projects", ids.len()),
                AccessScope::SingleProject(id) => format!("access to project {}", id),
                AccessScope::None => "no access".to_string(),
            };
            return Ok(TopsiResponse {
                message: format!(
                    "Topsi is running without LLM. You have {}. Your message: {}",
                    scope_description, message
                ),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            });
        };

        // Build context string based on access scope
        let context = self.build_context_for_scope(scope, user_context).await;

        // Get tool schemas in OpenAI format
        let tools = get_tool_schemas();

        tracing::debug!(
            "[TOPSI] Sending chat to LLM: {} chars, {} tools",
            message.len(),
            tools.len()
        );

        // Call LLM with tools
        let response = llm
            .generate_with_tools(TOPSI_SYSTEM_PROMPT, message, &context, &tools)
            .await
            .map_err(|e| TopsiError::LLMError(format!("LLM request failed: {}", e)))?;

        // Handle LLM response
        match response {
            LLMResponse::Text { content, usage } => {
                tracing::info!("[TOPSI] LLM returned text response ({} chars)", content.len());
                Ok(TopsiResponse {
                    message: content,
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: usage.as_ref().map(|u| u.input_tokens as i64),
                    output_tokens: usage.as_ref().map(|u| u.output_tokens as i64),
                })
            }
            LLMResponse::ToolCalls { calls, usage } => {
                tracing::info!("[TOPSI] LLM requested {} tool calls", calls.len());

                // Execute tool calls
                let tool_results = self
                    .execute_tool_calls(&calls, user_context, scope)
                    .await;

                // Build tool call results for response
                let tool_call_results: Vec<ToolCallResult> = calls
                    .iter()
                    .zip(tool_results.iter())
                    .map(|(call, result)| ToolCallResult {
                        tool_name: call.name.clone(),
                        arguments: call.arguments.clone(),
                        result: result.clone(),
                        success: !result
                            .get("error")
                            .map(|e| !e.is_null())
                            .unwrap_or(false),
                    })
                    .collect();

                // Check if there's a respond_to_user tool call - use its response as the message
                let message = tool_call_results
                    .iter()
                    .find(|r| r.tool_name == "respond_to_user" && r.success)
                    .and_then(|r| r.result.get("response"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        // Fall back to tool execution summary if no respond_to_user
                        let results_summary = tool_call_results
                            .iter()
                            .map(|r| {
                                if r.success {
                                    format!("✓ {}: {}", r.tool_name, r.result)
                                } else {
                                    format!("✗ {}: {}", r.tool_name, r.result)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("Executed {} tools:\n{}", tool_call_results.len(), results_summary)
                    });

                Ok(TopsiResponse {
                    message,
                    tool_calls: tool_call_results,
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: usage.as_ref().map(|u| u.input_tokens as i64),
                    output_tokens: usage.as_ref().map(|u| u.output_tokens as i64),
                })
            }
        }
    }

    /// Build context string for LLM based on user's access scope
    async fn build_context_for_scope(&self, scope: &AccessScope, user_context: &UserContext) -> String {
        let mut context_parts = vec![];

        // Add user context
        context_parts.push(format!(
            "User: {} ({})",
            user_context.email.as_deref().unwrap_or("unknown"),
            if user_context.is_admin { "admin" } else { "user" }
        ));

        // Add scope information and real data
        if let Some(pool) = &self.db {
            match scope {
                AccessScope::Admin => {
                    context_parts.push("Access Level: Full platform access (admin)".to_string());

                    // Get real project data
                    if let Ok(projects) = Project::find_all(pool).await {
                        context_parts.push(format!("\n## Projects in System ({})", projects.len()));
                        for project in projects.iter().take(10) {
                            context_parts.push(format!(
                                "- {} (ID: {}): {}",
                                project.name,
                                project.id,
                                project.git_repo_path.display()
                            ));
                        }
                        if projects.len() > 10 {
                            context_parts.push(format!("... and {} more projects", projects.len() - 10));
                        }
                    }

                    // Get agent data
                    if let Ok(agents) = Agent::find_all(pool).await {
                        context_parts.push(format!("\n## Registered Agents ({})", agents.len()));
                        for agent in agents.iter() {
                            context_parts.push(format!(
                                "- {} ({}): {} - Status: {:?}",
                                agent.short_name,
                                agent.designation,
                                agent.description.as_deref().unwrap_or("No description"),
                                agent.status
                            ));
                        }
                    }

                    // Get task counts
                    if let Ok(count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks")
                        .fetch_one(pool)
                        .await
                    {
                        context_parts.push(format!("\n## Task Statistics"));
                        context_parts.push(format!("Total tasks: {}", count));

                        // Get task counts by status
                        if let Ok(todo_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'"
                        ).fetch_one(pool).await {
                            context_parts.push(format!("Todo: {}", todo_count));
                        }
                        if let Ok(in_progress_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'"
                        ).fetch_one(pool).await {
                            context_parts.push(format!("In Progress: {}", in_progress_count));
                        }
                        if let Ok(done_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'done'"
                        ).fetch_one(pool).await {
                            context_parts.push(format!("Done: {}", done_count));
                        }
                    }
                }
                AccessScope::Projects(ids) => {
                    context_parts.push(format!(
                        "Access Level: Project access ({} projects)",
                        ids.len()
                    ));

                    // Get details for accessible projects
                    context_parts.push("\n## Accessible Projects".to_string());
                    for project_id in ids.iter().take(10) {
                        if let Ok(Some(project)) = Project::find_by_id(pool, *project_id).await {
                            context_parts.push(format!(
                                "- {} (ID: {}): {}",
                                project.name,
                                project.id,
                                project.git_repo_path.display()
                            ));
                        }
                    }
                }
                AccessScope::SingleProject(id) => {
                    context_parts.push(format!("Access Level: Single project access"));
                    if let Ok(Some(project)) = Project::find_by_id(pool, *id).await {
                        context_parts.push(format!(
                            "\n## Current Project: {} (ID: {})",
                            project.name, project.id
                        ));
                        context_parts.push(format!("Path: {}", project.git_repo_path.display()));
                    }
                }
                AccessScope::None => {
                    context_parts.push("Access Level: No project access".to_string());
                }
            }
        } else {
            context_parts.push("Warning: Database not connected - limited data available".to_string());
        }

        // Add system stats
        context_parts.push(format!("\nTopsi uptime: {}ms", self.uptime_ms()));

        context_parts.join("\n")
    }

    /// Execute tool calls requested by the LLM
    async fn execute_tool_calls(
        &self,
        calls: &[nora::brain::ToolCall],
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Vec<serde_json::Value> {
        let mut results = Vec::new();

        for call in calls {
            tracing::debug!(
                "[TOPSI] Executing tool: {} with args: {}",
                call.name,
                call.arguments
            );

            let result = match call.name.as_str() {
                "list_projects" => self.tool_list_projects(&call.arguments, scope).await,
                "list_nodes" => self.tool_list_nodes(&call.arguments, scope).await,
                "list_edges" => self.tool_list_edges(&call.arguments, scope).await,
                "find_path" => self.tool_find_path(&call.arguments, scope).await,
                "detect_issues" => self.tool_detect_issues(&call.arguments, scope).await,
                "get_topology_summary" => self.tool_get_topology_summary(&call.arguments, scope).await,
                "create_cluster" => self.tool_create_cluster(&call.arguments, user_context, scope).await,
                "verify_access" => self.tool_verify_access(&call.arguments, scope).await,
                "respond_to_user" => self.tool_respond_to_user(&call.arguments).await,
                _ => Err(TopsiError::ToolError(format!(
                    "Unknown tool: {}",
                    call.name
                ))),
            };

            results.push(match result {
                Ok(value) => value,
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            });
        }

        results
    }

    // ==================== Tool Implementations ====================

    /// List all accessible projects
    async fn tool_list_projects(
        &self,
        _args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "projects": []
            }));
        };

        let projects = match scope {
            AccessScope::Admin => {
                // Admin sees all projects
                Project::find_all(pool).await.map_err(|e| TopsiError::DatabaseError(e))?
            }
            AccessScope::Projects(ids) => {
                // User sees only their projects
                let mut projects = Vec::new();
                for id in ids {
                    if let Ok(Some(p)) = Project::find_by_id(pool, *id).await {
                        projects.push(p);
                    }
                }
                projects
            }
            AccessScope::SingleProject(id) => {
                if let Ok(Some(p)) = Project::find_by_id(pool, *id).await {
                    vec![p]
                } else {
                    vec![]
                }
            }
            AccessScope::None => vec![],
        };

        let project_list: Vec<serde_json::Value> = projects
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.to_string(),
                    "name": p.name,
                    "path": p.git_repo_path.display().to_string(),
                    "vibe_spent": p.vibe_spent_amount,
                    "vibe_budget": p.vibe_budget_limit,
                    "created_at": p.created_at.to_rfc3339()
                })
            })
            .collect();

        Ok(serde_json::json!({
            "projects": project_list,
            "total": projects.len()
        }))
    }

    /// List nodes (projects, agents, tasks) based on type filter
    async fn tool_list_nodes(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let node_type = args.get("node_type").and_then(|v| v.as_str());
        let _status = args.get("status").and_then(|v| v.as_str());

        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "nodes": []
            }));
        };

        let mut nodes = Vec::new();

        // Get nodes based on type filter
        match node_type {
            Some("project") | None => {
                // Get projects based on scope
                let projects = match scope {
                    AccessScope::Admin => Project::find_all(pool).await.unwrap_or_default(),
                    AccessScope::Projects(ids) => {
                        let mut ps = Vec::new();
                        for id in ids {
                            if let Ok(Some(p)) = Project::find_by_id(pool, *id).await {
                                ps.push(p);
                            }
                        }
                        ps
                    }
                    _ => vec![],
                };

                for p in projects {
                    nodes.push(serde_json::json!({
                        "type": "project",
                        "id": p.id.to_string(),
                        "name": p.name,
                        "status": "active"
                    }));
                }
            }
            _ => {}
        }

        match node_type {
            Some("agent") | None => {
                // Agents are visible to all (platform-wide)
                if let Ok(agents) = Agent::find_all(pool).await {
                    for a in agents {
                        nodes.push(serde_json::json!({
                            "type": "agent",
                            "id": a.id.to_string(),
                            "name": a.short_name,
                            "designation": a.designation,
                            "status": format!("{:?}", a.status).to_lowercase(),
                            "capabilities": a.capabilities
                        }));
                    }
                }
            }
            _ => {}
        }

        match node_type {
            Some("task") | None => {
                // Get tasks based on accessible projects
                let project_ids: Vec<Uuid> = match scope {
                    AccessScope::Admin => {
                        Project::find_all(pool).await.unwrap_or_default().iter().map(|p| p.id).collect()
                    }
                    AccessScope::Projects(ids) => ids.iter().copied().collect(),
                    AccessScope::SingleProject(id) => vec![*id],
                    AccessScope::None => vec![],
                };

                for project_id in project_ids.iter().take(5) {
                    // Query tasks for this project
                    let tasks: Vec<Task> = sqlx::query_as(
                        "SELECT * FROM tasks WHERE project_id = ? ORDER BY created_at DESC LIMIT 20"
                    )
                    .bind(project_id)
                    .fetch_all(pool)
                    .await
                    .unwrap_or_default();

                    for t in tasks {
                        nodes.push(serde_json::json!({
                            "type": "task",
                            "id": t.id.to_string(),
                            "title": t.title,
                            "project_id": t.project_id.to_string(),
                            "status": format!("{:?}", t.status).to_lowercase(),
                            "priority": format!("{:?}", t.priority).to_lowercase()
                        }));
                    }
                }
            }
            _ => {}
        }

        Ok(serde_json::json!({
            "nodes": nodes,
            "total": nodes.len(),
            "filter": {
                "node_type": node_type
            }
        }))
    }

    async fn tool_list_edges(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let edge_type = args.get("edge_type").and_then(|v| v.as_str());

        // For now return relationship types available
        Ok(serde_json::json!({
            "edges": [],
            "edge_types_available": ["depends_on", "assigned_to", "contains", "communicates_with"],
            "total": 0,
            "filter": {
                "edge_type": edge_type
            },
            "scope": format!("{:?}", scope),
            "note": "Edge traversal requires topology graph to be built"
        }))
    }

    async fn tool_find_path(
        &self,
        args: &serde_json::Value,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let from_node = args.get("from_node_id").and_then(|v| v.as_str());
        let to_node = args.get("to_node_id").and_then(|v| v.as_str());

        Ok(serde_json::json!({
            "path": [],
            "from": from_node,
            "to": to_node,
            "found": false,
            "message": "Path finding requires topology graph - use list_nodes to discover available nodes first"
        }))
    }

    async fn tool_detect_issues(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let issue_types = args
            .get("issue_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
            });

        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "issues": []
            }));
        };

        let mut issues = Vec::new();

        // Check for tasks stuck in progress
        if issue_types.as_ref().map_or(true, |t| t.contains(&"stale")) {
            let stale_tasks: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress' AND updated_at < datetime('now', '-7 days')"
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            if stale_tasks > 0 {
                issues.push(serde_json::json!({
                    "type": "stale",
                    "severity": "warning",
                    "description": format!("{} tasks have been in progress for over 7 days", stale_tasks),
                    "affected_count": stale_tasks
                }));
            }
        }

        // Check for unassigned high-priority tasks
        if issue_types.as_ref().map_or(true, |t| t.contains(&"bottleneck")) {
            let unassigned_high: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE (priority = 'critical' OR priority = 'high') AND assigned_agent IS NULL AND status = 'todo'"
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            if unassigned_high > 0 {
                issues.push(serde_json::json!({
                    "type": "bottleneck",
                    "severity": "high",
                    "description": format!("{} high/critical priority tasks are unassigned", unassigned_high),
                    "affected_count": unassigned_high,
                    "suggestion": "Consider assigning these tasks to available agents"
                }));
            }
        }

        Ok(serde_json::json!({
            "issues": issues,
            "total": issues.len(),
            "filter": {
                "issue_types": issue_types
            },
            "scope": format!("{:?}", scope)
        }))
    }

    async fn tool_get_topology_summary(
        &self,
        _args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected"
            }));
        };

        // Get real counts from database
        let project_count: i64 = match scope {
            AccessScope::Admin => {
                sqlx::query_scalar("SELECT COUNT(*) FROM projects")
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0)
            }
            AccessScope::Projects(ids) => ids.len() as i64,
            AccessScope::SingleProject(_) => 1,
            AccessScope::None => 0,
        };

        let agent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let active_task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let todo_task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let done_task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE status = 'done'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        // Calculate health score (simple heuristic)
        let health_score = if task_count > 0 {
            let completion_rate = done_task_count as f64 / task_count as f64;
            let active_rate = active_task_count as f64 / task_count.max(1) as f64;
            (completion_rate * 0.5 + (1.0 - active_rate.min(0.5)) * 0.5).min(1.0)
        } else {
            1.0
        };

        Ok(serde_json::json!({
            "projects": project_count,
            "agents": agent_count,
            "total_tasks": task_count,
            "tasks_by_status": {
                "todo": todo_task_count,
                "in_progress": active_task_count,
                "done": done_task_count
            },
            "health_score": format!("{:.2}", health_score),
            "scope": format!("{:?}", scope)
        }))
    }

    async fn tool_create_cluster(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
        let node_ids = args.get("node_ids").and_then(|v| v.as_array());

        Ok(serde_json::json!({
            "created": false,
            "cluster_name": name,
            "node_count": node_ids.map(|arr| arr.len()).unwrap_or(0),
            "message": "Cluster creation not yet implemented - clusters require topology graph"
        }))
    }

    /// Generate a conversational response to the user
    async fn tool_respond_to_user(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let message = args.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("I'm Topsi, your topological super intelligence. How can I help you today?");

        Ok(serde_json::json!({
            "response": message,
            "spoken": true
        }))
    }

    async fn tool_verify_access(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let user_id = args.get("user_id").and_then(|v| v.as_str());
        let resource_type = args.get("resource_type").and_then(|v| v.as_str());
        let resource_id = args.get("resource_id").and_then(|v| v.as_str());
        let action = args.get("action").and_then(|v| v.as_str());

        // Basic access check based on current scope
        let allowed = match scope {
            AccessScope::Admin => true,
            AccessScope::Projects(ids) => {
                if resource_type == Some("project") {
                    if let Some(rid) = resource_id {
                        if let Ok(uuid) = Uuid::parse_str(rid) {
                            ids.contains(&uuid)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    true // Allow other resources by default
                }
            }
            AccessScope::SingleProject(id) => {
                if resource_type == Some("project") {
                    resource_id == Some(&id.to_string())
                } else {
                    true
                }
            }
            AccessScope::None => false,
        };

        Ok(serde_json::json!({
            "user_id": user_id,
            "resource_type": resource_type,
            "resource_id": resource_id,
            "action": action,
            "allowed": allowed,
            "scope": format!("{:?}", scope)
        }))
    }

    /// Handle topology requests
    async fn handle_get_topology(
        &self,
        project_id: Option<Uuid>,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // Verify access to the project
        if let Some(pid) = project_id {
            if !self.access_control.can_access_project(user_context, pid).await {
                return Err(TopsiError::TopologyError(
                    "Access denied to project".to_string(),
                ));
            }
        }

        // Get topology summary
        let summary = self.get_topology_summary(project_id, scope).await?;

        Ok(TopsiResponse {
            message: "Topology retrieved successfully".to_string(),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: Some(summary),
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle issue detection
    async fn handle_detect_issues(
        &self,
        project_id: Option<Uuid>,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // Verify access
        if let Some(pid) = project_id {
            if !self.access_control.can_access_project(user_context, pid).await {
                return Err(TopsiError::TopologyError(
                    "Access denied to project".to_string(),
                ));
            }
        }

        // Detect issues (stub for now)
        let issues = self.detect_issues(project_id, scope).await?;

        Ok(TopsiResponse {
            message: format!("Detected {} issues", issues.len()),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues,
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle listing projects
    async fn handle_list_projects(
        &self,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        let project_count = match scope {
            AccessScope::Admin => {
                // Get all projects
                if let Some(pool) = &self.db {
                    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
                        .fetch_one(pool)
                        .await
                        .unwrap_or(0);
                    count as usize
                } else {
                    0
                }
            }
            AccessScope::Projects(ids) => ids.len(),
            AccessScope::SingleProject(_) => 1,
            AccessScope::None => 0,
        };

        Ok(TopsiResponse {
            message: format!("You have access to {} projects", project_count),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle command execution
    async fn handle_command(
        &self,
        command: &str,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // Parse and execute command
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(TopsiResponse {
                message: "No command provided".to_string(),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            });
        }

        match parts[0].to_lowercase().as_str() {
            "status" => {
                let is_admin = matches!(scope, AccessScope::Admin);
                Ok(TopsiResponse {
                    message: format!(
                        "Topsi Status:\n- Active: {}\n- Uptime: {}ms\n- Admin: {}",
                        self.is_active().await,
                        self.uptime_ms(),
                        is_admin
                    ),
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: None,
                    output_tokens: None,
                })
            }
            "help" => {
                Ok(TopsiResponse {
                    message: "Available commands: status, help, topology, issues".to_string(),
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: None,
                    output_tokens: None,
                })
            }
            _ => {
                Ok(TopsiResponse {
                    message: format!("Unknown command: {}", parts[0]),
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: None,
                    output_tokens: None,
                })
            }
        }
    }

    /// Get topology summary for accessible projects
    async fn get_topology_summary(
        &self,
        project_id: Option<Uuid>,
        scope: &AccessScope,
    ) -> Result<TopologySummary> {
        // Return a basic summary for now
        // TODO: Integrate with actual topology data from database
        Ok(TopologySummary {
            node_count: 0,
            edge_count: 0,
            cluster_count: 0,
            active_routes: 0,
            unresolved_issues: 0,
            nodes_by_type: vec![],
            edges_by_type: vec![],
            health_score: 1.0,
        })
    }

    /// Detect issues in the topology
    async fn detect_issues(
        &self,
        project_id: Option<Uuid>,
        scope: &AccessScope,
    ) -> Result<Vec<DetectedIssue>> {
        // Return empty for now
        // TODO: Implement actual issue detection
        Ok(vec![])
    }
}

/// Request types for Topsi
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum TopsiRequestType {
    /// Chat with Topsi
    Chat { message: String },
    /// Get topology for a project
    GetTopology { project_id: Option<Uuid> },
    /// Detect issues in topology
    DetectIssues { project_id: Option<Uuid> },
    /// List accessible projects
    ListProjects,
    /// Execute a command
    ExecuteCommand { command: String },
}

/// A request to Topsi
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiRequest {
    /// Request ID
    pub id: Uuid,
    /// Request type
    #[serde(flatten)]
    pub request_type: TopsiRequestType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl TopsiRequest {
    pub fn new(request_type: TopsiRequestType) -> Self {
        Self {
            id: Uuid::new_v4(),
            request_type,
            timestamp: Utc::now(),
        }
    }
}

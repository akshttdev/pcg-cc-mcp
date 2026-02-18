use db::models::{
    pulse_alert::PulseAlert,
    pulse_content_item::PulseContentItem,
    pulse_source::PulseSource,
};
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

// --- Request types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchPulseContentRequest {
    #[schemars(description = "The project ID to search content in")]
    pub project_id: String,
    #[schemars(description = "Optional keyword to search for in title/body")]
    pub keyword: Option<String>,
    #[schemars(description = "Optional source_id to filter by")]
    pub source_id: Option<String>,
    #[schemars(description = "Maximum results to return (default: 20)")]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetLatestPulseContentRequest {
    #[schemars(description = "The project ID to get latest content from")]
    pub project_id: String,
    #[schemars(description = "Number of items to return (default: 10)")]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TriggerPulseCollectionRequest {
    #[schemars(description = "The project ID to trigger collection for")]
    pub project_id: String,
    #[schemars(description = "Optional specific source_id to collect from")]
    pub source_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPulseStatusRequest {
    #[schemars(description = "The project ID to get status for")]
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskFromPulseRequest {
    #[schemars(description = "The project ID")]
    pub project_id: String,
    #[schemars(description = "The content item ID to create a task from")]
    pub content_item_id: String,
    #[schemars(description = "Title for the new task")]
    pub title: Option<String>,
    #[schemars(description = "Description for the new task")]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListPulseAlertsRequest {
    #[schemars(description = "The project ID to list alerts for")]
    pub project_id: String,
    #[schemars(description = "Maximum alerts to return (default: 20)")]
    pub limit: Option<i64>,
}

// --- Server ---

#[derive(Debug, Clone)]
pub struct PulseServer {
    pub pool: SqlitePool,
    tool_router: ToolRouter<PulseServer>,
}

impl PulseServer {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl PulseServer {
    #[tool(
        description = "Search Pulse Engine content items by keyword, source, or relevance. Returns collected news, articles, and social media content matching the query."
    )]
    async fn search_pulse_content(
        &self,
        Parameters(req): Parameters<SearchPulseContentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&req.project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": "Invalid project ID"}).to_string(),
                )]));
            }
        };

        let limit = req.limit.unwrap_or(20);
        match PulseContentItem::search(
            &self.pool,
            project_uuid,
            req.keyword.as_deref(),
            req.source_id.as_deref(),
            None,
            limit,
        )
        .await
        {
            Ok(items) => {
                let response = serde_json::json!({
                    "success": true,
                    "count": items.len(),
                    "items": items.iter().map(|i| serde_json::json!({
                        "id": i.id,
                        "title": i.title,
                        "url": i.url,
                        "source_id": i.source_id,
                        "source_type": i.source_type,
                        "summary": i.summary,
                        "relevance_score": i.relevance_score,
                        "collected_at": i.collected_at,
                        "status": i.pcg_status,
                    })).collect::<Vec<_>>()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_default(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Get the latest content items collected by Pulse Engine for a project. Returns the most recently collected items."
    )]
    async fn get_latest_pulse_content(
        &self,
        Parameters(req): Parameters<GetLatestPulseContentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&req.project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": "Invalid project ID"}).to_string(),
                )]));
            }
        };

        let limit = req.limit.unwrap_or(10);
        match PulseContentItem::find_latest(&self.pool, project_uuid, limit).await {
            Ok(items) => {
                let response = serde_json::json!({
                    "success": true,
                    "count": items.len(),
                    "items": items.iter().map(|i| serde_json::json!({
                        "id": i.id,
                        "title": i.title,
                        "url": i.url,
                        "source_id": i.source_id,
                        "summary": i.summary,
                        "relevance_score": i.relevance_score,
                        "collected_at": i.collected_at,
                        "author": i.author,
                    })).collect::<Vec<_>>()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_default(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Trigger a content collection run on the Pulse Engine. Optionally specify a source_id to collect from a specific source."
    )]
    async fn trigger_pulse_collection(
        &self,
        Parameters(req): Parameters<TriggerPulseCollectionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let pulse_url = std::env::var("PULSE_API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let mut url = format!("{}/api/v1/collect", pulse_url);
        if let Some(ref source_id) = req.source_id {
            url = format!("{}?source_id={}", url, source_id);
        }

        let client = reqwest::Client::new();
        match client.post(&url).send().await {
            Ok(resp) => {
                let data: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({"status": "triggered"}));
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "message": "Collection triggered",
                        "result": data,
                    }))
                    .unwrap_or_default(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::json!({"success": false, "error": format!("Pulse Engine unreachable: {}", e)}).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Get Pulse Engine health status and statistics for a project, including source counts and engine connectivity."
    )]
    async fn get_pulse_status(
        &self,
        Parameters(req): Parameters<GetPulseStatusRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&req.project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": "Invalid project ID"}).to_string(),
                )]));
            }
        };

        let sources = PulseSource::find_by_project(&self.pool, project_uuid)
            .await
            .unwrap_or_default();
        let total_content = PulseContentItem::count_by_project(&self.pool, project_uuid)
            .await
            .unwrap_or(0);
        let unacked_alerts = PulseAlert::count_unacknowledged(&self.pool, project_uuid)
            .await
            .unwrap_or(0);

        // Check engine health
        let pulse_url = std::env::var("PULSE_API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let client = reqwest::Client::new();
        let engine_online = client
            .get(&format!("{}/health", pulse_url))
            .send()
            .await
            .is_ok();

        let response = serde_json::json!({
            "success": true,
            "engine_online": engine_online,
            "total_sources": sources.len(),
            "active_sources": sources.iter().filter(|s| s.status == "active" && s.enabled).count(),
            "total_content_items": total_content,
            "unacknowledged_alerts": unacked_alerts,
            "sources": sources.iter().map(|s| serde_json::json!({
                "source_id": s.source_id,
                "source_type": s.source_type,
                "name": s.name,
                "status": s.status,
                "enabled": s.enabled,
            })).collect::<Vec<_>>(),
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    #[tool(
        description = "Create a task from a Pulse Engine content item. Promotes a content item to a tracked task in the project."
    )]
    async fn create_task_from_pulse(
        &self,
        Parameters(req): Parameters<CreateTaskFromPulseRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&req.project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": "Invalid project ID"}).to_string(),
                )]));
            }
        };

        // Find the content item
        let item = match PulseContentItem::find_by_id(&self.pool, &req.content_item_id).await {
            Ok(Some(item)) => item,
            Ok(None) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": "Content item not found"}).to_string(),
                )]));
            }
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
                )]));
            }
        };

        let title = req.title.unwrap_or_else(|| format!("[Pulse] {}", item.title));
        let description = req.description.unwrap_or_else(|| {
            format!(
                "From Pulse Engine content:\n\nSource: {}\nURL: {}\n\n{}",
                item.source_id,
                item.url,
                item.summary.as_deref().unwrap_or(item.body.as_deref().unwrap_or("")),
            )
        });

        let task_id = Uuid::new_v4();
        match sqlx::query(
            r#"INSERT INTO tasks (id, project_id, title, description, status, priority, created_by, created_at, updated_at)
               VALUES (?, ?, ?, ?, 'todo', 'medium', 'pulse-engine', datetime('now'), datetime('now'))"#,
        )
        .bind(task_id)
        .bind(project_uuid)
        .bind(&title)
        .bind(&description)
        .execute(&self.pool)
        .await
        {
            Ok(_) => {
                // Link content item to task
                let _ = PulseContentItem::link_task(&self.pool, &req.content_item_id, task_id).await;

                let response = serde_json::json!({
                    "success": true,
                    "task_id": task_id.to_string(),
                    "title": title,
                    "message": "Task created from Pulse content",
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_default(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
            )])),
        }
    }

    #[tool(
        description = "List active alerts from Pulse Engine for a project. Shows triggered alert rules and their associated content."
    )]
    async fn list_pulse_alerts(
        &self,
        Parameters(req): Parameters<ListPulseAlertsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&req.project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::json!({"success": false, "error": "Invalid project ID"}).to_string(),
                )]));
            }
        };

        let limit = req.limit.unwrap_or(20);
        match PulseAlert::find_by_project(&self.pool, project_uuid, limit).await {
            Ok(alerts) => {
                let response = serde_json::json!({
                    "success": true,
                    "count": alerts.len(),
                    "alerts": alerts.iter().map(|a| serde_json::json!({
                        "id": a.id,
                        "rule_id": a.rule_id,
                        "content_item_id": a.content_item_id,
                        "priority": a.priority,
                        "acknowledged": a.acknowledged,
                        "auto_task_id": a.auto_task_id,
                        "created_at": a.created_at.to_rfc3339(),
                    })).collect::<Vec<_>>()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_default(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
            )])),
        }
    }
}

#[tool_handler]
impl ServerHandler for PulseServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Pulse Engine MCP tools for content monitoring, search, collection, and alert management."
                    .to_string(),
            ),
        }
    }
}

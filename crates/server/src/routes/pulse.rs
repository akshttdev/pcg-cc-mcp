use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    response::sse::{Event, Sse},
    routing::{get, post, put},
};
use db::models::{
    pulse_alert::PulseAlert,
    pulse_alert_rule::{CreatePulseAlertRule, PulseAlertRule, UpdatePulseAlertRule},
    pulse_collection_run::PulseCollectionRun,
    pulse_content_item::{ContentActionRequest, PulseContentItem},
    pulse_source::{CreatePulseSource, PulseSource, UpdatePulseSource},
    pulse_tracking_config::{PulseTrackingConfig, UpdatePulseTrackingConfig},
};
use deployment::Deployment;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, pulse_publisher};

/// Pulse Engine API URL (configured via PULSE_API_URL env var)
fn pulse_api_url() -> String {
    std::env::var("PULSE_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

// --- Legacy response types (kept for backwards compat with frontend) ---

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseProject {
    pub name: String,
    pub description: Option<String>,
    pub adapters: i64,
    pub active_sources: Vec<String>,
    pub scheduler_running: bool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseProjectsResponse {
    pub projects: Vec<PulseProject>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseContentResponse {
    pub items: Vec<PulseContentItem>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseDashboardStats {
    pub total_content: i64,
    pub total_sources: usize,
    pub active_sources: usize,
    pub unacknowledged_alerts: i64,
}

// --- Query types ---

#[derive(Debug, Deserialize)]
pub struct ContentQueryParams {
    pub keyword: Option<String>,
    pub source_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyContentQuery {
    pub project: Option<String>,
    pub keyword: Option<String>,
    pub source_id: Option<String>,
    pub limit: Option<i64>,
}

// ========================
// Sources CRUD
// ========================

async fn list_sources(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<PulseSource>>>, ApiError> {
    let pool = &deployment.db().pool;
    let sources = PulseSource::find_by_project(pool, &project_id.to_string()).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to fetch sources: {}", e))
    })?;
    Ok(ResponseJson(ApiResponse::success(sources)))
}

async fn create_source(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    ResponseJson(mut data): ResponseJson<CreatePulseSource>,
) -> Result<ResponseJson<ApiResponse<PulseSource>>, ApiError> {
    data.project_id = project_id.to_string();
    let pool = &deployment.db().pool;
    let source = PulseSource::create(pool, &data).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to create source: {}", e))
    })?;
    Ok(ResponseJson(ApiResponse::success(source)))
}

async fn update_source(
    State(deployment): State<DeploymentImpl>,
    Path((_project_id, source_id)): Path<(Uuid, String)>,
    ResponseJson(data): ResponseJson<UpdatePulseSource>,
) -> Result<ResponseJson<ApiResponse<PulseSource>>, ApiError> {
    let pool = &deployment.db().pool;
    let source = PulseSource::update(pool, &source_id, &data).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to update source: {}", e))
    })?;
    Ok(ResponseJson(ApiResponse::success(source)))
}

async fn delete_source(
    State(deployment): State<DeploymentImpl>,
    Path((_project_id, source_id)): Path<(Uuid, String)>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    PulseSource::delete(pool, &source_id).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to delete source: {}", e))
    })?;
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({"deleted": true}))))
}

// ========================
// Content
// ========================

async fn list_content(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ContentQueryParams>,
) -> Result<ResponseJson<ApiResponse<PulseContentResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50);
    let items = PulseContentItem::search(
        pool,
        project_id,
        query.keyword.as_deref(),
        query.source_id.as_deref(),
        query.status.as_deref(),
        limit,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch content: {}", e)))?;

    let count = items.len();
    Ok(ResponseJson(ApiResponse::success(PulseContentResponse { items, count })))
}

async fn get_latest_content(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<LimitQuery>,
) -> Result<ResponseJson<ApiResponse<PulseContentResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(20);
    let items = PulseContentItem::find_latest(pool, project_id, limit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch latest content: {}", e)))?;

    let count = items.len();
    Ok(ResponseJson(ApiResponse::success(PulseContentResponse { items, count })))
}

async fn get_content_item(
    State(deployment): State<DeploymentImpl>,
    Path((_project_id, content_id)): Path<(Uuid, String)>,
) -> Result<ResponseJson<ApiResponse<PulseContentItem>>, ApiError> {
    let pool = &deployment.db().pool;
    let item = PulseContentItem::find_by_id(pool, &content_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch content item: {}", e)))?
        .ok_or(ApiError::NotFound("Content item not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(item)))
}

async fn content_action(
    State(deployment): State<DeploymentImpl>,
    Path((_project_id, content_id)): Path<(Uuid, String)>,
    ResponseJson(action): ResponseJson<ContentActionRequest>,
) -> Result<ResponseJson<ApiResponse<PulseContentItem>>, ApiError> {
    let pool = &deployment.db().pool;

    match action.action.as_str() {
        "review" => {
            let item = PulseContentItem::update_status(pool, &content_id, "reviewed")
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update status: {}", e)))?;
            Ok(ResponseJson(ApiResponse::success(item)))
        }
        "dismiss" => {
            let item = PulseContentItem::update_status(pool, &content_id, "dismissed")
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update status: {}", e)))?;
            Ok(ResponseJson(ApiResponse::success(item)))
        }
        "create-task" => {
            let title = action.task_title.unwrap_or_else(|| format!("[Pulse] Content review"));
            let task_id = Uuid::new_v4();
            let _content_item = PulseContentItem::find_by_id(pool, &content_id)
                .await
                .map_err(|e| ApiError::InternalError(format!("Content not found: {}", e)))?
                .ok_or(ApiError::NotFound("Content item not found".to_string()))?;

            // Create task
            let _ = sqlx::query(
                r#"INSERT INTO tasks (id, project_id, title, description, status, priority, created_by, created_at, updated_at)
                   VALUES (?, ?, ?, ?, 'todo', 'medium', 'pulse-engine', datetime('now'), datetime('now'))"#,
            )
            .bind(task_id)
            .bind(_content_item.project_id)
            .bind(&title)
            .bind(action.task_description.as_deref())
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to create task: {}", e)))?;

            let item = PulseContentItem::link_task(pool, &content_id, task_id)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to link task: {}", e)))?;
            Ok(ResponseJson(ApiResponse::success(item)))
        }
        "link-crm" => {
            let contact_id = action.crm_contact_id.ok_or(
                ApiError::BadRequest("crm_contact_id required for link-crm action".to_string()),
            )?;
            let item = PulseContentItem::link_crm_contact(pool, &content_id, contact_id)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to link CRM contact: {}", e)))?;
            Ok(ResponseJson(ApiResponse::success(item)))
        }
        _ => Err(ApiError::BadRequest(format!("Unknown action: {}", action.action))),
    }
}

// ========================
// Alerts
// ========================

async fn list_alerts(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<LimitQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<PulseAlert>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50);
    let alerts = PulseAlert::find_by_project(pool, project_id, limit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch alerts: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(alerts)))
}

async fn list_alert_rules(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<PulseAlertRule>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rules = PulseAlertRule::find_by_project(pool, project_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch alert rules: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(rules)))
}

async fn create_alert_rule(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    ResponseJson(mut data): ResponseJson<CreatePulseAlertRule>,
) -> Result<ResponseJson<ApiResponse<PulseAlertRule>>, ApiError> {
    data.project_id = project_id;
    let pool = &deployment.db().pool;
    let rule = PulseAlertRule::create(pool, &data)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create alert rule: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(rule)))
}

async fn update_alert_rule(
    State(deployment): State<DeploymentImpl>,
    Path((_project_id, rule_id)): Path<(Uuid, String)>,
    ResponseJson(data): ResponseJson<UpdatePulseAlertRule>,
) -> Result<ResponseJson<ApiResponse<PulseAlertRule>>, ApiError> {
    let pool = &deployment.db().pool;
    let rule = PulseAlertRule::update(pool, &rule_id, &data)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update alert rule: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(rule)))
}

async fn delete_alert_rule(
    State(deployment): State<DeploymentImpl>,
    Path((_project_id, rule_id)): Path<(Uuid, String)>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    PulseAlertRule::delete(pool, &rule_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete alert rule: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({"deleted": true}))))
}

// ========================
// Collection
// ========================

async fn trigger_collection(
    State(_deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    // Publish collect command via NATS for real-time delivery
    if let Some(publisher) = pulse_publisher::get_publisher() {
        publisher
            .publish_collect_command("_", &project_id.to_string(), None)
            .await;
    }

    // Also proxy to the Pulse Engine HTTP API
    let url = format!("{}/api/v1/collect?project_id={}", pulse_api_url(), project_id);
    let client = reqwest::Client::new();
    let resp = client.post(&url).send().await.map_err(|e| {
        ApiError::InternalError(format!("Failed to reach Pulse Engine: {}", e))
    })?;

    let data: serde_json::Value = resp.json().await.map_err(|e| {
        ApiError::InternalError(format!("Failed to parse Pulse response: {}", e))
    })?;

    Ok(ResponseJson(ApiResponse::success(data)))
}

async fn list_runs(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<LimitQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<PulseCollectionRun>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50);
    let runs = PulseCollectionRun::find_by_project(pool, project_id, limit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch runs: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(runs)))
}

// ========================
// Tracking config
// ========================

async fn get_tracking_config(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Option<PulseTrackingConfig>>>, ApiError> {
    let pool = &deployment.db().pool;
    let config = PulseTrackingConfig::find_by_project(pool, project_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch tracking config: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(config)))
}

async fn update_tracking_config(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    ResponseJson(data): ResponseJson<UpdatePulseTrackingConfig>,
) -> Result<ResponseJson<ApiResponse<PulseTrackingConfig>>, ApiError> {
    let pool = &deployment.db().pool;
    let config = PulseTrackingConfig::upsert(pool, project_id, None, &data)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update tracking config: {}", e)))?;
    Ok(ResponseJson(ApiResponse::success(config)))
}

// ========================
// Engine status (proxy)
// ========================

async fn engine_status(
    State(_deployment): State<DeploymentImpl>,
    Path(_project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let url = format!("{}/health", pulse_api_url());
    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            let data: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({"status": "unknown"}));
            Ok(ResponseJson(ApiResponse::success(data)))
        }
        Err(_) => {
            Ok(ResponseJson(ApiResponse::success(serde_json::json!({
                "status": "offline",
                "message": "Pulse Engine is not reachable"
            }))))
        }
    }
}

// ========================
// Dashboard stats
// ========================

async fn dashboard_stats(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<PulseDashboardStats>>, ApiError> {
    let pool = &deployment.db().pool;

    let total_content = PulseContentItem::count_by_project(pool, project_id)
        .await
        .unwrap_or(0);

    // Try project-scoped sources first; if empty, fall back to organization-scoped
    let mut sources = PulseSource::find_by_project(pool, &project_id.to_string())
        .await
        .unwrap_or_default();

    if sources.is_empty() {
        // Look up the project's organization_id and query org-scoped sources
        if let Ok(Some(project)) = db::models::project::Project::find_by_id(pool, &project_id.to_string()).await {
            if let Some(ref org_id) = project.organization_id {
                sources = PulseSource::find_by_organization(pool, org_id)
                    .await
                    .unwrap_or_default();
            }
        }
    }

    let total_sources = sources.len();
    let active_sources = sources.iter().filter(|s| s.status == "active" && s.enabled).count();

    let unacknowledged_alerts = PulseAlert::count_unacknowledged(pool, project_id)
        .await
        .unwrap_or(0);

    Ok(ResponseJson(ApiResponse::success(PulseDashboardStats {
        total_content,
        total_sources,
        active_sources,
        unacknowledged_alerts,
    })))
}

// ========================
// Legacy routes (backwards compat for existing frontend)
// ========================

async fn legacy_list_projects(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<PulseProjectsResponse>>, ApiError> {
    let url = format!("{}/api/v1/projects", pulse_api_url());
    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            let data: PulseProjectsResponse = resp.json().await.map_err(|e| {
                ApiError::InternalError(format!("Failed to parse Pulse response: {}", e))
            })?;
            Ok(ResponseJson(ApiResponse::success(data)))
        }
        Err(_) => {
            Ok(ResponseJson(ApiResponse::success(PulseProjectsResponse {
                projects: vec![],
                count: 0,
            })))
        }
    }
}

async fn legacy_get_latest_content(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<LegacyContentQuery>,
) -> Result<ResponseJson<ApiResponse<PulseContentResponse>>, ApiError> {
    let _pool = &deployment.db().pool;
    let _limit = query.limit.unwrap_or(20);

    // Fall back to proxying to Pulse Engine
    let mut url = format!("{}/api/v1/content/latest", pulse_api_url());
    let mut params = Vec::new();
    if let Some(ref project) = query.project {
        params.push(format!("project={}", project));
    }
    if let Some(l) = query.limit {
        params.push(format!("limit={}", l));
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            let data: PulseContentResponse = resp.json().await.map_err(|e| {
                ApiError::InternalError(format!("Failed to parse Pulse response: {}", e))
            })?;
            Ok(ResponseJson(ApiResponse::success(data)))
        }
        Err(_) => {
            Ok(ResponseJson(ApiResponse::success(PulseContentResponse {
                items: vec![],
                count: 0,
            })))
        }
    }
}

// ========================
// SSE — Real-time content feed
// ========================

/// SSE endpoint that polls for new pulse content items and streams them to the frontend.
/// Clients receive `pulse_content` events whenever new content arrives.
async fn stream_pulse_content(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pool = deployment.db().pool.clone();

    let stream = stream::unfold(
        (pool, project_id, String::new()),
        |(pool, project_id, mut last_id)| async move {
            // Poll every 2 seconds for new content
            tokio::time::sleep(Duration::from_secs(2)).await;

            match PulseContentItem::find_latest(&pool, project_id, 5).await {
                Ok(items) if !items.is_empty() => {
                    // Only send items we haven't sent before
                    let new_items: Vec<_> = if last_id.is_empty() {
                        // First poll — send the latest item's ID as baseline, don't flood
                        last_id = items[0].id.clone();
                        vec![]
                    } else {
                        items
                            .into_iter()
                            .take_while(|item| item.id != last_id)
                            .collect()
                    };

                    if !new_items.is_empty() {
                        last_id = new_items[0].id.clone();
                        let json = serde_json::to_string(&new_items)
                            .unwrap_or_else(|_| "[]".to_string());
                        let event = Event::default().data(json).event("pulse_content");
                        Some((Ok(event), (pool, project_id, last_id)))
                    } else {
                        let event = Event::default().comment("keepalive");
                        Some((Ok(event), (pool, project_id, last_id)))
                    }
                }
                _ => {
                    let event = Event::default().comment("keepalive");
                    Some((Ok(event), (pool, project_id, last_id)))
                }
            }
        },
    );

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

// --- Router ---

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Project-scoped routes (new API)
        .route("/api/pulse/projects/{project_id}/stats", get(dashboard_stats))
        .route("/api/pulse/projects/{project_id}/sources", get(list_sources).post(create_source))
        .route("/api/pulse/projects/{project_id}/sources/{source_id}", put(update_source).delete(delete_source))
        .route("/api/pulse/projects/{project_id}/content", get(list_content))
        .route("/api/pulse/projects/{project_id}/content/latest", get(get_latest_content))
        .route("/api/pulse/projects/{project_id}/content/{content_id}", get(get_content_item))
        .route("/api/pulse/projects/{project_id}/content/{content_id}/action", post(content_action))
        .route("/api/pulse/projects/{project_id}/alerts", get(list_alerts))
        .route("/api/pulse/projects/{project_id}/alert-rules", get(list_alert_rules).post(create_alert_rule))
        .route("/api/pulse/projects/{project_id}/alert-rules/{rule_id}", put(update_alert_rule).delete(delete_alert_rule))
        .route("/api/pulse/projects/{project_id}/collect", post(trigger_collection))
        .route("/api/pulse/projects/{project_id}/runs", get(list_runs))
        .route("/api/pulse/projects/{project_id}/tracking", get(get_tracking_config).put(update_tracking_config))
        .route("/api/pulse/projects/{project_id}/engine/status", get(engine_status))
        // SSE real-time stream
        .route("/api/pulse/projects/{project_id}/stream", get(stream_pulse_content))
        // Legacy routes (backwards compat)
        .route("/api/pulse/projects", get(legacy_list_projects))
        .route("/api/pulse/content/latest", get(legacy_get_latest_content))
}

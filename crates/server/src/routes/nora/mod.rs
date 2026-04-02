//! Nora executive assistant API routes

pub mod chat;
pub mod config;
pub mod coordination;
pub mod initialization;
pub mod modes;
pub mod project_ops;
pub mod rate_limiter;
pub mod voice;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::{
    agent_conversation::{AgentConversation, AgentConversationMessage},
    project::Project,
};
use deployment::Deployment;
use nora::{
    agent::{
        NoraRequest, NoraRequestType, NoraResponse, RapidPlaybookRequest, RapidPlaybookResult,
        RequestPriority,
    },
    coordination::{AgentCoordinationState, CoordinationEvent, CoordinationStats},
    graph::{GraphNodeStatus, GraphPlan, GraphPlanSummary},
    memory::{BudgetStatus, ProjectContext, ProjectStatus},
    personality::PersonalityConfig,
    tools::{NoraExecutiveTool, ToolExecutionResult},
    voice::{SpeechResponse, VoiceConfig, VoiceEngine, VoiceError, VoiceInteraction},
    NoraAgent, NoraConfig, NoraError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx;
use tokio::sync::{broadcast, RwLock};
use ts_rs::TS;
use uuid::Uuid;

// Re-export items used by sub-modules and external modules
pub use self::coordination::emit_coordination_event;
pub use self::{config::NoraModeSummary, initialization::initialize_nora_on_startup};
pub(crate) use self::{
    config::NORA_MODE_PRESETS,
    rate_limiter::{get_chat_rate_limiter, get_voice_rate_limiter},
};
use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

/// Global Nora agent instance
pub(crate) static NORA_INSTANCE: tokio::sync::OnceCell<Arc<RwLock<Option<NoraAgent>>>> =
    tokio::sync::OnceCell::const_new();

/// Global Nora initialization timestamp
pub(crate) static NORA_INIT_TIME: tokio::sync::OnceCell<DateTime<Utc>> =
    tokio::sync::OnceCell::const_new();

/// Nora manager for coordinating agent instances
#[derive(Clone)]
pub struct NoraManager {
    agent: Arc<RwLock<Option<NoraAgent>>>,
}

impl NoraManager {
    pub async fn new() -> Self {
        let agent = NORA_INSTANCE
            .get_or_init(|| async { Arc::new(RwLock::new(None)) })
            .await
            .clone();
        Self { agent }
    }

    pub async fn process_request(&self, request: NoraRequest) -> Result<NoraResponse, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.process_request(request).await
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn get_coordination_stats(&self) -> Result<CoordinationStats, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.coordination_manager
                .get_coordination_stats()
                .await
                .map_err(|e| NoraError::CoordinationError(e.to_string()))
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn get_all_agents(&self) -> Result<Vec<AgentCoordinationState>, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.coordination_manager
                .get_all_agents()
                .await
                .map_err(|e| NoraError::CoordinationError(e.to_string()))
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn initialize(&self, config: NoraConfig) -> Result<String, NoraError> {
        let mut agent = self.agent.write().await;
        let nora = NoraAgent::new(config).await?;
        let id = nora.id.to_string();
        let _ = NORA_INIT_TIME.set(Utc::now());
        *agent = Some(nora);
        Ok(id)
    }

    pub async fn is_active(&self) -> bool {
        let agent = self.agent.read().await;
        agent.is_some()
    }

    pub async fn get_uptime_ms(&self) -> Option<u64> {
        NORA_INIT_TIME.get().map(|init_time| {
            Utc::now()
                .signed_duration_since(*init_time)
                .num_milliseconds() as u64
        })
    }

    pub async fn sync_live_context(&self) -> Result<usize, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.sync_live_context().await
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn run_rapid_playbook(
        &self,
        payload: RapidPlaybookRequest,
    ) -> Result<RapidPlaybookResult, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.run_rapid_playbook(payload).await
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn reinitialize_with_config(
        &self,
        config: NoraConfig,
        preserve_memory: bool,
    ) -> Result<String, NoraError> {
        let mut agent_guard = self.agent.write().await;
        let new_agent = NoraAgent::new(config).await?;
        if preserve_memory {
            if let Some(old) = agent_guard.as_ref() {
                let old_memory = old.memory.read().await.clone();
                let old_context = old.context.read().await.clone();
                *new_agent.memory.write().await = old_memory;
                *new_agent.context.write().await = old_context;
            }
        }
        let id = new_agent.id.to_string();
        *agent_guard = Some(new_agent);
        Ok(id)
    }

    pub async fn list_graph_plans(&self) -> Result<Vec<GraphPlanSummary>, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            Ok(nora.graph_plan_summaries().await)
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn get_graph_plan(&self, plan_id: &str) -> Result<GraphPlan, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.graph_plan_detail(plan_id)
                .await
                .ok_or_else(|| NoraError::ConfigError("Plan not found".to_string()))
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }

    pub async fn update_graph_node_status(
        &self,
        plan_id: &str,
        node_id: &str,
        status: GraphNodeStatus,
    ) -> Result<GraphPlan, NoraError> {
        let agent = self.agent.read().await;
        if let Some(nora) = agent.as_ref() {
            nora.update_graph_node_status(plan_id, node_id, status)
                .await
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }
}

/// Initialize Nora routes
pub fn nora_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/nora/initialize", post(initialization::initialize_nora))
        .route("/nora/status", get(get_nora_status))
        .route("/nora/chat", post(chat::chat_with_nora))
        .route("/nora/chat/stream", post(chat::chat_with_nora_stream))
        .route("/nora/cache/stats", get(get_cache_stats))
        .route("/nora/cache/clear", post(clear_cache))
        .route(
            "/nora/voice/config",
            get(voice::get_voice_config).put(voice::update_voice_config),
        )
        .route("/nora/voice/synthesize", post(voice::synthesize_speech))
        .route("/nora/voice/transcribe", post(voice::transcribe_speech))
        .route("/nora/voice/interaction", post(voice::voice_interaction))
        .route(
            "/nora/voice/analytics/users/{user_id}",
            get(voice::get_user_voice_analytics),
        )
        .route(
            "/nora/voice/analytics/users",
            get(voice::get_all_users_voice_analytics),
        )
        .route(
            "/nora/voice/conversations/{session_id}",
            get(voice::get_session_conversation),
        )
        .route("/nora/tools/execute", post(execute_executive_tool))
        .route("/nora/tools/available", get(get_available_tools))
        .route("/nora/context/sync", post(sync_live_context_handler))
        .route("/nora/modes", get(modes::list_modes_handler))
        .route("/nora/modes/apply", post(modes::apply_mode_handler))
        .route("/nora/playbooks/rapid", post(modes::rapid_playbook_handler))
        .route("/nora/graph/plans", get(modes::list_graph_plans_handler))
        .route(
            "/nora/graph/plans/{plan_id}",
            get(modes::get_graph_plan_handler),
        )
        .route(
            "/nora/graph/plans/{plan_id}/nodes/{node_id}",
            patch(modes::update_graph_node_handler),
        )
        .route(
            "/nora/coordination/stats",
            get(coordination::get_coordination_stats),
        )
        .route(
            "/nora/coordination/agents",
            get(coordination::get_coordination_agents),
        )
        .route(
            "/nora/coordination/events",
            get(coordination::get_coordination_events_ws),
        )
        .route(
            "/nora/coordination/events/sse",
            get(coordination::get_coordination_events_sse),
        )
        .route(
            "/nora/coordination/agents/{agent_id}/directives",
            post(coordination::send_agent_directive),
        )
        .route("/nora/personality/config", get(get_personality_config))
        .route("/nora/personality/config", post(update_personality_config))
        .route(
            "/nora/project/create",
            post(project_ops::nora_create_project),
        )
        .route("/nora/board/create", post(project_ops::nora_create_board))
        .route("/nora/task/create", post(project_ops::nora_create_task))
        .layer(axum::middleware::from_fn(
            crate::middleware::request_id_middleware,
        ))
}

// ── Shared request/response types ──────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeNoraRequest {
    pub config: Option<NoraConfig>,
    pub activate_immediately: bool,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeNoraResponse {
    pub success: bool,
    pub nora_id: String,
    pub message: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraStatusResponse {
    pub is_active: bool,
    pub nora_id: Option<String>,
    pub uptime_ms: Option<u64>,
    pub voice_enabled: bool,
    pub coordination_enabled: bool,
    pub executive_mode: bool,
    pub personality_mode: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextSyncResponse {
    pub projects_refreshed: usize,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplyModeRequest {
    pub mode_id: String,
    #[serde(default)]
    pub preserve_memory: bool,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplyModeResponse {
    pub active_mode: String,
    pub nora_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RapidPlaybookBody {
    pub project_name: String,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub repo_hint: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNodeStatusBody {
    pub status: GraphNodeStatus,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub message: String,
    pub session_id: String,
    pub request_type: Option<NoraRequestType>,
    #[serde(default)]
    pub voice_enabled: bool,
    pub priority: Option<RequestPriority>,
    pub context: Option<serde_json::Value>,
    pub stream: Option<bool>,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSynthesisRequest {
    pub text: String,
    pub voice_profile: Option<String>,
    pub speed: Option<f32>,
    pub volume: Option<f32>,
    pub british_accent: Option<bool>,
    pub executive_tone: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionRequest {
    #[serde(alias = "audio")]
    pub audio_data: String,
    pub language: Option<String>,
    pub british_dialect: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionResponse {
    pub text: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteToolRequest {
    pub tool: NoraExecutiveTool,
    pub session_id: String,
    pub user_permissions: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceConfigResponse {
    pub config: VoiceConfig,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVoiceConfigRequest {
    pub config: VoiceConfig,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AvailableToolsResponse {
    pub tools: Vec<ToolInfo>,
    pub categories: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentDirectiveRequest {
    pub session_id: String,
    pub content: String,
    pub command: Option<String>,
    pub priority: Option<RequestPriority>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentDirectiveResponse {
    pub agent_id: String,
    pub agent_label: String,
    pub acknowledgement: String,
    pub echoed_command: String,
    pub priority: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub required_permissions: Vec<String>,
    pub estimated_duration: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraCreateProjectRequest {
    pub name: String,
    pub git_repo_path: String,
    pub setup_script: Option<String>,
    pub dev_script: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraProjectResponse {
    pub project_id: String,
    pub name: String,
    pub git_repo_path: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraCreateBoardRequest {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub board_type: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraBoardResponse {
    pub board_id: String,
    pub project_id: String,
    pub name: String,
    pub board_type: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraCreateTaskRequest {
    pub project_id: String,
    pub board_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraTaskResponse {
    pub task_id: String,
    pub project_id: String,
    pub board_id: Option<String>,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceAnalyticsSummary {
    pub user_id: Option<String>,
    pub total_interactions: i64,
    pub total_messages: i64,
    pub average_response_time_ms: f64,
    pub first_interaction: DateTime<Utc>,
    pub last_interaction: DateTime<Utc>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub unique_sessions: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct VoiceAnalyticsRow {
    pub user_id: Option<String>,
    pub total_interactions: i64,
    pub total_messages: i64,
    pub average_response_time_ms: f64,
    pub first_interaction: DateTime<Utc>,
    pub last_interaction: DateTime<Utc>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub unique_sessions: i64,
}

impl From<VoiceAnalyticsRow> for VoiceAnalyticsSummary {
    fn from(row: VoiceAnalyticsRow) -> Self {
        Self {
            user_id: row.user_id,
            total_interactions: row.total_interactions,
            total_messages: row.total_messages,
            average_response_time_ms: row.average_response_time_ms,
            first_interaction: row.first_interaction,
            last_interaction: row.last_interaction,
            total_input_tokens: row.total_input_tokens,
            total_output_tokens: row.total_output_tokens,
            unique_sessions: row.unique_sessions,
        }
    }
}

// ── Handlers ───────────────────────────────────────────────────────────

pub async fn get_nora_status(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<NoraStatusResponse>, ApiError> {
    let nora_instance = NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let instance = nora_instance.read().await;

    if let Some(nora) = instance.as_ref() {
        let is_active = nora.is_active().await;
        let uptime_ms = NORA_INIT_TIME.get().map(|init_time| {
            Utc::now()
                .signed_duration_since(*init_time)
                .num_milliseconds() as u64
        });

        Ok(Json(NoraStatusResponse {
            is_active,
            nora_id: Some(nora.id.to_string()),
            uptime_ms,
            voice_enabled: nora.config.voice.tts.provider
                != nora::voice::config::TTSProvider::System,
            coordination_enabled: nora.config.multi_agent_coordination,
            executive_mode: nora.config.executive_mode,
            personality_mode: "British Executive Assistant".to_string(),
        }))
    } else {
        Ok(Json(NoraStatusResponse {
            is_active: false,
            nora_id: None,
            uptime_ms: None,
            voice_enabled: false,
            coordination_enabled: false,
            executive_mode: false,
            personality_mode: "Not initialized".to_string(),
        }))
    }
}

async fn sync_live_context_handler(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<ContextSyncResponse>, ApiError> {
    let manager = NoraManager::new().await;
    let refreshed = manager
        .sync_live_context()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ContextSyncResponse {
        projects_refreshed: refreshed,
    }))
}

pub async fn get_cache_stats(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<nora::cache::CacheStats>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let stats = nora
        .get_cache_stats()
        .ok_or_else(|| ApiError::InternalError("LLM cache not available".to_string()))?;

    crate::nora_metrics::update_cache_metrics(&stats);
    Ok(Json(stats))
}

pub async fn clear_cache(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    if let Some(llm) = &nora.llm {
        llm.clear_cache().await;
        Ok(Json(
            json!({ "success": true, "message": "LLM cache cleared successfully" }),
        ))
    } else {
        Err(ApiError::InternalError("LLM not available".to_string()))
    }
}

pub async fn execute_executive_tool(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Json<ToolExecutionResult>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let user_permissions = vec![
        nora::tools::Permission::ReadOnly,
        nora::tools::Permission::Write,
        nora::tools::Permission::Execute,
        nora::tools::Permission::Executive,
    ];

    let result = nora
        .executive_tools
        .execute_tool(request.tool, user_permissions)
        .await
        .map_err(|e| {
            tracing::error!("Tool execution error: {}", e);
            ApiError::InternalError(format!("Tool execution failed: {}", e))
        })?;

    Ok(Json(result))
}

pub async fn get_available_tools(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<AvailableToolsResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let tool_definitions = nora.executive_tools.get_available_tools();
    let tools: Vec<ToolInfo> = tool_definitions
        .into_iter()
        .map(|tool| ToolInfo {
            name: tool.name.clone(),
            description: tool.description.clone(),
            category: format!("{:?}", tool.category),
            required_permissions: tool
                .required_permissions
                .iter()
                .map(|p| format!("{:?}", p))
                .collect(),
            estimated_duration: tool.estimated_duration.clone(),
        })
        .collect();

    let categories: Vec<String> = tools
        .iter()
        .map(|tool| tool.category.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Ok(Json(AvailableToolsResponse { tools, categories }))
}

pub async fn get_personality_config(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<PersonalityConfig>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;
    Ok(Json(nora.config.personality.clone()))
}

pub async fn update_personality_config(
    State(_state): State<DeploymentImpl>,
    Json(config): Json<PersonalityConfig>,
) -> Result<Json<PersonalityConfig>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let mut instance = nora_instance.write().await;
    let nora = instance
        .as_mut()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;
    nora.config.personality = config.clone();
    Ok(Json(config))
}

// ── Helper functions ───────────────────────────────────────────────────

pub async fn get_nora_instance() -> Result<Arc<RwLock<Option<NoraAgent>>>, ApiError> {
    NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))
        .cloned()
}

pub(crate) fn map_projects_to_context(projects: Vec<Project>) -> Vec<ProjectContext> {
    projects
        .into_iter()
        .map(|project| ProjectContext {
            project_id: project.id.to_string(),
            name: project.name,
            description: format!("Repository: {}", project.git_repo_path.to_string_lossy()),
            status: ProjectStatus::InProgress,
            progress_percentage: 0.0,
            team_members: Vec::new(),
            budget_status: BudgetStatus {
                allocated: 0.0,
                spent: 0.0,
                remaining: 0.0,
                burn_rate: 0.0,
                forecast_completion: 0.0,
            },
            key_milestones: Vec::new(),
            risks: Vec::new(),
        })
        .collect()
}

pub(crate) fn estimate_speech_duration(text: &str) -> u64 {
    let word_count = text.split_whitespace().count();
    let minutes = word_count as f64 / 150.0;
    (minutes * 60.0 * 1000.0) as u64
}

pub(crate) fn voice_error_to_api(err: VoiceError) -> ApiError {
    ApiError::InternalError(format!("Voice error: {}", err))
}

//! Nora executive assistant API routes

pub mod chat;
pub mod coordination;
pub mod modes;
pub mod project_ops;
pub mod voice;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post, patch},
};
use chrono::{DateTime, Utc};
use db::models::agent_conversation::{AgentConversation, AgentConversationMessage};
use db::models::project::Project;
use sqlx;
use deployment::Deployment;
use cinematics::{CinematicsConfig, CinematicsService};
use nora::{
    NoraAgent, NoraConfig, NoraError,
    agent::{NoraRequest, NoraRequestType, NoraResponse, RapidPlaybookRequest, RapidPlaybookResult, RequestPriority},
    brain::{LLMConfig, infer_provider_from_model},
    LLMProvider,
    coordination::{AgentCoordinationState, CoordinationEvent, CoordinationStats},
    graph::{GraphNodeStatus, GraphPlan, GraphPlanSummary},
    memory::{BudgetStatus, ProjectContext, ProjectStatus},
    personality::PersonalityConfig,
    tools::{NoraExecutiveTool, ToolExecutionResult},
    voice::{SpeechResponse, TTSConfig, VoiceConfig, VoiceEngine, VoiceError, VoiceInteraction},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast, RwLock};
use ts_rs::TS;
use uuid::Uuid;

use db::models::vibe_deposit::{VibeDeposit, VibeWithdrawal};
use db::models::vibe_transaction::{VibeSourceType, VibeTransaction};
use services::services::vibe_pricing::VibePricingService;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext, middleware::rate_limit::TokenBucket};

// Re-export items used by external modules
pub use self::coordination::emit_coordination_event;

/// Global Nora agent instance
pub(crate) static NORA_INSTANCE: tokio::sync::OnceCell<Arc<RwLock<Option<NoraAgent>>>> =
    tokio::sync::OnceCell::const_new();

/// Global Nora initialization timestamp
pub(crate) static NORA_INIT_TIME: tokio::sync::OnceCell<DateTime<Utc>> = tokio::sync::OnceCell::const_new();

/// Global rate limiter for chat endpoints (20 req/min, refill 1 per 3 seconds)
pub(crate) static CHAT_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

/// Global rate limiter for voice synthesis (30 req/min, refill 1 per 2 seconds)
pub(crate) static VOICE_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

#[derive(Clone)]
pub(crate) struct NoraModePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub config: NoraConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoraModeSummary {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub(crate) static NORA_MODE_PRESETS: Lazy<Vec<NoraModePreset>> = Lazy::new(|| {
    let default_cfg = NoraConfig::default();
    let mut rapid_cfg = NoraConfig::default();
    rapid_cfg.voice = VoiceConfig::development();
    rapid_cfg.personality = PersonalityConfig::casual_british();

    let mut boardroom_cfg = NoraConfig::default();
    boardroom_cfg.voice = VoiceConfig::british_executive();
    boardroom_cfg.personality = PersonalityConfig::british_executive_assistant();

    vec![
        NoraModePreset {
            id: "rapid-builder",
            label: "Rapid Builder",
            description: "Fast prototyping mode with casual tone and lightweight voice stack",
            config: rapid_cfg,
        },
        NoraModePreset {
            id: "boardroom",
            label: "Boardroom",
            description: "High-formality executive briefing mode",
            config: boardroom_cfg,
        },
        NoraModePreset {
            id: "standard",
            label: "Standard",
            description: "Balanced configuration used by default",
            config: default_cfg,
        },
    ]
});

/// Nora manager for coordinating agent instances
#[derive(Clone)]
pub struct NoraManager {
    agent: Arc<RwLock<Option<NoraAgent>>>,
}

impl NoraManager {
    /// Create a new NoraManager
    pub async fn new() -> Self {
        let agent = NORA_INSTANCE
            .get_or_init(|| async { Arc::new(RwLock::new(None)) })
            .await
            .clone();

        Self { agent }
    }

    /// Process a request with Nora
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

    /// Get coordination statistics
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

    /// Get all agents
    pub async fn get_all_agents(
        &self,
    ) -> Result<Vec<nora::coordination::AgentCoordinationState>, NoraError> {
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

    /// Initialize Nora with config
    pub async fn initialize(&self, config: NoraConfig) -> Result<String, NoraError> {
        let mut agent = self.agent.write().await;
        let nora = NoraAgent::new(config).await?;
        let id = nora.id.to_string();

        // Record initialization time
        let _ = NORA_INIT_TIME.set(Utc::now());

        *agent = Some(nora);
        Ok(id)
    }

    /// Check if Nora is active
    pub async fn is_active(&self) -> bool {
        let agent = self.agent.read().await;
        agent.is_some()
    }

    /// Get uptime in milliseconds
    pub async fn get_uptime_ms(&self) -> Option<u64> {
        if let Some(init_time) = NORA_INIT_TIME.get() {
            let now = Utc::now();
            let duration = now.signed_duration_since(*init_time);
            Some(duration.num_milliseconds() as u64)
        } else {
            None
        }
    }

    /// Sync Nora's context with live project data
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

    /// Run rapid prototyping playbook
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

    /// Reinitialize Nora with a new config (optionally preserving memory/context)
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
            nora
                .graph_plan_detail(plan_id)
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
            nora.update_graph_node_status(plan_id, node_id, status).await
        } else {
            Err(NoraError::NotInitialized(
                "Nora agent not initialized".to_string(),
            ))
        }
    }
}

/// Get or initialize chat rate limiter
pub(crate) async fn get_chat_rate_limiter() -> &'static Arc<TokenBucket> {
    CHAT_RATE_LIMITER
        .get_or_init(|| async {
            // 20 tokens max, refill at 1 token per 3 seconds (20/min)
            Arc::new(TokenBucket::new(20.0, 1.0 / 3.0))
        })
        .await
}

/// Get or initialize voice rate limiter
pub(crate) async fn get_voice_rate_limiter() -> &'static Arc<TokenBucket> {
    VOICE_RATE_LIMITER
        .get_or_init(|| async {
            // 30 tokens max, refill at 1 token per 2 seconds (30/min)
            Arc::new(TokenBucket::new(30.0, 0.5))
        })
        .await
}

/// Initialize Nora routes
pub fn nora_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/nora/initialize", post(initialize_nora))
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
        .route("/nora/voice/analytics/users/{user_id}", get(voice::get_user_voice_analytics))
        .route("/nora/voice/analytics/users", get(voice::get_all_users_voice_analytics))
        .route("/nora/voice/conversations/{session_id}", get(voice::get_session_conversation))
        .route("/nora/tools/execute", post(execute_executive_tool))
        .route("/nora/tools/available", get(get_available_tools))
        .route("/nora/context/sync", post(sync_live_context_handler))
        .route("/nora/modes", get(modes::list_modes_handler))
        .route("/nora/modes/apply", post(modes::apply_mode_handler))
        .route("/nora/playbooks/rapid", post(modes::rapid_playbook_handler))
        .route("/nora/graph/plans", get(modes::list_graph_plans_handler))
        .route("/nora/graph/plans/{plan_id}", get(modes::get_graph_plan_handler))
        .route(
            "/nora/graph/plans/{plan_id}/nodes/{node_id}",
            patch(modes::update_graph_node_handler),
        )
        .route("/nora/coordination/stats", get(coordination::get_coordination_stats))
        .route("/nora/coordination/agents", get(coordination::get_coordination_agents))
        .route("/nora/coordination/events", get(coordination::get_coordination_events_ws))
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
        .route("/nora/project/create", post(project_ops::nora_create_project))
        .route("/nora/board/create", post(project_ops::nora_create_board))
        .route("/nora/task/create", post(project_ops::nora_create_task))
        .layer(axum::middleware::from_fn(
            crate::middleware::request_id_middleware,
        ))
}

// ── Shared request/response types ──────────────────────────────────────

/// Request to initialize Nora
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeNoraRequest {
    pub config: Option<NoraConfig>,
    pub activate_immediately: bool,
}

/// Nora initialization response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeNoraResponse {
    pub success: bool,
    pub nora_id: String,
    pub message: String,
    pub capabilities: Vec<String>,
}

/// Nora status response
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

/// Chat request to Nora
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
    /// Project to bill VIBE usage against
    pub project_id: Option<Uuid>,
}

/// Voice synthesis request
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

/// Voice transcription request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionRequest {
    #[serde(alias = "audio")]
    pub audio_data: String, // Base64 encoded
    pub language: Option<String>,
    pub british_dialect: Option<bool>,
}

/// Voice transcription response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionResponse {
    pub text: String,
}

/// Executive tool execution request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteToolRequest {
    pub tool: NoraExecutiveTool,
    pub session_id: String,
    pub user_permissions: Vec<String>, // Will be converted to Permission enum
}

/// Voice configuration response wrapper
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceConfigResponse {
    pub config: VoiceConfig,
}

/// Voice configuration update request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVoiceConfigRequest {
    pub config: VoiceConfig,
}

/// Available tools response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AvailableToolsResponse {
    pub tools: Vec<ToolInfo>,
    pub categories: Vec<String>,
}

/// Directives sent to specific agents from the global console
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentDirectiveRequest {
    pub session_id: String,
    pub content: String,
    pub command: Option<String>,
    pub priority: Option<RequestPriority>,
    pub context: Option<serde_json::Value>,
}

/// Acknowledgement payload returned when an agent accepts a directive
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

/// Tool information
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub required_permissions: Vec<String>,
    pub estimated_duration: Option<String>,
}

/// Nora create project request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraCreateProjectRequest {
    pub name: String,
    pub git_repo_path: String,
    pub setup_script: Option<String>,
    pub dev_script: Option<String>,
}

/// Nora create project response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraProjectResponse {
    pub project_id: String,
    pub name: String,
    pub git_repo_path: String,
    pub created_at: String,
}

/// Nora create board request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraCreateBoardRequest {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub board_type: Option<String>, // "kanban" or "scrum"
}

/// Nora create board response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraBoardResponse {
    pub board_id: String,
    pub project_id: String,
    pub name: String,
    pub board_type: String,
    pub created_at: String,
}

/// Nora create task request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraCreateTaskRequest {
    pub project_id: String,
    pub board_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>, // "low", "medium", or "high"
    pub tags: Option<Vec<String>>,
}

/// Nora create task response
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

/// Voice analytics summary for a user
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

/// Database row for voice analytics queries (runtime-checked)
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

// ── Handlers that live in mod.rs ───────────────────────────────────────

/// Initialize Nora executive assistant
pub async fn initialize_nora(
    State(state): State<DeploymentImpl>,
    Json(request): Json<InitializeNoraRequest>,
) -> Result<Json<InitializeNoraResponse>, ApiError> {
    tracing::info!("Initializing Nora executive assistant");

    let nora_instance = NORA_INSTANCE
        .get_or_init(|| async { Arc::new(RwLock::new(None)) })
        .await;

    // If Nora is already initialized and activation is not forced, return current status
    if !request.activate_immediately {
        let instance = nora_instance.read().await;
        if let Some(existing) = instance.as_ref() {
            return Ok(Json(InitializeNoraResponse {
                success: true,
                nora_id: existing.id.to_string(),
                message: "Nora is already active and ready to assist.".to_string(),
                capabilities: default_capabilities(),
            }));
        }
    }

    let mut config = request.config.unwrap_or_default();
    apply_llm_overrides(&mut config);

    // Load persisted voice configuration if available
    if let Ok(Some(persisted_config)) =
        db::models::nora_config::NoraVoiceConfig::get(&state.db().pool).await
    {
        match serde_json::from_str::<VoiceConfig>(&persisted_config.config_json) {
            Ok(voice_config) => {
                tracing::info!("Loaded persisted voice configuration from database");
                config.voice = voice_config;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse persisted voice config, using default: {}",
                    e
                );
            }
        }
    } else {
        tracing::info!("No persisted voice configuration found, using default");
    }

    let projects = Project::find_all(&state.db().pool).await.map_err(|e| {
        tracing::error!("Failed to load projects for Nora context: {}", e);
        ApiError::InternalError(format!("Failed to load projects: {}", e))
    })?;

    let project_context = map_projects_to_context(projects);

    let nora_agent = nora::initialize_nora(config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to initialize Nora: {}", e);
            ApiError::InternalError(format!("Nora initialization failed: {}", e))
        })?
        .with_media_pipeline(state.media_pipeline().clone())
        .with_database(state.db().pool.clone());

    nora_agent
        .seed_projects(project_context)
        .await
        .map_err(|e| {
            tracing::error!("Failed to seed Nora context: {}", e);
            ApiError::InternalError(format!("Failed to seed Nora context: {}", e))
        })?;

    let nora_id = nora_agent.id.to_string();

    if request.activate_immediately {
        nora_agent
            .set_active(true)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to activate Nora: {}", e)))?;
        crate::nora_metrics::set_nora_active(true);
    }

    {
        let mut instance = nora_instance.write().await;
        *instance = Some(nora_agent);
    }

    tracing::info!("Nora initialized successfully with ID: {}", nora_id);

    Ok(Json(InitializeNoraResponse {
        success: true,
        nora_id,
        message:
            "Good day! I'm Nora, your executive assistant. I'm delighted to be at your service."
                .to_string(),
        capabilities: default_capabilities(),
    }))
}

/// Initialize Nora on server startup (called automatically)
/// This is a non-HTTP helper for auto-initialization
pub async fn initialize_nora_on_startup(state: &DeploymentImpl) -> Result<String, String> {
    tracing::info!("Auto-initializing Nora executive assistant on server startup");

    let nora_instance = NORA_INSTANCE
        .get_or_init(|| async { Arc::new(RwLock::new(None)) })
        .await;

    // Check if already initialized
    {
        let instance = nora_instance.read().await;
        if instance.is_some() {
            tracing::info!("Nora already initialized, skipping auto-initialization");
            return Ok("Already initialized".to_string());
        }
    }

    // Create default config with environment overrides
    let mut config = NoraConfig::default();
    apply_llm_overrides(&mut config);

    // Auto-detect ElevenLabs before DB load (DB config wins if it exists)
    config.voice.tts = TTSConfig::auto_detect();

    // Load persisted voice configuration if available (overrides auto-detect)
    if let Ok(Some(persisted_config)) =
        db::models::nora_config::NoraVoiceConfig::get(&state.db().pool).await
    {
        match serde_json::from_str::<VoiceConfig>(&persisted_config.config_json) {
            Ok(voice_config) => {
                tracing::info!("Loaded persisted voice configuration from database");
                config.voice = voice_config;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse persisted voice config, using default: {}",
                    e
                );
            }
        }
    }

    // Load projects for context
    let projects = Project::find_all(&state.db().pool)
        .await
        .map_err(|e| format!("Failed to load projects: {}", e))?;

    let project_context = map_projects_to_context(projects);

    // Initialize Cinematics service for Master Cinematographer agent
    let cinematics_config = CinematicsConfig::default();
    tracing::info!(
        "Initializing CinematicsService with ComfyUI at {}",
        cinematics_config.comfy_base_url
    );
    let cinematics = Arc::new(CinematicsService::new(
        state.db().pool.clone(),
        cinematics_config,
    ));

    // Initialize Nora agent
    let nora_agent = nora::initialize_nora(config)
        .await
        .map_err(|e| format!("Nora initialization failed: {}", e))?
        .with_media_pipeline(state.media_pipeline().clone())
        .with_database(state.db().pool.clone())
        .with_cinematics(cinematics);

    // Seed project context
    nora_agent
        .seed_projects(project_context)
        .await
        .map_err(|e| format!("Failed to seed Nora context: {}", e))?;

    let nora_id = nora_agent.id.to_string();

    // Activate Nora by default on startup
    nora_agent
        .set_active(true)
        .await
        .map_err(|e| format!("Failed to activate Nora: {}", e))?;
    crate::nora_metrics::set_nora_active(true);

    // Store in global instance
    {
        let mut instance = nora_instance.write().await;
        *instance = Some(nora_agent);
    }

    // Record initialization time
    let _ = NORA_INIT_TIME.set(Utc::now());

    tracing::info!("Nora auto-initialized successfully with ID: {}", nora_id);
    Ok(nora_id)
}

/// Get Nora status
pub async fn get_nora_status(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<NoraStatusResponse>, ApiError> {
    let nora_instance = NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let instance = nora_instance.read().await;

    if let Some(nora) = instance.as_ref() {
        let is_active = nora.is_active().await;

        // Calculate uptime
        let uptime_ms = if let Some(init_time) = NORA_INIT_TIME.get() {
            let now = Utc::now();
            let duration = now.signed_duration_since(*init_time);
            Some(duration.num_milliseconds() as u64)
        } else {
            None
        };

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

/// Get cache statistics
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

    // Update Prometheus metrics with current cache stats
    crate::nora_metrics::update_cache_metrics(&stats);

    tracing::debug!(
        "Cache stats - Hits: {}, Misses: {}, Hit Rate: {:.2}%",
        stats.hits,
        stats.misses,
        stats.hit_rate * 100.0
    );

    Ok(Json(stats))
}

/// Clear the LLM cache
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
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "LLM cache cleared successfully"
        })))
    } else {
        Err(ApiError::InternalError("LLM not available".to_string()))
    }
}

/// Execute executive tool
pub async fn execute_executive_tool(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Json<ToolExecutionResult>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    // Convert string permissions to Permission enum
    // Grant all permissions for local development — proper RBAC mapping will be added later
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

/// Get available executive tools
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

/// Get personality configuration
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

/// Update personality configuration
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
    // TODO: Apply personality changes to the personality module

    Ok(Json(config))
}

// ── Helper functions ───────────────────────────────────────────────────

/// Get the global NORA instance
pub async fn get_nora_instance() -> Result<Arc<RwLock<Option<NoraAgent>>>, ApiError> {
    NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))
        .map(|instance| instance.clone())
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

/// NOTE: Billing logic (resolve_billing_project, check_vibe_balance) currently lives
/// inline in the chat handlers. Consider extracting to a shared billing module later.
pub(crate) fn apply_llm_overrides(config: &mut NoraConfig) {
    // If OPENAI_API_KEY is set, ensure we have an LLM config
    // This allows the LLM to work without requiring NORA_LLM_* env vars
    if std::env::var("OPENAI_API_KEY").is_ok() && config.llm.is_none() {
        config.llm = Some(LLMConfig::default());
        tracing::info!("LLM enabled via OPENAI_API_KEY");
    }

    if let Ok(model) = std::env::var("NORA_LLM_MODEL") {
        let llm = config.llm.get_or_insert_with(LLMConfig::default);
        llm.model = model.clone();
        // Auto-infer provider from model name
        llm.provider = infer_provider_from_model(&model);
        tracing::info!("Nora LLM model set to: {} (provider: {:?})", model, llm.provider);
    }

    // Explicit provider override (takes precedence over inference)
    if let Ok(provider) = std::env::var("NORA_LLM_PROVIDER") {
        let llm = config.llm.get_or_insert_with(LLMConfig::default);
        llm.provider = match provider.to_lowercase().as_str() {
            "anthropic" | "claude" => LLMProvider::Anthropic,
            "openai" | "gpt" => LLMProvider::OpenAI,
            _ => LLMProvider::Ollama,
        };
    }

    if let Ok(endpoint) = std::env::var("NORA_LLM_ENDPOINT") {
        config.llm.get_or_insert_with(LLMConfig::default).endpoint = Some(endpoint);
    }

    if let Ok(temp) = std::env::var("NORA_LLM_TEMPERATURE") {
        if let Ok(value) = temp.parse::<f32>() {
            config
                .llm
                .get_or_insert_with(LLMConfig::default)
                .temperature = value;
        }
    }

    if let Ok(max_tokens) = std::env::var("NORA_LLM_MAX_TOKENS") {
        if let Ok(value) = max_tokens.parse::<u32>() {
            config.llm.get_or_insert_with(LLMConfig::default).max_tokens = value;
        }
    }

    if let Ok(prompt) = std::env::var("NORA_LLM_SYSTEM_PROMPT") {
        config
            .llm
            .get_or_insert_with(LLMConfig::default)
            .system_prompt = prompt;
    }
}

pub(crate) fn estimate_speech_duration(text: &str) -> u64 {
    // Rough estimation: average speaking rate is about 150 words per minute
    let word_count = text.split_whitespace().count();
    let minutes = word_count as f64 / 150.0;
    (minutes * 60.0 * 1000.0) as u64 // Convert to milliseconds
}

pub(crate) fn voice_error_to_api(err: VoiceError) -> ApiError {
    ApiError::InternalError(format!("Voice error: {}", err))
}

pub(crate) fn default_capabilities() -> Vec<String> {
    vec![
        "Voice Interaction".to_string(),
        "Task Coordination".to_string(),
        "Strategic Planning".to_string(),
        "Performance Analysis".to_string(),
        "Decision Support".to_string(),
        "Communication Management".to_string(),
    ]
}

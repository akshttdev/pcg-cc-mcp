//! Nora executive assistant API routes

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade},
    response::{
        Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post, patch},
};
use chrono::{DateTime, Utc};
use db::models::agent_conversation::{AgentConversation, AgentConversationMessage};
use db::models::project::Project;
use deployment::Deployment;
use futures::stream::Stream;
use cinematics::{CinematicsConfig, CinematicsService};
use nora::{
    NoraAgent, NoraConfig, NoraError,
    agent::{NoraRequest, NoraRequestType, NoraResponse, RapidPlaybookRequest, RapidPlaybookResult, RequestPriority},
    brain::LLMConfig,
    coordination::{AgentCoordinationState, CoordinationEvent, CoordinationStats},
    graph::{GraphNodeStatus, GraphPlan, GraphPlanSummary},
    memory::{BudgetStatus, ProjectContext, ProjectStatus},
    personality::PersonalityConfig,
    tools::{NoraExecutiveTool, ToolExecutionResult},
    voice::{SpeechResponse, VoiceConfig, VoiceEngine, VoiceError, VoiceInteraction},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast, RwLock};
use ts_rs::TS;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::rate_limit::TokenBucket};

/// Global Nora agent instance
static NORA_INSTANCE: tokio::sync::OnceCell<Arc<RwLock<Option<NoraAgent>>>> =
    tokio::sync::OnceCell::const_new();

/// Global Nora initialization timestamp
static NORA_INIT_TIME: tokio::sync::OnceCell<DateTime<Utc>> = tokio::sync::OnceCell::const_new();

/// Global rate limiter for chat endpoints (20 req/min, refill 1 per 3 seconds)
static CHAT_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

/// Global rate limiter for voice synthesis (30 req/min, refill 1 per 2 seconds)
static VOICE_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

#[derive(Clone)]
struct NoraModePreset {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    config: NoraConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NoraModeSummary {
    id: &'static str,
    label: &'static str,
    description: &'static str,
}

static NORA_MODE_PRESETS: Lazy<Vec<NoraModePreset>> = Lazy::new(|| {
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
async fn get_chat_rate_limiter() -> &'static Arc<TokenBucket> {
    CHAT_RATE_LIMITER
        .get_or_init(|| async {
            // 20 tokens max, refill at 1 token per 3 seconds (20/min)
            Arc::new(TokenBucket::new(20.0, 1.0 / 3.0))
        })
        .await
}

/// Get or initialize voice rate limiter
async fn get_voice_rate_limiter() -> &'static Arc<TokenBucket> {
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
        .route("/nora/chat", post(chat_with_nora))
        .route("/nora/chat/stream", post(chat_with_nora_stream))
        .route("/nora/cache/stats", get(get_cache_stats))
        .route("/nora/cache/clear", post(clear_cache))
        .route(
            "/nora/voice/config",
            get(get_voice_config).put(update_voice_config),
        )
        .route("/nora/voice/synthesize", post(synthesize_speech))
        .route("/nora/voice/transcribe", post(transcribe_speech))
        .route("/nora/voice/interaction", post(voice_interaction))
        .route("/nora/voice/analytics/users/{user_id}", get(get_user_voice_analytics))
        .route("/nora/voice/analytics/users", get(get_all_users_voice_analytics))
        .route("/nora/voice/conversations/{session_id}", get(get_session_conversation))
        .route("/nora/tools/execute", post(execute_executive_tool))
        .route("/nora/tools/available", get(get_available_tools))
        .route("/nora/context/sync", post(sync_live_context_handler))
        .route("/nora/modes", get(list_modes_handler))
        .route("/nora/modes/apply", post(apply_mode_handler))
        .route("/nora/playbooks/rapid", post(rapid_playbook_handler))
        .route("/nora/graph/plans", get(list_graph_plans_handler))
        .route("/nora/graph/plans/{plan_id}", get(get_graph_plan_handler))
        .route(
            "/nora/graph/plans/{plan_id}/nodes/{node_id}",
            patch(update_graph_node_handler),
        )
        .route("/nora/coordination/stats", get(get_coordination_stats))
        .route("/nora/coordination/agents", get(get_coordination_agents))
        .route("/nora/coordination/events", get(get_coordination_events_ws))
        .route(
            "/nora/coordination/events/sse",
            get(get_coordination_events_sse),
        )
        .route(
            "/nora/coordination/agents/{agent_id}/directives",
            post(send_agent_directive),
        )
        .route("/nora/personality/config", get(get_personality_config))
        .route("/nora/personality/config", post(update_personality_config))
        .route("/nora/project/create", post(nora_create_project))
        .route("/nora/board/create", post(nora_create_board))
        .route("/nora/task/create", post(nora_create_task))
        .layer(axum::middleware::from_fn(
            crate::middleware::request_id_middleware,
        ))
}

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
    pub voice_enabled: bool,
    pub priority: Option<RequestPriority>,
    pub context: Option<serde_json::Value>,
    pub stream: Option<bool>,
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
        .with_database(state.db().pool.clone())
        .with_media_pipeline(state.media_pipeline().clone());

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
        .with_database(state.db().pool.clone())
        .with_media_pipeline(state.media_pipeline().clone())
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

async fn list_modes_handler() -> Json<Vec<NoraModeSummary>> {
    let summaries = NORA_MODE_PRESETS
        .iter()
        .map(|preset| NoraModeSummary {
            id: preset.id,
            label: preset.label,
            description: preset.description,
        })
        .collect();
    Json(summaries)
}

async fn apply_mode_handler(
    Json(body): Json<ApplyModeRequest>,
) -> Result<Json<ApplyModeResponse>, ApiError> {
    let manager = NoraManager::new().await;
    let preset = NORA_MODE_PRESETS
        .iter()
        .find(|preset| preset.id == body.mode_id)
        .ok_or_else(|| ApiError::BadRequest("Unknown Nora mode".to_string()))?;

    let nora_id = manager
        .reinitialize_with_config(preset.config.clone(), body.preserve_memory)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApplyModeResponse {
        active_mode: preset.label.to_string(),
        nora_id,
    }))
}

async fn rapid_playbook_handler(
    Json(body): Json<RapidPlaybookBody>,
) -> Result<Json<RapidPlaybookResult>, ApiError> {
    let manager = NoraManager::new().await;
    let payload = RapidPlaybookRequest {
        project_name: body.project_name,
        objectives: body.objectives,
        repo_hint: body.repo_hint,
        notes: body.notes,
    };

    let result = manager
        .run_rapid_playbook(payload)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(result))
}

async fn list_graph_plans_handler(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<Vec<GraphPlanSummary>>, ApiError> {
    let manager = NoraManager::new().await;
    let plans = manager
        .list_graph_plans()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(plans))
}

async fn get_graph_plan_handler(
    State(_state): State<DeploymentImpl>,
    Path(plan_id): Path<String>,
) -> Result<Json<GraphPlan>, ApiError> {
    let manager = NoraManager::new().await;
    let plan = manager
        .get_graph_plan(&plan_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(plan))
}

async fn update_graph_node_handler(
    State(_state): State<DeploymentImpl>,
    Path((plan_id, node_id)): Path<(String, String)>,
    Json(body): Json<UpdateNodeStatusBody>,
) -> Result<Json<GraphPlan>, ApiError> {
    let manager = NoraManager::new().await;
    let plan = manager
        .update_graph_node_status(&plan_id, &node_id, body.status.clone())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(node) = plan.nodes.iter().find(|node| node.id == node_id) {
        emit_coordination_event(CoordinationEvent::AgentDirectiveIssued {
            agent_id: node
                .agent
                .clone()
                .unwrap_or_else(|| "NORA_GRAPH".to_string()),
            issued_by: "NORA_GRAPH".to_string(),
            content: format!(
                "Node '{}' advanced to {:?}",
                node.label, body.status
            ),
            priority: Some(format!("{:?}", body.status)),
            timestamp: Utc::now(),
        })
        .await;
    }

    Ok(Json(plan))
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

/// Chat with Nora
pub async fn chat_with_nora(
    State(_state): State<DeploymentImpl>,
    request_id: Option<axum::extract::Extension<crate::middleware::RequestId>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<NoraResponse>, ApiError> {
    let _timer = crate::nora_metrics::start_request_timer("chat");

    // Apply rate limiting
    let rate_limiter = get_chat_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Chat rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your chat requests.".to_string(),
        ));
    }

    tracing::info!("Received chat request: {:?}", request.message);

    // Get request ID from middleware or generate new one
    let req_id = request_id
        .map(|ext| ext.0.as_str().to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    tracing::info!("Nora instance found, checking if active...");
    if !nora.is_active().await {
        tracing::warn!("Nora is not active");
        crate::nora_metrics::record_request("chat", "normal");
        return Err(ApiError::BadRequest("Nora is not active".to_string()));
    }

    tracing::info!("Nora is active, creating request...");
    let nora_request = NoraRequest {
        request_id: req_id,
        session_id: request.session_id,
        request_type: request
            .request_type
            .unwrap_or(NoraRequestType::TextInteraction),
        content: request.message.clone(),
        context: request.context,
        voice_enabled: request.voice_enabled,
        priority: request.priority.unwrap_or(RequestPriority::Normal),
        timestamp: chrono::Utc::now(),
    };

    let priority_str = match nora_request.priority {
        RequestPriority::Low => "low",
        RequestPriority::Normal => "normal",
        RequestPriority::High => "high",
        RequestPriority::Urgent => "urgent",
        RequestPriority::Executive => "executive",
    };

    tracing::info!("Processing request with content: {}", request.message);
    let response = nora.process_request(nora_request).await.map_err(|e| {
        tracing::error!("Nora processing error: {}", e);
        crate::nora_metrics::record_request("chat", priority_str);
        ApiError::InternalError(format!("Nora processing failed: {}", e))
    })?;

    crate::nora_metrics::record_request("chat", priority_str);
    tracing::info!("Request processed successfully");
    Ok(Json(response))
}

/// Chat with Nora using streaming (SSE)
pub async fn chat_with_nora_stream(
    State(_state): State<DeploymentImpl>,
    request_id: Option<axum::extract::Extension<crate::middleware::RequestId>>,
    Json(request): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError> {
    use futures::stream::StreamExt;

    let _timer = crate::nora_metrics::start_request_timer("chat_stream");

    // Apply rate limiting
    let rate_limiter = get_chat_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Chat stream rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your chat requests.".to_string(),
        ));
    }

    tracing::info!("Received streaming chat request: {:?}", request.message);

    // Get request ID from middleware or generate new one
    let _req_id = request_id
        .map(|ext| ext.0.as_str().to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    if !nora.is_active().await {
        tracing::warn!("Nora is not active");
        return Err(ApiError::BadRequest("Nora is not active".to_string()));
    }

    // Get the LLM client directly
    let llm_client = nora
        .llm
        .clone()
        .ok_or_else(|| ApiError::InternalError("LLM not available".to_string()))?;

    // Prepare context
    let context = request
        .context
        .map(|c| serde_json::to_string_pretty(&c).unwrap_or_default())
        .unwrap_or_default();

    let message = request.message.clone();

    // Create stream
    let llm_stream = llm_client
        .generate_stream("", &message, &context)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create stream: {}", e)))?;

    // Convert to SSE stream
    let sse_stream = llm_stream.map(|chunk_result| match chunk_result {
        Ok(chunk) => {
            tracing::debug!("Streaming chunk: {} chars", chunk.len());
            Ok(Event::default().data(chunk))
        }
        Err(e) => {
            tracing::error!("Stream error: {}", e);
            Ok(Event::default().data(format!("[ERROR]: {}", e)))
        }
    });

    crate::nora_metrics::record_request("chat_stream", "normal");

    Ok(Sse::new(sse_stream))
}

/// Synthesize speech using Nora's voice
pub async fn synthesize_speech(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<VoiceSynthesisRequest>,
) -> Result<Json<SpeechResponse>, ApiError> {
    // Apply rate limiting
    let rate_limiter = get_voice_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Voice synthesis rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your voice synthesis requests.".to_string(),
        ));
    }

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let start = std::time::Instant::now();
    let audio_data = nora
        .voice_engine
        .synthesize_speech(&request.text)
        .await
        .map_err(|e| {
            tracing::error!("Speech synthesis error: {}", e);
            ApiError::InternalError(format!("Speech synthesis failed: {}", e))
        })?;

    let duration = start.elapsed().as_secs_f64();
    crate::nora_metrics::record_tts_call("openai", "success", duration);

    // Create a proper SpeechResponse
    let processing_time_ms = (duration * 1000.0) as u64;
    let response = SpeechResponse {
        audio_data,
        duration_ms: estimate_speech_duration(&request.text),
        sample_rate: 22050, // Default for most TTS services
        format: nora::voice::AudioFormat::Mp3,
        processing_time_ms,
    };

    Ok(Json(response))
}

/// Transcribe speech using Nora's STT
pub async fn transcribe_speech(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<VoiceTranscriptionRequest>,
) -> Result<Json<VoiceTranscriptionResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let start = std::time::Instant::now();
    let transcription = nora
        .voice_engine
        .transcribe_speech(&request.audio_data)
        .await
        .map_err(|e| {
            tracing::error!("Speech transcription error: {}", e);
            ApiError::InternalError(format!("Speech transcription failed: {}", e))
        })?;

    let duration = start.elapsed().as_secs_f64();
    crate::nora_metrics::record_stt_call("openai", "success", duration);

    Ok(Json(VoiceTranscriptionResponse {
        text: transcription,
    }))
}

/// Handle voice interaction
pub async fn voice_interaction(
    State(state): State<DeploymentImpl>,
    Json(request): Json<VoiceInteraction>,
) -> Result<Json<VoiceInteraction>, ApiError> {
    // Apply rate limiting
    let rate_limiter = get_voice_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Voice interaction rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your voice interaction requests.".to_string(),
        ));
    }

    let start = std::time::Instant::now();
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    // Process the voice interaction
    let mut processed_interaction = request;

    // If there's audio input, transcribe it
    if let Some(audio_data) = &processed_interaction.audio_input {
        let transcription = nora
            .voice_engine
            .transcribe_speech(audio_data)
            .await
            .map_err(|e| ApiError::InternalError(format!("Transcription failed: {}", e)))?;
        processed_interaction.transcription = Some(transcription.clone());

        // Process the transcribed text with Nora
        let nora_request = NoraRequest {
            request_id: Uuid::new_v4().to_string(),
            session_id: processed_interaction.session_id.clone(),
            request_type: NoraRequestType::VoiceInteraction,
            content: transcription,
            context: None,
            voice_enabled: true,
            priority: RequestPriority::Normal,
            timestamp: chrono::Utc::now(),
        };

        let response = nora
            .process_request(nora_request)
            .await
            .map_err(|e| ApiError::InternalError(format!("Nora processing failed: {}", e)))?;

        processed_interaction.response_text = response.content;

        // Generate voice response if requested
        if let Some(voice_response) = response.voice_response {
            processed_interaction.audio_response = Some(voice_response);
        }
    }

    let processing_time_ms = start.elapsed().as_millis() as u64;
    processed_interaction.processing_time_ms = processing_time_ms;
    processed_interaction.timestamp = chrono::Utc::now();

    // Persist conversation to database
    let pool = &state.db().pool;
    let conversation_result = AgentConversation::get_or_create(
        pool,
        nora.id,
        &processed_interaction.session_id,
        None, // project_id
    )
    .await;

    if let Ok(conversation) = conversation_result {
        // Update conversation with user_id if provided
        if let Some(user_id) = &processed_interaction.user_id {
            if conversation.user_id.is_none() {
                let _ = sqlx::query("UPDATE agent_conversations SET user_id = ? WHERE id = ?")
                    .bind(user_id)
                    .bind(conversation.id)
                    .execute(pool)
                    .await;
            }
        }

        // Save user message (transcription) and assistant response
        if let Some(transcription) = &processed_interaction.transcription {
            let _ = AgentConversationMessage::add_user_message(
                pool,
                conversation.id,
                transcription,
            )
            .await;

            let _ = AgentConversationMessage::add_assistant_message(
                pool,
                conversation.id,
                &processed_interaction.response_text,
                None, // model
                None, // provider
                None, // input_tokens
                None, // output_tokens
                Some(processing_time_ms as i64),
            )
            .await;
        }
    } else {
        tracing::warn!("Failed to persist voice conversation to database: {:?}", conversation_result.err());
    }

    Ok(Json(processed_interaction))
}

/// Get voice analytics for a specific user
pub async fn get_user_voice_analytics(
    Path(user_id): Path<String>,
    State(state): State<DeploymentImpl>,
) -> Result<Json<VoiceAnalyticsSummary>, ApiError> {
    let pool = &state.db().pool;
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    // Query analytics for the specific user
    let result = sqlx::query!(
        r#"
        SELECT
            user_id as "user_id: Option<String>",
            COUNT(DISTINCT id) as "total_interactions!: i64",
            SUM(message_count) as "total_messages!: i64",
            AVG(julianday(last_message_at) - julianday(created_at)) * 86400000 as "average_response_time_ms!: f64",
            MIN(created_at) as "first_interaction!: DateTime<Utc>",
            MAX(updated_at) as "last_interaction!: DateTime<Utc>",
            SUM(COALESCE(total_input_tokens, 0)) as "total_input_tokens!: i64",
            SUM(COALESCE(total_output_tokens, 0)) as "total_output_tokens!: i64",
            COUNT(DISTINCT session_id) as "unique_sessions!: i64"
        FROM agent_conversations
        WHERE agent_id = ? AND user_id = ? AND status = 'active'
        "#,
        nora.id,
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch user voice analytics: {}", e);
        ApiError::InternalError(format!("Failed to fetch analytics: {}", e))
    })?;

    Ok(Json(VoiceAnalyticsSummary {
        user_id: result.user_id.flatten(),
        total_interactions: result.total_interactions,
        total_messages: result.total_messages,
        average_response_time_ms: result.average_response_time_ms,
        first_interaction: result.first_interaction,
        last_interaction: result.last_interaction,
        total_input_tokens: result.total_input_tokens,
        total_output_tokens: result.total_output_tokens,
        unique_sessions: result.unique_sessions,
    }))
}

/// Get voice analytics for all users
pub async fn get_all_users_voice_analytics(
    State(state): State<DeploymentImpl>,
) -> Result<Json<Vec<VoiceAnalyticsSummary>>, ApiError> {
    let pool = &state.db().pool;
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    // Query analytics grouped by user
    let results = sqlx::query!(
        r#"
        SELECT
            user_id as "user_id: Option<String>",
            COUNT(DISTINCT id) as "total_interactions!: i64",
            SUM(message_count) as "total_messages!: i64",
            AVG(julianday(last_message_at) - julianday(created_at)) * 86400000 as "average_response_time_ms!: f64",
            MIN(created_at) as "first_interaction!: DateTime<Utc>",
            MAX(updated_at) as "last_interaction!: DateTime<Utc>",
            SUM(COALESCE(total_input_tokens, 0)) as "total_input_tokens!: i64",
            SUM(COALESCE(total_output_tokens, 0)) as "total_output_tokens!: i64",
            COUNT(DISTINCT session_id) as "unique_sessions!: i64"
        FROM agent_conversations
        WHERE agent_id = ? AND status = 'active' AND user_id IS NOT NULL
        GROUP BY user_id
        ORDER BY MAX(updated_at) DESC
        "#,
        nora.id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch all users voice analytics: {}", e);
        ApiError::InternalError(format!("Failed to fetch analytics: {}", e))
    })?;

    let analytics: Vec<VoiceAnalyticsSummary> = results
        .into_iter()
        .map(|r| VoiceAnalyticsSummary {
            user_id: r.user_id.flatten(),
            total_interactions: r.total_interactions,
            total_messages: r.total_messages,
            average_response_time_ms: r.average_response_time_ms,
            first_interaction: r.first_interaction,
            last_interaction: r.last_interaction,
            total_input_tokens: r.total_input_tokens,
            total_output_tokens: r.total_output_tokens,
            unique_sessions: r.unique_sessions,
        })
        .collect();

    Ok(Json(analytics))
}

/// Get conversation history for a specific session
pub async fn get_session_conversation(
    Path(session_id): Path<String>,
    State(state): State<DeploymentImpl>,
) -> Result<Json<Vec<AgentConversationMessage>>, ApiError> {
    let pool = &state.db().pool;
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let conversation = AgentConversation::find_by_agent_session(pool, nora.id, &session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find conversation: {}", e);
            ApiError::InternalError(format!("Failed to find conversation: {}", e))
        })?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

    let messages = AgentConversationMessage::find_by_conversation(pool, conversation.id, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch conversation messages: {}", e);
            ApiError::InternalError(format!("Failed to fetch messages: {}", e))
        })?;

    Ok(Json(messages))
}

/// Get the current Nora voice configuration
pub async fn get_voice_config(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<VoiceConfigResponse>, ApiError> {
    let nora_instance = NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    Ok(Json(VoiceConfigResponse {
        config: nora.config.voice.clone(),
    }))
}

/// Update Nora's voice configuration and reinitialize the engine
pub async fn update_voice_config(
    State(state): State<DeploymentImpl>,
    Json(request): Json<UpdateVoiceConfigRequest>,
) -> Result<Json<VoiceConfigResponse>, ApiError> {
    let nora_instance = NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let mut instance = nora_instance.write().await;
    let nora = instance
        .as_mut()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let new_config = request.config.clone();
    let new_engine = VoiceEngine::new(new_config.clone())
        .await
        .map_err(|err| voice_error_to_api(err))?;

    // Update in-memory configuration
    nora.config.voice = new_config.clone();
    nora.voice_engine = Arc::new(new_engine);

    // Persist configuration to database
    let config_json = serde_json::to_string(&new_config)
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize config: {}", e)))?;

    db::models::nora_config::NoraVoiceConfig::save(&state.db().pool, &config_json)
        .await
        .map_err(|e| {
            tracing::error!("Failed to persist voice config: {}", e);
            ApiError::InternalError(format!("Failed to save configuration: {}", e))
        })?;

    tracing::info!("Voice configuration updated and persisted");

    Ok(Json(VoiceConfigResponse { config: new_config }))
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
    // This is a simplified conversion - in real implementation you'd have proper mapping
    let user_permissions = vec![nora::tools::Permission::Execute]; // TODO: Proper permission mapping

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

/// Get coordination statistics
pub async fn get_coordination_stats(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<CoordinationStats>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let stats = nora
        .coordination_manager
        .get_coordination_stats()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get coordination stats: {}", e)))?;

    Ok(Json(stats))
}

/// Get coordination agent list
pub async fn get_coordination_agents(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<Vec<AgentCoordinationState>>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let agents = nora
        .coordination_manager
        .get_all_agents()
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get coordination agents: {}", e))
        })?;

    Ok(Json(agents))
}

/// Send directives to specific agents (non-Nora) via the global console
pub async fn send_agent_directive(
    Path(agent_id): Path<String>,
    State(_state): State<DeploymentImpl>,
    Json(request): Json<AgentDirectiveRequest>,
) -> Result<Json<AgentDirectiveResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let priority_label = request.priority.as_ref().map(|priority| match priority {
        RequestPriority::Low => "low".to_string(),
        RequestPriority::Normal => "normal".to_string(),
        RequestPriority::High => "high".to_string(),
        RequestPriority::Urgent => "urgent".to_string(),
        RequestPriority::Executive => "executive".to_string(),
    });

    let coordination_manager = nora.coordination_manager.clone();
    let agent_state = coordination_manager
        .get_agent(&agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to lookup agent: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", agent_id)))?;

    coordination_manager
        .record_directive(
            &agent_id,
            &request.session_id,
            &request.content,
            priority_label.clone(),
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to record directive: {}", e)))?;

    let acknowledgement = format!(
        "{} acknowledges directive and is prioritizing: {}",
        agent_state.agent_type, request.content
    );

    let echoed_command = request
        .command
        .clone()
        .filter(|cmd| !cmd.is_empty())
        .unwrap_or_else(|| request.content.clone());

    let response = AgentDirectiveResponse {
        agent_id,
        agent_label: agent_state.agent_type,
        acknowledgement,
        echoed_command,
        priority: priority_label,
        timestamp: Utc::now(),
    };

    Ok(Json(response))
}

/// WebSocket endpoint for coordination events
pub async fn get_coordination_events_ws(
    ws: WebSocketUpgrade,
    State(_state): State<DeploymentImpl>,
) -> Response {
    ws.on_upgrade(handle_coordination_events_websocket)
}

/// SSE endpoint for coordination events (fallback when WebSockets are unavailable)
pub async fn get_coordination_events_sse(
    State(_state): State<DeploymentImpl>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let receiver = {
        let instance = nora_instance.read().await;
        let nora = instance
            .as_ref()
            .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;
        nora.coordination_manager.subscribe_to_events().await
    };

    let stream = futures::stream::unfold(receiver, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let payload = coordination_event_payload(event);
                    let data = payload.to_string();
                    return Some((Ok(Event::default().event("coordination_event").data(data)), rx));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::new()))
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

/// Emit a coordination event so other routes can surface activity without recreating handles
pub async fn emit_coordination_event(event: CoordinationEvent) {
    if let Some(instance) = NORA_INSTANCE.get() {
        let agent = instance.read().await;
        if let Some(nora) = agent.as_ref() {
            if let Err(err) = nora.coordination_manager.emit_event(event).await {
                tracing::debug!("Failed to emit coordination event: {}", err);
            }
        }
    }
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

/// Create a project via Nora
pub async fn nora_create_project(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<NoraCreateProjectRequest>,
) -> Result<Json<NoraProjectResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let project = nora
        .executor
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Task executor not initialized".to_string()))?
        .create_project(
            request.name,
            request.git_repo_path,
            request.setup_script,
            request.dev_script,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create project: {}", e)))?;

    Ok(Json(NoraProjectResponse {
        project_id: project.id.to_string(),
        name: project.name,
        git_repo_path: project.git_repo_path.to_string_lossy().to_string(),
        created_at: project.created_at.to_string(),
    }))
}

/// Create a board via Nora
pub async fn nora_create_board(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<NoraCreateBoardRequest>,
) -> Result<Json<NoraBoardResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid project ID: {}", e)))?;

    // Simplified board types: only Default and Custom
    let board_type = match request.board_type.as_deref() {
        Some("default") | Some("main") => {
            Some(db::models::project_board::ProjectBoardType::Default)
        }
        Some("custom") | Some(_) => Some(db::models::project_board::ProjectBoardType::Custom),
        None => None,
    };

    let board = nora
        .executor
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Task executor not initialized".to_string()))?
        .create_board(project_id, request.name, request.description, board_type)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create board: {}", e)))?;

    Ok(Json(NoraBoardResponse {
        board_id: board.id.to_string(),
        project_id: board.project_id.to_string(),
        name: board.name,
        board_type: format!("{:?}", board.board_type),
        created_at: board.created_at.to_string(),
    }))
}

/// Create a task on a board via Nora
pub async fn nora_create_task(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<NoraCreateTaskRequest>,
) -> Result<Json<NoraTaskResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid project ID: {}", e)))?;

    let board_id = Uuid::parse_str(&request.board_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid board ID: {}", e)))?;

    let priority = request.priority.and_then(|p| match p.as_str() {
        "low" => Some(db::models::task::Priority::Low),
        "medium" => Some(db::models::task::Priority::Medium),
        "high" => Some(db::models::task::Priority::High),
        _ => None,
    });

    let task = nora
        .executor
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Task executor not initialized".to_string()))?
        .create_task_on_board(
            project_id,
            board_id,
            request.title,
            request.description,
            priority,
            request.tags,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create task: {}", e)))?;

    Ok(Json(NoraTaskResponse {
        task_id: task.id.to_string(),
        project_id: task.project_id.to_string(),
        board_id: task.board_id.map(|id| id.to_string()),
        title: task.title,
        status: format!("{:?}", task.status),
        priority: format!("{:?}", task.priority),
        created_at: task.created_at.to_string(),
    }))
}

// Helper functions

/// Get the global NORA instance
pub async fn get_nora_instance() -> Result<Arc<RwLock<Option<NoraAgent>>>, ApiError> {
    NORA_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))
        .map(|instance| instance.clone())
}

async fn handle_coordination_events_websocket(mut socket: axum::extract::ws::WebSocket) {
    tracing::info!("Coordination events WebSocket connection established");

    let Ok(nora_instance) = get_nora_instance().await else {
        let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
        return;
    };

    let receiver_result = {
        let instance = nora_instance.read().await;
        if let Some(nora) = instance.as_ref() {
            Ok(nora.coordination_manager.subscribe_to_events().await)
        } else {
            Err(ApiError::NotFound("Nora not initialized".to_string()))
        }
    };

    let mut receiver = match receiver_result {
        Ok(rx) => rx,
        Err(_) => {
            let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
            return;
        }
    };

    loop {
        tokio::select! {
            socket_msg = socket.recv() => {
                match socket_msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(axum::extract::ws::Message::Ping(p))) => {
                        if socket.send(axum::extract::ws::Message::Pong(p)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::warn!("Coordination WebSocket receive error: {}", e);
                        break;
                    }
                }
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        let payload = coordination_event_payload(event);
                        let text = payload.to_string();
                        if socket
                            .send(axum::extract::ws::Message::Text(text.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("Coordination event receiver lagged by {} messages", skipped);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Coordination event channel closed");
                        break;
                    }
                }
            }
        }
    }

    let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
}

fn coordination_event_payload(event: CoordinationEvent) -> serde_json::Value {
    match event {
        CoordinationEvent::AgentStatusUpdate {
            agent_id,
            status,
            capabilities,
            timestamp,
        } => json!({
            "type": "AgentStatusUpdate",
            "agentId": agent_id,
            "status": status,
            "capabilities": capabilities,
            "timestamp": timestamp,
        }),
        CoordinationEvent::TaskHandoff {
            from_agent,
            to_agent,
            task_id,
            context,
            timestamp,
        } => json!({
            "type": "TaskHandoff",
            "fromAgent": from_agent,
            "toAgent": to_agent,
            "taskId": task_id,
            "context": context,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ConflictResolution {
            conflict_id,
            involved_agents,
            description,
            priority,
            timestamp,
        } => json!({
            "type": "ConflictResolution",
            "conflictId": conflict_id,
            "involvedAgents": involved_agents,
            "description": description,
            "priority": priority,
            "timestamp": timestamp,
        }),
        CoordinationEvent::HumanAvailabilityUpdate {
            user_id,
            availability,
            available_until,
            timestamp,
        } => json!({
            "type": "HumanAvailabilityUpdate",
            "userId": user_id,
            "availability": availability,
            "availableUntil": available_until,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ApprovalRequest {
            request_id,
            requesting_agent,
            action_description,
            required_approver,
            urgency,
            timestamp,
        } => json!({
            "type": "ApprovalRequest",
            "requestId": request_id,
            "requestingAgent": requesting_agent,
            "actionDescription": action_description,
            "requiredApprover": required_approver,
            "urgency": urgency,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutiveAlert {
            alert_id,
            source,
            message,
            severity,
            requires_action,
            timestamp,
        } => json!({
            "type": "ExecutiveAlert",
            "alertId": alert_id,
            "source": source,
            "message": message,
            "severity": severity,
            "requiresAction": requires_action,
            "timestamp": timestamp,
        }),
        CoordinationEvent::AgentDirectiveIssued {
            agent_id,
            issued_by,
            content,
            priority,
            timestamp,
        } => json!({
            "type": "AgentDirective",
            "agentId": agent_id,
            "issuedBy": issued_by,
            "content": content,
            "priority": priority,
            "timestamp": timestamp,
        }),
        CoordinationEvent::WorkflowProgress {
            workflow_instance_id,
            agent_id,
            agent_codename,
            workflow_name,
            current_stage,
            total_stages,
            stage_name,
            status,
            project_id,
            timestamp,
        } => json!({
            "type": "WorkflowProgress",
            "workflowInstanceId": workflow_instance_id,
            "agentId": agent_id,
            "agentCodename": agent_codename,
            "workflowName": workflow_name,
            "currentStage": current_stage,
            "totalStages": total_stages,
            "stageName": stage_name,
            "status": status,
            "projectId": project_id,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionStarted {
            execution_id,
            project_id,
            agent_codename,
            workflow_name,
            timestamp,
        } => json!({
            "type": "ExecutionStarted",
            "executionId": execution_id,
            "projectId": project_id,
            "agentCodename": agent_codename,
            "workflowName": workflow_name,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionStageStarted {
            execution_id,
            stage_index,
            stage_name,
            agent_codename,
            timestamp,
        } => json!({
            "type": "ExecutionStageStarted",
            "executionId": execution_id,
            "stageIndex": stage_index,
            "stageName": stage_name,
            "agentCodename": agent_codename,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionStageCompleted {
            execution_id,
            stage_index,
            stage_name,
            output_summary,
            timestamp,
        } => json!({
            "type": "ExecutionStageCompleted",
            "executionId": execution_id,
            "stageIndex": stage_index,
            "stageName": stage_name,
            "outputSummary": output_summary,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionCompleted {
            execution_id,
            project_id,
            tasks_created,
            artifacts_count,
            duration_ms,
            timestamp,
        } => json!({
            "type": "ExecutionCompleted",
            "executionId": execution_id,
            "projectId": project_id,
            "tasksCreated": tasks_created,
            "artifactsCount": artifacts_count,
            "durationMs": duration_ms,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionFailed {
            execution_id,
            error,
            stage,
            timestamp,
        } => json!({
            "type": "ExecutionFailed",
            "executionId": execution_id,
            "error": error,
            "stage": stage,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionTaskCreated {
            execution_id,
            task_id,
            task_title,
            board_id,
            timestamp,
        } => json!({
            "type": "ExecutionTaskCreated",
            "executionId": execution_id,
            "taskId": task_id,
            "taskTitle": task_title,
            "boardId": board_id,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionArtifactProduced {
            execution_id,
            artifact_type,
            title,
            stage,
            timestamp,
        } => json!({
            "type": "ExecutionArtifactProduced",
            "executionId": execution_id,
            "artifactType": artifact_type,
            "title": title,
            "stage": stage,
            "timestamp": timestamp,
        }),
    }
}

fn map_projects_to_context(projects: Vec<Project>) -> Vec<ProjectContext> {
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

fn apply_llm_overrides(config: &mut NoraConfig) {
    // If OPENAI_API_KEY is set, ensure we have an LLM config
    // This allows the LLM to work without requiring NORA_LLM_* env vars
    if std::env::var("OPENAI_API_KEY").is_ok() && config.llm.is_none() {
        config.llm = Some(LLMConfig::default());
        tracing::info!("LLM enabled via OPENAI_API_KEY");
    }

    if let Ok(model) = std::env::var("NORA_LLM_MODEL") {
        config.llm.get_or_insert_with(LLMConfig::default).model = model;
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

fn estimate_speech_duration(text: &str) -> u64 {
    // Rough estimation: average speaking rate is about 150 words per minute
    let word_count = text.split_whitespace().count();
    let minutes = word_count as f64 / 150.0;
    (minutes * 60.0 * 1000.0) as u64 // Convert to milliseconds
}

fn voice_error_to_api(err: VoiceError) -> ApiError {
    ApiError::InternalError(format!("Voice error: {}", err))
}

fn default_capabilities() -> Vec<String> {
    vec![
        "Voice Interaction".to_string(),
        "Task Coordination".to_string(),
        "Strategic Planning".to_string(),
        "Performance Analysis".to_string(),
        "Decision Support".to_string(),
        "Communication Management".to_string(),
    ]
}

// ApiError is imported from crate::error::ApiError

//! Topsi Platform Agent API routes
//!
//! Topsi is the platform orchestrator that replaces the default control agent
//! for user-facing interactions. It provides containerized access control
//! ensuring strict client data isolation.

pub mod admin;
pub mod chat;
pub mod meetings;
pub mod topology;
pub mod voice;

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::{
    system_settings::SystemSetting,
    task_attempt::{CreateTaskAttempt, TaskAttempt},
    topsi_user_settings::{classify_tool_risk, TopsiUserSettings},
};
use deployment::Deployment;
use executors::{executors::BaseCodingAgent, profile::ExecutorProfileId};
// Import voice types from Nora
use nora::voice::{AudioFormat, SpeechResponse, VoiceConfig, VoiceEngine};
use serde::{Deserialize, Serialize};
use services::services::container::ContainerService;
use sqlx;
use tokio::sync::RwLock;
use topsi::{
    initialize_topsi, AccessScope, DetectedIssue, ProjectAccess, RecommendationBatch,
    TaskExecutionBridge, TopologySummary, TopsiAgent, TopsiConfig, TopsiError, TopsiRequest,
    TopsiRequestType, TopsiResponse, UserContext, VideoJobBridge,
    TopsiRequestType, TopsiResponse, UserContext,
};
use ts_rs::TS;
use uuid::Uuid;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

/// Bridge between Topsi and the Deployment layer for task execution
struct DeploymentBridge {
    deployment: DeploymentImpl,
}

/// Map a PCG Router model_id to the appropriate BaseCodingAgent CLI wrapper
fn model_id_to_executor(model_id: &str) -> BaseCodingAgent {
    if model_id.starts_with("claude-") || model_id.starts_with("anthropic") {
        BaseCodingAgent::ClaudeCode
    } else if model_id.starts_with("gemini-") {
        BaseCodingAgent::Gemini
    } else if model_id.starts_with("gpt-")
        || model_id.starts_with("o1")
        || model_id.starts_with("o3")
        || model_id.starts_with("o4")
    {
        BaseCodingAgent::ClaudeCode // fallback to Claude Code CLI for OpenAI models
    } else if model_id.starts_with("qwen") {
        BaseCodingAgent::QwenCode
    } else {
        BaseCodingAgent::ClaudeCode // safe default
    }
}

#[async_trait::async_trait]
impl TaskExecutionBridge for DeploymentBridge {
    async fn start_task_attempt(
        &self,
        task_id: Uuid,
        model_id: &str,
        base_branch: &str,
    ) -> std::result::Result<serde_json::Value, String> {
        // Map model_id to the appropriate CLI executor
        let base_agent: BaseCodingAgent = model_id_to_executor(model_id);

        let executor_profile_id = ExecutorProfileId::new(base_agent);

        // Create task attempt in DB
        let task_attempt = TaskAttempt::create(
            &self.deployment.db().pool,
            &CreateTaskAttempt {
                executor: executor_profile_id.executor,
                base_branch: base_branch.to_string(),
            },
            task_id,
        )
        .await
        .map_err(|e| format!("Failed to create task attempt: {}", e))?;

        // Start execution via container service
        let execution_process = self
            .deployment
            .container()
            .start_attempt(&task_attempt, executor_profile_id.clone())
            .await
            .map_err(|e| format!("Failed to start execution: {}", e))?;

        // Track analytics
        self.deployment
            .track_if_analytics_allowed(
                "task_attempt_started_by_topsi",
                serde_json::json!({
                    "task_id": task_attempt.task_id.to_string(),
                    "executor": &executor_profile_id.executor,
                    "attempt_id": task_attempt.id.to_string(),
                }),
            )
            .await;

        tracing::info!(
            "[TOPSI] Started execution process {} for task {} via model {}",
            execution_process.id,
            task_id,
            model_id
        );

        Ok(serde_json::json!({
            "success": true,
            "task_attempt_id": task_attempt.id.to_string(),
            "execution_process_id": execution_process.id.to_string(),
            "model_id": model_id,
            "executor": executor_profile_id.executor,
            "status": "started",
            "message": format!("Task execution started with model {}", model_id)
        }))
    }
}

/// Bridge between Topsi and the video production pipeline (ElevenLabs + HeyGen)
struct VideoProductionBridge {
    pool: sqlx::SqlitePool,
}

#[async_trait::async_trait]
impl VideoJobBridge for VideoProductionBridge {
    async fn create_video_job(
        &self,
        avatar_slug: &str,
        script_text: &str,
        background_url: Option<&str>,
        segments_json: Option<&str>,
    ) -> std::result::Result<serde_json::Value, String> {
        use db::models::{
            avatar_profile::AvatarProfile,
            video_job::{CreateVideoJob, VideoJob},
        };

        // Look up the avatar by slug
        let avatar = AvatarProfile::find_by_slug(&self.pool, avatar_slug)
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Avatar '{}' not found", avatar_slug))?;

        if avatar.heygen_avatar_id.is_none() {
            return Err(format!(
                "Avatar '{}' has no HeyGen avatar ID configured. Please set heygen_avatar_id first.",
                avatar_slug
            ));
        }

        // Create the video job record
        let job = VideoJob::create(
            &self.pool,
            CreateVideoJob {
                avatar_profile_id: avatar.id,
                script_text: script_text.to_string(),
                background_url: background_url.map(|s| s.to_string()),
                segments_json: segments_json.map(|s| s.to_string()),
            },
            None,
        )
        .await
        .map_err(|e| format!("Failed to create video job: {}", e))?;

        let job_id = job.id;
        let pool2 = self.pool.clone();
        let script = job.script_text.clone();
        let bg = job.background_url.clone();
        let segs = job.segments_json.clone();

        // Spawn the async pipeline
        tokio::spawn(async move {
            if let Err(e) = crate::routes::video_gen::produce_job(
                &pool2,
                job_id,
                &avatar,
                &script,
                bg.as_deref(),
                segs.as_deref(),
            )
            .await
            {
                tracing::error!("Video pipeline failed for job {}: {}", job_id, e);
                let _ =
                    VideoJob::update_status(&pool2, job_id, "failed", Some(&e.to_string())).await;
            }
        });

        tracing::info!(
            "[TOPSI] Video job {} created for avatar '{}'",
            job_id,
            avatar_slug
        );

        Ok(serde_json::json!({
            "success": true,
            "job_id": job_id.to_string(),
            "avatar": avatar_slug,
            "status": "tts_generating",
            "message": format!("Video production started. Job ID: {}. The video will be ready in ~5 minutes.", job_id)
        }))
    }
}

/// Global Topsi agent instance
pub(crate) static TOPSI_INSTANCE: tokio::sync::OnceCell<Arc<RwLock<Option<TopsiAgent>>>> =
    tokio::sync::OnceCell::const_new();

/// Global Topsi initialization timestamp
pub(crate) static TOPSI_INIT_TIME: tokio::sync::OnceCell<DateTime<Utc>> =
    tokio::sync::OnceCell::const_new();

/// Global voice engine instance for Topsi
pub(crate) static TOPSI_VOICE_ENGINE: tokio::sync::OnceCell<Arc<RwLock<Option<VoiceEngine>>>> =
    tokio::sync::OnceCell::const_new();

/// Topsi's agent UUID as stored in the agents table (used for conversation persistence)
pub(crate) static TOPSI_DB_AGENT_ID: tokio::sync::OnceCell<Uuid> =
    tokio::sync::OnceCell::const_new();

/// Topsi manager for coordinating the agent instance
#[derive(Clone)]
pub struct TopsiManager {
    agent: Arc<RwLock<Option<TopsiAgent>>>,
}

impl TopsiManager {
    /// Create a new TopsiManager
    pub async fn new() -> Self {
        let agent = TOPSI_INSTANCE
            .get_or_init(|| async { Arc::new(RwLock::new(None)) })
            .await
            .clone();

        Self { agent }
    }

    /// Process a request with Topsi
    pub async fn process_request(
        &self,
        request: TopsiRequest,
        user_context: &UserContext,
        session_id: Option<&str>,
    ) -> Result<TopsiResponse, TopsiError> {
        let agent = self.agent.read().await;
        if let Some(topsi) = agent.as_ref() {
            topsi
                .process_request(request, user_context, session_id)
                .await
        } else {
            Err(TopsiError::NotInitialized(
                "Topsi agent not initialized".to_string(),
            ))
        }
    }

    /// Initialize Topsi with config
    pub async fn initialize(&self, config: TopsiConfig) -> Result<String, TopsiError> {
        let mut agent = self.agent.write().await;
        let topsi = TopsiAgent::new(config).await?;
        let id = topsi.id.to_string();

        // Record initialization time
        let _ = TOPSI_INIT_TIME.set(Utc::now());

        *agent = Some(topsi);
        Ok(id)
    }

    /// Check if Topsi is active
    pub async fn is_active(&self) -> bool {
        let agent = self.agent.read().await;
        if let Some(topsi) = agent.as_ref() {
            topsi.is_active().await
        } else {
            false
        }
    }

    /// Get uptime in milliseconds
    pub async fn get_uptime_ms(&self) -> Option<u64> {
        if let Some(init_time) = TOPSI_INIT_TIME.get() {
            let now = Utc::now();
            let duration = now.signed_duration_since(*init_time);
            Some(duration.num_milliseconds() as u64)
        } else {
            None
        }
    }
}

/// Initialize Topsi routes
pub fn topsi_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/topsi/initialize", post(initialize_topsi_handler))
        .route("/topsi/status", get(get_topsi_status))
        .route("/topsi/chat", post(chat::chat_with_topsi))
        .route("/topsi/topology", get(topology::get_topology_overview))
        .route(
            "/topsi/topology/{project_id}",
            get(topology::get_project_topology),
        )
        .route("/topsi/issues", get(topology::detect_issues))
        .route(
            "/topsi/issues/{project_id}",
            get(topology::detect_project_issues),
        )
        .route("/topsi/projects", get(topology::get_accessible_projects))
        .route("/topsi/recommendations", get(topology::get_recommendations))
        .route(
            "/topsi/recommendations/{project_id}",
            get(topology::get_project_recommendations),
        )
        .route("/topsi/command", post(chat::execute_command))
        // Voice routes for Topsi
        .route("/topsi/voice/synthesize", post(voice::synthesize_speech))
        .route("/topsi/voice/transcribe", post(voice::transcribe_speech))
        .route("/topsi/voice/interaction", post(voice::voice_interaction))
        .route(
            "/topsi/voice/config",
            get(voice::get_voice_config).put(voice::update_voice_config),
        )
        // Meeting mode routes
        .route("/topsi/meeting/list", get(meetings::list_meetings))
        .route("/topsi/meeting/start", post(meetings::start_meeting))
        .route("/topsi/meeting/join", post(meetings::join_meeting))
        .route(
            "/topsi/meeting/message",
            post(meetings::meeting_text_message),
        )
        .route("/topsi/meeting/audio", post(meetings::meeting_audio_chunk))
        .route("/topsi/meeting/end", post(meetings::end_meeting))
        .route(
            "/topsi/meeting/status/{session_id}",
            get(meetings::meeting_status),
        )
        .route(
            "/topsi/meeting/notes/{session_id}",
            get(meetings::get_meeting_notes).post(meetings::regenerate_meeting_notes),
        )
        .route(
            "/topsi/meeting/transcript/{session_id}",
            get(meetings::get_meeting_transcript),
        )
        .route(
            "/topsi/meeting/share/{session_id}",
            post(meetings::share_meeting),
        )
        // Admin prompt management (production-safe, admin-only)
        .route(
            "/topsi/admin/prompt",
            get(admin::get_admin_prompt).put(admin::update_admin_prompt),
        )
        // Per-user settings
        .route(
            "/topsi/user-settings",
            get(admin::get_user_settings).put(admin::update_user_settings),
        )
        // Tool metadata (risk classification)
        .route("/topsi/tools", get(admin::get_tool_risk_map))
        .layer(axum::middleware::from_fn(
            crate::middleware::request_id_middleware,
        ))
}

// ============================================================================
// Request/Response Types (shared across sub-modules)
// ============================================================================

/// Request to initialize Topsi
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeTopsiRequest {
    pub config: Option<TopsiConfig>,
    pub activate_immediately: bool,
}

/// Topsi initialization response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeTopsiResponse {
    pub success: bool,
    pub topsi_id: String,
    pub message: String,
    pub capabilities: Vec<String>,
}

/// Topsi status response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiStatusResponse {
    pub is_active: bool,
    pub topsi_id: Option<String>,
    pub uptime_ms: Option<u64>,
    pub access_scope: String,
    pub projects_visible: usize,
    pub system_health: Option<f64>,
}

/// Chat request to Topsi
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiChatRequest {
    pub message: String,
    pub session_id: String,
    pub project_id: Option<Uuid>,
    pub context: Option<serde_json::Value>,
    /// Optional PCG Router `model_id` to override Topsi's configured default LLM
    /// for this turn (e.g. "claude-sonnet-4-6", "gpt-4o"). When absent, Topsi
    /// uses the LLM configured at agent initialization.
    #[serde(default)]
    pub model_id: Option<String>,
}

/// Topology overview response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopologyOverviewResponse {
    pub summaries: Vec<ProjectTopologySummary>,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_clusters: usize,
    pub system_health: Option<f64>,
}

/// Summary of a single project's topology
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTopologySummary {
    pub project_id: Uuid,
    pub project_name: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub cluster_count: usize,
    pub active_routes: usize,
    pub health_score: f64,
}

/// Issues response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssuesResponse {
    pub issues: Vec<DetectedIssue>,
    pub total_count: usize,
    pub critical_count: usize,
    pub warning_count: usize,
}

/// Accessible projects response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AccessibleProjectsResponse {
    pub projects: Vec<ProjectAccess>,
    pub access_level: String,
    pub is_admin: bool,
}

/// Recommendations query parameters
#[derive(Debug, Deserialize)]
pub struct RecommendationsQuery {
    pub max_count: Option<usize>,
}

/// Command request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub command: String,
    pub args: Option<serde_json::Value>,
    pub session_id: String,
    pub project_id: Option<Uuid>,
}

// ============================================================================
// Route Handlers (kept in mod.rs: init + status)
// ============================================================================

/// Initialize Topsi platform agent
pub async fn initialize_topsi_handler(
    State(state): State<DeploymentImpl>,
    Json(request): Json<InitializeTopsiRequest>,
) -> Result<Json<InitializeTopsiResponse>, ApiError> {
    tracing::info!("Initializing Topsi platform agent");

    let topsi_instance = TOPSI_INSTANCE
        .get_or_init(|| async { Arc::new(RwLock::new(None)) })
        .await;

    // If Topsi is already initialized and activation is not forced, return current status
    if !request.activate_immediately {
        let instance = topsi_instance.read().await;
        if let Some(existing) = instance.as_ref() {
            return Ok(Json(InitializeTopsiResponse {
                success: true,
                topsi_id: existing.id.to_string(),
                message: "Topsi is already active and ready to assist.".to_string(),
                capabilities: default_capabilities(),
            }));
        }
    }

    let mut config = request.config.unwrap_or_default();
    apply_topsi_llm_overrides(&mut config);

    // Create the execution bridge with deployment handle
    let bridge = Arc::new(DeploymentBridge {
        deployment: state.clone(),
    });
    let video_bridge = Arc::new(VideoProductionBridge {
        pool: state.db().pool.clone(),
    });

    let topsi_agent = initialize_topsi(config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to initialize Topsi: {}", e);
            ApiError::InternalError(format!("Topsi initialization failed: {}", e))
        })?
        .with_database(state.db().pool.clone())
        .await
        .with_execution_bridge(bridge)
        .with_video_job_bridge(video_bridge);

    let topsi_id = topsi_agent.id.to_string();

    if request.activate_immediately {
        topsi_agent
            .set_active(true)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to activate Topsi: {}", e)))?;
    }

    {
        let mut instance = topsi_instance.write().await;
        *instance = Some(topsi_agent);
    }

    // Record initialization time
    let _ = TOPSI_INIT_TIME.set(Utc::now());

    tracing::info!("Topsi initialized successfully with ID: {}", topsi_id);

    Ok(Json(InitializeTopsiResponse {
        success: true,
        topsi_id,
        message: "Topsi platform agent initialized. I can help you manage your projects and coordinate with the ecosystem.".to_string(),
        capabilities: default_capabilities(),
    }))
}

/// Initialize Topsi on server startup (called automatically)
pub async fn initialize_topsi_on_startup(state: &DeploymentImpl) -> Result<String, String> {
    tracing::info!("Auto-initializing Topsi platform agent on server startup");

    let topsi_instance = TOPSI_INSTANCE
        .get_or_init(|| async { Arc::new(RwLock::new(None)) })
        .await;

    // Check if already initialized
    {
        let instance = topsi_instance.read().await;
        if instance.is_some() {
            tracing::info!("Topsi already initialized, skipping auto-initialization");
            return Ok("Already initialized".to_string());
        }
    }

    let mut config = TopsiConfig::default();
    apply_topsi_llm_overrides(&mut config);

    // Load persisted autonomy level from system_settings
    let pool = &state.db().pool;
    if let Ok(Some(level_str)) = SystemSetting::get(pool, "topsi_autonomy_level").await {
        config.autonomy_level = topsi::config::AutonomyLevel::parse_level(&level_str);
        tracing::info!("Loaded persisted autonomy level: {}", level_str);
    }

    // Create the execution bridge with deployment handle
    let bridge = Arc::new(DeploymentBridge {
        deployment: state.clone(),
    });
    let video_bridge = Arc::new(VideoProductionBridge {
        pool: state.db().pool.clone(),
    });

    let topsi_agent = initialize_topsi(config)
        .await
        .map_err(|e| format!("Topsi initialization failed: {}", e))?
        .with_database(state.db().pool.clone())
        .await
        .with_execution_bridge(bridge)
        .with_video_job_bridge(video_bridge);

    let topsi_id = topsi_agent.id.to_string();

    // Resolve Topsi's DB agent ID (for FK-safe conversation persistence)
    let db_agent_id: Option<Uuid> = sqlx::query_scalar::<_, String>(
        "SELECT id FROM agents WHERE short_name = 'Topsi' AND status = 'active' LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|id_str| Uuid::parse_str(&id_str).ok());

    if let Some(db_id) = db_agent_id {
        let _ = TOPSI_DB_AGENT_ID.set(db_id);
        tracing::info!("Topsi DB agent ID resolved: {}", db_id);
    } else {
        tracing::warn!(
            "Topsi agent not found in agents table — conversation persistence will be skipped"
        );
    }

    // Activate by default
    topsi_agent
        .set_active(true)
        .await
        .map_err(|e| format!("Failed to activate Topsi: {}", e))?;

    {
        let mut instance = topsi_instance.write().await;
        *instance = Some(topsi_agent);
    }

    let _ = TOPSI_INIT_TIME.set(Utc::now());

    tracing::info!("Topsi auto-initialized successfully with ID: {}", topsi_id);
    Ok(topsi_id)
}

/// Get Topsi status
pub async fn get_topsi_status(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<TopsiStatusResponse>, ApiError> {
    let topsi_instance = TOPSI_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let instance = topsi_instance.read().await;

    if let Some(topsi) = instance.as_ref() {
        let is_active = topsi.is_active().await;

        // Calculate uptime
        let uptime_ms = if let Some(init_time) = TOPSI_INIT_TIME.get() {
            let now = Utc::now();
            let duration = now.signed_duration_since(*init_time);
            Some(duration.num_milliseconds() as u64)
        } else {
            None
        };

        // Get user context from request headers (auth token or cookie)
        let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
        let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
        let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;
        let scope = topsi.access_control.get_access_scope(&user_context).await;

        let (access_scope_str, projects_visible) = match &scope {
            AccessScope::Admin => ("admin".to_string(), 0), // Would need to count all
            AccessScope::Projects(ids) => ("projects".to_string(), ids.len()),
            AccessScope::SingleProject(_) => ("single_project".to_string(), 1),
            AccessScope::None => ("none".to_string(), 0),
        };

        Ok(Json(TopsiStatusResponse {
            is_active,
            topsi_id: Some(topsi.id.to_string()),
            uptime_ms,
            access_scope: access_scope_str,
            projects_visible,
            system_health: None, // Only for admins
        }))
    } else {
        Ok(Json(TopsiStatusResponse {
            is_active: false,
            topsi_id: None,
            uptime_ms: None,
            access_scope: "none".to_string(),
            projects_visible: 0,
            system_health: None,
        }))
    }
}

// ============================================================================
// Helper Functions (shared across sub-modules)
// ============================================================================

/// Get the global Topsi instance
pub async fn get_topsi_instance() -> Result<Arc<RwLock<Option<TopsiAgent>>>, ApiError> {
    TOPSI_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))
        .cloned()
}

/// Get user context from state (simplified - would extract from auth in production)
pub(crate) async fn get_user_context_from_req(
    state: &DeploymentImpl,
    auth_header: Option<&str>,
    cookie_header: Option<&str>,
) -> UserContext {
    use crate::middleware::access_control::get_current_user;

    // Try to get authenticated user from session/token
    match get_current_user(state, auth_header, cookie_header).await {
        Ok(access_ctx) => {
            // Convert AccessContext to Topsi UserContext
            if access_ctx.is_admin {
                UserContext::admin(access_ctx.user_id.to_string())
                    .with_session(Uuid::new_v4().to_string())
            } else {
                UserContext::user(access_ctx.user_id.to_string())
                    .with_session(Uuid::new_v4().to_string())
            }
        }
        Err(_) => {
            // No authentication - return restricted context
            UserContext::user("anonymous").with_session(Uuid::new_v4().to_string())
        }
    }
}

// TODO: unused — comment out to suppress warning
// // Legacy function - kept for backward compatibility but marked deprecated
// #[deprecated(note = "Use get_user_context_from_req instead")]
// async fn get_user_context_from_state(_state: &DeploymentImpl) -> UserContext {
//     // Return anonymous user context
//     UserContext::user("anonymous")
//         .with_session(Uuid::new_v4().to_string())
// }

fn default_capabilities() -> Vec<String> {
    vec![
        "Topology Management".to_string(),
        "Access Control".to_string(),
        "Pattern Detection".to_string(),
        "Route Planning".to_string(),
        "Issue Detection".to_string(),
        "Cluster Management".to_string(),
        "Multi-Project Coordination".to_string(),
        "LLM-Powered Conversations".to_string(),
    ]
}

/// Apply environment variable overrides to Topsi config
///
/// Supports the following env vars:
/// - TOPSI_LLM_MODEL: Override the LLM model (e.g., "gpt-4o", "claude-sonnet-4")
/// - TOPSI_LLM_PROVIDER: Override the provider (e.g., "openai", "anthropic", "ollama")
/// - TOPSI_LLM_TEMPERATURE: Override temperature (0.0 - 1.0)
/// - TOPSI_LLM_MAX_TOKENS: Override max tokens
fn apply_topsi_llm_overrides(config: &mut TopsiConfig) {
    // Model override
    if let Ok(model) = std::env::var("TOPSI_LLM_MODEL") {
        tracing::info!("Topsi LLM model overridden to: {}", model);
        config.llm.model = model;
    }

    // Provider override
    if let Ok(provider) = std::env::var("TOPSI_LLM_PROVIDER") {
        tracing::info!("Topsi LLM provider overridden to: {}", provider);
        config.llm.provider = provider;
    }

    // Temperature override
    if let Ok(temp_str) = std::env::var("TOPSI_LLM_TEMPERATURE") {
        if let Ok(temp) = temp_str.parse::<f32>() {
            tracing::info!("Topsi LLM temperature overridden to: {}", temp);
            config.llm.temperature = temp;
        }
    }

    // Max tokens override
    if let Ok(tokens_str) = std::env::var("TOPSI_LLM_MAX_TOKENS") {
        if let Ok(tokens) = tokens_str.parse::<u32>() {
            tracing::info!("Topsi LLM max_tokens overridden to: {}", tokens);
            config.llm.max_tokens = tokens;
        }
    }
}

// Billing logic extracted to crate::helpers::billing (Sprint 5).

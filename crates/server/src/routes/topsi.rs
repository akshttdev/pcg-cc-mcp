//! Topsi Platform Agent API routes
//!
//! Topsi is the platform orchestrator that replaces the default control agent
//! for user-facing interactions. It provides containerized access control
//! ensuring strict client data isolation.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use topsi::{
    TopsiAgent, TopsiConfig, TopsiError, TopsiRequest, TopsiRequestType, TopsiResponse,
    TopologySummary, DetectedIssue, UserContext, AccessScope, ProjectAccess,
    RecommendationBatch, TaskExecutionBridge,
    initialize_topsi,
};
use ts_rs::TS;
use uuid::Uuid;

use deployment::Deployment;
use db::models::project::Project;
use sqlx;
use db::models::task_attempt::{CreateTaskAttempt, TaskAttempt};
use db::models::vibe_deposit::{VibeDeposit, VibeWithdrawal};
use db::models::vibe_transaction::{VibeSourceType, VibeTransaction};
use executors::executors::BaseCodingAgent;
use executors::profile::ExecutorProfileId;
use services::services::container::ContainerService;
use services::services::vibe_pricing::VibePricingService;

// Import voice types from Nora
use nora::voice::{
    VoiceConfig, VoiceEngine, SpeechRequest, SpeechResponse, AudioFormat,
    tts::VoiceProfile,
};

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

/// Bridge between Topsi and the Deployment layer for task execution
struct DeploymentBridge {
    deployment: DeploymentImpl,
}

#[async_trait::async_trait]
impl TaskExecutionBridge for DeploymentBridge {
    async fn start_task_attempt(
        &self,
        task_id: Uuid,
        executor_name: &str,
        base_branch: &str,
    ) -> std::result::Result<serde_json::Value, String> {
        // Parse executor name to BaseCodingAgent enum
        let base_agent: BaseCodingAgent = executor_name
            .parse()
            .map_err(|_| format!("Unknown executor '{}'. Available: CLAUDE_CODE, AMP, GEMINI, CODEX", executor_name))?;

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
            "[TOPSI] Started execution process {} for task {} via {}",
            execution_process.id,
            task_id,
            executor_name
        );

        Ok(serde_json::json!({
            "success": true,
            "task_attempt_id": task_attempt.id.to_string(),
            "execution_process_id": execution_process.id.to_string(),
            "executor": executor_name,
            "status": "started",
            "message": format!("Task execution started with {} agent", executor_name)
        }))
    }
}

/// Global Topsi agent instance
static TOPSI_INSTANCE: tokio::sync::OnceCell<Arc<RwLock<Option<TopsiAgent>>>> =
    tokio::sync::OnceCell::const_new();

/// Global Topsi initialization timestamp
static TOPSI_INIT_TIME: tokio::sync::OnceCell<DateTime<Utc>> = tokio::sync::OnceCell::const_new();

/// Global voice engine instance for Topsi
static TOPSI_VOICE_ENGINE: tokio::sync::OnceCell<Arc<RwLock<Option<VoiceEngine>>>> =
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
            topsi.process_request(request, user_context, session_id).await
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
        .route("/topsi/chat", post(chat_with_topsi))
        .route("/topsi/topology", get(get_topology_overview))
        .route("/topsi/topology/{project_id}", get(get_project_topology))
        .route("/topsi/issues", get(detect_issues))
        .route("/topsi/issues/{project_id}", get(detect_project_issues))
        .route("/topsi/projects", get(get_accessible_projects))
        .route("/topsi/recommendations", get(get_recommendations))
        .route("/topsi/recommendations/{project_id}", get(get_project_recommendations))
        .route("/topsi/command", post(execute_command))
        // Voice routes for Topsi
        .route("/topsi/voice/synthesize", post(synthesize_speech))
        .route("/topsi/voice/transcribe", post(transcribe_speech))
        .route("/topsi/voice/interaction", post(voice_interaction))
        .route("/topsi/voice/config", get(get_voice_config).put(update_voice_config))
        // Meeting mode routes
        .route("/topsi/meeting/heartbeat", post(heartbeat_meeting))
        .route("/topsi/meeting/list", get(list_meetings))
        .route("/topsi/meeting/start", post(start_meeting))
        .route("/topsi/meeting/join", post(join_meeting))
        .route("/topsi/meeting/message", post(meeting_text_message))
        .route("/topsi/meeting/audio", post(meeting_audio_chunk))
        .route("/topsi/meeting/end", post(end_meeting))
        .route("/topsi/meeting/status/{session_id}", get(meeting_status))
        .route("/topsi/meeting/notes/{session_id}", get(get_meeting_notes).post(regenerate_meeting_notes))
        .route("/topsi/meeting/transcript/{session_id}", get(get_meeting_transcript))
        .route("/topsi/meeting/share/{session_id}", post(share_meeting))
        // Recording upload / transcript import / person linking
        .route(
            "/topsi/meeting/upload",
            post(upload_meeting_recording)
                .layer(DefaultBodyLimit::max(200 * 1024 * 1024)),
        )
        .route("/topsi/meeting/upload-transcript", post(upload_meeting_transcript))
        .route("/topsi/meeting/suggest-persons/{session_id}", get(suggest_meeting_persons))
        .route("/topsi/meeting/link-persons/{session_id}", post(link_meeting_persons))
        .layer(axum::middleware::from_fn(
            crate::middleware::request_id_middleware,
        ))
}

// ============================================================================
// Request/Response Types
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
// Voice Request/Response Types
// ============================================================================

/// Request to synthesize speech
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSynthesisRequest {
    pub text: String,
    pub voice_profile: Option<String>,
    pub speed: Option<f32>,
    pub executive_tone: Option<bool>,
}

/// Request to transcribe speech
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionRequest {
    pub audio_data: String, // Base64 encoded audio
    pub language: Option<String>,
}

/// Response from transcription
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionResponse {
    pub text: String,
    pub confidence: Option<f32>,
    pub processing_time_ms: u64,
}

/// Voice interaction request (combined transcribe + process + synthesize)
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiVoiceInteraction {
    pub session_id: String,
    pub audio_input: Option<String>,     // Base64 encoded audio input
    pub text_input: Option<String>,       // Text input (alternative to audio)
    pub transcription: Option<String>,    // Transcribed text (filled by server)
    pub response_text: Option<String>,    // Topsi's response (filled by server)
    pub audio_response: Option<String>,   // Base64 encoded audio response
    pub processing_time_ms: Option<u64>,
    pub timestamp: Option<DateTime<Utc>>,
    /// Action signal for frontend (e.g. "start_meeting", "end_meeting")
    pub action: Option<String>,
    /// Meeting session ID if a meeting was started/is active
    pub meeting_session_id: Option<String>,
    /// Project ID associated with the action
    pub action_project_id: Option<String>,
}

/// Voice config response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiVoiceConfigResponse {
    pub tts_provider: String,
    pub stt_provider: String,
    pub voice_profile: String,
    pub is_ready: bool,
}

/// Update voice config request
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTopsiVoiceConfigRequest {
    pub tts_provider: Option<String>,
    pub stt_provider: Option<String>,
    pub voice_profile: Option<String>,
}

// ============================================================================
// Meeting Request/Response Types
// ============================================================================

/// Request to start a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartMeetingRequest {
    pub project_id: String,
    pub title: Option<String>,
    pub session_id: Option<String>,
}

/// Response from starting a meeting
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartMeetingResponse {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub message: String,
}

/// Request with an audio chunk from a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAudioChunkRequest {
    pub session_id: String,
    pub audio_data: String, // Base64 encoded audio
    pub chunk_index: u32,
    pub duration_ms: u32,
}

/// Response from processing a meeting audio chunk
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAudioChunkResponse {
    pub session_id: String,
    pub chunk_index: u32,
    pub text: String,
    pub speaker_label: Option<String>,
    pub is_topsi_addressed: bool,
    pub topsi_response: Option<String>,
    pub topsi_audio_response: Option<String>,
    pub segment_index: i32,
}

/// Request to send a heartbeat for an active meeting session
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingHeartbeatRequest {
    pub session_id: String,
}

/// Request to end a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EndMeetingRequest {
    pub session_id: String,
    pub generate_notes: Option<bool>,
}

/// Response from ending a meeting
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EndMeetingResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: i32,
    pub participant_count: i32,
    pub segment_count: usize,
    pub notes: Option<topsi::MeetingNotes>,
}

/// Request to share a meeting
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShareMeetingRequest {
    pub user_ids: Vec<String>,
}

/// Meeting status response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingStatusResponse {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub duration_seconds: Option<i32>,
    pub participant_count: i32,
    pub segment_count: i64,
    pub is_active: bool,
}

/// Meeting transcript response
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTranscriptResponse {
    pub session_id: String,
    pub segments: Vec<db::models::meeting_session::MeetingSegment>,
    pub total_count: i64,
}

/// Request to join an existing meeting session
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingRequest {
    pub session_id: String,
}

/// Response from joining a meeting
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingResponse {
    pub session_id: String,
    pub title: String,
    pub project_id: String,
    pub participant_count: i32,
}

/// Request to add a typed text message/link to an active meeting (no audio)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMessageRequest {
    pub session_id: String,
    pub text: String,
    pub speaker_label: Option<String>,
    pub is_link: Option<bool>,
}

/// Response from posting a meeting message
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMessageResponse {
    pub session_id: String,
    pub segment_index: i32,
    pub text: String,
}

/// Query params for listing meetings
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMeetingsQuery {
    pub project_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Response for listing meetings
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMeetingsResponse {
    pub meetings: Vec<MeetingSessionSummary>,
    pub total: usize,
}

/// Summary of a meeting session for list views
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSessionSummary {
    pub id: String,
    pub project_id: String,
    pub project_name: Option<String>,
    pub org_name: Option<String>,
    pub title: String,
    pub status: String,
    pub started_by: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<i32>,
    pub participant_count: Option<i32>,
    pub notes: Option<serde_json::Value>,
}

/// Response from joining a meeting, extended with project/org context
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingResponseFull {
    pub session_id: String,
    pub title: String,
    pub project_id: String,
    pub project_name: Option<String>,
    pub org_name: Option<String>,
    pub participant_count: i32,
}

// ============================================================================
// Route Handlers
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

    let topsi_agent = initialize_topsi(config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to initialize Topsi: {}", e);
            ApiError::InternalError(format!("Topsi initialization failed: {}", e))
        })?
        .with_database(state.db().pool.clone())
        .await
        .with_execution_bridge(bridge);

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

    // Create the execution bridge with deployment handle
    let bridge = Arc::new(DeploymentBridge {
        deployment: state.clone(),
    });

    let topsi_agent = initialize_topsi(config)
        .await
        .map_err(|e| format!("Topsi initialization failed: {}", e))?
        .with_database(state.db().pool.clone())
        .await
        .with_execution_bridge(bridge);

    let topsi_id = topsi_agent.id.to_string();

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

/// Chat with Topsi
pub async fn chat_with_topsi(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    headers: axum::http::HeaderMap,
    Json(request): Json<TopsiChatRequest>,
) -> Result<Json<TopsiResponse>, ApiError> {
    tracing::info!("Received chat request: {:?}", request.message);

    let pool = state.db().pool.clone();

    // Resolve project to bill against
    let billing_project_id = match request.project_id {
        Some(pid) => Some(pid),
        None => {
            let home: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT home_project_id FROM users WHERE id = ?",
            )
            .bind(access_ctx.user_id.as_bytes().as_slice())
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
            home.and_then(|bytes| Uuid::from_slice(&bytes).ok())
        }
    };

    // VIBE Balance Check — uses real deposit ledger
    if let Some(project_id) = billing_project_id {
        let total_deposited = VibeDeposit::total_deposited(&pool, project_id).await.unwrap_or(0);
        let total_withdrawn = VibeWithdrawal::total_withdrawn(&pool, project_id).await.unwrap_or(0);
        let total_spent = VibeTransaction::sum_by_source(&pool, VibeSourceType::Project, project_id, None)
            .await
            .map(|s| s.total_vibe)
            .unwrap_or(0);
        let balance = total_deposited - total_withdrawn - total_spent;
        if balance <= 0 {
            return Err(ApiError::PaymentRequired(
                "Insufficient VIBE balance. Deposit VIBE tokens to your project to continue.".into(),
            ));
        }
    }

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    // Get REAL user context from authentication
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session_id = request.session_id.clone();

    let topsi_request = TopsiRequest::new(TopsiRequestType::Chat {
        message: request.message,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, Some(&session_id))
        .await
        .map_err(|e| {
            tracing::error!("Topsi processing error: {}", e);
            ApiError::InternalError(format!("Topsi processing failed: {}", e))
        })?;

    // Record VIBE cost
    if let Some(project_id) = billing_project_id {
        let input_tokens = response.input_tokens.unwrap_or(0);
        let output_tokens = response.output_tokens.unwrap_or(0);
        if input_tokens > 0 || output_tokens > 0 {
            let vibe_pricing = VibePricingService::new(pool.clone());
            match vibe_pricing.record_llm_usage(
                VibeSourceType::Project, project_id,
                "claude-sonnet-4-20250514",
                input_tokens, output_tokens,
                None, None, None,
            ).await {
                Ok(tx) => {
                    let _ = Project::adjust_vibe_spent(&pool, project_id, tx.amount_vibe).await;
                    tracing::info!("[VIBE] Topsi recorded {} VIBE for project {}", tx.amount_vibe, project_id);
                }
                Err(e) => tracing::error!("[VIBE] Failed to record Topsi usage: {}", e),
            }
        }
    }

    Ok(Json(response))
}

/// Get topology overview (all accessible projects)
pub async fn get_topology_overview(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<TopologyOverviewResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetTopology {
        project_id: None,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get topology: {}", e)))?;

    // Build overview from response
    let summary = response.topology_summary.unwrap_or(TopologySummary {
        node_count: 0,
        edge_count: 0,
        cluster_count: 0,
        active_routes: 0,
        unresolved_issues: 0,
        nodes_by_type: Vec::new(),
        edges_by_type: Vec::new(),
        health_score: 1.0,
    });

    Ok(Json(TopologyOverviewResponse {
        summaries: Vec::new(), // Would need to return per-project summaries
        total_nodes: summary.node_count,
        total_edges: summary.edge_count,
        total_clusters: summary.cluster_count,
        system_health: Some(summary.health_score),
    }))
}

/// Get topology for a specific project
pub async fn get_project_topology(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<TopologyOverviewResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Verify access
    if !topsi.access_control.can_access_project(&user_context, project_id).await {
        return Err(ApiError::Forbidden(format!(
            "Access denied to project {}",
            project_id
        )));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetTopology {
        project_id: Some(project_id),
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get topology: {}", e)))?;

    let summary = response.topology_summary.unwrap_or_default();

    Ok(Json(TopologyOverviewResponse {
        summaries: Vec::new(),
        total_nodes: summary.node_count,
        total_edges: summary.edge_count,
        total_clusters: summary.cluster_count,
        system_health: Some(summary.health_score),
    }))
}

/// Detect issues across accessible projects
pub async fn detect_issues(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<IssuesResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::DetectIssues {
        project_id: None,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to detect issues: {}", e)))?;

    let critical_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "critical")
        .count();
    let warning_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "warning")
        .count();

    Ok(Json(IssuesResponse {
        total_count: response.issues.len(),
        critical_count,
        warning_count,
        issues: response.issues,
    }))
}

/// Detect issues for a specific project
pub async fn detect_project_issues(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<IssuesResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Verify access
    if !topsi.access_control.can_access_project(&user_context, project_id).await {
        return Err(ApiError::Forbidden(format!(
            "Access denied to project {}",
            project_id
        )));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::DetectIssues {
        project_id: Some(project_id),
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to detect issues: {}", e)))?;

    let critical_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "critical")
        .count();
    let warning_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "warning")
        .count();

    Ok(Json(IssuesResponse {
        total_count: response.issues.len(),
        critical_count,
        warning_count,
        issues: response.issues,
    }))
}

/// Get accessible projects for current user
pub async fn get_accessible_projects(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AccessibleProjectsResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let projects = topsi
        .access_control
        .get_accessible_projects(&user_context)
        .await;

    let access_level = if user_context.is_admin {
        "admin"
    } else if projects.is_empty() {
        "none"
    } else {
        "user"
    };

    Ok(Json(AccessibleProjectsResponse {
        projects,
        access_level: access_level.to_string(),
        is_admin: user_context.is_admin,
    }))
}

/// Get recommendations across all accessible projects
pub async fn get_recommendations(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Query(params): Query<RecommendationsQuery>,
) -> Result<Json<RecommendationBatch>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetRecommendations {
        project_id: None,
        max_count: params.max_count,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get recommendations: {}", e)))?;

    // Parse the JSON message back into a RecommendationBatch
    let batch: RecommendationBatch = serde_json::from_str(&response.message)
        .map_err(|e| ApiError::InternalError(format!("Failed to parse recommendations: {}", e)))?;

    Ok(Json(batch))
}

/// Get recommendations for a specific project
pub async fn get_project_recommendations(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<Uuid>,
    Query(params): Query<RecommendationsQuery>,
) -> Result<Json<RecommendationBatch>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Verify access
    if !topsi.access_control.can_access_project(&user_context, project_id).await {
        return Err(ApiError::Forbidden(format!(
            "Access denied to project {}",
            project_id
        )));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetRecommendations {
        project_id: Some(project_id),
        max_count: params.max_count,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get recommendations: {}", e)))?;

    let batch: RecommendationBatch = serde_json::from_str(&response.message)
        .map_err(|e| ApiError::InternalError(format!("Failed to parse recommendations: {}", e)))?;

    Ok(Json(batch))
}

/// Execute a Topsi command
pub async fn execute_command(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CommandRequest>,
) -> Result<Json<TopsiResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::ExecuteCommand {
        command: request.command,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| {
            tracing::error!("Command execution error: {}", e);
            ApiError::InternalError(format!("Command execution failed: {}", e))
        })?;

    Ok(Json(response))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the global Topsi instance
pub async fn get_topsi_instance() -> Result<Arc<RwLock<Option<TopsiAgent>>>, ApiError> {
    TOPSI_INSTANCE
        .get()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))
        .map(|instance| instance.clone())
}

/// Get user context from state (simplified - would extract from auth in production)
async fn get_user_context_from_req(
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
            UserContext::user("anonymous")
                .with_session(Uuid::new_v4().to_string())
        }
    }
}

// Legacy function - kept for backward compatibility but marked deprecated
#[deprecated(note = "Use get_user_context_from_req instead")]
async fn get_user_context_from_state(_state: &DeploymentImpl) -> UserContext {
    // Return anonymous user context
    UserContext::user("anonymous")
        .with_session(Uuid::new_v4().to_string())
}

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

// ============================================================================
// Voice Route Handlers
// ============================================================================

/// Sanitize text for TTS synthesis
///
/// Removes JSON, tool execution details, and special characters that can cause
/// TTS engines (especially Chatterbox) to fail.
fn sanitize_text_for_tts(text: &str) -> String {
    let mut result = text.to_string();

    // Remove JSON blocks (anything between { and } that looks like JSON)
    // This regex approach handles nested braces
    let mut depth = 0;
    let mut json_start = None;
    let chars: Vec<char> = result.chars().collect();
    let mut ranges_to_remove = Vec::new();

    for (i, ch) in chars.iter().enumerate() {
        match ch {
            '{' => {
                if depth == 0 {
                    json_start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = json_start {
                        ranges_to_remove.push((start, i + 1));
                    }
                    json_start = None;
                }
            }
            _ => {}
        }
    }

    // Remove JSON blocks in reverse order to preserve indices
    for (start, end) in ranges_to_remove.into_iter().rev() {
        result = format!("{}{}", &result[..start], &result[end..]);
    }

    // Remove tool execution prefixes like "Executed N tools:" and checkmarks
    result = result
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            // Skip lines that look like tool execution summaries
            !trimmed.starts_with("Executed ")
                && !trimmed.starts_with("✓ ")
                && !trimmed.starts_with("✗ ")
                && !trimmed.contains("list_projects:")
                && !trimmed.contains("list_tasks:")
                && !trimmed.contains("create_task:")
                && !trimmed.contains("update_task:")
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Remove common special characters that cause TTS issues
    result = result
        .replace("```", "")
        .replace("`", "")
        .replace("**", "")
        .replace("__", "")
        .replace("##", "")
        .replace("# ", "")
        .replace("[", "")
        .replace("]", "")
        .replace("(", "")
        .replace(")", "");

    // Collapse multiple spaces and trim
    let mut prev_space = false;
    result = result
        .chars()
        .filter(|&c| {
            if c.is_whitespace() {
                if prev_space {
                    false
                } else {
                    prev_space = true;
                    true
                }
            } else {
                prev_space = false;
                true
            }
        })
        .collect();

    result.trim().to_string()
}

/// Get or initialize the Topsi voice engine
async fn get_or_init_voice_engine() -> Result<Arc<RwLock<Option<VoiceEngine>>>, ApiError> {
    let engine = TOPSI_VOICE_ENGINE
        .get_or_init(|| async {
            tracing::info!("Initializing Topsi voice engine...");

            // Check for Chatterbox availability, fall back to OpenAI if not available
            // Respect CHATTERBOX_URL env var (e.g. http://localhost:8100), else check CHATTERBOX_PORT
            let chatterbox_url = std::env::var("CHATTERBOX_URL")
                .map(|url| format!("{}/health", url.trim_end_matches('/')))
                .unwrap_or_else(|_| {
                    let port = std::env::var("CHATTERBOX_PORT").unwrap_or_else(|_| "8100".to_string());
                    format!("http://localhost:{}/health", port)
                });
            let chatterbox_available = reqwest::Client::new()
                .get(&chatterbox_url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);

            let config = if chatterbox_available {
                tracing::info!("Chatterbox TTS available, using local voice engine");
                VoiceConfig::british_executive()
            } else {
                tracing::info!("Chatterbox not available, using OpenAI TTS");
                // Create config with OpenAI as TTS provider
                let mut config = VoiceConfig::development();
                config.tts.provider = nora::voice::config::TTSProvider::OpenAI;
                config.tts.voice_id = "fable".to_string(); // British-leaning voice
                config
            };

            match VoiceEngine::new(config).await {
                Ok(engine) => {
                    tracing::info!("Topsi voice engine initialized successfully");
                    Arc::new(RwLock::new(Some(engine)))
                }
                Err(e) => {
                    tracing::error!("Failed to initialize Topsi voice engine: {}", e);
                    Arc::new(RwLock::new(None))
                }
            }
        })
        .await;

    Ok(engine.clone())
}

/// Synthesize speech from text using Topsi's voice engine
pub async fn synthesize_speech(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<VoiceSynthesisRequest>,
) -> Result<Json<SpeechResponse>, ApiError> {
    let start = std::time::Instant::now();

    let engine_lock = get_or_init_voice_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Voice engine not initialized".to_string()))?;

    tracing::info!("Topsi synthesizing speech: {} chars", request.text.len());

    let audio_data = engine
        .synthesize_speech(&request.text)
        .await
        .map_err(|e| {
            tracing::error!("Topsi speech synthesis error: {}", e);
            ApiError::InternalError(format!("Speech synthesis failed: {}", e))
        })?;

    let duration = start.elapsed();
    let processing_time_ms = duration.as_millis() as u64;

    // Estimate duration based on text length (150 words per minute)
    let word_count = request.text.split_whitespace().count();
    let estimated_duration_ms = (word_count as f64 / 150.0 * 60.0 * 1000.0) as u64;

    Ok(Json(SpeechResponse {
        audio_data,
        duration_ms: estimated_duration_ms,
        sample_rate: 24000,
        format: AudioFormat::Wav,
        processing_time_ms,
    }))
}

/// Transcribe speech to text using Topsi's voice engine
pub async fn transcribe_speech(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<VoiceTranscriptionRequest>,
) -> Result<Json<VoiceTranscriptionResponse>, ApiError> {
    let start = std::time::Instant::now();

    let engine_lock = get_or_init_voice_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Voice engine not initialized".to_string()))?;

    tracing::info!("Topsi transcribing speech...");

    let text = engine
        .transcribe_speech(&request.audio_data)
        .await
        .map_err(|e| {
            tracing::error!("Topsi speech transcription error: {}", e);
            ApiError::InternalError(format!("Speech transcription failed: {}", e))
        })?;

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(VoiceTranscriptionResponse {
        text,
        confidence: Some(1.0),
        processing_time_ms,
    }))
}

/// Detect meeting intent in user's transcribed text
fn detect_meeting_intent(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();

    // Start meeting patterns
    let start_patterns = [
        "start a meeting",
        "starting a meeting",
        "start the meeting",
        "begin a meeting",
        "begin the meeting",
        "we're starting a meeting",
        "were starting a meeting",
        "let's start a meeting",
        "lets start a meeting",
        "start meeting mode",
        "enter meeting mode",
        "meeting mode",
        "transcribe the meeting",
        "transcribe this meeting",
        "take meeting notes",
        "record this meeting",
        "record the meeting",
        "only speak when spoken to",
    ];

    for pattern in &start_patterns {
        if lower.contains(pattern) {
            return Some("start_meeting");
        }
    }

    // End meeting patterns
    let end_patterns = [
        "end the meeting",
        "stop the meeting",
        "meeting is over",
        "meeting's over",
        "end meeting mode",
        "exit meeting mode",
        "stop recording the meeting",
    ];

    for pattern in &end_patterns {
        if lower.contains(pattern) {
            return Some("end_meeting");
        }
    }

    None
}

/// Resolve user's home project (first project they have access to)
async fn resolve_user_project(pool: &sqlx::SqlitePool, user_id: &str) -> Option<String> {
    // Try to find user's home project first, then fall back to first project membership
    #[derive(sqlx::FromRow)]
    struct ProjectRow {
        id: Vec<u8>,
    }

    // Try projects the user owns/is a member of
    let result = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT p.id FROM projects p
        JOIN project_members pm ON pm.project_id = p.id
        WHERE pm.user_id = ?1
        ORDER BY p.created_at ASC
        LIMIT 1
        "#,
    )
    .bind(user_id.as_bytes())
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(row) = result {
        if let Ok(uuid) = Uuid::from_slice(&row.id) {
            return Some(uuid.to_string());
        }
    }

    // Fall back to first project in DB
    let result = sqlx::query_as::<_, ProjectRow>(
        r#"SELECT id FROM projects ORDER BY created_at ASC LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(row) = result {
        if let Ok(uuid) = Uuid::from_slice(&row.id) {
            return Some(uuid.to_string());
        }
    }

    None
}

/// Handle full voice interaction: transcribe -> process with Topsi -> synthesize response
pub async fn voice_interaction(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<TopsiVoiceInteraction>,
) -> Result<Json<TopsiVoiceInteraction>, ApiError> {
    let start = std::time::Instant::now();

    let engine_lock = get_or_init_voice_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Voice engine not initialized".to_string()))?;

    let mut result = request.clone();

    // Step 1: Transcribe audio input if present
    let input_text = if let Some(audio_data) = &request.audio_input {
        let transcription = engine
            .transcribe_speech(audio_data)
            .await
            .map_err(|e| ApiError::InternalError(format!("Transcription failed: {}", e)))?;
        result.transcription = Some(transcription.clone());
        transcription
    } else if let Some(text) = &request.text_input {
        text.clone()
    } else {
        return Err(ApiError::BadRequest("No audio or text input provided".to_string()));
    };

    // Step 1.5: Check for meeting intent BEFORE sending to Topsi chat
    let meeting_intent = detect_meeting_intent(&input_text);

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;
    let pool = state.db().pool.clone();

    if let Some(intent) = meeting_intent {
        let topsi_instance = get_topsi_instance().await?;
        let instance = topsi_instance.read().await;
        let topsi = instance
            .as_ref()
            .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

        if !topsi.is_active().await {
            return Err(ApiError::BadRequest("Topsi is not active".to_string()));
        }

        match intent {
            "start_meeting" => {
                // Resolve a project for this user
                let project_id = resolve_user_project(&pool, &user_context.user_id)
                    .await
                    .unwrap_or_else(|| "default".to_string());

                // Start meeting via Topsi
                let topsi_request = TopsiRequest::new(TopsiRequestType::StartMeeting {
                    project_id: project_id.clone(),
                    title: Some("Voice-initiated meeting".to_string()),
                });

                let response = topsi
                    .process_request(topsi_request, &user_context, None)
                    .await
                    .map_err(|e| ApiError::InternalError(format!("Failed to start meeting: {}", e)))?;

                // Extract session_id from response
                let response_json: serde_json::Value = serde_json::from_str(&response.message)
                    .unwrap_or_else(|_| serde_json::json!({}));
                let session_id = response_json["session_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let spoken_response = "Meeting mode activated. I'll be listening silently and taking notes. Just say my name if you need me.";
                result.response_text = Some(spoken_response.to_string());
                result.action = Some("start_meeting".to_string());
                result.meeting_session_id = Some(session_id);
                result.action_project_id = Some(project_id);

                // Synthesize the spoken response
                let audio_response = engine
                    .synthesize_speech(spoken_response)
                    .await
                    .map_err(|e| ApiError::InternalError(format!("Speech synthesis failed: {}", e)))?;
                result.audio_response = Some(audio_response);
            }
            "end_meeting" => {
                // Check if there's an active meeting for this user
                let active_meetings = db::models::meeting_session::MeetingSession::find_active_by_project(
                    &pool,
                    &resolve_user_project(&pool, &user_context.user_id).await.unwrap_or_default(),
                )
                .await
                .unwrap_or_default();

                if let Some(active) = active_meetings.first() {
                    let topsi_request = TopsiRequest::new(TopsiRequestType::EndMeeting {
                        session_id: active.id.clone(),
                        generate_notes: true,
                    });

                    let _response = topsi
                        .process_request(topsi_request, &user_context, None)
                        .await
                        .map_err(|e| ApiError::InternalError(format!("Failed to end meeting: {}", e)))?;

                    let spoken_response = "Meeting ended. I've generated notes from the transcript.";
                    result.response_text = Some(spoken_response.to_string());
                    result.action = Some("end_meeting".to_string());
                    result.meeting_session_id = Some(active.id.clone());

                    let audio_response = engine
                        .synthesize_speech(spoken_response)
                        .await
                        .map_err(|e| ApiError::InternalError(format!("Speech synthesis failed: {}", e)))?;
                    result.audio_response = Some(audio_response);
                } else {
                    let spoken_response = "There's no active meeting to end.";
                    result.response_text = Some(spoken_response.to_string());

                    let audio_response = engine
                        .synthesize_speech(spoken_response)
                        .await
                        .map_err(|e| ApiError::InternalError(format!("Speech synthesis failed: {}", e)))?;
                    result.audio_response = Some(audio_response);
                }
            }
            _ => {}
        }

        result.processing_time_ms = Some(start.elapsed().as_millis() as u64);
        result.timestamp = Some(Utc::now());
        return Ok(Json(result));
    }

    // Step 2: Normal Topsi chat processing (no meeting intent detected)
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::Chat {
        message: input_text,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Topsi processing failed: {}", e)))?;

    result.response_text = Some(response.message.clone());

    // Step 3: Synthesize audio response
    // Sanitize the response text to remove JSON, tool output, and special chars
    let tts_text = sanitize_text_for_tts(&response.message);

    if tts_text.is_empty() {
        // If sanitization removed everything, use a default response
        tracing::warn!("TTS text was empty after sanitization, using fallback");
        result.audio_response = None;
    } else {
        tracing::info!("Synthesizing TTS for {} chars (sanitized from {})",
            tts_text.len(), response.message.len());

        let audio_response = engine
            .synthesize_speech(&tts_text)
            .await
            .map_err(|e| ApiError::InternalError(format!("Speech synthesis failed: {}", e)))?;

        result.audio_response = Some(audio_response);
    }

    result.processing_time_ms = Some(start.elapsed().as_millis() as u64);
    result.timestamp = Some(Utc::now());

    Ok(Json(result))
}

/// Get current voice configuration
pub async fn get_voice_config(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<TopsiVoiceConfigResponse>, ApiError> {
    let engine_lock = get_or_init_voice_engine().await?;
    let engine_guard = engine_lock.read().await;
    let is_ready = engine_guard.is_some();

    Ok(Json(TopsiVoiceConfigResponse {
        tts_provider: "system".to_string(), // Using SystemTTS (Chatterbox)
        stt_provider: "whisper".to_string(),
        voice_profile: "british_executive_female".to_string(),
        is_ready,
    }))
}

/// Update voice configuration
pub async fn update_voice_config(
    State(_state): State<DeploymentImpl>,
    Json(request): Json<UpdateTopsiVoiceConfigRequest>,
) -> Result<Json<TopsiVoiceConfigResponse>, ApiError> {
    tracing::info!("Updating Topsi voice config: {:?}", request);

    // For now, just return current config
    // Full config update would require reinitializing the engine
    Ok(Json(TopsiVoiceConfigResponse {
        tts_provider: request.tts_provider.unwrap_or_else(|| "system".to_string()),
        stt_provider: request.stt_provider.unwrap_or_else(|| "whisper".to_string()),
        voice_profile: request.voice_profile.unwrap_or_else(|| "british_executive_female".to_string()),
        is_ready: true,
    }))
}

// ============================================================================
// Meeting Mode Handlers
// ============================================================================

/// Helper: get accessible project IDs (as lowercase hex, 32 chars) for a user
/// Returns None for admins (all projects accessible).
async fn get_accessible_project_hex_ids(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    is_admin: bool,
) -> Option<std::collections::HashSet<String>> {
    if is_admin {
        return None;
    }
    let uid = match uuid::Uuid::parse_str(user_id) {
        Ok(u) => u,
        Err(_) => return Some(std::collections::HashSet::new()),
    };
    // project_members.project_id is BLOB; compare via hex
    let ids: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?1",
    )
    .bind(uid.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Some(
        ids.into_iter()
            .map(|b| hex::encode(&b))
            .collect(),
    )
}

/// Helper: check if a project_id (UUID text with dashes) is accessible given a hex-id set
fn project_is_accessible(
    project_id_str: &str,
    accessible: &Option<std::collections::HashSet<String>>,
) -> bool {
    match accessible {
        None => true, // admin
        Some(set) => {
            let hex = project_id_str.replace('-', "").to_lowercase();
            set.contains(&hex)
        }
    }
}

/// List meeting sessions — scoped to user's accessible projects
pub async fn list_meetings(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Query(params): Query<ListMeetingsQuery>,
) -> Result<Json<ListMeetingsResponse>, ApiError> {
    let pool = &state.db().pool;
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_ctx = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let accessible = get_accessible_project_hex_ids(pool, &user_ctx.user_id, user_ctx.is_admin).await;

    let sessions = db::models::meeting_session::MeetingSession::list(
        pool,
        params.project_id.as_deref(),
        limit,
        offset,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to list meetings: {}", e)))?;

    let status_filter = params.status.as_deref();

    // Build a project_id → (project_name, org_name) lookup via a single query
    #[derive(sqlx::FromRow)]
    struct ProjectRow {
        id_hex: String,
        name: String,
    }
    let project_rows: Vec<ProjectRow> = sqlx::query_as(
        "SELECT lower(hex(id)) as id_hex, name FROM projects WHERE deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let project_map: std::collections::HashMap<String, String> = project_rows
        .into_iter()
        .map(|r| (r.id_hex, r.name))
        .collect();

    let meetings: Vec<MeetingSessionSummary> = sessions
        .into_iter()
        .filter(|s| status_filter.map_or(true, |f| s.status == f))
        .filter(|s| project_is_accessible(&s.project_id, &accessible))
        .map(|s| {
            let notes_value = s
                .notes
                .as_ref()
                .and_then(|n| serde_json::from_str::<serde_json::Value>(n).ok());

            let hex = s.project_id.replace('-', "").to_lowercase();
            let project_name = project_map.get(&hex).cloned();

            MeetingSessionSummary {
                id: s.id,
                project_id: s.project_id,
                project_name,
                org_name: None, // org→project link not in live DB yet
                title: s.title,
                status: s.status,
                started_by: s.started_by,
                started_at: s.started_at,
                ended_at: s.ended_at,
                duration_seconds: s.duration_seconds,
                participant_count: s.participant_count,
                notes: notes_value,
            }
        })
        .collect();

    let total = meetings.len();
    Ok(Json(ListMeetingsResponse { meetings, total }))
}

/// Join an existing active meeting session (increments participant count)
pub async fn join_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<JoinMeetingRequest>,
) -> Result<Json<JoinMeetingResponse>, ApiError> {
    let pool = &state.db().pool;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_ctx = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(pool, &request.session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", request.session_id)))?;

    if session.status != "active" {
        return Err(ApiError::BadRequest("Meeting is not active".to_string()));
    }

    // Enforce: user must be a project member (or admin) to join
    if !user_ctx.is_admin {
        let accessible = get_accessible_project_hex_ids(pool, &user_ctx.user_id, false).await;
        if !project_is_accessible(&session.project_id, &accessible) {
            return Err(ApiError::Forbidden(
                "You are not a member of this project. Ask the meeting host to add you.".to_string(),
            ));
        }
    }

    let new_count = session.participant_count.unwrap_or(0) + 1;
    let updated = db::models::meeting_session::MeetingSession::update(
        pool,
        &session.id,
        db::models::meeting_session::UpdateMeetingSession {
            participant_count: Some(new_count),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Look up project name for the response
    let project_name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM projects WHERE lower(hex(id)) = lower(replace(?1, '-', '')) AND deleted_at IS NULL",
    )
    .bind(&session.project_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    tracing::info!(
        "[MEETING] User {} joined session {} (project: {}) — participants: {}",
        user_ctx.user_id, session.id, session.project_id, new_count
    );

    Ok(Json(JoinMeetingResponse {
        session_id: updated.id,
        title: updated.title,
        project_id: updated.project_id,
        participant_count: updated.participant_count.unwrap_or(new_count),
    }))
}

/// Add a typed text message or link to an active meeting without audio
pub async fn meeting_text_message(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<MeetingMessageRequest>,
) -> Result<Json<MeetingMessageResponse>, ApiError> {
    let pool = &state.db().pool;

    let session = db::models::meeting_session::MeetingSession::find_by_id(pool, &request.session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", request.session_id)))?;

    if session.status != "active" {
        return Err(ApiError::BadRequest("Meeting is not active".to_string()));
    }

    let count = db::models::meeting_session::MeetingSegment::count_by_session(pool, &request.session_id)
        .await
        .unwrap_or(0);

    let text = if request.is_link.unwrap_or(false) {
        format!("[SHARED LINK] {}", request.text)
    } else {
        request.text.clone()
    };

    let segment = db::models::meeting_session::MeetingSegment::create(
        pool,
        db::models::meeting_session::CreateMeetingSegment {
            meeting_session_id: request.session_id.clone(),
            segment_index: count as i32,
            speaker_label: request.speaker_label.clone(),
            text: text.clone(),
            confidence: Some(1.0),
            start_time_ms: 0,
            end_time_ms: 0,
            is_topsi_addressed: false,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let speaker = request.speaker_label.as_deref().unwrap_or("participant");
    tracing::info!("[MEETING] Text message from {} in session {}: {}", speaker, request.session_id, &text[..text.len().min(80)]);

    Ok(Json(MeetingMessageResponse {
        session_id: request.session_id,
        segment_index: segment.segment_index,
        text,
    }))
}

/// Heartbeat — bumps updated_at so stale-session cleanup doesn't end this meeting
pub async fn heartbeat_meeting(
    State(state): State<DeploymentImpl>,
    Json(request): Json<MeetingHeartbeatRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();
    db::models::meeting_session::MeetingSession::heartbeat(&pool, &request.session_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Heartbeat failed: {}", e)))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

/// Start a new meeting session
pub async fn start_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<StartMeetingRequest>,
) -> Result<Json<StartMeetingResponse>, ApiError> {
    tracing::info!("Starting meeting for project: {}", request.project_id);

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::StartMeeting {
        project_id: request.project_id,
        title: request.title,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to start meeting: {}", e)))?;

    // Parse the response message as JSON to extract fields
    let response_json: serde_json::Value = serde_json::from_str(&response.message)
        .unwrap_or_else(|_| serde_json::json!({"message": response.message}));

    Ok(Json(StartMeetingResponse {
        session_id: response_json["session_id"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        title: response_json["title"]
            .as_str()
            .unwrap_or("Untitled Meeting")
            .to_string(),
        status: "active".to_string(),
        started_at: response_json["started_at"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        message: response_json["message"]
            .as_str()
            .unwrap_or("Meeting started")
            .to_string(),
    }))
}

/// Process an audio chunk from a meeting
pub async fn meeting_audio_chunk(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<MeetingAudioChunkRequest>,
) -> Result<Json<MeetingAudioChunkResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Step 1: Transcribe the audio using voice engine
    let transcribed_text = {
        let engine_result = get_or_init_voice_engine().await;
        if let Ok(engine_lock) = engine_result {
            let engine_guard = engine_lock.read().await;
            if let Some(engine) = engine_guard.as_ref() {
                match engine.transcribe_speech(&request.audio_data).await {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::warn!("Meeting transcription failed, using empty: {}", e);
                        String::new()
                    }
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    };

    if transcribed_text.is_empty() {
        // Return acknowledgement for empty/silent chunks
        return Ok(Json(MeetingAudioChunkResponse {
            session_id: request.session_id,
            chunk_index: request.chunk_index,
            text: String::new(),
            speaker_label: None,
            is_topsi_addressed: false,
            topsi_response: None,
            topsi_audio_response: None,
            segment_index: request.chunk_index as i32,
        }));
    }

    // Step 2: Process through Topsi agent (stores segment, detects wake word)
    let topsi_request = TopsiRequest::new(TopsiRequestType::MeetingAudioChunk {
        session_id: request.session_id.clone(),
        audio_data: transcribed_text.clone(),
        chunk_index: request.chunk_index,
        duration_ms: request.duration_ms,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Meeting audio processing failed: {}", e)))?;

    // Parse response
    let response_json: serde_json::Value = serde_json::from_str(&response.message)
        .unwrap_or_else(|_| serde_json::json!({}));

    let is_addressed = response_json["is_topsi_addressed"].as_bool().unwrap_or(false);
    let topsi_response_text = response_json["topsi_response"].as_str().map(|s| s.to_string());

    // Step 3: If Topsi responded, synthesize audio response
    let topsi_audio = if let Some(ref response_text) = topsi_response_text {
        let engine_result = get_or_init_voice_engine().await;
        if let Ok(engine_lock) = engine_result {
            let engine_guard = engine_lock.read().await;
            if let Some(engine) = engine_guard.as_ref() {
                let tts_text = sanitize_text_for_tts(response_text);
                if !tts_text.is_empty() {
                    match engine.synthesize_speech(&tts_text).await {
                        Ok(audio) => Some(audio),
                        Err(e) => {
                            tracing::warn!("Failed to synthesize meeting response: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(MeetingAudioChunkResponse {
        session_id: request.session_id,
        chunk_index: request.chunk_index,
        text: transcribed_text,
        speaker_label: response_json["speaker_label"]
            .as_str()
            .map(|s| s.to_string()),
        is_topsi_addressed: is_addressed,
        topsi_response: topsi_response_text,
        topsi_audio_response: topsi_audio,
        segment_index: response_json["segment_index"].as_i64().unwrap_or(0) as i32,
    }))
}

/// End a meeting session
pub async fn end_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<EndMeetingRequest>,
) -> Result<Json<EndMeetingResponse>, ApiError> {
    tracing::info!("Ending meeting: {}", request.session_id);

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::EndMeeting {
        session_id: request.session_id.clone(),
        generate_notes: request.generate_notes.unwrap_or(true),
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to end meeting: {}", e)))?;

    let response_json: serde_json::Value = serde_json::from_str(&response.message)
        .unwrap_or_else(|_| serde_json::json!({}));

    let notes: Option<topsi::MeetingNotes> = response_json
        .get("notes")
        .and_then(|n| serde_json::from_value(n.clone()).ok());

    Ok(Json(EndMeetingResponse {
        session_id: request.session_id,
        status: "ended".to_string(),
        duration_seconds: response_json["duration_seconds"].as_i64().unwrap_or(0) as i32,
        participant_count: response_json["participant_count"].as_i64().unwrap_or(0) as i32,
        segment_count: response_json["segment_count"].as_u64().unwrap_or(0) as usize,
        notes,
    }))
}

/// Get meeting status
pub async fn meeting_status(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<MeetingStatusResponse>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    // Access control — owner/admin/shared OR any project member
    let has_access = session.has_access(&user_context.user_id, user_context.is_admin)
        || db::models::meeting_session::MeetingSession::is_project_member(
            &pool,
            &session_id,
            &user_context.user_id,
        )
        .await;
    if !has_access {
        return Err(ApiError::Forbidden("Access denied to this meeting".to_string()));
    }

    let segment_count =
        db::models::meeting_session::MeetingSegment::count_by_session(&pool, &session_id)
            .await
            .unwrap_or(0);

    let is_active = session.status == "active";

    Ok(Json(MeetingStatusResponse {
        session_id: session.id,
        title: session.title,
        status: session.status,
        started_at: session.started_at,
        duration_seconds: session.duration_seconds,
        participant_count: session.participant_count.unwrap_or(0),
        segment_count,
        is_active,
    }))
}

/// Get meeting notes
pub async fn get_meeting_notes(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    let has_access = session.has_access(&user_context.user_id, user_context.is_admin)
        || db::models::meeting_session::MeetingSession::is_project_member(
            &pool,
            &session_id,
            &user_context.user_id,
        )
        .await;
    if !has_access {
        return Err(ApiError::Forbidden("Access denied to this meeting".to_string()));
    }

    let notes = session.notes.and_then(|n| serde_json::from_str::<serde_json::Value>(&n).ok());

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "notes": notes,
    })))
}

/// Regenerate AI notes for an ended meeting from its stored DB segments
pub async fn regenerate_meeting_notes(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Only admins or session starters can trigger regeneration
    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;
    if !session.has_access(&user_context.user_id, user_context.is_admin) {
        return Err(ApiError::Forbidden("Access denied".to_string()));
    }

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let notes = topsi
        .handle_regenerate_meeting_notes(&session_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Note generation failed: {e}")))?;

    let notes_json = serde_json::to_string(&notes).unwrap_or_default();

    // Persist to DB
    let _ = db::models::meeting_session::MeetingSession::update(
        &pool,
        &session_id,
        db::models::meeting_session::UpdateMeetingSession {
            notes: Some(notes_json),
            ..Default::default()
        },
    )
    .await;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "notes": notes,
    })))
}

/// Get meeting transcript
pub async fn get_meeting_transcript(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<MeetingTranscriptResponse>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    let has_access = session.has_access(&user_context.user_id, user_context.is_admin)
        || db::models::meeting_session::MeetingSession::is_project_member(
            &pool,
            &session_id,
            &user_context.user_id,
        )
        .await;
    if !has_access {
        return Err(ApiError::Forbidden("Access denied to this meeting".to_string()));
    }

    let segments =
        db::models::meeting_session::MeetingSegment::find_by_session(&pool, &session_id)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to fetch transcript: {}", e))
            })?;

    let total_count = segments.len() as i64;

    Ok(Json(MeetingTranscriptResponse {
        session_id,
        segments,
        total_count,
    }))
}

// ============================================================================
// Recording upload / transcript import / person suggestion + linking
// ============================================================================

/// POST /api/topsi/meeting/upload — multipart audio file → transcribe → session + notes
pub async fn upload_meeting_recording(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_ctx = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut project_id_str = String::new();
    let mut title = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::BadRequest(e.to_string()))? {
        match field.name() {
            Some("audio") => {
                let data = field.bytes().await.map_err(|e| ApiError::BadRequest(e.to_string()))?;
                audio_bytes = Some(data.to_vec());
            }
            Some("project_id") => {
                project_id_str = field.text().await.unwrap_or_default();
            }
            Some("title") => {
                title = field.text().await.unwrap_or_default();
            }
            _ => {}
        }
    }

    let audio_bytes = audio_bytes.ok_or_else(|| ApiError::BadRequest("Missing 'audio' field".into()))?;
    if project_id_str.is_empty() {
        return Err(ApiError::BadRequest("Missing 'project_id' field".into()));
    }

    // Verify project exists
    let project_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE lower(hex(id)) = lower(replace(?1,'-','')) AND deleted_at IS NULL",
    )
    .bind(&project_id_str)
    .fetch_one(&pool)
    .await
    .map(|c: i64| c > 0)
    .unwrap_or(false);
    if !project_exists {
        return Err(ApiError::NotFound(format!("Project not found: {}", project_id_str)));
    }

    // Transcribe via voice engine
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &audio_bytes);
    let engine_lock = get_or_init_voice_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Voice engine not initialized".into()))?;

    let transcript_text = engine
        .transcribe_speech(&b64)
        .await
        .map_err(|e| ApiError::InternalError(format!("Transcription failed: {}", e)))?;
    drop(engine_guard);

    ingest_transcript_text(&state, &pool, &user_ctx, &project_id_str, &title, &transcript_text, "upload").await
}

/// POST /api/topsi/meeting/upload-transcript — JSON transcript paste → session + notes
#[derive(serde::Deserialize)]
pub struct UploadTranscriptRequest {
    pub project_id: String,
    pub title: Option<String>,
    pub transcript_text: String,
    pub source: Option<String>,
}

pub async fn upload_meeting_transcript(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(body): Json<UploadTranscriptRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_ctx = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let project_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE lower(hex(id)) = lower(replace(?1,'-','')) AND deleted_at IS NULL",
    )
    .bind(&body.project_id)
    .fetch_one(&pool)
    .await
    .map(|c: i64| c > 0)
    .unwrap_or(false);
    if !project_exists {
        return Err(ApiError::NotFound(format!("Project not found: {}", body.project_id)));
    }

    let source = body.source.as_deref().unwrap_or("manual");
    let title = body.title.as_deref().unwrap_or("Imported Transcript");

    ingest_transcript_text(&state, &pool, &user_ctx, &body.project_id, title, &body.transcript_text, source).await
}

/// Shared helper: takes a raw transcript string, creates a MeetingSession + segments + notes.
async fn ingest_transcript_text(
    state: &DeploymentImpl,
    pool: &sqlx::SqlitePool,
    user_ctx: &topsi::UserContext,
    project_id: &str,
    title: &str,
    transcript_text: &str,
    started_by: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    use db::models::meeting_session::{CreateMeetingSession, CreateMeetingSegment, MeetingSegment, UpdateMeetingSession};
    use db::models::project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource};

    let display_title = if title.is_empty() { "Imported Recording" } else { title };

    // Create session
    let session = db::models::meeting_session::MeetingSession::create(
        pool,
        CreateMeetingSession {
            project_id: project_id.to_string(),
            title: Some(display_title.to_string()),
            started_by: started_by.to_string(),
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // Split into sentences
    let sentences: Vec<&str> = transcript_text
        .split(|c| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let segment_count = sentences.len();
    for (idx, sentence) in sentences.iter().enumerate() {
        let _ = MeetingSegment::create(
            pool,
            CreateMeetingSegment {
                meeting_session_id: session.id.clone(),
                segment_index: idx as i32,
                speaker_label: None,
                text: sentence.to_string(),
                confidence: Some(1.0),
                start_time_ms: (idx as i64) * 5000,
                end_time_ms: (idx as i64 + 1) * 5000,
                is_topsi_addressed: false,
            },
        )
        .await;
    }

    // Mark as ended
    let estimated_duration = (segment_count as i32).saturating_mul(5);
    let _ = db::models::meeting_session::MeetingSession::update(
        pool,
        &session.id,
        UpdateMeetingSession {
            status: Some(db::models::meeting_session::MeetingStatus::Ended),
            ended_at: Some(chrono::Utc::now().to_rfc3339()),
            duration_seconds: Some(estimated_duration),
            participant_count: Some(1),
            ..Default::default()
        },
    )
    .await;

    // Generate notes via Topsi
    let notes_json_str = if let Ok(topsi_arc) = get_topsi_instance().await {
        let guard = topsi_arc.read().await;
        if let Some(topsi) = guard.as_ref() {
            match topsi.handle_regenerate_meeting_notes(&session.id).await {
                Ok(notes) => {
                    let s = serde_json::to_string(&notes).unwrap_or_default();
                    let _ = db::models::meeting_session::MeetingSession::update(
                        pool,
                        &session.id,
                        UpdateMeetingSession {
                            notes: Some(s.clone()),
                            ..Default::default()
                        },
                    )
                    .await;
                    Some(s)
                }
                Err(e) => {
                    tracing::warn!("Note generation failed for uploaded session: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Register in knowledge graph
    if let Ok(pid) = uuid::Uuid::parse_str(project_id) {
        let summary = notes_json_str.as_deref().unwrap_or(transcript_text);
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Conversation,
            &session.id,
            display_title,
            Some(&summary[..summary.len().min(500)]),
            0.85,
        )
        .await;
    }

    let notes_value: Option<serde_json::Value> = notes_json_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(Json(serde_json::json!({
        "session_id": session.id,
        "title": display_title,
        "notes": notes_value,
        "segment_count": segment_count,
    })))
}

/// GET /api/topsi/meeting/suggest-persons/:session_id
///
/// Uses Topsi LLM to identify CRM persons mentioned in the meeting transcript/notes.
pub async fn suggest_meeting_persons(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_ctx = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Session not found: {}", session_id)))?;

    // Build transcript excerpt
    let segments = db::models::meeting_session::MeetingSegment::find_by_session(&pool, &session_id)
        .await
        .unwrap_or_default();
    let transcript_excerpt: String = segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let transcript_excerpt = &transcript_excerpt[..transcript_excerpt.len().min(3000)];

    let notes_summary = session.notes.as_deref().unwrap_or("").to_string();

    // Load CRM persons
    #[derive(sqlx::FromRow)]
    struct PersonRow {
        id: String,
        full_name: String,
        email: Option<String>,
        person_type: String,
    }

    let persons: Vec<PersonRow> = sqlx::query_as(
        "SELECT id, full_name, email, person_type FROM persons ORDER BY full_name LIMIT 200",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    if persons.is_empty() {
        return Ok(Json(serde_json::json!({ "suggestions": [] })));
    }

    let persons_list = persons
        .iter()
        .map(|p| format!("{{\"id\":\"{}\",\"name\":\"{}\",\"type\":\"{}\"}}", p.id, p.full_name, p.person_type))
        .collect::<Vec<_>>()
        .join(",");

    let prompt = format!(
        "Meeting title: {title}\n\
         Meeting notes summary: {notes}\n\
         Transcript excerpt: {transcript}\n\n\
         CRM persons list: [{persons}]\n\n\
         From this meeting transcript and notes, identify any people that match from the CRM persons list.\n\
         Return ONLY a valid JSON array (no markdown, no explanation) in this exact format:\n\
         [{{\"person_id\":\"...\",\"full_name\":\"...\",\"confidence\":0.0,\"reason\":\"...\"}}]\n\
         Only include matches with confidence > 0.4. Return [] if none match.",
        title = session.title,
        notes = &notes_summary[..notes_summary.len().min(500)],
        transcript = transcript_excerpt,
        persons = persons_list,
    );

    // Use Topsi LLM
    let topsi_arc = get_topsi_instance().await?;
    let guard = topsi_arc.read().await;
    let topsi = guard
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".into()))?;

    let req = topsi::TopsiRequest::new(topsi::TopsiRequestType::Chat { message: prompt });
    let response = topsi
        .process_request(req, &user_ctx, Some(&session_id))
        .await
        .map_err(|e| ApiError::InternalError(format!("Topsi error: {}", e)))?;
    drop(guard);

    // Extract JSON array from response
    let text = &response.message;
    let suggestions: serde_json::Value = {
        // Try to find a JSON array in the response
        let start = text.find('[').unwrap_or(0);
        let end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        if start < end {
            serde_json::from_str(&text[start..end]).unwrap_or(serde_json::json!([]))
        } else {
            serde_json::json!([])
        }
    };

    // Sort by confidence desc
    let mut arr = suggestions.as_array().cloned().unwrap_or_default();
    arr.sort_by(|a, b| {
        let ca = a.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let cb = b.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(Json(serde_json::json!({ "suggestions": arr })))
}

/// POST /api/topsi/meeting/link-persons/:session_id
///
/// Links CRM persons to the session and enriches their intelligence profiles with
/// meeting context.
#[derive(serde::Deserialize)]
pub struct LinkPersonsRequest {
    pub person_ids: Vec<String>,
}

pub async fn link_meeting_persons(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
    Json(body): Json<LinkPersonsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Session not found: {}", session_id)))?;

    // Build transcript excerpt for context
    let segments = db::models::meeting_session::MeetingSegment::find_by_session(&pool, &session_id)
        .await
        .unwrap_or_default();
    let transcript_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
    let transcript_excerpt = &transcript_text[..transcript_text.len().min(500)];

    // Parse notes summary
    let notes_summary = session.notes.as_deref().unwrap_or("").to_string();
    let notes_parsed: Option<serde_json::Value> = serde_json::from_str(&notes_summary).ok();
    let summary_text = notes_parsed
        .as_ref()
        .and_then(|n| n.get("summary").and_then(|s| s.as_str()))
        .unwrap_or(transcript_excerpt);
    let topics_text = notes_parsed
        .as_ref()
        .and_then(|n| n.get("topics").and_then(|t| t.as_array()))
        .map(|ts| ts.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let actions_text = notes_parsed
        .as_ref()
        .and_then(|n| n.get("action_items").and_then(|a| a.as_array()))
        .map(|acts| acts.iter().filter_map(|a| a.get("description").and_then(|d| d.as_str())).collect::<Vec<_>>().join("; "))
        .unwrap_or_default();

    // Get project UUID from session
    let project_uuid = uuid::Uuid::parse_str(&session.project_id).ok();

    // Merge existing linked_person_ids with new ones
    let mut existing_ids: Vec<String> = serde_json::from_str(&session.linked_person_ids).unwrap_or_default();
    for pid in &body.person_ids {
        if !existing_ids.contains(pid) {
            existing_ids.push(pid.clone());
        }
    }

    // Save updated linked_person_ids
    let _ = db::models::meeting_session::MeetingSession::link_persons(&pool, &session_id, &existing_ids).await;

    let mut enriched_count = 0usize;

    for person_id_str in &body.person_ids {
        let person_uuid = match uuid::Uuid::parse_str(person_id_str) {
            Ok(u) => u,
            Err(_) => continue,
        };

        #[derive(sqlx::FromRow)]
        struct PersonRow {
            full_name: String,
            intelligence_raw: Option<String>,
        }

        let person: Option<PersonRow> = sqlx::query_as(
            "SELECT full_name, intelligence_raw FROM persons WHERE id = ?",
        )
        .bind(person_uuid)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        let Some(person) = person else { continue };

        let enrichment_append = format!(
            "\n\n[Meeting: {} on {}]\nSummary: {}\nTopics: {}\nAction items: {}\nContext: {}",
            session.title,
            &session.started_at[..10.min(session.started_at.len())],
            summary_text,
            topics_text,
            actions_text,
            transcript_excerpt,
        );

        let new_raw = format!(
            "{}{}",
            person.intelligence_raw.as_deref().unwrap_or(""),
            enrichment_append
        );

        let result = crate::routes::intelligence::write_intelligence_results(
            &pool,
            person_uuid,
            summary_text,
            0.75,
            &new_raw,
            project_uuid,
            &person.full_name,
        )
        .await;

        if result.is_ok() {
            enriched_count += 1;
        } else {
            tracing::warn!("Failed to enrich person {} from meeting {}", person_id_str, session_id);
        }
    }

    Ok(Json(serde_json::json!({
        "linked": body.person_ids.len(),
        "enriched": enriched_count,
        "linked_person_ids": existing_ids,
    })))
}

/// Share a meeting with other users
pub async fn share_meeting(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
    Json(request): Json<ShareMeetingRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db().pool.clone();

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session = db::models::meeting_session::MeetingSession::find_by_id(&pool, &session_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Meeting session not found: {}", session_id)))?;

    // Only admin or meeting starter can share
    if !user_context.is_admin && session.started_by != user_context.user_id {
        return Err(ApiError::Forbidden(
            "Only the meeting creator or admin can share meetings".to_string(),
        ));
    }

    // Merge new user_ids with existing shared_with
    let mut shared_users: Vec<String> = session
        .shared_with
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    for user_id in &request.user_ids {
        if !shared_users.contains(user_id) {
            shared_users.push(user_id.clone());
        }
    }

    let shared_json = serde_json::to_string(&shared_users).unwrap_or_else(|_| "[]".to_string());

    db::models::meeting_session::MeetingSession::update(
        &pool,
        &session_id,
        db::models::meeting_session::UpdateMeetingSession {
            shared_with: Some(shared_json),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to update sharing: {}", e)))?;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "shared_with": shared_users,
        "message": format!("Meeting shared with {} users", request.user_ids.len()),
    })))
}

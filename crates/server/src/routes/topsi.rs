//! Topsi Platform Agent API routes
//!
//! Topsi is the platform orchestrator that replaces the default control agent
//! for user-facing interactions. It provides containerized access control
//! ensuring strict client data isolation.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use topsi::{
    TopsiAgent, TopsiConfig, TopsiError, TopsiRequest, TopsiRequestType, TopsiResponse,
    TopologySummary, DetectedIssue, UserContext, AccessScope, ProjectAccess,
    initialize_topsi,
};
use ts_rs::TS;
use uuid::Uuid;

use deployment::Deployment;

// Import voice types from Nora
use nora::voice::{
    VoiceConfig, VoiceEngine, SpeechRequest, SpeechResponse, AudioFormat,
    tts::VoiceProfile,
};

use crate::{DeploymentImpl, error::ApiError};

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
    ) -> Result<TopsiResponse, TopsiError> {
        let agent = self.agent.read().await;
        if let Some(topsi) = agent.as_ref() {
            topsi.process_request(request, user_context).await
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
        .route("/topsi/command", post(execute_command))
        // Voice routes for Topsi
        .route("/topsi/voice/synthesize", post(synthesize_speech))
        .route("/topsi/voice/transcribe", post(transcribe_speech))
        .route("/topsi/voice/interaction", post(voice_interaction))
        .route("/topsi/voice/config", get(get_voice_config).put(update_voice_config))
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

    let topsi_agent = initialize_topsi(config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to initialize Topsi: {}", e);
            ApiError::InternalError(format!("Topsi initialization failed: {}", e))
        })?
        .with_database(state.db().pool.clone());

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

    let topsi_agent = initialize_topsi(config)
        .await
        .map_err(|e| format!("Topsi initialization failed: {}", e))?
        .with_database(state.db().pool.clone());

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

        // Get user context from request (simplified - in production would extract from auth)
        let user_context = get_user_context_from_state(&state).await;
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
    Json(request): Json<TopsiChatRequest>,
) -> Result<Json<TopsiResponse>, ApiError> {
    tracing::info!("Received chat request: {:?}", request.message);

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    // Get user context
    let user_context = get_user_context_from_state(&state).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::Chat {
        message: request.message,
    });

    let response = topsi
        .process_request(topsi_request, &user_context)
        .await
        .map_err(|e| {
            tracing::error!("Topsi processing error: {}", e);
            ApiError::InternalError(format!("Topsi processing failed: {}", e))
        })?;

    Ok(Json(response))
}

/// Get topology overview (all accessible projects)
pub async fn get_topology_overview(
    State(state): State<DeploymentImpl>,
) -> Result<Json<TopologyOverviewResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let user_context = get_user_context_from_state(&state).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetTopology {
        project_id: None,
    });

    let response = topsi
        .process_request(topsi_request, &user_context)
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
    Path(project_id): Path<Uuid>,
) -> Result<Json<TopologyOverviewResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let user_context = get_user_context_from_state(&state).await;

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
        .process_request(topsi_request, &user_context)
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
) -> Result<Json<IssuesResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let user_context = get_user_context_from_state(&state).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::DetectIssues {
        project_id: None,
    });

    let response = topsi
        .process_request(topsi_request, &user_context)
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
    Path(project_id): Path<Uuid>,
) -> Result<Json<IssuesResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let user_context = get_user_context_from_state(&state).await;

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
        .process_request(topsi_request, &user_context)
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
) -> Result<Json<AccessibleProjectsResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let user_context = get_user_context_from_state(&state).await;

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

/// Execute a Topsi command
pub async fn execute_command(
    State(state): State<DeploymentImpl>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<TopsiResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let user_context = get_user_context_from_state(&state).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::ExecuteCommand {
        command: request.command,
    });

    let response = topsi
        .process_request(topsi_request, &user_context)
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
async fn get_user_context_from_state(_state: &DeploymentImpl) -> UserContext {
    // In production, this would extract user info from the request auth header
    // For now, create a default admin context for development
    // TODO: Integrate with actual auth system

    // Check if there's a logged-in user from the auth middleware
    // For now, default to admin for development
    UserContext::admin("dev-admin-user")
        .with_email("admin@pcg.local")
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
            let chatterbox_available = reqwest::Client::new()
                .get("http://localhost:8100/health")
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

/// Handle full voice interaction: transcribe -> process with Topsi -> synthesize response
pub async fn voice_interaction(
    State(state): State<DeploymentImpl>,
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

    // Step 2: Process with Topsi
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    let user_context = get_user_context_from_state(&state).await;
    let topsi_request = TopsiRequest::new(TopsiRequestType::Chat {
        message: input_text,
    });

    let response = topsi
        .process_request(topsi_request, &user_context)
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

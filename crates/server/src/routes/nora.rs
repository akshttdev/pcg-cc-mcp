//! Nora executive assistant API routes

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{State, WebSocketUpgrade},
    response::{Response, sse::{Event, Sse}},
    routing::{get, post},
};
use futures::stream::Stream;
use chrono::{DateTime, Utc};
use db::models::project::Project;
use deployment::Deployment;
use nora::{
    NoraAgent, NoraConfig, NoraError,
    agent::{NoraRequest, NoraRequestType, NoraResponse, RequestPriority},
    brain::LLMConfig,
    coordination::{AgentCoordinationState, CoordinationEvent, CoordinationStats},
    memory::{BudgetStatus, ProjectContext, ProjectStatus},
    personality::PersonalityConfig,
    tools::{NoraExecutiveTool, ToolExecutionResult},
    voice::{SpeechResponse, VoiceConfig, VoiceEngine, VoiceError, VoiceInteraction},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::rate_limit::TokenBucket};

/// Global Nora agent instance
static NORA_INSTANCE: tokio::sync::OnceCell<Arc<RwLock<Option<NoraAgent>>>> =
    tokio::sync::OnceCell::const_new();

/// Global Nora initialization timestamp
static NORA_INIT_TIME: tokio::sync::OnceCell<DateTime<Utc>> =
    tokio::sync::OnceCell::const_new();

/// Global rate limiter for chat endpoints (20 req/min, refill 1 per 3 seconds)
static CHAT_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

/// Global rate limiter for voice synthesis (30 req/min, refill 1 per 2 seconds)
static VOICE_RATE_LIMITER: tokio::sync::OnceCell<Arc<TokenBucket>> =
    tokio::sync::OnceCell::const_new();

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
        .route("/nora/tools/execute", post(execute_executive_tool))
        .route("/nora/tools/available", get(get_available_tools))
        .route("/nora/coordination/stats", get(get_coordination_stats))
        .route("/nora/coordination/agents", get(get_coordination_agents))
        .route("/nora/coordination/events", get(get_coordination_events_ws))
        .route("/nora/personality/config", get(get_personality_config))
        .route("/nora/personality/config", post(update_personality_config))
        .layer(axum::middleware::from_fn(crate::middleware::request_id_middleware))
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

    let projects = Project::find_all(&state.db().pool).await.map_err(|e| {
        tracing::error!("Failed to load projects for Nora context: {}", e);
        ApiError::InternalError(format!("Failed to load projects: {}", e))
    })?;

    let project_context = map_projects_to_context(projects);

    let nora_agent = nora::initialize_nora(config).await.map_err(|e| {
        tracing::error!("Failed to initialize Nora: {}", e);
        ApiError::InternalError(format!("Nora initialization failed: {}", e))
    })?;

    // Wire up database pool for task execution
    let nora_agent = nora_agent.with_database(state.db().pool.clone());

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
            "Rate limit exceeded. Please slow down your chat requests.".to_string()
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
            "Rate limit exceeded. Please slow down your chat requests.".to_string()
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
    let llm_client = nora.llm.clone().ok_or_else(|| {
        ApiError::InternalError("LLM not available".to_string())
    })?;
    
    // Prepare context
    let context = request.context
        .map(|c| serde_json::to_string_pretty(&c).unwrap_or_default())
        .unwrap_or_default();
    
    let message = request.message.clone();
    
    // Create stream
    let llm_stream = llm_client
        .generate_stream("", &message, &context)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create stream: {}", e)))?;
    
    // Convert to SSE stream
    let sse_stream = llm_stream.map(|chunk_result| {
        match chunk_result {
            Ok(chunk) => {
                tracing::debug!("Streaming chunk: {} chars", chunk.len());
                Ok(Event::default().data(chunk))
            }
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                Ok(Event::default().data(format!("[ERROR]: {}", e)))
            }
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
            "Rate limit exceeded. Please slow down your voice synthesis requests.".to_string()
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
    State(_state): State<DeploymentImpl>,
    Json(request): Json<VoiceInteraction>,
) -> Result<Json<VoiceInteraction>, ApiError> {
    // Apply rate limiting
    let rate_limiter = get_voice_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Voice interaction rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your voice interaction requests.".to_string()
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

    Ok(Json(processed_interaction))
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
    State(_state): State<DeploymentImpl>,
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

    nora.config.voice = new_config.clone();
    nora.voice_engine = Arc::new(new_engine);

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

/// WebSocket endpoint for coordination events
pub async fn get_coordination_events_ws(
    ws: WebSocketUpgrade,
    State(_state): State<DeploymentImpl>,
) -> Response {
    ws.on_upgrade(handle_coordination_events_websocket)
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

// Helper functions

async fn get_nora_instance() -> Result<Arc<RwLock<Option<NoraAgent>>>, ApiError> {
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
                        let payload = match event {
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
                        };

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

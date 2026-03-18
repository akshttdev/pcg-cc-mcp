//! Nora initialization handlers (HTTP + startup).

use std::sync::Arc;

use axum::{Json, extract::State};
use chrono::Utc;
use cinematics::{CinematicsConfig, CinematicsService};
use db::models::project::Project;
use nora::{NoraConfig, voice::{TTSConfig, VoiceConfig}};

use super::*;
use super::config::{apply_llm_overrides, default_capabilities};

/// Initialize Nora executive assistant (HTTP handler)
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

/// Initialize Nora on server startup (called automatically, non-HTTP).
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

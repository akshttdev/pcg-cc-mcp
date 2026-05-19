//! Nora voice handlers: synthesize, transcribe, voice interaction, analytics

use super::*;

/// Synthesize speech using Nora's voice
pub async fn synthesize_speech(
    State(state): State<DeploymentImpl>,
    Json(request): Json<VoiceSynthesisRequest>,
) -> Result<Json<SpeechResponse>, ApiError> {
    use services::services::editron::UsageTracker;

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
    let clean_text = crate::routes::twilio::strip_markdown_for_tts(&request.text);
    let char_count = clean_text.chars().count() as i64;
    let provider = nora.config.voice.tts.provider.as_str();

    let result = nora.voice_engine.synthesize_speech(&clean_text).await;

    let duration = start.elapsed().as_secs_f64();
    let duration_ms = (duration * 1000.0) as i64;

    // Log TTS usage
    let pool = &state.db().pool;
    match &result {
        Ok(_) => {
            crate::nora_metrics::record_tts_call(provider, "success", duration);
            UsageTracker::log_tts_operation(
                pool,
                provider,
                "synthesize",
                Some(char_count),
                None,
                duration_ms,
                true,
                None,
                None,
            )
            .await;
        }
        Err(e) => {
            crate::nora_metrics::record_tts_call(provider, "error", duration);
            let error_msg = e.to_string();
            UsageTracker::log_tts_operation(
                pool,
                provider,
                "synthesize",
                Some(char_count),
                None,
                duration_ms,
                false,
                Some(&error_msg),
                None,
            )
            .await;
        }
    }

    let audio_data = result.map_err(|e| {
        tracing::error!("Speech synthesis error: {}", e);
        ApiError::InternalError(format!("Speech synthesis failed: {}", e))
    })?;

    // Create a proper SpeechResponse
    let processing_time_ms = (duration * 1000.0) as u64;
    let response = SpeechResponse {
        audio_data,
        duration_ms: estimate_speech_duration(&clean_text),
        sample_rate: 22050, // Default for most TTS services
        format: nora::voice::AudioFormat::Mp3,
        processing_time_ms,
    };

    Ok(Json(response))
}

/// Transcribe speech using Nora's STT
pub async fn transcribe_speech(
    State(state): State<DeploymentImpl>,
    Json(request): Json<VoiceTranscriptionRequest>,
) -> Result<Json<VoiceTranscriptionResponse>, ApiError> {
    use services::services::editron::UsageTracker;

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let start = std::time::Instant::now();
    let provider = nora.config.voice.stt.provider.as_str();

    let result = nora
        .voice_engine
        .transcribe_speech(&request.audio_data)
        .await;

    let duration = start.elapsed().as_secs_f64();
    let duration_ms = (duration * 1000.0) as i64;

    // Log STT usage
    let pool = &state.db().pool;
    match &result {
        Ok(transcription) => {
            crate::nora_metrics::record_stt_call(provider, "success", duration);
            UsageTracker::log_stt_operation(
                pool,
                provider,
                "transcribe",
                None, // audio duration not easily available
                Some(transcription.chars().count() as i64),
                duration_ms,
                true,
                None,
                None,
            )
            .await;
        }
        Err(e) => {
            crate::nora_metrics::record_stt_call(provider, "error", duration);
            let error_msg = e.to_string();
            UsageTracker::log_stt_operation(
                pool,
                provider,
                "transcribe",
                None,
                None,
                duration_ms,
                false,
                Some(&error_msg),
                None,
            )
            .await;
        }
    }

    let transcription = result.map_err(|e| {
        tracing::error!("Speech transcription error: {}", e);
        ApiError::InternalError(format!("Speech transcription failed: {}", e))
    })?;

    Ok(Json(VoiceTranscriptionResponse {
        text: transcription,
    }))
}

/// Handle voice interaction
pub async fn voice_interaction(
    State(state): State<DeploymentImpl>,
    Json(request): Json<VoiceInteraction>,
) -> Result<Json<VoiceInteraction>, ApiError> {
    use services::services::editron::UsageTracker;

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

    let pool = &state.db().pool;
    let stt_provider = nora.config.voice.stt.provider.as_str();

    // Process the voice interaction
    let mut processed_interaction = request;

    // If there's audio input, transcribe it
    if let Some(audio_data) = &processed_interaction.audio_input {
        let stt_start = std::time::Instant::now();
        let stt_result = nora.voice_engine.transcribe_speech(audio_data).await;
        let stt_duration_ms = stt_start.elapsed().as_millis() as i64;

        // Log STT usage
        match &stt_result {
            Ok(t) => {
                UsageTracker::log_stt_operation(
                    pool,
                    stt_provider,
                    "transcribe",
                    None,
                    Some(t.chars().count() as i64),
                    stt_duration_ms,
                    true,
                    None,
                    None,
                )
                .await;
            }
            Err(e) => {
                let error_msg = e.to_string();
                UsageTracker::log_stt_operation(
                    pool,
                    stt_provider,
                    "transcribe",
                    None,
                    None,
                    stt_duration_ms,
                    false,
                    Some(&error_msg),
                    None,
                )
                .await;
            }
        }

        let transcription = stt_result
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
            let _ =
                AgentConversationMessage::add_user_message(pool, conversation.id, transcription)
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
        tracing::warn!(
            "Failed to persist voice conversation to database: {:?}",
            conversation_result.err()
        );
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
    let result: VoiceAnalyticsRow = sqlx::query_as(
        r#"
        SELECT
            user_id,
            COUNT(DISTINCT id) as total_interactions,
            COALESCE(SUM(message_count), 0) as total_messages,
            COALESCE(AVG(julianday(last_message_at) - julianday(created_at)) * 86400000, 0.0) as average_response_time_ms,
            MIN(created_at) as first_interaction,
            MAX(updated_at) as last_interaction,
            SUM(COALESCE(total_input_tokens, 0)) as total_input_tokens,
            SUM(COALESCE(total_output_tokens, 0)) as total_output_tokens,
            COUNT(DISTINCT session_id) as unique_sessions
        FROM agent_conversations
        WHERE agent_id = ? AND user_id = ? AND status = 'active'
        "#,
    )
    .bind(&nora.id)
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch user voice analytics: {}", e);
        ApiError::InternalError(format!("Failed to fetch analytics: {}", e))
    })?;

    Ok(Json(VoiceAnalyticsSummary::from(result)))
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
    let results: Vec<VoiceAnalyticsRow> = sqlx::query_as(
        r#"
        SELECT
            user_id,
            COUNT(DISTINCT id) as total_interactions,
            COALESCE(SUM(message_count), 0) as total_messages,
            COALESCE(AVG(julianday(last_message_at) - julianday(created_at)) * 86400000, 0.0) as average_response_time_ms,
            MIN(created_at) as first_interaction,
            MAX(updated_at) as last_interaction,
            SUM(COALESCE(total_input_tokens, 0)) as total_input_tokens,
            SUM(COALESCE(total_output_tokens, 0)) as total_output_tokens,
            COUNT(DISTINCT session_id) as unique_sessions
        FROM agent_conversations
        WHERE agent_id = ? AND status = 'active' AND user_id IS NOT NULL
        GROUP BY user_id
        ORDER BY MAX(updated_at) DESC
        "#,
    )
    .bind(&nora.id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch all users voice analytics: {}", e);
        ApiError::InternalError(format!("Failed to fetch analytics: {}", e))
    })?;

    let analytics: Vec<VoiceAnalyticsSummary> = results
        .into_iter()
        .map(VoiceAnalyticsSummary::from)
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

    // Use PCG Router if database is connected, otherwise fall back to config-based
    let new_engine = if nora.pool.is_some() {
        // Prefer PCG Router for database-driven provider selection
        let pool_arc = std::sync::Arc::new(state.db().pool.clone());
        UnifiedVoiceEngine::from_pool(pool_arc)
    } else {
        // Fall back to config-based engine
        UnifiedVoiceEngine::from_config(new_config.clone())
            .await
            .map_err(voice_error_to_api)?
    };

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

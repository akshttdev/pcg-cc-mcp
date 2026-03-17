//! Voice synthesis, transcription, and voice interaction handlers for Topsi

use super::*;

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
// Voice Helper Functions
// ============================================================================

/// Sanitize text for TTS synthesis
///
/// Removes JSON, tool execution details, and special characters that can cause
/// TTS engines (especially Chatterbox) to fail.
pub(crate) fn sanitize_text_for_tts(text: &str) -> String {
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
pub(crate) async fn get_or_init_voice_engine() -> Result<Arc<RwLock<Option<VoiceEngine>>>, ApiError> {
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

/// Detect meeting intent in user's transcribed text
pub(crate) fn detect_meeting_intent(text: &str) -> Option<&'static str> {
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
pub(crate) async fn resolve_user_project(pool: &sqlx::SqlitePool, user_id: &str) -> Option<String> {
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

// ============================================================================
// Voice Route Handlers
// ============================================================================

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

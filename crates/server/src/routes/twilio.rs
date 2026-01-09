//! Twilio webhook routes for NORA phone integration
//!
//! These endpoints handle incoming phone calls via Twilio,
//! enabling users to interact with NORA by calling a phone number.
//! Now uses NORA's voice engine for TTS instead of Twilio's Polly.

use std::sync::Arc;

use axum::{
    Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use nora::{
    agent::{NoraRequest, NoraRequestType, RequestPriority},
    twilio::{
        TwilioCallHandler, TwilioCallRequest, TwilioConfig, TwilioSpeechResult,
        TwilioStatusCallback, TwimlBuilder, get_audio_cache,
    },
    voice::AudioFormat,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{DeploymentImpl, routes::nora::get_nora_instance};

/// Global Twilio call handler
static TWILIO_HANDLER: tokio::sync::OnceCell<Arc<TwilioCallHandler>> =
    tokio::sync::OnceCell::const_new();

/// Get or initialize the Twilio call handler
async fn get_twilio_handler() -> Option<Arc<TwilioCallHandler>> {
    if let Some(handler) = TWILIO_HANDLER.get() {
        return Some(handler.clone());
    }

    // Try to initialize from environment
    if let Some(config) = TwilioConfig::from_env() {
        if config.is_configured() {
            let handler = Arc::new(TwilioCallHandler::new(config));
            if TWILIO_HANDLER.set(handler.clone()).is_ok() {
                info!("Twilio handler initialized successfully with NORA voice engine");
                return Some(handler);
            }
        }
    }

    warn!("Twilio is not configured");
    None
}

/// Initialize Twilio routes
pub fn twilio_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/twilio/voice", post(handle_incoming_call))
        .route("/twilio/speech", post(handle_speech_input))
        .route("/twilio/audio/{audio_id}", get(serve_audio))
        .route("/twilio/status", post(handle_status_callback))
        .route("/twilio/fallback", post(handle_fallback))
        .route("/twilio/health", get(twilio_health))
}

/// Query parameters for speech endpoint
#[derive(Debug, Deserialize)]
pub struct SpeechQueryParams {
    call_sid: Option<String>,
}

/// Health check response for Twilio integration
#[derive(Debug, Serialize)]
pub struct TwilioHealthResponse {
    pub configured: bool,
    pub active_calls: usize,
    pub phone_number: Option<String>,
    pub using_nora_voice: bool,
}

/// Generate audio using NORA's voice engine and cache it
async fn generate_and_cache_audio(
    text: &str,
    call_sid: Option<String>,
) -> Result<String, String> {
    // Get NORA instance
    let nora_instance = get_nora_instance()
        .await
        .map_err(|e| format!("NORA not available: {}", e))?;

    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| "NORA not initialized".to_string())?;

    // Synthesize speech using NORA's voice engine
    info!("Synthesizing speech with NORA voice engine: '{}'",
        if text.len() > 50 { format!("{}...", &text[..50]) } else { text.to_string() }
    );

    let audio_base64 = nora
        .voice_engine
        .synthesize_speech(text)
        .await
        .map_err(|e| format!("TTS synthesis failed: {}", e))?;

    // Cache the audio
    let cache = get_audio_cache().await;
    let audio_id = cache
        .store(&audio_base64, AudioFormat::Mp3, text, call_sid)
        .await?;

    Ok(audio_id)
}

/// Build the full audio URL for Twilio to fetch
fn build_audio_url(webhook_base_url: &str, audio_id: &str) -> String {
    format!("{}/api/twilio/audio/{}", webhook_base_url, audio_id)
}

/// Serve cached audio to Twilio
///
/// GET /api/twilio/audio/:audio_id
///
/// Returns the audio file for Twilio's <Play> element to fetch
pub async fn serve_audio(
    State(_state): State<DeploymentImpl>,
    Path(audio_id): Path<String>,
) -> impl IntoResponse {
    info!("Serving audio: {}", audio_id);

    let cache = get_audio_cache().await;

    match cache.get(&audio_id).await {
        Some(cached) => {
            let content_type = cached.content_type();
            info!(
                "Serving {} bytes of {} audio for id {}",
                cached.audio_bytes.len(),
                content_type,
                audio_id
            );

            (
                StatusCode::OK,
                [("Content-Type", content_type)],
                cached.audio_bytes,
            )
        }
        None => {
            warn!("Audio not found in cache: {}", audio_id);
            (
                StatusCode::NOT_FOUND,
                [("Content-Type", "text/plain")],
                b"Audio not found".to_vec(),
            )
        }
    }
}

/// Handle incoming call webhook from Twilio
///
/// POST /api/twilio/voice
///
/// Twilio calls this endpoint when someone calls the configured phone number.
/// Returns TwiML that greets the caller using NORA's voice and starts listening for speech.
pub async fn handle_incoming_call(
    State(_state): State<DeploymentImpl>,
    Form(request): Form<TwilioCallRequest>,
) -> impl IntoResponse {
    info!(
        "Incoming Twilio call: {} from {}",
        request.call_sid, request.from
    );

    let handler = match get_twilio_handler().await {
        Some(h) => h,
        None => {
            error!("Twilio not configured - rejecting call");
            let twiml = TwimlBuilder::new()
                .say_british("I apologise, the phone system is not currently configured. Please try again later.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Register the call with the handler
    if let Err(e) = handler.handle_incoming_call(request.clone()).await {
        error!("Error registering incoming call: {}", e);
    }

    // Generate greeting audio using NORA's voice engine
    let greeting = &handler.config().greeting_message;
    let audio_result = generate_and_cache_audio(greeting, Some(request.call_sid.clone())).await;

    let speech_url = format!(
        "{}/api/twilio/speech?call_sid={}",
        handler.config().webhook_base_url,
        request.call_sid
    );

    let twiml = match audio_result {
        Ok(audio_id) => {
            let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
            info!("Generated greeting audio: {} -> {}", audio_id, audio_url);

            TwimlBuilder::greeting_with_audio_and_gather(
                &audio_url,
                &speech_url,
                &handler.config().speech_language,
            )
        }
        Err(e) => {
            // Fall back to Polly TTS if NORA voice fails
            warn!("NORA voice synthesis failed, falling back to Polly: {}", e);
            TwimlBuilder::greeting_with_gather(
                greeting,
                &speech_url,
                &handler.config().speech_language,
            )
        }
    };

    info!("Generated TwiML for incoming call {}", request.call_sid);
    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

/// Handle speech input webhook from Twilio
///
/// POST /api/twilio/speech
///
/// Twilio calls this endpoint after the caller speaks.
/// Processes the speech through NORA and returns a TwiML response with NORA's voice.
pub async fn handle_speech_input(
    State(_state): State<DeploymentImpl>,
    Query(params): Query<SpeechQueryParams>,
    Form(speech_result): Form<TwilioSpeechResult>,
) -> impl IntoResponse {
    let call_sid = params
        .call_sid
        .as_deref()
        .unwrap_or(&speech_result.call_sid);

    info!(
        "Speech input for call {}: {:?}",
        call_sid, speech_result.speech_result
    );

    let handler = match get_twilio_handler().await {
        Some(h) => h,
        None => {
            let twiml = TwimlBuilder::new()
                .say_british("The system is currently unavailable.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Get the caller's speech text
    let speech_text = match &speech_result.speech_result {
        Some(text) if !text.trim().is_empty() => text.clone(),
        _ => {
            // No speech detected - generate "I didn't catch that" using NORA voice
            let prompt = "I didn't catch that. Could you please repeat?";
            let speech_url = format!(
                "{}/api/twilio/speech?call_sid={}",
                handler.config().webhook_base_url,
                call_sid
            );

            let twiml = match generate_and_cache_audio(prompt, Some(call_sid.to_string())).await {
                Ok(audio_id) => {
                    let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
                    TwimlBuilder::new()
                        .gather_speech_with_audio(
                            &audio_url,
                            &speech_url,
                            10,
                            &handler.config().speech_language,
                            None,
                        )
                        .say_british("If you'd like to end the call, simply say goodbye.")
                        .redirect(&speech_url)
                        .build()
                }
                Err(_) => {
                    // Fall back to Polly
                    TwimlBuilder::new()
                        .gather_speech(
                            &speech_url,
                            10,
                            &handler.config().speech_language,
                            None,
                            Some(prompt),
                        )
                        .say_british("If you'd like to end the call, simply say goodbye.")
                        .redirect(&speech_url)
                        .build()
                }
            };

            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Get NORA session ID for this call
    let session_id = handler
        .get_session_id(call_sid)
        .await
        .unwrap_or_else(|| format!("twilio-{}", Uuid::new_v4()));

    // Get conversation context
    let context = handler
        .get_call_state(call_sid)
        .await
        .map(|state| state.get_conversation_context());

    // Process through NORA to get the response text
    let nora_response = match process_with_nora(&speech_text, &session_id, context).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error processing with NORA: {}", e);
            "I apologise, I'm having trouble processing your request. Could you please try again?"
                .to_string()
        }
    };

    // Record the conversation in the call handler
    if let Err(e) = handler
        .handle_speech_input(speech_result.clone(), nora_response.clone())
        .await
    {
        warn!("Error recording conversation: {}", e);
    }

    // Check for goodbye phrases
    let caller_text = speech_result
        .speech_result
        .as_deref()
        .unwrap_or("")
        .to_lowercase();

    let is_goodbye = caller_text.contains("goodbye")
        || caller_text.contains("bye")
        || caller_text.contains("thank you")
        || caller_text.contains("thanks")
        || caller_text.contains("that's all")
        || caller_text.contains("hang up")
        || caller_text.contains("end call");

    // Generate audio using NORA's voice engine
    let audio_result = generate_and_cache_audio(&nora_response, Some(call_sid.to_string())).await;

    let speech_url = format!(
        "{}/api/twilio/speech?call_sid={}",
        handler.config().webhook_base_url,
        call_sid
    );

    let twiml = match audio_result {
        Ok(audio_id) => {
            let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
            info!(
                "Generated response audio: {} for call {}",
                audio_id, call_sid
            );

            if is_goodbye {
                TwimlBuilder::goodbye_with_audio(&audio_url)
            } else {
                TwimlBuilder::respond_with_audio_and_gather(
                    &audio_url,
                    &speech_url,
                    &handler.config().speech_language,
                )
            }
        }
        Err(e) => {
            // Fall back to Polly TTS
            warn!("NORA voice synthesis failed, falling back to Polly: {}", e);
            if is_goodbye {
                TwimlBuilder::goodbye(&nora_response)
            } else {
                TwimlBuilder::respond_and_gather(
                    &nora_response,
                    &speech_url,
                    &handler.config().speech_language,
                )
            }
        }
    };

    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

/// Handle call status callback from Twilio
///
/// POST /api/twilio/status
///
/// Twilio calls this endpoint when the call status changes.
pub async fn handle_status_callback(
    State(_state): State<DeploymentImpl>,
    Form(status): Form<TwilioStatusCallback>,
) -> impl IntoResponse {
    info!(
        "Call status update: {} -> {}",
        status.call_sid, status.call_status
    );

    if let Some(handler) = get_twilio_handler().await {
        if let Err(e) = handler
            .handle_status_update(&status.call_sid, &status.call_status, status.call_duration)
            .await
        {
            warn!("Error handling status update: {}", e);
        }
    }

    // Clean up cached audio for completed calls
    if status.call_status == "completed"
        || status.call_status == "failed"
        || status.call_status == "busy"
        || status.call_status == "no-answer"
    {
        let cache = get_audio_cache().await;
        cache.cleanup_call(&status.call_sid).await;
        info!("Cleaned up audio cache for call {}", status.call_sid);
    }

    StatusCode::OK
}

/// Handle fallback webhook (called on errors)
///
/// POST /api/twilio/fallback
pub async fn handle_fallback(
    State(_state): State<DeploymentImpl>,
    Form(request): Form<TwilioCallRequest>,
) -> impl IntoResponse {
    error!("Twilio fallback triggered for call: {}", request.call_sid);

    // Try to use NORA voice for error message
    let error_message = "I apologise, we're experiencing technical difficulties. Please try your call again in a few minutes.";

    let twiml = match generate_and_cache_audio(error_message, Some(request.call_sid.clone())).await
    {
        Ok(audio_id) => {
            if let Some(handler) = get_twilio_handler().await {
                let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
                TwimlBuilder::new()
                    .play(&audio_url, 1)
                    .pause(1)
                    .hangup()
                    .build()
            } else {
                TwimlBuilder::new()
                    .say_british(error_message)
                    .pause(1)
                    .hangup()
                    .build()
            }
        }
        Err(_) => TwimlBuilder::new()
            .say_british(error_message)
            .pause(1)
            .hangup()
            .build(),
    };

    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

/// Twilio health check endpoint
///
/// GET /api/twilio/health
pub async fn twilio_health(State(_state): State<DeploymentImpl>) -> impl IntoResponse {
    let (configured, active_calls, phone_number, using_nora_voice) =
        if let Some(handler) = get_twilio_handler().await {
            let calls = handler.get_active_calls().await;
            let phone = if handler.is_configured() {
                Some(handler.config().phone_number.clone())
            } else {
                None
            };

            // Check if NORA voice is available
            let nora_voice_available = get_nora_instance()
                .await
                .map(|_| true)
                .unwrap_or(false);

            (handler.is_configured(), calls.len(), phone, nora_voice_available)
        } else {
            (false, 0, None, false)
        };

    let response = TwilioHealthResponse {
        configured,
        active_calls,
        phone_number,
        using_nora_voice,
    };

    (StatusCode::OK, axum::Json(response))
}

/// Process speech input with NORA
async fn process_with_nora(
    speech_text: &str,
    session_id: &str,
    context: Option<String>,
) -> Result<String, String> {
    // Get NORA instance
    let nora_instance = get_nora_instance()
        .await
        .map_err(|e| format!("NORA not available: {}", e))?;

    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| "NORA not initialized".to_string())?;

    // Build the request with context about it being a phone call
    let phone_context = json!({
        "source": "phone_call",
        "instruction": "This is a phone conversation. Please respond naturally and conversationally, keeping your response concise (2-3 sentences max) as it will be spoken aloud. Use British English.",
        "conversation_history": context.unwrap_or_default()
    });

    let request = NoraRequest {
        request_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        request_type: NoraRequestType::VoiceInteraction,
        content: speech_text.to_string(),
        context: Some(phone_context),
        voice_enabled: true, // Now using NORA's voice engine!
        priority: RequestPriority::Normal,
        timestamp: Utc::now(),
    };

    // Process the request
    let response = nora
        .process_request(request)
        .await
        .map_err(|e| format!("NORA processing error: {}", e))?;

    // Return just the text content (we synthesize speech separately)
    Ok(response.content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twiml_generation() {
        let twiml = TwimlBuilder::new()
            .say_british("Hello, this is a test.")
            .hangup()
            .build();

        assert!(twiml.contains("<Response>"));
        assert!(twiml.contains("Hello"));
        assert!(twiml.contains("<Hangup/>"));
    }

    #[test]
    fn test_audio_url_generation() {
        let url = build_audio_url("https://example.com", "abc123");
        assert_eq!(url, "https://example.com/api/twilio/audio/abc123");
    }
}

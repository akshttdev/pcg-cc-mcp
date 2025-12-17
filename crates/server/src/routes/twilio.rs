//! Twilio webhook routes for NORA phone integration
//!
//! These endpoints handle incoming phone calls via Twilio,
//! enabling users to interact with NORA by calling a phone number.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use chrono::Utc;
use nora::{
    agent::{NoraRequest, NoraRequestType, RequestPriority},
    twilio::{
        TwilioCallHandler, TwilioCallRequest, TwilioConfig, TwilioSpeechResult,
        TwilioStatusCallback, TwimlBuilder,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{routes::nora::get_nora_instance, DeploymentImpl};

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
                info!("Twilio handler initialized successfully");
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
}

/// Handle incoming call webhook from Twilio
///
/// POST /api/twilio/voice
///
/// Twilio calls this endpoint when someone calls the configured phone number.
/// Returns TwiML that greets the caller and starts listening for speech.
pub async fn handle_incoming_call(
    State(_state): State<DeploymentImpl>,
    Form(request): Form<TwilioCallRequest>,
) -> impl IntoResponse {
    info!("Incoming Twilio call: {} from {}", request.call_sid, request.from);

    let handler = match get_twilio_handler().await {
        Some(h) => h,
        None => {
            error!("Twilio not configured - rejecting call");
            let twiml = TwimlBuilder::new()
                .say_british("I apologise, the phone system is not currently configured. Please try again later.")
                .hangup()
                .build();
            return (
                StatusCode::OK,
                [("Content-Type", "application/xml")],
                twiml,
            );
        }
    };

    match handler.handle_incoming_call(request).await {
        Ok(twiml) => (StatusCode::OK, [("Content-Type", "application/xml")], twiml),
        Err(e) => {
            error!("Error handling incoming call: {}", e);
            let twiml = handler.generate_error_response(&e.to_string());
            (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
        }
    }
}

/// Handle speech input webhook from Twilio
///
/// POST /api/twilio/speech
///
/// Twilio calls this endpoint after the caller speaks.
/// Processes the speech through NORA and returns a TwiML response.
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
            // No speech detected - prompt again
            let speech_url = format!(
                "{}/api/twilio/speech?call_sid={}",
                handler.config().webhook_base_url,
                call_sid
            );
            let twiml = TwimlBuilder::new()
                .gather_speech(
                    &speech_url,
                    10,
                    &handler.config().speech_language,
                    None,
                    Some("I didn't catch that. Could you please repeat?"),
                )
                .say_british("If you'd like to end the call, simply say goodbye.")
                .redirect(&speech_url)
                .build();
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

    // Process through NORA
    let nora_response = match process_with_nora(&speech_text, &session_id, context).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error processing with NORA: {}", e);
            "I apologise, I'm having trouble processing your request. Could you please try again?"
                .to_string()
        }
    };

    // Generate TwiML response
    match handler.handle_speech_input(speech_result, nora_response).await {
        Ok(twiml) => (StatusCode::OK, [("Content-Type", "application/xml")], twiml),
        Err(e) => {
            error!("Error generating speech response: {}", e);
            let twiml = handler.generate_error_response(&e.to_string());
            (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
        }
    }
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

    let twiml = TwimlBuilder::new()
        .say_british(
            "I apologise, we're experiencing technical difficulties. \
             Please try your call again in a few minutes.",
        )
        .pause(1)
        .hangup()
        .build();

    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

/// Twilio health check endpoint
///
/// GET /api/twilio/health
pub async fn twilio_health(State(_state): State<DeploymentImpl>) -> impl IntoResponse {
    let (configured, active_calls, phone_number) = if let Some(handler) = get_twilio_handler().await
    {
        let calls = handler.get_active_calls().await;
        let phone = if handler.is_configured() {
            Some(handler.config().phone_number.clone())
        } else {
            None
        };
        (handler.is_configured(), calls.len(), phone)
    } else {
        (false, 0, None)
    };

    let response = TwilioHealthResponse {
        configured,
        active_calls,
        phone_number,
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
        "instruction": "This is a phone conversation. Please respond naturally and conversationally, keeping your response concise (2-3 sentences max) as it will be spoken aloud.",
        "conversation_history": context.unwrap_or_default()
    });

    let request = NoraRequest {
        request_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        request_type: NoraRequestType::VoiceInteraction,
        content: speech_text.to_string(),
        context: Some(phone_context),
        voice_enabled: false, // We use Twilio's TTS, not our own
        priority: RequestPriority::Normal,
        timestamp: Utc::now(),
    };

    // Process the request
    let response = nora
        .process_request(request)
        .await
        .map_err(|e| format!("NORA processing error: {}", e))?;

    // Return just the text content for Twilio TTS
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
}

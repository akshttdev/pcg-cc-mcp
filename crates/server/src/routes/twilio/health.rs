//! Health check endpoint for the Twilio integration.

use super::*;

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

            let nora_voice_available = get_nora_instance().await.map(|_| true).unwrap_or(false);

            (
                handler.is_configured(),
                calls.len(),
                phone,
                nora_voice_available,
            )
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

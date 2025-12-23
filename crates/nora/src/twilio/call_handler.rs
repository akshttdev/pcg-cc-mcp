//! Twilio call handler for managing phone conversations with NORA
//!
//! Handles the lifecycle of phone calls and integrates with NORA's
//! conversation capabilities.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    twiml::TwimlBuilder, TwilioCallRequest, TwilioConfig, TwilioError, TwilioResult,
    TwilioSpeechResult,
};

/// Manages active Twilio phone calls
pub struct TwilioCallHandler {
    config: TwilioConfig,
    active_calls: Arc<RwLock<HashMap<String, TwilioCallState>>>,
}

/// State for an active phone call
#[derive(Debug, Clone)]
pub struct TwilioCallState {
    /// Twilio call SID
    pub call_sid: String,
    /// Internal session ID (maps to NORA session)
    pub session_id: String,
    /// Caller's phone number
    pub caller_number: String,
    /// Called number (Twilio number)
    pub called_number: String,
    /// Current call status
    pub status: CallStatus,
    /// Conversation history for this call
    pub conversation: Vec<ConversationTurn>,
    /// Call start time
    pub started_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Caller info (if available)
    pub caller_info: Option<CallerInfo>,
}

/// Call status
#[derive(Debug, Clone, PartialEq)]
pub enum CallStatus {
    Ringing,
    InProgress,
    OnHold,
    Completed,
    Failed,
    Busy,
    NoAnswer,
}

/// Information about the caller
#[derive(Debug, Clone)]
pub struct CallerInfo {
    pub name: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

/// A turn in the conversation
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub speaker: Speaker,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub confidence: Option<f64>,
}

/// Who is speaking
#[derive(Debug, Clone, PartialEq)]
pub enum Speaker {
    Caller,
    Nora,
}

impl TwilioCallHandler {
    /// Create a new call handler
    pub fn new(config: TwilioConfig) -> Self {
        Self {
            config,
            active_calls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if Twilio is properly configured
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Get the Twilio config
    pub fn config(&self) -> &TwilioConfig {
        &self.config
    }

    /// Handle an incoming call - returns TwiML for initial greeting
    pub async fn handle_incoming_call(&self, request: TwilioCallRequest) -> TwilioResult<String> {
        if !self.is_configured() {
            return Err(TwilioError::NotConfigured);
        }

        info!(
            "Incoming call: {} from {} to {}",
            request.call_sid, request.from, request.to
        );

        // Create call state
        let session_id = format!("twilio-{}", Uuid::new_v4());
        let caller_info = CallerInfo {
            name: request.caller_name,
            city: request.from_city,
            state: request.from_state,
            country: request.from_country,
        };

        let call_state = TwilioCallState {
            call_sid: request.call_sid.clone(),
            session_id: session_id.clone(),
            caller_number: request.from.clone(),
            called_number: request.to,
            status: CallStatus::InProgress,
            conversation: vec![],
            started_at: Utc::now(),
            last_activity: Utc::now(),
            caller_info: Some(caller_info),
        };

        // Store call state
        {
            let mut calls = self.active_calls.write().await;
            calls.insert(request.call_sid.clone(), call_state);
        }

        // Generate greeting TwiML
        let speech_url = format!(
            "{}/api/twilio/speech?call_sid={}",
            self.config.webhook_base_url, request.call_sid
        );

        let twiml = TwimlBuilder::greeting_with_gather(
            &self.config.greeting_message,
            &speech_url,
            &self.config.speech_language,
        );

        info!("Generated greeting TwiML for call {}", request.call_sid);
        Ok(twiml)
    }

    /// Handle speech input from caller - returns TwiML with NORA's response
    pub async fn handle_speech_input(
        &self,
        speech_result: TwilioSpeechResult,
        nora_response: String,
    ) -> TwilioResult<String> {
        let call_sid = &speech_result.call_sid;

        // Get call state
        let mut calls = self.active_calls.write().await;
        let call_state = calls
            .get_mut(call_sid)
            .ok_or_else(|| TwilioError::CallNotFound(call_sid.clone()))?;

        // Record user's speech
        if let Some(ref text) = speech_result.speech_result {
            info!(
                "Caller said: '{}' (confidence: {:?})",
                text, speech_result.confidence
            );

            call_state.conversation.push(ConversationTurn {
                speaker: Speaker::Caller,
                content: text.clone(),
                timestamp: Utc::now(),
                confidence: speech_result.confidence,
            });
        }

        // Record NORA's response
        call_state.conversation.push(ConversationTurn {
            speaker: Speaker::Nora,
            content: nora_response.clone(),
            timestamp: Utc::now(),
            confidence: None,
        });

        call_state.last_activity = Utc::now();

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

        // Generate response TwiML
        let twiml = if is_goodbye {
            call_state.status = CallStatus::Completed;
            TwimlBuilder::goodbye(&nora_response)
        } else {
            let speech_url = format!(
                "{}/api/twilio/speech?call_sid={}",
                self.config.webhook_base_url, call_sid
            );
            TwimlBuilder::respond_and_gather(
                &nora_response,
                &speech_url,
                &self.config.speech_language,
            )
        };

        Ok(twiml)
    }

    /// Handle call status update
    pub async fn handle_status_update(
        &self,
        call_sid: &str,
        status: &str,
        duration: Option<u32>,
    ) -> TwilioResult<()> {
        info!(
            "Call {} status update: {} (duration: {:?}s)",
            call_sid, status, duration
        );

        let mut calls = self.active_calls.write().await;

        if let Some(call_state) = calls.get_mut(call_sid) {
            call_state.status = match status {
                "ringing" => CallStatus::Ringing,
                "in-progress" => CallStatus::InProgress,
                "completed" => CallStatus::Completed,
                "failed" => CallStatus::Failed,
                "busy" => CallStatus::Busy,
                "no-answer" => CallStatus::NoAnswer,
                _ => call_state.status.clone(),
            };

            // Clean up completed calls after a delay
            if call_state.status == CallStatus::Completed
                || call_state.status == CallStatus::Failed
                || call_state.status == CallStatus::Busy
                || call_state.status == CallStatus::NoAnswer
            {
                // In production, you might want to persist this to a database
                // before removing from memory
                info!(
                    "Call {} ended with {} turns",
                    call_sid,
                    call_state.conversation.len()
                );
            }
        } else {
            warn!("Status update for unknown call: {}", call_sid);
        }

        Ok(())
    }

    /// Get the NORA session ID for a call
    pub async fn get_session_id(&self, call_sid: &str) -> Option<String> {
        let calls = self.active_calls.read().await;
        calls.get(call_sid).map(|s| s.session_id.clone())
    }

    /// Get call state
    pub async fn get_call_state(&self, call_sid: &str) -> Option<TwilioCallState> {
        let calls = self.active_calls.read().await;
        calls.get(call_sid).cloned()
    }

    /// Get all active calls
    pub async fn get_active_calls(&self) -> Vec<TwilioCallState> {
        let calls = self.active_calls.read().await;
        calls.values().cloned().collect()
    }

    /// Clean up old inactive calls
    pub async fn cleanup_stale_calls(&self, max_age_minutes: i64) {
        let mut calls = self.active_calls.write().await;
        let cutoff = Utc::now() - chrono::Duration::minutes(max_age_minutes);

        let stale_calls: Vec<String> = calls
            .iter()
            .filter(|(_, state)| state.last_activity < cutoff)
            .map(|(sid, _)| sid.clone())
            .collect();

        for sid in stale_calls {
            info!("Cleaning up stale call: {}", sid);
            calls.remove(&sid);
        }
    }

    /// Generate error TwiML response
    pub fn generate_error_response(&self, message: &str) -> String {
        error!("Twilio error: {}", message);
        TwimlBuilder::error(
            "I apologise, but I'm experiencing technical difficulties. Please try again later.",
            None,
        )
    }

    /// Validate Twilio request signature (security)
    pub fn validate_request_signature(
        &self,
        signature: &str,
        url: &str,
        params: &HashMap<String, String>,
    ) -> bool {
        use ring::hmac;

        // Build the validation string
        let mut validation_string = url.to_string();
        let mut sorted_params: Vec<(&String, &String)> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));

        for (key, value) in sorted_params {
            validation_string.push_str(key);
            validation_string.push_str(value);
        }

        // Compute HMAC-SHA1
        let key = hmac::Key::new(
            hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            self.config.auth_token.as_bytes(),
        );
        let computed_signature = hmac::sign(&key, validation_string.as_bytes());
        let computed_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            computed_signature.as_ref(),
        );

        // Constant-time comparison
        signature == computed_b64
    }
}

impl TwilioCallState {
    /// Get conversation as a formatted string for NORA context
    pub fn get_conversation_context(&self) -> String {
        if self.conversation.is_empty() {
            return String::new();
        }

        let mut context = String::from("Previous conversation:\n");
        for turn in &self.conversation {
            let speaker = match turn.speaker {
                Speaker::Caller => "Caller",
                Speaker::Nora => "Nora",
            };
            context.push_str(&format!("{}: {}\n", speaker, turn.content));
        }
        context
    }

    /// Get the last user message
    pub fn get_last_user_message(&self) -> Option<&str> {
        self.conversation
            .iter()
            .rev()
            .find(|t| t.speaker == Speaker::Caller)
            .map(|t| t.content.as_str())
    }

    /// Get call duration
    pub fn duration_seconds(&self) -> i64 {
        (Utc::now() - self.started_at).num_seconds()
    }
}

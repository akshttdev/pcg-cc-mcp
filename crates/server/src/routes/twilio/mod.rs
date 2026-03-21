//! Twilio webhook routes for NORA phone integration
//!
//! These endpoints handle incoming phone calls via Twilio,
//! enabling users to interact with NORA by calling a phone number.
//! Now uses NORA's voice engine for TTS instead of Twilio's Polly.
//!
//! Also wires caller memory: CRM lookup, CallLog creation, AgentConversation
//! persistence, new-caller onboarding, and VIBE-sponsored first calls.
//!
//! # Single-Instance Constraint
//!
//! This module uses three process-local `Lazy<Arc<Mutex<HashMap>>>` statics for
//! in-flight state: `CALL_DB_CONTEXTS`, `ACTIVE_CALL_PHONES`, and `SMS_THREAD_BUFFER`.
//! These are **not shared across processes**. Running multiple server instances
//! (e.g. Fly.io scale-out) will cause:
//!
//! - SMS thread buffering to split across instances (messages from the same sender
//!   may land on different nodes, breaking the 8-second debounce window)
//! - Active call phone lookups to miss cross-instance calls
//! - Call DB context to be unavailable after a mid-call failover
//!
//! **Before scaling to multi-instance**, migrate these maps to a shared store
//! (Redis, NATS KV, or SQLite WAL with short TTLs). See the deferred item in
//! `planning/branch-merge-analysis.md` for details.

mod audio;
mod calls;
mod health;
mod media;
mod nora_integration;
mod onboarding;
mod sms;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{
    Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use db::{
    db_uuid::DbUuid,
    models::{
        agent_conversation::{AgentConversation, AgentConversationMessage, ConversationStatus},
        call_log::{CallDirection, CallLog, CallStatus, CreateCallLog, UpdateCallLog},
        crm_contact::{ContactSource, CreateCrmContact, CrmContact, LifecycleStage},
        vibe_transaction::VibeSourceType,
    },
};
use deployment::Deployment;
use nora::{
    agent::{NoraRequest, NoraRequestType, RequestPriority},
    twilio::{
        TwilioCallHandler, TwilioCallRequest, TwilioConfig, TwilioSpeechResult,
        TwilioStatusCallback, TwimlBuilder, get_audio_cache,
    },
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use services::services::vibe_pricing::VibePricingService;
use tokio::{sync::Mutex, time::timeout};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{DeploymentImpl, routes::nora::get_nora_instance};

// ─────────────────────────────────────────────────────────────────────────────
// Shared statics
// ─────────────────────────────────────────────────────────────────────────────

/// Global Twilio call handler
static TWILIO_HANDLER: tokio::sync::OnceCell<Arc<TwilioCallHandler>> =
    tokio::sync::OnceCell::const_new();

/// In-memory map: call_sid → CallDbContext (active calls only)
static CALL_DB_CONTEXTS: Lazy<Arc<Mutex<HashMap<String, CallDbContext>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Secondary index: caller phone → call_sid (for SMS-during-call lookup)
static ACTIVE_CALL_PHONES: Lazy<Arc<Mutex<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Debounce window for SMS thread buffering (seconds)
const SMS_THREAD_WINDOW_SECS: u64 = 8;

/// Maximum time allowed for TTS generation (Twilio has ~15s timeout, leave margin for response)
const TTS_TIMEOUT: Duration = Duration::from_secs(8);

/// Maximum time for LLM to respond (leave time for TTS after)
const LLM_TIMEOUT: Duration = Duration::from_secs(12);

/// Global SMS thread buffer: sender phone → buffered thread
static SMS_THREAD_BUFFER: Lazy<Arc<Mutex<HashMap<String, SmsThread>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// ─────────────────────────────────────────────────────────────────────────────
// Shared types
// ─────────────────────────────────────────────────────────────────────────────

/// Buffered SMS thread state per sender
struct SmsThread {
    messages: Vec<String>,
    /// Pending media URLs to fetch+describe before sending to Nora
    pending_media: Vec<(String, Option<String>)>,
    last_received: SystemTime,
    person_context: Option<serde_json::Value>,
}

/// An SMS received while a call is active, optionally with ingested content.
#[derive(Debug, Clone)]
struct InCallSms {
    body: String,
    from: String,
    received_at: chrono::DateTime<chrono::Utc>,
    /// Fetched/extracted content if the SMS contained a URL.
    ingested_content: Option<String>,
}

/// Who is calling — drives system prompt + context depth
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum CallerRole {
    /// PCG admin (is_admin=true)
    PcgAdmin,
    /// PCG team member (user_role='host')
    PcgTeam,
    /// Known external client (returning)
    ReturningClient,
    /// Brand new / unknown caller
    NewCaller,
}

/// Per-call DB context kept in memory while a call is active
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CallDbContext {
    call_log_id: Uuid,
    conversation_id: Uuid,
    crm_contact_id: DbUuid,
    project_id: Uuid,
    caller_role: CallerRole,
    /// E.164 phone number of the caller (used as SMS-during-call lookup key)
    caller_phone: String,
    /// PCG user ID if caller is a PCG team member
    pcg_user_id: Option<Uuid>,
    /// Caller profile pre-serialised as JSON string for the LLM
    caller_profile_json: String,
    /// Pre-built PCG team context (projects/tasks) — only for host callers
    pcg_team_context_json: Option<String>,
    /// SMS messages received while this call is active (drained each speech turn)
    sms_queue: Arc<tokio::sync::Mutex<Vec<InCallSms>>>,
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

/// SignalWire / Twilio-compatible SMS webhook params
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TwilioSmsRequest {
    pub message_sid: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub from_city: Option<String>,
    pub from_country: Option<String>,
    pub num_media: Option<String>,
    // MMS media attachments — provider sends MediaUrl0..N + MediaContentType0..N
    pub media_url_0: Option<String>,
    pub media_url_1: Option<String>,
    pub media_url_2: Option<String>,
    pub media_content_type_0: Option<String>,
    pub media_content_type_1: Option<String>,
    pub media_content_type_2: Option<String>,
}

/// System prompt for PCG team members — Nora as internal orchestrator
const NORA_PCG_TEAM_SYSTEM: &str = "\
You are Nora, PCG's Executive AI Assistant speaking with a member of the PCG team on a phone call. \
You are their intelligent operations assistant — you know their projects, tasks, and boards.

You can help with: creating tasks, updating project status, checking what's in progress, \
scheduling work, summarising project activity, capturing meeting notes, and orchestrating \
agent workflows.

The caller's active projects and tasks will be provided in the context. Reference them naturally.

Rules:\
- Keep every response to 2-3 SHORT sentences maximum\
- Be direct and efficient — you're talking to a colleague\
- Use British English\
- When the caller asks you to do something (create a task, etc.), confirm: \"I'll create that task for you.\"\
- If asked about your capabilities, explain you can orchestrate their PCG workspace by voice";

/// System prompt for external callers — Nora as PCG representative
const NORA_CLIENT_SYSTEM: &str = "\
You are Nora, PCG's Executive AI Assistant. You are speaking on a phone call on behalf of \
Power Club Global (PCG) — a premium AI-powered business platform that helps entrepreneurs, \
executives, and growing teams run their operations with intelligent agents.

PCG's capabilities include: AI project management, autonomous agents that execute tasks, \
CRM and client management, content creation, social media management, financial tracking \
with VIBE tokens, team collaboration, and custom AI workflows.

Your role on this call is to represent PCG warmly and professionally — understand the caller's \
goals, answer their questions, and help them see how PCG can help them. New callers can get \
their own PCG environment and their own AI assistant (Topsi) to manage their work.

Rules for phone calls:\
- Keep every response to 2-3 SHORT sentences maximum\
- Be warm, natural, and conversational\
- Use British English\
- Never list more than 2-3 items at once — summarise instead\
- If asked about capabilities, give a brief compelling overview then ask what they need help with";

// ─────────────────────────────────────────────────────────────────────────────
// Shared helper functions
// ─────────────────────────────────────────────────────────────────────────────

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

/// Truncate a string to `max` characters, appending "…" if truncated.
fn truncate_for_sms(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", truncated)
}

/// Send an outbound SMS via SignalWire (or Twilio-compatible) REST API.
async fn send_outbound_sms(to: &str, body: &str) -> Result<(), anyhow::Error> {
    let account_sid = std::env::var("TWILIO_ACCOUNT_SID")
        .map_err(|_| anyhow::anyhow!("TWILIO_ACCOUNT_SID not set"))?;
    let auth_token = std::env::var("TWILIO_AUTH_TOKEN")
        .map_err(|_| anyhow::anyhow!("TWILIO_AUTH_TOKEN not set"))?;
    // Accept TWILIO_PHONE_NUMBER (current) or legacy TWILIO_FROM_NUMBER
    let from_number = std::env::var("TWILIO_PHONE_NUMBER")
        .or_else(|_| std::env::var("TWILIO_FROM_NUMBER"))
        .map_err(|_| anyhow::anyhow!("TWILIO_PHONE_NUMBER not set"))?;

    // Use SignalWire endpoint when SIGNALWIRE_SPACE_URL is configured
    let url = if let Ok(space) = std::env::var("SIGNALWIRE_SPACE_URL") {
        format!(
            "https://{}/api/laml/2010-04-01/Accounts/{}/Messages.json",
            space, account_sid
        )
    } else {
        format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            account_sid
        )
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .basic_auth(&account_sid, Some(&auth_token))
        .form(&[("To", to), ("From", &from_number), ("Body", body)])
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(anyhow::anyhow!(
            "SMS send error {}: {}",
            status,
            &text[..text.len().min(200)]
        ))
    }
}

/// Look up person context from the persons table by phone number.
/// Returns a JSON Value with `name`, `email` if found.
async fn lookup_sms_sender_context(
    pool: &sqlx::SqlitePool,
    phone: &str,
) -> Option<serde_json::Value> {
    // PCG team check first — higher priority than CRM lookup
    if let Some((_uid, full_name, is_admin)) = onboarding::lookup_pcg_team_member(pool, phone).await
    {
        return Some(serde_json::json!({
            "name": full_name,
            "phone": phone,
            "caller_type": if is_admin { "pcg_admin" } else { "pcg_team" },
            "is_pcg_team": true,
        }));
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        full_name: String,
        email: Option<String>,
    }
    // phones column is a JSON array of {value, label} objects; search for the phone in it
    let row: Option<Row> = sqlx::query_as(
        "SELECT full_name, email FROM persons \
         WHERE phones LIKE ? OR phones LIKE ? \
         LIMIT 1",
    )
    .bind(format!("%\"{}%", phone))
    .bind(format!("%{}%", phone))
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    row.map(|r| {
        serde_json::json!({
            "name": r.full_name,
            "email": r.email,
            "phone": phone,
            "caller_type": "client",
            "is_pcg_team": false,
        })
    })
}

/// Strip markdown formatting so TTS doesn't read symbols aloud.
pub(crate) fn strip_markdown_for_tts(text: &str) -> String {
    // Remove bold/italic markers (** __ * _), inline code backticks, and headers (#)
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' | '`' | '#' => {
                // Skip runs of the same symbol
                while chars.peek() == Some(&c) {
                    chars.next();
                }
            }
            // Replace markdown links [label](url) with just the label
            '[' => {
                let label: String = chars.by_ref().take_while(|&ch| ch != ']').collect();
                // Consume (url) if present
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&label);
            }
            _ => out.push(c),
        }
    }
    // Collapse runs of spaces that removing symbols may leave
    let cleaned: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned
}

/// Safely truncate a string for logging (UTF-8 aware)
fn truncate_for_log(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

/// Initialize Twilio routes
pub fn twilio_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/twilio/voice", post(calls::handle_incoming_call))
        .route("/twilio/sms", post(sms::handle_incoming_sms))
        .route("/twilio/speech", post(calls::handle_speech_input))
        .route("/twilio/audio/{audio_id}", get(audio::serve_audio))
        .route("/twilio/status", post(calls::handle_call_status))
        .route("/twilio/fallback", post(calls::handle_fallback))
        .route("/twilio/health", get(health::twilio_health))
}

// TODO: unused — comment out to suppress warning
// /// Escape special XML characters for TwiML body
// fn xml_escape(s: &str) -> String {
//     s.replace('&', "&amp;")
//         .replace('<', "&lt;")
//         .replace('>', "&gt;")
//         .replace('"', "&quot;")
//         .replace('\'', "&apos;")
// }

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
        let url = audio::build_audio_url("https://example.com", "abc123");
        assert_eq!(url, "https://example.com/api/twilio/audio/abc123");
    }
}

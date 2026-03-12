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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};


use axum::{
    Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;

use db::models::agent_conversation::{
    AgentConversation, AgentConversationMessage, ConversationStatus,
};
use db::models::call_log::{CallDirection, CallLog, CallStatus, CreateCallLog, UpdateCallLog};
use db::models::crm_contact::{
    ContactSource, CreateCrmContact, CrmContact, LifecycleStage,
};
use db::models::vibe_transaction::VibeSourceType;
use services::services::vibe_pricing::VibePricingService;
use nora::twilio::{
    TwilioCallHandler, TwilioCallRequest, TwilioConfig, TwilioSpeechResult,
    TwilioStatusCallback, TwimlBuilder, get_audio_cache,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{DeploymentImpl, routes::nora::get_nora_instance};
use deployment::Deployment;

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

/// Buffered SMS thread state per sender
struct SmsThread {
    messages: Vec<String>,
    /// Pending media URLs to fetch+describe before sending to Nora
    pending_media: Vec<(String, Option<String>)>,
    last_received: SystemTime,
    person_context: Option<serde_json::Value>,
}

/// Global SMS thread buffer: sender phone → buffered thread
static SMS_THREAD_BUFFER: Lazy<Arc<Mutex<HashMap<String, SmsThread>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Look up person context from the persons table by phone number.
/// Returns a JSON Value with `name`, `email` if found.
async fn lookup_sms_sender_context(
    pool: &sqlx::SqlitePool,
    phone: &str,
) -> Option<serde_json::Value> {
    // PCG team check first — higher priority than CRM lookup
    if let Some((_uid, full_name, is_admin)) = lookup_pcg_team_member(pool, phone).await {
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

    row.map(|r| serde_json::json!({
        "name": r.full_name,
        "email": r.email,
        "phone": phone,
        "caller_type": "client",
        "is_pcg_team": false,
    }))
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
        Err(anyhow::anyhow!("SMS send error {}: {}", status, &text[..text.len().min(200)]))
    }
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
    crm_contact_id: Uuid,
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
        .route("/twilio/sms", post(handle_incoming_sms))
        .route("/twilio/speech", post(handle_speech_input))
        .route("/twilio/audio/{audio_id}", get(serve_audio))
        .route("/twilio/status", post(handle_call_status))
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

/// Maximum time allowed for TTS generation (Twilio has ~15s timeout, leave margin for response)
const TTS_TIMEOUT: Duration = Duration::from_secs(8);

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

    // Synthesize speech using NORA's voice engine with timeout
    let truncated_text = truncate_for_log(text, 50);
    info!("Synthesizing speech with NORA voice engine: '{}'", truncated_text);

    // Strip markdown before TTS so symbols like * aren't read aloud
    let clean_text = strip_markdown_for_tts(text);

    // Apply timeout to TTS generation
    let tts_future = nora.voice_engine.synthesize_speech_with_format(&clean_text);
    let (audio_base64, audio_format) = match timeout(TTS_TIMEOUT, tts_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(format!("TTS synthesis failed: {}", e)),
        Err(_) => return Err(format!("TTS timeout after {:?}", TTS_TIMEOUT)),
    };

    // Cache the audio using the actual format returned by the TTS provider
    let cache = get_audio_cache().await;
    let audio_id = cache
        .store(&audio_base64, audio_format, &clean_text, call_sid)
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

// ---------------------------------------------------------------------------
// Onboarding helpers
// ---------------------------------------------------------------------------

/// Create a bare-bones PCG user account for a new phone caller.
/// Returns (user_id, project_id).
async fn create_caller_account(
    pool: &sqlx::SqlitePool,
    phone: &str,
    full_name: &str,
) -> anyhow::Result<(Uuid, Uuid)> {
    // Sanitise phone into a valid username slug
    let slug = phone
        .replace('+', "")
        .replace(['-', ' ', '(', ')'], "_");
    let username = format!("caller_{}", slug);
    let email = format!("{}@pcg.phone.noreply", slug);

    // Check if user already exists
    let existing: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ? LIMIT 1")
            .bind(&username)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    if let Some((id_bytes,)) = existing {
        let user_id = Uuid::from_slice(&id_bytes)?;
        // Find their most recent project via project_members
        let project_row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT project_id FROM project_members WHERE user_id = ? ORDER BY granted_at DESC LIMIT 1"
        )
        .bind(user_id.as_bytes().as_slice())
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((pid_bytes,)) = project_row {
            let project_id = Uuid::from_slice(&pid_bytes)?;
            return Ok((user_id, project_id));
        }
        // No project yet — create one
        let project_id = create_caller_project(pool, user_id, full_name).await?;
        return Ok((user_id, project_id));
    }

    // New user — hash a random password (caller won't use password login)
    let random_pw = Uuid::new_v4().to_string();
    let password_hash = db::services::AuthService::hash_password(&random_pw)
        .unwrap_or_else(|_| format!("!invalid_{}", Uuid::new_v4()));

    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
           VALUES (?, ?, ?, ?, ?, 0, 1)"#,
    )
    .bind(user_id.as_bytes().as_slice())
    .bind(&username)
    .bind(&email)
    .bind(full_name)
    .bind(&password_hash)
    .execute(pool)
    .await?;

    let project_id = create_caller_project(pool, user_id, full_name).await?;

    info!("Created PCG account for caller {}: user={}, project={}", phone, user_id, project_id);
    Ok((user_id, project_id))
}

/// Create a project for a phone caller and add them as owner.
async fn create_caller_project(
    pool: &sqlx::SqlitePool,
    user_id: Uuid,
    full_name: &str,
) -> anyhow::Result<Uuid> {
    let project_id = Uuid::new_v4();
    let project_name = format!("{}'s Projects", full_name);
    let git_repo_path = format!("/pcg/callers/{}", project_id);

    sqlx::query(
        r#"INSERT INTO projects (id, name, git_repo_path, created_at, updated_at)
           VALUES (?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"#,
    )
    .bind(project_id.as_bytes().as_slice())
    .bind(&project_name)
    .bind(&git_repo_path)
    .execute(pool)
    .await?;

    // Add as owner in project_members
    let member_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO project_members (id, project_id, user_id, role)
           VALUES (?, ?, ?, 'owner')"#,
    )
    .bind(member_id.as_bytes().as_slice())
    .bind(project_id.as_bytes().as_slice())
    .bind(user_id.as_bytes().as_slice())
    .execute(pool)
    .await?;

    Ok(project_id)
}

// ---------------------------------------------------------------------------
// PCG team phone recognition
// ---------------------------------------------------------------------------

/// Check if this phone number belongs to a PCG team member.
///
/// Reads `PCG_TEAM_PHONES` env var — comma-separated `phone:username` pairs, e.g.
/// `PCG_TEAM_PHONES="+13059847801:admin,+15551112222:Sirak"`
///
/// Returns `(user_id, full_name, is_admin)` when found.
async fn lookup_pcg_team_member(
    pool: &sqlx::SqlitePool,
    phone: &str,
) -> Option<(Uuid, String, bool)> {
    let mapping = std::env::var("PCG_TEAM_PHONES").unwrap_or_default();
    if mapping.is_empty() {
        return None;
    }

    // Find the username for this phone
    let username = mapping
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.trim().splitn(2, ':');
            let p = parts.next()?.trim();
            let u = parts.next()?.trim();
            if p == phone.trim() { Some(u.to_string()) } else { None }
        })
        .next()?;

    // Look up user in DB
    let row: Option<(Vec<u8>, String, i64)> = sqlx::query_as(
        "SELECT id, full_name, is_admin FROM users WHERE username = ? LIMIT 1",
    )
    .bind(&username)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    row.and_then(|(id_bytes, full_name, is_admin)| {
        Uuid::from_slice(&id_bytes)
            .ok()
            .map(|uid| (uid, full_name, is_admin != 0))
    })
}

/// Load a compact project + task summary for a PCG team member.
/// Returns a JSON string suitable for inclusion in the LLM context.
async fn build_pcg_team_context(pool: &sqlx::SqlitePool, user_id: Uuid) -> String {
    // Load their projects (most recently updated first)
    #[derive(sqlx::FromRow)]
    struct ProjRow {
        name: String,
        description: Option<String>,
    }

    let projects: Vec<ProjRow> = sqlx::query_as(
        r#"SELECT p.name, p.description
           FROM projects p
           JOIN project_members pm ON pm.project_id = p.id
           WHERE pm.user_id = ?
           ORDER BY p.updated_at DESC
           LIMIT 8"#,
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Load their recent active tasks
    #[derive(sqlx::FromRow)]
    struct TaskRow {
        title: String,
        status: String,
        project_name: String,
    }

    let tasks: Vec<TaskRow> = sqlx::query_as(
        r#"SELECT t.title, t.status, p.name as project_name
           FROM tasks t
           JOIN projects p ON p.id = t.project_id
           JOIN project_members pm ON pm.project_id = p.id
           WHERE pm.user_id = ?
             AND t.status NOT IN ('completed', 'cancelled', 'archived')
           ORDER BY t.updated_at DESC
           LIMIT 12"#,
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let projects_json: Vec<serde_json::Value> = projects
        .iter()
        .map(|p| json!({ "name": p.name, "description": p.description }))
        .collect();

    let tasks_json: Vec<serde_json::Value> = tasks
        .iter()
        .map(|t| json!({ "title": t.title, "status": t.status, "project": t.project_name }))
        .collect();

    json!({
        "projects": projects_json,
        "active_tasks": tasks_json,
    })
    .to_string()
}

/// Get a fallback project id (first project in DB).
async fn get_fallback_project_id(pool: &sqlx::SqlitePool) -> Option<Uuid> {
    let row: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT id FROM projects ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    row.and_then(|(bytes,)| Uuid::from_slice(&bytes).ok())
}

// ---------------------------------------------------------------------------
// handle_incoming_call
// ---------------------------------------------------------------------------

/// Handle incoming call webhook from Twilio
///
/// POST /api/twilio/voice
pub async fn handle_incoming_call(
    State(deployment): State<DeploymentImpl>,
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

    // Register the call with the in-memory handler
    if let Err(e) = handler.handle_incoming_call(request.clone()).await {
        error!("Error registering incoming call: {}", e);
    }

    let pool = &deployment.db().pool;

    // ------------------------------------------------------------------
    // 1. Check if this is a PCG team member (highest priority)
    // ------------------------------------------------------------------
    let twilio_caller_name = request.caller_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown Caller")
        .to_string();

    let pcg_member = lookup_pcg_team_member(pool, &request.from).await;

    // ------------------------------------------------------------------
    // 2. Branch: PCG team vs external caller
    // ------------------------------------------------------------------
    enum CallerBranch {
        PcgTeam {
            user_id: Uuid,
            full_name: String,
            is_admin: bool,
            project_id: Uuid,
            team_context_json: String,
            crm_contact_id: Uuid,
        },
        External {
            contact: CrmContact,
            project_id: Uuid,
            caller_role: CallerRole,
            previous_calls: usize,
            previous_summaries: Vec<String>,
        },
    }

    let branch = if let Some((user_id, full_name, is_admin)) = pcg_member {
        // PCG team member calling
        let project_id = {
            let row: Option<(Vec<u8>,)> = sqlx::query_as(
                "SELECT project_id FROM project_members WHERE user_id = ? ORDER BY granted_at DESC LIMIT 1"
            )
            .bind(user_id.as_bytes().as_slice())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
            match row.and_then(|(b,)| Uuid::from_slice(&b).ok()) {
                Some(pid) => pid,
                None => get_fallback_project_id(pool).await.unwrap_or_else(Uuid::new_v4),
            }
        };

        let team_context = build_pcg_team_context(pool, user_id).await;

        // Look up organization_id from the project
        let organization_id = {
            let row: Option<(Vec<u8>,)> = sqlx::query_as(
                "SELECT organization_id FROM projects WHERE id = ?"
            )
            .bind(project_id.as_bytes().as_slice())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
            row.and_then(|(b,)| Uuid::from_slice(&b).ok()).unwrap_or_else(Uuid::new_v4)
        };


        // Ensure CRM contact exists for PCG team member (for call_log FK)
        let crm_contact_id = {
            match CrmContact::find_by_phone_global(pool, &request.from).await {
                Ok(Some(c)) => c.id,
                _ => {
                    // Create internal CRM entry for the team member
                    let first = full_name.split_whitespace().next().unwrap_or(&full_name).to_string();
                    let last = full_name.split_whitespace().nth(1).map(|s| s.to_string());
                    match CrmContact::create(pool, CreateCrmContact {
                        organization_id,
                        client_id: None,

                        first_name: Some(first),
                        last_name: last,
                        email: None,
                        phone: Some(request.from.clone()),
                        mobile: None,
                        avatar_url: None,
                        company_name: Some("Power Club Global".to_string()),
                        job_title: if is_admin { Some("Administrator".to_string()) } else { Some("Team Member".to_string()) },
                        department: None,
                        linkedin_url: None,
                        twitter_handle: None,
                        website: None,
                        source: Some(ContactSource::Manual),
                        lifecycle_stage: Some(LifecycleStage::Customer),
                        tags: Some(vec!["pcg-team".to_string()]),
                        custom_fields: None,
                        zoho_contact_id: None,
                        gmail_contact_id: None,
                    }).await {
                        Ok(c) => c.id,
                        Err(_) => Uuid::new_v4(),
                    }
                }
            }
        };

        info!("PCG team member calling: {} (admin={}), project={}", full_name, is_admin, project_id);
        CallerBranch::PcgTeam { user_id, full_name, is_admin, project_id, team_context_json: team_context, crm_contact_id }
    } else {
        // External caller — CRM lookup
        let existing_contact = CrmContact::find_by_phone_global(pool, &request.from)
            .await
            .unwrap_or(None);

        if let Some(contact) = existing_contact {
            // Returning external client
            let prev_logs = CallLog::find_by_crm_contact(pool, contact.id, 3)
                .await
                .unwrap_or_default();
            let project_id = match prev_logs.first().map(|l| l.project_id) {
                Some(pid) => pid,
                None => get_fallback_project_id(pool).await.unwrap_or_else(Uuid::new_v4),
            };
            let call_count = prev_logs.len();
            let summaries: Vec<String> = prev_logs.iter().filter_map(|l| l.summary.clone()).collect();
            info!("Returning client {} ({} previous calls)", request.from, call_count);
            CallerBranch::External {
                contact,
                project_id,
                caller_role: CallerRole::ReturningClient,
                previous_calls: call_count,
                previous_summaries: summaries,
            }
        } else {
            // New caller — create account + CRM
            let (_user_id, project_id) = create_caller_account(pool, &request.from, &twilio_caller_name)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to create caller account: {}", e);
                    (Uuid::new_v4(), Uuid::new_v4())
                });

            // Look up organization_id from the project
            let organization_id = {
                let row: Option<(Vec<u8>,)> = sqlx::query_as(
                    "SELECT organization_id FROM projects WHERE id = ?"
                )
                .bind(project_id.as_bytes().as_slice())
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
                row.and_then(|(b,)| Uuid::from_slice(&b).ok()).unwrap_or_else(Uuid::new_v4)
            };

            let contact = match CrmContact::create(pool, CreateCrmContact {
                organization_id,
                client_id: None,

                first_name: Some(twilio_caller_name.split_whitespace().next().unwrap_or(&twilio_caller_name).to_string()),
                last_name: twilio_caller_name.split_whitespace().nth(1).map(|s| s.to_string()),
                email: None,
                phone: Some(request.from.clone()),
                mobile: None,
                avatar_url: None,
                company_name: None,
                job_title: None,
                department: None,
                linkedin_url: None,
                twitter_handle: None,
                website: None,
                source: Some(ContactSource::Manual),
                lifecycle_stage: Some(LifecycleStage::Lead),
                tags: None,
                custom_fields: None,
                zoho_contact_id: None,
                gmail_contact_id: None,
            }).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create CRM contact: {}", e);
                    let twiml = TwimlBuilder::new()
                        .say_british("I apologise, we're experiencing a technical issue. Please call back in a moment.")
                        .hangup()
                        .build();
                    return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
                }
            };

            info!("New caller {} onboarded: contact={}, project={}", request.from, contact.id, project_id);
            CallerBranch::External {
                contact,
                project_id,
                caller_role: CallerRole::NewCaller,
                previous_calls: 0,
                previous_summaries: vec![],
            }
        }
    };

    // ------------------------------------------------------------------
    // 3. Unpack branch into unified variables
    // ------------------------------------------------------------------
    let (caller_name, caller_role, pcg_user_id, project_id, crm_contact_id, pcg_team_context_json, caller_profile_json) = match branch {
        CallerBranch::PcgTeam { user_id, full_name, is_admin, project_id, team_context_json, crm_contact_id } => {
            let role = if is_admin { CallerRole::PcgAdmin } else { CallerRole::PcgTeam };
            let profile = json!({
                "caller_phone": request.from,
                "caller_name": full_name,
                "caller_role": if is_admin { "pcg_admin" } else { "pcg_team" },
                "is_pcg_team": true,
                "company": "Power Club Global",
            }).to_string();
            (full_name, role, Some(user_id), project_id, crm_contact_id, Some(team_context_json), profile)
        }
        CallerBranch::External { contact, project_id, caller_role, previous_calls, previous_summaries } => {
            let name = contact.first_name.as_deref()
                .map(|f| if let Some(l) = &contact.last_name {
                    format!("{} {}", f, l)
                } else {
                    f.to_string()
                })
                .unwrap_or_else(|| twilio_caller_name.clone());
            let is_new = caller_role == CallerRole::NewCaller;
            let profile = json!({
                "caller_phone": request.from,
                "caller_name": name,
                "caller_role": if is_new { "new_caller" } else { "returning_client" },
                "is_pcg_team": false,
                "company": contact.company_name,
                "job_title": contact.job_title,
                "lifecycle_stage": contact.lifecycle_stage,
                "previous_calls": previous_calls,
                "sponsored_call": is_new,
                "previous_summaries": previous_summaries,
            }).to_string();
            (name, caller_role, None, project_id, contact.id, None, profile)
        }
    };

    // ------------------------------------------------------------------
    // 4. Create CallLog
    // ------------------------------------------------------------------
    let call_log = match CallLog::create(
        pool,
        CreateCallLog {
            project_id,
            call_sid: request.call_sid.clone(),
            parent_call_sid: None,
            account_sid: None,
            from_number: request.from.clone(),
            to_number: request.to.clone(),
            from_formatted: None,
            to_formatted: None,
            caller_name: Some(caller_name.clone()),
            direction: CallDirection::Inbound,
            status: CallStatus::InProgress,
            answered_by: None,
            start_time: Some(Utc::now()),
        },
    )
    .await
    {
        Ok(log) => {
            let _ = CallLog::update(
                pool,
                log.id,
                UpdateCallLog {
                    crm_contact_id: Some(crm_contact_id),
                    ..Default::default()
                },
            )
            .await;
            log
        }
        Err(e) => {
            error!("Failed to create call log: {}", e);
            let twiml = TwimlBuilder::new()
                .say_british("I apologise, we're experiencing a technical issue logging this call. Please try again.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // ------------------------------------------------------------------
    // 5. Get or create AgentConversation
    // ------------------------------------------------------------------
    let session_id = format!("twilio-{}", request.call_sid);

    let nora_agent_id: Uuid = {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT id FROM agents WHERE short_name = 'Nora' LIMIT 1")
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
        row.and_then(|(bytes,)| Uuid::from_slice(&bytes).ok())
            .unwrap_or_else(Uuid::new_v4)
    };

    let conversation = match AgentConversation::get_or_create(
        pool,
        nora_agent_id,
        &session_id,
        Some(project_id),
    )
    .await
    {
        Ok(conv) => conv,
        Err(e) => {
            error!("Failed to get/create AgentConversation: {}", e);
            let twiml = TwimlBuilder::new()
                .say_british("I apologise, I cannot start a conversation right now. Please try again.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // ------------------------------------------------------------------
    // 6. Store in CALL_DB_CONTEXTS + register phone→call_sid mapping
    // ------------------------------------------------------------------
    {
        let mut map = CALL_DB_CONTEXTS.lock().await;
        map.insert(
            request.call_sid.clone(),
            CallDbContext {
                call_log_id: call_log.id,
                conversation_id: conversation.id,
                crm_contact_id,
                project_id,
                caller_role: caller_role.clone(),
                caller_phone: request.from.clone(),
                pcg_user_id,
                caller_profile_json: caller_profile_json.clone(),
                pcg_team_context_json: pcg_team_context_json.clone(),
                sms_queue: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            },
        );
    }
    {
        let mut phones = ACTIVE_CALL_PHONES.lock().await;
        phones.insert(request.from.clone(), request.call_sid.clone());
    }

    // ------------------------------------------------------------------
    // 7. Personalised greeting
    // ------------------------------------------------------------------
    let greeting = match &caller_role {
        CallerRole::PcgAdmin | CallerRole::PcgTeam => {
            let first = caller_name.split_whitespace().next().unwrap_or(&caller_name);
            format!("Hello {}! How can I help you today?", first)
        }
        CallerRole::ReturningClient => {
            let first = caller_name.split_whitespace().next().unwrap_or("there");
            format!("Welcome back, {}! Lovely to hear from you again. How can I help you today?", first)
        }
        CallerRole::NewCaller => handler.config().greeting_message.clone(),
    };

    let audio_result = generate_and_cache_audio(&greeting, Some(request.call_sid.clone())).await;

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
            warn!("NORA voice synthesis failed, falling back to Polly: {}", e);
            TwimlBuilder::greeting_with_gather(
                &greeting,
                &speech_url,
                &handler.config().speech_language,
            )
        }
    };

    info!("Generated TwiML for incoming call {}", request.call_sid);
    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

// ---------------------------------------------------------------------------
// handle_speech_input
// ---------------------------------------------------------------------------

/// Handle speech input webhook from Twilio
///
/// POST /api/twilio/speech
pub async fn handle_speech_input(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<SpeechQueryParams>,
    Form(speech_result): Form<TwilioSpeechResult>,
) -> impl IntoResponse {
    let call_sid = params
        .call_sid
        .as_deref()
        .unwrap_or(&speech_result.call_sid)
        .to_string();

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
            // No speech detected
            let prompt = "I didn't catch that. Could you please repeat?";
            let speech_url = format!(
                "{}/api/twilio/speech?call_sid={}",
                handler.config().webhook_base_url,
                call_sid
            );

            let twiml = match generate_and_cache_audio(prompt, Some(call_sid.clone())).await {
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
                Err(_) => TwimlBuilder::new()
                    .gather_speech(
                        &speech_url,
                        10,
                        &handler.config().speech_language,
                        None,
                        Some(prompt),
                    )
                    .say_british("If you'd like to end the call, simply say goodbye.")
                    .redirect(&speech_url)
                    .build(),
            };

            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Get NORA session ID for this call
    let session_id = handler
        .get_session_id(&call_sid)
        .await
        .unwrap_or_else(|| format!("twilio-{}", Uuid::new_v4()));

    // Get conversation context from the in-memory call handler (turn history)
    let context = handler
        .get_call_state(&call_sid)
        .await
        .map(|state| state.get_conversation_context());

    // Retrieve CallDbContext for this call (if available)
    let db_ctx = {
        let map = CALL_DB_CONTEXTS.lock().await;
        map.get(call_sid.as_str()).cloned()
    };

    // Drain any SMS messages queued while this call was active
    let queued_sms: Vec<serde_json::Value> = if let Some(ref ctx) = db_ctx {
        let mut queue = ctx.sms_queue.lock().await;
        queue.drain(..).map(|sms| {
            let mut entry = json!({
                "from": sms.from,
                "received_at": sms.received_at.to_rfc3339(),
                "body": sms.body,
            });
            if let Some(content) = sms.ingested_content {
                entry["ingested_content"] = json!(content);
            }
            entry
        }).collect()
    } else {
        vec![]
    };
    let sms_note = if !queued_sms.is_empty() {
        format!(
            "\n\nNOTE: The caller sent {} SMS message(s) during this call. Acknowledge them naturally and use their content in your response:\n{}",
            queued_sms.len(),
            serde_json::to_string_pretty(&queued_sms).unwrap_or_default()
        )
    } else {
        String::new()
    };

    // Build caller-aware phone context
    let phone_context = if let Some(ref ctx) = db_ctx {
        let caller_profile: serde_json::Value =
            serde_json::from_str(&ctx.caller_profile_json).unwrap_or_default();

        match &ctx.caller_role {
            CallerRole::PcgAdmin | CallerRole::PcgTeam => {
                // Full orchestration context for PCG team
                let team_data: serde_json::Value = ctx.pcg_team_context_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                json!({
                    "source": "phone_call",
                    "caller_type": "pcg_team",
                    "caller": caller_profile,
                    "pcg_work": team_data,
                    "conversation_history": context.unwrap_or_default(),
                    "sms_received_during_call": queued_sms,
                    "instruction": format!("Keep responses to 2-3 SHORT sentences. Be direct and action-oriented. British English. When asked to create tasks or update projects, confirm what you will do.{}", sms_note)
                })
            }
            CallerRole::ReturningClient => {
                json!({
                    "source": "phone_call",
                    "caller_type": "returning_client",
                    "caller": caller_profile,
                    "conversation_history": context.unwrap_or_default(),
                    "sms_received_during_call": queued_sms,
                    "instruction": format!("Keep responses to 2-3 SHORT sentences. Be warm and professional. British English. Reference previous context where relevant.{}", sms_note)
                })
            }
            CallerRole::NewCaller => {
                json!({
                    "source": "phone_call",
                    "caller_type": "new_caller",
                    "caller": caller_profile,
                    "conversation_history": context.unwrap_or_default(),
                    "sms_received_during_call": queued_sms,
                    "instruction": format!("Keep responses to 2-3 SHORT sentences. Be warm and welcoming. British English. Help them understand what PCG can do for them.{}", sms_note)
                })
            }
        }
    } else {
        json!({
            "source": "phone_call",
            "instruction": "This is a phone call. Keep your response to 1-2 SHORT sentences only. Be conversational and natural. Use British English.",
            "conversation_history": context.unwrap_or_default()
        })
    };

    // Process through NORA to get response text
    let (nora_response_raw, input_tokens, output_tokens) =
        match process_with_nora(&speech_text, &session_id, Some(phone_context)).await {
            Ok(result) => result,
            Err(e) => {
                error!("Error processing with NORA: {}", e);
                ("I apologise, I'm having trouble processing your request. Could you please try again?".to_string(), 0i64, 0i64)
            }
        };
    // Strip markdown so neither ElevenLabs TTS nor Twilio <Say> reads symbols aloud
    let nora_response = strip_markdown_for_tts(&nora_response_raw);

    // Record VIBE usage for this phone turn (fire-and-forget)
    if let Some(ref ctx) = db_ctx {
        if input_tokens > 0 || output_tokens > 0 {
            let pool = deployment.db().pool.clone();
            let project_id = ctx.project_id;
            let (in_tok, out_tok) = (input_tokens, output_tokens);
            tokio::spawn(async move {
                let pricing = VibePricingService::new(pool.clone());
                if let Ok(tx) = pricing.record_llm_usage(
                    VibeSourceType::Project,
                    project_id,
                    "claude-haiku-4-5-20251001",
                    in_tok,
                    out_tok,
                    None,
                    None,
                    None,
                ).await {
                    if let Err(e) = db::models::project::Project::adjust_vibe_spent(&pool, project_id, tx.amount_vibe).await {
                        tracing::warn!("[VIBE] Failed to adjust project vibe_spent: {e}");
                    }
                    tracing::info!("[VIBE] Phone turn: {} VIBE charged to project={}", tx.amount_vibe, project_id);
                }
            });
        }
    }

    // Persist messages to DB (fire-and-forget — don't block the response)
    if let Some(ref ctx) = db_ctx {
        let pool = deployment.db().pool.clone();
        let conversation_id = ctx.conversation_id;
        let speech_clone = speech_text.clone();
        let response_clone = nora_response.clone();

        tokio::spawn(async move {
            if let Err(e) =
                AgentConversationMessage::add_user_message(&pool, conversation_id, &speech_clone)
                    .await
            {
                warn!("Failed to persist user message: {}", e);
            }
            if let Err(e) = AgentConversationMessage::add_assistant_message(
                &pool,
                conversation_id,
                &response_clone,
                Some("claude-sonnet-4-20250514"),
                Some("anthropic"),
                None,
                None,
                None,
            )
            .await
            {
                warn!("Failed to persist assistant message: {}", e);
            }
        });
    }

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
    let audio_result = generate_and_cache_audio(&nora_response, Some(call_sid.clone())).await;

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

// ---------------------------------------------------------------------------
// handle_call_status (formerly handle_status_callback)
// ---------------------------------------------------------------------------

/// Handle call status callback from Twilio
///
/// POST /api/twilio/status
///
/// Finalises the CallLog, archives the AgentConversation, updates CRM.
pub async fn handle_call_status(
    State(deployment): State<DeploymentImpl>,
    Form(status): Form<TwilioStatusCallback>,
) -> impl IntoResponse {
    info!(
        "Call status update: {} -> {}",
        status.call_sid, status.call_status
    );

    // Notify the in-memory handler
    if let Some(handler) = get_twilio_handler().await {
        if let Err(e) = handler
            .handle_status_update(&status.call_sid, &status.call_status, status.call_duration)
            .await
        {
            warn!("Error handling status update: {}", e);
        }
    }

    // Clean up cached audio
    if matches!(
        status.call_status.as_str(),
        "completed" | "failed" | "busy" | "no-answer"
    ) {
        let cache = get_audio_cache().await;
        cache.cleanup_call(&status.call_sid).await;
        info!("Cleaned up audio cache for call {}", status.call_sid);
    }

    // Only do DB finalisation on completed calls
    if status.call_status != "completed" {
        return StatusCode::OK;
    }

    let pool = &deployment.db().pool;

    // Retrieve and remove CallDbContext + clear phone mapping
    let db_ctx = {
        let mut map = CALL_DB_CONTEXTS.lock().await;
        map.remove(status.call_sid.as_str())
    };
    if let Some(ref ctx) = db_ctx {
        let mut phones = ACTIVE_CALL_PHONES.lock().await;
        phones.remove(&ctx.caller_phone);
    }

    let db_ctx = match db_ctx {
        Some(ctx) => ctx,
        None => {
            warn!("No CallDbContext for completed call {}", status.call_sid);
            return StatusCode::OK;
        }
    };

    // Load all messages to build transcript
    let messages =
        AgentConversationMessage::find_recent(pool, db_ctx.conversation_id, 200)
            .await
            .unwrap_or_default();

    let transcript = messages
        .iter()
        .map(|m| format!("[{}]: {}", m.role.to_uppercase(), m.content))
        .collect::<Vec<_>>()
        .join("\n");

    // Finalise CallLog
    let duration = status.call_duration.unwrap_or(0) as i32;
    if let Err(e) = CallLog::update(
        pool,
        db_ctx.call_log_id,
        UpdateCallLog {
            status: Some(CallStatus::Completed),
            end_time: Some(Utc::now()),
            duration_seconds: Some(duration),
            transcription: Some(transcript),
            transcription_status: Some("completed".to_string()),
            crm_contact_id: Some(db_ctx.crm_contact_id),
            ..Default::default()
        },
    )
    .await
    {
        warn!("Failed to finalise call log {}: {}", db_ctx.call_log_id, e);
    }

    // Archive AgentConversation
    if let Err(e) = AgentConversation::update_status(
        pool,
        db_ctx.conversation_id,
        ConversationStatus::Archived,
    )
    .await
    {
        warn!("Failed to archive conversation {}: {}", db_ctx.conversation_id, e);
    }

    // Update CRM last_contacted_at
    if let Err(e) = CrmContact::record_contact_made(pool, db_ctx.crm_contact_id).await {
        warn!("Failed to update CRM contact {}: {}", db_ctx.crm_contact_id, e);
    }

    info!(
        "Call {} finalised: log={}, conversation={}, crm={}",
        status.call_sid, db_ctx.call_log_id, db_ctx.conversation_id, db_ctx.crm_contact_id
    );

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

/// Maximum time for LLM to respond (leave time for TTS after)
const LLM_TIMEOUT: Duration = Duration::from_secs(12);

/// System prompt for PCG team members — Nora as internal orchestrator
const NORA_PCG_TEAM_SYSTEM: &str = "\
You are Nora, PCG's Executive AI Assistant speaking with a member of the PCG team on a phone call. \
You are their intelligent operations assistant — you know their projects, tasks, and boards.\

You can help with: creating tasks, updating project status, checking what's in progress, \
scheduling work, summarising project activity, capturing meeting notes, and orchestrating \
agent workflows.\

The caller's active projects and tasks will be provided in the context. Reference them naturally.\

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
executives, and growing teams run their operations with intelligent agents.\

PCG's capabilities include: AI project management, autonomous agents that execute tasks, \
CRM and client management, content creation, social media management, financial tracking \
with VIBE tokens, team collaboration, and custom AI workflows.\

Your role on this call is to represent PCG warmly and professionally — understand the caller's \
goals, answer their questions, and help them see how PCG can help them. New callers can get \
their own PCG environment and their own AI assistant (Topsi) to manage their work.\

Rules for phone calls:\
- Keep every response to 2-3 SHORT sentences maximum\
- Be warm, natural, and conversational\
- Use British English\
- Never list more than 2-3 items at once — summarise instead\
- If asked about capabilities, give a brief compelling overview then ask what they need help with";

/// Process speech input by calling Anthropic directly — lean prompt, no context bloat
/// Returns (response_text, input_tokens, output_tokens)
async fn process_with_nora(
    speech_text: &str,
    _session_id: &str,
    phone_context: Option<serde_json::Value>,
) -> Result<(String, i64, i64), String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    // Pick system prompt based on caller type
    let caller_type = phone_context
        .as_ref()
        .and_then(|ctx| ctx.get("caller_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let system_prompt = if caller_type == "pcg_team" {
        NORA_PCG_TEAM_SYSTEM
    } else {
        NORA_CLIENT_SYSTEM
    };

    // Build conversation messages
    let mut messages = Vec::new();

    // For PCG team: inject their project/task context
    if caller_type == "pcg_team" {
        if let Some(ctx) = &phone_context {
            if let Some(work) = ctx.get("pcg_work") {
                let work_str = serde_json::to_string_pretty(work).unwrap_or_default();
                if !work_str.is_empty() && work_str != "null" {
                    messages.push(json!({
                        "role": "user",
                        "content": format!("[Your current PCG workspace:\n{}]", work_str)
                    }));
                    messages.push(json!({
                        "role": "assistant",
                        "content": "I have your workspace context — projects and active tasks loaded."
                    }));
                }
            }
        }
    }

    // Include prior turn history if available
    if let Some(ctx) = &phone_context {
        if let Some(history) = ctx.get("conversation_history").and_then(|h| h.as_str()) {
            if !history.is_empty() {
                messages.push(json!({
                    "role": "user",
                    "content": format!("[Previous conversation context:\n{}]", history)
                }));
                messages.push(json!({
                    "role": "assistant",
                    "content": "Understood, I have the conversation context."
                }));
            }
        }
    }

    // Caller info prefix
    let caller_note = phone_context
        .as_ref()
        .and_then(|ctx| ctx.get("caller"))
        .map(|c| {
            let name = c.get("caller_name").and_then(|v| v.as_str()).unwrap_or("");
            let role = c.get("caller_role").and_then(|v| v.as_str()).unwrap_or("");
            if !name.is_empty() && name != "Unknown Caller" {
                match role {
                    "pcg_admin" => format!("[PCG Admin: {}] ", name),
                    "pcg_team" => format!("[PCG Team: {}] ", name),
                    "returning_client" => {
                        let prev = c.get("previous_calls").and_then(|v| v.as_i64()).unwrap_or(0);
                        format!("[Returning client: {}, {} previous calls] ", name, prev)
                    }
                    _ => format!("[New caller: {}] ", name),
                }
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    messages.push(json!({
        "role": "user",
        "content": format!("{}{}", caller_note, speech_text)
    }));

    let body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 250,
        "system": system_prompt,
        "messages": messages
    });

    let client = reqwest::Client::new();
    let fut = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send();

    let resp = match timeout(LLM_TIMEOUT, fut).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return Err(format!("HTTP error: {}", e)),
        Err(_) => {
            warn!("LLM timeout after {:?}", LLM_TIMEOUT);
            return Ok(("I'm just pulling that information up — could you give me one moment?".to_string(), 0, 0));
        }
    };

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

    let input_tokens = data["usage"]["input_tokens"].as_i64().unwrap_or(0);
    let output_tokens = data["usage"]["output_tokens"].as_i64().unwrap_or(0);
    let text = data["content"][0]["text"]
        .as_str()
        .unwrap_or("I'm sorry, I didn't quite catch that. Could you say that again?")
        .to_string();

    Ok((text, input_tokens, output_tokens))
}

// ─────────────────────────────────────────────────────────────────────────────
// SMS Handling
// ─────────────────────────────────────────────────────────────────────────────

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

/// POST /twilio/sms — Handle incoming SMS messages
pub async fn handle_incoming_sms(
    State(deployment): State<DeploymentImpl>,
    Form(request): Form<TwilioSmsRequest>,
) -> impl IntoResponse {
    info!(
        "Incoming SMS: {} from {} (sid={})",
        &request.body[..request.body.len().min(80)],
        request.from,
        request.message_sid
    );

    let pool = &deployment.db().pool;

    // ── Check if sender is on an active call ─────────────────────────────────
    let active_call_sid = {
        let phones = ACTIVE_CALL_PHONES.lock().await;
        phones.get(&request.from).cloned()
    };

    if let Some(call_sid) = active_call_sid {
        // Caller is mid-call — ingest and queue the SMS for Nora to use
        info!("SMS from {} received during active call {} — queuing for Nora", request.from, call_sid);

        let sms_queue = {
            let map = CALL_DB_CONTEXTS.lock().await;
            map.get(&call_sid).map(|ctx| ctx.sms_queue.clone())
        };

        if let Some(queue) = sms_queue {
            // Spawn content ingestion so we don't block Twilio's webhook timeout

            let body = request.body.clone();
            let from = request.from.clone();
            tokio::spawn(async move {
                let ingested = ingest_sms_content(&body).await;
                let mut q = queue.lock().await;
                q.push(InCallSms {
                    body,
                    from,
                    received_at: chrono::Utc::now(),
                    ingested_content: ingested,
                });
            });
            let twiml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Response>\
                <Message>Got it — I'll bring that into our conversation now.</Message>\

                </Response>";
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml.to_string());
        }
    }

    // ── No active call — SMS thread buffering + Nora orchestration ───────────
    // Collect MMS media attachments (provider sends MediaUrl0..N as form fields)
    let media_attachments: Vec<(String, Option<String>)> = [
        (request.media_url_0.clone(), request.media_content_type_0.clone()),
        (request.media_url_1.clone(), request.media_content_type_1.clone()),
        (request.media_url_2.clone(), request.media_content_type_2.clone()),
    ]
    .into_iter()
    .filter_map(|(url, ct)| url.map(|u| (u, ct)))
    .collect();

    if !media_attachments.is_empty() {
        info!("SMS from {} includes {} media attachment(s)", request.from, media_attachments.len());
    }

    // Resolve sender identity (persons > CRM > pcg_team)
    let person_context = lookup_sms_sender_context(pool, &request.from).await;
    let caller_name = person_context
        .as_ref()
        .and_then(|c| c.get("name").and_then(|v| v.as_str()))
        .unwrap_or("there")
        .to_string();

    // Buffer message + media URLs immediately — image fetch happens inside the debounce
    // spawn so we don't block the webhook response (SignalWire times out after ~15s)
    let msg_count = {
        let mut buffer = SMS_THREAD_BUFFER.lock().await;
        let entry = buffer.entry(request.from.clone()).or_insert_with(|| SmsThread {
            messages: Vec::new(),
            pending_media: Vec::new(),
            last_received: SystemTime::now(),
            person_context: person_context.clone(),
        });
        entry.messages.push(request.body.clone());
        // Accumulate media from all messages in the thread
        entry.pending_media.extend(media_attachments);
        entry.last_received = SystemTime::now();
        if entry.person_context.is_none() && person_context.is_some() {
            entry.person_context = person_context.clone();
        }
        entry.messages.len()
    };

    info!("SMS thread from {}: {} message(s) buffered", request.from, msg_count);

    // Spawn debounced processor — each message spawns one; only the "last" one processes
    let from_clone = request.from.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(SMS_THREAD_WINDOW_SECS)).await;

        // Drain buffer only if we're still the most recent processor
        let thread = {
            let mut buffer = SMS_THREAD_BUFFER.lock().await;
            let should_process = buffer.get(&from_clone).map(|t| {
                t.last_received
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs()
                    >= SMS_THREAD_WINDOW_SECS - 5
            }).unwrap_or(false);
            if should_process { buffer.remove(&from_clone) } else { None }
        };

        if let Some(thread) = thread {
            // Fetch and process all media attachments (async, outside webhook handler)
            let media_result = if !thread.pending_media.is_empty() {
                info!("Fetching {} media attachment(s) for SMS thread from {}", thread.pending_media.len(), from_clone);
                fetch_and_describe_media(thread.pending_media).await
            } else {
                MediaResult { text_content: None, image_description: None }
            };

            // Build combined message — start with buffered SMS bodies
            let mut combined = thread.messages.iter()
                .enumerate()
                .map(|(i, m)| format!("[{}] {}", i + 1, m))
                .collect::<Vec<_>>()
                .join("\n\n");

            // Append text content from text/plain media (SignalWire sends message body this way)
            if let Some(text) = media_result.text_content {
                info!("Recovered text from media attachment ({} chars): {:?}", text.len(), &text[..text.len().min(80)]);
                combined = if combined.trim().is_empty() {
                    text
                } else {
                    format!("{}\n\n{}", combined, text)
                };
            }

            // Append image description
            if let Some(desc) = media_result.image_description {
                info!("Image described ({} chars), appending to thread", desc.len());
                combined = format!("{}\n\n[Attached image: {}]", combined, desc);
            }

            let sender_label = thread.person_context
                .as_ref()
                .and_then(|c| c.get("name").and_then(|v| v.as_str()))
                .map(|n| format!("{} ({})", n, from_clone))
                .unwrap_or_else(|| from_clone.clone());

            let nora_content = format!(
                "[SMS THREAD from {} — {} message(s)]\n\n{}\n\n\
                [Channel: SMS. Reply in plain text only, no markdown. \
                Be concise — under 300 characters where possible.]",
                sender_label,
                thread.messages.len(),
                combined
            );

            let reply = match process_sms_with_nora(&nora_content, &from_clone, thread.person_context).await {
                Ok(text) => truncate_for_sms(&text, 320),
                Err(e) => {
                    error!("SMS Nora processing failed for {}: {}", from_clone, e);
                    "I hit a snag processing your request — please try again shortly.".to_string()
                }
            };

            if let Err(e) = send_outbound_sms(&from_clone, &reply).await {
                error!("Failed to send outbound SMS to {}: {}", from_clone, e);
            }
        }
    });

    // Return empty TwiML immediately — Nora replies via outbound SMS only
    (StatusCode::OK, [("Content-Type", "application/xml")], "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Response/>".to_string())
}

/// Fetch MMS media from SignalWire/Twilio (requires basic auth), then ask
/// Claude Vision to describe the images. Returns a combined text description.
/// Result of processing MMS media attachments.
struct MediaResult {
    /// Text extracted from text/plain media attachments (the user's actual message body)
    pub text_content: Option<String>,
    /// Claude Vision description of any image attachments
    pub image_description: Option<String>,
}

/// Fetch and process all MMS media attachments from SignalWire/Twilio.
/// - text/plain → fetched and returned as the user's message text
/// - image/* → fetched, base64-encoded, described via Claude Vision
async fn fetch_and_describe_media(media: Vec<(String, Option<String>)>) -> MediaResult {
    if media.is_empty() {
        return MediaResult { text_content: None, image_description: None };
    }

    let account_sid = match std::env::var("TWILIO_ACCOUNT_SID") {
        Ok(v) => v,
        Err(_) => return MediaResult { text_content: None, image_description: None },
    };
    let auth_token = match std::env::var("TWILIO_AUTH_TOKEN") {
        Ok(v) => v,
        Err(_) => return MediaResult { text_content: None, image_description: None },
    };
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .ok();

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(_) => return MediaResult { text_content: None, image_description: None },
    };

    let mut image_blocks: Vec<serde_json::Value> = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();

    for (url, content_type) in media.iter().take(5) {
        let mime = content_type.as_deref().unwrap_or("application/octet-stream");

        if mime.starts_with("text/plain") {
            // This is the user's message body sent as a media attachment by SignalWire
            match client.get(url).basic_auth(&account_sid, Some(&auth_token)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(text) = resp.text().await {
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            info!("Fetched text/plain media ({} chars): {:?}", trimmed.len(), &trimmed[..trimmed.len().min(100)]);
                            text_parts.push(trimmed);
                        }
                    }
                }
                Ok(resp) => warn!("Text media fetch {} returned HTTP {}", url, resp.status()),
                Err(e) => warn!("Text media fetch error for {}: {}", url, e),
            }
        } else if mime.starts_with("image/") {
            match client.get(url).basic_auth(&account_sid, Some(&auth_token)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        image_blocks.push(serde_json::json!({
                            "type": "image",
                            "source": { "type": "base64", "media_type": mime, "data": b64 }
                        }));
                    }
                }
                Ok(resp) => warn!("Image fetch {} returned HTTP {}", url, resp.status()),
                Err(e) => warn!("Image fetch error for {}: {}", url, e),
            }
        } else {
            info!("Skipping unsupported media type: {} ({})", url, mime);
        }
    }

    let text_content = if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) };

    // Describe images with Claude Vision if we have any
    let image_description = if image_blocks.is_empty() || api_key.is_none() {
        None
    } else {
        let mut content = image_blocks;
        content.push(serde_json::json!({
            "type": "text",
            "text": "Describe what you see in these image(s) in detail. Include subject matter, \
                     any visible text or numbers, colours, composition, and any context useful \
                     for someone who hasn't seen the image. Be thorough but concise."
        }));

        let api_key = api_key.unwrap();
        let body = serde_json::json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": content }]
        });

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .ok();

        match resp {
            Some(r) => {
                let data: serde_json::Value = r.json().await.unwrap_or_default();
                data["content"][0]["text"].as_str().map(|s| s.to_string())
            }
            None => None,
        }
    };

    MediaResult { text_content, image_description }
}

/// Fetch and extract readable content from any URLs in an SMS body.
/// Returns a summarised string of ingested content, or None if no URLs found.
async fn ingest_sms_content(body: &str) -> Option<String> {
    // Find URLs in the message
    let urls: Vec<&str> = body.split_whitespace()
        .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
        .collect();

    if urls.is_empty() {
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (compatible; NoraBot/1.0)")
        .build()
        .ok()?;

    let mut parts = Vec::new();

    for url in urls.iter().take(3) {
        match client.get(*url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let content_type = resp.headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();

                if content_type.contains("text/html") || content_type.contains("text/plain") {
                    if let Ok(text) = resp.text().await {
                        let extracted = extract_text_from_html(&text);
                        // Truncate to 1500 chars per URL to keep context manageable
                        let snippet = if extracted.len() > 1500 {
                            format!("{}…", &extracted[..1500])
                        } else {
                            extracted
                        };
                        parts.push(format!("[Content from {}]:\n{}", url, snippet));
                    }
                } else {
                    parts.push(format!("[Link {} — content type: {}]", url, content_type));
                }
            }
            Ok(resp) => {
                parts.push(format!("[Link {} — HTTP {}]", url, resp.status()));
            }
            Err(e) => {
                parts.push(format!("[Link {} — fetch error: {}]", url, e));
            }
        }
    }

    if parts.is_empty() { None } else { Some(parts.join("\n\n")) }
}

/// Strip HTML tags and collapse whitespace to get readable text.
fn extract_text_from_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_buf = String::new();

    for c in html.chars() {
        match c {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag_lower = tag_buf.trim().to_lowercase();
                if tag_lower.starts_with("script") || tag_lower.starts_with("style") {
                    in_script_or_style = true;
                } else if tag_lower.starts_with("/script") || tag_lower.starts_with("/style") {
                    in_script_or_style = false;
                } else if tag_lower == "br" || tag_lower == "p" || tag_lower == "/p"
                    || tag_lower.starts_with("h") || tag_lower.starts_with("/h")
                {
                    out.push('\n');
                }
            }
            _ if in_tag => tag_buf.push(c),
            _ if in_script_or_style => {}
            _ => out.push(c),
        }
    }

    // Collapse whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Process an SMS through the real Nora agent (with full tool access),
/// falling back to a direct Claude API call if Nora is unavailable.
async fn process_sms_with_nora(
    message: &str,
    from_number: &str,
    context: Option<serde_json::Value>,
) -> Result<String, String> {
    let caller_type = context
        .as_ref()
        .and_then(|ctx| ctx.get("caller_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("client");

    let is_team = caller_type == "pcg_team" || caller_type == "pcg_admin";

    let caller_note = context
        .as_ref()
        .and_then(|ctx| ctx.get("name"))
        .and_then(|v| v.as_str())
        .filter(|n| !n.is_empty())
        .map(|name| format!("[SMS from {} ({})] ", name, from_number))
        .unwrap_or_else(|| format!("[SMS from {}] ", from_number));

    let full_content = format!("{}{}", caller_note, message);

    // ── Route through the real Nora agent (has tools — can create tasks, etc.) ─
    let nora_result: Option<String> = async {
        let nora_arc = get_nora_instance().await.ok()?;
        let guard = nora_arc.read().await;
        let nora = guard.as_ref()?;

        // Always use TextInteraction — it calls process_text_with_tools which reads the
        // actual message content via LLM + tools. TaskCoordination ignores request.content
        // and returns a canned template response.
        let request_type = NoraRequestType::TextInteraction;

        let nora_req = NoraRequest {
            request_id: Uuid::new_v4().to_string(),
            session_id: format!("sms-{}", from_number),
            request_type,
            content: full_content.clone(),
            context: context.clone(),
            voice_enabled: false,
            priority: if is_team { RequestPriority::High } else { RequestPriority::Normal },
            timestamp: chrono::Utc::now(),
        };

        match timeout(Duration::from_secs(55), nora.process_request(nora_req)).await {
            Ok(Ok(response)) => Some(response.content),
            Ok(Err(e)) => { error!("Nora agent error on SMS from {}: {}", from_number, e); None }
            Err(_) => { warn!("Nora agent timed out on SMS from {}", from_number); None }
        }
    }.await;

    if let Some(text) = nora_result {
        return Ok(text);
    }

    // ── Fallback: direct Claude API call ──────────────────────────────────────
    info!("Falling back to direct Claude API for SMS from {}", from_number);

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let system_prompt = if is_team { NORA_PCG_TEAM_SYSTEM } else { NORA_CLIENT_SYSTEM };
    let sms_instruction = "[SMS channel — reply as plain text, no markdown, \
        keep under 300 characters. British English.]";

    let body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 512,
        "system": format!("{}\n\n{}", system_prompt, sms_instruction),
        "messages": [{ "role": "user", "content": full_content }]
    });

    let client = reqwest::Client::new();
    let fut = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send();

    let resp = match timeout(Duration::from_secs(20), fut).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return Err(format!("HTTP error: {}", e)),
        Err(_) => return Ok("I'm just catching up — please send again in a moment.".into()),
    };

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;
    let text = data["content"][0]["text"]
        .as_str()
        .unwrap_or("Sorry, I didn't quite catch that. Could you rephrase?")
        .to_string();

    Ok(text)
}


/// Escape special XML characters for TwiML body
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
                    while let Some(ch) = chars.next() {
                        if ch == ')' { break; }
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

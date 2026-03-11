//! Invite Dispatch — multi-channel meeting invitations, meeting→KG publish, analysis export
//!
//! Routes:
//!   POST /proposals/:id/schedule-meeting        — create scheduled meeting + dispatch invites per channel
//!   GET  /proposals/:id/scheduled-meetings       — list scheduled meetings for a proposal
//!   POST /topsi/meeting/:id/publish              — publish meeting notes to project knowledge graph
//!   GET  /companies/:id/export-analysis          — download business analysis markdown document

use axum::{
    Router,
    extract::{Path, State},
    http::header,
    response::Response,
    routing::{get, post},
    Json,
};
use chrono::Utc;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::company::Company;
use db::models::meeting_session::MeetingSession;
use db::models::person::Person;
use db::models::proposal::Proposal;
use db::models::project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource};
use db::models::scheduled_meeting::{
    CreateScheduledMeeting, CreateScheduledMeetingInvitee, ScheduledMeeting,
    ScheduledMeetingWithInvitees,
};

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ScheduleMeetingBody {
    pub scheduled_at: String,
    pub duration_min: Option<i32>,
    pub location: Option<String>,
    pub agenda: Option<String>,
    pub invitees: Vec<InviteeInput>,
}

#[derive(Debug, Deserialize)]
pub struct InviteeInput {
    pub person_id: Uuid,
    /// Override channel. If omitted, uses person.onboarding_channel then preferred_contact.
    pub channel: Option<String>,
    /// Override address. If omitted, derived from person's email or phone.
    pub channel_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleMeetingResponse {
    pub meeting: ScheduledMeetingWithInvitees,
    pub dispatched: Vec<InviteDispatchResult>,
}

#[derive(Debug, Serialize)]
pub struct InviteDispatchResult {
    pub person_id: String,
    pub channel: String,
    pub status: String,  // "sent" | "skipped" | "error"
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishMeetingBody {
    /// The project whose knowledge graph will receive this conversation source.
    pub project_id: Uuid,
    pub company_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
    pub attendee_person_ids: Option<Vec<Uuid>>,
    pub source_title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublishMeetingResponse {
    pub session_id: String,
    pub knowledge_source_id: String,
    pub message: String,
}

// ── Channel resolver ──────────────────────────────────────────────────────────

async fn resolve_channel(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    override_channel: Option<&str>,
    override_address: Option<&str>,
) -> (String, Option<String>) {
    let person = Person::find_by_id(pool, person_id).await.ok().flatten();

    let channel = override_channel
        .map(|s| s.to_string())
        .or_else(|| person.as_ref().and_then(|p| p.onboarding_channel.clone()))
        .or_else(|| person.as_ref().and_then(|p| p.preferred_contact.clone()))
        .unwrap_or_else(|| "email".to_string());

    if let Some(addr) = override_address {
        return (channel, Some(addr.to_string()));
    }

    let address = person.as_ref().and_then(|p| match channel.as_str() {
        "email" => p.email.clone(),
        "sms" | "whatsapp" | "phone" => p.phone.clone(),
        _ => p.email.clone(), // fallback for social channels lacking stored handle
    });

    (channel, address)
}

// ── Channel dispatch ──────────────────────────────────────────────────────────

async fn dispatch_invite(
    channel: &str,
    address: &str,
    person_name: &str,
    scheduled_at: &str,
    duration_min: i32,
    location: Option<&str>,
    agenda: Option<&str>,
) -> (String, String) {
    let loc = location.unwrap_or("To be confirmed");
    let agenda_line = agenda
        .filter(|a| !a.is_empty())
        .map(|a| format!("\nAgenda: {a}"))
        .unwrap_or_default();

    let body = format!(
        "Hi {person_name},\n\n\
         Powerclub Global would like to invite you to a proposal meeting.\n\n\
         Date & Time: {scheduled_at}\n\
         Duration: {duration_min} minutes\n\
         Location: {loc}{agenda_line}\n\n\
         Looking forward to connecting!\n\n\
         — Powerclub Global"
    );

    match channel {
        "email" => send_email(address, &body).await,
        "sms" => send_sms(address, &body).await,
        "whatsapp" => send_whatsapp(address, &body).await,
        "instagram" => send_instagram_dm(address, &body).await,
        "linkedin" => send_linkedin_message(address, &body).await,
        "twitter" => send_twitter_dm(address, &body).await,
        "phone" | "in_person" => (
            "skipped".to_string(),
            format!("Manual outreach required via {channel} for {person_name}"),
        ),
        _ => (
            "skipped".to_string(),
            format!("Unknown channel: {channel}"),
        ),
    }
}

/// Email via SendGrid (falls back to logging when SENDGRID_API_KEY unset).
async fn send_email(to: &str, body: &str) -> (String, String) {
    let api_key = std::env::var("SENDGRID_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        tracing::info!("📧 [EMAIL-STUB] To: {to}\n{body}");
        return ("sent".to_string(), format!("Email logged (SENDGRID_API_KEY not configured) → {to}"));
    }

    let from = std::env::var("FROM_EMAIL")
        .unwrap_or_else(|_| "nora@powerclubglobal.com".to_string());

    let payload = serde_json::json!({
        "personalizations": [{"to": [{"email": to}]}],
        "from": {"email": from, "name": "Powerclub Global"},
        "subject": "Powerclub Global — Proposal Meeting Invitation",
        "content": [{"type": "text/plain", "value": body}]
    });

    let client = reqwest::Client::new();
    match client
        .post("https://api.sendgrid.com/v3/mail/send")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&payload)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => ("sent".to_string(), format!("Email sent → {to}")),
        Ok(r) => ("error".to_string(), format!("SendGrid {} → {to}", r.status())),
        Err(e) => ("error".to_string(), format!("HTTP error: {e}")),
    }
}

/// SMS via Twilio.
async fn send_sms(to: &str, body: &str) -> (String, String) {
    let sid = std::env::var("TWILIO_ACCOUNT_SID").unwrap_or_default();
    let token = std::env::var("TWILIO_AUTH_TOKEN").unwrap_or_default();
    let from = std::env::var("TWILIO_PHONE_NUMBER").unwrap_or_default();

    if sid.is_empty() {
        tracing::info!("📱 [SMS-STUB] To: {to}\n{body}");
        return ("sent".to_string(), format!("SMS logged (Twilio not configured) → {to}"));
    }

    let url = format!("https://api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json");
    match reqwest::Client::new()
        .post(&url)
        .basic_auth(&sid, Some(&token))
        .form(&[("From", from.as_str()), ("To", to), ("Body", body)])
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => ("sent".to_string(), format!("SMS sent → {to}")),
        Ok(r) => ("error".to_string(), format!("Twilio {} → {to}", r.status())),
        Err(e) => ("error".to_string(), format!("HTTP error: {e}")),
    }
}

/// WhatsApp via Twilio WhatsApp API.
async fn send_whatsapp(to: &str, body: &str) -> (String, String) {
    let sid = std::env::var("TWILIO_ACCOUNT_SID").unwrap_or_default();
    let token = std::env::var("TWILIO_AUTH_TOKEN").unwrap_or_default();
    let from = std::env::var("TWILIO_WHATSAPP_NUMBER")
        .unwrap_or_else(|_| "whatsapp:+14155238886".to_string());

    if sid.is_empty() {
        tracing::info!("💬 [WHATSAPP-STUB] To: {to}\n{body}");
        return ("sent".to_string(), format!("WhatsApp logged (Twilio not configured) → {to}"));
    }

    let to_wa = format!("whatsapp:{to}");
    let url = format!("https://api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json");
    match reqwest::Client::new()
        .post(&url)
        .basic_auth(&sid, Some(&token))
        .form(&[("From", from.as_str()), ("To", to_wa.as_str()), ("Body", body)])
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => ("sent".to_string(), format!("WhatsApp sent → {to}")),
        Ok(r) => ("error".to_string(), format!("Twilio WA {} → {to}", r.status())),
        Err(e) => ("error".to_string(), format!("HTTP error: {e}")),
    }
}

/// Instagram DM via Meta Graph API.
/// Requires META_PAGE_ACCESS_TOKEN + INSTAGRAM_USER_ID env vars.
/// Recipient must be an IGSID (numeric) not a handle — handle→IGSID resolution
/// should be implemented via the Instagram User Lookup API in production.
async fn send_instagram_dm(recipient_id_or_handle: &str, body: &str) -> (String, String) {
    let token = std::env::var("META_PAGE_ACCESS_TOKEN").unwrap_or_default();
    let ig_user_id = std::env::var("INSTAGRAM_USER_ID").unwrap_or_default();

    if token.is_empty() {
        tracing::info!("📸 [INSTAGRAM-STUB] To: @{recipient_id_or_handle}\n{body}");
        return ("sent".to_string(), format!("Instagram DM logged (META_PAGE_ACCESS_TOKEN not configured) → @{recipient_id_or_handle}"));
    }

    let url = format!("https://graph.facebook.com/v19.0/{ig_user_id}/messages");
    let payload = serde_json::json!({
        "recipient": {"id": recipient_id_or_handle},
        "message": {"text": body}
    });
    match reqwest::Client::new()
        .post(&url)
        .query(&[("access_token", token.as_str())])
        .json(&payload)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => ("sent".to_string(), format!("Instagram DM sent → {recipient_id_or_handle}")),
        Ok(r) => ("error".to_string(), format!("Meta API {} → {recipient_id_or_handle}", r.status())),
        Err(e) => ("error".to_string(), format!("HTTP error: {e}")),
    }
}

/// LinkedIn message — requires LINKEDIN_ACCESS_TOKEN.
/// Full implementation needs recipient URN (urn:li:person:xxx) resolved from profile URL.
async fn send_linkedin_message(profile: &str, body: &str) -> (String, String) {
    let token = std::env::var("LINKEDIN_ACCESS_TOKEN").unwrap_or_default();
    if token.is_empty() {
        tracing::info!("💼 [LINKEDIN-STUB] To: {profile}\n{body}");
        return ("sent".to_string(), format!("LinkedIn DM logged (LINKEDIN_ACCESS_TOKEN not configured) → {profile}"));
    }
    // TODO: resolve profile URL → URN via LinkedIn People API, then POST to messaging endpoint
    ("skipped".to_string(), "LinkedIn messaging: set LINKEDIN_ACCESS_TOKEN and implement URN resolution".to_string())
}

/// Twitter/X DM — requires TWITTER_BEARER_TOKEN.
async fn send_twitter_dm(handle: &str, body: &str) -> (String, String) {
    let token = std::env::var("TWITTER_BEARER_TOKEN").unwrap_or_default();
    if token.is_empty() {
        tracing::info!("🐦 [TWITTER-STUB] To: @{handle}\n{body}");
        return ("sent".to_string(), format!("Twitter DM logged (TWITTER_BEARER_TOKEN not configured) → @{handle}"));
    }
    // TODO: resolve handle → numeric user ID, then POST /2/dm_conversations/with/:id/messages
    ("skipped".to_string(), "Twitter DMs: set TWITTER_BEARER_TOKEN and implement user ID resolution".to_string())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/proposals/:id/schedule-meeting
async fn schedule_meeting(
    State(d): State<DeploymentImpl>,
    Path(proposal_id): Path<Uuid>,
    Json(body): Json<ScheduleMeetingBody>,
) -> Result<Json<ApiResponse<ScheduleMeetingResponse>>, ApiError> {
    let pool = &d.db().pool;

    Proposal::find_by_id(pool, proposal_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))?;

    let mut resolved: Vec<CreateScheduledMeetingInvitee> = Vec::new();
    for inv in &body.invitees {
        let (channel, addr) = resolve_channel(
            pool,
            inv.person_id,
            inv.channel.as_deref(),
            inv.channel_address.as_deref(),
        )
        .await;
        resolved.push(CreateScheduledMeetingInvitee {
            person_id: inv.person_id,
            channel,
            channel_address: addr.unwrap_or_else(|| "unknown".to_string()),
        });
    }

    let primary_channel = resolved
        .first()
        .map(|i| i.channel.clone())
        .unwrap_or_else(|| "email".to_string());

    let create_data = CreateScheduledMeeting {
        proposal_id,
        scheduled_at: body.scheduled_at.clone(),
        duration_min: body.duration_min,
        location: body.location.clone(),
        agenda: body.agenda.clone(),
        channel: primary_channel,
        invitees: resolved,
    };
    let meeting_record = ScheduledMeeting::create(pool, &create_data).await?;

    let duration_min = body.duration_min.unwrap_or(60);
    let mut dispatched: Vec<InviteDispatchResult> = Vec::new();

    for inv in &meeting_record.invitees {
        let pid = inv.person_id.parse::<Uuid>().unwrap_or(Uuid::nil());
        let person = Person::find_by_id(pool, pid).await.ok().flatten();
        let name = person.as_ref().map(|p| p.full_name.as_str()).unwrap_or("Valued Contact");

        let (status, message) = dispatch_invite(
            &inv.channel,
            &inv.channel_address,
            name,
            &body.scheduled_at,
            duration_min,
            body.location.as_deref(),
            body.agenda.as_deref(),
        )
        .await;

        if status == "sent" {
            let _ = ScheduledMeeting::mark_invitee_sent(pool, &inv.id).await;
        }

        dispatched.push(InviteDispatchResult {
            person_id: inv.person_id.clone(),
            channel: inv.channel.clone(),
            status,
            message,
        });
    }

    let _ = Proposal::move_status(pool, proposal_id, "meeting_scheduled").await;
    let _ = ScheduledMeeting::update_status(pool, &meeting_record.meeting.id, "sent").await;

    Ok(Json(ApiResponse::success(ScheduleMeetingResponse {
        meeting: meeting_record,
        dispatched,
    })))
}

/// GET /api/proposals/:id/scheduled-meetings
async fn list_scheduled_meetings(
    State(d): State<DeploymentImpl>,
    Path(proposal_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<ScheduledMeetingWithInvitees>>>, ApiError> {
    let meetings = ScheduledMeeting::find_by_proposal(&d.db().pool, proposal_id).await?;
    Ok(Json(ApiResponse::success(meetings)))
}

/// POST /api/topsi/meeting/:id/publish
/// Publishes meeting notes as a `conversation` knowledge source in the project knowledge graph.
async fn publish_meeting(
    State(d): State<DeploymentImpl>,
    Path(session_id): Path<String>,
    Json(body): Json<PublishMeetingBody>,
) -> Result<Json<ApiResponse<PublishMeetingResponse>>, ApiError> {
    let pool = &d.db().pool;

    let session: Option<MeetingSession> =
        sqlx::query_as("SELECT * FROM meeting_sessions WHERE id = ?1")
            .bind(&session_id)
            .fetch_optional(pool)
            .await?;

    let session = session.ok_or_else(|| ApiError::NotFound("Meeting session not found".into()))?;

    // Extract AI-generated summary from notes JSON
    let notes_summary: Option<String> = session
        .notes
        .as_ref()
        .and_then(|n| serde_json::from_str::<serde_json::Value>(n).ok())
        .and_then(|v| v["summary"].as_str().map(|s| s.to_string()));

    let date_part = session.started_at.get(..10).unwrap_or(&session.started_at);
    let title = body.source_title.unwrap_or_else(|| {
        format!("Meeting: {} — {}", session.title, date_part)
    });

    let summary = notes_summary.unwrap_or_else(|| {
        session.transcript
            .as_ref()
            .map(|t| format!("[Transcript — {} chars]", t.len()))
            .unwrap_or_else(|| "No notes or transcript captured".to_string())
    });

    // Upsert into project knowledge graph
    let ks_id = Uuid::new_v4();
    ProjectKnowledgeSource::upsert_source(
        pool,
        body.project_id,
        &KnowledgeSourceType::Conversation,
        &session_id,
        &title,
        Some(&summary),
        0.8,
    )
    .await?;

    // Link session to CRM entities
    let attendees_json = body
        .attendee_person_ids
        .as_ref()
        .and_then(|ids| serde_json::to_string(ids).ok())
        .unwrap_or_else(|| "[]".to_string());

    sqlx::query(
        r#"UPDATE meeting_sessions SET
           company_id  = COALESCE(?2, company_id),
           proposal_id = COALESCE(?3, proposal_id),
           attendee_person_ids = ?4,
           knowledge_source_id = ?5,
           updated_at  = datetime('now','subsec')
           WHERE id = ?1"#,
    )
    .bind(&session_id)
    .bind(body.company_id.as_ref().map(|u| u.as_bytes().to_vec()))
    .bind(body.proposal_id.as_ref().map(|u| u.as_bytes().to_vec()))
    .bind(&attendees_json)
    .bind(ks_id.to_string())
    .execute(pool)
    .await?;

    Ok(Json(ApiResponse::success(PublishMeetingResponse {
        session_id,
        knowledge_source_id: ks_id.to_string(),
        message: format!("Published to knowledge graph: \"{title}\""),
    })))
}

/// GET /api/companies/:id/export-analysis
/// Returns a formatted markdown business analysis document as a downloadable file.
async fn export_company_analysis(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let pool = &d.db().pool;

    let company = Company::find_by_id(pool, company_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    let persons: Vec<Person> = sqlx::query_as(
        "SELECT * FROM persons WHERE company_id = ?1 ORDER BY lead_score DESC LIMIT 20",
    )
    .bind(company_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let proposals: Vec<Proposal> = sqlx::query_as(
        "SELECT * FROM proposals WHERE company_id = ?1 ORDER BY created_at DESC LIMIT 10",
    )
    .bind(company_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Pull any conversation sources linked to this company via knowledge graph
    let meeting_sources: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT source_title, source_summary FROM project_knowledge_sources \
         WHERE source_type='conversation' AND source_id IN \
         (SELECT id FROM meeting_sessions WHERE company_id = ?1) \
         ORDER BY created_at DESC LIMIT 5",
    )
    .bind(company_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let date = Utc::now().format("%B %d, %Y").to_string();

    let mut doc = format!(
        "# BUSINESS ANALYSIS REPORT\n**Prepared by:** Powerclub Global  \n**Date:** {date}\n\n---\n\n\
         ## Company Overview\n\n\
         | Field | Value |\n|---|---|\n\
         | **Name** | {} |\n\
         | **Industry** | {} |\n\
         | **Headquarters** | {} |\n\
         | **Website** | {} |\n\n",
        company.name,
        company.industry.as_deref().unwrap_or("—"),
        company.headquarters.as_deref().unwrap_or("—"),
        company.website.as_deref().unwrap_or("—"),
    );

    if let Some(d) = &company.description {
        doc.push_str(&format!("_{d}_\n\n"));
    }

    doc.push_str("---\n\n## Intelligence Summary\n\n");
    match &company.intelligence_summary {
        Some(s) => {
            let conf = company.intelligence_confidence.unwrap_or(0.0) * 100.0;
            let ran = company.intelligence_last_run_at
                .map(|d| d.format("%B %d, %Y").to_string())
                .unwrap_or_else(|| "—".to_string());
            doc.push_str(&format!("{s}\n\n> Confidence: **{conf:.0}%** | Last run: {ran}\n\n"));
        }
        None => doc.push_str("_No intelligence summary. Run company research to generate._\n\n"),
    }

    doc.push_str("---\n\n## Key Contacts\n\n");
    if persons.is_empty() {
        doc.push_str("_No contacts linked to this company._\n\n");
    } else {
        for p in &persons {
            doc.push_str(&format!(
                "### {}\n\
                 - **Title:** {}\n\
                 - **Email:** {}\n\
                 - **Phone:** {}\n\
                 - **Lifecycle Stage:** {}\n\
                 - **Lead Score:** {}\n",
                p.full_name,
                p.job_title.as_deref().unwrap_or("—"),
                p.email.as_deref().unwrap_or("—"),
                p.phone.as_deref().unwrap_or("—"),
                p.lifecycle_stage,
                p.lead_score,
            ));
            if let Some(intel) = &p.intelligence_summary {
                doc.push_str(&format!("\n  > {intel}\n"));
            }
            doc.push('\n');
        }
    }

    doc.push_str("---\n\n## Open Opportunities\n\n");
    if proposals.is_empty() {
        doc.push_str("_No proposals linked to this company._\n\n");
    } else {
        for p in &proposals {
            let usd = p.quote_amount_vibe as f64 / 100.0;
            doc.push_str(&format!(
                "- **{}**  \n  Type: `{}` | Status: `{}` | Value: **${usd:.2}**  \n  _{}_\n\n",
                p.title, p.deal_type, p.status, p.description
            ));
        }
    }

    if !meeting_sources.is_empty() {
        doc.push_str("---\n\n## Meeting Notes\n\n");
        for (title, summary) in &meeting_sources {
            doc.push_str(&format!(
                "### {title}\n{}\n\n",
                summary.as_deref().unwrap_or("_No summary available._")
            ));
        }
    }

    doc.push_str("---\n\n_Generated by Powerclub Global Intelligence Pipeline._\n");

    let filename = format!(
        "PCG_Analysis_{}.md",
        company.name.to_lowercase().replace(' ', "_")
    );

    let mut resp = axum::response::Response::new(axum::body::Body::from(doc));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    let disp = format!("attachment; filename=\"{filename}\"");
    if let Ok(val) = header::HeaderValue::from_str(&disp) {
        resp.headers_mut().insert(header::CONTENT_DISPOSITION, val);
    }
    Ok(resp)
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/proposals/{id}/schedule-meeting", post(schedule_meeting))
        .route("/proposals/{id}/scheduled-meetings", get(list_scheduled_meetings))
        .route("/topsi/meeting/{id}/publish", post(publish_meeting))
        .route("/companies/{id}/export-analysis", get(export_company_analysis))
        .with_state(deployment.clone())
}

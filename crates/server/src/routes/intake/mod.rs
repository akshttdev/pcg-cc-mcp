//! Call Transcript & Email Intake Pipeline
//!
//! Stages:
//!   1. Ingest  — POST /intake/email (webhook) or /intake/upload (manual)
//!   2. Extract — Claude parses participants, pain points, topics, action items
//!   3. Associate — fuzzy-match extracted names/emails → persons table (create if new)
//!   4. Research — trigger intelligence pipeline on matched persons (if stale)
//!   5. Report  — aggregate all context → generate business_audit report

mod handlers;
mod pipeline;
mod report;

use axum::{
    Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::DeploymentImpl;

// Re-export the router constructor
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let state = deployment.clone();
    Router::new()
        // Intake ingestion
        .route("/call-intake/email", post(handlers::email_webhook))
        .route("/call-intake/upload", post(handlers::upload_transcript))
        .route("/call-intake", get(handlers::list_intake))
        .route("/call-intake/{id}", get(handlers::get_intake_item))
        .route(
            "/call-intake/{id}/process",
            post(handlers::process_intake_item_handler),
        )
        .route("/call-intake/{id}/status", get(handlers::get_intake_status))
        // Business reports
        .route("/business-reports", get(handlers::list_reports))
        .route(
            "/business-reports/generate",
            post(handlers::generate_report_handler),
        )
        .route(
            "/business-reports/{id}",
            get(handlers::get_report).patch(handlers::patch_report),
        )
        // Human review endpoints
        .route(
            "/business-reports/{id}/approve",
            post(handlers::approve_business_report),
        )
        .route(
            "/business-reports/{id}/request-revision",
            post(handlers::request_revision),
        )
        .with_state(state)
}

// ── Request / Response types ──────────────────────────────────────────────────

/// Inbound email webhook payload (SendGrid / Zoho inbound parse).
/// Also used for manual uploads via multipart or JSON.
#[derive(Debug, Deserialize)]
pub struct EmailIntakePayload {
    pub subject: Option<String>,
    pub from: Option<String>, // "Name <email>" or just email
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub text: Option<String>, // plain-text body
    pub html: Option<String>,
    pub date: Option<String>,
    pub message_id: Option<String>,
    /// Scope this intake to a specific organization
    pub organization_id: Option<Uuid>,
    /// Assign leads to a specific user (account manager)
    pub assigned_to: Option<Uuid>,
}

/// Manual transcript upload
#[derive(Debug, Deserialize)]
pub struct UploadIntakePayload {
    pub content: String,
    pub subject: Option<String>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub call_date: Option<String>,
    pub duration_seconds: Option<i64>,
    /// Link an existing call_log by id
    pub call_log_id: Option<Uuid>,
    /// Scope this intake to a specific organization
    pub organization_id: Option<Uuid>,
    /// Assign leads to a specific user (account manager)
    pub assigned_to: Option<Uuid>,
}

/// Trigger report generation for a person
#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    pub person_id: Uuid,
    pub report_type: Option<String>,
}

/// Body for POST /api/business-reports/:id/request-revision
#[derive(Debug, Deserialize)]
pub struct RevisionRequest {
    pub notes: String,
}

// Claude extraction response structures
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ExtractedIntake {
    pub participants: Vec<ExtractedParticipant>,
    pub individuals: Vec<ExtractedIndividual>,
    pub businesses: Vec<ExtractedBusiness>,
    pub call_date: Option<String>,
    pub call_summary: String,
    pub topics: Vec<String>,
    pub pain_points: Vec<String>,
    pub action_items: Vec<ExtractedActionItem>,
    pub sentiment: String,
    pub opportunity_signals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ExtractedParticipant {
    pub name: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub role: Option<String>,
    pub is_prospect: bool,
}

/// Individual person identified in the call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ExtractedIndividual {
    pub name: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub role: Option<String>,
    pub linkedin: Option<String>,
    pub is_prospect: bool,
    pub notes: Option<String>,
}

/// Business/organisation identified in the call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ExtractedBusiness {
    pub name: String,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub size_estimate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ExtractedActionItem {
    pub action: String,
    pub owner: Option<String>,
    pub deadline: Option<String>,
}

// Claude report response structure — enhanced with analytics sections
#[derive(Debug, Deserialize)]
pub(crate) struct GeneratedReport {
    pub title: String,
    pub executive_summary: String,
    pub company_overview: String,
    pub pain_points: Vec<serde_json::Value>,
    pub opportunities: Vec<serde_json::Value>,
    pub recommended_services: Vec<serde_json::Value>,
    pub next_steps: Vec<serde_json::Value>,
    pub individual_profiles: Vec<serde_json::Value>,
    pub market_analysis: String,
    pub competitor_analysis: Vec<serde_json::Value>,
    pub target_clients: String,
    pub brand_positioning: String,
    pub digital_presence: String,
    pub sources: Vec<serde_json::Value>,
    pub full_report_md: String,
}

// ── Org routing ───────────────────────────────────────────────────────────────

/// Sirak Studios org ID (fixed seed value from migrations)
const SIRAK_STUDIOS_ORG: &str = "02020202020202020202020202020202";
/// PCG org ID (fixed seed value)
const PCG_ORG: &str = "01010101010101010101010101010101";
/// Sirak's email domain — any sender from this domain routes to Sirak Studios
const SIRAK_DOMAIN: &str = "sirakstudios.com";

use uuid::Uuid;

/// Determine which org and assignee to use for an intake item.
/// - @sirakstudios.com senders → Sirak Studios org, assigned to Sirak's user
/// - Explicit override in payload takes precedence
/// - Everything else → PCG org, no forced assignee
pub(crate) async fn resolve_org_and_assignee(
    pool: &sqlx::SqlitePool,
    from_email: Option<&str>,
    explicit_org: Option<Uuid>,
    explicit_assignee: Option<Uuid>,
) -> (Option<Uuid>, Option<Uuid>) {
    // Explicit payload values always win
    if explicit_org.is_some() || explicit_assignee.is_some() {
        return (explicit_org, explicit_assignee);
    }

    let domain = from_email.and_then(|e| e.split('@').nth(1)).unwrap_or("");

    if domain.eq_ignore_ascii_case(SIRAK_DOMAIN) {
        // Route to Sirak Studios — look up Sirak's user id dynamically
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
        }

        let sirak_user = sqlx::query_as::<_, Row>(
            "SELECT id FROM users WHERE email = 'sirak@powerclubglobal.com' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.id);

        let org_id = SIRAK_STUDIOS_ORG.parse::<Uuid>().ok();
        (org_id, sirak_user)
    } else {
        // Default: PCG org, no forced assignee
        let org_id = PCG_ORG.parse::<Uuid>().ok();
        (org_id, None)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub(crate) fn parse_from_field(
    from: Option<&str>,
    from_name: Option<&str>,
    from_email: Option<&str>,
) -> (Option<String>, Option<String>) {
    if from_name.is_some() || from_email.is_some() {
        return (from_name.map(String::from), from_email.map(String::from));
    }
    if let Some(f) = from {
        // Parse "Name <email>" format
        if let Some(lt) = f.find('<') {
            let name = f[..lt].trim().trim_matches('"').to_string();
            let email = f[lt + 1..].trim_end_matches('>').trim().to_string();
            return (
                if name.is_empty() { None } else { Some(name) },
                if email.is_empty() { None } else { Some(email) },
            );
        }
        // Just an email address
        return (None, Some(f.to_string()));
    }
    (None, None)
}

//! Call Transcript & Email Intake Pipeline
//!
//! Stages:
//!   1. Ingest  — POST /intake/email (webhook) or /intake/upload (manual)
//!   2. Extract — Claude parses participants, pain points, topics, action items
//!   3. Associate — fuzzy-match extracted names/emails → persons table (create if new)
//!   4. Research — trigger intelligence pipeline on matched persons (if stale)
//!   5. Report  — aggregate all context → generate business_audit report

use axum::{
    Extension, Router,
    extract::{Path, State},
    routing::{get, patch, post},
    Json,
};
use db::models::{
    business_report::{BusinessReport, CreateBusinessReport, PatchBusinessReport},
    call_intake_item::{CallIntakeItem, CreateCallIntakeItem},
    company::Company,
    crm_deal::{CreateCrmDeal, CrmDeal},
    person::Person,
    project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
    proposal::{CreateProposal, Proposal},
};
use deployment::Deployment;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info, warn};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

// ── Request / Response types ──────────────────────────────────────────────────

/// Inbound email webhook payload (SendGrid / Zoho inbound parse).
/// Also used for manual uploads via multipart or JSON.
#[derive(Debug, Deserialize)]
pub struct EmailIntakePayload {
    pub subject: Option<String>,
    pub from: Option<String>,         // "Name <email>" or just email
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub text: Option<String>,         // plain-text body
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

// Claude extraction response structure
#[derive(Debug, Serialize, Deserialize)]
struct ExtractedIntake {
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
struct ExtractedParticipant {
    pub name: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub role: Option<String>,
    pub is_prospect: bool,
}

/// Individual person identified in the call
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExtractedIndividual {
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
struct ExtractedBusiness {
    pub name: String,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub size_estimate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractedActionItem {
    pub action: String,
    pub owner: Option<String>,
    pub deadline: Option<String>,
}

// Claude report response structure — enhanced with analytics sections
#[derive(Debug, Deserialize)]
struct GeneratedReport {
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

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let state = deployment.clone();
    Router::new()
        // Intake ingestion
        .route("/call-intake/email", post(email_webhook))
        .route("/call-intake/upload", post(upload_transcript))
        .route("/call-intake", get(list_intake))
        .route("/call-intake/{id}", get(get_intake_item))
        .route("/call-intake/{id}/process", post(process_intake_item_handler))
        .route("/call-intake/{id}/status", get(get_intake_status))
        // Business reports
        .route("/business-reports", get(list_reports))
        .route("/business-reports/generate", post(generate_report_handler))
        .route("/business-reports/{id}", get(get_report).patch(patch_report))
        // Human review endpoints
        .route("/business-reports/{id}/approve", post(approve_business_report))
        .route("/business-reports/{id}/request-revision", post(request_revision))
        .with_state(state)
}

// ── Org routing ───────────────────────────────────────────────────────────────

/// Sirak Studios org ID (fixed seed value from migrations)
const SIRAK_STUDIOS_ORG: &str = "02020202020202020202020202020202";
/// PCG org ID (fixed seed value)
const PCG_ORG: &str = "01010101010101010101010101010101";
/// Sirak's email domain — any sender from this domain routes to Sirak Studios
const SIRAK_DOMAIN: &str = "sirakstudios.com";

/// Determine which org and assignee to use for an intake item.
/// - @sirakstudios.com senders → Sirak Studios org, assigned to Sirak's user
/// - Explicit override in payload takes precedence
/// - Everything else → PCG org, no forced assignee
async fn resolve_org_and_assignee(
    pool: &sqlx::SqlitePool,
    from_email: Option<&str>,
    explicit_org: Option<Uuid>,
    explicit_assignee: Option<Uuid>,
) -> (Option<Uuid>, Option<Uuid>) {
    // Explicit payload values always win
    if explicit_org.is_some() || explicit_assignee.is_some() {
        return (explicit_org, explicit_assignee);
    }

    let domain = from_email
        .and_then(|e| e.split('@').nth(1))
        .unwrap_or("");

    if domain.eq_ignore_ascii_case(SIRAK_DOMAIN) {
        // Route to Sirak Studios — look up Sirak's user id dynamically
        #[derive(sqlx::FromRow)]
        struct Row { id: Uuid }

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

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/intake/email
/// Webhook endpoint for Zoho/SendGrid inbound email parse.
pub async fn email_webhook(
    State(d): State<DeploymentImpl>,
    Json(body): Json<EmailIntakePayload>,
) -> Result<Json<ApiResponse<CallIntakeItem>>, ApiError> {
    let pool = &d.db().pool;

    // Parse "Name <email>" format if from_email not provided separately
    let (from_name, from_email) = parse_from_field(
        body.from.as_deref(),
        body.from_name.as_deref(),
        body.from_email.as_deref(),
    );

    let raw_content = body.text
        .or(body.html)
        .unwrap_or_default();

    if raw_content.trim().is_empty() {
        return Err(ApiError::BadRequest("Email body is empty".into()));
    }

    // Auto-route based on sender domain:
    //   @sirakstudios.com → Sirak Studios org, assigned to Sirak
    //   everything else   → PCG org (default)
    let (org_id, assigned) = resolve_org_and_assignee(
        pool,
        from_email.as_deref(),
        body.organization_id,
        body.assigned_to,
    ).await;

    let item = CallIntakeItem::create(
        pool,
        CreateCallIntakeItem {
            source_type: "email".into(),
            source_ref: body.message_id.clone(),
            raw_content: Some(raw_content.clone()),
            subject: body.subject.clone(),
            from_email: from_email.clone(),
            from_name: from_name.clone(),
            call_date: body.date.clone(),
            duration_seconds: None,
            metadata: Some(serde_json::json!({
                "message_id": body.message_id,
                "organization_id": org_id.map(|id| id.to_string()),
                "assigned_to": assigned.map(|id| id.to_string()),
            }).to_string()),
        },
    ).await?;

    let item_id = item.id;
    let pool_clone = pool.clone();

    // Process asynchronously
    tokio::spawn(async move {
        if let Err(e) = run_intake_pipeline(pool_clone, item_id, org_id, assigned).await {
            error!("Intake pipeline failed for {}: {}", item_id, e);
        }
    });

    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/intake/upload
/// Manual transcript upload (paste or file content).
pub async fn upload_transcript(
    State(d): State<DeploymentImpl>,
    Json(body): Json<UploadIntakePayload>,
) -> Result<Json<ApiResponse<CallIntakeItem>>, ApiError> {
    let pool = &d.db().pool;

    if body.content.trim().is_empty() {
        return Err(ApiError::BadRequest("Content is empty".into()));
    }

    let source_ref = body.call_log_id.map(|id| id.to_string());

    let (org_id, assigned) = resolve_org_and_assignee(
        pool,
        body.from_email.as_deref(),
        body.organization_id,
        body.assigned_to,
    ).await;

    let item = CallIntakeItem::create(
        pool,
        CreateCallIntakeItem {
            source_type: if body.call_log_id.is_some() { "call_log".into() } else { "upload".into() },
            source_ref,
            raw_content: Some(body.content),
            subject: body.subject,
            from_email: body.from_email,
            from_name: body.from_name,
            call_date: body.call_date,
            duration_seconds: body.duration_seconds,
            metadata: Some(serde_json::json!({
                "organization_id": org_id.map(|id| id.to_string()),
                "assigned_to": assigned.map(|id| id.to_string()),
            }).to_string()),
        },
    ).await?;

    let item_id = item.id;
    let pool_clone = pool.clone();
    let org_id = body.organization_id;
    let assigned = body.assigned_to;

    tokio::spawn(async move {
        if let Err(e) = run_intake_pipeline(pool_clone, item_id, org_id, assigned).await {
            error!("Intake pipeline failed for {}: {}", item_id, e);
        }
    });

    Ok(Json(ApiResponse::success(item)))
}

/// GET /api/intake
pub async fn list_intake(
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<CallIntakeItem>>>, ApiError> {
    let items = CallIntakeItem::list(&d.db().pool).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/intake/:id
pub async fn get_intake_item(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CallIntakeItem>>, ApiError> {
    let item = CallIntakeItem::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Intake item not found".into()))?;
    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/intake/:id/process — manually re-trigger pipeline
pub async fn process_intake_item_handler(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &d.db().pool;

    // Reset to pending
    sqlx::query(
        "UPDATE call_intake_items SET status = 'pending', error = NULL, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;

    // Read org/assigned from item metadata if available
    #[derive(sqlx::FromRow)]
    struct MetaRow { metadata: Option<String> }
    let meta = sqlx::query_as::<_, MetaRow>("SELECT metadata FROM call_intake_items WHERE id = ?")
        .bind(id).fetch_optional(pool).await.ok().flatten();
    let (org_id, assigned) = meta.and_then(|m| {
        let v: serde_json::Value = serde_json::from_str(m.metadata.as_deref().unwrap_or("{}")).ok()?;
        let org = v["organization_id"].as_str().and_then(|s| s.parse::<Uuid>().ok());
        let asgn = v["assigned_to"].as_str().and_then(|s| s.parse::<Uuid>().ok());
        Some((org, asgn))
    }).unwrap_or((None, None));

    let pool_clone = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = run_intake_pipeline(pool_clone, id, org_id, assigned).await {
            error!("Re-process pipeline failed for {}: {}", id, e);
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({ "status": "queued", "id": id }))))
}

/// GET /api/intake/:id/status
pub async fn get_intake_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let item = CallIntakeItem::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Intake item not found".into()))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": item.id,
        "status": item.status,
        "person_id": item.person_id,
        "report_id": item.report_id,
        "error": item.error,
        "processed_at": item.processed_at,
    }))))
}

/// GET /api/business-reports
pub async fn list_reports(
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<BusinessReport>>>, ApiError> {
    let reports = BusinessReport::list(&d.db().pool).await?;
    Ok(Json(ApiResponse::success(reports)))
}

/// GET /api/business-reports/:id
pub async fn get_report(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<BusinessReport>>, ApiError> {
    let report = BusinessReport::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;
    Ok(Json(ApiResponse::success(report)))
}

/// PATCH /api/business-reports/:id — inline section editing
pub async fn patch_report(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchBusinessReport>,
) -> Result<Json<ApiResponse<BusinessReport>>, ApiError> {
    let report = BusinessReport::patch(&d.db().pool, id, body)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;
    Ok(Json(ApiResponse::success(report)))
}

/// POST /api/business-reports/:id/approve — authenticated
/// Marks report approved, advances CRM deal to Proposal stage, auto-creates a Proposal record.
pub async fn approve_business_report(
    State(d): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &d.db().pool;
    let user_id = access_context.user_id;

    let report = BusinessReport::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;

    // Mark approved
    sqlx::query(
        "UPDATE business_reports SET review_status = 'approved', reviewed_by = ?,
         reviewed_at = datetime('now','subsec'), updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(user_id)
    .bind(id)
    .execute(pool)
    .await?;

    // Advance CRM deal to "Proposal" stage if linked
    let mut deal_json: Option<serde_json::Value> = None;
    if let Some(deal_id) = report.crm_deal_id {
        if let Ok(deal) = CrmDeal::find_by_id(pool, deal_id).await {
            if let Some(pipeline_id) = deal.crm_pipeline_id {
                // Find the "Proposal" stage in this pipeline
                let proposal_stage: Option<(Vec<u8>,)> = sqlx::query_as(
                    "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND name = 'Proposal' LIMIT 1",
                )
                .bind(pipeline_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((stage_bytes,)) = proposal_stage {
                    if let Ok(stage_id) = Uuid::from_slice(&stage_bytes) {
                        let _ = CrmDeal::move_to_stage(pool, deal_id, stage_id, 0).await;
                    }
                }
            }
            deal_json = Some(serde_json::json!({ "id": deal.id, "name": deal.name }));
        }
    }

    // Auto-create a proposal from the report
    let mut proposal_json: Option<serde_json::Value> = None;
    if let Some(person_id) = report.person_id {
        // Check if a proposal already exists for this person
        let exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM proposals WHERE lead_id = ?",
        )
        .bind(person_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

        if !exists {
            // Build description from report sections
            let summary = report.executive_summary.as_deref().unwrap_or("");
            let services = report.recommended_services.as_str();
            let next = report.next_steps.as_str();
            let description = format!(
                "## Executive Summary\n{summary}\n\n## Recommended Services\n{services}\n\n## Next Steps\n{next}"
            );

            // Look up person name for the title
            #[derive(sqlx::FromRow)]
            struct NameRow { full_name: String, company_name: Option<String> }
            let person_info = sqlx::query_as::<_, NameRow>(
                "SELECT full_name, company_name FROM persons WHERE id = ?",
            )
            .bind(person_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

            let title = if let Some(ref p) = person_info {
                if let Some(ref company) = p.company_name {
                    format!("{} — Proposal", company)
                } else {
                    format!("{} — Proposal", p.full_name)
                }
            } else {
                format!("Proposal from Report {}", id)
            };

            // Find org for this person
            #[derive(sqlx::FromRow)]
            struct OrgRow { organization_id: Uuid }
            let org_id = sqlx::query_as::<_, OrgRow>(
                "SELECT organization_id FROM person_organization_contacts WHERE person_id = ? LIMIT 1",
            )
            .bind(person_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.organization_id);

            let new_proposal = Proposal::create(pool, CreateProposal {
                title,
                lead_id: Some(person_id),
                organization_id: org_id,
                owner_id: Some(user_id),
                project_id: None,
                company_id: report.company_id,
                description: Some(description),
                quote_amount_vibe: None,
                deal_type: None,
                contact_ids: Some(vec![person_id.to_string()]),
            }).await;

            match new_proposal {
                Ok(p) => {
                    proposal_json = Some(serde_json::json!({ "id": p.id, "title": p.title, "status": p.status }));
                    info!("Auto-created proposal {} from approved report {}", p.id, id);
                }
                Err(e) => warn!("Failed to create proposal from report {}: {}", id, e),
            }
        }
    }

    let updated = BusinessReport::find_by_id(pool, id).await?.unwrap();
    Ok(Json(ApiResponse::success(serde_json::json!({
        "report": updated,
        "deal": deal_json,
        "proposal": proposal_json,
    }))))
}

/// POST /api/business-reports/:id/request-revision — authenticated
/// Body: { notes: String }
#[derive(Debug, Deserialize)]
pub struct RevisionRequest {
    pub notes: String,
}

pub async fn request_revision(
    State(d): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<RevisionRequest>,
) -> Result<Json<ApiResponse<BusinessReport>>, ApiError> {
    let pool = &d.db().pool;
    let user_id = access_context.user_id;

    let report = BusinessReport::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;

    // Mark rejected with notes
    sqlx::query(
        "UPDATE business_reports SET review_status = 'rejected', reviewed_by = ?,
         reviewed_at = datetime('now','subsec'), review_notes = ?,
         updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(user_id)
    .bind(&body.notes)
    .bind(id)
    .execute(pool)
    .await?;

    // Move CRM deal back to "Research" stage if linked
    if let Some(deal_id) = report.crm_deal_id {
        if let Ok(deal) = CrmDeal::find_by_id(pool, deal_id).await {
            if let Some(pipeline_id) = deal.crm_pipeline_id {
                let research_stage: Option<(Vec<u8>,)> = sqlx::query_as(
                    "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND name = 'Research' LIMIT 1",
                )
                .bind(pipeline_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((stage_bytes,)) = research_stage {
                    if let Ok(stage_id) = Uuid::from_slice(&stage_bytes) {
                        let _ = CrmDeal::move_to_stage(pool, deal_id, stage_id, 0).await;
                    }
                }
            }
        }
    }

    let updated = BusinessReport::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found after update".into()))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// POST /api/business-reports/generate
pub async fn generate_report_handler(
    State(d): State<DeploymentImpl>,
    Json(body): Json<GenerateReportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &d.db().pool;

    // Verify person exists
    Person::find_by_id(pool, body.person_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Person not found".into()))?;

    let person_id = body.person_id;
    let report_type = body.report_type.unwrap_or_else(|| "business_audit".into());
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        // Load businesses/individuals from most recent intake item for this person
        let (businesses, individuals, intake_id) = {
            #[derive(sqlx::FromRow)]
            struct IntakeRow { id: Uuid, extracted_businesses: String, extracted_individuals: String }
            let row = sqlx::query_as::<_, IntakeRow>(
                "SELECT id, extracted_businesses, extracted_individuals FROM call_intake_items
                 WHERE person_id = ? ORDER BY created_at DESC LIMIT 1",
            )
            .bind(person_id)
            .fetch_optional(&pool_clone)
            .await
            .ok()
            .flatten();

            if let Some(r) = row {
                let biz: Vec<ExtractedBusiness> = serde_json::from_str(&r.extracted_businesses).unwrap_or_default();
                let ind: Vec<ExtractedIndividual> = serde_json::from_str(&r.extracted_individuals).unwrap_or_default();
                (biz, ind, r.id)
            } else {
                (vec![], vec![], Uuid::nil())
            }
        };

        if let Err(e) = run_report_generation(
            pool_clone, person_id, report_type, businesses, individuals, intake_id, None,
        ).await {
            error!("Report generation failed for person {}: {}", person_id, e);
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "status": "queued",
        "person_id": person_id,
        "message": "Report generation started — poll GET /api/persons/:id/reports"
    }))))
}

// ── Core pipeline ─────────────────────────────────────────────────────────────

/// Full intake processing pipeline for a single item.
async fn run_intake_pipeline(
    pool: sqlx::SqlitePool,
    item_id: Uuid,
    organization_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
) -> anyhow::Result<()> {
    info!("Starting intake pipeline for item {}", item_id);

    // Mark processing
    sqlx::query(
        "UPDATE call_intake_items SET status = 'processing', updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(item_id)
    .execute(&pool)
    .await?;

    // Load item
    let item = CallIntakeItem::find_by_id(&pool, item_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Intake item not found"))?;

    let raw = match &item.raw_content {
        Some(c) if !c.trim().is_empty() => c.clone(),
        _ => {
            mark_failed(&pool, item_id, "No content to process").await;
            return Ok(());
        }
    };

    // Stage 1: Extract structure from raw content via Claude
    let extracted = match extract_intake_structure(&raw).await {
        Ok(e) => e,
        Err(e) => {
            mark_failed(&pool, item_id, &format!("Extraction failed: {e}")).await;
            return Ok(());
        }
    };

    // Stage 2: Associate participants to CRM persons
    let primary_person_id = associate_participants(&pool, &extracted.participants, &item, organization_id, assigned_to).await;

    // Stage 2b: Auto-create CRM deal for the primary prospect in the Acquisition pipeline
    let crm_deal_id = if let Some(pid) = primary_person_id {
        ensure_crm_deal_for_person(
            &pool, pid, item_id, organization_id,
            &extracted.call_summary, "Lead",
        ).await
    } else {
        None
    };

    // Stage 3: Persist extracted data
    sqlx::query(
        "UPDATE call_intake_items SET
            status = 'processed',
            processed_at = datetime('now','subsec'),
            person_id = COALESCE(?, person_id),
            extracted_participants = ?,
            extracted_individuals = ?,
            extracted_businesses = ?,
            extracted_topics = ?,
            extracted_action_items = ?,
            extracted_pain_points = ?,
            extracted_sentiment = ?,
            call_summary = ?,
            call_date = COALESCE(?, call_date),
            updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(primary_person_id)
    .bind(serde_json::to_string(&extracted.participants).unwrap_or_else(|_| "[]".into()))
    .bind(serde_json::to_string(&extracted.individuals).unwrap_or_else(|_| "[]".into()))
    .bind(serde_json::to_string(&extracted.businesses).unwrap_or_else(|_| "[]".into()))
    .bind(serde_json::to_string(&extracted.topics).unwrap_or_else(|_| "[]".into()))
    .bind(serde_json::to_string(&extracted.action_items).unwrap_or_else(|_| "[]".into()))
    .bind(serde_json::to_string(&extracted.pain_points).unwrap_or_else(|_| "[]".into()))
    .bind(&extracted.sentiment)
    .bind(&extracted.call_summary)
    .bind(&extracted.call_date)
    .bind(item_id)
    .execute(&pool)
    .await?;

    // Stage 4: Register in knowledge graph (link to any active project for this person)
    if let Some(pid) = primary_person_id {
        register_call_in_knowledge_graph(&pool, pid, item_id, &extracted).await;
    }

    // Stage 5 & 6: Run company research passes, then generate comprehensive report
    // This runs in background so intake item is already marked processed
    let businesses = extracted.businesses.clone();
    let individuals = extracted.individuals.clone();
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        // Run company research passes for each identified business (up to 3 passes each)
        for biz in &businesses {
            if biz.name.len() > 2 {
                for pass in 1u32..=3 {
                    if let Err(e) = run_company_research_pass(&pool_clone, &biz.name, item_id, pass).await {
                        warn!("Company research pass {} failed for '{}': {}", pass, biz.name, e);
                        break;
                    }
                }
            }
        }

        // Queue research for the primary prospect
        if let Some(pid) = primary_person_id {
            trigger_research_if_needed(&pool_clone, pid).await;

            // Advance CRM deal to "Research" stage now that research is queued
            if let Some(deal_id) = crm_deal_id {
                advance_deal_stage(&pool_clone, deal_id, "Research").await;
            }
        }

        // Ingest contextual individuals (mentioned people, not prospects) into org KG
        // These get a person record for context + KG entry, but NO proposal
        ingest_contextual_individuals(&pool_clone, &individuals, item_id, organization_id).await;

        // Now generate the comprehensive report with all research context
        if let Some(pid) = primary_person_id {
            if let Err(e) = run_report_generation(
                pool_clone.clone(), pid, "business_audit".into(),
                businesses, individuals, item_id, crm_deal_id,
            ).await {
                warn!("Auto-report generation failed for person {}: {}", pid, e);
            }
            // Update intake item with report_id is handled inside run_report_generation
        }
    });

    info!("Intake pipeline complete for item {}", item_id);
    Ok(())
}

// ── Stage 1: Extract structure ────────────────────────────────────────────────

async fn extract_intake_structure(raw: &str) -> anyhow::Result<ExtractedIntake> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let system = "You are an expert business analyst specialising in extracting structured data \
        from call transcripts, meeting notes, and email summaries. Always respond with valid JSON only — \
        no markdown fences, no explanation.";

    let prompt = format!(
        r#"Analyse this call/meeting content and return a JSON object with these exact fields:

{{
  "participants": [
    {{
      "name": "string",
      "email": "string or null",
      "company": "string or null",
      "role": "string or null — their job title or role in the call",
      "is_prospect": true/false — true if they appear to be a client/prospect (not PCG/Powerclub team)
    }}
  ],
  "individuals": [
    {{
      "name": "full name of the individual",
      "email": "string or null",
      "company": "their current company or null",
      "role": "their job title or role or null",
      "linkedin": "LinkedIn URL if mentioned, else null",
      "is_prospect": true if client/prospect, false if PCG team member,
      "notes": "any specific notes about this person from the call or null"
    }}
  ],
  "businesses": [
    {{
      "name": "company/brand name",
      "website": "URL if mentioned, else null",
      "industry": "industry sector or null",
      "description": "1-2 sentence description of the business based on context",
      "size_estimate": "e.g. 'startup', 'SMB', 'enterprise', 'solopreneur' or null"
    }}
  ],
  "call_date": "ISO date string (YYYY-MM-DD) if mentioned, else null",
  "call_summary": "2-4 sentence summary of what was discussed",
  "topics": ["array", "of", "main", "topics"],
  "pain_points": ["specific pain points, challenges, or problems the prospect mentioned"],
  "action_items": [
    {{
      "action": "what needs to be done",
      "owner": "who is responsible (name or null)",
      "deadline": "deadline if mentioned, else null"
    }}
  ],
  "sentiment": "positive | neutral | negative",
  "opportunity_signals": ["signals that indicate business opportunity for PCG/Powerclub Global"]
}}

Important: The "individuals" array should include EVERY person mentioned (not just those on the call).
The "businesses" array should include EVERY company/brand discussed.
Fill in as much detail as you can infer from context.

CONTENT:
{}
"#,
        &raw[..raw.len().min(15000)]
    );

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "system": system,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    let body: Value = res.json().await?;
    let text = body["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text in Claude response: {:?}", body))?;

    // Strip markdown fences if present
    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let extracted: ExtractedIntake = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse extraction JSON: {e}\nRaw: {json_str}"))?;

    Ok(extracted)
}

// ── Stage 2: Associate participants to CRM ─────────────────────────────────────

async fn associate_participants(
    pool: &sqlx::SqlitePool,
    participants: &[ExtractedParticipant],
    item: &CallIntakeItem,
    organization_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
) -> Option<Uuid> {
    let mut primary_person_id: Option<Uuid> = None;

    for participant in participants {
        if !participant.is_prospect {
            continue; // Skip PCG team members
        }

        // Try email match first
        let person_id = if let Some(email) = &participant.email {
            find_person_by_email(pool, email).await
        } else {
            None
        };

        // Fall back to name match
        let person_id = if person_id.is_none() {
            find_person_by_name(pool, &participant.name).await
        } else {
            person_id
        };

        let pid = if let Some(id) = person_id {
            update_person_from_intake(pool, id, participant, item).await;
            // Ensure org link exists even for pre-existing persons
            if let Some(org_id) = organization_id {
                link_person_to_org(pool, id, org_id, assigned_to).await;
            }
            id
        } else {
            match create_person_from_intake(pool, participant, item, organization_id, assigned_to).await {
                Some(id) => id,
                None => continue,
            }
        };

        if primary_person_id.is_none() {
            primary_person_id = Some(pid);
        }
    }

    primary_person_id
}

async fn link_person_to_org(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    org_id: Uuid,
    assigned_to: Option<Uuid>,
) {
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO person_organization_contacts (person_id, organization_id, context)
         VALUES (?, ?, 'lead')",
    )
    .bind(person_id)
    .bind(org_id)
    .execute(pool)
    .await;

    if let Some(assignee) = assigned_to {
        let _ = sqlx::query(
            "UPDATE persons SET assigned_to = ?, updated_at = datetime('now','subsec') WHERE id = ? AND assigned_to IS NULL",
        )
        .bind(assignee)
        .bind(person_id)
        .execute(pool)
        .await;
    }
}

async fn find_person_by_email(pool: &sqlx::SqlitePool, email: &str) -> Option<Uuid> {
    #[derive(sqlx::FromRow)]
    struct Row { id: Uuid }

    sqlx::query_as::<_, Row>(
        "SELECT id FROM persons WHERE email = ? COLLATE NOCASE \
         OR emails LIKE ? LIMIT 1",
    )
    .bind(email)
    .bind(format!("%{email}%"))
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.id)
}

async fn find_person_by_name(pool: &sqlx::SqlitePool, name: &str) -> Option<Uuid> {
    #[derive(sqlx::FromRow)]
    struct Row { id: Uuid }

    // Exact match first
    let exact = sqlx::query_as::<_, Row>(
        "SELECT id FROM persons WHERE full_name = ? COLLATE NOCASE LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.id);

    if exact.is_some() {
        return exact;
    }

    // Fuzzy: first + last name parts
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let first = parts[0];
        let last = parts[parts.len() - 1];
        sqlx::query_as::<_, Row>(
            "SELECT id FROM persons WHERE full_name LIKE ? AND full_name LIKE ? LIMIT 1",
        )
        .bind(format!("%{first}%"))
        .bind(format!("%{last}%"))
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.id)
    } else {
        None
    }
}

async fn create_person_from_intake(
    pool: &sqlx::SqlitePool,
    participant: &ExtractedParticipant,
    item: &CallIntakeItem,
    organization_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
) -> Option<Uuid> {
    let id = Uuid::new_v4();
    let channel = match item.source_type.as_str() {
        "email" | "email_message" => "email",
        "call_log" => "phone",
        _ => "phone",
    };

    let result = sqlx::query(
        "INSERT INTO persons
         (id, full_name, email, company_name, job_title, onboarding_channel,
          person_type, assigned_to, intelligence_status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 'lead', ?, 'idle', datetime('now','subsec'), datetime('now','subsec'))",
    )
    .bind(id)
    .bind(&participant.name)
    .bind(&participant.email)
    .bind(&participant.company)
    .bind(&participant.role)
    .bind(channel)
    .bind(assigned_to)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            info!("Created new person: {} ({})", participant.name, id);
            // Link to organization immediately
            if let Some(org_id) = organization_id {
                link_person_to_org(pool, id, org_id, assigned_to).await;
            }
            // Link person to their company record (find or create)
            if let Some(company_name) = &participant.company {
                if !company_name.trim().is_empty() {
                    if let Ok(company) = Company::find_or_create(
                        pool,
                        company_name.trim(),
                        organization_id,
                        None,
                    ).await {
                        let _ = sqlx::query(
                            "INSERT OR IGNORE INTO person_company_roles \
                             (id, person_id, company_id, role, is_primary) \
                             VALUES (randomblob(16), ?, ?, 'contact', 1)",
                        )
                        .bind(id)
                        .bind(company.id)
                        .execute(pool)
                        .await;
                    }
                }
            }
            // Auto-create a proposal (drafted) so this lead appears in the pipeline
            auto_create_proposal(pool, id, participant, item, organization_id, assigned_to).await;
            Some(id)
        }
        Err(e) => {
            warn!("Failed to create person {}: {}", participant.name, e);
            None
        }
    }
}

/// Create a draft proposal for a newly-discovered lead so it appears in the pipeline Kanban.
async fn auto_create_proposal(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    participant: &ExtractedParticipant,
    item: &CallIntakeItem,
    organization_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
) {
    // Skip if a proposal already exists for this lead
    let exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM proposals WHERE lead_id = ?",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(false);

    if exists {
        return;
    }

    let company = participant.company.as_deref().unwrap_or("");
    let title = if company.is_empty() {
        format!("Lead: {}", participant.name)
    } else {
        format!("{} — {}", company, participant.name)
    };

    let description = format!(
        "Intake source: {}\nSubject: {}\nDate: {}",
        item.source_type,
        item.subject.as_deref().unwrap_or("—"),
        item.call_date.as_deref().unwrap_or("—"),
    );

    let _ = Proposal::create(pool, CreateProposal {
        title,
        lead_id: Some(person_id),
        organization_id,
        owner_id: assigned_to,
        project_id: None,
        company_id: None,
        description: Some(description),
        quote_amount_vibe: None,
        deal_type: None,
        contact_ids: Some(vec![person_id.to_string()]),
    }).await;

    info!("Auto-created proposal for new lead {}", person_id);
}

async fn update_person_from_intake(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    participant: &ExtractedParticipant,
    _item: &CallIntakeItem,
) {
    // Fill in blanks only — don't overwrite existing data
    let _ = sqlx::query(
        "UPDATE persons SET
            company_name = COALESCE(company_name, ?),
            job_title = COALESCE(job_title, ?),
            email = COALESCE(email, ?),
            updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&participant.company)
    .bind(&participant.role)
    .bind(&participant.email)
    .bind(person_id)
    .execute(pool)
    .await;
}

// ── Stage 4: Knowledge graph registration ─────────────────────────────────────

async fn register_call_in_knowledge_graph(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    item_id: Uuid,
    extracted: &ExtractedIntake,
) {
    // Find any project this person is linked to via proposals or project_members
    #[derive(sqlx::FromRow)]
    struct Row { project_id: Option<Uuid> }

    let maybe_project: Option<Uuid> = sqlx::query_as::<_, Row>(
        "SELECT p.project_id FROM proposals p
         JOIN json_each(p.contact_ids) j ON j.value = lower(hex(?))
         LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| r.project_id);

    if let Some(project_id) = maybe_project {
        let title = format!("Call: {}", extracted.call_summary.chars().take(80).collect::<String>());
        let summary = format!(
            "Topics: {}. Pain points: {}.",
            extracted.topics.join(", "),
            extracted.pain_points.join("; ")
        );
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            project_id,
            &KnowledgeSourceType::Artifact,
            &item_id.to_string(),
            &title,
            Some(&summary),
            0.7,
        )
        .await;
    }
}

// ── Stage 5a: Company research passes ────────────────────────────────────────

/// Run one research pass on a company using Claude with web search.
async fn run_company_research_pass(
    pool: &sqlx::SqlitePool,
    company_name: &str,
    intake_item_id: Uuid,
    pass_number: u32,
) -> anyhow::Result<()> {
    let focus = match pass_number {
        1 => "identity_and_overview",
        2 => "market_position_and_competitors",
        _ => "digital_presence_and_opportunities",
    };

    info!("Company research pass {} ({}) for '{}'", pass_number, focus, company_name);

    let pass_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO company_research_passes
         (id, company_name, intake_item_id, pass_number, research_focus, status)
         VALUES (?, ?, ?, ?, ?, 'running')",
    )
    .bind(pass_id)
    .bind(company_name)
    .bind(intake_item_id)
    .bind(pass_number as i64)
    .bind(focus)
    .execute(pool)
    .await?;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let focus_prompt = match focus {
        "identity_and_overview" => format!(
            "Research '{company_name}': What do they do? What industry are they in? Who founded it? \
             What is their size, location, and key offerings? Find their website and social media."
        ),
        "market_position_and_competitors" => format!(
            "Research '{company_name}' market position: Who are their main competitors? \
             What is their unique value proposition? How do they differentiate? \
             What market segment do they serve? What are their strengths and weaknesses?"
        ),
        _ => format!(
            "Research '{company_name}' digital presence and business opportunities: \
             What is their web presence like? Social media activity? Content strategy? \
             What business challenges might they face where a creative agency could help? \
             What are potential growth opportunities?"
        ),
    };

    let messages = vec![serde_json::json!({
        "role": "user",
        "content": format!(
            "{focus_prompt}\n\nReturn a JSON object: \
            {{\"summary\": \"3-5 sentence overview\", \
              \"key_findings\": {{\"relevant_details\": \"...\"}}, \
              \"sources\": [{{\"title\": \"...\", \"url\": \"...\"}}], \
              \"confidence_score\": 0.0-1.0}}"
        )
    })];

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "web-search-2025-03-05")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "tools": [{
                "type": "web_search_20250305",
                "name": "web_search",
                "max_uses": 4
            }],
            "messages": messages
        }))
        .send()
        .await?;

    let body: serde_json::Value = res.json().await?;

    // Extract text content from response (may include tool use blocks)
    let text = body["content"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|b| b["type"].as_str() == Some("text"))
                .and_then(|b| b["text"].as_str())
        })
        .unwrap_or("{}");

    let json_str = text.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap_or_else(|_| {
        serde_json::json!({"summary": text, "key_findings": {}, "sources": [], "confidence_score": 0.3})
    });

    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let key_findings = serde_json::to_string(&parsed["key_findings"]).unwrap_or_else(|_| "{}".into());
    let sources = serde_json::to_string(&parsed["sources"]).unwrap_or_else(|_| "[]".into());
    let confidence = parsed["confidence_score"].as_f64().unwrap_or(0.5);

    sqlx::query(
        "UPDATE company_research_passes SET
         status = 'done', summary = ?, key_findings = ?, sources = ?,
         confidence_score = ?, agent_used = 'claude-sonnet',
         updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&summary)
    .bind(&key_findings)
    .bind(&sources)
    .bind(confidence)
    .bind(pass_id)
    .execute(pool)
    .await?;

    info!("Company research pass {} complete for '{}'", pass_number, company_name);
    Ok(())
}

// ── Stage 5b: Trigger person research ────────────────────────────────────────

async fn trigger_research_if_needed(pool: &sqlx::SqlitePool, person_id: Uuid) {
    #[derive(sqlx::FromRow)]
    struct Row { intelligence_status: Option<String> }

    let status = sqlx::query_as::<_, Row>(
        "SELECT intelligence_status FROM persons WHERE id = ?",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| r.intelligence_status)
    .unwrap_or_else(|| "idle".into());

    // Only trigger if idle or failed (not already running/queued/done)
    if matches!(status.as_str(), "idle" | "failed") {
        let _ = sqlx::query(
            "UPDATE persons SET intelligence_status = 'queued', \
             intelligence_agent = 'scout', updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(person_id)
        .execute(pool)
        .await;

        info!("Queued research for person {}", person_id);
        // Note: Full Nora orchestration would be triggered here with get_nora_instance().
        // For now the status is set to 'queued' and the admin can manually trigger via
        // POST /api/persons/:id/research.
    }
}

// ── Stage 6: Report generation ────────────────────────────────────────────────

async fn run_report_generation(
    pool: sqlx::SqlitePool,
    person_id: Uuid,
    report_type: String,
    businesses: Vec<ExtractedBusiness>,
    individuals: Vec<ExtractedIndividual>,
    primary_intake_id: Uuid,
    crm_deal_id: Option<Uuid>,
) -> anyhow::Result<()> {
    info!("Generating {} report for person {}", report_type, person_id);

    // Load person
    let person = Person::find_by_id(&pool, person_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Person not found"))?;

    // Load all intake items for this person
    let intake_items = CallIntakeItem::list_by_person(&pool, person_id).await?;

    if intake_items.is_empty() {
        warn!("No intake items for person {} — skipping report", person_id);
        return Ok(());
    }

    // Gather call transcripts / summaries
    let mut call_context = String::new();
    let mut intake_ids: Vec<String> = Vec::new();

    for item in &intake_items {
        intake_ids.push(item.id.to_string());
        let summary = item.call_summary.as_deref().unwrap_or("");
        let topics: Vec<String> = serde_json::from_str(&item.extracted_topics).unwrap_or_default();
        let pain_points: Vec<String> = serde_json::from_str(&item.extracted_pain_points).unwrap_or_default();

        call_context.push_str(&format!(
            "\n--- Call/Email ({}) ---\nSummary: {}\nTopics: {}\nPain Points: {}\n",
            item.call_date.as_deref().unwrap_or("unknown date"),
            summary,
            topics.join(", "),
            pain_points.join("; "),
        ));

        // Also include raw content excerpt
        if let Some(raw) = &item.raw_content {
            call_context.push_str(&format!(
                "Transcript excerpt:\n{}\n",
                &raw[..raw.len().min(2000)]
            ));
        }
    }

    // Load company research results for context
    let mut company_research_context = String::new();
    for biz in &businesses {
        let passes: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
            "SELECT research_focus, summary FROM company_research_passes
             WHERE company_name = ? AND status = 'done'
             ORDER BY pass_number ASC LIMIT 5",
        )
        .bind(&biz.name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        if !passes.is_empty() {
            company_research_context.push_str(&format!("\n\n=== Company Research: {} ===\n", biz.name));
            for (focus, summary) in &passes {
                company_research_context.push_str(&format!("[{}] {}\n", focus, summary));
            }
        }

        // Gather sources from research passes
        let sources_json: Vec<serde_json::Value> = sqlx::query_as::<_, (String,)>(
            "SELECT sources FROM company_research_passes
             WHERE company_name = ? AND status = 'done'",
        )
        .bind(&biz.name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .flat_map(|(s,)| serde_json::from_str::<Vec<serde_json::Value>>(&s).unwrap_or_default())
        .collect();

        if !sources_json.is_empty() {
            company_research_context.push_str(&format!(
                "Sources: {}\n",
                sources_json.iter()
                    .filter_map(|s| s["url"].as_str().map(|u| u.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // Intelligence summary
    let intel_summary = person.intelligence_summary.as_deref().unwrap_or("No prior research available.");
    let person_name = &person.full_name;
    let company = person.company_name.as_deref().unwrap_or(
        businesses.first().map(|b| b.name.as_str()).unwrap_or("Unknown company")
    );

    let biz_list = businesses.iter().map(|b| {
        format!("- {} ({}): {}", b.name,
            b.website.as_deref().unwrap_or("no website"),
            b.description.as_deref().unwrap_or(""))
    }).collect::<Vec<_>>().join("\n");

    let ind_list = individuals.iter().map(|i| {
        format!("- {} | {} at {} | {}",
            i.name,
            i.role.as_deref().unwrap_or("unknown role"),
            i.company.as_deref().unwrap_or("unknown company"),
            i.notes.as_deref().unwrap_or(""))
    }).collect::<Vec<_>>().join("\n");

    let generated = generate_report_with_claude(
        person_name,
        company,
        intel_summary,
        &call_context,
        &company_research_context,
        &biz_list,
        &ind_list,
        &report_type,
    ).await?;

    // Collect all sources from company research passes
    let all_sources: Vec<serde_json::Value> = sqlx::query_as::<_, (String,)>(
        "SELECT sources FROM company_research_passes WHERE intake_item_id = ? AND status = 'done'",
    )
    .bind(primary_intake_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .flat_map(|(s,)| serde_json::from_str::<Vec<serde_json::Value>>(&s).unwrap_or_default())
    .collect();

    let mut merged_sources = generated.sources.clone();
    merged_sources.extend(all_sources);

    // Resolve the person's primary company for the report
    #[derive(sqlx::FromRow)]
    struct CompanyIdRow { company_id: Option<Uuid> }
    let company_id: Option<Uuid> = sqlx::query_as::<_, CompanyIdRow>(
        "SELECT company_id FROM person_company_roles WHERE person_id = ? AND is_primary = 1 LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| r.company_id);

    // Store report
    let report = BusinessReport::create(
        &pool,
        CreateBusinessReport {
            person_id: Some(person_id),
            company_id,
            report_type: Some(report_type.clone()),
            title: generated.title.clone(),
            executive_summary: Some(generated.executive_summary),
            company_overview: Some(generated.company_overview),
            pain_points: Some(serde_json::to_string(&generated.pain_points).unwrap_or_else(|_| "[]".into())),
            opportunities: Some(serde_json::to_string(&generated.opportunities).unwrap_or_else(|_| "[]".into())),
            recommended_services: Some(serde_json::to_string(&generated.recommended_services).unwrap_or_else(|_| "[]".into())),
            next_steps: Some(serde_json::to_string(&generated.next_steps).unwrap_or_else(|_| "[]".into())),
            full_report_md: Some(generated.full_report_md),
            individual_profiles: Some(serde_json::to_string(&generated.individual_profiles).unwrap_or_else(|_| "[]".into())),
            market_analysis: Some(generated.market_analysis),
            competitor_analysis: Some(serde_json::to_string(&generated.competitor_analysis).unwrap_or_else(|_| "[]".into())),
            target_clients: Some(generated.target_clients),
            brand_positioning: Some(generated.brand_positioning),
            digital_presence: Some(generated.digital_presence),
            sources: Some(serde_json::to_string(&merged_sources).unwrap_or_else(|_| "[]".into())),
            intake_item_ids: Some(serde_json::to_string(&intake_ids).unwrap_or_else(|_| "[]".into())),
            call_log_ids: Some("[]".into()),
            created_by: None,
        },
    ).await?;

    // Link crm_deal_id to the report
    if let Some(deal_id) = crm_deal_id {
        let _ = sqlx::query(
            "UPDATE business_reports SET crm_deal_id = ? WHERE id = ?",
        )
        .bind(deal_id)
        .bind(report.id)
        .execute(&pool)
        .await;
    }

    BusinessReport::mark_ready(&pool, report.id).await?;

    // Link report back to intake items
    let report_id = report.id;
    for intake_id in &intake_ids {
        if let Ok(iid) = Uuid::parse_str(intake_id) {
            let _ = sqlx::query(
                "UPDATE call_intake_items SET report_id = ?, updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(report_id)
            .bind(iid)
            .execute(&pool)
            .await;
        }
    }

    // Advance CRM deal to "Analysis Done" now that report is complete
    if let Some(deal_id) = crm_deal_id {
        advance_deal_stage(&pool, deal_id, "Analysis Done").await;
    }

    // Ingest report sources into org-scoped knowledge graph
    ingest_sources_into_kg(&pool, person_id, &merged_sources).await;

    info!("Report {} created for person {}", report.id, person_id);
    Ok(())
}

/// Register research source URLs in the org-scoped knowledge graph.
async fn ingest_sources_into_kg(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    sources: &[serde_json::Value],
) {
    // Find which org this person belongs to
    #[derive(sqlx::FromRow)]
    struct Row { organization_id: Uuid }

    let org_id = sqlx::query_as::<_, Row>(
        "SELECT organization_id FROM person_organization_contacts WHERE person_id = ? LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.organization_id);

    if let Some(org_id) = org_id {
        for src in sources {
            let url = match src["url"].as_str() { Some(u) if u.starts_with("http") => u, _ => continue };
            let title = src["title"].as_str().unwrap_or(url);
            let excerpt = src["excerpt"].as_str().unwrap_or("");
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO project_knowledge_sources
                 (id, owner_type, owner_id, source_type, source_id, source_title,
                  source_summary, coverage_score, is_active, is_stale,
                  created_at, updated_at)
                 VALUES (?, 'organization', ?, 'web_page', ?, ?, ?, 0.6, 1, 0,
                  datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(Uuid::new_v4())
            .bind(org_id)
            .bind(url)
            .bind(title)
            .bind(excerpt)
            .execute(pool)
            .await;
        }
    }
}

async fn generate_report_with_claude(
    person_name: &str,
    company: &str,
    intel_summary: &str,
    call_context: &str,
    company_research_context: &str,
    biz_list: &str,
    ind_list: &str,
    report_type: &str,
) -> anyhow::Result<GeneratedReport> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let system = "You are a senior business development analyst at Powerclub Global, \
        a creative agency specialising in media production, brand strategy, talent management, \
        and digital content. Your job is to analyse prospect/client intelligence and generate \
        comprehensive, actionable business audit reports with deep analytics. \
        Always respond with valid JSON only — no markdown fences, no explanation outside the JSON.";

    let prompt = format!(
        r#"Generate a comprehensive {report_type} analytics report for:
Primary Contact: {person_name}
Primary Company: {company}

--- IDENTIFIED INDIVIDUALS ---
{ind_list}

--- IDENTIFIED BUSINESSES ---
{biz_list}

--- INTELLIGENCE RESEARCH ---
{intel_summary}

--- COMPANY RESEARCH (multi-pass) ---
{company_research_context}

--- CALL/MEETING CONTEXT ---
{call_context}

Return a JSON object with these EXACT fields (all required):
{{
  "title": "string — concise report title e.g. 'Business Analytics: Acme Corp'",
  "executive_summary": "string — 3-5 sentences summarising key findings and opportunity for PCG",
  "company_overview": "string — 3-5 sentences about the prospect company based on all research",
  "individual_profiles": [
    {{
      "name": "person's full name",
      "role": "their title/role",
      "company": "their company",
      "linkedin": "LinkedIn URL or null",
      "summary": "2-3 sentence profile of this individual",
      "key_insights": "what is most relevant about this person for our engagement"
    }}
  ],
  "market_analysis": "string — 3-6 sentence markdown prose on their market position, size, trends, growth trajectory",
  "competitor_analysis": [
    {{
      "name": "competitor name",
      "website": "URL or null",
      "strengths": "what they do well",
      "weaknesses": "where they fall short",
      "threat_level": "high|medium|low"
    }}
  ],
  "target_clients": "string — 2-4 sentence markdown prose on who THEIR ideal customers are",
  "brand_positioning": "string — 2-4 sentence markdown prose on how they position themselves in the market",
  "digital_presence": "string — 2-4 sentence markdown prose on their web, social, SEO, content presence",
  "pain_points": [
    {{"point": "specific pain point", "severity": "high|medium|low"}}
  ],
  "opportunities": [
    {{
      "title": "opportunity title",
      "description": "2-3 sentences explaining the opportunity for PCG",
      "priority": "high|medium|low",
      "estimated_value": "e.g. '$5k-15k project' or 'ongoing retainer'"
    }}
  ],
  "recommended_services": [
    {{
      "name": "PCG service name",
      "rationale": "why this service fits this prospect",
      "timeline": "e.g. 'Q2 2026' or '30-60 days'"
    }}
  ],
  "next_steps": [
    {{
      "action": "specific action",
      "owner": "PCG team member or role",
      "deadline": "timeline string"
    }}
  ],
  "sources": [
    {{
      "title": "page/article title",
      "url": "full URL",
      "excerpt": "1-2 sentence relevant excerpt or null"
    }}
  ],
  "full_report_md": "string — complete narrative report in markdown, 800+ words, with sections: \
    Executive Summary, Individual Profiles, Company Overview, Market Analysis, \
    Competitive Landscape, Brand Positioning, Digital Presence, Pain Points, \
    Opportunity Analysis, Recommended Services, Next Steps, Conclusion"
}}
"#
    );

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-opus-4-6",
            "max_tokens": 8192,
            "system": system,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    let body: Value = res.json().await?;
    let text = body["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text in Claude response: {:?}", body))?;

    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let report: GeneratedReport = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse report JSON: {e}\nRaw: {}", &json_str[..json_str.len().min(500)]))?;

    Ok(report)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_from_field(
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
            let email = f[lt+1..].trim_end_matches('>').trim().to_string();
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

/// Ingest contextually-mentioned individuals (non-prospects) into the org knowledge graph.
/// These are people referenced in the call — we research who they are for context,
/// but we do NOT create proposals or treat them as pipeline leads.
async fn ingest_contextual_individuals(
    pool: &sqlx::SqlitePool,
    individuals: &[ExtractedIndividual],
    intake_item_id: Uuid,
    organization_id: Option<Uuid>,
) {
    let Some(org_id) = organization_id else { return };

    for ind in individuals {
        // Skip if they're a prospect — handled by associate_participants already
        if ind.is_prospect {
            continue;
        }
        // Skip generic/unknown names
        if ind.name.len() < 3 || ind.name.to_lowercase().contains("unknown") {
            continue;
        }

        // Check if person already exists
        let existing = find_person_by_name(pool, &ind.name).await;
        let person_id = if let Some(pid) = existing {
            pid
        } else {
            // Create a minimal person record for context (type = 'contact', not 'lead')
            let id = Uuid::new_v4();
            let ok = sqlx::query(
                "INSERT OR IGNORE INTO persons
                 (id, full_name, email, company_name, job_title,
                  person_type, intelligence_status, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, 'contact', 'idle', datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(id)
            .bind(&ind.name)
            .bind(&ind.email)
            .bind(&ind.company)
            .bind(&ind.role)
            .execute(pool)
            .await
            .is_ok();
            if !ok { continue; }
            id
        };

        // Register in org knowledge graph as 'entity' type
        let summary = format!(
            "{}{}{} — mentioned in call context. Notes: {}",
            ind.role.as_deref().unwrap_or(""),
            if ind.role.is_some() && ind.company.is_some() { " at " } else { "" },
            ind.company.as_deref().unwrap_or(""),
            ind.notes.as_deref().unwrap_or("No additional context"),
        );

        let _ = sqlx::query(
            "INSERT OR IGNORE INTO project_knowledge_sources
             (id, owner_type, owner_id, source_type, source_id, source_title,
              source_summary, coverage_score, is_active, is_stale,
              created_at, updated_at)
             VALUES (?, 'organization', ?, 'entity', ?, ?, ?, 0.4, 1, 0,
              datetime('now','subsec'), datetime('now','subsec'))",
        )
        .bind(Uuid::new_v4())
        .bind(org_id)
        .bind(person_id)
        .bind(&ind.name)
        .bind(&summary)
        .execute(pool)
        .await;
    }
}

async fn mark_failed(pool: &sqlx::SqlitePool, item_id: Uuid, error: &str) {
    let _ = sqlx::query(
        "UPDATE call_intake_items SET status = 'failed', error = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(error)
    .bind(item_id)
    .execute(pool)
    .await;
}

// ── CRM Deal automation helpers ───────────────────────────────────────────────

/// Look up the Acquisition pipeline for the given org, find or create a deal for this
/// person in the given stage. Returns the deal ID if successful.
async fn ensure_crm_deal_for_person(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    intake_item_id: Uuid,
    organization_id: Option<Uuid>,
    summary: &str,
    stage_name: &str,
) -> Option<Uuid> {
    let org_id = organization_id?;

    // Find the Acquisition (sales) pipeline for this org
    #[derive(sqlx::FromRow)]
    struct PipelineRow { id: Uuid }

    let pipeline = sqlx::query_as::<_, PipelineRow>(
        "SELECT id FROM crm_pipelines WHERE organization_id = ? AND pipeline_type = 'sales'
         AND project_id IS NULL LIMIT 1",
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let pipeline_id = pipeline.map(|p| p.id)?;

    // Find the named stage in this pipeline
    #[derive(sqlx::FromRow)]
    struct StageRow { id: Vec<u8> }

    let stage = sqlx::query_as::<_, StageRow>(
        "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND name = ? LIMIT 1",
    )
    .bind(pipeline_id)
    .bind(stage_name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let stage_id = stage.and_then(|s| Uuid::from_slice(&s.id).ok())?;

    // Look up person name + contact for dedup check
    #[derive(sqlx::FromRow)]
    struct PersonRow { full_name: String }

    let person_name = sqlx::query_as::<_, PersonRow>(
        "SELECT full_name FROM persons WHERE id = ?",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.full_name)
    .unwrap_or_else(|| "Unknown".to_string());

    let deal_name = format!("{} — Discovery Lead", person_name);

    // Check for existing deal with same name in this org (dedup)
    if let Ok(Some(existing)) = CrmDeal::find_by_name_and_org(pool, &deal_name, org_id).await {
        // Link intake item to existing deal
        let _ = sqlx::query(
            "UPDATE call_intake_items SET crm_deal_id = ? WHERE id = ?",
        )
        .bind(existing.id)
        .bind(intake_item_id)
        .execute(pool)
        .await;
        return Some(existing.id);
    }

    // Find crm_contact_id for this person if available
    #[derive(sqlx::FromRow)]
    struct ContactRow { id: Uuid }
    let contact_id = sqlx::query_as::<_, ContactRow>(
        "SELECT id FROM crm_contacts WHERE person_id = ? LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.id);

    // Create the deal
    let deal_result = CrmDeal::create(pool, CreateCrmDeal {
        organization_id: org_id,
        client_id: None,
        crm_contact_id: contact_id,
        crm_pipeline_id: Some(pipeline_id),
        crm_stage_id: Some(stage_id),
        name: deal_name,
        description: Some(summary[..summary.len().min(500)].to_string()),
        amount: None,
        currency: None,
        expected_close_date: None,
        tags: None,
        custom_fields: None,
    }).await;

    match deal_result {
        Ok(deal) => {
            info!("Auto-created CRM deal {} for person {} in pipeline {}", deal.id, person_id, pipeline_id);
            // Link intake item to deal
            let _ = sqlx::query(
                "UPDATE call_intake_items SET crm_deal_id = ? WHERE id = ?",
            )
            .bind(deal.id)
            .bind(intake_item_id)
            .execute(pool)
            .await;
            Some(deal.id)
        }
        Err(e) => {
            warn!("Failed to create CRM deal for person {}: {:?}", person_id, e);
            None
        }
    }
}

/// Advance a CRM deal to the named stage in its pipeline.
async fn advance_deal_stage(pool: &sqlx::SqlitePool, deal_id: Uuid, stage_name: &str) {
    let deal = match CrmDeal::find_by_id(pool, deal_id).await {
        Ok(d) => d,
        Err(_) => return,
    };

    if let Some(pipeline_id) = deal.crm_pipeline_id {
        #[derive(sqlx::FromRow)]
        struct StageRow { id: Vec<u8> }

        let stage = sqlx::query_as::<_, StageRow>(
            "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND name = ? LIMIT 1",
        )
        .bind(pipeline_id)
        .bind(stage_name)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some(s) = stage {
            if let Ok(stage_id) = Uuid::from_slice(&s.id) {
                let _ = CrmDeal::move_to_stage(pool, deal_id, stage_id, 0).await;
                info!("Advanced deal {} to stage '{}'", deal_id, stage_name);
            }
        }
    }
}

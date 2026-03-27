//! Contact Intelligence — contact research pipeline.
//!
//! Architecture:
//!   1. POST /api/contacts/:id/research  → Nora receives the orchestration request
//!   2. Nora delegates to Scout (Social Intelligence) or Astra (Strategy Research)
//!   3. Agent performs web research via its tool suite
//!   4. Results are stored in crm_contacts.intelligence_* fields
//!   5. Social profiles, company contact methods, proposals, tasks, and
//!      business analysis files are all created/updated from the research output.

//!
//! The endpoint is non-blocking — it sets intelligence_status='queued', fires
//! the Nora request asynchronously, and returns immediately with a job token.
//! The frontend polls GET /api/contacts/:id/intelligence-status.

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use db::{
    db_uuid::DbUuid,
    models::{
        crm_contact::CrmContact,
        project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
    },
};
use deployment::Deployment;
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl, error::ApiError, helpers::uuid_params::parse_db_uuid_param,
    middleware::access_control::AccessContext, routes::nora::get_nora_instance,
};

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ResearchRequest {
    /// Optional project scope — if provided, results also go into that project's knowledge graph
    pub project_id: Option<Uuid>,

    pub agent_preference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResearchJobResponse {
    pub contact_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct IntelligenceStatusResponse {
    pub contact_id: String,
    pub status: String,
    pub summary: Option<String>,
    pub confidence: f64,
    pub agent: Option<String>,
    pub last_run_at: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/contacts/:id/research
pub async fn trigger_research(
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
    Json(body): Json<ResearchRequest>,
) -> Result<Json<ApiResponse<ResearchJobResponse>>, ApiError> {
    let contact_id = parse_db_uuid_param(&contact_id, "contact ID")?;
    let pool = &d.db().pool;

    let contact = CrmContact::find_by_id(pool, &contact_id).await?;

    // Set queued status on the contact directly
    sqlx::query(
        "UPDATE crm_contacts SET intelligence_status = 'queued', \
         intelligence_agent = ?, updated_at = datetime('now','subsec') \
         WHERE CAST(id AS TEXT) = ?",
    )
    .bind(body.agent_preference.as_deref().unwrap_or("scout"))
    .bind(contact_id.as_ref())
    .execute(pool)
    .await?;

    let agent_pref = body.agent_preference.as_deref().unwrap_or("Scout");
    let contact_name = contact.full_name.clone().unwrap_or_default();

    // Fire async task — Nora orchestrates, Scout/Astra executes
    let pool_clone = pool.clone();
    let project_id = body.project_id;
    let contact_clone = contact.clone();
    let contact_id_str = contact_id.to_string();

    tokio::spawn(async move {
        let result = run_research_direct(&pool_clone, &contact_clone, project_id).await;

        if let Err(e) = result {
            tracing::error!(
                "[Scout] Research failed for contact {} (legacy path): {}",
                contact_id_str,
                e
            );
            if let Err(e2) = sqlx::query(
                "UPDATE crm_contacts SET intelligence_status = 'failed', updated_at = datetime('now','subsec') \
                 WHERE CAST(id AS TEXT) = ?",
            )
            .bind(&contact_id_str)
            .execute(&pool_clone)
            .await
            {
                tracing::warn!("[Intelligence] Failed to reset status to 'failed': {}", e2);
            }
        }
    });

    Ok(Json(ApiResponse::success(ResearchJobResponse {
        contact_id: contact_id.to_string(),
        status: "queued".into(),
        message: format!(
            "Research queued for {} — {} will gather intelligence and update this profile.",
            contact_name, agent_pref
        ),
    })))
}

/// GET /api/contacts/:id/intelligence-status
pub async fn get_intelligence_status(
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<IntelligenceStatusResponse>>, ApiError> {
    let contact_id = parse_db_uuid_param(&contact_id, "contact ID")?;
    let pool = &d.db().pool;

    #[derive(sqlx::FromRow)]
    struct Row {
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: f64,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT intelligence_status, intelligence_summary, intelligence_confidence, \
         intelligence_agent, intelligence_last_run_at FROM crm_contacts WHERE CAST(id AS TEXT) = ?",
    )
    .bind(contact_id.as_ref())
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    Ok(Json(ApiResponse::success(IntelligenceStatusResponse {
        contact_id: contact_id.to_string(),
        status: row.intelligence_status,
        summary: row.intelligence_summary,
        confidence: row.intelligence_confidence,
        agent: row.intelligence_agent,
        last_run_at: row.intelligence_last_run_at,
    })))
}

// ── Core research execution ───────────────────────────────────────────────────

async fn run_research_via_nora(
    pool: &sqlx::SqlitePool,
    contact: CrmContact,
    prompt: String,
    project_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let contact_id = contact.id.clone();
    let full_name = contact.full_name.clone().unwrap_or_default();

    sqlx::query(
        "UPDATE crm_contacts SET intelligence_status = 'running', updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
    )
    .bind(contact_id.as_ref())
    .execute(pool)
    .await?;

    // Get Nora instance — Nora orchestrates the delegation
    let nora_instance = match get_nora_instance().await {
        Ok(n) => n,
        Err(_) => {
            // Nora not initialized — run a direct research fallback
            return run_research_direct(pool, &contact, project_id).await;
        }
    };

    // Use Nora agent directly from the locked instance
    let response = {
        let nora_guard = nora_instance.read().await;
        let Some(nora) = nora_guard.as_ref() else {
            drop(nora_guard);
            return run_research_direct(pool, &contact, project_id).await;
        };

        let nora_request = NoraRequest {
            request_id: DbUuid::new().to_string(),
            session_id: format!("research-{}", contact_id),
            request_type: NoraRequestType::TextInteraction,
            content: prompt,
            context: None,
            voice_enabled: false,
            priority: RequestPriority::High,
            timestamp: chrono::Utc::now(),
        };

        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            nora.process_request(nora_request),
        )
        .await
        .map_err(|_| "Research timed out after 60s")?
        .map_err(|e| format!("Nora error: {}", e))?
    };

    // Detect Nora failure responses (quota exceeded, API errors, tool-use logs) and fall back to direct
    let content_lower = response.content.to_lowercase();
    let is_failure_response = content_lower.contains("api quota")
        || content_lower.contains("quota limit")
        || content_lower.contains("quota exceeded")
        || content_lower.contains("quota limitation")
        || content_lower.contains("openai api quota")
        || (content_lower.contains("api quota") && content_lower.contains("exceeded"));

    // Detect Nora returning action logs instead of clean JSON — fall back to direct
    let is_action_log = response.content.contains("I've completed")
        || response.content.contains("completed multiple actions")
        || response.content.contains("completed the requested")
        || response.content.starts_with("✅")
        || (response.content.contains("\"success\":true,\"url\"")
            && !response.content.contains("\"summary\""))
        || (response.content.contains("\"results\":[{\"title\"")
            && !response.content.contains("\"summary\""));

    if is_failure_response || is_action_log {
        tracing::warn!(
            "Nora research produced unusable output for contact {}, falling back to direct Anthropic research",
            contact_id
        );
        return run_research_direct(pool, &contact, project_id).await;
    }

    // Parse Nora's response and write to contact record
    let summary = extract_summary_from_response(&response.content);
    let confidence = extract_confidence_from_response(&response.content);

    write_intelligence_results(
        pool,
        contact_id.as_ref(),
        &summary,
        confidence,
        &response.content,
        project_id,
        &full_name,
    )
    .await?;
    Ok(())
}

/// Fallback: direct Anthropic web search when Nora is not initialized.
/// Uses Scout's persona — social intelligence specialization.
/// When SIMULATE_LLM=1, writes simulated data and returns immediately.
async fn run_research_direct(
    pool: &sqlx::SqlitePool,
    contact: &CrmContact,
    project_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let contact_name = contact.full_name.as_deref().unwrap_or("Unknown");

    // Simulation mode: write realistic placeholder data without calling LLM
    if std::env::var("SIMULATE_LLM").unwrap_or_default() == "1" {
        let summary = format!(
            "Contact appears to be a decision-maker in their organization. \
             Key areas: digital transformation, operational efficiency, strategic partnerships. \
             Company is positioned for growth with opportunities in automation and AI integration."
        );
        tracing::info!(
            "[Scout] Simulated research for '{}' (SIMULATE_LLM=1)",
            contact_name
        );
        write_intelligence_results(
            pool,
            contact.id.as_ref(),
            &summary,
            0.75,
            &format!("[Simulated Scout Research for {}]", contact_name),
            project_id,
            contact_name,
        )
        .await?;
        return Ok(());
    }

    use services::services::workflow_llm::{LLMResponse, WorkflowLLMService};

    let system = "You are Scout, Social Intelligence Analyst for Power Club Global. \
        Your specialty is finding and structuring online presence data about individuals. \
        Research the person thoroughly based on your knowledge. Return their: \
        current role, company details, social profiles, recent activity, and public bio. \
        Return a detailed intelligence summary as plain text (not JSON).";

    let prompt = format!(
        "Research this contact for PCG: Name={}, Company={}, Email={}, Title={}. \
        Provide a comprehensive intelligence brief about this person and their company. \
        Include: professional background, notable achievements, social media presence, \
        company overview, competitive positioning, and any publicly available information. \
        Write as a structured intelligence report, not JSON.",
        contact_name,
        contact.company_name.as_deref().unwrap_or("unknown"),
        contact.email.as_deref().unwrap_or("unknown"),
        contact.job_title.as_deref().unwrap_or("unknown"),
    );

    let messages = vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": prompt}),
    ];

    // Use PCG Router — automatic priority-based fallback + cost tracking
    let (response, metadata) = WorkflowLLMService::completion_with_tools(
        pool,
        messages,
        &[],  // no tools needed for text research
        None, // use default model priority
        Some(2048),
        None,
    )
    .await
    .map_err(|e| format!("PCG Router LLM call failed: {}", e))?;

    let response_text = match response {
        LLMResponse::Text { content, .. } => content,
        LLMResponse::ToolCalls { .. } => {
            return Err("Unexpected tool calls in research response".into());
        }
    };

    tracing::info!(
        "[Scout] PCG Router response for {} via {}/{}: {} chars",
        contact_name,
        metadata.provider,
        metadata.model_used,
        response_text.len()
    );

    // Check for error responses — reset status to idle for retry
    if response_text.contains("rate_limit_error")
        || response_text.contains("rate limit")
        || response_text.contains("credit balance")
    {
        tracing::warn!(
            "[Scout] Research hit API limit for '{}', resetting to idle for retry",
            contact_name
        );
        let _ = sqlx::query(
            "UPDATE crm_contacts SET intelligence_status = 'idle', updated_at = datetime('now','subsec') \
             WHERE CAST(id AS TEXT) = ?",
        )
        .bind(contact.id.as_ref())
        .execute(pool)
        .await;
        return Ok(());
    }

    let summary = extract_summary_from_response(&response_text);
    let confidence = extract_confidence_from_response(&response_text);

    write_intelligence_results(
        pool,
        contact.id.as_ref(),
        &summary,
        confidence,
        &response_text,
        project_id,
        contact_name,
    )
    .await?;
    Ok(())
}

/// Write intelligence results to crm_contacts.
/// Accepts either a contact_id directly or a person_id (legacy bridge).
pub async fn write_intelligence_results(
    pool: &sqlx::SqlitePool,
    contact_or_person_id: &str,
    summary: &str,
    confidence: f64,
    raw: &str,
    project_id: Option<Uuid>,
    full_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try direct contact update first, fall back to person bridge
    let rows = sqlx::query(
        "UPDATE crm_contacts SET \
         intelligence_status = 'done', \
         intelligence_summary = ?1, \
         intelligence_raw = ?2, \
         intelligence_confidence = ?3, \
         intelligence_last_run_at = datetime('now','subsec'), \
         intelligence_agent = 'scout', \
         research_pass_count = COALESCE(research_pass_count, 0) + 1, \
         updated_at = datetime('now','subsec') \
         WHERE CAST(id AS TEXT) = ?4",
    )
    .bind(summary)
    .bind(raw)
    .bind(confidence)
    .bind(contact_or_person_id)
    .execute(pool)
    .await?;

    let affected = rows.rows_affected();

    if affected == 0 {
        tracing::error!(
            "[Scout] No crm_contacts row found for id '{}' — intelligence data lost!",
            contact_or_person_id
        );
    }

    // Register in knowledge graph
    if let Some(pid) = project_id {
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Entity,
            contact_or_person_id,
            &format!("Contact: {}", full_name),
            Some(&format!("Scout intelligence: {}", summary)),
            confidence,
        )
        .await;
    }

    tracing::info!(
        "[Scout] Intelligence complete for '{}' (confidence: {:.0}%)",
        full_name,
        confidence * 100.0
    );
    Ok(())
}

// ── Text extraction helpers ───────────────────────────────────────────────────

fn extract_summary_from_json(v: &serde_json::Value) -> Option<String> {
    // Direct "summary" key
    if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
        return Some(s.to_string());
    }
    // Nested under "subject" or "pcg_contact_intelligence"
    for key in &["pcg_contact_intelligence", "subject", "person", "contact"] {
        if let Some(nested) = v.get(key) {
            if let Some(s) = nested.get("summary").and_then(|s| s.as_str()) {
                return Some(s.to_string());
            }
            // One more level: subject.overview, subject.professional_summary, etc.
            for inner in &["overview", "professional_summary", "bio", "profile_summary"] {
                if let Some(s) = nested.get(inner).and_then(|s| s.as_str()) {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn extract_summary_from_response(text: &str) -> String {
    // Try direct JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(s) = extract_summary_from_json(&v) {
            return s;
        }
    }
    // Try JSON block — scan for embedded JSON
    let mut search = text;
    while let Some(start) = search.find('{') {
        let slice = &search[start..];
        if let Some(end) = slice.rfind('}') {
            if end > 0 {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&slice[..=end]) {
                    if let Some(s) = extract_summary_from_json(&v) {
                        return s;
                    }
                }
            }
        }
        // Move past this brace and keep searching
        search = &search[start + 1..];
        if search.is_empty() {
            break;
        }
    }
    // Fall back to first 300 chars of text (strip markdown fences)
    let clean = text
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim();
    clean.chars().take(300).collect()
}

fn extract_confidence_from_response(text: &str) -> f64 {
    // Try to parse JSON and extract confidence field
    let json_str = if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        &text[start..=end]
    } else {
        text
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(c) = v.get("confidence").and_then(|c| c.as_f64()) {
            return c.clamp(0.0, 1.0);
        }
    }
    // Heuristic: longer responses tend to be more confident
    if text.len() > 500 {
        0.7
    } else if text.len() > 200 {
        0.5
    } else {
        0.3
    }
}

fn parse_research_json(text: &str) -> serde_json::Value {
    // Try direct parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return v;
    }
    // Try extracting JSON block
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                    return v;
                }
            }
        }
    }
    serde_json::Value::Object(Default::default())
}

async fn update_company_contact_methods(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    parsed: &serde_json::Value,
) {
    let contact_keys: &[(&str, &str, &str)] = &[
        ("company_phone", "phone", "Phone"),
        ("company_email", "email", "Email"),
        ("company_website", "website", "Website"),
        ("phone", "phone", "Phone"),
        ("email", "email", "Email"),
        ("website", "website", "Website"),
    ];
    for (json_key, method_type, label) in contact_keys {
        if let Some(val) = parsed.get(*json_key).and_then(|v| v.as_str()) {
            if !val.trim().is_empty() {
                let _ = sqlx::query(
                    "INSERT INTO company_contact_methods (id, company_id, method_type, label, value) \
                     VALUES (randomblob(16), ?, ?, ?, ?) ON CONFLICT DO NOTHING",
                )
                .bind(company_id)
                .bind(*method_type)
                .bind(*label)
                .bind(val.trim())
                .execute(pool)
                .await;
            }
        }
    }
}

// ── Company Intelligence ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CompanyResearchRequest {
    pub agent_preference: Option<String>,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CompanyResearchJobResponse {
    pub company_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CompanyIntelligenceStatusResponse {
    pub company_id: Uuid,
    pub status: String,
    pub summary: Option<String>,
    pub confidence: f64,
    pub agent: Option<String>,
    pub last_run_at: Option<String>,
}

/// POST /api/companies/:id/research
pub async fn trigger_company_research(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<String>,
    Json(body): Json<CompanyResearchRequest>,
) -> Result<Json<ApiResponse<CompanyResearchJobResponse>>, ApiError> {
    let company_id = DbUuid::parse(&company_id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    use db::models::company::Company;
    let pool = d.db().pool.clone();
    let company_db_id = DbUuid::from(company_id);

    let company = Company::find_by_id(&pool, &company_db_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    // Use string binding to match TEXT-stored company IDs
    sqlx::query(
        "UPDATE companies SET intelligence_status = 'queued', updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
    )
    .bind(company_id.to_string())
    .execute(&pool)
    .await?;

    let company_name = company.name.clone();
    let project_id = body.project_id;
    let pool2 = pool.clone();

    tokio::spawn(async move {
        // Use direct research (OpenAI-first) instead of Nora agent routing
        run_company_research_direct(&pool2, company_id, &company_name, project_id).await;
    });

    Ok(Json(ApiResponse::success(CompanyResearchJobResponse {
        company_id,
        status: "queued".into(),
        message: format!(
            "Research queued for {} — Astra will gather company intelligence.",
            company.name
        ),
    })))
}

/// GET /api/companies/:id/intelligence-status
pub async fn get_company_intelligence_status(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<String>,
) -> Result<Json<ApiResponse<CompanyIntelligenceStatusResponse>>, ApiError> {
    let company_id = DbUuid::parse(&company_id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    #[derive(sqlx::FromRow)]
    struct Row {
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: Option<f64>,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }
    let pool = &d.db().pool;
    let row: Option<Row> = sqlx::query_as(
        "SELECT intelligence_status, intelligence_summary, intelligence_confidence, \
         intelligence_agent, intelligence_last_run_at FROM companies WHERE id = ?",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Company not found".into()))?;
    Ok(Json(ApiResponse::success(
        CompanyIntelligenceStatusResponse {
            company_id,
            status: row.intelligence_status,
            summary: row.intelligence_summary,
            confidence: row.intelligence_confidence.unwrap_or(0.0),
            agent: row.intelligence_agent,
            last_run_at: row.intelligence_last_run_at,
        },
    )))
}

#[allow(dead_code)]
async fn run_company_research(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    company_name: &str,
    project_id: Option<Uuid>,
) {
    sqlx::query(
        "UPDATE companies SET intelligence_status = 'running', updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(company_id)
    .execute(pool)
    .await
    .ok();

    run_company_research_direct(pool, company_id, company_name, project_id).await;
}

pub async fn run_company_research_direct(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    company_name: &str,
    project_id: Option<Uuid>,
) {
    // Simulation mode: write realistic placeholder data without calling LLM
    if std::env::var("SIMULATE_LLM").unwrap_or_default() == "1" {
        tracing::info!(
            "[Scout] Simulated company research for '{}' (SIMULATE_LLM=1)",
            company_name
        );
        let summary = format!(
            "{} is a mid-size company positioned for growth in their market segment. \
             Key opportunities: digital transformation, process automation, and strategic partnerships. \
             Competitive landscape shows room for differentiation through technology adoption.",
            company_name
        );
        write_company_intel_results(pool, company_id, &summary, 0.75).await;
        // Register in knowledge graph if project scope provided
        if let Some(pid) = project_id {
            let _ = ProjectKnowledgeSource::upsert_source(
                pool,
                pid,
                &KnowledgeSourceType::Entity,
                &company_id.to_string(),
                &format!("Company: {}", company_name),
                Some(&format!("Scout intelligence: {}", summary)),
                0.75,
            )
            .await;
        }
        return;
    }

    use services::services::workflow_llm::{LLMResponse, WorkflowLLMService};

    tracing::info!(
        "[Scout] Starting company research for {} (id: {}) via PCG Router",
        company_name,
        company_id
    );

    let prompt = format!(
        "Research the company '{}'. Provide a comprehensive company intelligence brief including: \
        1) Company overview and background, \
        2) Website and contact information, \
        3) Social media presence, \
        4) Leadership and key personnel, \
        5) Business model and services/products, \
        6) Market positioning and competitive landscape, \
        7) Key opportunities for a creative agency partnership. \
        Write as a structured intelligence report.",
        company_name
    );

    let system = "You are Scout, a business intelligence analyst. Research companies thoroughly and produce comprehensive intelligence briefs.";

    let messages = vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": prompt}),
    ];

    let response_text = match WorkflowLLMService::completion_with_tools(
        pool,
        messages,
        &[],
        None,
        Some(3000),
        None,
    )
    .await
    {
        Ok((LLMResponse::Text { content, .. }, metadata)) => {
            tracing::info!(
                "[Scout] PCG Router company research for {} via {}/{}: {} chars",
                company_name,
                metadata.provider,
                metadata.model_used,
                content.len()
            );
            content
        }
        Ok((LLMResponse::ToolCalls { .. }, _)) => {
            tracing::error!("[Scout] Unexpected tool calls in company research");
            write_company_intel_results(
                pool,
                company_id,
                "LLM returned tool calls instead of text",
                0.0,
            )
            .await;
            return;
        }
        Err(e) => {
            tracing::error!(
                "[Scout] PCG Router failed for company {}: {}",
                company_name,
                e
            );
            write_company_intel_results(pool, company_id, &format!("Research failed: {}", e), 0.0)
                .await;
            return;
        }
    };

    let parsed = parse_research_json(&response_text);
    let confidence = parsed
        .get("confidence")
        .and_then(|c| c.as_f64())
        .unwrap_or_else(|| if response_text.len() > 300 { 0.75 } else { 0.3 })
        .clamp(0.0, 1.0);
    let summary = if response_text.len() > 50 {
        parsed
            .get("summary")
            .and_then(|s| s.as_str())
            .unwrap_or(&response_text[..response_text.len().min(2000)])
            .to_string()
    } else {
        format!("Research unavailable for {}", company_name)
    };

    // Update company record with found data
    let website = parsed
        .get("website")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let description = parsed
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let industry = parsed
        .get("industry")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let _ = sqlx::query(
        "UPDATE companies SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         intelligence_agent = 'scout', \
         website = COALESCE(NULLIF(?, ''), website), \
         description = COALESCE(NULLIF(?, ''), description), \
         industry = COALESCE(NULLIF(?, ''), industry), \
         updated_at = datetime('now','subsec') \
         WHERE CAST(id AS TEXT) = ?",
    )
    .bind(&summary)
    .bind(confidence)
    .bind(website)
    .bind(description)
    .bind(industry)
    .bind(company_id.to_string())
    .execute(pool)
    .await;

    // Add contact methods found in research
    update_company_contact_methods(pool, company_id, &parsed).await;

    // Add social profiles as contact methods
    let social_keys: &[(&str, &str)] = &[
        ("instagram", "Instagram"),
        ("linkedin", "LinkedIn"),
        ("twitter", "Twitter"),
        ("facebook", "Facebook"),
    ];
    for (key, label) in social_keys {
        if let Some(val) = parsed.get(*key).and_then(|v| v.as_str()) {
            if !val.trim().is_empty() {
                let _ = sqlx::query(
                    "INSERT INTO company_contact_methods (id, company_id, method_type, label, value) \
                     VALUES (randomblob(16), ?, ?, ?, ?) ON CONFLICT DO NOTHING",
                )
                .bind(company_id)
                .bind(*key)
                .bind(*label)
                .bind(val.trim())
                .execute(pool)
                .await;
            }
        }
    }

    // Register in knowledge graph if project_id provided
    if let Some(pid) = project_id {
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Entity,
            &company_id.to_string(),
            &format!("Company: {}", company_name),
            Some(&summary),
            confidence,
        )
        .await;
    }

    tracing::info!(
        "Company research complete for {} (confidence: {:.0}%)",
        company_name,
        confidence * 100.0
    );
}

async fn write_company_intel_results(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    summary: &str,
    confidence: f64,
) {
    let _ = sqlx::query(
        "UPDATE companies SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         updated_at = datetime('now','subsec') \
         WHERE CAST(id AS TEXT) = ?",
    )
    .bind(summary)
    .bind(confidence)
    .bind(company_id.to_string())
    .execute(pool)
    .await;
}

// ── Contact-first research (called from deal automations) ────────────────────

/// Run Scout research directly on a crm_contact — no person bridge needed.
/// Writes results to crm_contacts.intelligence_* columns.
pub async fn run_contact_research_direct(
    pool: &sqlx::SqlitePool,
    contact_id: Uuid,
    full_name: &str,
    email: &str,
    company_name: &str,
    job_title: &str,
    project_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use services::services::workflow_llm::{LLMResponse, WorkflowLLMService};

    tracing::info!(
        "[Scout] Starting contact research for '{}' (id: {}) via PCG Router",
        full_name,
        contact_id
    );

    let system = "You are Scout, Social Intelligence Analyst for Power Club Global. \
        Your specialty is finding and structuring online presence data about individuals. \
        Research the person thoroughly based on your knowledge. Return their: \
        current role, company details, social profiles, recent activity, and public bio. \
        Return a detailed intelligence summary as plain text (not JSON).";

    let prompt = format!(
        "Research this contact for PCG: Name={}, Company={}, Email={}, Title={}. \
        Provide a comprehensive intelligence brief about this person and their company. \
        Include: professional background, notable achievements, social media presence, \
        company overview, competitive positioning, and any publicly available information. \
        Write as a structured intelligence report, not JSON.",
        full_name,
        if company_name.is_empty() {
            "unknown"
        } else {
            company_name
        },
        if email.is_empty() { "unknown" } else { email },
        if job_title.is_empty() {
            "unknown"
        } else {
            job_title
        },
    );

    let messages = vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": prompt}),
    ];

    let (response, metadata) =
        WorkflowLLMService::completion_with_tools(pool, messages, &[], None, Some(2048), None)
            .await
            .map_err(|e| format!("PCG Router LLM call failed: {}", e))?;

    let response_text = match response {
        LLMResponse::Text { content, .. } => content,
        LLMResponse::ToolCalls { .. } => {
            // Write failure status so it's visible, not silent
            let _ = sqlx::query(
                "UPDATE crm_contacts SET intelligence_status = 'failed', \
                 intelligence_summary = 'LLM returned tool calls instead of text', \
                 updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
            )
            .bind(contact_id.to_string())
            .execute(pool)
            .await;
            return Err("Unexpected tool calls in contact research response".into());
        }
    };

    tracing::info!(
        "[Scout] PCG Router response for '{}' via {}/{}: {} chars",
        full_name,
        metadata.provider,
        metadata.model_used,
        response_text.len()
    );

    // Check for API rate limit errors — reset to idle for retry
    if response_text.contains("rate_limit_error")
        || response_text.contains("rate limit")
        || response_text.contains("credit balance")
    {
        tracing::warn!(
            "[Scout] Research hit API limit for '{}', resetting to idle for retry",
            full_name
        );
        let _ = sqlx::query(
            "UPDATE crm_contacts SET intelligence_status = 'idle', \
             updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
        )
        .bind(contact_id.to_string())
        .execute(pool)
        .await;
        return Ok(());
    }

    let summary = extract_summary_from_response(&response_text);
    let confidence = extract_confidence_from_response(&response_text);

    // Write directly to crm_contacts — no person bridge
    let rows = sqlx::query(
        "UPDATE crm_contacts SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_raw = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         intelligence_agent = 'scout', \
         research_pass_count = COALESCE(research_pass_count, 0) + 1, \
         updated_at = datetime('now','subsec') \
         WHERE CAST(id AS TEXT) = ?",
    )
    .bind(&summary)
    .bind(&response_text)
    .bind(confidence)
    .bind(contact_id.to_string())
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        tracing::error!(
            "[Scout] Contact {} not found when writing research results — data lost!",
            contact_id
        );
        return Err(format!("Contact {} not found when writing results", contact_id).into());
    }

    // Register in knowledge graph if project_id provided
    if let Some(pid) = project_id {
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Entity,
            &contact_id.to_string(),
            &format!("Contact: {}", full_name),
            Some(&format!("Scout intelligence: {}", summary)),
            confidence,
        )
        .await;
    }

    tracing::info!(
        "[Scout] Contact research complete for '{}' (confidence: {:.0}%)",
        full_name,
        confidence * 100.0
    );
    Ok(())
}

/// Trigger contact research internally (called from stage-transition hooks).
/// Assumes intelligence_status is already set to 'queued'.
pub async fn trigger_research_for_contact(
    pool: &sqlx::SqlitePool,
    contact_id: &DbUuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let contact = CrmContact::find_by_id(pool, contact_id)
        .await
        .map_err(|e| format!("Contact not found: {}", e))?;

    let contact_name = contact.full_name.as_deref().unwrap_or("Unknown");
    let research_prompt = format!(
        "[INTELLIGENCE TASK — delegate to Scout] \
         Research the following contact and return ONLY a valid JSON object (no markdown, no preamble): \
         Name: {}, Email: {}, Company: {}, Job Title: {}, Person type: {}. \
         Use web search to find their professional background, social media, and their company's \
         public contact information, website, Google My Business listing, and social media. \
         \
         Return this exact JSON structure: \
         {{ \
           \"summary\": \"1-2 sentence professional profile\", \
           \"social_profiles\": [{{\"platform\": \"linkedin\", \"handle\": \"...\", \"url\": \"...\", \"followers\": 0}}], \
           \"company_description\": \"brief company description\", \
           \"company_website\": \"https://...\", \
           \"company_phone\": \"+1...\", \
           \"company_email\": \"info@...\", \
           \"company_instagram\": \"@handle\", \
           \"company_linkedin\": \"url\", \
           \"company_twitter\": \"@handle\", \
           \"company_facebook\": \"url\", \
           \"gmb_rating\": 4.5, \
           \"gmb_review_count\": 42, \
           \"deal_potential\": \"high\", \
           \"recommended_approach\": \"1 sentence\", \
           \"confidence\": 0.8 \
         }}",
        contact_name,
        contact.email.as_deref().unwrap_or("unknown"),
        contact.company_name.as_deref().unwrap_or("unknown"),
        contact.job_title.as_deref().unwrap_or("unknown"),
        contact.person_type.as_deref().unwrap_or("contact"),
    );

    run_research_via_nora(pool, contact, research_prompt, None).await
}

// ── Contact-targeted endpoints ────────────────────────────────────────────────
// These read/write crm_contacts directly — the canonical path after unification.

/// POST /api/crm/contacts/:id/research — trigger research for a CRM contact
pub async fn trigger_contact_research(
    Extension(_access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
    Json(body): Json<ResearchRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let contact_id =
        DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let pool = &d.db().pool;

    // Read contact directly — no person bridge needed
    #[derive(sqlx::FromRow)]
    struct ContactRow {
        full_name: Option<String>,
        email: Option<String>,
        company_name: Option<String>,
        job_title: Option<String>,
    }
    let contact: ContactRow = sqlx::query_as(
        "SELECT full_name, email, company_name, job_title FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    // Set status to queued on contact
    sqlx::query(
        "UPDATE crm_contacts SET intelligence_status = 'queued', \
         intelligence_agent = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(body.agent_preference.as_deref().unwrap_or("scout"))
    .bind(&contact_id)
    .execute(pool)
    .await?;

    // Run research directly on contact — no person bridge
    let contact_uuid = contact_id.to_uuid();
    let pool_clone = pool.clone();
    let name = contact
        .full_name
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let email = contact.email.clone().unwrap_or_default();
    let company = contact.company_name.clone().unwrap_or_default();
    let title = contact.job_title.clone().unwrap_or_default();
    let project_id = body.project_id;
    tokio::spawn(async move {
        if let Err(e) = run_contact_research_direct(
            &pool_clone,
            contact_uuid,
            &name,
            &email,
            &company,
            &title,
            project_id,
        )
        .await
        {
            tracing::error!(
                "[Scout] Contact research failed for {} ({}): {}",
                name,
                contact_uuid,
                e
            );
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "contact_id": contact_id.to_string(),
        "status": "queued",
        "message": format!("Research queued for {}", contact.full_name.unwrap_or_else(|| "contact".into())),
    }))))
}

/// GET /api/crm/contacts/:id/intelligence-status
pub async fn get_contact_intelligence_status(
    Extension(_access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let contact_id =
        DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let pool = &d.db().pool;

    #[derive(sqlx::FromRow)]
    struct Row {
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: f64,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT intelligence_status, intelligence_summary, intelligence_confidence, \
         intelligence_agent, intelligence_last_run_at FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "contact_id": contact_id.to_string(),
        "status": row.intelligence_status,
        "summary": row.intelligence_summary,
        "confidence": row.intelligence_confidence,
        "agent": row.intelligence_agent,
        "last_run_at": row.intelligence_last_run_at,
    }))))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Contact intelligence (canonical)
        .route(
            "/crm/contacts/{id}/research",
            post(trigger_contact_research),
        )
        .route(
            "/crm/contacts/{id}/intelligence-status",
            get(get_contact_intelligence_status),
        )
        .route(
            "/crm/contacts/{id}/research-passes",
            get(list_research_passes),
        )
        .route(
            "/crm/contacts/{id}/research-passes/next",
            post(trigger_next_research_pass),
        )
        .route("/crm/contacts/{id}/reports", get(list_person_reports))
        // Company intelligence
        .route(
            "/companies/{id}/intelligence-status",
            get(get_company_intelligence_status),
        )
        .with_state(deployment.clone())
}

// ── Iterative research pass endpoints ────────────────────────────────────────

use db::models::{business_report::BusinessReport, contact_research_pass::ContactResearchPass};

#[derive(Debug, Deserialize)]
pub struct NextPassRequest {
    /// Override the auto-selected focus: 'identity' | 'market_position' | 'competitors' | 'target_clients' | 'deep_strategy' | 'custom'
    pub focus: Option<String>,
    pub custom_prompt: Option<String>,
    pub project_id: Option<Uuid>,
}

/// GET /api/crm/contacts/:id/research-passes — list all research passes
pub async fn list_research_passes(
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ContactResearchPass>>>, ApiError> {
    let contact_id =
        DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let passes = ContactResearchPass::list_for_contact(&d.db().pool, &contact_id).await?;
    Ok(Json(ApiResponse::success(passes)))
}

/// POST /api/crm/contacts/:id/research-passes/next
/// Triggers the next logical research pass, building on all prior passes.
pub async fn trigger_next_research_pass(
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
    Json(body): Json<NextPassRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let contact_id =
        DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let pool = &d.db().pool;

    // Read contact directly
    #[derive(sqlx::FromRow)]
    struct ContactRow {
        full_name: Option<String>,
        company_name: Option<String>,
        intelligence_summary: Option<String>,
    }
    let contact: ContactRow = sqlx::query_as(
        "SELECT full_name, company_name, intelligence_summary FROM crm_contacts WHERE id = ?",
    )
    .bind(contact_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    // Determine next pass number and auto-select focus
    let pass_number = ContactResearchPass::next_pass_number(pool, &contact_id).await;
    let focus = body
        .focus
        .clone()
        .unwrap_or_else(|| auto_focus(pass_number));

    // Collect prior pass summaries for context
    let prior_passes = ContactResearchPass::list_for_contact(pool, &contact_id).await?;
    let prior_context: String = prior_passes
        .iter()
        .filter(|p| p.status == "done")
        .map(|p| {
            format!(
                "Pass {} ({}): {}",
                p.pass_number,
                p.research_focus,
                p.summary.as_deref().unwrap_or("(no summary)")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let pass = ContactResearchPass::create(
        pool,
        &contact_id,
        pass_number,
        &focus,
        body.custom_prompt.as_deref(),
    )
    .await?;
    let pass_id = pass.id.to_uuid();

    // Build research prompt incorporating all prior context
    let intel = contact
        .intelligence_summary
        .as_deref()
        .unwrap_or("")
        .to_string();
    let name = contact.full_name.unwrap_or_else(|| "Unknown".into());
    let company = contact
        .company_name
        .unwrap_or_else(|| "Unknown company".into());
    let project_id = body.project_id;
    let pool_clone = pool.clone();
    let pool_for_err = pool.clone();
    let focus_for_resp = focus.clone();
    let contact_id_str = contact_id.to_string();

    tokio::spawn(async move {
        if let Err(e) = run_research_pass(
            pool_clone,
            pass_id,
            &contact_id_str,
            pass_number,
            &focus,
            &name,
            &company,
            &intel,
            &prior_context,
            project_id,
        )
        .await
        {
            tracing::error!("Research pass failed for {}: {}", pass_id, e);
            let _ = sqlx::query(
                "UPDATE contact_research_passes SET status = 'failed', error = ?, completed_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
            )
            .bind(e.to_string())
            .bind(pass_id.to_string())
            .execute(&pool_for_err)
            .await;
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "pass_id": pass_id,
        "pass_number": pass_number,
        "focus": focus_for_resp,
        "status": "queued"
    }))))
}

/// GET /api/crm/contacts/:id/reports
pub async fn list_person_reports(
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<BusinessReport>>>, ApiError> {
    DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let reports = BusinessReport::list_by_contact(&d.db().pool, &contact_id).await?;
    Ok(Json(ApiResponse::success(reports)))
}

fn auto_focus(pass_number: i64) -> String {
    match pass_number {
        1 => "identity",
        2 => "market_position",
        3 => "competitors",
        4 => "target_clients",
        _ => "deep_strategy",
    }
    .to_string()
}

async fn run_research_pass(
    pool: sqlx::SqlitePool,
    pass_id: Uuid,
    contact_id: &str,
    pass_number: i64,
    focus: &str,
    name: &str,
    company: &str,
    existing_intel: &str,
    prior_context: &str,
    project_id: Option<Uuid>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    // Mark running
    sqlx::query("UPDATE contact_research_passes SET status = 'running' WHERE CAST(id AS TEXT) = ?")
        .bind(pass_id.to_string())
        .execute(&pool)
        .await?;

    let focus_instructions = match focus {
        "identity" => format!(
            "Research who {} at {} is: professional background, career history, social presence, \
             personal brand, public statements, media appearances.",
            name, company
        ),
        "market_position" => format!(
            "Research {}'s market position and business model: revenue, growth stage, \
             funding, industry standing, key partnerships, recent news.",
            company
        ),
        "competitors" => format!(
            "Identify {}'s top 5-8 direct competitors. For each: name, positioning, \
             strengths/weaknesses, how {} differentiates from them.",
            company, company
        ),
        "target_clients" => format!(
            "Research who {}'s ideal and current clients/customers are: industries, \
             company sizes, demographics, key use cases, testimonials or case studies.",
            company
        ),
        "deep_strategy" => format!(
            "Deep strategic analysis of {}: growth strategy, product roadmap signals, \
             hiring patterns, content strategy, event participation, PCG engagement opportunity.",
            company
        ),
        _ => format!("Research: {}", focus),
    };

    let prior_section = if prior_context.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nPRIOR RESEARCH (build on this, don't repeat):\n{}",
            prior_context
        )
    };

    let existing_section = if existing_intel.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nEXISTING INTELLIGENCE:\n{}",
            &existing_intel[..existing_intel.len().min(2000)]
        )
    };

    let system = "You are an expert business intelligence researcher. Conduct thorough web research \
        and return structured findings. Always respond with valid JSON only.";

    let prompt = format!(
        "RESEARCH TASK — Pass #{pass_number} | Focus: {focus}\n\n\
        Subject: {name} ({company})\n\
        {existing_section}{prior_section}\n\n\
        FOCUS FOR THIS PASS:\n{focus_instructions}\n\n\
        Return JSON:\n\
        {{\n\
          \"summary\": \"3-5 sentence summary of findings for this pass\",\n\
          \"key_findings\": [\n\
            {{\"finding\": \"string\", \"confidence\": \"high|medium|low\", \"source\": \"where found or inferred\"}}\n\
          ],\n\
          \"search_queries\": [\"queries you would use to verify this\"],\n\
          \"updated_intelligence\": \"comprehensive updated intelligence summary incorporating all passes\",\n\
          \"confidence_score\": 0.0-1.0\n\
        }}"
    );

    use services::services::workflow_llm::{LLMResponse, WorkflowLLMService};

    let messages = vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": prompt}),
    ];

    let (response, metadata) =
        WorkflowLLMService::completion_with_tools(&pool, messages, &[], None, Some(4096), None)
            .await
            .map_err(|e| anyhow::anyhow!("PCG Router failed for research pass: {}", e))?;

    let text = match response {
        LLMResponse::Text { content, .. } => content,
        LLMResponse::ToolCalls { .. } => {
            anyhow::bail!("Unexpected tool calls in research pass response");
        }
    };

    tracing::info!(
        "[Scout] Research pass #{} via {}/{}: {} chars",
        pass_number,
        metadata.provider,
        metadata.model_used,
        text.len()
    );

    let text = &text;

    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: Value = serde_json::from_str(json_str).map_err(|e| {
        anyhow::anyhow!(
            "JSON parse error: {e}\nRaw: {}",
            &json_str[..json_str.len().min(300)]
        )
    })?;

    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let updated_intel = parsed["updated_intelligence"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let confidence_score = parsed["confidence_score"].as_f64().unwrap_or(0.5);
    let key_findings =
        serde_json::to_string(&parsed["key_findings"]).unwrap_or_else(|_| "[]".into());
    let search_queries =
        serde_json::to_string(&parsed["search_queries"]).unwrap_or_else(|_| "[]".into());

    // Save pass results
    sqlx::query(
        "UPDATE contact_research_passes SET
            status = 'done',
            summary = ?,
            raw_results = ?,
            key_findings = ?,
            search_queries = ?,
            confidence_delta = ?,
            agent_used = 'claude',
            completed_at = datetime('now','subsec')
         WHERE CAST(id AS TEXT) = ?",
    )
    .bind(&summary)
    .bind(json_str)
    .bind(&key_findings)
    .bind(&search_queries)
    .bind(confidence_score)
    .bind(pass_id.to_string())
    .execute(&pool)
    .await?;

    // Update crm_contacts with accumulated intelligence
    if !updated_intel.is_empty() {
        let new_confidence = (confidence_score as f64).min(1.0);
        let depth = match pass_number {
            1 => "shallow",
            2 | 3 => "moderate",
            _ => "deep",
        };
        sqlx::query(
            "UPDATE crm_contacts SET
                intelligence_summary = ?,
                intelligence_status = 'done',
                intelligence_last_run_at = datetime('now','subsec'),
                intelligence_confidence = ?,
                intelligence_agent = 'claude',
                research_pass_count = ?,
                research_depth = ?,
                updated_at = datetime('now','subsec')
             WHERE CAST(id AS TEXT) = ?",
        )
        .bind(&updated_intel)
        .bind(new_confidence)
        .bind(pass_number)
        .bind(depth)
        .bind(contact_id)
        .execute(&pool)
        .await?;
    }

    // Register each key finding in the project knowledge graph
    if let Some(pid) = project_id {
        let title = format!(
            "Research Pass #{}: {} — {}",
            pass_number,
            focus,
            &name[..name.len().min(40)]
        );
        let _ = ProjectKnowledgeSource::upsert_source(
            &pool,
            pid,
            &KnowledgeSourceType::Entity,
            &pass_id.to_string(),
            &title,
            Some(&summary),
            confidence_score,
        )
        .await;
    }

    // Also register in the org-level knowledge graph (owner_type='organization')
    // Find org from person's associations
    #[derive(sqlx::FromRow)]
    struct OrgRow {
        org_id: Option<Uuid>,
    }
    if let Ok(Some(row)) = sqlx::query_as::<_, OrgRow>(
        "SELECT col.organization_id AS org_id FROM contact_organization_links col WHERE col.crm_contact_id = ? LIMIT 1",
    )
    .bind(contact_id)
    .fetch_optional(&pool)
    .await {
        if let Some(org_id) = row.org_id {
            let title = format!("[Org KG] Research Pass #{}: {} — {}", pass_number, focus, &name[..name.len().min(40)]);
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO project_knowledge_sources
                 (id, owner_type, owner_id, source_type, source_id, source_title, source_summary,
                  coverage_score, auto_registered, is_active, created_at, updated_at)
                 VALUES (?, 'organization', ?, 'entity', ?, ?, ?, ?, 1, 1, datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(DbUuid::new().to_string())
            .bind(org_id.to_string())
            .bind(pass_id.to_string())
            .bind(&title)
            .bind(&summary)
            .bind(confidence_score)
            .execute(&pool)
            .await;
        }
    }

    tracing::info!(
        "[Scout] Research pass #{} complete for contact {} ({})",
        pass_number,
        contact_id,
        focus
    );
    Ok(())
}

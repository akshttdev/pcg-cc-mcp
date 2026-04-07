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
    extract::{Path, State},
    routing::{get, post},
    Extension, Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::{
        crm_contact::CrmContact,
        project_knowledge_source::{
            KnowledgeOwnerScope, KnowledgeSourceType, ProjectKnowledgeSource,
        },
    },
};
use deployment::Deployment;
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    error::ApiError, helpers::uuid_params::parse_db_uuid_param,
    middleware::access_control::AccessContext, routes::nora::get_nora_instance, DeploymentImpl,
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

    // Pull any prior KG entity entries for this person to prime the research
    let prior_kg = if let Some(pid) = project_id {
        ProjectKnowledgeSource::find_by_project(pool, pid)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|s| {
                s.source_type == "entity" && s.source_id == contact.id.as_ref() && !s.is_stale
            })
            .filter_map(|s| s.source_summary)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };

    let _prior_context_section = if prior_kg.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nPrior intelligence already collected (do not repeat, build on this):\n{}",
            prior_kg
        )
    };

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
        if let Err(e) = sqlx::query(
            "UPDATE crm_contacts SET intelligence_status = 'idle', updated_at = datetime('now','subsec') \
             WHERE CAST(id AS TEXT) = ?",
        )
        .bind(contact.id.as_ref())
        .execute(pool)
        .await
        {
            tracing::warn!("[Scout] Failed to reset contact status to idle: {}", e);
        }
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

/// Write a person entity into every applicable Knowledge Graph scope:
///  1. Project scope  — if project_id provided
///  2. Org scope      — from person_organization_contacts; ALSO derived from project.organization_id
///  3. Company scope  — from person_company_roles joined to companies table
///
/// source_id_override: stable ID for the entry (person UUID for final pass, pass UUID per-pass).
/// title_prefix: shown in KG source list (e.g. "Person", "Pass #2 (business_profile)").
#[allow(dead_code)]
async fn register_person_in_kg(
    pool: &sqlx::SqlitePool,
    person_id: &str,
    full_name: &str,
    summary: &str,
    confidence: f64,
    project_id: Option<Uuid>,
    source_id_override: Option<&str>,
    title_prefix: Option<&str>,
) {
    let source_id = source_id_override.unwrap_or(person_id);
    let prefix = title_prefix.unwrap_or("Person");
    let title = format!("{}: {}", prefix, &full_name[..full_name.len().min(60)]);

    // ── 1. Project scope ────────────────────────────────────────────────────
    if let Some(pid) = project_id {
        let _ = ProjectKnowledgeSource::upsert_scoped(
            pool,
            &KnowledgeOwnerScope::Project,
            &pid.to_string(),
            Some(pid),
            &KnowledgeSourceType::Entity,
            source_id,
            &title,
            Some(summary),
            confidence,
        )
        .await;
    }

    // ── 2. Org scope ─────────────────────────────────────────────────────────
    // Collect org IDs from two sources: contact_organization_links + project.organization_id
    let mut org_ids: Vec<String> = Vec::new();

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct OrgRow {
        org_id: Uuid,
    }

    if let Ok(rows) = sqlx::query_as::<_, OrgRow>(
        "SELECT organization_id AS org_id FROM contact_organization_links WHERE crm_contact_id = ?",
    )
    .bind(person_id)
    .fetch_all(pool)
    .await
    {
        for r in rows {
            org_ids.push(r.org_id.to_string());
        }
    }

    // Derive org from project context (the org that owns the project that triggered research)
    if let Some(pid) = project_id {
        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct ProjOrgRow {
            org_id: Option<Uuid>,
        }
        if let Ok(Some(row)) = sqlx::query_as::<_, ProjOrgRow>(
            "SELECT organization_id AS org_id FROM projects WHERE id = ?",
        )
        .bind(pid)
        .fetch_optional(pool)
        .await
        {
            if let Some(oid) = row.org_id {
                let s = oid.to_string();
                if !org_ids.contains(&s) {
                    org_ids.push(s);
                }
            }
        }
    }

    org_ids.dedup();
    for org_id in &org_ids {
        let _ = ProjectKnowledgeSource::upsert_scoped(
            pool,
            &KnowledgeOwnerScope::Organization,
            org_id,
            None,
            &KnowledgeSourceType::Entity,
            source_id,
            &title,
            Some(summary),
            confidence,
        )
        .await;
    }

    // ── 3. Company scope ─────────────────────────────────────────────────────
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct CompanyRow {
        company_id: Uuid,
        company_name: String,
    }

    if let Ok(companies) = sqlx::query_as::<_, CompanyRow>(
        "SELECT c.id AS company_id, c.name AS company_name \
         FROM contact_company_roles pcr \
         JOIN companies c ON c.id = pcr.company_id \
         WHERE pcr.crm_contact_id = ?",
    )
    .bind(person_id)
    .fetch_all(pool)
    .await
    {
        for co in &companies {
            let co_title = format!(
                "{} at {}",
                &full_name[..full_name.len().min(40)],
                co.company_name
            );
            let _ = ProjectKnowledgeSource::upsert_scoped(
                pool,
                &KnowledgeOwnerScope::Company,
                &co.company_id.to_string(),
                None,
                &KnowledgeSourceType::Entity,
                source_id,
                &co_title,
                Some(summary),
                confidence,
            )
            .await;
        }
    }
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
    // Fall back to full text (strip markdown fences) — don't truncate, the LLM returned prose
    let clean = text
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim();
    clean.to_string()
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
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<String>,
) -> Result<Json<ApiResponse<CompanyIntelligenceStatusResponse>>, ApiError> {
    let company_id = DbUuid::parse(&company_id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    #[derive(sqlx::FromRow)]
    struct Row {
        organization_id: Option<String>,
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: Option<f64>,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }
    let pool = &d.db().pool;
    let row: Option<Row> = sqlx::query_as(
        "SELECT organization_id, intelligence_status, intelligence_summary, \
         intelligence_confidence, intelligence_agent, intelligence_last_run_at \
         FROM companies WHERE id = ?",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    // Verify org membership
    if let Some(ref org_id) = row.organization_id {
        access_context.require_org_membership(pool, org_id).await?;
    }
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

    // Fetch the company's known website to anchor the search to the right entity
    let known_website: Option<String> =
        sqlx::query_scalar("SELECT website FROM companies WHERE id = ?")
            .bind(company_id.as_bytes().as_slice())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .flatten();

    let website_ctx = match &known_website {
        Some(w) if !w.trim().is_empty() => format!(
            " Their official website is {}. Search that domain directly — do NOT research similarly-named businesses.",
            w
        ),
        _ => String::new(),
    };

    let prompt = format!(
        "Research the company '{name}' for Power Club Global's CRM.{website_ctx} \
        Use web search to gather current, accurate information about THIS specific business. \
        Return ONLY a valid JSON object (no markdown, no preamble) with these exact keys:\n\
        {{\
          \"summary\": \"one-sentence overview\",\
          \"executive_summary\": \"2-3 paragraph executive overview covering what they do, market position, and relevance\",\
          \"company_overview\": \"detailed description of services, business model, history\",\
          \"market_analysis\": \"market position, industry size, trends, growth trajectory\",\
          \"brand_positioning\": \"how they position themselves, brand identity, messaging\",\
          \"target_clients\": \"ideal customer profile, demographics, psychographics\",\
          \"digital_presence\": \"website quality, social media footprint, SEO presence, content strategy\",\
          \"competitors\": [{{\
            \"name\": \"competitor name\",\
            \"website\": \"url or null\",\
            \"strengths\": \"what they do well\",\
            \"weaknesses\": \"where they are weak\",\
            \"threat_level\": \"high|medium|low\"\
          }}],\
          \"pain_points\": [{{\
            \"point\": \"specific challenge or gap\",\
            \"severity\": \"high|medium|low\"\
          }}],\
          \"opportunities\": [{{\
            \"title\": \"opportunity name\",\
            \"description\": \"how PCG could help\",\
            \"priority\": \"high|medium|low\",\
            \"estimated_value\": \"rough revenue estimate e.g. $5K-$15K/mo\"\
          }}],\
          \"recommended_services\": [{{\
            \"name\": \"service name\",\
            \"rationale\": \"why this fits\",\
            \"timeline\": \"e.g. Immediate, 3-6 months\"\
          }}],\
          \"sources\": [{{\
            \"title\": \"page title\",\
            \"url\": \"source url\"\
          }}],\
          \"founder_name\": \"name or null\",\
          \"industry\": \"industry category\",\
          \"founded_year\": 2020,\
          \"size_estimate\": \"1-5 | 5-20 | 20-100 | 100+ employees\",\
          \"website\": \"primary website url\",\
          \"description\": \"one-line tagline or description\",\
          \"instagram\": \"handle or url or null\",\
          \"twitter\": \"handle or url or null\",\
          \"linkedin\": \"profile url or null\",\
          \"facebook\": \"page url or null\",\
          \"confidence\": 0.85\
        }}",
        name = company_name,
        website_ctx = website_ctx
    );

    let system = "You are Scout, a business intelligence analyst for Power Club Global. \
        Research companies thoroughly and return ONLY valid JSON — no markdown, no preamble, no backticks. \
        You MUST include every key specified in the user's request. Use null for any field you cannot find.";

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

    let raw_json = serde_json::to_string(&parsed).unwrap_or_else(|_| response_text.clone());
    if let Err(e) = sqlx::query(
        "UPDATE companies SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_raw = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         intelligence_agent = 'scout', \
         website = COALESCE(NULLIF(?, ''), website), \
         description = COALESCE(NULLIF(?, ''), description), \
         industry = COALESCE(NULLIF(?, ''), industry), \
         updated_at = datetime('now','subsec') \
         WHERE id = ? OR CAST(id AS TEXT) = ?",
    )
    .bind(&summary)
    .bind(&raw_json)
    .bind(confidence)
    .bind(website)
    .bind(description)
    .bind(industry)
    .bind(company_id.to_string()) // TEXT-stored companies.id
    .bind(company_id.to_string())
    .execute(pool)
    .await
    {
        tracing::warn!(
            "[Scout] Failed to update company intelligence status for {}: {}",
            company_id,
            e
        );
    }

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

    // Register in project KG scope (if project_id provided)
    if let Some(pid) = project_id {
        let _ = ProjectKnowledgeSource::upsert_scoped(
            pool,
            &KnowledgeOwnerScope::Project,
            &pid.to_string(),
            Some(pid),
            &KnowledgeSourceType::Entity,
            &company_id.to_string(),
            &format!("Company: {}", company_name),
            Some(&summary),
            confidence,
        )
        .await;
    }
    // Always register in the company's own KG scope
    let _ = ProjectKnowledgeSource::upsert_scoped(
        pool,
        &KnowledgeOwnerScope::Company,
        &company_id.to_string(),
        None,
        &KnowledgeSourceType::Entity,
        &company_id.to_string(),
        &format!("Company: {}", company_name),
        Some(&summary),
        confidence,
    )
    .await;

    // Phase I complete — advance pipeline (auto or human review depending on org)
    after_phase1_complete(pool, company_id, company_name, project_id).await;

    tracing::info!(
        "Company research complete for {} (confidence: {:.0}%)",
        company_name,
        confidence * 100.0
    );
}

/// After Stage 1 intel completes, auto-create a "Review Intel" task on each deal's project
async fn auto_create_intel_review_tasks(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    company_name: &str,
) {
    // Find deals linked to clients that map to this company
    #[derive(sqlx::FromRow)]
    struct DealRow {
        deal_id: String,
        project_id: Option<String>,
        owner_user_id: Option<String>,
    }

    let company_id_str = company_id.to_string();

    let deals: Vec<DealRow> = sqlx::query_as(
        r#"SELECT d.id as deal_id, d.project_id, d.owner_user_id
           FROM crm_deals d
           JOIN clients cl ON cl.id = d.client_id
           WHERE cl.company_id = ?
             AND d.stage NOT IN ('won', 'lost')
             AND d.project_id IS NOT NULL"#,
    )
    .bind(&company_id_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for deal in &deals {
        let Some(project_id) = &deal.project_id else {
            continue;
        };
        let task_id = uuid::Uuid::new_v4().to_string();
        let title = format!("Review Intel: {}", company_name);
        let description = format!(
            "Phase I Scout research is complete for {}. \
             Review the intel, correct any errors (website, contacts, market data), \
             and approve to trigger Phase II deep research (Astra Business Analysis Report).",
            company_name
        );
        let _ = sqlx::query(
            "INSERT INTO tasks (id, project_id, crm_deal_id, title, description, status, priority,
             assignee_id, tags, created_by, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 'todo', 'high', ?, 'intel_review', 'system',
             datetime('now','subsec'), datetime('now','subsec'))",
        )
        .bind(&task_id)
        .bind(project_id)
        .bind(&deal.deal_id)
        .bind(&title)
        .bind(&description)
        .bind(&deal.owner_user_id)
        .execute(pool)
        .await;
    }

    if !deals.is_empty() {
        tracing::info!(
            "[Scout] Created {} intel review tasks for company {}",
            deals.len(),
            company_name
        );
    }
}

/// Called when Phase I Scout research completes for a company.
/// - Marks the phase1_scout task as done.
/// - If the intake was from a trusted auto-pipeline sender (Sirak Studios):
///   immediately creates Phase II task and spawns Astra deep research.
/// - Otherwise: creates a human "Review Intel" task for operator approval.
async fn after_phase1_complete(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    company_name: &str,
    project_id: Option<Uuid>,
) {
    use crate::routes::intake::pipeline::SIRAK_CONFIG;

    // Mark phase1_scout task done
    let _ = sqlx::query(
        "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec')
         WHERE workflow_type = 'phase1_scout' AND entity_id = ? AND status != 'done'",
    )
    .bind(company_id.to_string())
    .execute(pool)
    .await;

    // Check if this company originated from an auto-pipeline intake (trusted Sirak sender)
    let is_auto_pipeline: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM call_intake_items ci
         WHERE ci.company_id = ?
           AND (ci.from_email LIKE '%@sirakstudios.com'
                OR ci.from_email = 'sirak@sirakstudios.com'
                OR ci.from_email = 'aaren@sirakstudios.com')",
    )
    .bind(company_id.to_string())
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(false);

    if is_auto_pipeline {
        tracing::info!(
            "[Auto-pipeline] Phase I done for '{}' — spawning Phase II (Astra)",
            company_name
        );

        // Create Phase II task as in_progress
        let task2_id = uuid::Uuid::new_v4().to_string();
        let task2_title = format!("Astra Report: {}", company_name);
        let task2_desc = format!(
            "Phase II — Astra deep research business analysis for {}.\n\
             Triggered automatically after Phase I Scout completion.\n\
             Company ID: {}",
            company_name, company_id
        );
        let _ = sqlx::query(
            "INSERT INTO tasks (id, project_id, title, description, status, priority,
             agent_id, created_by, tags, workflow_type, entity_type, entity_id,
             created_at, updated_at)
             VALUES (?, ?, ?, ?, 'inprogress', 'high', ?, 'auto-pipeline',
             '[\"phase2\",\"auto-pipeline\"]', 'phase2_astra', 'company', ?,
             datetime('now','subsec'), datetime('now','subsec'))",
        )
        .bind(&task2_id)
        .bind(SIRAK_CONFIG.project_id)
        .bind(&task2_title)
        .bind(&task2_desc)
        .bind(SIRAK_CONFIG.nora_agent_id)
        .bind(company_id.to_string())
        .execute(pool)
        .await;
        tracing::info!("[Auto-pipeline] Created Phase II task '{}'", task2_title);

        // Resolve client_id and deal_id for Phase II context
        let client_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM clients WHERE company_id = ? AND organization_id = ?
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(company_id.to_string())
        .bind(SIRAK_CONFIG.org_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let deal_id: Option<String> = if let Some(ref cid) = client_id {
            sqlx::query_scalar(
                "SELECT id FROM crm_deals WHERE client_id = ?
                 AND stage NOT IN ('won','lost') ORDER BY created_at DESC LIMIT 1",
            )
            .bind(cid)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        // Spawn Phase II in background
        let pool2 = pool.clone();
        let company_id_str = company_id.to_string();
        let client_id_clone = client_id.clone();
        let deal_id_clone = deal_id.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::routes::intake::report::run_phase2_from_company_intel(
                pool2,
                &company_id_str,
                client_id_clone.as_deref(),
                deal_id_clone.as_deref(),
                "auto-pipeline",
            )
            .await
            {
                tracing::error!(
                    "[Auto-pipeline] Phase II failed for {}: {}",
                    company_id_str,
                    e
                );
            }
        });
    } else {
        // Standard flow: create human review task
        auto_create_intel_review_tasks(pool, company_id, company_name).await;
    }

    let _ = project_id; // reserved for future project-scoped task routing
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
         WHERE id = ?",
    )
    .bind(summary)
    .bind(confidence)
    .bind(company_id.to_string()) // companies.id is TEXT
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
            if let Err(e) = sqlx::query(
                "UPDATE crm_contacts SET intelligence_status = 'failed', \
                 intelligence_summary = 'LLM returned tool calls instead of text', \
                 updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
            )
            .bind(contact_id.to_string())
            .execute(pool)
            .await
            {
                tracing::warn!(
                    "[Scout] Failed to write failure status for contact {}: {}",
                    contact_id,
                    e
                );
            }
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
        if let Err(e) = sqlx::query(
            "UPDATE crm_contacts SET intelligence_status = 'idle', \
             updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
        )
        .bind(contact_id.to_string())
        .execute(pool)
        .await
        {
            tracing::warn!(
                "[Scout] Failed to reset contact {} status to idle: {}",
                contact_id,
                e
            );
        }
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
    Extension(access_context): Extension<AccessContext>,
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
        organization_id: Option<String>,
        full_name: Option<String>,
        email: Option<String>,
        company_name: Option<String>,
        job_title: Option<String>,
    }
    let contact: ContactRow = sqlx::query_as(
        "SELECT organization_id, full_name, email, company_name, job_title FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    // Verify org membership
    if let Some(ref org_id) = contact.organization_id {
        access_context.require_org_membership(pool, org_id).await?;
    }

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
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let contact_id =
        DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let pool = &d.db().pool;

    #[derive(sqlx::FromRow)]
    struct Row {
        organization_id: Option<String>,
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: f64,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT organization_id, intelligence_status, intelligence_summary, intelligence_confidence, \
         intelligence_agent, intelligence_last_run_at FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    // Verify org membership
    if let Some(ref org_id) = row.organization_id {
        access_context.require_org_membership(pool, org_id).await?;
    }

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
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ContactResearchPass>>>, ApiError> {
    let contact_id =
        DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let pool = &d.db().pool;
    #[derive(sqlx::FromRow)]
    struct OrgRow {
        organization_id: Option<String>,
    }
    let org_row: Option<OrgRow> =
        sqlx::query_as("SELECT organization_id FROM crm_contacts WHERE id = ?")
            .bind(&contact_id)
            .fetch_optional(pool)
            .await?;
    if let Some(OrgRow {
        organization_id: Some(ref org_id),
    }) = org_row
    {
        access_context.require_org_membership(pool, org_id).await?;
    }
    let passes = ContactResearchPass::list_for_contact(pool, &contact_id).await?;
    Ok(Json(ApiResponse::success(passes)))
}

/// POST /api/crm/contacts/:id/research-passes/next
/// Triggers the next logical research pass, building on all prior passes.
pub async fn trigger_next_research_pass(
    Extension(access_context): Extension<AccessContext>,
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
        organization_id: Option<String>,
        full_name: Option<String>,
        company_name: Option<String>,
        intelligence_summary: Option<String>,
    }
    let contact: ContactRow = sqlx::query_as(
        "SELECT organization_id, full_name, company_name, intelligence_summary FROM crm_contacts WHERE id = ?",
    )
    .bind(contact_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Contact not found".into()))?;

    // Verify org membership
    if let Some(ref org_id) = contact.organization_id {
        access_context.require_org_membership(pool, org_id).await?;
    }

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
        let _pass_id_err = pass_id.clone();
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
            if let Err(db_err) = sqlx::query(
                "UPDATE contact_research_passes SET status = 'failed', error = ?, completed_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
            )
            .bind(e.to_string())
            .bind(pass_id.to_string())
            .execute(&pool_for_err)
            .await
            {
                tracing::warn!("Failed to write error status for pass {}: {}", pass_id, db_err);
            }
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "pass_id": pass_id.to_string(),
        "pass_number": pass_number,
        "focus": focus_for_resp,
        "status": "queued"
    }))))
}

/// GET /api/crm/contacts/:id/reports
pub async fn list_person_reports(
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<BusinessReport>>>, ApiError> {
    DbUuid::parse(&contact_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let pool = &d.db().pool;
    #[derive(sqlx::FromRow)]
    struct OrgRow {
        organization_id: Option<String>,
    }
    let org_row: Option<OrgRow> =
        sqlx::query_as("SELECT organization_id FROM crm_contacts WHERE id = ?")
            .bind(&contact_id)
            .fetch_optional(pool)
            .await?;
    if let Some(OrgRow {
        organization_id: Some(ref org_id),
    }) = org_row
    {
        access_context.require_org_membership(pool, org_id).await?;
    }
    let reports = BusinessReport::list_by_contact(pool, &contact_id).await?;
    Ok(Json(ApiResponse::success(reports)))
}

fn auto_focus(pass_number: i64) -> String {
    match pass_number {
        1 => "identity_social",
        2 => "business_profile",
        3 => "media_content",
        4 => "audience_market",
        _ => "pcg_opportunity",
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
    _project_id: Option<Uuid>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    // Mark running
    sqlx::query("UPDATE contact_research_passes SET status = 'running' WHERE CAST(id AS TEXT) = ?")
        .bind(pass_id.to_string())
        .execute(&pool)
        .await?;

    let focus_instructions = match focus {
        "identity_social" => format!(
            "Research who {} ({}) is as a person and their full social media presence. Find: \
             1) Full confirmed name, location (city/state), professional headline. \
             2) Best available photo URL (LinkedIn headshot, Instagram profile pic, or website). \
             3) ALL social media handles with platform, handle, full profile URL, follower count, \
                following count, bio text, verified status, engagement rate if discoverable, \
                post count, top content themes (3-5 topics they post about most). \
             4) Website(s), Linktree, booking pages, any other online presence URLs. \
             5) Short personal brand statement or tagline if they use one.",
            name, company
        ),
        "business_profile" => format!(
            "Research all of {}'s business affiliations and the financials behind their primary venture(s). Find: \
             1) Current and past business roles (company name, their role/title, start year, end year, \
                whether current, company website URL, company type/category, brief description). \
             2) Revenue estimates for their primary business with the basis for the estimate. \
             3) Pricing model — specific product/service tiers with prices if publicly visible. \
             4) Platform stack — what tools, apps, or platforms they use to run their business. \
             5) Estimated active customers/members/clients.",
            name
        ),
        "media_content" => format!(
            "Research {}'s public media presence and content strategy. Find: \
             1) Podcast appearances (as guest or host) — show name, episode title, date, URL. \
             2) Press mentions and news articles — publication, headline, date, URL. \
             3) YouTube features or speaking events — event/video name, date, URL. \
             4) Content strategy: primary platforms, posting frequency per platform, \
                content types (reels/stories/long-form/etc), brand voice description, \
                best performing content type, hashtag strategy if visible. \
             5) Any notable collaborations, brand deals, or partnerships.",
            name
        ),
        "audience_market" => format!(
            "Research the market position and audience of {} / {}. Find: \
             1) Target audience demographics (age range, gender breakdown if known, interests, location). \
             2) Audience engagement signals (comment quality, community vibe, testimonials). \
             3) Top 4-6 direct competitors — for each: name, website, how they differ, threat level (high/medium/low). \
             4) Market category, industry standing (niche vs mainstream), geographic focus. \
             5) Unique differentiators that separate {} from competitors.",
            name, company, name
        ),
        "pcg_opportunity" => format!(
            "Perform a PCG agency opportunity assessment for {} ({}). PCG is a full-service creative agency \
             offering: Brand Identity, VSL/Video Production, Landing Pages, Sales Funnels, Social Strategy, \
             App/Content Creation, PR/Media. Find: \
             1) Specific pain points that map to PCG services (what is visually inconsistent, \
                what is missing from their funnel, where their online presence is weak). \
             2) Budget signals — what have they already invested in, what pricing tier do they sell at, \
                estimated monthly/annual revenue. \
             3) Decision-making style — are they DIY, do they hire help, do they act fast or deliberate. \
             4) Communication style and preferences based on their content and public interactions. \
             5) Overall PCG opportunity score 0-10 with rationale. \
             6) Recommended PCG engagement strategy — which service to lead with, what angle to use, \
                what NOT to say to this person.",
            name, company
        ),
        // Legacy focuses (backward compat)
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

    let system =
        "You are an expert business intelligence researcher. Conduct thorough web research \
        and return structured findings. Always respond with valid JSON only.";

    let prompt = format!(
        "RESEARCH TASK — Pass #{pass_number} | Focus: {focus}\n\n\
        Subject: {name} ({company})\n\
        {existing_section}{prior_section}\n\n\
        FOCUS FOR THIS PASS:\n{focus_instructions}\n\n\
        Return ONLY valid JSON with this structure (include all fields, use null for unknown):\n\
        {{\n\
          \"summary\": \"3-5 sentence summary of what was discovered in this pass\",\n\
          \"key_findings\": [\n\
            {{\"finding\": \"specific fact discovered\", \"confidence\": \"high|medium|low\", \"source\": \"URL or platform where found\"}}\n\
          ],\n\
          \"search_queries\": [\"actual queries used to find this\"],\n\
          \"updated_intelligence\": \"full cumulative intelligence summary incorporating all passes so far\",\n\
          \"confidence_score\": 0.0,\n\
          \"photo_url\": \"best available photo URL or null\",\n\
          \"location\": \"City, State/Country or null\",\n\
          \"social_profiles\": [\n\
            {{\n\
              \"platform\": \"instagram|tiktok|linkedin|twitter|youtube|facebook|website|linktree\",\n\
              \"handle\": \"handle without @ or null\",\n\
              \"url\": \"full profile URL\",\n\
              \"follower_count\": 0,\n\
              \"following_count\": 0,\n\
              \"bio\": \"profile bio text or null\",\n\
              \"verified\": false,\n\
              \"engagement_rate\": 0.0,\n\
              \"post_count\": 0,\n\
              \"content_themes\": [\"theme1\", \"theme2\"]\n\
            }}\n\
          ],\n\
          \"business_affiliations\": [\n\
            {{\n\
              \"company_name\": \"name\",\n\
              \"role\": \"Founder|CEO|etc\",\n\
              \"title\": \"exact job title\",\n\
              \"start_date\": \"2021 or null\",\n\
              \"end_date\": null,\n\
              \"current\": true,\n\
              \"description\": \"what the company does\",\n\
              \"company_url\": \"https://...\",\n\
              \"company_type\": \"category\"\n\
            }}\n\
          ],\n\
          \"media_appearances\": [\n\
            {{\"type\": \"podcast|press|video|speaking\", \"title\": \"title\", \"publication\": \"name\", \"date\": \"YYYY-MM\", \"url\": \"URL or null\", \"summary\": \"one sentence\"}}\n\
          ],\n\
          \"content_strategy\": {{\n\
            \"primary_platforms\": [\"instagram\"],\n\
            \"posting_frequency\": \"description or null\",\n\
            \"content_types\": [\"reels\", \"stories\"],\n\
            \"brand_voice\": \"description or null\",\n\
            \"best_performing\": \"description or null\"\n\
          }},\n\
          \"pcg_opportunity\": {{\n\
            \"score\": 0.0,\n\
            \"recommended_tier\": \"Tier 1|Tier 2|Tier 3 or null\",\n\
            \"recommended_services\": [\"Brand Kit\"],\n\
            \"pain_points\": [\"specific gap mapped to PCG service\"],\n\
            \"budget_signals\": \"evidence-based estimate or null\",\n\
            \"engagement_style\": \"communication preference description or null\",\n\
            \"best_approach\": \"recommended opening angle for PCG pitch or null\"\n\
          }},\n\
          \"background\": \"professional background narrative or null\",\n\
          \"positioning\": \"market positioning or null\",\n\
          \"specialization\": \"area of expertise or null\",\n\
          \"expertise\": [\"skill1\", \"skill2\"],\n\
          \"target_clients\": [\"who they serve\"],\n\
          \"competitors\": [\"competitor name — how they differ\"],\n\
          \"strategy\": \"strategic direction or null\",\n\
          \"communication_style\": \"how they communicate or null\",\n\
          \"opportunities\": [\"opportunity for PCG\"],\n\
          \"risks\": [\"risk or objection\"],\n\
          \"network\": \"notable connections or null\"\n\
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

        // Extract photo_url and location from parsed JSON
        let photo_url = parsed["photo_url"]
            .as_str()
            .filter(|s| !s.is_empty() && *s != "null");
        let _location = parsed["location"]
            .as_str()
            .filter(|s| !s.is_empty() && *s != "null");

        sqlx::query(
            "UPDATE crm_contacts SET
                intelligence_summary = ?,
                intelligence_raw = ?,
                intelligence_status = 'done',
                intelligence_last_run_at = datetime('now','subsec'),
                intelligence_confidence = ?,
                intelligence_agent = 'claude',
                research_pass_count = ?,
                research_depth = ?,
                avatar_url = COALESCE(NULLIF(?, ''), avatar_url),
                updated_at = datetime('now','subsec')
             WHERE CAST(id AS TEXT) = ?",
        )
        .bind(&updated_intel)
        .bind(json_str)
        .bind(new_confidence)
        .bind(pass_number)
        .bind(depth)
        .bind(photo_url.unwrap_or(""))
        .bind(contact_id)
        .execute(&pool)
        .await?;

        // Update location if found (stored in company_name fallback via notes or a dedicated col)
        // For now store in intelligence_raw which we already wrote above
    }

    // Also register in the org-level knowledge graph (owner_type='organization')
    // Find org from contact's associations
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

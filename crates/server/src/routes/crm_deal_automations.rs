//! CRM Deal Automations
//!
//! Handles AI-powered deal automations: research triggers, proposal generation (Cash),
//! deck generation (Lux), deep research (Astra), invoice handling, won automation,
//! and transcript management.

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use db::{
    db_uuid::DbUuid,
    models::{
        crm_deal::CrmDeal,
        project_knowledge_source::{
            KnowledgeOwnerScope, KnowledgeSourceType, ProjectKnowledgeSource,
        },
    },
};
use deployment::Deployment;
use utils::response::ApiResponse;

use super::crm_deals::require_deal_org_access;
use crate::{
    error::ApiError, helpers::uuid_params::parse_db_uuid_param,
    middleware::access_control::AccessContext, DeploymentImpl,
};

// ── Scout: Who-Is Research ────────────────────────────────────────────────────

/// Auto-trigger Scout research for a deal's contact and company.
/// Works directly with crm_contacts — no person bridge needed.
pub async fn trigger_who_is_research(
    pool: &sqlx::SqlitePool,
    deal_id: DbUuid,
    contact_id: Option<DbUuid>,
) {
    let Some(contact_id) = contact_id else {
        tracing::warn!(
            "[Scout] Deal {} has no crm_contact_id — cannot trigger research",
            deal_id
        );
        return;
    };

    // Read contact directly from crm_contacts (canonical source)
    #[derive(sqlx::FromRow)]
    struct ContactRow {
        full_name: Option<String>,
        email: Option<String>,
        company_name: Option<String>,
        job_title: Option<String>,
        intelligence_status: Option<String>,
    }

    let contact = match sqlx::query_as::<_, ContactRow>(
        "SELECT full_name, email, company_name, job_title, intelligence_status \
         FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            tracing::error!(
                "[Scout] Contact {} not found in crm_contacts (deal {})",
                contact_id,
                deal_id
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                "[Scout] Failed to query crm_contacts for {} (deal {}): {}",
                contact_id,
                deal_id,
                e
            );
            return;
        }
    };

    let status = contact.intelligence_status.as_deref().unwrap_or("idle");

    // Only trigger if not already running/queued
    if status != "idle" && !status.is_empty() {
        tracing::info!(
            "[Scout] Contact {} already has intelligence_status='{}', skipping",
            contact_id,
            status
        );
        return;
    }

    let contact_name = contact
        .full_name
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    tracing::info!(
        "[Scout] Auto-triggering research for contact {} '{}' (deal {})",
        contact_id,
        contact_name,
        deal_id
    );

    // Queue contact research
    if let Err(e) = sqlx::query(
        "UPDATE crm_contacts SET intelligence_status = 'queued', \
         intelligence_agent = 'scout', \
         updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(&contact_id)
    .execute(pool)
    .await
    {
        tracing::error!(
            "[Scout] Failed to queue contact {} for research: {}",
            contact_id,
            e
        );
        return;
    }

    // Spawn contact research
    let contact_uuid = contact_id.to_uuid();
    let pool_contact = pool.clone();
    let name = contact_name.clone();
    let email = contact.email.clone().unwrap_or_default();
    let company = contact.company_name.clone().unwrap_or_default();
    let title = contact.job_title.clone().unwrap_or_default();
    tokio::spawn(async move {
        if let Err(e) = crate::routes::intelligence::run_contact_research_direct(
            &pool_contact,
            contact_uuid,
            &name,
            &email,
            &company,
            &title,
            None,
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

    // Create Phase 1 visibility tasks + trigger company research
    let project_id: Option<DbUuid> =
        sqlx::query_scalar("SELECT project_id FROM crm_deals WHERE id = ?")
            .bind(&deal_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    let has_phase1_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? \
         AND title LIKE 'Phase 1 Research:%' AND deleted_at IS NULL",
    )
    .bind(&deal_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_phase1_tasks == 0 {
        // Build task list: always contact, optionally company
        let mut tasks: Vec<(String, &str)> = vec![(
            format!("Phase 1 Research: {} (Contact)", contact_name),
            "Scout gathering intelligence profile for this contact",
        )];
        if let Some(cn) = contact.company_name.as_deref().filter(|n| !n.is_empty()) {
            tasks.push((
                format!("Phase 1 Research: {} (Company)", cn),
                "Scout gathering company intelligence and building company wiki",
            ));
        }

        for (title, description) in &tasks {
            if let Err(e) =
                create_research_task(pool, &deal_id, project_id.as_ref(), title, description).await
            {
                tracing::error!(
                    "[Scout] Failed to create task '{}' for deal {}: {}",
                    title,
                    deal_id,
                    e
                );
            }
        }
    }

    // Register deal in the Knowledge Graph so agents have deal context
    register_deal_in_kg(pool, &deal_id).await;

    // Trigger company research in parallel (auto-create company if missing)
    if let Some(company_name) = contact.company_name.as_deref().filter(|n| !n.is_empty()) {
        trigger_company_research_if_idle(pool, company_name, &contact_id).await;
    }
}

/// Write the CRM deal into the KG as a context_injection source so all agents on this
/// project automatically receive deal metadata (title, stage, contact, value) in their prompts.
/// Public wrapper — called from crm_deals.rs on create/update
pub async fn register_deal_in_kg_pub(pool: &sqlx::SqlitePool, deal_id: &DbUuid) {
    register_deal_in_kg(pool, deal_id).await;
}

async fn register_deal_in_kg(pool: &sqlx::SqlitePool, deal_id: &DbUuid) {
    #[derive(sqlx::FromRow)]
    struct DealRow {
        name: String,
        stage: String,
        contact_name: Option<String>,
        amount: Option<f64>,
        probability: Option<i64>,
        expected_close_date: Option<String>,
        description: Option<String>,
        org_id: Option<DbUuid>,
        project_id: Option<DbUuid>,
    }

    let row = sqlx::query_as::<_, DealRow>(
        "SELECT
           cd.name,
           cd.stage,
           cc.full_name AS contact_name,
           cd.amount,
           cd.probability,
           cd.expected_close_date,
           cd.description,
           cp.organization_id AS org_id,
           cd.project_id
         FROM crm_deals cd
         LEFT JOIN crm_contacts cc ON cc.id = cd.crm_contact_id
         LEFT JOIN crm_pipelines cp ON cp.id = cd.crm_pipeline_id
         WHERE cd.id = ?",
    )
    .bind(deal_id)
    .fetch_optional(pool)
    .await;

    let Ok(Some(deal)) = row else { return };

    let title = deal.name.as_str();
    let stage = deal.stage.as_str();
    let contact = deal.contact_name.as_deref().unwrap_or("Unknown Contact");

    let summary = {
        let mut parts = vec![format!("Stage: {}", stage), format!("Contact: {}", contact)];
        if let Some(v) = deal.amount {
            parts.push(format!("Value: ${:.0}", v));
        }
        if let Some(p) = deal.probability {
            parts.push(format!("Probability: {}%", p));
        }
        if let Some(d) = &deal.expected_close_date {
            parts.push(format!("Expected Close: {}", d));
        }
        if let Some(d) = &deal.description {
            parts.push(format!("Notes: {}", &d[..d.len().min(200)]));
        }
        parts.join(" | ")
    };

    let coverage = deal.probability.map(|p| p as f64 / 100.0).unwrap_or(0.5);
    let source_id = format!("deal_{}", deal_id);
    let source_title = format!("Deal: {}", title);

    // Project scope
    if let Some(ref pid) = deal.project_id {
        let pid_uuid = pid.to_uuid();
        let _ = ProjectKnowledgeSource::upsert_scoped(
            pool,
            &KnowledgeOwnerScope::Project,
            &pid_uuid.to_string(),
            Some(pid_uuid),
            &KnowledgeSourceType::ContextInjection,
            &source_id,
            &source_title,
            Some(&summary),
            coverage,
        )
        .await;
    }

    // Deal scope (deal's own KG — uses partial-index conflict target for null project_id)
    let _ = ProjectKnowledgeSource::upsert_owner_scoped(
        pool,
        &KnowledgeOwnerScope::Deal,
        &deal_id.to_string(),
        &KnowledgeSourceType::ContextInjection,
        &source_id,
        &source_title,
        Some(&summary),
        coverage,
    )
    .await;

    // Org scope
    if let Some(org) = deal.org_id {
        let _ = ProjectKnowledgeSource::upsert_owner_scoped(
            pool,
            &KnowledgeOwnerScope::Organization,
            &org.to_string(),
            &KnowledgeSourceType::ContextInjection,
            &source_id,
            &source_title,
            Some(&summary),
            coverage,
        )
        .await;
    }

    tracing::info!(
        "[KG] Deal '{}' registered in knowledge graph (stage: {})",
        title,
        stage
    );
}

/// Insert a Phase 1 research visibility task for a deal.
async fn create_research_task(
    pool: &sqlx::SqlitePool,
    deal_id: &DbUuid,
    project_id: Option<&DbUuid>,
    title: &str,
    description: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, \
         created_at, updated_at) \
         VALUES (?, ?, ?, 'inprogress', ?, ?, datetime('now','subsec'), datetime('now','subsec'))",
    )
    .bind(DbUuid::new())
    .bind(title)
    .bind(description)
    .bind(deal_id)
    .bind(project_id)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Trigger Scout company research. Auto-creates the company if it doesn't exist.
async fn trigger_company_research_if_idle(
    pool: &sqlx::SqlitePool,
    company_name: &str,
    contact_id: &DbUuid,
) {
    #[derive(sqlx::FromRow)]
    struct CompanyRow {
        id: DbUuid,
        intelligence_status: Option<String>,
    }
    let company = match sqlx::query_as::<_, CompanyRow>(
        "SELECT id, intelligence_status FROM companies WHERE name = ? COLLATE NOCASE LIMIT 1",
    )
    .bind(company_name)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            // Auto-create company so research has a target
            let new_id = DbUuid::new();
            tracing::info!(
                "[Scout] Auto-creating company '{}' (id: {}) for contact {}",
                company_name,
                new_id,
                contact_id
            );
            // Look up org_id from the contact
            let org_id: Option<String> =
                sqlx::query_scalar("SELECT organization_id FROM crm_contacts WHERE id = ?")
                    .bind(contact_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

            let Some(org_id) = org_id else {
                tracing::error!(
                    "[Scout] Cannot auto-create company — contact {} has no organization_id",
                    contact_id
                );
                return;
            };

            if let Err(e) = sqlx::query(
                "INSERT INTO companies (id, name, organization_id, intelligence_status, \
                 created_at, updated_at) \
                 VALUES (?, ?, ?, 'idle', datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(&new_id)
            .bind(company_name)
            .bind(&org_id)
            .execute(pool)
            .await
            {
                tracing::error!(
                    "[Scout] Failed to auto-create company '{}': {}",
                    company_name,
                    e
                );
                return;
            }

            // Link contact to company
            let _ = sqlx::query(
                "UPDATE crm_contacts SET company_id = ? WHERE id = ? AND company_id IS NULL",
            )
            .bind(&new_id)
            .bind(contact_id)
            .execute(pool)
            .await;

            CompanyRow {
                id: new_id,
                intelligence_status: Some("idle".to_string()),
            }
        }
        Err(e) => {
            tracing::error!(
                "[Scout] Failed to look up company '{}': {}",
                company_name,
                e
            );
            return;
        }
    };

    let status = company.intelligence_status.as_deref().unwrap_or("idle");
    if status != "idle" && !status.is_empty() {
        tracing::info!(
            "[Scout] Company '{}' already has intelligence_status='{}', skipping",
            company_name,
            status
        );
        return;
    }

    tracing::info!(
        "[Scout] Auto-triggering company research for '{}' ({})",
        company_name,
        company.id
    );
    let company_uuid = company.id.to_uuid();
    crate::routes::intelligence::run_company_research_direct(
        pool,
        company_uuid,
        company_name,
        None,
    )
    .await;
}

// ── Phase 1 Business Report Generation ────────────────────────────────────────

/// Generate a Phase 1 business analysis report when a deal enters Business Analysis stage.
/// Compiles person + company intelligence into a structured business_report record.
pub async fn generate_phase1_business_report(
    pool: &sqlx::SqlitePool,
    deal_id: DbUuid,
    contact_id: Option<DbUuid>,
    _project_id: Option<DbUuid>,
) {
    // Check if a report already exists for this deal
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM business_reports WHERE crm_deal_id = ? AND deleted_at IS NULL",
    )
    .bind(&deal_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    if existing > 0 {
        tracing::info!(
            "Phase 1 report already exists for deal {}, skipping",
            deal_id
        );
        return;
    }

    let Some(contact_id) = contact_id else { return };

    let contact_company: Option<String> = {
        #[derive(sqlx::FromRow)]
        struct CRow {
            company_name: Option<String>,
        }
        sqlx::query_as::<_, CRow>("SELECT company_name FROM crm_contacts WHERE id = ? LIMIT 1")
            .bind(&contact_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.company_name)
    };
    let company_name = contact_company;

    #[derive(sqlx::FromRow)]
    struct PersonIntelRow {
        id: DbUuid,
        full_name: Option<String>,
        intelligence_summary: Option<String>,
        email: Option<String>,
        job_title: Option<String>,
    }
    let person_intel = sqlx::query_as::<_, PersonIntelRow>(
        "SELECT id, full_name, intelligence_summary, email, job_title FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    #[derive(sqlx::FromRow)]
    struct CompanyIntelRow {
        id: DbUuid,
        intelligence_summary: Option<String>,
        industry: Option<String>,
        description: Option<String>,
    }
    let company_intel = if let Some(ref cn) = company_name {
        sqlx::query_as::<_, CompanyIntelRow>(
            "SELECT id, intelligence_summary, industry, description FROM companies WHERE name = ? COLLATE NOCASE LIMIT 1",
        )
        .bind(cn)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let contact_name = person_intel
        .as_ref()
        .and_then(|p| p.full_name.clone())
        .unwrap_or_default();
    let company_display = company_name.as_deref().unwrap_or("Unknown Company");

    let person_summary = person_intel
        .as_ref()
        .and_then(|p| p.intelligence_summary.clone());
    let company_summary = company_intel
        .as_ref()
        .and_then(|c| c.intelligence_summary.clone());

    // Build raw intel context for Astra
    let mut raw_intel_parts: Vec<String> = Vec::new();
    if let Some(ref pi) = person_intel {
        let mut parts = Vec::new();
        if let Some(ref name) = pi.full_name {
            parts.push(format!("Name: {}", name));
        }
        if let Some(ref email) = pi.email {
            parts.push(format!("Email: {}", email));
        }
        if let Some(ref title) = pi.job_title {
            parts.push(format!("Title: {}", title));
        }
        if let Some(ref s) = person_summary {
            parts.push(s.clone());
        }
        if !parts.is_empty() {
            raw_intel_parts.push(format!("## Contact Intelligence\n{}", parts.join("\n")));
        }
    }
    if let Some(ref ci) = company_intel {
        let mut parts = Vec::new();
        if let Some(ref industry) = ci.industry {
            parts.push(format!("Industry: {}", industry));
        }
        if let Some(ref desc) = ci.description {
            parts.push(format!("Overview: {}", desc));
        }
        if let Some(ref s) = company_summary {
            parts.push(s.clone());
        }
        if !parts.is_empty() {
            raw_intel_parts.push(format!("## Company Intelligence\n{}", parts.join("\n")));
        }
    }

    // Call Astra to generate a proper Phase 1 analysis (if we have intel data)
    let astra_system = "You are Astra, a business intelligence analyst at Sirak Studios (a PowerClub Global company). Your role is to synthesise raw intelligence into actionable business analysis for the sales team.\n\nGiven the available intel on a prospect, produce a structured Phase 1 Business Analysis with these sections:\n\n## Executive Summary\nA concise 2-3 paragraph overview: who they are, where they are in their business, and the core opportunity for Sirak Studios.\n\n## Pain Points\n3-5 specific challenges this prospect likely faces (based on their situation).\n\n## Opportunities for Sirak Studios\n3-5 concrete ways we can create value — be specific about services that would fit.\n\n## Recommended Services\nRanked list of recommended service packages with brief justification and estimated value tier.\n\n## Strategic Notes\nKey context the account manager needs: relationship signals, timing considerations, likely objections, competitive threats.\n\nBe analytical, specific, and direct. No fluff. The AM should be able to walk into a discovery call fully prepared.";

    let (executive_summary, report_status) = if raw_intel_parts.is_empty() {
        (None, "draft")
    } else {
        let raw_intel = raw_intel_parts.join("\n\n---\n\n");
        let user_msg = format!(
            "Generate a Phase 1 business analysis for this prospect.\n\nDeal: {}\n\n{}",
            contact_name, raw_intel
        );
        match call_llm(astra_system, &user_msg).await {
            Ok(analysis) => {
                tracing::info!("[generate_phase1_business_report] Astra generated Phase 1 analysis for deal {}", deal_id);
                (Some(analysis), "ready")
            }
            Err(e) => {
                tracing::warn!(
                    "[generate_phase1_business_report] Astra call failed, storing raw intel: {}",
                    e
                );
                // Fallback: store raw intel concatenation
                let fallback = raw_intel_parts.join("\n\n");
                (Some(fallback), "draft")
            }
        }
    };

    let individual_profiles_json = person_intel
        .as_ref()
        .map(|pi| {
            let profile_parts: Vec<String> = [
                pi.full_name.as_deref().map(|s| format!("Name: {}", s)),
                pi.email.as_deref().map(|s| format!("Email: {}", s)),
                pi.job_title.as_deref().map(|s| format!("Title: {}", s)),
                pi.intelligence_summary.as_deref().map(|s| s.to_string()),
            ]
            .into_iter()
            .flatten()
            .collect();
            serde_json::json!([{"name": contact_name, "profile": profile_parts.join("\n")}])
                .to_string()
        })
        .unwrap_or_else(|| "[]".to_string());

    let company_overview = company_intel.as_ref().map(|c| {
        let mut parts = Vec::new();
        if let Some(ref industry) = c.industry {
            parts.push(format!("**Industry:** {}", industry));
        }
        if let Some(ref desc) = c.description {
            parts.push(format!("**Overview:** {}", desc));
        }
        if let Some(ref summary) = c.intelligence_summary {
            parts.push(summary.clone());
        }
        parts.join("\n")
    });

    let title = format!("Business Analytics: {} — {}", company_display, contact_name);
    // BLOB-column binding: business_reports.id/person_id/company_id/crm_deal_id are BLOB
    let person_uuid = person_intel.as_ref().map(|p| p.id.to_uuid());
    let company_uuid = company_intel.as_ref().map(|c| c.id.to_uuid());
    let deal_uuid = Some(deal_id.to_uuid());

    let report_id = DbUuid::new().to_uuid();

    let res = sqlx::query(
        r#"INSERT INTO business_reports
           (id, person_id, company_id, crm_deal_id, report_type, title, status,
            executive_summary, company_overview, individual_profiles,
            pain_points, opportunities, recommended_services, next_steps,
            competitor_analysis, intake_item_ids, call_log_ids)
           VALUES (?, ?, ?, ?, 'phase1_analysis', ?, ?, ?, ?, ?, '[]', '[]', '[]', '[]', '[]', '[]', '[]')"#,
    )
    .bind(report_id)
    .bind(person_uuid)
    .bind(company_uuid)
    .bind(deal_uuid)
    .bind(&title)
    .bind(report_status)
    .bind(&executive_summary)
    .bind(&company_overview)
    .bind(&individual_profiles_json)
    .execute(pool)
    .await;

    match res {
        Ok(_) => tracing::info!(
            "Phase 1 business report {} created for deal {}",
            report_id,
            deal_id
        ),
        Err(e) => tracing::error!(
            "Failed to create Phase 1 report for deal {}: {}",
            deal_id,
            e
        ),
    }

    // Mark Phase 1 research tasks as done
    if let Err(e) = sqlx::query(
        "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec') WHERE crm_deal_id = ? AND title LIKE 'Phase 1 Research:%' AND status = 'inprogress' AND deleted_at IS NULL",
    )
    .bind(&deal_id)
    .execute(pool)
    .await
    {
        tracing::error!("[generate_phase1_business_report] Failed to mark Phase 1 tasks as done: {}", e);
    }
}

// ── LLM helper: OpenAI-first with Anthropic fallback ─────────────────────────

/// Calls an LLM with a system prompt and user message.
/// Tries OpenAI (gpt-4o) first, falls back to Anthropic (claude-opus-4-6).
async fn call_llm(system_prompt: &str, user_message: &str) -> Result<String, ApiError> {
    let http = reqwest::Client::new();

    // Try OpenAI first
    if let Ok(openai_key) = std::env::var("OPENAI_API_KEY") {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "max_tokens": 4096,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message}
            ]
        });
        match http
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", openai_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(val) = resp.json::<serde_json::Value>().await {
                    if let Some(text) = val["choices"][0]["message"]["content"].as_str() {
                        tracing::info!("[LLM] OpenAI gpt-4o response received");
                        return Ok(text.to_string());
                    }
                }
            }
            Ok(resp) => {
                tracing::warn!(
                    "[LLM] OpenAI returned {}, falling back to Anthropic",
                    resp.status()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "[LLM] OpenAI request failed: {}, falling back to Anthropic",
                    e
                );
            }
        }
    }

    // Fallback to Anthropic
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| {
            ApiError::BadRequest(
                "No LLM API key configured (tried OPENAI_API_KEY, ANTHROPIC_API_KEY)".into(),
            )
        })?;

    let body = serde_json::json!({
        "model": "claude-opus-4-6",
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_message}]
    });

    let resp = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::BadRequest(format!("LLM API error: {}", e)))?;

    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ApiError::BadRequest(format!("LLM response parse error: {}", e)))?;

    let text = val["content"]
        .as_array()
        .and_then(|a| a.iter().find(|c| c["type"] == "text"))
        .and_then(|c| c["text"].as_str())
        .unwrap_or("LLM generation failed")
        .to_string();

    tracing::info!("[LLM] Anthropic claude-opus-4-6 response received");
    Ok(text)
}

// ── F12: Astra Pass 2 — enhance business report with discovery context, then chain Cash ──

pub async fn trigger_deep_research_pass2(pool: &sqlx::SqlitePool, deal_id: DbUuid) {
    // 1. Fetch deal
    let deal = match CrmDeal::find_by_id(pool, &deal_id).await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Astra Pass 2: failed to fetch deal {}: {}", deal_id, e);
            return;
        }
    };

    // 2. Fetch existing business report
    #[derive(sqlx::FromRow)]
    struct ReportRow {
        id: String,
        executive_summary: Option<String>,
        #[allow(dead_code)]
        company_overview: Option<String>,
        #[allow(dead_code)]
        individual_profiles: Option<String>,
        research_depth: Option<i32>,
    }
    let report = sqlx::query_as::<_, ReportRow>(
        "SELECT id, executive_summary, company_overview, individual_profiles, COALESCE(research_depth, 1) as research_depth FROM business_reports WHERE crm_deal_id = ? AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&deal_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // 3. Fetch discovery transcripts
    #[derive(sqlx::FromRow)]
    struct TransRow {
        summary: Option<String>,
        transcript_text: Option<String>,
    }
    let transcripts: Vec<TransRow> = sqlx::query_as::<_, TransRow>(
        "SELECT summary, transcript_text FROM deal_transcripts WHERE deal_id = ? ORDER BY created_at ASC"
    )
    .bind(&deal_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 4. Fetch person + company intel
    let person_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct PI {
            intelligence_summary: Option<String>,
            full_name: Option<String>,
        }
        sqlx::query_as::<_, PI>(
            "SELECT intelligence_summary, full_name FROM crm_contacts WHERE id = ? LIMIT 1",
        )
        .bind(cid)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|p| {
            p.intelligence_summary
                .map(|s| format!("**{}**\n{}", p.full_name.unwrap_or_default(), s))
        })
    } else {
        None
    };

    let company_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct CI {
            intelligence_summary: Option<String>,
            name: Option<String>,
        }
        sqlx::query_as::<_, CI>(
            "SELECT c.intelligence_summary, c.name FROM companies c JOIN crm_contacts cc ON lower(cc.company_name) = lower(c.name) WHERE cc.id = ? LIMIT 1"
        ).bind(cid).fetch_optional(pool).await.ok().flatten()
        .and_then(|c| c.intelligence_summary.map(|s| format!("**{}**\n{}", c.name.unwrap_or_default(), s)))
    } else {
        None
    };

    // 5. Build Astra prompt
    let existing_report_text = report
        .as_ref()
        .and_then(|r| r.executive_summary.clone())
        .unwrap_or_else(|| "No existing report.".to_string());

    let discovery_text = if transcripts.is_empty() {
        "No discovery transcripts available.".to_string()
    } else {
        transcripts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let content = t
                    .summary
                    .as_deref()
                    .or(t.transcript_text.as_deref())
                    .unwrap_or("(empty)");
                format!("### Transcript {}\n{}", i + 1, content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let mut context_parts = vec![
        format!("## Existing Business Report\n{}", existing_report_text),
        format!("## Discovery Transcripts\n{}", discovery_text),
    ];
    if let Some(ref pi) = person_intel {
        context_parts.push(format!("## Person Intelligence\n{}", pi));
    }
    if let Some(ref ci) = company_intel {
        context_parts.push(format!("## Company Intelligence\n{}", ci));
    }
    if let Some(ref desc) = deal.description {
        context_parts.push(format!("## Deal Notes\n{}", desc));
    }

    let astra_system = "You are Astra, a senior business intelligence analyst at Sirak Studios (a PowerClub Global company). You're preparing the final briefing document that Cash (our proposal agent) will use to write the proposal — so every insight needs to be actionable and specific.\n\nYour job: synthesise all available intelligence (prior report + discovery transcripts + operator notes + person/company intel) into an enhanced business analysis.\n\nOutput these sections:\n\n## Executive Summary\n2-3 paragraphs: who the client is, their current situation, and the core opportunity. Updated with discovery learnings.\n\n## Client Pain Points\nSpecific, discovery-validated pain points. Quote or reference what was said in the discovery call where possible.\n\n## Strategic Opportunities\nConcrete service opportunities ranked by fit and value. Include estimated value ranges.\n\n## Recommended Services\nFinal ranked list of recommended deliverables with pricing tier guidance.\n\n## Competitive Context\nAny known alternatives, DIY solutions, or competitors the client mentioned.\n\n## Proposal Guidance for Cash\nKey angles, messaging hooks, and pricing considerations Cash should factor into the proposal. What will close this deal?\n\nBe direct, specific, and analytical. The AM should not need to fill in gaps.";
    let user_msg = format!(
        "Enhance this business report with the discovery context below. Deal: {}\n\n{}",
        deal.name,
        context_parts.join("\n\n---\n\n")
    );

    // 6. Call LLM
    match call_llm(astra_system, &user_msg).await {
        Ok(enhanced_report) => {
            // Update existing report or create one
            if let Some(ref r) = report {
                let new_depth = r.research_depth.unwrap_or(1) + 1;
                if let Err(e) = sqlx::query(
                    "UPDATE business_reports SET executive_summary = ?, research_depth = ?, status = 'enhanced', updated_at = datetime('now','subsec') WHERE id = ?"
                )
                .bind(&enhanced_report)
                .bind(new_depth)
                .bind(&r.id)
                .execute(pool)
                .await
                {
                    tracing::error!("[trigger_deep_research_pass2] Failed to update Astra report: {}", e);
                }
                tracing::info!(
                    "Astra Pass 2 enhanced report {} for deal {} (depth {})",
                    r.id,
                    deal_id,
                    new_depth
                );
            } else {
                // No existing report — create one (BLOB columns: use .to_uuid() for binding)
                let report_id = DbUuid::new().to_uuid();
                let deal_uuid = DbUuid::parse(deal_id.as_str()).ok().map(|d| d.to_uuid());
                if let Err(e) = sqlx::query(
                    r#"INSERT INTO business_reports
                       (id, crm_deal_id, report_type, title, status, executive_summary, research_depth,
                        company_overview, individual_profiles, pain_points, opportunities, recommended_services, next_steps,
                        competitor_analysis, intake_item_ids, call_log_ids)
                       VALUES (?, ?, 'phase2_analysis', ?, 'enhanced', ?, 2, '', '[]', '[]', '[]', '[]', '[]', '[]', '[]', '[]')"#
                )
                .bind(report_id)
                .bind(deal_uuid)
                .bind(format!("Phase 2 Business Analysis: {}", deal.name))
                .bind(&enhanced_report)
                .execute(pool)
                .await
                {
                    tracing::error!("[trigger_deep_research_pass2] Failed to create business report: {}", e);
                }
                tracing::info!(
                    "Astra Pass 2 created new report {} for deal {}",
                    report_id,
                    deal_id
                );
            }
        }
        Err(e) => {
            tracing::error!("Astra Pass 2 LLM call failed for deal {}: {}", deal_id, e);
        }
    }

    // 7. Chain Cash — generate proposal after Astra completes
    generate_proposal_background(pool, deal_id).await;
}

// ── Background helper: generate proposal (Cash) ─────────────────────────────

async fn generate_proposal_background(pool: &sqlx::SqlitePool, deal_id: DbUuid) {
    let result = generate_proposal_core(pool, &deal_id).await;
    match result {
        Ok(_) => tracing::info!("Cash auto-generated proposal for deal {}", deal_id),
        Err(e) => tracing::warn!("Cash auto-generation failed for deal {}: {}", deal_id, e),
    }
}

async fn generate_proposal_core(pool: &sqlx::SqlitePool, id: &DbUuid) -> Result<CrmDeal, ApiError> {
    let deal = CrmDeal::find_by_id(pool, id).await?;

    // Gather context: business report, person intel, company intel, transcripts
    let report_summary: Option<String> = {
        #[derive(sqlx::FromRow)]
        struct BizReport {
            executive_summary: Option<String>,
        }
        sqlx::query_as::<_, BizReport>("SELECT executive_summary FROM business_reports WHERE crm_deal_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(id).fetch_optional(pool).await.ok().flatten().and_then(|r| r.executive_summary)
    };
    let person_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct PersonIntel {
            intelligence_summary: Option<String>,
            full_name: Option<String>,
            company_name: Option<String>,
        }
        sqlx::query_as::<_, PersonIntel>("SELECT intelligence_summary, full_name, company_name FROM crm_contacts WHERE id = ? LIMIT 1")
            .bind(cid).fetch_optional(pool).await.ok().flatten()
            .map(|p| format!("**Contact:** {}\n**Company:** {}\n\n{}", p.full_name.unwrap_or_default(), p.company_name.unwrap_or_default(), p.intelligence_summary.unwrap_or_default()))
    } else {
        None
    };
    // Company intelligence from the companies table
    let company_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct CI {
            intelligence_summary: Option<String>,
            name: Option<String>,
        }
        sqlx::query_as::<_, CI>(
            "SELECT c.intelligence_summary, c.name FROM companies c JOIN crm_contacts cc ON lower(cc.company_name) = lower(c.name) WHERE cc.id = ? LIMIT 1"
        ).bind(cid).fetch_optional(pool).await.ok().flatten()
        .and_then(|c| c.intelligence_summary.map(|s| format!("**Company: {}**\n\n{}", c.name.unwrap_or_default(), s)))
    } else {
        None
    };
    let transcripts: Vec<String> = {
        #[derive(sqlx::FromRow)]
        struct TranscriptRow {
            summary: Option<String>,
        }
        sqlx::query_as::<_, TranscriptRow>(
            "SELECT summary FROM deal_transcripts WHERE deal_id = ? ORDER BY created_at ASC",
        )
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| t.summary)
        .collect()
    };

    let mut context_parts = Vec::new();
    if let Some(ref bi) = report_summary {
        context_parts.push(format!("## Business Intelligence Report\n{}", bi));
    }
    if let Some(ref ci) = company_intel {
        context_parts.push(format!("## Company Intelligence\n{}", ci));
    }
    if let Some(ref pi) = person_intel {
        context_parts.push(format!("## Contact Profile\n{}", pi));
    }
    if !transcripts.is_empty() {
        context_parts.push(format!(
            "## Discovery Transcript Summaries\n{}",
            transcripts.join("\n\n")
        ));
    }
    if let Some(ref desc) = deal.description {
        context_parts.push(format!("## Deal Notes\n{}", desc));
    }
    // F13: Do NOT feed deal.amount to Cash — Cash should determine pricing independently

    let context = if context_parts.is_empty() {
        format!(
            "Deal: {}\nClient company: {}",
            deal.name,
            deal.name.split('—').last().unwrap_or(&deal.name).trim()
        )
    } else {
        context_parts.join("\n\n---\n\n")
    };

    let cash_system = "You are Cash, a razor-sharp sales strategist and proposal architect at Sirak Studios (a PowerClub Global company), a premium creative production and brand strategy agency.\n\nYour proposals close deals. Every section earns its place. Be specific, bold, and value-driven — speak to the client's actual situation.\n\nPRICING TIERS (use these as your guide):\n- Tier 1 (Essentials): $3,500–6,500 — Logo, 1-page site, brand colours, social kit\n- Tier 2 (Strategic Brand): $8,500–18,000 — Full brand identity, landing page + VSL, ICP research, email automation, funnel strategy\n- Tier 3 (Growth Engine): $20,000–45,000 — Full rebrand, multi-page site, paid ad creative, content system, 90-day growth roadmap\n- Tier 4 (Enterprise Partnership): $50,000+ — White-glove multi-channel strategy, custom dev, retained advisory\n\nPer-deliverable pricing benchmarks:\n- Brand Kit / Identity System: $3,500–7,500\n- Landing Page + VSL Strategy: $4,000–8,000\n- ICP Research (3 profiles): $1,500–3,000\n- Email Sequence + Automation: $2,500–5,000\n- Funnel Mapping: $1,500–3,000\n- Lead Generation Strategy: $2,000–4,000\n- Investor / Pitch Deck: $5,000–12,000\n- Social Media Content System: $3,000–6,000\n- Video Production (per video): $2,000–8,000\n\nWrite proposals in clear, compelling markdown with these exact sections:\n## Executive Summary\n## Client Situation\n## Proposed Solution\n## Deliverables\n## Timeline\n## Investment\n## Next Steps\n\nIn the Deliverables section, list each deliverable with a brief description and its individual price.\nIn the Investment section, show the total, any available payment options, and what makes this transformative.\n\nIMPORTANT: At the very end of the proposal, include a JSON block with the deliverables list:\n\n```json\n[{\"title\": \"Deliverable Name\", \"description\": \"Brief description\", \"estimated_value\": 5000}]\n```\n\nThe estimated_value must be a number (no $ sign, no quotes). The sum of all estimated_values should equal the total in the Investment section.";
    let user_prompt = format!(
        "Write a tailored, high-converting proposal for the following client engagement.\n\n{}",
        context
    );

    let proposal_text = call_llm(cash_system, &user_prompt).await?;

    sqlx::query("UPDATE crm_deals SET proposal_text = ?, proposal_status = 'draft', updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(&proposal_text)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to save proposal: {}", e)))?;

    // F13: Parse ```json block for estimated_value fields and update deal.amount
    if let Some(start) = proposal_text.find("```json") {
        let after = &proposal_text[start + 7..];
        if let Some(end) = after.find("```") {
            let json_str = after[..end].trim();
            if let Ok(deliverables) = serde_json::from_str::<serde_json::Value>(json_str) {
                let items = deliverables
                    .as_array()
                    .cloned()
                    .or_else(|| {
                        deliverables
                            .get("deliverables")
                            .and_then(|d| d.as_array())
                            .cloned()
                    })
                    .unwrap_or_default();
                let total_value: f64 = items
                    .iter()
                    .filter_map(|item| item.get("estimated_value").and_then(|v| v.as_f64()))
                    .sum();
                if total_value > 0.0 {
                    if let Err(e) = sqlx::query(
                        "UPDATE crm_deals SET amount = ?, updated_at = datetime('now','subsec') WHERE id = ?"
                    )
                    .bind(total_value)
                    .bind(id)
                    .execute(pool)
                    .await
                    {
                        tracing::error!("[generate_proposal_core] Failed to update deal amount from estimated_values: {}", e);
                    }
                    tracing::info!(
                        "Cash set deal.amount to ${:.0} from deliverable estimated_values for deal {}",
                        total_value,
                        id
                    );
                }
            }
        }
    }

    let updated = CrmDeal::find_by_id(pool, id).await?;
    tracing::info!("Cash generated proposal for deal {}", id);
    Ok(updated)
}

// ── POST /crm/deals/:id/generate-proposal (Cash HTTP handler) ───────────────

pub async fn generate_proposal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let updated = generate_proposal_core(pool, &id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

// ── POST /crm/deals/:id/approve-proposal ────────────────────────────────────

pub async fn approve_proposal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;
    sqlx::query("UPDATE crm_deals SET proposal_status = 'approved', updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve: {}", e)))?;

    // F2: Parse deliverables from proposal ```json block and INSERT INTO deliverables
    let deal = CrmDeal::find_by_id(pool, &id).await?;
    let mut deliverable_count = 0i32;
    if let Some(ref proposal_text) = deal.proposal_text {
        if let Some(start) = proposal_text.find("```json") {
            let after = &proposal_text[start + 7..];
            if let Some(end) = after.find("```") {
                let json_str = after[..end].trim();
                if let Ok(deliverables_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    let items = deliverables_val
                        .as_array()
                        .cloned()
                        .or_else(|| {
                            deliverables_val
                                .get("deliverables")
                                .and_then(|d| d.as_array())
                                .cloned()
                        })
                        .unwrap_or_default();

                    for item in &items {
                        let title = item["title"]
                            .as_str()
                            .or_else(|| item["name"].as_str())
                            .unwrap_or("Deliverable");
                        let desc = item["description"].as_str().unwrap_or("");
                        let deliverable_id = DbUuid::new();
                        sqlx::query(
                            "INSERT INTO deliverables (id, crm_deal_id, project_id, title, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'pending', datetime('now','subsec'), datetime('now','subsec'))"
                        )
                        .bind(&deliverable_id)
                        .bind(&id)
                        .bind(&deal.project_id)
                        .bind(title)
                        .bind(desc)
                        .execute(pool)
                        .await
                        .map_err(|e| {
                            tracing::error!("[approve_proposal] Failed to create deliverable from proposal: {}", e);
                            ApiError::InternalError(format!("Failed to create deliverable from proposal: {}", e))
                        })?;
                        deliverable_count += 1;
                    }
                }
            }
        }
    }

    // Log activity
    if deliverable_count > 0 {
        if let Err(e) = sqlx::query(
            "INSERT INTO crm_activities (id, organization_id, crm_contact_id, crm_deal_id, activity_type, subject, activity_at) VALUES (?, ?, ?, ?, 'deliverables_created', ?, datetime('now','subsec'))"
        )
        .bind(DbUuid::new())
        .bind(&deal.organization_id)
        .bind(&deal.crm_contact_id)
        .bind(&id)
        .bind(format!("Proposal approved: {} deliverables created", deliverable_count))
        .execute(pool)
        .await
        {
            tracing::error!("[approve_proposal] Failed to log deliverables_created activity: {}", e);
        }
        tracing::info!(
            "Proposal approved for deal {}: {} deliverables created",
            id,
            deliverable_count
        );
    }

    let updated = CrmDeal::find_by_id(pool, &id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

// ── Background helper: generate deck (Lux) ──────────────────────────────────

pub async fn generate_deck_background(pool: &sqlx::SqlitePool, deal_id: DbUuid) {
    match generate_deck_core(pool, &deal_id).await {
        Ok(_) => tracing::info!("Lux auto-generated deck for deal {}", deal_id),
        Err(e) => tracing::warn!("Lux auto-generation failed for deal {}: {}", deal_id, e),
    }
}

// ── POST /crm/deals/:id/generate-deck (Lux) ─────────────────────────────────

async fn generate_deck_core(pool: &sqlx::SqlitePool, id: &DbUuid) -> Result<CrmDeal, ApiError> {
    let deal = CrmDeal::find_by_id(pool, id).await?;

    if deal.proposal_text.is_none() {
        return Err(ApiError::BadRequest(
            "Generate proposal first before generating deck".into(),
        ));
    }

    let brand_context: Option<String> = if let Some(ref org_id) = deal.organization_id {
        #[derive(sqlx::FromRow)]
        struct BrandRow {
            primary_color: Option<String>,
            brand_voice: Option<String>,
            tagline: Option<String>,
        }
        sqlx::query_as::<_, BrandRow>("SELECT primary_color, brand_voice, tagline FROM organization_brand_profiles WHERE organization_id = ? LIMIT 1")
            .bind(org_id.to_string()).fetch_optional(pool).await.ok().flatten()
            .map(|b| format!("Brand primary color: {}\nBrand voice: {}\nTagline: {}",
                b.primary_color.unwrap_or_default(), b.brand_voice.unwrap_or_default(), b.tagline.unwrap_or_default()))
    } else {
        None
    };

    let proposal = deal.proposal_text.as_deref().unwrap_or("");
    let lux_system = "You are Lux, a world-class presentation strategist at Sirak Studios. You transform approved proposals into compelling, slide-by-slide deck scripts that close deals.\n\nOutput a complete deck script in markdown. Separate each slide with ---.\n\nFor each slide use this structure:\n## [Slide Title]\n**Key Message:** One punchy sentence the audience must remember.\n**Visuals:** Concrete art direction (colours, imagery, layout).\n**Speaker Notes:** What the presenter says verbatim — confident, conversational, persuasive.\n\nSlide flow: Cover → About Us → Client Situation & Pain Points → The Opportunity → Our Solution → Deliverables (one per slide for key items) → Timeline → Investment → Why Sirak Studios → Social Proof → Next Steps → Close\n\nTone: premium, direct, human. No corporate fluff. The client should feel understood and excited.";
    let user_prompt = match brand_context {
        Some(ref bc) => format!(
            "Create a sales deck for this proposal.\n\nBrand context:\n{}\n\n---\n\nProposal:\n{}",
            bc, proposal
        ),
        None => format!("Create a sales deck for this proposal:\n\n{}", proposal),
    };

    let deck_script = call_llm(lux_system, &user_prompt).await?;

    let deck_id = DbUuid::new();
    let deck_url = format!("/api/crm/deals/{}/deck/{}", id, deck_id);

    if let Err(e) = sqlx::query(
        "INSERT OR IGNORE INTO project_knowledge_sources (id, owner_type, owner_id, source_type, source_id, source_title, source_summary, coverage_score, is_active, created_at, updated_at) VALUES (?, 'deal', ?, 'deck_script', ?, ?, ?, 0.8, 1, datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(DbUuid::new().to_string())
    .bind(id.to_string())
    .bind(deck_id.to_string())
    .bind(format!("Sales Deck: {}", deal.name))
    .bind(deck_script.chars().take(500).collect::<String>())
    .execute(pool).await
    {
        tracing::error!("[generate_deck_core] Failed to insert knowledge source: {}", e);
    }

    let deck_data = serde_json::json!({ "deck_id": deck_id.to_string(), "script": deck_script });
    sqlx::query("UPDATE crm_deals SET custom_fields = ?, deck_url = ?, updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(deck_data.to_string())
        .bind(&deck_url)
        .bind(id)
        .execute(pool).await
        .map_err(|e| {
            tracing::error!("[generate_deck_core] Failed to save deck URL: {}", e);
            ApiError::InternalError(format!("Failed to save deck URL: {}", e))
        })?;

    let updated = CrmDeal::find_by_id(pool, id).await?;
    tracing::info!("Lux generated deck for deal {}", id);
    Ok(updated)
}

// ── POST /crm/deals/:id/generate-deck (Lux HTTP handler) ────────────────────

pub async fn generate_deck(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let updated = generate_deck_core(pool, &id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

// ── POST /crm/deals/:id/send-invoice ────────────────────────────────────────

pub async fn send_deal_invoice(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    // Check if invoice already sent
    if deal.invoice_id.is_some() {
        return Err(ApiError::BadRequest(
            "Invoice already sent for this deal".into(),
        ));
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE invoice_type = 'ar'")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let invoice_number = format!("INV-{:05}", count + 1);
    let invoice_id = DbUuid::new().to_uuid();
    let amount_usd = deal.amount.unwrap_or(0.0);
    let amount_vibe = (amount_usd * 100.0) as i64;

    let _client_name = body["client_name"].as_str().unwrap_or(&deal.name);
    let notes = body["notes"].as_str().unwrap_or("Proposal invoice");
    let due_days = body["due_days"].as_i64().unwrap_or(14);

    sqlx::query(
        "INSERT INTO invoices (id, invoice_number, invoice_type, status, amount_vibe, amount_usd, due_date, notes, title) VALUES (?, ?, 'ar', 'sent', ?, ?, datetime('now', ?), ?, ?)"
    )
    .bind(invoice_id)
    .bind(&invoice_number)
    .bind(amount_vibe)
    .bind(amount_usd)
    .bind(format!("+{} days", due_days))
    .bind(notes)
    .bind(format!("Proposal: {}", deal.name))
    .execute(pool).await
    .map_err(|e| ApiError::BadRequest(format!("Failed to create invoice: {}", e)))?;

    // Link invoice to deal
    sqlx::query(
        "UPDATE crm_deals SET invoice_id = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(invoice_id.to_string())
    .bind(&id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to link invoice: {}", e)))?;

    tracing::info!(
        "Invoice {} sent for deal {} (${:.2})",
        invoice_number,
        id,
        amount_usd
    );
    Ok(Json(ApiResponse::success(serde_json::json!({
        "invoice_id": invoice_id.to_string(),
        "invoice_number": invoice_number,
        "amount_usd": amount_usd,
        "status": "sent"
    }))))
}

// ── POST /crm/deals/:id/mark-won ─────────────────────────────────────────────
/// Won automation chain: set won_at, move to Won stage, create client + project + tasks
/// Result of Won provisioning — shared between mark_deal_won endpoint and stage transition
#[derive(Debug, Clone, serde::Serialize)]
pub struct WonProvisionResult {
    pub client_id: String,
    pub client_name: String,
    pub project_id: String,
    pub project_name: String,
    pub tasks_created: i32,
    pub vibe_amount: Option<f64>,
}

/// Provision a Won deal: create client, project, tasks from deliverables, VIBE transaction, log activity.
/// Idempotent — checks if client/project already exist. Called by both mark_deal_won and stage transition.
pub async fn provision_won_deal(
    pool: &sqlx::SqlitePool,
    deal: &CrmDeal,
) -> Result<WonProvisionResult, String> {
    let deal_id = &deal.id;

    // Resolve contact info for client/project creation
    #[derive(sqlx::FromRow)]
    struct ContactInfo {
        full_name: Option<String>,
        company_name: Option<String>,
        email: Option<String>,
    }
    let contact_info = if let Some(ref cid) = deal.crm_contact_id {
        sqlx::query_as::<_, ContactInfo>(
            "SELECT full_name, company_name, email FROM crm_contacts WHERE id = ? LIMIT 1",
        )
        .bind(cid)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let company_name = contact_info
        .as_ref()
        .and_then(|c| c.company_name.clone())
        .unwrap_or_else(|| deal.name.clone());

    // ── Create or find Client record ─────────────────────────────────────────
    let client_id = {
        #[derive(sqlx::FromRow)]
        struct C {
            id: DbUuid,
        }
        let existing = if let Some(ref org_id) = deal.organization_id {
            sqlx::query_as::<_, C>(
                "SELECT id FROM clients WHERE organization_id = ? AND name = ? LIMIT 1",
            )
            .bind(org_id)
            .bind(&company_name)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        if let Some(c) = existing {
            c.id
        } else {
            let cid = DbUuid::new();
            let slug = company_name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");
            if let Err(e) = sqlx::query(
                "INSERT INTO clients (id, organization_id, name, slug, crm_contact_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
            )
            .bind(&cid)
            .bind(&deal.organization_id)
            .bind(&company_name)
            .bind(&slug)
            .bind(&deal.crm_contact_id)
            .execute(pool).await
            {
                return Err(format!("Failed to create client: {}", e));
            }
            cid
        }
    };

    // Promote existing client to client_since (first payment/won)
    let _ = sqlx::query(
        "UPDATE clients SET
         client_since = COALESCE(client_since, datetime('now','subsec')),
         prospect_at = COALESCE(prospect_at, datetime('now','subsec')),
         updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&client_id)
    .execute(pool)
    .await;

    // ── Create Project ───────────────────────────────────────────────────────
    // Check if project already linked (idempotent)
    if deal.project_id.is_some() {
        tracing::info!(
            "[provision_won_deal] Deal {} already has project, skipping creation",
            deal_id
        );
    }

    let project_id = if let Some(ref existing_project_id) = deal.project_id {
        existing_project_id.clone()
    } else {
        let pid = DbUuid::new();
        let project_name = format!("{} — {}", deal.name, company_name);
        let repo_path = format!("deals/{}", deal_id);
        if let Err(e) = sqlx::query(
            "INSERT INTO projects (id, name, git_repo_path, organization_id, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(&pid)
        .bind(&project_name)
        .bind(&repo_path)
        .bind(&deal.organization_id)
        .execute(pool).await
        {
            return Err(format!("Failed to create project: {}", e));
        }

        // Link project to deal
        if let Err(e) = sqlx::query(
            "UPDATE crm_deals SET project_id = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&pid)
        .bind(deal_id)
        .execute(pool)
        .await
        {
            tracing::error!("[provision_won_deal] Failed to link project {} to deal {}: {}", pid, deal_id, e);
        }
        pid
    };

    let project_name = format!("{} — {}", deal.name, company_name);

    // ── Move deliverables to the project ─────────────────────────────────────
    if let Err(e) = sqlx::query(
        "UPDATE deliverables SET project_id = ?, updated_at = datetime('now','subsec') WHERE crm_deal_id = ?"
    )
    .bind(&project_id)
    .bind(deal_id)
    .execute(pool)
    .await
    {
        tracing::error!("[provision_won_deal] Failed to move deliverables to project: {}", e);
    }

    // ── Create tasks from deliverables ───────────────────────────────────────
    #[derive(sqlx::FromRow)]
    struct DeliverableRow {
        #[allow(dead_code)]
        id: DbUuid,
        title: String,
        description: Option<String>,
    }
    let deliverables: Vec<DeliverableRow> = sqlx::query_as::<_, DeliverableRow>(
        "SELECT id, title, description FROM deliverables WHERE crm_deal_id = ? ORDER BY created_at ASC"
    )
    .bind(deal_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut task_count = deliverables.len() as i32;
    for deliv in &deliverables {
        let task_id = DbUuid::new();
        if let Err(e) = sqlx::query(
            "INSERT INTO tasks (id, project_id, crm_deal_id, title, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'todo', datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(&task_id).bind(&project_id).bind(deal_id)
        .bind(&deliv.title).bind(&deliv.description)
        .execute(pool).await
        {
            tracing::error!("[provision_won_deal] Failed to create task from deliverable: {}", e);
        }
    }

    // Fallback: if no deliverables exist, create a default setup task
    if task_count == 0 {
        let task_id = DbUuid::new();
        if let Err(e) = sqlx::query(
            "INSERT INTO tasks (id, project_id, crm_deal_id, title, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'todo', datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(&task_id).bind(&project_id).bind(deal_id)
        .bind("Project Setup & Kickoff")
        .bind(format!("Initial project setup for {}. Review proposal and create specific deliverables.", company_name))
        .execute(pool).await
        {
            tracing::error!("[provision_won_deal] Failed to create default setup task: {}", e);
        }
        task_count = 1;
    }

    // ── VIBE transaction for deal value ──────────────────────────────────────
    let vibe_amount = if let Some(amount) = deal.amount {
        let vibe = amount * 100.0; // 1 USD = 100 VIBE
        let tx_id = DbUuid::new();
        if let Err(e) = sqlx::query(
            "INSERT INTO vibe_transactions (id, amount, transaction_type, description, created_at) VALUES (?, ?, 'deal_won', ?, datetime('now','subsec'))"
        )
        .bind(&tx_id)
        .bind(vibe)
        .bind(format!("Deal won: {} — ${:.2}", deal.name, amount))
        .execute(pool)
        .await
        {
            tracing::error!("[provision_won_deal] Failed to create VIBE transaction: {}", e);
        }
        Some(amount)
    } else {
        None
    };

    // ── Person invitation stub ───────────────────────────────────────────────
    if let Some(ref ci) = contact_info {
        tracing::info!(
            "Won deal {}: person invitation pending for {} ({}) — wire invite system later",
            deal_id,
            ci.full_name.as_deref().unwrap_or("unknown"),
            ci.email.as_deref().unwrap_or("no-email")
        );
    }

    // ── Log Won activity ─────────────────────────────────────────────────────
    if let Err(e) = sqlx::query(
        "INSERT INTO crm_activities (id, organization_id, crm_contact_id, crm_deal_id, activity_type, subject, activity_at) VALUES (?, ?, ?, ?, 'deal_won', ?, datetime('now','subsec'))"
    )
    .bind(DbUuid::new())
    .bind(&deal.organization_id)
    .bind(&deal.crm_contact_id)
    .bind(deal_id)
    .bind(format!("Deal won: {} — {} tasks created", deal.name, task_count))
    .execute(pool).await
    {
        tracing::error!("[provision_won_deal] Failed to log Won activity: {}", e);
    }

    tracing::info!(
        "Deal {} provisioned as Won. Client '{}', Project '{}', {} tasks created",
        deal_id,
        company_name,
        project_name,
        task_count
    );

    Ok(WonProvisionResult {
        client_id: client_id.to_string(),
        client_name: company_name,
        project_id: project_id.to_string(),
        project_name,
        tasks_created: task_count,
        vibe_amount,
    })
}

pub async fn mark_deal_won(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    let win_reason = body["win_reason"].as_str().map(|s| s.to_string());

    // Find Won stage for this pipeline
    let won_stage: Option<DbUuid> = if let Some(ref pipeline_id) = deal.crm_pipeline_id {
        #[derive(sqlx::FromRow)]
        struct S {
            id: DbUuid,
        }
        sqlx::query_as::<_, S>("SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND (stage_type = 'won' OR name = 'Won') LIMIT 1")
            .bind(pipeline_id).fetch_optional(pool).await.ok().flatten().map(|s| s.id)
    } else {
        None
    };

    // Move to Won stage
    if let Some(ref won_stage_id) = won_stage {
        CrmDeal::move_to_stage(pool, &id, won_stage_id, 0)
            .await
            .map_err(|e| {
                tracing::error!(
                    "[mark_deal_won] Failed to move deal {} to Won stage: {}",
                    id,
                    e
                );
                ApiError::InternalError(format!("Failed to move deal to Won stage: {}", e))
            })?;
    }

    // Set won_at and win_reason
    sqlx::query("UPDATE crm_deals SET won_at = datetime('now','subsec'), win_reason = COALESCE(?, win_reason), updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(&win_reason)
        .bind(&id)
        .execute(pool).await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update deal: {}", e)))?;

    // Provision: client, project, deliverables→tasks, VIBE transaction, activity log
    let result = provision_won_deal(pool, &deal)
        .await
        .map_err(|e| ApiError::InternalError(format!("Won provisioning failed: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "deal_id": id.to_string(),
        "client_id": result.client_id,
        "client_name": result.client_name,
        "project_id": result.project_id,
        "project_name": result.project_name,
        "tasks_created": result.tasks_created,
        "vibe_amount": result.vibe_amount,
    }))))
}

// ── GET /crm/deals/:id/transcripts ──────────────────────────────────────────

pub async fn list_deal_transcripts(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<db::models::crm_deal::DealTranscript>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let transcripts = sqlx::query_as::<_, db::models::crm_deal::DealTranscript>(
        "SELECT * FROM deal_transcripts WHERE deal_id = ? ORDER BY created_at ASC",
    )
    .bind(&id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;
    Ok(Json(ApiResponse::success(transcripts)))
}

// ── POST /crm/deals/:id/transcripts ─────────────────────────────────────────

pub async fn link_deal_transcript(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<db::models::crm_deal::LinkTranscriptRequest>,
) -> Result<Json<ApiResponse<db::models::crm_deal::DealTranscript>>, ApiError> {
    let pool = &deployment.db().pool;
    let deal_id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &deal_id).await?;
    let transcript_id = DbUuid::new();
    let matched_by = body.matched_by.as_deref().unwrap_or("manual");

    sqlx::query(
        "INSERT INTO deal_transcripts (id, deal_id, intake_item_id, call_log_id, transcript_text, summary, matched_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now','subsec'))"
    )
    .bind(&transcript_id)
    .bind(&deal_id)
    .bind(&body.intake_item_id)
    .bind(&body.call_log_id)
    .bind(&body.transcript_text)
    .bind(&body.summary)
    .bind(matched_by)
    .execute(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to link transcript: {}", e)))?;

    let record = sqlx::query_as::<_, db::models::crm_deal::DealTranscript>(
        "SELECT * FROM deal_transcripts WHERE id = ?",
    )
    .bind(&transcript_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;

    // Register transcript in deal's knowledge graph
    {
        let kg_pool = pool.clone();
        let kg_deal_id = deal_id.to_string();
        let kg_trans_id = transcript_id.to_string();
        let kg_summary = body.summary.clone();
        tokio::spawn(async move {
            let _ = ProjectKnowledgeSource::upsert_owner_scoped(
                &kg_pool,
                &KnowledgeOwnerScope::Deal,
                &kg_deal_id,
                &KnowledgeSourceType::Conversation,
                &kg_trans_id,
                "Deal Transcript",
                kg_summary.as_deref(),
                0.6,
            )
            .await;
        });
    }

    Ok(Json(ApiResponse::success(record)))
}

// ── POST /crm/deals/:id/data-sources ─────────────────────────────────────────
/// Link a data source from the data library to a deal.
pub async fn link_deal_data_source(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<db::models::crm_deal::LinkDataSourceRequest>,
) -> Result<Json<ApiResponse<db::models::crm_deal::DealDataSource>>, ApiError> {
    let pool = &deployment.db().pool;
    let deal_id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &deal_id).await?;

    let link_id = DbUuid::new();
    let data_source_id = DbUuid::parse(&body.data_source_id)
        .map_err(|_| ApiError::BadRequest("Invalid data_source_id".to_string()))?;

    let relevant_stages_json = body
        .relevant_stages
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok());
    let relevant_agents_json = body
        .relevant_agents
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok());

    sqlx::query(
        "INSERT INTO deal_data_sources (id, deal_id, data_source_id, relevant_stages, relevant_agents, linked_by, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now','subsec'))"
    )
    .bind(&link_id)
    .bind(&deal_id)
    .bind(&data_source_id)
    .bind(&relevant_stages_json)
    .bind(&relevant_agents_json)
    .bind(access_context.user_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to link data source: {}", e)))?;

    let record = sqlx::query_as::<_, db::models::crm_deal::DealDataSource>(
        "SELECT * FROM deal_data_sources WHERE id = ?",
    )
    .bind(&link_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;

    // Register data source in deal's knowledge graph
    {
        let kg_pool = pool.clone();
        let kg_deal_id = deal_id.to_string();
        let kg_source_id = data_source_id.to_string();
        tokio::spawn(async move {
            let _ = ProjectKnowledgeSource::upsert_owner_scoped(
                &kg_pool,
                &KnowledgeOwnerScope::Deal,
                &kg_deal_id,
                &KnowledgeSourceType::Artifact,
                &kg_source_id,
                "Deal Data Source",
                None,
                0.5,
            )
            .await;
        });
    }

    Ok(Json(ApiResponse::success(record)))
}

// ── GET /crm/deals/:id/data-sources ──────────────────────────────────────────
/// List data sources linked to a deal.
pub async fn list_deal_data_sources(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<db::models::crm_deal::DealDataSource>>>, ApiError> {
    let pool = &deployment.db().pool;
    let deal_id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &deal_id).await?;

    let sources = sqlx::query_as::<_, db::models::crm_deal::DealDataSource>(
        "SELECT * FROM deal_data_sources WHERE deal_id = ? ORDER BY created_at ASC",
    )
    .bind(&deal_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;

    Ok(Json(ApiResponse::success(sources)))
}

// ── POST /crm/deals/:id/generate-invite ──────────────────────────────────────
/// Generate a token-based invite link for the deal's contact person.
/// Returns the invite URL for clipboard copy.
pub async fn generate_deal_invite(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let deal_id = parse_db_uuid_param(&id, "deal ID")?;
    let deal = require_deal_org_access(&access_context, pool, &deal_id).await?;

    // Verify deal is won
    if deal.won_at.is_none() {
        return Err(ApiError::BadRequest(
            "Deal must be won before generating an invite link".to_string(),
        ));
    }

    // Look up contact info
    let contact_email: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct EmailRow {
            email: Option<String>,
        }
        sqlx::query_as::<_, EmailRow>("SELECT email FROM crm_contacts WHERE id = ? LIMIT 1")
            .bind(cid)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.email)
    } else {
        None
    };

    // Generate invite token (64 hex chars from two UUIDs)
    let token = format!(
        "{}{}",
        DbUuid::new().to_string().replace('-', ""),
        DbUuid::new().to_string().replace('-', "")
    );

    // Store in deal's custom_fields
    let custom_fields = deal
        .custom_fields
        .as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let mut updated = custom_fields;
    updated["invite_token"] = serde_json::json!(token);
    updated["invite_status"] = serde_json::json!("pending");
    updated["invite_created_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    sqlx::query(
        "UPDATE crm_deals SET custom_fields = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(updated.to_string())
    .bind(&deal_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to save invite token: {}", e)))?;

    // Build invite URL (uses the accept endpoint from invitations system pattern)
    let invite_url = format!("/invite/{}", token);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "invite_url": invite_url,
        "token": token,
        "contact_email": contact_email,
        "status": "pending",
    }))))
}

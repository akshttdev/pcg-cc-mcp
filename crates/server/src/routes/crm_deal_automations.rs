//! CRM Deal Automations
//!
//! Handles AI-powered deal automations: research triggers, proposal generation (Cash),
//! deck generation (Lux), deep research (Astra), invoice handling, won automation,
//! and transcript management.

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use db::{
    db_uuid::DbUuid,
    models::crm_deal::CrmDeal,
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, helpers::uuid_params::parse_db_uuid_param, middleware::access_control::AccessContext};

use super::crm_deals::require_deal_org_access;

// ── Scout: Who-Is Research ────────────────────────────────────────────────────

/// Auto-trigger "Who Is" research for a deal's contact person and company
pub async fn trigger_who_is_research(
    pool: &sqlx::SqlitePool,
    deal_id: DbUuid,
    contact_id: Option<DbUuid>,
) {
    let Some(contact_id) = contact_id else { return };

    // Look up person_id from crm_contacts (direction 1: crm_contacts.person_id)
    // OR from persons table (direction 2: persons.crm_contact_id → migration-linked records)
    #[derive(sqlx::FromRow)]
    struct ContactRow {
        person_id: Option<DbUuid>,
    }

    // Direction 1: crm_contacts.person_id
    let person_id_via_contact =
        sqlx::query_as::<_, ContactRow>("SELECT person_id FROM crm_contacts WHERE id = ?")
            .bind(&contact_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.person_id);

    // Direction 2: persons.crm_contact_id (migration-linked records)
    let person_id = if person_id_via_contact.is_some() {
        person_id_via_contact
    } else {
        #[derive(sqlx::FromRow)]
        struct PersonRow {
            id: DbUuid,
        }
        sqlx::query_as::<_, PersonRow>("SELECT id FROM persons WHERE crm_contact_id = ? LIMIT 1")
            .bind(&contact_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.id)
    };

    let Some(person_id) = person_id else { return };

    // Check if person intelligence is idle or null — only trigger if not already running
    #[derive(sqlx::FromRow)]
    struct IntelRow {
        intelligence_status: Option<String>,
        company_name: Option<String>,
    }
    let intel = sqlx::query_as::<_, IntelRow>(
        "SELECT p.intelligence_status, p.company_name FROM persons p WHERE p.id = ?",
    )
    .bind(&person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Extract fields from intel before any borrows
    let intel_status = intel.as_ref().map(|i| {
        i.intelligence_status
            .as_deref()
            .unwrap_or("idle")
            .to_string()
    });
    let intel_company = intel.as_ref().and_then(|i| i.company_name.clone());

    if let Some(ref status) = intel_status {
        if status == "idle" || status == "" {
            // Trigger person research via the same logic as POST /api/persons/:id/research
            tracing::info!(
                "Auto-triggering Who Is research for person {} (deal {})",
                person_id,
                deal_id
            );
            let _ = sqlx::query(
                "UPDATE persons SET intelligence_status = 'queued', updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(&person_id)
            .execute(pool)
            .await;

            // Convert DbUuid to Uuid for intelligence API
            let person_uuid = person_id.to_uuid();
            let pool2 = pool.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::routes::intelligence::trigger_research_for_person(&pool2, person_uuid)
                        .await
                {
                    tracing::error!(
                        "Auto Who Is research failed for person {}: {}",
                        person_uuid,
                        e
                    );
                }
            });

            // Fetch deal's project_id for workflow task visibility
            #[derive(sqlx::FromRow)]
            struct DealProjectRow {
                project_id: Option<DbUuid>,
            }
            let deal_project = sqlx::query_as::<_, DealProjectRow>(
                "SELECT project_id FROM crm_deals WHERE id = ?",
            )
            .bind(&deal_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

            #[derive(sqlx::FromRow)]
            struct PersonNameRow {
                full_name: Option<String>,
            }
            let person_data =
                sqlx::query_as::<_, PersonNameRow>("SELECT full_name FROM persons WHERE id = ?")
                    .bind(&person_id)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();

            let project_id_ref = deal_project.as_ref().and_then(|d| d.project_id.as_ref());

            // Create Phase 1 workflow visibility tasks (deduped by title prefix)
            let existing_phase1: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND title LIKE 'Phase 1 Research:%' AND deleted_at IS NULL",
            )
            .bind(&deal_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            if existing_phase1 == 0 {
                if let Some(ref pd) = person_data {
                    if let Some(ref name) = pd.full_name {
                        let task_id = DbUuid::new();
                        let _ = sqlx::query(
                            "INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, created_at, updated_at) VALUES (?, ?, ?, 'inprogress', ?, ?, datetime('now','subsec'), datetime('now','subsec'))",
                        )
                        .bind(&task_id)
                        .bind(format!("Phase 1 Research: {} (Person)", name))
                        .bind("AI research gathering intelligence profile for this contact")
                        .bind(&deal_id)
                        .bind(project_id_ref)
                        .execute(pool)
                        .await;
                    }
                }

                if let Some(ref cn) = intel_company.as_ref().filter(|n| !n.is_empty()) {
                    let task_id = DbUuid::new();
                    let _ = sqlx::query(
                        "INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, created_at, updated_at) VALUES (?, ?, ?, 'inprogress', ?, ?, datetime('now','subsec'), datetime('now','subsec'))",
                    )
                    .bind(&task_id)
                    .bind(format!("Phase 1 Research: {} (Company)", cn))
                    .bind("AI research gathering company intelligence and building company wiki")
                    .bind(&deal_id)
                    .bind(project_id_ref)
                    .execute(pool)
                    .await;
                }
            }
        }
    }

    // If person has a company_name, check/create Company and trigger company research if idle
    if let Some(ref company_name) = intel_company.filter(|n| !n.is_empty()) {
        trigger_company_research_if_idle(pool, company_name).await;
    }
}

/// Trigger company research if the company's intel status is idle
async fn trigger_company_research_if_idle(pool: &sqlx::SqlitePool, company_name: &str) {
    #[derive(sqlx::FromRow)]
    struct CompanyRow {
        id: DbUuid,
        intelligence_status: Option<String>,
    }
    let company = sqlx::query_as::<_, CompanyRow>(
        "SELECT id, intelligence_status FROM companies WHERE name = ? COLLATE NOCASE LIMIT 1",
    )
    .bind(company_name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(company) = company {
        let status = company.intelligence_status.as_deref().unwrap_or("idle");
        if status == "idle" || status == "" {
            tracing::info!(
                "Auto-triggering company research for '{}' ({})",
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
    }
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

    // Resolve person via persons.crm_contact_id (reverse lookup)
    #[derive(sqlx::FromRow)]
    struct PersonDataRow {
        id: DbUuid,
        company_name: Option<String>,
    }
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
    let person_row = sqlx::query_as::<_, PersonDataRow>(
        "SELECT id, company_name FROM persons WHERE crm_contact_id = ? LIMIT 1",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let person_id = person_row.as_ref().map(|p| p.id.clone());
    let company_name = person_row
        .as_ref()
        .and_then(|p| p.company_name.clone())
        .or(contact_company);

    #[derive(sqlx::FromRow)]
    struct PersonIntelRow {
        id: DbUuid,
        full_name: Option<String>,
        intelligence_summary: Option<String>,
        email: Option<String>,
        job_title: Option<String>,
    }
    let person_intel = if let Some(ref pid) = person_id {
        sqlx::query_as::<_, PersonIntelRow>(
            "SELECT id, full_name, intelligence_summary, email, job_title FROM persons WHERE id = ?",
        )
        .bind(pid)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

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

    let executive_summary = match (person_summary.as_deref(), company_summary.as_deref()) {
        (Some(ps), Some(cs)) => Some(format!(
            "## Person Intelligence\n{}\n\n## Company Intelligence\n{}",
            ps, cs
        )),
        (Some(ps), None) => Some(format!("## Person Intelligence\n{}", ps)),
        (None, Some(cs)) => Some(format!("## Company Intelligence\n{}", cs)),
        (None, None) => None,
    };

    let individual_profile = if let Some(ref pi) = person_intel {
        let mut parts = Vec::new();
        if let Some(ref name) = pi.full_name {
            parts.push(format!("**Name:** {}", name));
        }
        if let Some(ref email) = pi.email {
            parts.push(format!("**Email:** {}", email));
        }
        if let Some(ref title) = pi.job_title {
            parts.push(format!("**Title:** {}", title));
        }
        if let Some(ref summary) = pi.intelligence_summary {
            parts.push(format!("\n{}", summary));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    } else {
        None
    };

    let company_overview = company_intel.as_ref().map(|c| {
        let mut parts = Vec::new();
        if let Some(ref industry) = c.industry {
            parts.push(format!("**Industry:** {}", industry));
        }
        if let Some(ref desc) = c.description {
            parts.push(format!("**Overview:** {}", desc));
        }
        if let Some(ref summary) = c.intelligence_summary {
            parts.push(format!("\n{}", summary));
        }
        parts.join("\n")
    });

    let title = format!(
        "Phase 1 Business Analysis: {} / {}",
        contact_name, company_display
    );
    // BLOB-column binding: business_reports.id/person_id/company_id/crm_deal_id are BLOB
    // Phase C TODO: migrate these columns to TEXT so we can bind DbUuid directly
    let person_uuid = person_intel.as_ref().map(|p| p.id.to_uuid());
    let company_uuid = company_intel.as_ref().map(|c| c.id.to_uuid());
    let deal_uuid = Some(deal_id.to_uuid());

    let report_id = DbUuid::new().to_uuid();
    let individual_profiles_json = individual_profile
        .map(|p| serde_json::json!([{"name": contact_name, "profile": p}]).to_string())
        .unwrap_or_else(|| "[]".to_string());

    let res = sqlx::query(
        r#"INSERT INTO business_reports
           (id, person_id, company_id, crm_deal_id, report_type, title, status,
            executive_summary, company_overview, individual_profiles,
            pain_points, opportunities, recommended_services, next_steps,
            competitor_analysis, intake_item_ids, call_log_ids)
           VALUES (?, ?, ?, ?, 'phase1_analysis', ?, 'draft', ?, ?, ?, '[]', '[]', '[]', '[]', '[]', '[]', '[]')"#,
    )
    .bind(report_id)
    .bind(person_uuid)
    .bind(company_uuid)
    .bind(deal_uuid)
    .bind(&title)
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
    let _ = sqlx::query(
        "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec') WHERE crm_deal_id = ? AND title LIKE 'Phase 1 Research:%' AND status = 'inprogress' AND deleted_at IS NULL",
    )
    .bind(&deal_id)
    .execute(pool)
    .await;
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
            "SELECT intelligence_summary, full_name FROM persons WHERE crm_contact_id = ? LIMIT 1",
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
            "SELECT c.intelligence_summary, c.name FROM companies c JOIN persons p ON lower(p.company_name) = lower(c.name) WHERE p.crm_contact_id = ? LIMIT 1"
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

    let astra_system = "You are Astra, a business intelligence analyst at PowerClub Global. Your task is to enhance an existing business analysis report with new discovery context from client conversations. Synthesize all available intelligence into a comprehensive, actionable business report. Include: Executive Summary, Client Pain Points, Opportunities, Recommended Services (with estimated value ranges), Competitive Landscape, and Strategic Recommendations. Be specific and data-driven.";
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
                let _ = sqlx::query(
                    "UPDATE business_reports SET executive_summary = ?, research_depth = ?, status = 'enhanced', updated_at = datetime('now','subsec') WHERE id = ?"
                )
                .bind(&enhanced_report)
                .bind(new_depth)
                .bind(&r.id)
                .execute(pool)
                .await;
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
                let _ = sqlx::query(
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
                .await;
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
        sqlx::query_as::<_, PersonIntel>("SELECT intelligence_summary, full_name, company_name FROM persons WHERE crm_contact_id = ? LIMIT 1")
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
            "SELECT c.intelligence_summary, c.name FROM companies c JOIN persons p ON lower(p.company_name) = lower(c.name) WHERE p.crm_contact_id = ? LIMIT 1"
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

    let cash_system = "You are Cash, a razor-sharp sales strategist and proposal architect at PowerClub Global, a creative production and brand strategy agency. You transform business intelligence into irresistible, tailored proposals. Every word earns its place. Write proposals in clear, compelling markdown with these sections: Executive Summary, Client Situation, Proposed Solution, Deliverables, Timeline, Investment, and Next Steps. IMPORTANT: At the very end, include a JSON block with the deliverables list using this exact format:\n\n```json\n[{\"title\": \"Deliverable Name\", \"description\": \"Brief description\"}]\n```\n\nBe bold, specific, and value-driven. British English optional.";
    let user_prompt = format!(
        "Write a professional proposal for the following deal.\n\n{}",
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
                    let _ = sqlx::query(
                        "UPDATE crm_deals SET amount = ?, updated_at = datetime('now','subsec') WHERE id = ?"
                    )
                    .bind(total_value)
                    .bind(id)
                    .execute(pool)
                    .await;
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
                        let _ = sqlx::query(
                            "INSERT INTO deliverables (id, crm_deal_id, project_id, title, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'pending', datetime('now','subsec'), datetime('now','subsec'))"
                        )
                        .bind(&deliverable_id)
                        .bind(&id)
                        .bind(&deal.project_id)
                        .bind(title)
                        .bind(desc)
                        .execute(pool)
                        .await;
                        deliverable_count += 1;
                    }
                }
            }
        }
    }

    // Log activity
    if deliverable_count > 0 {
        let _ = sqlx::query(
            "INSERT INTO crm_activities (id, organization_id, crm_contact_id, crm_deal_id, activity_type, subject, activity_at) VALUES (?, ?, ?, ?, 'deliverables_created', ?, datetime('now','subsec'))"
        )
        .bind(DbUuid::new())
        .bind(&deal.organization_id)
        .bind(&deal.crm_contact_id)
        .bind(&id)
        .bind(format!("Proposal approved: {} deliverables created", deliverable_count))
        .execute(pool)
        .await;
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
    let lux_system = "You are Lux, a master presentation designer. You transform proposals into structured, high-impact slide decks. Output a complete slide-by-slide script in markdown, with each slide on a --- separator. Each slide has: SLIDE TITLE, KEY MESSAGE (1 sentence), VISUAL SUGGESTION, and SPEAKER NOTES. Design for clarity, impact, and brand alignment. Slides: Cover, Agenda, Client Situation, Our Solution, Deliverables, Timeline, Investment, Why Us, Next Steps, Close.";
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

    let _ = sqlx::query(
        "INSERT OR IGNORE INTO project_knowledge_sources (id, owner_type, owner_id, source_type, source_id, source_title, source_summary, coverage_score, is_active, created_at, updated_at) VALUES (?, 'deal', ?, 'deck_script', ?, ?, ?, 0.8, 1, datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(DbUuid::new().to_string())
    .bind(id.to_string())
    .bind(deck_id.to_string())
    .bind(format!("Sales Deck: {}", deal.name))
    .bind(deck_script.chars().take(500).collect::<String>())
    .execute(pool).await;

    let deck_data = serde_json::json!({ "deck_id": deck_id.to_string(), "script": deck_script });
    let _ = sqlx::query("UPDATE crm_deals SET custom_fields = ?, deck_url = ?, updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(deck_data.to_string())
        .bind(&deck_url)
        .bind(id)
        .execute(pool).await;

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
        let _ = CrmDeal::move_to_stage(pool, &id, won_stage_id, 0).await;
    }

    // Set won_at and win_reason
    sqlx::query("UPDATE crm_deals SET won_at = datetime('now','subsec'), win_reason = COALESCE(?, win_reason), updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(&win_reason)
        .bind(&id)
        .execute(pool).await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update deal: {}", e)))?;

    // Resolve contact info for client/project creation
    #[derive(sqlx::FromRow)]
    struct ContactInfo {
        full_name: Option<String>,
        company_name: Option<String>,
        email: Option<String>,
    }
    let contact_info = if let Some(ref cid) = deal.crm_contact_id {
        sqlx::query_as::<_, ContactInfo>("SELECT p.full_name, p.company_name, p.email FROM persons p WHERE p.crm_contact_id = ? LIMIT 1")
            .bind(cid).fetch_optional(pool).await.ok().flatten()
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
            let _ = sqlx::query(
                "INSERT INTO clients (id, organization_id, name, slug, crm_contact_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
            )
            .bind(&cid)
            .bind(&deal.organization_id)
            .bind(&company_name)
            .bind(&slug)
            .bind(&deal.crm_contact_id)
            .execute(pool).await;
            cid
        }
    };

    // ── Create Project ───────────────────────────────────────────────────────
    let project_id = DbUuid::new();
    let project_name = format!("{} — {}", deal.name, company_name);
    let _ = sqlx::query(
        "INSERT INTO projects (id, name, git_repo_path, client_id, organization_id, created_at, updated_at) VALUES (?, ?, '', ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(&project_id)
    .bind(&project_name)
    .bind(&client_id)
    .bind(&deal.organization_id)
    .execute(pool).await;

    // Link project to deal
    let _ = sqlx::query(
        "UPDATE crm_deals SET project_id = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(&project_id)
    .bind(&id)
    .execute(pool)
    .await;

    // ── F3: Move deliverables to the new project ────────────────────────────
    let _ = sqlx::query(
        "UPDATE deliverables SET project_id = ?, updated_at = datetime('now','subsec') WHERE crm_deal_id = ?"
    )
    .bind(&project_id)
    .bind(&id)
    .execute(pool)
    .await;

    // ── F3: Create tasks from deliverables table (not JSON re-parsing) ──────
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
    .bind(&id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut task_count = deliverables.len() as i32;
    for deliv in &deliverables {
        let task_id = DbUuid::new();
        let _ = sqlx::query(
            "INSERT INTO tasks (id, project_id, crm_deal_id, title, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'todo', datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(&task_id).bind(&project_id).bind(&id)
        .bind(&deliv.title).bind(&deliv.description)
        .execute(pool).await;
    }

    // Fallback: if no deliverables exist, create a default setup task
    if task_count == 0 {
        let task_id = DbUuid::new();
        let _ = sqlx::query(
            "INSERT INTO tasks (id, project_id, crm_deal_id, title, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'todo', datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(&task_id).bind(&project_id).bind(&id)
        .bind("Project Setup & Kickoff")
        .bind(format!("Initial project setup for {}. Review proposal and create specific deliverables.", company_name))
        .execute(pool).await;
        task_count = 1;
    }

    // ── F3: VIBE transaction for deal value ─────────────────────────────────
    if let Some(amount) = deal.amount {
        let vibe_amount = amount * 100.0; // 1 USD = 100 VIBE
        let tx_id = DbUuid::new();
        let _ = sqlx::query(
            "INSERT INTO vibe_transactions (id, amount, transaction_type, description, created_at) VALUES (?, ?, 'deal_won', ?, datetime('now','subsec'))"
        )
        .bind(&tx_id)
        .bind(vibe_amount)
        .bind(format!("Deal won: {} — ${:.2}", deal.name, amount))
        .execute(pool)
        .await;
        tracing::info!(
            "VIBE transaction {} created: {} VIBE for deal {}",
            tx_id,
            vibe_amount,
            id
        );
    }

    // F3: Person invitation — log for now (full invite system to be wired later)
    if let Some(ref ci) = contact_info {
        tracing::info!(
            "Won deal {}: person invitation pending for {} ({}) — wire invite system later",
            id,
            ci.full_name.as_deref().unwrap_or("unknown"),
            ci.email.as_deref().unwrap_or("no-email")
        );
    }

    // Log Won activity
    let _ = sqlx::query(
        "INSERT INTO crm_activities (id, organization_id, crm_contact_id, crm_deal_id, activity_type, subject, activity_at) VALUES (?, ?, ?, ?, 'deal_won', ?, datetime('now','subsec'))"
    )
    .bind(DbUuid::new())
    .bind(&deal.organization_id)
    .bind(&deal.crm_contact_id)
    .bind(&id)
    .bind(format!("Deal won: {} — {} tasks created", deal.name, task_count))
    .execute(pool).await;

    tracing::info!(
        "Deal {} marked Won. Client '{}', Project '{}', {} tasks created",
        id,
        company_name,
        project_name,
        task_count
    );
    Ok(Json(ApiResponse::success(serde_json::json!({
        "deal_id": id.to_string(),
        "client_id": client_id.to_string(),
        "project_id": project_id.to_string(),
        "project_name": project_name,
        "tasks_created": task_count,
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

    Ok(Json(ApiResponse::success(record)))
}

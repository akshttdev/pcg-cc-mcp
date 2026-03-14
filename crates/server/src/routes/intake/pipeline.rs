//! Core intake pipeline orchestration, CRM association, and knowledge graph registration.
//!
//! Contains:
//! - `run_intake_pipeline` — the main 6-stage pipeline
//! - Stage 1 dispatch to `extract::extract_intake_structure`
//! - Stage 2: participant → CRM person association (find/create/update)
//! - Stage 4: knowledge graph registration
//! - CRM deal automation helpers

use db::models::{
    call_intake_item::CallIntakeItem,
    company::Company,
    crm_deal::{CreateCrmDeal, CrmDeal},
    project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
    proposal::{CreateProposal, Proposal},
};
use tracing::{info, warn};
use uuid::Uuid;

use super::{ExtractedIndividual, ExtractedIntake, ExtractedParticipant};
use super::report::{run_company_research_pass, run_report_generation};

// ── Core pipeline ─────────────────────────────────────────────────────────────

/// Full intake processing pipeline for a single item.
pub async fn run_intake_pipeline(
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
    let extracted = match super::report::extract_intake_structure(&raw).await {
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
        }
    });

    info!("Intake pipeline complete for item {}", item_id);
    Ok(())
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

pub(super) async fn find_person_by_email(pool: &sqlx::SqlitePool, email: &str) -> Option<Uuid> {
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

pub(super) async fn find_person_by_name(pool: &sqlx::SqlitePool, name: &str) -> Option<Uuid> {
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

// ── Contextual individual ingestion ──────────────────────────────────────────

/// Ingest contextually-mentioned individuals (non-prospects) into the org knowledge graph.
/// These are people referenced in the call — we research who they are for context,
/// but we do NOT create proposals or treat them as pipeline leads.
async fn ingest_contextual_individuals(
    pool: &sqlx::SqlitePool,
    individuals: &[ExtractedIndividual],
    _intake_item_id: Uuid,
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

// ── Helpers ───────────────────────────────────────────────────────────────────

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
pub(super) async fn advance_deal_stage(pool: &sqlx::SqlitePool, deal_id: Uuid, stage_name: &str) {
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

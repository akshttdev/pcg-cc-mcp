//! Unified Stage Transition Processor
//!
//! Single entry point for ALL CRM deal stage transitions (DnD, advance button, agent-driven).
//! Reads stage_config from DB for data-driven behavior, falls back to hardcoded logic for
//! stages without config.

use chrono::{DateTime, Utc};
use db::{
    db_uuid::DbUuid,
    models::{
        crm_deal::{CreateCrmDeal, CrmDeal},
        crm_pipeline::{CrmPipeline, CrmPipelineStage, PipelineType},
    },
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;

use crate::routes::{
    crm_deal_automations::{
        generate_deck_background, generate_phase1_business_report, trigger_deep_research_pass2,
        trigger_who_is_research,
    },
    crm_deal_transitions::manage_stage_review_tasks,
};

// ── Stage Config Schema ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StageConfig {
    #[serde(default)]
    pub assigned_agent: Option<String>,
    #[serde(default)]
    pub auto_trigger: bool,
    #[serde(default = "default_cancel_window")]
    pub cancel_window_secs: u32,
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub approval_gate: bool,
    #[serde(default)]
    pub on_enter_actions: Vec<StageAction>,
    #[serde(default)]
    pub on_exit_validations: Vec<StageValidation>,
    #[serde(default)]
    pub stage_owner: Option<StageOwner>,
}

fn default_cancel_window() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StageAction {
    TriggerAgent { agent: String, flow_type: String },
    CreateReviewTask { description: String },
    CreateDeliveryDeal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StageValidation {
    RequireField { field: String, message: String },
    RequireIntel { entity: String, status: String },
    RequirePendingTasks { count: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StageOwner {
    pub label: String,
    #[serde(rename = "type")]
    pub owner_type: String, // "agent", "human", "team"
}

// ── Transition Result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TransitionResult {
    pub deal: CrmDeal,
    pub warnings: Vec<ValidationWarning>,
    pub agent_flow_id: Option<String>,
    pub cancel_deadline: Option<DateTime<Utc>>,
    pub actions_taken: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

// ── Processor ────────────────────────────────────────────────────────────────

/// Load and parse stage_config JSON from a CrmPipelineStage.
pub fn parse_stage_config(stage: &CrmPipelineStage) -> Option<StageConfig> {
    stage.stage_config.as_deref().and_then(|json| {
        serde_json::from_str(json)
            .map_err(|e| {
                tracing::warn!(
                    "[StageTransition] Failed to parse stage_config for stage '{}' (id={}): {}",
                    stage.name,
                    stage.id,
                    e
                );
            })
            .ok()
    })
}

/// Unified stage transition processor.
///
/// Both `move_deal_stage` (DnD) and `advance_deal` (button) call this after
/// moving the deal in the database. It handles:
/// 1. Exit validation (soft warnings, not blocking)
/// 2. Entry actions (agent triggers, review tasks, delivery deal creation)
/// 3. Won transition handling (deduplicated)
///
/// The processor merges two config sources:
/// - **Explicit** `on_enter_actions` / `on_exit_validations` (set via migration or raw JSON)
/// - **Derived** from simple UI fields (`assigned_agent`, `required_fields`, `approval_gate`)
///
/// Explicit actions always take priority. Derived actions only fire when the
/// explicit arrays don't already cover the same intent (e.g., if `on_enter_actions`
/// already contains a `TriggerAgent`, the `assigned_agent` field won't add a duplicate).
pub async fn process_transition(
    pool: &SqlitePool,
    deal: &CrmDeal,
    from_stage: Option<&CrmPipelineStage>,
    to_stage: &CrmPipelineStage,
) -> TransitionResult {
    let mut warnings = Vec::new();
    let mut actions_taken = Vec::new();
    let mut agent_flow_id: Option<String> = None;
    let mut cancel_deadline: Option<DateTime<Utc>> = None;

    let to_config = parse_stage_config(to_stage);
    let from_config = from_stage.and_then(parse_stage_config);

    // ── 1. Exit validation (explicit on_exit_validations + derived from required_fields) ──
    if let Some(ref config) = from_config {
        let effective_validations = build_effective_exit_validations(config);
        for validation in &effective_validations {
            match validation {
                StageValidation::RequireField { field, message } => {
                    let value = match field.as_str() {
                        "description" => deal.description.as_deref().unwrap_or(""),
                        "name" => &deal.name,
                        "amount" => {
                            if deal.amount.is_some() {
                                "set"
                            } else {
                                ""
                            }
                        }
                        "crm_contact_id" => {
                            if deal.crm_contact_id.is_some() {
                                "set"
                            } else {
                                ""
                            }
                        }
                        "proposal_text" => deal.proposal_text.as_deref().unwrap_or(""),
                        "deck_url" => deal.deck_url.as_deref().unwrap_or(""),
                        _ => "",
                    };
                    if value.trim().is_empty() {
                        warnings.push(ValidationWarning {
                            field: field.clone(),
                            message: message.clone(),
                        });
                    }
                }
                StageValidation::RequireIntel { entity, status } => {
                    let intel_ok = check_intel_status(pool, deal, entity, status).await;
                    if !intel_ok {
                        warnings.push(ValidationWarning {
                            field: format!("{}_intelligence", entity),
                            message: format!("{} intelligence is not '{}' yet", entity, status),
                        });
                    }
                }
                StageValidation::RequirePendingTasks { count } => {
                    let pending = count_pending_tasks(pool, deal).await;
                    if pending > *count as i64 {
                        warnings.push(ValidationWarning {
                            field: "pending_tasks".to_string(),
                            message: format!("{} pending task(s) should be completed", pending),
                        });
                    }
                }
            }
        }
    }

    // ── 2. Won transition (deduplicated, uses contact_id dedup) ─────────
    let is_won = to_stage.is_won.unwrap_or(0) == 1;
    if is_won {
        if let Some(action) = handle_won_transition(pool, deal).await {
            actions_taken.push(action);
        }
    }

    // ── 3. Entry actions (explicit on_enter_actions + derived from UI fields) ──
    if let Some(ref config) = to_config {
        let effective_actions = build_effective_entry_actions(config);

        for action in &effective_actions {
            match action {
                StageAction::TriggerAgent { agent, flow_type } => {
                    const KNOWN_AGENTS: &[&str] =
                        &["scout", "astra", "cash", "lux", "nora", "assistant"];
                    if !KNOWN_AGENTS.contains(&agent.to_lowercase().as_str()) {
                        tracing::warn!(
                            "[StageTransition] Unknown agent '{}' in stage config — skipping",
                            agent
                        );
                        continue;
                    }
                    if config.auto_trigger {
                        match schedule_agent_flow(
                            pool,
                            deal,
                            agent,
                            flow_type,
                            config.cancel_window_secs,
                        )
                        .await
                        {
                            Ok((flow_id, deadline)) => {
                                agent_flow_id = Some(flow_id);
                                cancel_deadline = Some(deadline);
                                actions_taken.push(format!("Scheduled {} agent", agent));
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to schedule agent flow for deal {}: {}",
                                    deal.id,
                                    e
                                );
                            }
                        }
                    }
                }
                StageAction::CreateReviewTask { description } => {
                    let stage_name = to_stage.name.to_lowercase();
                    manage_stage_review_tasks(pool, deal, description, &stage_name).await;
                    actions_taken.push("Created review task".to_string());
                }
                StageAction::CreateDeliveryDeal => {
                    if let Some(action) = handle_won_transition(pool, deal).await {
                        actions_taken.push(action);
                    }
                }
            }
        }
    } else {
        // ── Hardcoded fallback for stages without config ────────────────
        run_hardcoded_entry_actions(pool, deal, to_stage, &mut actions_taken).await;
    }

    TransitionResult {
        deal: deal.clone(),
        warnings,
        agent_flow_id,
        cancel_deadline,
        actions_taken,
    }
}

// ── Effective Action/Validation Builders ─────────────────────────────────────

/// Build the effective list of entry actions by merging explicit `on_enter_actions`
/// with actions derived from simple UI fields (`assigned_agent`, `approval_gate`).
///
/// Rules:
/// - If `on_enter_actions` already contains a `TriggerAgent`, don't add another from `assigned_agent`
/// - If `on_enter_actions` already contains a `CreateReviewTask`, don't add another from `approval_gate`
/// - Explicit actions always come first (preserve ordering from migration/admin JSON)
fn build_effective_entry_actions(config: &StageConfig) -> Vec<StageAction> {
    let mut actions = config.on_enter_actions.clone();

    let has_trigger_agent = actions
        .iter()
        .any(|a| matches!(a, StageAction::TriggerAgent { .. }));
    let has_review_task = actions
        .iter()
        .any(|a| matches!(a, StageAction::CreateReviewTask { .. }));

    // Derive TriggerAgent from assigned_agent + auto_trigger
    if !has_trigger_agent {
        if let Some(ref agent) = config.assigned_agent {
            if config.auto_trigger && !agent.is_empty() {
                actions.push(StageAction::TriggerAgent {
                    agent: agent.clone(),
                    flow_type: agent_default_flow_type(agent),
                });
            }
        }
    }

    // Derive CreateReviewTask from approval_gate
    if !has_review_task && config.approval_gate {
        actions.push(StageAction::CreateReviewTask {
            description: "Approval required before deal can leave this stage".to_string(),
        });
    }

    actions
}

/// Build the effective list of exit validations by merging explicit `on_exit_validations`
/// with validations derived from `required_fields`.
///
/// Rules:
/// - If `on_exit_validations` already contains a `RequireField` for a given field, skip it
/// - Explicit validations come first
fn build_effective_exit_validations(config: &StageConfig) -> Vec<StageValidation> {
    let mut validations = config.on_exit_validations.clone();

    let existing_fields: Vec<String> = validations
        .iter()
        .filter_map(|v| match v {
            StageValidation::RequireField { field, .. } => Some(field.clone()),
            _ => None,
        })
        .collect();

    for field in &config.required_fields {
        if !existing_fields.iter().any(|f| f == field) {
            let label = match field.as_str() {
                "description" => "Operator context (description)",
                "amount" => "Deal amount",
                "crm_contact_id" => "Contact",
                "proposal_text" => "Proposal",
                "deck_url" => "Deck",
                other => other,
            };
            validations.push(StageValidation::RequireField {
                field: field.clone(),
                message: format!("{} is required before advancing", label),
            });
        }
    }

    validations
}

/// Map agent name to a sensible default flow_type when derived from the UI
/// (the full seed configs specify explicit flow_types, but UI-only configs don't).
fn agent_default_flow_type(agent: &str) -> String {
    match agent.to_lowercase().as_str() {
        "scout" => "research".to_string(),
        "astra" => "business_analysis".to_string(),
        "cash" => "proposal".to_string(),
        "lux" => "deck".to_string(),
        "nora" => "assistant".to_string(),
        other => other.to_string(),
    }
}

// ── Hardcoded Fallback ───────────────────────────────────────────────────────

/// Runs the existing hardcoded stage-entry automations for stages that don't
/// have stage_config yet. This preserves backward compatibility during the
/// incremental migration.
async fn run_hardcoded_entry_actions(
    pool: &SqlitePool,
    deal: &CrmDeal,
    to_stage: &CrmPipelineStage,
    actions_taken: &mut Vec<String>,
) {
    let stage_name_lower = to_stage.name.to_lowercase();
    let stage_type_lower = to_stage.stage_type.as_deref().unwrap_or("").to_lowercase();

    // Intel → Scout: Phase I Who-Is research
    if stage_name_lower == "intel" || stage_type_lower == "intel" {
        let pool_bg = pool.clone();
        let deal_id = deal.id.clone();
        let contact_id = deal.crm_contact_id.clone();
        tokio::spawn(async move {
            trigger_who_is_research(&pool_bg, deal_id, contact_id).await;
        });
        actions_taken.push("Triggered Scout research".to_string());
    }

    // Business Analysis → Astra: Phase I business report
    if stage_name_lower == "business analysis" || stage_type_lower == "business_analysis" {
        let pool_bg = pool.clone();
        let deal_id = deal.id.clone();
        let contact_id = deal.crm_contact_id.clone();
        let project_id = deal.project_id.clone();
        tokio::spawn(async move {
            generate_phase1_business_report(&pool_bg, deal_id, contact_id, project_id).await;
        });
        actions_taken.push("Triggered Astra business report".to_string());
    }

    // Proposal → Astra Pass 2 + Cash chain
    if stage_name_lower == "proposal" || stage_type_lower == "proposal" {
        if deal.proposal_text.is_none() || deal.proposal_text.as_deref() == Some("") {
            let pool_bg = pool.clone();
            let deal_id = deal.id.clone();
            tokio::spawn(async move {
                trigger_deep_research_pass2(&pool_bg, deal_id).await;
            });
            actions_taken.push("Triggered Astra Pass 2 + Cash proposal".to_string());
        }
    }

    // Polish → Lux: deck generation
    if stage_name_lower == "polish" || stage_type_lower == "polish" {
        if deal.deck_url.is_none() && deal.proposal_text.is_some() {
            let pool_bg = pool.clone();
            let deal_id = deal.id.clone();
            tokio::spawn(async move {
                generate_deck_background(&pool_bg, deal_id).await;
            });
            actions_taken.push("Triggered Lux deck generation".to_string());
        }
    }

    // Review task creation for gated stages
    let review_stages = [
        (
            "intel",
            "Review Phase I intelligence (Scout): person profile & company overview",
        ),
        (
            "business analysis",
            "Review business report (Astra): pain points, opportunities, recommended services",
        ),
        (
            "discovery",
            "Review discovery transcript and confirm proposal readiness",
        ),
        (
            "proposal",
            "Review and approve proposal (Cash) before moving to Polish",
        ),
        (
            "polish",
            "Review and approve deck (Lux) before presenting to client",
        ),
        (
            "present",
            "Confirm invoice sent and await payment confirmation",
        ),
        (
            "follow up",
            "Update follow-up status — won, lost, or still in discussion",
        ),
    ];

    for (stage_key, task_desc) in &review_stages {
        if stage_name_lower == *stage_key || stage_type_lower == *stage_key {
            manage_stage_review_tasks(pool, deal, task_desc, stage_key).await;
            actions_taken.push("Created review task".to_string());
            break;
        }
    }
}

// ── Won Transition (deduplicated) ────────────────────────────────────────────

/// Handle Won stage transition: auto-create a Delivery pipeline deal.
/// Uses contact_id dedup (more reliable than name pattern matching).
/// Returns an action description if a delivery deal was created.
async fn handle_won_transition(pool: &SqlitePool, deal: &CrmDeal) -> Option<String> {
    let pipeline_id = deal.crm_pipeline_id.as_ref()?;
    let pipeline = CrmPipeline::find_by_id(pool, pipeline_id)
        .await
        .map_err(|e| tracing::warn!("[StageTransition] Pipeline lookup failed: {e}"))
        .ok()?;

    // Only for sales/clients pipelines
    if pipeline.pipeline_type != "sales" && pipeline.pipeline_type != "clients" {
        return None;
    }

    let deal_org_id = deal.organization_id.as_ref()?;
    let delivery_pipeline =
        CrmPipeline::find_by_type_for_org(pool, deal_org_id, PipelineType::Delivery)
            .await
            .map_err(|e| tracing::warn!("[StageTransition] Delivery pipeline lookup failed: {e}"))
            .ok()
            .flatten()?;

    // Dedup: check if delivery deal already exists for this contact
    let has_delivery_deal = if let Some(ref contact_id) = deal.crm_contact_id {
        let contact_deals = CrmDeal::find_by_contact(pool, contact_id)
            .await
            .unwrap_or_default();
        contact_deals
            .iter()
            .any(|d| d.crm_pipeline_id.as_ref() == Some(&delivery_pipeline.id))
    } else {
        false
    };

    if has_delivery_deal {
        tracing::warn!(
            "Skipping delivery deal creation: contact already has a deal in delivery pipeline {}",
            delivery_pipeline.id
        );
        return None;
    }

    let delivery_stages = CrmPipelineStage::find_by_pipeline(pool, &delivery_pipeline.id)
        .await
        .unwrap_or_default();
    let first_stage = delivery_stages.first()?;

    match CrmDeal::create(
        pool,
        CreateCrmDeal {
            organization_id: deal_org_id.clone(),
            client_id: deal.client_id.clone(),
            crm_contact_id: deal.crm_contact_id.clone(),
            crm_pipeline_id: Some(delivery_pipeline.id.clone()),
            crm_stage_id: Some(first_stage.id.clone()),
            name: format!("{} - Delivery", deal.name),
            description: Some(format!("Auto-created from won deal: {}", deal.name)),
            amount: deal.amount,
            currency: Some(deal.currency.clone()),
            expected_close_date: None,
            tags: None,
            custom_fields: None,
        },
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::error!(
                "[StageTransition] Failed to create delivery deal for {}: {}",
                deal.id,
                e
            );
            return None;
        }
    }

    Some("Created delivery pipeline deal".to_string())
}

// ── Agent Flow Scheduling ────────────────────────────────────────────────────

/// Schedule an agent flow for a deal with a cancel window.
/// Returns (flow_id, cancel_deadline).
async fn schedule_agent_flow(
    pool: &SqlitePool,
    deal: &CrmDeal,
    agent_name: &str,
    flow_type: &str,
    cancel_window_secs: u32,
) -> anyhow::Result<(String, DateTime<Utc>)> {
    // Clamp cancel window to reasonable bounds (0 = immediate, max 1 hour)
    let clamped_window = cancel_window_secs.min(3600);
    let flow_id = DbUuid::new().to_string();
    let deadline = Utc::now() + chrono::Duration::seconds(clamped_window as i64);

    // Map pipeline flow types to agent_flows CHECK constraint values.
    // The original flow_type is preserved in flow_config for the executor.
    let db_flow_type = match flow_type {
        "research" => "research",
        "business_analysis" | "analysis" => "analysis",
        "proposal" | "deck" => "content_creation",
        _ => "custom",
    };

    // We need a task_id for the agent_flows table (FK to tasks.id BLOB).
    // Find an existing task linked to this deal, or create a minimal one.
    // Then convert to BLOB format for the FK.
    let task_id_text: String = match sqlx::query_scalar::<_, String>(
        "SELECT id FROM tasks WHERE crm_deal_id = ? AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
    )
    .bind(deal.id.to_string())
    .fetch_optional(pool)
    .await
    {
        Ok(Some(id)) => id,
        _ => {
            // Create a minimal agent task for the FK reference
            let new_id = DbUuid::new().to_string();
            sqlx::query(
                "INSERT INTO tasks (id, title, status, project_id, crm_deal_id, created_by, created_at, updated_at) \
                 VALUES (?1, ?2, 'inprogress', '', ?3, 'system', datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(&new_id)
            .bind(format!("{} agent task — {}", agent_name, &deal.name))
            .bind(deal.id.to_string())
            .execute(pool)
            .await?;
            new_id
        }
    };
    let task_id = task_id_text;

    let flow_config = serde_json::json!({
        "agent_name": agent_name,
        "flow_type": flow_type,
        "deal_id": deal.id.to_string(),
        "deal_name": deal.name,
        "contact_id": deal.crm_contact_id,
    });

    // Insert as TEXT UUIDs — DbUuid reads both BLOB and TEXT transparently.
    // The agent_flows schema has BLOB columns but SQLite is type-flexible;
    // TEXT UUIDs work and DbUuid handles the decode on read.
    sqlx::query(
        r#"
        INSERT INTO agent_flows (
            id, task_id, flow_type, status, current_phase,
            flow_config, human_approval_required, planning_started_at,
            crm_deal_id, cancel_deadline
        )
        VALUES (?1, ?2, ?3, 'planning', 'planning', ?4, 0, datetime('now', 'subsec'), ?5, ?6)
        "#,
    )
    .bind(&flow_id)
    .bind(&task_id)
    .bind(db_flow_type)
    .bind(flow_config.to_string())
    .bind(deal.id.to_string())
    .bind(deadline.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
    .execute(pool)
    .await?;

    Ok((flow_id, deadline))
}

// ── Helper Functions ─────────────────────────────────────────────────────────

async fn check_intel_status(
    pool: &SqlitePool,
    deal: &CrmDeal,
    entity: &str,
    expected_status: &str,
) -> bool {
    let contact_id = match &deal.crm_contact_id {
        Some(id) => id,
        None => return false,
    };

    match entity {
        "person" => {
            #[derive(sqlx::FromRow)]
            struct IntelRow {
                intelligence_status: Option<String>,
            }
            let status = sqlx::query_as::<_, IntelRow>(
                "SELECT intelligence_status FROM persons WHERE crm_contact_id = ? OR CAST(crm_contact_id AS TEXT) = ? LIMIT 1",
            )
            .bind(contact_id)
            .bind(contact_id.to_string())
            .fetch_optional(pool)
            .await
            .map_err(|e| tracing::warn!("[StageTransition] Person intel query failed: {e}"))
            .ok()
            .flatten()
            .and_then(|r| r.intelligence_status);

            matches!(status.as_deref(), Some(s) if s == expected_status || s == "complete")
        }
        "company" => {
            #[derive(sqlx::FromRow)]
            struct IntelRow {
                intelligence_status: Option<String>,
            }
            let status = sqlx::query_as::<_, IntelRow>(
                "SELECT co.intelligence_status FROM companies co JOIN persons p ON lower(p.company_name) = lower(co.name) WHERE p.crm_contact_id = ? OR CAST(p.crm_contact_id AS TEXT) = ? LIMIT 1",
            )
            .bind(contact_id)
            .bind(contact_id.to_string())
            .fetch_optional(pool)
            .await
            .map_err(|e| tracing::warn!("[StageTransition] Company intel query failed: {e}"))
            .ok()
            .flatten()
            .and_then(|r| r.intelligence_status);

            matches!(status.as_deref(), Some(s) if s == expected_status || s == "complete")
        }
        _ => false,
    }
}

async fn count_pending_tasks(pool: &SqlitePool, deal: &CrmDeal) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&deal.id)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

/// Cancel a pending agent flow if within the cancel window.
pub async fn cancel_agent_flow(pool: &SqlitePool, deal_id: &str) -> Result<bool, anyhow::Error> {
    let result = sqlx::query(
        r#"
        UPDATE agent_flows SET
            status = 'failed',
            last_error = 'Cancelled by user',
            updated_at = datetime('now', 'subsec')
        WHERE crm_deal_id = ?1
          AND status = 'planning'
          AND cancel_deadline > datetime('now', 'subsec')
        "#,
    )
    .bind(deal_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

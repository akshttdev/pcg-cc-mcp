//! CRM Deal Stage Transitions
//!
//! Handles deal stage movement, advance requirements, review task creation,
//! and Phase 1 business report generation.

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use db::{
    db_uuid::DbUuid,
    models::crm_deal::{CreateCrmDeal, CrmDeal},
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use super::{
    crm_deal_automations::{
        generate_deck_background, generate_phase1_business_report, trigger_deep_research_pass2,
        trigger_who_is_research,
    },
    crm_deals::{MoveDealRequest, require_deal_org_access},
};
use crate::{
    DeploymentImpl, error::ApiError, helpers::uuid_params::parse_db_uuid_param,
    middleware::access_control::AccessContext,
};

/// PATCH /crm/deals/:id/stage - Move deal to new stage (drag-drop)
/// When a Sales pipeline deal moves to a Won stage, auto-create a Delivery pipeline deal
pub async fn move_deal_stage(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<MoveDealRequest>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let stage_id = DbUuid::from(data.stage_id);

    // Check if the target stage is a "won" stage
    let target_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &stage_id)
        .await
        .ok();
    let is_won = target_stage
        .as_ref()
        .map(|s| s.is_won.unwrap_or(0) == 1)
        .unwrap_or(false);

    // Move the deal
    let deal = CrmDeal::move_to_stage(pool, &id, &stage_id, data.position).await?;

    // If won, check if this is a sales pipeline deal and auto-create delivery deal
    if is_won {
        if let Some(ref pipeline_id) = deal.crm_pipeline_id {
            if let Ok(pipeline) =
                db::models::crm_pipeline::CrmPipeline::find_by_id(pool, pipeline_id).await
            {
                if pipeline.pipeline_type == "sales" {
                    // Find the delivery pipeline for this organization
                    if let Some(ref deal_org_id) = deal.organization_id {
                        if let Ok(Some(delivery_pipeline)) =
                            db::models::crm_pipeline::CrmPipeline::find_by_type_for_org(
                                pool,
                                deal_org_id,
                                db::models::crm_pipeline::PipelineType::Delivery,
                            )
                            .await
                        {
                            // Check if a delivery deal already exists for this contact
                            let has_delivery_deal =
                                if let Some(ref contact_id) = deal.crm_contact_id {
                                    let contact_deals = CrmDeal::find_by_contact(pool, contact_id)
                                        .await
                                        .unwrap_or_default();
                                    contact_deals.iter().any(|d| {
                                        d.crm_pipeline_id.as_ref() == Some(&delivery_pipeline.id)
                                    })
                                } else {
                                    false
                                };

                            if !has_delivery_deal {
                                // Get the first stage (Onboarding) of the delivery pipeline
                                let delivery_stages =
                                    db::models::crm_pipeline::CrmPipelineStage::find_by_pipeline(
                                        pool,
                                        &delivery_pipeline.id,
                                    )
                                    .await
                                    .unwrap_or_default();

                                let onboarding_stage = delivery_stages.first();

                                // Create delivery deal
                                let _ = CrmDeal::create(
                                    pool,
                                    CreateCrmDeal {
                                        organization_id: deal
                                            .organization_id
                                            .clone()
                                            .unwrap_or_else(|| DbUuid::from(Uuid::nil())),
                                        client_id: deal.client_id.clone(),
                                        crm_contact_id: deal.crm_contact_id.clone(),
                                        crm_pipeline_id: Some(delivery_pipeline.id.clone()),
                                        crm_stage_id: onboarding_stage.map(|s| s.id.clone()),
                                        name: format!("{} - Delivery", deal.name),
                                        description: Some(format!(
                                            "Auto-created from won sales deal: {}",
                                            deal.name
                                        )),
                                        amount: deal.amount,
                                        currency: Some(deal.currency.clone()),
                                        expected_close_date: None,
                                        tags: None,
                                        custom_fields: None,
                                    },
                                )
                                .await;
                            }
                        }
                    }
                }
            }
        }
    }

    // Stage-aware auto-triggers (9-stage Dealflow pipeline)
    let stage_name_lower = target_stage
        .as_ref()
        .map(|s| s.name.to_lowercase())
        .unwrap_or_default();
    let stage_type_lower = target_stage
        .as_ref()
        .and_then(|s| s.stage_type.as_deref())
        .unwrap_or("")
        .to_lowercase();

    // "Intel" stage → Scout: auto-trigger Phase I Who-Is research
    if stage_name_lower == "intel" || stage_type_lower == "intel" {
        let pool_bg = pool.clone();
        let deal_id = deal.id.clone();
        let contact_id = deal.crm_contact_id.clone();
        tokio::spawn(async move {
            trigger_who_is_research(&pool_bg, deal_id, contact_id).await;
        });
    }

    // "Proposal" stage → Astra Pass 2 + Cash chain: deep research then auto-generate proposal
    if stage_name_lower == "proposal" || stage_type_lower == "proposal" {
        if deal.proposal_text.is_none() || deal.proposal_text.as_deref() == Some("") {
            let pool_bg = pool.clone();
            let deal_id = deal.id.clone();
            tokio::spawn(async move {
                trigger_deep_research_pass2(&pool_bg, deal_id).await;
            });
            tracing::info!(
                "Astra Pass 2 + Cash chain auto-triggered for deal {} entering Proposal stage",
                deal.id
            );
        }
    }

    // "Polish" stage → Lux: auto-generate deck if proposal exists and no deck yet
    if stage_name_lower == "polish" || stage_type_lower == "polish" {
        if deal.deck_url.is_none() && deal.proposal_text.is_some() {
            let pool_bg = pool.clone();
            let deal_id = deal.id.clone();
            tokio::spawn(async move {
                generate_deck_background(&pool_bg, deal_id).await;
            });
            tracing::info!(
                "Lux auto-triggered for deal {} entering Polish stage",
                deal.id
            );
        }
    }

    // Stages that need a human review task before advancing
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
            manage_stage_review_tasks(pool, &deal, task_desc, stage_key).await;
            break;
        }
    }

    Ok(Json(ApiResponse::success(deal)))
}

/// GET /crm/deals/:id/advance-requirements - Pre-flight check for advancing a deal
///
/// Returns what is required before the deal can advance to the next stage.
/// Callers should poll this before showing the advance button as enabled.
pub async fn get_advance_requirements(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;

    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    let current_stage_id = match &deal.crm_stage_id {
        Some(s) => s.clone(),
        None => {
            return Ok(Json(serde_json::json!({
                "can_advance": false,
                "blocking_reason": "Deal has no stage assigned",
                "pending_tasks": 0,
                "current_stage": null,
                "next_stage": null,
                "intel_status": null,
            })));
        }
    };

    // Count pending review tasks
    let pending_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Get current stage info
    let current_stage =
        db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &current_stage_id)
            .await
            .map_err(|_| ApiError::NotFound("Current stage not found".to_string()))?;

    let is_closed = current_stage.is_closed.unwrap_or(0) == 1;

    // Look up next stage
    #[derive(sqlx::FromRow)]
    struct StageNameRow {
        name: String,
    }
    let next_stage_name: Option<String> = sqlx::query_as::<_, StageNameRow>(
        "SELECT name FROM crm_pipeline_stages WHERE pipeline_id = ? AND position = ? LIMIT 1",
    )
    .bind(&current_stage.pipeline_id)
    .bind(current_stage.position + 1)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.name);

    // Get intel status for the linked person (if any)
    let intel_status: Option<String> = if let Some(ref contact_id) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct IntelRow {
            intelligence_status: Option<String>,
        }
        // Try crm_contacts.person_id → persons
        let status = sqlx::query_as::<_, IntelRow>(
            "SELECT p.intelligence_status FROM persons p
             JOIN crm_contacts c ON c.person_id = p.id
             WHERE c.id = ? LIMIT 1",
        )
        .bind(contact_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.intelligence_status)
        .flatten();

        if status.is_some() {
            status
        } else {
            // Fallback: persons.crm_contact_id
            sqlx::query_as::<_, IntelRow>(
                "SELECT intelligence_status FROM persons WHERE crm_contact_id = ? LIMIT 1",
            )
            .bind(contact_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.intelligence_status)
            .flatten()
        }
    } else {
        None
    };

    let can_advance = pending_tasks == 0 && !is_closed && next_stage_name.is_some();
    let blocking_reason = if is_closed {
        Some("Deal is at a closed stage".to_string())
    } else if next_stage_name.is_none() {
        Some("Deal is already at the final stage".to_string())
    } else if pending_tasks > 0 {
        Some(format!(
            "{} pending review task(s) must be completed first",
            pending_tasks
        ))
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "can_advance": can_advance,
        "blocking_reason": blocking_reason,
        "pending_tasks": pending_tasks,
        "current_stage": current_stage.name,
        "next_stage": next_stage_name,
        "intel_status": intel_status,
    })))
}

/// POST /crm/deals/:id/advance - Advance deal to next stage after review approval
pub async fn advance_deal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;

    let deal = require_deal_org_access(&access_context, pool, &id).await?;
    let current_stage_id = deal
        .crm_stage_id
        .clone()
        .ok_or_else(|| ApiError::BadRequest("Deal has no stage assigned".to_string()))?;

    // Validate: active review tasks for the CURRENT stage must be done
    let current_stage =
        db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &current_stage_id)
            .await
            .map_err(|_| ApiError::NotFound("Current stage not found".to_string()))?;
    let current_stage_name_lower = current_stage.name.to_lowercase();
    let review_prefix = format!("Review & approve: {}%", current_stage_name_lower);

    let pending_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND (title LIKE ? OR (title LIKE 'Review & approve:%' AND title NOT LIKE 'Review & approve: %')) AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&id)
    .bind(&review_prefix)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if pending_tasks > 0 {
        return Err(ApiError::BadRequest(format!(
            "Cannot advance: {} pending review task(s) must be completed first",
            pending_tasks
        )));
    }

    // F7: Intel Completeness Gate — require operator context + person/company intel before advancing from Intel→BA
    if current_stage_name_lower == "intel" {
        // Check deal.description is not empty (operator context)
        let desc = deal.description.as_deref().unwrap_or("");
        if desc.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "Cannot advance from Intel: deal description (operator context) is required. Add notes about the lead before advancing.".to_string(),
            ));
        }

        // Check person intelligence_status via crm_contact_id (use CAST for BLOB/TEXT compat)
        let person_intel_status: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)]
            struct PIS {
                intelligence_status: Option<String>,
            }
            sqlx::query_as::<_, PIS>(
                "SELECT intelligence_status FROM persons WHERE crm_contact_id = ? OR CAST(crm_contact_id AS TEXT) = ? LIMIT 1"
            )
            .bind(cid)
            .bind(cid.to_string())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.intelligence_status)
        } else {
            None
        };

        if !matches!(
            person_intel_status.as_deref(),
            Some("done") | Some("complete")
        ) {
            return Err(ApiError::BadRequest(format!(
                "Cannot advance from Intel: person intelligence is '{}'. Wait for Scout research to finish.",
                person_intel_status.as_deref().unwrap_or("missing")
            )));
        }

        // Check company intelligence_status via person.company_name (CAST for BLOB/TEXT compat)
        let company_intel_status: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)]
            struct CIS {
                intelligence_status: Option<String>,
            }
            sqlx::query_as::<_, CIS>(
                "SELECT co.intelligence_status FROM companies co JOIN persons p ON lower(p.company_name) = lower(co.name) WHERE p.crm_contact_id = ? OR CAST(p.crm_contact_id AS TEXT) = ? LIMIT 1"
            )
            .bind(cid)
            .bind(cid.to_string())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.intelligence_status)
        } else {
            None
        };

        if !matches!(
            company_intel_status.as_deref(),
            Some("done") | Some("complete")
        ) {
            return Err(ApiError::BadRequest(format!(
                "Cannot advance from Intel: company intelligence is '{}'. Wait for company research to finish.",
                company_intel_status.as_deref().unwrap_or("missing")
            )));
        }
    }

    // Find next stage by position + 1 within the same pipeline
    let next_position = current_stage.position + 1;

    #[derive(sqlx::FromRow)]
    struct NextStageRow {
        id: DbUuid,
    }
    let next_stage = sqlx::query_as::<_, NextStageRow>(
        "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND position = ? LIMIT 1",
    )
    .bind(&current_stage.pipeline_id)
    .bind(next_position)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to find next stage: {}", e)))?
    .ok_or_else(|| ApiError::BadRequest("Deal is already at the final stage".to_string()))?;

    // Move deal to next stage — position 0 (will be sorted by position within stage)
    let deal = CrmDeal::move_to_stage(pool, &id, &next_stage.id, 0).await?;

    // Fire stage-entry hooks for the new stage
    let new_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &next_stage.id)
        .await
        .ok();
    let new_stage_name = new_stage
        .as_ref()
        .map(|s| s.name.to_lowercase())
        .unwrap_or_default();
    let is_won = new_stage
        .as_ref()
        .map(|s| s.is_won.unwrap_or(0) == 1)
        .unwrap_or(false);

    // Auto-create delivery deal on Closed Won (with dedup check)
    if is_won {
        if let Some(ref pipeline_id) = deal.crm_pipeline_id {
            if let Ok(pipeline) =
                db::models::crm_pipeline::CrmPipeline::find_by_id(pool, pipeline_id).await
            {
                if pipeline.pipeline_type == "clients" || pipeline.pipeline_type == "sales" {
                    if let Some(ref deal_org_id) = deal.organization_id {
                        if let Ok(Some(delivery_pipeline)) =
                            db::models::crm_pipeline::CrmPipeline::find_by_type_for_org(
                                pool,
                                deal_org_id,
                                db::models::crm_pipeline::PipelineType::Delivery,
                            )
                            .await
                        {
                            // Dedup: check if a delivery deal with the same name pattern already exists
                            let delivery_deal_name = format!("{} - Delivery", deal.name);
                            let existing_count: i64 = sqlx::query_scalar(
                                "SELECT COUNT(*) FROM crm_deals WHERE crm_pipeline_id = ? AND name = ?",
                            )
                            .bind(&delivery_pipeline.id)
                            .bind(&delivery_deal_name)
                            .fetch_one(pool)
                            .await
                            .unwrap_or(0);

                            if existing_count > 0 {
                                tracing::warn!(
                                    "Skipping delivery deal creation: deal '{}' already exists in pipeline {}",
                                    delivery_deal_name,
                                    delivery_pipeline.id
                                );
                            } else {
                                let delivery_stages =
                                    db::models::crm_pipeline::CrmPipelineStage::find_by_pipeline(
                                        pool,
                                        &delivery_pipeline.id,
                                    )
                                    .await
                                    .unwrap_or_default();
                                if let Some(first_stage) = delivery_stages.first() {
                                    let _ = CrmDeal::create(
                                        pool,
                                        CreateCrmDeal {
                                            organization_id: deal_org_id.clone(),
                                            client_id: deal.client_id.clone(),
                                            crm_contact_id: deal.crm_contact_id.clone(),
                                            crm_pipeline_id: Some(delivery_pipeline.id.clone()),
                                            crm_stage_id: Some(first_stage.id.clone()),
                                            name: delivery_deal_name,
                                            description: Some(format!(
                                                "Auto-created from won deal: {}",
                                                deal.name
                                            )),
                                            amount: deal.amount,
                                            currency: Some(deal.currency.clone()),
                                            expected_close_date: None,
                                            tags: None,
                                            custom_fields: None,
                                        },
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Stage-entry hooks for 9-stage Dealflow pipeline
    let new_stage_type =
        db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &next_stage.id)
            .await
            .ok()
            .and_then(|s| s.stage_type)
            .unwrap_or_default()
            .to_lowercase();

    // Create review task for stages that gate on human approval
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
        if new_stage_name == *stage_key || new_stage_type == *stage_key {
            manage_stage_review_tasks(pool, &deal, task_desc, stage_key).await;
            break;
        }
    }

    // Intel → Scout: Phase I Who-Is research
    if new_stage_name == "intel" || new_stage_type == "intel" {
        let pool_bg = pool.clone();
        let deal_id = deal.id.clone();
        let contact_id = deal.crm_contact_id.clone();
        tokio::spawn(async move {
            trigger_who_is_research(&pool_bg, deal_id, contact_id).await;
        });
    }

    // Business Analysis → Astra: Phase II business report
    if new_stage_name == "business analysis" || new_stage_type == "business_analysis" {
        let pool_bg = pool.clone();
        let deal_id_bg = deal.id.clone();
        let contact_id_bg = deal.crm_contact_id.clone();
        let project_id_bg = deal.project_id.clone();
        tokio::spawn(async move {
            generate_phase1_business_report(&pool_bg, deal_id_bg, contact_id_bg, project_id_bg)
                .await;
        });
    }

    Ok(Json(ApiResponse::success(deal)))
}

/// Create a review task for a deal stage if none exists for this specific stage.
/// Also cancels review tasks from previous stages.
pub async fn manage_stage_review_tasks(
    pool: &sqlx::SqlitePool,
    deal: &CrmDeal,
    description: &str,
    stage_name: &str,
) {
    // Cancel review tasks from previous stages
    let current_prefix = format!("Review & approve: {} —", stage_name);
    let _ = sqlx::query(
        "UPDATE tasks SET status = 'cancelled', updated_at = datetime('now','subsec') WHERE crm_deal_id = ? AND title LIKE 'Review & approve:%' AND title NOT LIKE ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&deal.id)
    .bind(format!("{}%", current_prefix))
    .execute(pool)
    .await;

    // Check if a review task already exists for THIS stage
    let existing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND title LIKE ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&deal.id)
    .bind(format!("{}%", current_prefix))
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if existing_count == 0 {
        let task_id = DbUuid::new();
        let task_title = format!("Review & approve: {} — {}", stage_name, deal.name);
        // Default assignee: first admin user (so tasks show up in My Tasks)
        let default_assignee: Option<String> =
            sqlx::query_scalar::<_, String>("SELECT hex(id) FROM users WHERE is_admin = 1 LIMIT 1")
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .and_then(|hex| {
                    // hex(id) returns 32 chars for a 16-byte UUID BLOB
                    if hex.len() != 32 {
                        return None;
                    }
                    let h = hex.to_lowercase();
                    Some(format!(
                        "{}-{}-{}-{}-{}",
                        &h[..8],
                        &h[8..12],
                        &h[12..16],
                        &h[16..20],
                        &h[20..]
                    ))
                });

        let _ = sqlx::query(
            r#"
            INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, assignee_id, created_at, updated_at)
            VALUES (?, ?, ?, 'todo', ?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))
            "#,
        )
        .bind(&task_id)
        .bind(&task_title)
        .bind(description)
        .bind(&deal.id)
        .bind(&deal.project_id)
        .bind(&default_assignee)
        .execute(pool)
        .await;

        // F8: BA Operator Assignment — assign review task to org-specific operator
        if stage_name == "business analysis" {
            if let Some(ref org_id) = deal.organization_id {
                // Look up organization name
                #[derive(sqlx::FromRow)]
                struct OrgNameRow {
                    name: String,
                }
                let org_name = sqlx::query_as::<_, OrgNameRow>(
                    "SELECT name FROM organizations WHERE id = ? LIMIT 1",
                )
                .bind(org_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .map(|r| r.name);

                let assignee_username = match org_name.as_deref() {
                    Some(n) if n.contains("Sirak") => Some("Sirak"),
                    Some(n) if n.contains("PowerClub") || n.contains("PCG") => Some("Bodhi"),
                    _ => None,
                };

                if let Some(username) = assignee_username {
                    #[derive(sqlx::FromRow)]
                    struct UserIdRow {
                        id: DbUuid,
                    }
                    if let Some(user) = sqlx::query_as::<_, UserIdRow>(
                        "SELECT id FROM users WHERE username = ? LIMIT 1",
                    )
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten()
                    {
                        let _ = sqlx::query(
                            "UPDATE tasks SET assignee_id = ?, updated_at = datetime('now','subsec') WHERE id = ?"
                        )
                        .bind(user.id.to_string())
                        .bind(&task_id)
                        .execute(pool)
                        .await;
                        tracing::info!(
                            "BA review task assigned to {} for deal {}",
                            username,
                            deal.id
                        );
                    }
                }
            }
        }
    }
}

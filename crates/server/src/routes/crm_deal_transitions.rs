//! CRM Deal Stage Transitions
//!
//! Handles deal stage movement, advance requirements, review task creation,
//! and Phase 1 business report generation.

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use db::{db_uuid::DbUuid, models::crm_deal::CrmDeal};
use deployment::Deployment;
use utils::response::ApiResponse;

use super::crm_deals::{MoveDealRequest, require_deal_org_access};
use crate::{
    DeploymentImpl, error::ApiError, helpers::uuid_params::parse_db_uuid_param,
    middleware::access_control::AccessContext,
};

/// PATCH /crm/deals/:id/stage - Move deal to new stage (drag-drop or context menu)
///
/// Unified transition: moves the deal, then runs the StageTransitionProcessor
/// which handles automations, gates, review tasks, and agent scheduling.
pub async fn move_deal_stage(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<MoveDealRequest>,
) -> Result<Json<ApiResponse<crate::stage_transition::TransitionResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;
    let stage_id = DbUuid::from(data.stage_id);

    // Load source and target stages
    let from_stage = deal.crm_stage_id.as_ref().and_then(|sid| {
        futures::executor::block_on(db::models::crm_pipeline::CrmPipelineStage::find_by_id(
            pool, sid,
        ))
        .ok()
    });
    let to_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &stage_id)
        .await
        .map_err(|_| ApiError::NotFound("Target stage not found".to_string()))?;

    // Move the deal in the database
    let deal = CrmDeal::move_to_stage(pool, &id, &stage_id, data.position).await?;

    // Run unified transition processor (automations, gates, agent scheduling)
    let result =
        crate::stage_transition::process_transition(pool, &deal, from_stage.as_ref(), &to_stage)
            .await;

    Ok(Json(ApiResponse::success(result)))
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
        .and_then(|r| r.intelligence_status);

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
            .and_then(|r| r.intelligence_status)
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

    // F7: Intel Completeness Gate — collect ALL validation failures, return together
    if current_stage_name_lower == "intel" {
        let mut warnings: Vec<serde_json::Value> = Vec::new();

        // Check deal.description is not empty (operator context)
        let desc = deal.description.as_deref().unwrap_or("");
        if desc.trim().is_empty() {
            warnings.push(serde_json::json!({
                "field": "description",
                "message": "Deal description (operator context) is required. Add notes about the lead before advancing."
            }));
        }

        // Check person intelligence_status via crm_contact_id (use CAST for BLOB/TEXT compat)
        let person_intel_status: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)]
            struct PersonIntelStatus {
                intelligence_status: Option<String>,
            }
            sqlx::query_as::<_, PersonIntelStatus>(
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
            warnings.push(serde_json::json!({
                "field": "person_intelligence",
                "message": format!("Person intelligence is '{}'. Wait for Scout research to finish.", person_intel_status.as_deref().unwrap_or("missing"))
            }));
        }

        // Check company intelligence_status via person.company_name (CAST for BLOB/TEXT compat)
        let company_intel_status: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)]
            struct CompanyIntelStatus {
                intelligence_status: Option<String>,
            }
            sqlx::query_as::<_, CompanyIntelStatus>(
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
            warnings.push(serde_json::json!({
                "field": "company_intelligence",
                "message": format!("Company intelligence is '{}'. Wait for company research to finish.", company_intel_status.as_deref().unwrap_or("missing"))
            }));
        }

        if !warnings.is_empty() {
            let fields: Vec<String> = warnings
                .iter()
                .filter_map(|w| w["field"].as_str().map(String::from))
                .collect();
            let summary = format!(
                "Cannot advance from Intel: {} validation(s) failed ({})",
                warnings.len(),
                fields.join(", ")
            );
            let body = serde_json::json!({
                "success": false,
                "message": summary,
                "warnings": warnings,
            });
            return Err(ApiError::ValidationFailed(body));
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

    // Load the target stage for the processor
    let to_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &next_stage.id)
        .await
        .map_err(|_| ApiError::NotFound("Next stage not found".to_string()))?;

    // Run unified transition processor (automations, gates, agent scheduling)
    let result =
        crate::stage_transition::process_transition(pool, &deal, Some(&current_stage), &to_stage)
            .await;

    Ok(Json(ApiResponse::success(result.deal)))
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
    if let Err(e) = sqlx::query(
        "UPDATE tasks SET status = 'cancelled', updated_at = datetime('now','subsec') WHERE crm_deal_id = ? AND title LIKE 'Review & approve:%' AND title NOT LIKE ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&deal.id)
    .bind(format!("{}%", current_prefix))
    .execute(pool)
    .await
    {
        tracing::error!("[manage_stage_review_tasks] Failed to cancel old review tasks: {}", e);
    }

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

        if let Err(e) = sqlx::query(
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
        .await
        {
            tracing::error!("[manage_stage_review_tasks] Failed to create review task: {}", e);
            return;
        }

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
                        if let Err(e) = sqlx::query(
                            "UPDATE tasks SET assignee_id = ?, updated_at = datetime('now','subsec') WHERE id = ?"
                        )
                        .bind(user.id.to_string())
                        .bind(&task_id)
                        .execute(pool)
                        .await
                        {
                            tracing::error!("[manage_stage_review_tasks] Failed to assign BA review task: {}", e);
                        }
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

/// POST /crm/deals/:id/cancel-agent - Cancel a pending agent flow within the cancel window
pub async fn cancel_deal_agent(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;

    let cancelled = crate::stage_transition::cancel_agent_flow(pool, id.as_str())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to cancel agent: {}", e)))?;

    if cancelled {
        Ok(Json(ApiResponse::success(serde_json::json!({
            "cancelled": true,
            "message": "Agent flow cancelled"
        }))))
    } else {
        Err(ApiError::BadRequest(
            "No cancellable agent flow found (may have already started or expired)".to_string(),
        ))
    }
}

/// POST /crm/deals/:id/approve-agent - Skip the cancel window and start the agent immediately
pub async fn approve_deal_agent(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    require_deal_org_access(&access_context, pool, &id).await?;

    // Clear the cancel_deadline so the worker picks it up immediately
    let result = sqlx::query(
        r#"
        UPDATE agent_flows SET
            cancel_deadline = NULL,
            updated_at = datetime('now', 'subsec')
        WHERE crm_deal_id = ?1
          AND status = 'planning'
          AND cancel_deadline > datetime('now', 'subsec')
        "#,
    )
    .bind(id.as_str())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to approve agent: {}", e)))?;

    if result.rows_affected() > 0 {
        Ok(Json(ApiResponse::success(serde_json::json!({
            "approved": true,
            "message": "Agent will start on next poll cycle"
        }))))
    } else {
        Err(ApiError::BadRequest(
            "No pending agent flow found to approve".to_string(),
        ))
    }
}

/// POST /crm/deals/:id/retrigger-agent - Re-trigger the agent for the deal's current stage
///
/// Used after cancelling or when an agent flow failed. Creates a new agent flow
/// for the current stage's configured agent, same as the initial stage transition.
pub async fn retrigger_deal_agent(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    // Get the current stage config to find the assigned agent
    let stage_id = deal
        .crm_stage_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Deal has no stage assigned".into()))?;
    let stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, stage_id)
        .await
        .map_err(|_| ApiError::NotFound("Stage not found".into()))?;

    let config: Option<crate::stage_transition::StageConfig> = stage
        .stage_config
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let config =
        config.ok_or_else(|| ApiError::BadRequest("Current stage has no configuration".into()))?;

    let agent = config
        .assigned_agent
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Current stage has no agent assigned".into()))?;

    let flow_type = crate::stage_transition::agent_default_flow_type(agent);

    // Check there's no already-running flow for this deal
    let active_flows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_flows WHERE crm_deal_id = ?1 AND status IN ('planning', 'executing')",
    )
    .bind(id.as_str())
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if active_flows > 0 {
        return Err(ApiError::BadRequest(
            "Deal already has an active agent flow — cancel it first".into(),
        ));
    }

    // Schedule the new agent flow (same as stage_transition::schedule_agent_flow)
    let (flow_id, deadline) = crate::stage_transition::schedule_agent_flow(
        pool,
        &deal,
        agent,
        &flow_type,
        config.cancel_window_secs,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to schedule agent: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "retriggered": true,
        "agent_flow_id": flow_id,
        "agent": agent,
        "cancel_deadline": deadline.to_rfc3339(),
        "message": format!("Re-triggered {} agent for current stage", agent),
    }))))
}

/// GET /crm/deals/:id/agent-flows - List agent flows for a deal with events
pub async fn get_deal_agent_flows(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "deal ID")?;
    let _deal = require_deal_org_access(&access_context, pool, &id).await?;

    let flows = db::models::agent_flow::AgentFlow::find_by_deal(pool, id.as_str())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch agent flows: {}", e)))?;

    let mut result = Vec::new();
    for flow in flows {
        let events = db::models::agent_flow_event::AgentFlowEvent::find_by_flow(pool, &flow.id)
            .await
            .unwrap_or_default();

        let event_values: Vec<serde_json::Value> = events
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id.to_string(),
                    "event_type": e.event_type.to_string(),
                    "event_data": e.event_data,
                    "created_at": e.created_at.to_rfc3339(),
                })
            })
            .collect();

        result.push(serde_json::json!({
            "id": flow.id.to_string(),
            "status": flow.status.to_string(),
            "flow_type": flow.flow_type.to_string(),
            "flow_config": flow.flow_config,
            "current_phase": flow.current_phase.to_string(),
            "retry_count": flow.retry_count,
            "last_error": flow.last_error,
            "cancel_deadline": flow.cancel_deadline.map(|d| d.to_rfc3339()),
            "execution_started_at": flow.execution_started_at.map(|d| d.to_rfc3339()),
            "execution_completed_at": flow.execution_completed_at.map(|d| d.to_rfc3339()),
            "created_at": flow.created_at.to_rfc3339(),
            "updated_at": flow.updated_at.to_rfc3339(),
            "events": event_values,
        }));
    }

    Ok(Json(ApiResponse::success(result)))
}

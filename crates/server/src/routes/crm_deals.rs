//! CRM Deal Management Routes
//!
//! Handles deal CRUD operations and Kanban board data.

use axum::{
    Extension,
    extract::{Path, Query, State},
    routing::{get, patch, post, delete},
    Json, Router,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};
use db::db_uuid::DbUuid;
use db::models::crm_deal::{
    CrmDeal, CreateCrmDeal, KanbanBoardData, UpdateCrmDeal, CrmDealWithContact,
};
use db::models::user::Organization;

#[derive(Debug, Serialize)]
pub struct DealTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct DealKnowledgeSource {
    pub source_type: String,
    pub source_title: String,
    pub source_summary: Option<String>,
    pub coverage_score: f64,
    pub last_refreshed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrmDealRich {
    #[serde(flatten)]
    pub deal: CrmDealWithContact,
    pub company_id: Option<String>,
    pub company_intelligence_summary: Option<String>,
    pub company_intelligence_status: Option<String>,
    pub company_intelligence_confidence: Option<f64>,
    pub company_intelligence_last_run_at: Option<String>,
    pub tasks: Vec<DealTask>,
    pub knowledge_sources: Vec<DealKnowledgeSource>,
}

#[derive(Debug, Deserialize)]
pub struct ListDealsQuery {
    pub organization_id: Option<Uuid>,

    pub pipeline_id: Option<Uuid>,
    pub stage_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct MoveDealRequest {
    pub stage_id: Uuid,
    pub position: i32,
}

/// Verify the user has access to a deal's organization.
/// Loads the deal, checks its organization_id, then verifies org membership.
async fn require_deal_org_access(
    access: &AccessContext,
    pool: &sqlx::SqlitePool,
    deal_id: &DbUuid,
) -> Result<CrmDeal, ApiError> {
    let deal = CrmDeal::find_by_id(pool, deal_id).await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    if access.is_admin {
        return Ok(deal);
    }
    if let Some(ref org_id) = deal.organization_id {
        let role = Organization::get_user_role(pool, org_id.as_str(), access.user_id.as_str()).await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this deal's organization".into()));
        }
    }
    Ok(deal)
}

/// Check that the user is a member of the given organization.
async fn require_org_membership(
    access: &AccessContext,
    pool: &sqlx::SqlitePool,
    org_id: &str,
) -> Result<(), ApiError> {
    if access.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access.user_id.as_str()).await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    if role.is_none() {
        return Err(ApiError::Forbidden("Not a member of this organization".into()));
    }
    Ok(())
}

/// GET /crm/deals - List deals with optional filters
async fn list_deals(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListDealsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmDeal>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Org authorization: check membership on the organization_id filter (or pipeline's org)
    if let Some(org_id) = &query.organization_id {
        require_org_membership(&access_context, pool, &org_id.to_string()).await?;
    } else if let Some(pipeline_id) = &query.pipeline_id {
        let pipeline = db::models::crm_pipeline::CrmPipeline::find_by_id(pool, &DbUuid::from(*pipeline_id)).await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
        if let Some(ref org_id) = pipeline.organization_id {
            require_org_membership(&access_context, pool, org_id.as_str()).await?;
        }
    }

    let deals = if let Some(pipeline_id) = query.pipeline_id {
        CrmDeal::find_by_pipeline(pool, &DbUuid::from(pipeline_id)).await?
    } else if let Some(stage_id) = query.stage_id {
        CrmDeal::find_by_stage(pool, &DbUuid::from(stage_id)).await?
    } else if let Some(contact_id) = query.contact_id {
        CrmDeal::find_by_contact(pool, &DbUuid::from(contact_id)).await?
    } else if let Some(org_id) = query.organization_id {
        CrmDeal::find_by_organization(pool, &DbUuid::from(org_id)).await?
    } else {
        return Err(ApiError::BadRequest(
            "Must provide organization_id, pipeline_id, stage_id, or contact_id".to_string(),

        ));
    };

    Ok(Json(ApiResponse::success(deals)))
}

/// GET /crm/deals/enriched?organization_id=... - List enriched deals (with person_id, contact info) for an org
async fn list_enriched_deals(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListDealsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmDealWithContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_id = query.organization_id.ok_or_else(|| ApiError::BadRequest("organization_id required".into()))?;
    require_org_membership(&access_context, pool, &org_id.to_string()).await?;
    let raw_deals = CrmDeal::find_by_organization(pool, &DbUuid::from(org_id)).await?;
    let mut enriched: Vec<CrmDealWithContact> = Vec::with_capacity(raw_deals.len());
    for deal in raw_deals {
        let contact_info = if let Some(ref cid) = deal.crm_contact_id {
            db::models::crm_contact::CrmContact::find_by_id(pool, cid).await.ok()
        } else { None };

        let person_id: Option<DbUuid> = if let Some(ref cid) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)] struct Row { id: DbUuid }
            sqlx::query_as::<_, Row>("SELECT id FROM persons WHERE crm_contact_id = ? LIMIT 1")
                .bind(cid).fetch_optional(pool).await.ok().flatten().map(|r| r.id)
        } else { None };

        let report_id = if let Some(ref pid) = person_id {
            CrmDeal::report_id_for_person_pub(pool, pid).await
        } else { None };

        let (project_name, task_total, task_done, deliverable_count) =
            if let Some(ref pid) = deal.project_id {
                CrmDeal::fetch_project_stats_pub(pool, pid).await
            } else { (None, 0, 0, 0) };

        let (intelligence_status, intelligence_summary, intelligence_confidence, research_pass_count,
             report_status, report_review_status, review_task_id, review_task_status, review_task_assignee,
        ) = CrmDeal::fetch_intel_data_pub(pool, &deal.id, person_id.as_ref()).await;

        let (company_intelligence_status, company_id, company_intelligence_summary) =
            CrmDeal::fetch_company_intel_status(
                pool, contact_info.as_ref().and_then(|c| c.company_name.as_deref()),
            ).await;

        enriched.push(CrmDealWithContact {
            contact_name: contact_info.as_ref().and_then(|c| c.full_name.clone()),
            contact_email: contact_info.as_ref().and_then(|c| c.email.clone()),
            contact_company: contact_info.as_ref().and_then(|c| c.company_name.clone()),
            contact_avatar_url: contact_info.as_ref().and_then(|c| c.avatar_url.clone()),
            project_name,
            task_total,
            task_done,
            deliverable_count,
            person_id,
            report_id,
            intelligence_status,
            intelligence_summary,
            intelligence_confidence,
            research_pass_count,
            report_status,
            report_review_status,
            review_task_id,
            review_task_status,
            review_task_assignee,
            company_intelligence_status,
            company_intelligence_summary,
            company_id,
            deal,
        });
    }
    Ok(Json(ApiResponse::success(enriched)))
}

/// GET /crm/deals/kanban/:pipeline_id - Get Kanban board data
async fn get_kanban_data(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<ApiResponse<KanbanBoardData>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline_id = DbUuid::parse(&pipeline_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;

    // Org authorization: load pipeline to get its org_id
    let pipeline = db::models::crm_pipeline::CrmPipeline::find_by_id(pool, &pipeline_id).await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    if let Some(ref org_id) = pipeline.organization_id {
        require_org_membership(&access_context, pool, org_id.as_str()).await?;
    }

    let kanban_data = CrmDeal::get_kanban_data(pool, &pipeline_id).await?;
    Ok(Json(ApiResponse::success(kanban_data)))
}

/// GET /crm/deals/:id - Get single deal
async fn get_deal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;
    Ok(Json(ApiResponse::success(deal)))
}

/// GET /crm/deals/:id/rich - Get deal with all enriched data for detail panel
async fn get_deal_rich(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDealRich>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;

    // Build CrmDealWithContact inline (mirrors kanban enrichment)
    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    let contact_info = if let Some(ref contact_id) = deal.crm_contact_id {
        db::models::crm_contact::CrmContact::find_by_id(pool, contact_id).await.ok()
    } else {
        None
    };

    // person_id via persons.crm_contact_id (reverse lookup) or email fallback
    let person_id: Option<DbUuid> = {
        let by_contact: Option<DbUuid> = if let Some(ref contact_id) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)] struct Row { id: DbUuid }
            sqlx::query_as::<_, Row>("SELECT id FROM persons WHERE crm_contact_id = ? LIMIT 1")
                .bind(contact_id)
                .fetch_optional(pool).await.ok().flatten().map(|r| r.id)
        } else { None };

        if by_contact.is_some() {
            by_contact
        } else if let Some(email) = contact_info.as_ref().and_then(|c| c.email.as_deref()) {
            #[derive(sqlx::FromRow)] struct Row { id: DbUuid }
            sqlx::query_as::<_, Row>("SELECT id FROM persons WHERE email = ? LIMIT 1")
                .bind(email)
                .fetch_optional(pool).await.ok().flatten().map(|r| r.id)
        } else {
            None
        }
    };

    let report_id = if let Some(ref pid) = person_id {
        CrmDeal::report_id_for_person_pub(pool, pid).await
    } else { None };

    let (project_name, task_total, task_done, deliverable_count) =
        if let Some(ref pid) = deal.project_id {
            CrmDeal::fetch_project_stats_pub(pool, pid).await
        } else { (None, 0, 0, 0) };

    let (
        intelligence_status, intelligence_summary, intelligence_confidence, research_pass_count,
        report_status, report_review_status,
        review_task_id, review_task_status, review_task_assignee,
    ) = CrmDeal::fetch_intel_data_pub(pool, &deal.id, person_id.as_ref()).await;

    // Look up company intelligence status before constructing deal_with_contact
    let company_name_for_intel = contact_info.as_ref().and_then(|c| c.company_name.as_deref());
    let deal_company_intel_status = {
        if let Some(cn) = company_name_for_intel {
            if !cn.is_empty() {
                #[derive(sqlx::FromRow)]
                struct StatusRow { intelligence_status: Option<String> }
                sqlx::query_as::<_, StatusRow>(
                    "SELECT intelligence_status FROM companies WHERE name = ? COLLATE NOCASE LIMIT 1",
                )
                .bind(cn)
                .fetch_optional(pool).await.ok().flatten().and_then(|r| r.intelligence_status)
            } else { None }
        } else { None }
    };

    let deal_with_contact = CrmDealWithContact {
        contact_name: contact_info.as_ref().and_then(|c| c.full_name.clone()),
        contact_email: contact_info.as_ref().and_then(|c| c.email.clone()),
        contact_company: contact_info.as_ref().and_then(|c| c.company_name.clone()),
        contact_avatar_url: contact_info.as_ref().and_then(|c| c.avatar_url.clone()),
        project_name,
        task_total,
        task_done,
        deliverable_count,
        person_id,
        report_id,
        intelligence_status,
        intelligence_summary,
        intelligence_confidence,
        research_pass_count,
        report_status,
        report_review_status,
        review_task_id,
        review_task_status,
        review_task_assignee,
        company_intelligence_status: deal_company_intel_status,
        company_intelligence_summary: None,
        company_id: None,
        deal,
    };

    // Company intel — look up via contact's company_name → companies table
    let (company_id, company_intelligence_summary, company_intelligence_status,
         company_intelligence_confidence, company_intelligence_last_run_at) = {
        let company_name = deal_with_contact.contact_company.as_deref().unwrap_or("");
        if !company_name.is_empty() {
            #[derive(sqlx::FromRow)]
            struct CompanyRow {
                id: Uuid,
                intelligence_summary: Option<String>,
                intelligence_status: Option<String>,
                intelligence_confidence: Option<f64>,
                intelligence_last_run_at: Option<String>,
            }
            match sqlx::query_as::<_, CompanyRow>(
                "SELECT id, intelligence_summary, intelligence_status, intelligence_confidence, intelligence_last_run_at FROM companies WHERE name = ? LIMIT 1"
            )
            .bind(company_name)
            .fetch_optional(pool).await {
                Ok(Some(c)) => (
                    Some(c.id.to_string()),
                    c.intelligence_summary,
                    c.intelligence_status,
                    c.intelligence_confidence,
                    c.intelligence_last_run_at,
                ),
                _ => (None, None, None, None, None),
            }
        } else {
            (None, None, None, None, None)
        }
    };

    // All tasks linked to this deal
    #[derive(sqlx::FromRow)]
    struct TaskRow {
        id: Uuid,
        title: String,
        description: Option<String>,
        status: String,
        created_at: String,
    }
    let tasks: Vec<DealTask> = sqlx::query_as::<_, TaskRow>(
        "SELECT id, title, description, status, created_at FROM tasks WHERE crm_deal_id = ? AND deleted_at IS NULL ORDER BY created_at ASC"
    )
    .bind(&deal_with_contact.deal.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|t| DealTask {
        id: t.id.to_string(),
        title: t.title,
        description: t.description,
        status: t.status,
        created_at: t.created_at,
    })
    .collect();

    // Knowledge sources for the company (owner_type='company', owner_id=company hex)
    let knowledge_sources: Vec<DealKnowledgeSource> = if let Some(ref cid) = company_id {
        #[derive(sqlx::FromRow)]
        struct KsRow {
            source_type: String,
            source_title: String,
            source_summary: Option<String>,
            coverage_score: f64,
            last_refreshed_at: Option<String>,
        }
        // owner_id stored as uppercase hex of UUID bytes
        let owner_id_hex = cid.replace('-', "").to_uppercase();
        sqlx::query_as::<_, KsRow>(
            "SELECT source_type, source_title, source_summary, coverage_score, last_refreshed_at FROM project_knowledge_sources WHERE owner_type='company' AND owner_id=? ORDER BY coverage_score DESC"
        )
        .bind(&owner_id_hex)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|k| DealKnowledgeSource {
            source_type: k.source_type,
            source_title: k.source_title,
            source_summary: k.source_summary,
            coverage_score: k.coverage_score,
            last_refreshed_at: k.last_refreshed_at,
        })
        .collect()
    } else {
        Vec::new()
    };

    Ok(Json(ApiResponse::success(CrmDealRich {
        deal: deal_with_contact,
        company_id,
        company_intelligence_summary,
        company_intelligence_status,
        company_intelligence_confidence,
        company_intelligence_last_run_at,
        tasks,
        knowledge_sources,
    })))
}

/// POST /crm/deals - Create deal
async fn create_deal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(mut data): Json<CreateCrmDeal>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    // Sanitize amount — reject Infinity/NaN
    if let Some(amt) = data.amount {
        if !amt.is_finite() {
            data.amount = None;
        }
    }
    let pool = &deployment.db().pool;
    require_org_membership(&access_context, pool, data.organization_id.as_str()).await?;
    let deal = CrmDeal::create(pool, data).await?;

    // Log deal creation activity
    sqlx::query(
        r#"
        INSERT INTO crm_activities (
            id, organization_id, crm_contact_id, crm_deal_id, activity_type,

            subject, activity_at
        )
        VALUES (?1, ?2, ?3, ?4, 'deal_created', ?5, datetime('now', 'subsec'))
        "#,
    )
    .bind(DbUuid::new())
    .bind(&deal.organization_id)
    .bind(&deal.crm_contact_id)
    .bind(&deal.id)
    .bind(format!("Created deal: {}", deal.name))
    .execute(pool)
    .await?;

    // Auto-trigger stage-entry hooks if deal starts in Intel stage
    if let Some(ref stage_id) = deal.crm_stage_id {
        if let Ok(stage) = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, stage_id).await {
            let sn = stage.name.to_lowercase();
            let st = stage.stage_type.as_deref().unwrap_or("").to_lowercase();
            if sn == "intel" || st == "intel" {
                let pool_bg = pool.clone();
                let deal_id = deal.id.clone();
                let contact_id = deal.crm_contact_id.clone();
                tokio::spawn(async move {
                    trigger_who_is_research(&pool_bg, deal_id, contact_id).await;
                });
                tracing::info!("Scout auto-triggered for new deal {} in Intel stage", deal.id);
            }
        }
    }

    Ok(Json(ApiResponse::success(deal)))
}

/// PATCH /crm/deals/:id - Update deal
async fn update_deal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(mut data): Json<UpdateCrmDeal>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    // Sanitize amount — reject Infinity/NaN
    if let Some(amt) = data.amount {
        if !amt.is_finite() {
            data.amount = None;
        }
    }
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let deal = CrmDeal::update(pool, &id, data).await?;
    Ok(Json(ApiResponse::success(deal)))
}

/// PATCH /crm/deals/:id/stage - Move deal to new stage (drag-drop)
/// When a Sales pipeline deal moves to a Won stage, auto-create a Delivery pipeline deal
async fn move_deal_stage(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<MoveDealRequest>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let stage_id = DbUuid::from(data.stage_id);

    // Check if the target stage is a "won" stage
    let target_stage =
        db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &stage_id).await.ok();
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
                            let has_delivery_deal = if let Some(ref contact_id) = deal.crm_contact_id {
                                let contact_deals =
                                    CrmDeal::find_by_contact(pool, contact_id).await.unwrap_or_default();
                                contact_deals
                                    .iter()
                                    .any(|d| d.crm_pipeline_id.as_ref() == Some(&delivery_pipeline.id))
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
                                        organization_id: deal.organization_id.clone().unwrap_or_else(|| DbUuid::from(Uuid::nil())),
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
            tracing::info!("Astra Pass 2 + Cash chain auto-triggered for deal {} entering Proposal stage", deal.id);
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
            tracing::info!("Lux auto-triggered for deal {} entering Polish stage", deal.id);
        }
    }

    // Stages that need a human review task before advancing
    let review_stages = [
        ("intel", "Review Phase I intelligence (Scout): person profile & company overview"),
        ("business analysis", "Review business report (Astra): pain points, opportunities, recommended services"),
        ("discovery", "Review discovery transcript and confirm proposal readiness"),
        ("proposal", "Review and approve proposal (Cash) before moving to Polish"),
        ("polish", "Review and approve deck (Lux) before presenting to client"),
        ("present", "Confirm invoice sent and await payment confirmation"),
        ("follow up", "Update follow-up status — won, lost, or still in discussion"),
    ];

    for (stage_key, task_desc) in &review_stages {
        if stage_name_lower == *stage_key || stage_type_lower == *stage_key {
            create_review_task_if_needed(pool, &deal, task_desc, stage_key).await;
            break;
        }
    }

    Ok(Json(ApiResponse::success(deal)))

}

/// Auto-trigger "Who Is" research for a deal's contact person and company
async fn trigger_who_is_research(
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
    // // OLD: only checked crm_contacts.person_id — missed migration-linked records
    // let person_id = sqlx::query_as::<_, ContactRow>(
    //     "SELECT person_id FROM crm_contacts WHERE id = ?",
    // )
    // .bind(&contact_id)
    // .fetch_optional(pool)
    // .await
    // .ok()
    // .flatten()
    // .and_then(|r| r.person_id);

    // Direction 1: crm_contacts.person_id
    let person_id_via_contact = sqlx::query_as::<_, ContactRow>(
        "SELECT person_id FROM crm_contacts WHERE id = ?",
    )
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
        sqlx::query_as::<_, PersonRow>(
            "SELECT id FROM persons WHERE crm_contact_id = ? LIMIT 1",
        )
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
    let intel_status = intel.as_ref().map(|i| i.intelligence_status.as_deref().unwrap_or("idle").to_string());
    let intel_company = intel.as_ref().and_then(|i| i.company_name.clone());

    if let Some(ref status) = intel_status {
        if status == "idle" || status == "" {
            // Trigger person research via the same logic as POST /api/persons/:id/research
            tracing::info!("Auto-triggering Who Is research for person {} (deal {})", person_id, deal_id);
            let _ = sqlx::query(
                "UPDATE persons SET intelligence_status = 'queued', updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(&person_id)
            .execute(pool)
            .await;

            // Convert DbUuid to Uuid for intelligence API
            let person_uuid = uuid::Uuid::parse_str(person_id.as_str()).unwrap_or_default();
            let pool2 = pool.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::routes::intelligence::trigger_research_for_person(&pool2, person_uuid).await {
                    tracing::error!("Auto Who Is research failed for person {}: {}", person_uuid, e);
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
            let person_data = sqlx::query_as::<_, PersonNameRow>(
                "SELECT full_name FROM persons WHERE id = ?",
            )
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
                            "INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, created_at, updated_at) VALUES (?, ?, ?, 'in_progress', ?, ?, datetime('now','subsec'), datetime('now','subsec'))",
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
                        "INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, created_at, updated_at) VALUES (?, ?, ?, 'in_progress', ?, ?, datetime('now','subsec'), datetime('now','subsec'))",
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
            tracing::info!("Auto-triggering company research for '{}' ({})", company_name, company.id);
            let company_uuid = uuid::Uuid::parse_str(company.id.as_str()).unwrap_or_default();
            crate::routes::intelligence::run_company_research_direct(
                pool, company_uuid, company_name, None,
            )
            .await;
        }
    }
}

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
        tracing::info!("Phase 1 report already exists for deal {}, skipping", deal_id);
        return;
    }

    let Some(contact_id) = contact_id else { return };

    // Resolve person via persons.crm_contact_id (reverse lookup — crm_contacts has no person_id col)
    #[derive(sqlx::FromRow)]
    struct PersonDataRow {
        id: DbUuid,
        company_name: Option<String>,
    }
    let contact_company: Option<String> = {
        #[derive(sqlx::FromRow)] struct CRow { company_name: Option<String> }
        sqlx::query_as::<_, CRow>("SELECT company_name FROM crm_contacts WHERE id = ? LIMIT 1")
            .bind(&contact_id)
            .fetch_optional(pool).await.ok().flatten().and_then(|r| r.company_name)
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
    let company_name = person_row.as_ref().and_then(|p| p.company_name.clone()).or(contact_company);

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
    } else { None };

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
    } else { None };

    let contact_name = person_intel.as_ref().and_then(|p| p.full_name.clone()).unwrap_or_default();
    let company_display = company_name.as_deref().unwrap_or("Unknown Company");

    let person_summary = person_intel.as_ref().and_then(|p| p.intelligence_summary.clone());
    let company_summary = company_intel.as_ref().and_then(|c| c.intelligence_summary.clone());

    let executive_summary = match (person_summary.as_deref(), company_summary.as_deref()) {
        (Some(ps), Some(cs)) => Some(format!(
            "## Person Intelligence\n{}\n\n## Company Intelligence\n{}", ps, cs
        )),
        (Some(ps), None) => Some(format!("## Person Intelligence\n{}", ps)),
        (None, Some(cs)) => Some(format!("## Company Intelligence\n{}", cs)),
        (None, None) => None,
    };

    let individual_profile = if let Some(ref pi) = person_intel {
        let mut parts = Vec::new();
        if let Some(ref name) = pi.full_name { parts.push(format!("**Name:** {}", name)); }
        if let Some(ref email) = pi.email { parts.push(format!("**Email:** {}", email)); }
        if let Some(ref title) = pi.job_title { parts.push(format!("**Title:** {}", title)); }
        if let Some(ref summary) = pi.intelligence_summary { parts.push(format!("\n{}", summary)); }
        if parts.is_empty() { None } else { Some(parts.join("\n")) }
    } else { None };

    let company_overview = company_intel.as_ref().map(|c| {
        let mut parts = Vec::new();
        if let Some(ref industry) = c.industry { parts.push(format!("**Industry:** {}", industry)); }
        if let Some(ref desc) = c.description { parts.push(format!("**Overview:** {}", desc)); }
        if let Some(ref summary) = c.intelligence_summary { parts.push(format!("\n{}", summary)); }
        parts.join("\n")
    });

    let title = format!("Phase 1 Business Analysis: {} / {}", contact_name, company_display);
    // Uses uuid::Uuid for BLOB-column binding (business_reports.id/crm_deal_id are BLOB)
    let person_uuid = person_intel.as_ref().and_then(|p| uuid::Uuid::parse_str(p.id.as_str()).ok());
    let company_uuid = company_intel.as_ref().and_then(|c| uuid::Uuid::parse_str(c.id.as_str()).ok());
    let deal_uuid = uuid::Uuid::parse_str(deal_id.as_str()).ok();

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
        Ok(_) => tracing::info!("Phase 1 business report {} created for deal {}", report_id, deal_id),
        Err(e) => tracing::error!("Failed to create Phase 1 report for deal {}: {}", deal_id, e),
    }

    // Mark Phase 1 research tasks as done
    let _ = sqlx::query(
        "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec') WHERE crm_deal_id = ? AND title LIKE 'Phase 1 Research:%' AND status = 'in_progress' AND deleted_at IS NULL",
    )
    .bind(&deal_id)
    .execute(pool)
    .await;
}

/// Create a review task for a deal stage if none exists for this specific stage.
/// Also cancels review tasks from previous stages.
async fn create_review_task_if_needed(pool: &sqlx::SqlitePool, deal: &CrmDeal, description: &str, stage_name: &str) {
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
        let default_assignee: Option<String> = sqlx::query_scalar::<_, String>(
            "SELECT hex(id) FROM users WHERE is_admin = 1 LIMIT 1"
        ).fetch_optional(pool).await.ok().flatten()
        .map(|hex| {
            let h = hex.to_lowercase();
            format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
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
                struct OrgNameRow { name: String }
                let org_name = sqlx::query_as::<_, OrgNameRow>(
                    "SELECT name FROM organizations WHERE id = ? LIMIT 1"
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
                    struct UserIdRow { id: DbUuid }
                    if let Some(user) = sqlx::query_as::<_, UserIdRow>(
                        "SELECT id FROM users WHERE username = ? LIMIT 1"
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
                        tracing::info!("BA review task assigned to {} for deal {}", username, deal.id);
                    }
                }
            }
        }
    }
}

/// GET /crm/deals/:id/advance-requirements - Pre-flight check for advancing a deal
///
/// Returns what is required before the deal can advance to the next stage.
/// Callers should poll this before showing the advance button as enabled.
async fn get_advance_requirements(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;

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
    let current_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &current_stage_id)
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
        struct IntelRow { intelligence_status: Option<String> }
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
        Some(format!("{} pending review task(s) must be completed first", pending_tasks))
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
///
/// Auth: This handler is behind the `require_auth` middleware layer applied to all
/// `protected_routes` in `routes/mod.rs`, which rejects unauthenticated requests
/// with 401. Per-deal ownership checks are not performed here (consistent with all
/// other CRM deal handlers: create, update, move_deal_stage, delete).
async fn advance_deal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;

    let deal = require_deal_org_access(&access_context, pool, &id).await?;
    let current_stage_id = deal.crm_stage_id.clone()
        .ok_or_else(|| ApiError::BadRequest("Deal has no stage assigned".to_string()))?;

    // Validate: active review tasks for the CURRENT stage must be done
    let current_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &current_stage_id).await
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
        return Err(ApiError::BadRequest(
            format!("Cannot advance: {} pending review task(s) must be completed first", pending_tasks),
        ));
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
            struct PIS { intelligence_status: Option<String> }
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

        if !matches!(person_intel_status.as_deref(), Some("done") | Some("complete")) {
            return Err(ApiError::BadRequest(
                format!("Cannot advance from Intel: person intelligence is '{}'. Wait for Scout research to finish.",
                    person_intel_status.as_deref().unwrap_or("missing"))
            ));
        }

        // Check company intelligence_status via person.company_name (CAST for BLOB/TEXT compat)
        let company_intel_status: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)]
            struct CIS { intelligence_status: Option<String> }
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

        if !matches!(company_intel_status.as_deref(), Some("done") | Some("complete")) {
            return Err(ApiError::BadRequest(
                format!("Cannot advance from Intel: company intelligence is '{}'. Wait for company research to finish.",
                    company_intel_status.as_deref().unwrap_or("missing"))
            ));
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
    let new_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &next_stage.id).await.ok();
    let new_stage_name = new_stage.as_ref().map(|s| s.name.to_lowercase()).unwrap_or_default();
    let is_won = new_stage.as_ref().map(|s| s.is_won.unwrap_or(0) == 1).unwrap_or(false);

    // Auto-create delivery deal on Closed Won (with dedup check)
    if is_won {
        if let Some(ref pipeline_id) = deal.crm_pipeline_id {
            if let Ok(pipeline) = db::models::crm_pipeline::CrmPipeline::find_by_id(pool, pipeline_id).await {
                if pipeline.pipeline_type == "clients" || pipeline.pipeline_type == "sales" {
                    if let Some(ref deal_org_id) = deal.organization_id {
                        if let Ok(Some(delivery_pipeline)) =
                            db::models::crm_pipeline::CrmPipeline::find_by_type_for_org(
                                pool, deal_org_id,
                                db::models::crm_pipeline::PipelineType::Delivery,
                            ).await
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
                                    delivery_deal_name, delivery_pipeline.id
                                );
                            } else {
                                let delivery_stages =
                                    db::models::crm_pipeline::CrmPipelineStage::find_by_pipeline(pool, &delivery_pipeline.id)
                                        .await
                                        .unwrap_or_default();
                                if let Some(first_stage) = delivery_stages.first() {
                                    let _ = CrmDeal::create(pool, CreateCrmDeal {
                                        organization_id: deal_org_id.clone(),
                                        client_id: deal.client_id.clone(),
                                        crm_contact_id: deal.crm_contact_id.clone(),
                                        crm_pipeline_id: Some(delivery_pipeline.id.clone()),
                                        crm_stage_id: Some(first_stage.id.clone()),
                                        name: delivery_deal_name,
                                        description: Some(format!("Auto-created from won deal: {}", deal.name)),
                                        amount: deal.amount,
                                        currency: Some(deal.currency.clone()),
                                        expected_close_date: None,
                                        tags: None,
                                        custom_fields: None,
                                    }).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Stage-entry hooks for 9-stage Dealflow pipeline
    let new_stage_type = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &next_stage.id)
        .await.ok().and_then(|s| s.stage_type).unwrap_or_default().to_lowercase();

    // Create review task for stages that gate on human approval
    let review_stages = [
        ("intel", "Review Phase I intelligence (Scout): person profile & company overview"),
        ("business analysis", "Review business report (Astra): pain points, opportunities, recommended services"),
        ("discovery", "Review discovery transcript and confirm proposal readiness"),
        ("proposal", "Review and approve proposal (Cash) before moving to Polish"),
        ("polish", "Review and approve deck (Lux) before presenting to client"),
        ("present", "Confirm invoice sent and await payment confirmation"),
        ("follow up", "Update follow-up status — won, lost, or still in discussion"),
    ];
    for (stage_key, task_desc) in &review_stages {
        if new_stage_name == *stage_key || new_stage_type == *stage_key {
            create_review_task_if_needed(pool, &deal, task_desc, stage_key).await;
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
            generate_phase1_business_report(&pool_bg, deal_id_bg, contact_id_bg, project_id_bg).await;
        });
    }

    Ok(Json(ApiResponse::success(deal)))
}

/// GET /organizations/:org_id/crm/deals - List deals for an organization
async fn list_org_deals(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<CrmDeal>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_id = DbUuid::parse(&org_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_org_membership(&access_context, pool, org_id.as_str()).await?;
    let deals = CrmDeal::find_by_organization(pool, &org_id).await?;
    Ok(Json(ApiResponse::success(deals)))
}


/// DELETE /crm/deals/:id - Delete deal
async fn delete_deal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_deal_org_access(&access_context, pool, &id).await?;
    CrmDeal::delete(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub organization_id: Uuid,

    pub pipeline_id: Option<Uuid>,
}

#[derive(Debug, serde::Serialize)]
pub struct PipelineMetrics {
    pub pipeline_id: Option<Uuid>,
    pub total_deals: i64,
    pub total_value: f64,
    pub weighted_value: f64,
    pub avg_deal_size: f64,
    pub win_rate: f64,
    pub deals_by_stage: Vec<StageMetric>,
    pub monthly_summary: Vec<MonthlySummary>,
}

#[derive(Debug, serde::Serialize)]
pub struct StageMetric {
    pub stage_id: String,
    pub stage_name: String,
    pub count: i64,
    pub total_value: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct MonthlySummary {
    pub month: String,
    pub new_deals: i64,
    pub won_deals: i64,
    pub lost_deals: i64,
    pub total_value: f64,
}

/// GET /crm/deals/metrics - Pipeline metrics
async fn get_metrics(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<MetricsQuery>,
) -> Result<Json<ApiResponse<PipelineMetrics>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_membership(&access_context, pool, &query.organization_id.to_string()).await?;

    // Get all deals for the organization (optionally filtered by pipeline)
    let deals = if let Some(pipeline_id) = query.pipeline_id {
        CrmDeal::find_by_pipeline(pool, &DbUuid::from(pipeline_id)).await?
    } else {
        CrmDeal::find_by_organization(pool, &DbUuid::from(query.organization_id)).await?
    };

    let total_deals = deals.len() as i64;
    let total_value: f64 = deals.iter().filter_map(|d| d.amount).sum();

    let weighted_value: f64 = deals
        .iter()
        .map(|d| d.amount.unwrap_or(0.0) * (d.probability as f64 / 100.0))
        .sum();

    let avg_deal_size = if total_deals > 0 {
        total_value / total_deals as f64
    } else {
        0.0
    };

    // Win rate: deals_won / (deals_won + deals_lost)
    let mut won_count = 0i64;
    let mut lost_count = 0i64;

    // Get stage info for each deal to determine won/lost
    for deal in &deals {
        if let Some(ref stage_id) = deal.crm_stage_id {
            if let Ok(stage) =
                db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, stage_id).await
            {
                if stage.is_closed.unwrap_or(0) == 1 {
                    if stage.is_won.unwrap_or(0) == 1 {
                        won_count += 1;
                    } else {
                        lost_count += 1;
                    }
                }
            }
        }
    }

    let win_rate = if won_count + lost_count > 0 {
        won_count as f64 / (won_count + lost_count) as f64
    } else {
        0.0
    };

    // Deals by stage
    let mut stage_map: std::collections::HashMap<String, (String, i64, f64)> =
        std::collections::HashMap::new();
    for deal in &deals {
        let stage_id = deal
            .crm_stage_id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let entry = stage_map
            .entry(stage_id.clone())
            .or_insert_with(|| (deal.stage.clone(), 0, 0.0));
        entry.1 += 1;
        entry.2 += deal.amount.unwrap_or(0.0);
    }

    let deals_by_stage: Vec<StageMetric> = stage_map
        .into_iter()
        .map(|(stage_id, (stage_name, count, total_value))| StageMetric {
            stage_id,
            stage_name,
            count,
            total_value,
        })
        .collect();

    // Monthly summary (last 6 months)
    let monthly_summary = Vec::new(); // Simplified - could do SQL aggregation

    Ok(Json(ApiResponse::success(PipelineMetrics {
        pipeline_id: query.pipeline_id,
        total_deals,
        total_value,
        weighted_value,
        avg_deal_size,
        win_rate,
        deals_by_stage,
        monthly_summary,
    })))
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
                tracing::warn!("[LLM] OpenAI returned {}, falling back to Anthropic", resp.status());
            }
            Err(e) => {
                tracing::warn!("[LLM] OpenAI request failed: {}, falling back to Anthropic", e);
            }
        }
    }

    // Fallback to Anthropic
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| ApiError::BadRequest("No LLM API key configured (tried OPENAI_API_KEY, ANTHROPIC_API_KEY)".into()))?;

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

    let val: serde_json::Value = resp.json().await
        .map_err(|e| ApiError::BadRequest(format!("LLM response parse error: {}", e)))?;

    let text = val["content"].as_array()
        .and_then(|a| a.iter().find(|c| c["type"] == "text"))
        .and_then(|c| c["text"].as_str())
        .unwrap_or("LLM generation failed")
        .to_string();

    tracing::info!("[LLM] Anthropic claude-opus-4-6 response received");
    Ok(text)
}

// ── F12: Astra Pass 2 — enhance business report with discovery context, then chain Cash ──
async fn trigger_deep_research_pass2(pool: &sqlx::SqlitePool, deal_id: DbUuid) {
    // 1. Fetch deal
    let deal = match CrmDeal::find_by_id(pool, &deal_id).await {
        Ok(d) => d,
        Err(e) => { tracing::error!("Astra Pass 2: failed to fetch deal {}: {}", deal_id, e); return; }
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
    struct TransRow { summary: Option<String>, transcript_text: Option<String> }
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
        struct PI { intelligence_summary: Option<String>, full_name: Option<String> }
        sqlx::query_as::<_, PI>("SELECT intelligence_summary, full_name FROM persons WHERE crm_contact_id = ? LIMIT 1")
            .bind(cid).fetch_optional(pool).await.ok().flatten()
            .and_then(|p| p.intelligence_summary.map(|s| format!("**{}**\n{}", p.full_name.unwrap_or_default(), s)))
    } else { None };

    let company_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)]
        struct CI { intelligence_summary: Option<String>, name: Option<String> }
        sqlx::query_as::<_, CI>(
            "SELECT c.intelligence_summary, c.name FROM companies c JOIN persons p ON lower(p.company_name) = lower(c.name) WHERE p.crm_contact_id = ? LIMIT 1"
        ).bind(cid).fetch_optional(pool).await.ok().flatten()
        .and_then(|c| c.intelligence_summary.map(|s| format!("**{}**\n{}", c.name.unwrap_or_default(), s)))
    } else { None };

    // 5. Build Astra prompt
    let existing_report_text = report.as_ref()
        .and_then(|r| r.executive_summary.clone())
        .unwrap_or_else(|| "No existing report.".to_string());

    let discovery_text = if transcripts.is_empty() {
        "No discovery transcripts available.".to_string()
    } else {
        transcripts.iter().enumerate().map(|(i, t)| {
            let content = t.summary.as_deref()
                .or(t.transcript_text.as_deref())
                .unwrap_or("(empty)");
            format!("### Transcript {}\n{}", i + 1, content)
        }).collect::<Vec<_>>().join("\n\n")
    };

    let mut context_parts = vec![
        format!("## Existing Business Report\n{}", existing_report_text),
        format!("## Discovery Transcripts\n{}", discovery_text),
    ];
    if let Some(ref pi) = person_intel { context_parts.push(format!("## Person Intelligence\n{}", pi)); }
    if let Some(ref ci) = company_intel { context_parts.push(format!("## Company Intelligence\n{}", ci)); }
    if let Some(ref desc) = deal.description { context_parts.push(format!("## Deal Notes\n{}", desc)); }

    let astra_system = "You are Astra, a business intelligence analyst at PowerClub Global. Your task is to enhance an existing business analysis report with new discovery context from client conversations. Synthesize all available intelligence into a comprehensive, actionable business report. Include: Executive Summary, Client Pain Points, Opportunities, Recommended Services (with estimated value ranges), Competitive Landscape, and Strategic Recommendations. Be specific and data-driven.";
    let user_msg = format!("Enhance this business report with the discovery context below. Deal: {}\n\n{}", deal.name, context_parts.join("\n\n---\n\n"));

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
                tracing::info!("Astra Pass 2 enhanced report {} for deal {} (depth {})", r.id, deal_id, new_depth);
            } else {
                // No existing report — create one
                let report_id = uuid::Uuid::new_v4();
                let deal_uuid = uuid::Uuid::parse_str(deal_id.as_str()).ok();
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
                tracing::info!("Astra Pass 2 created new report {} for deal {}", report_id, deal_id);
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
        #[derive(sqlx::FromRow)] struct BizReport { executive_summary: Option<String> }
        sqlx::query_as::<_, BizReport>("SELECT executive_summary FROM business_reports WHERE crm_deal_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(id).fetch_optional(pool).await.ok().flatten().and_then(|r| r.executive_summary)
    };
    let person_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)] struct PersonIntel { intelligence_summary: Option<String>, full_name: Option<String>, company_name: Option<String> }
        sqlx::query_as::<_, PersonIntel>("SELECT intelligence_summary, full_name, company_name FROM persons WHERE crm_contact_id = ? LIMIT 1")
            .bind(cid).fetch_optional(pool).await.ok().flatten()
            .map(|p| format!("**Contact:** {}\n**Company:** {}\n\n{}", p.full_name.unwrap_or_default(), p.company_name.unwrap_or_default(), p.intelligence_summary.unwrap_or_default()))
    } else { None };
    // Company intelligence from the companies table
    let company_intel: Option<String> = if let Some(ref cid) = deal.crm_contact_id {
        #[derive(sqlx::FromRow)] struct CI { intelligence_summary: Option<String>, name: Option<String> }
        sqlx::query_as::<_, CI>(
            "SELECT c.intelligence_summary, c.name FROM companies c JOIN persons p ON lower(p.company_name) = lower(c.name) WHERE p.crm_contact_id = ? LIMIT 1"
        ).bind(cid).fetch_optional(pool).await.ok().flatten()
        .and_then(|c| c.intelligence_summary.map(|s| format!("**Company: {}**\n\n{}", c.name.unwrap_or_default(), s)))
    } else { None };
    let transcripts: Vec<String> = {
        #[derive(sqlx::FromRow)] struct TranscriptRow { summary: Option<String> }
        sqlx::query_as::<_, TranscriptRow>("SELECT summary FROM deal_transcripts WHERE deal_id = ? ORDER BY created_at ASC")
            .bind(id).fetch_all(pool).await.unwrap_or_default()
            .into_iter().filter_map(|t| t.summary).collect()
    };

    let mut context_parts = Vec::new();
    if let Some(ref bi) = report_summary { context_parts.push(format!("## Business Intelligence Report\n{}", bi)); }
    if let Some(ref ci) = company_intel { context_parts.push(format!("## Company Intelligence\n{}", ci)); }
    if let Some(ref pi) = person_intel { context_parts.push(format!("## Contact Profile\n{}", pi)); }
    if !transcripts.is_empty() { context_parts.push(format!("## Discovery Transcript Summaries\n{}", transcripts.join("\n\n"))); }
    if let Some(ref desc) = deal.description { context_parts.push(format!("## Deal Notes\n{}", desc)); }
    // F13: Do NOT feed deal.amount to Cash — Cash should determine pricing independently

    let context = if context_parts.is_empty() {
        format!("Deal: {}\nClient company: {}", deal.name, deal.name.split('—').last().unwrap_or(&deal.name).trim())
    } else {
        context_parts.join("\n\n---\n\n")
    };

    let cash_system = "You are Cash, a razor-sharp sales strategist and proposal architect at PowerClub Global, a creative production and brand strategy agency. You transform business intelligence into irresistible, tailored proposals. Every word earns its place. Write proposals in clear, compelling markdown with these sections: Executive Summary, Client Situation, Proposed Solution, Deliverables, Timeline, Investment, and Next Steps. IMPORTANT: At the very end, include a JSON block with the deliverables list using this exact format:\n\n```json\n[{\"title\": \"Deliverable Name\", \"description\": \"Brief description\"}]\n```\n\nBe bold, specific, and value-driven. British English optional.";
    let user_prompt = format!("Write a professional proposal for the following deal.\n\n{}", context);

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
                let items = deliverables.as_array().cloned()
                    .or_else(|| deliverables.get("deliverables").and_then(|d| d.as_array()).cloned())
                    .unwrap_or_default();
                let total_value: f64 = items.iter()
                    .filter_map(|item| {
                        item.get("estimated_value")
                            .and_then(|v| v.as_f64())
                    })
                    .sum();
                if total_value > 0.0 {
                    let _ = sqlx::query(
                        "UPDATE crm_deals SET amount = ?, updated_at = datetime('now','subsec') WHERE id = ?"
                    )
                    .bind(total_value)
                    .bind(id)
                    .execute(pool)
                    .await;
                    tracing::info!("Cash set deal.amount to ${:.0} from deliverable estimated_values for deal {}", total_value, id);
                }
            }
        }
    }

    let updated = CrmDeal::find_by_id(pool, id).await?;
    tracing::info!("Cash generated proposal for deal {}", id);
    Ok(updated)
}

// ── POST /crm/deals/:id/generate-proposal (Cash HTTP handler) ───────────────
async fn generate_proposal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let updated = generate_proposal_core(pool, &id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

// ── POST /crm/deals/:id/approve-proposal ────────────────────────────────────
async fn approve_proposal(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
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
                    let items = deliverables_val.as_array().cloned()
                        .or_else(|| deliverables_val.get("deliverables").and_then(|d| d.as_array()).cloned())
                        .unwrap_or_default();

                    for item in &items {
                        let title = item["title"].as_str()
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
        tracing::info!("Proposal approved for deal {}: {} deliverables created", id, deliverable_count);
    }

    let updated = CrmDeal::find_by_id(pool, &id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

// ── Background helper: generate deck (Lux) ──────────────────────────────────
async fn generate_deck_background(pool: &sqlx::SqlitePool, deal_id: DbUuid) {
    match generate_deck_core(pool, &deal_id).await {
        Ok(_) => tracing::info!("Lux auto-generated deck for deal {}", deal_id),
        Err(e) => tracing::warn!("Lux auto-generation failed for deal {}: {}", deal_id, e),
    }
}

// ── POST /crm/deals/:id/generate-deck (Lux) ─────────────────────────────────
async fn generate_deck_core(pool: &sqlx::SqlitePool, id: &DbUuid) -> Result<CrmDeal, ApiError> {
    let deal = CrmDeal::find_by_id(pool, id).await?;

    if deal.proposal_text.is_none() {
        return Err(ApiError::BadRequest("Generate proposal first before generating deck".into()));
    }

    let brand_context: Option<String> = if let Some(ref org_id) = deal.organization_id {
        #[derive(sqlx::FromRow)] struct BrandRow { primary_color: Option<String>, brand_voice: Option<String>, tagline: Option<String> }
        sqlx::query_as::<_, BrandRow>("SELECT primary_color, brand_voice, tagline FROM organization_brand_profiles WHERE organization_id = ? LIMIT 1")
            .bind(org_id.to_string()).fetch_optional(pool).await.ok().flatten()
            .map(|b| format!("Brand primary color: {}\nBrand voice: {}\nTagline: {}",
                b.primary_color.unwrap_or_default(), b.brand_voice.unwrap_or_default(), b.tagline.unwrap_or_default()))
    } else { None };

    let proposal = deal.proposal_text.as_deref().unwrap_or("");
    let lux_system = "You are Lux, a master presentation designer. You transform proposals into structured, high-impact slide decks. Output a complete slide-by-slide script in markdown, with each slide on a --- separator. Each slide has: SLIDE TITLE, KEY MESSAGE (1 sentence), VISUAL SUGGESTION, and SPEAKER NOTES. Design for clarity, impact, and brand alignment. Slides: Cover, Agenda, Client Situation, Our Solution, Deliverables, Timeline, Investment, Why Us, Next Steps, Close.";
    let user_prompt = match brand_context {
        Some(ref bc) => format!("Create a sales deck for this proposal.\n\nBrand context:\n{}\n\n---\n\nProposal:\n{}", bc, proposal),
        None => format!("Create a sales deck for this proposal:\n\n{}", proposal),
    };

    let deck_script = call_llm(lux_system, &user_prompt).await?;

    let deck_id = uuid::Uuid::new_v4();
    let deck_url = format!("/api/crm/deals/{}/deck/{}", id, deck_id);

    let _ = sqlx::query(
        "INSERT OR IGNORE INTO project_knowledge_sources (id, owner_type, owner_id, source_type, source_id, source_title, source_summary, coverage_score, is_active, created_at, updated_at) VALUES (?, 'deal', ?, 'deck_script', ?, ?, ?, 0.8, 1, datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(uuid::Uuid::new_v4().to_string())
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
async fn generate_deck(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let updated = generate_deck_core(pool, &id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

// ── POST /crm/deals/:id/send-invoice ────────────────────────────────────────
async fn send_deal_invoice(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    // Check if invoice already sent
    if deal.invoice_id.is_some() {
        return Err(ApiError::BadRequest("Invoice already sent for this deal".into()));
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE invoice_type = 'ar'")
        .fetch_one(pool).await.unwrap_or(0);
    let invoice_number = format!("INV-{:05}", count + 1);
    let invoice_id = uuid::Uuid::new_v4();
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
    sqlx::query("UPDATE crm_deals SET invoice_id = ?, updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(invoice_id.to_string())
        .bind(&id)
        .execute(pool).await
        .map_err(|e| ApiError::BadRequest(format!("Failed to link invoice: {}", e)))?;

    tracing::info!("Invoice {} sent for deal {} (${:.2})", invoice_number, id, amount_usd);
    Ok(Json(ApiResponse::success(serde_json::json!({
        "invoice_id": invoice_id.to_string(),
        "invoice_number": invoice_number,
        "amount_usd": amount_usd,
        "status": "sent"
    }))))
}

// ── POST /crm/deals/:id/mark-won ─────────────────────────────────────────────
/// Won automation chain: set won_at, move to Won stage, create client + project + tasks
async fn mark_deal_won(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    let deal = require_deal_org_access(&access_context, pool, &id).await?;

    let win_reason = body["win_reason"].as_str().map(|s| s.to_string());

    // Find Won stage for this pipeline
    let won_stage: Option<DbUuid> = if let Some(ref pipeline_id) = deal.crm_pipeline_id {
        #[derive(sqlx::FromRow)] struct S { id: DbUuid }
        sqlx::query_as::<_, S>("SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND (stage_type = 'won' OR name = 'Won') LIMIT 1")
            .bind(pipeline_id).fetch_optional(pool).await.ok().flatten().map(|s| s.id)
    } else { None };

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
    } else { None };

    let company_name = contact_info.as_ref().and_then(|c| c.company_name.clone())
        .unwrap_or_else(|| deal.name.clone());

    // ── Create or find Client record ─────────────────────────────────────────
    let client_id = {
        #[derive(sqlx::FromRow)] struct C { id: DbUuid }
        let existing = if let Some(ref org_id) = deal.organization_id {
            sqlx::query_as::<_, C>("SELECT id FROM clients WHERE organization_id = ? AND name = ? LIMIT 1")
                .bind(org_id).bind(&company_name).fetch_optional(pool).await.ok().flatten()
        } else { None };

        if let Some(c) = existing {
            c.id
        } else {
            let cid = DbUuid::new();
            let slug = company_name.to_lowercase()
                .chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect::<String>()
                .split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
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
    let _ = sqlx::query("UPDATE crm_deals SET project_id = ?, updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(&project_id).bind(&id).execute(pool).await;

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
    struct DeliverableRow { #[allow(dead_code)] id: DbUuid, title: String, description: Option<String> }
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
        tracing::info!("VIBE transaction {} created: {} VIBE for deal {}", tx_id, vibe_amount, id);
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

    tracing::info!("Deal {} marked Won. Client '{}', Project '{}', {} tasks created", id, company_name, project_name, task_count);
    Ok(Json(ApiResponse::success(serde_json::json!({
        "deal_id": id.to_string(),
        "client_id": client_id.to_string(),
        "project_id": project_id.to_string(),
        "project_name": project_name,
        "tasks_created": task_count,
    }))))
}

// ── GET /crm/deals/:id/transcripts ──────────────────────────────────────────
async fn list_deal_transcripts(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<db::models::crm_deal::DealTranscript>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
    require_deal_org_access(&access_context, pool, &id).await?;
    let transcripts = sqlx::query_as::<_, db::models::crm_deal::DealTranscript>(
        "SELECT * FROM deal_transcripts WHERE deal_id = ? ORDER BY created_at ASC"
    )
    .bind(&id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;
    Ok(Json(ApiResponse::success(transcripts)))
}

// ── POST /crm/deals/:id/transcripts ─────────────────────────────────────────
async fn link_deal_transcript(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<db::models::crm_deal::LinkTranscriptRequest>,
) -> Result<Json<ApiResponse<db::models::crm_deal::DealTranscript>>, ApiError> {
    let pool = &deployment.db().pool;
    let deal_id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?;
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
        "SELECT * FROM deal_transcripts WHERE id = ?"
    )
    .bind(&transcript_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;

    Ok(Json(ApiResponse::success(record)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/crm/deals", get(list_deals))
        .route("/crm/deals", post(create_deal))
        .route("/crm/deals/enriched", get(list_enriched_deals))
        .route("/crm/deals/metrics", get(get_metrics))
        .route("/crm/deals/kanban/{pipeline_id}", get(get_kanban_data))
        .route("/crm/deals/{id}", get(get_deal))
        .route("/crm/deals/{id}", patch(update_deal))
        .route("/crm/deals/{id}", delete(delete_deal))
        .route("/crm/deals/{id}/rich", get(get_deal_rich))
        .route("/crm/deals/{id}/stage", patch(move_deal_stage))
        .route("/crm/deals/{id}/advance-requirements", get(get_advance_requirements))
        .route("/crm/deals/{id}/advance", post(advance_deal))
        .route("/crm/deals/{id}/generate-proposal", post(generate_proposal))
        .route("/crm/deals/{id}/approve-proposal", post(approve_proposal))
        .route("/crm/deals/{id}/generate-deck", post(generate_deck))
        .route("/crm/deals/{id}/send-invoice", post(send_deal_invoice))
        .route("/crm/deals/{id}/mark-won", post(mark_deal_won))
        .route("/crm/deals/{id}/transcripts", get(list_deal_transcripts))
        .route("/crm/deals/{id}/transcripts", post(link_deal_transcript))
        // Org-scoped CRM deal routes
        .route("/organizations/{org_id}/crm/deals", get(list_org_deals))
}

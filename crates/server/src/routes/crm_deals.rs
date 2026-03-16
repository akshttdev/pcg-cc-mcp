//! CRM Deal Management Routes
//!
//! Handles deal CRUD operations and Kanban board data.

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post, delete},
    Json, Router,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};
use db::db_uuid::DbUuid;
use db::models::crm_deal::{
    CrmDeal, CreateCrmDeal, KanbanBoardData, UpdateCrmDeal, CrmDealWithContact,
};

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

/// GET /crm/deals - List deals with optional filters
async fn list_deals(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListDealsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmDeal>>>, ApiError> {
    let pool = &deployment.db().pool;

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

/// GET /crm/deals/kanban/:pipeline_id - Get Kanban board data
async fn get_kanban_data(
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<Uuid>,
) -> Result<Json<ApiResponse<KanbanBoardData>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline_id = DbUuid::from(pipeline_id);
    let kanban_data = CrmDeal::get_kanban_data(pool, &pipeline_id).await?;
    Ok(Json(ApiResponse::success(kanban_data)))
}

/// GET /crm/deals/:id - Get single deal
async fn get_deal(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    let deal = CrmDeal::find_by_id(pool, &id).await?;
    Ok(Json(ApiResponse::success(deal)))
}

/// GET /crm/deals/:id/rich - Get deal with all enriched data for detail panel
async fn get_deal_rich(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CrmDealRich>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);

    // Build CrmDealWithContact inline (mirrors kanban enrichment)
    let deal = CrmDeal::find_by_id(pool, &id).await?;

    let contact_info = if let Some(ref contact_id) = deal.crm_contact_id {
        db::models::crm_contact::CrmContact::find_by_id(pool, contact_id).await.ok()
    } else {
        None
    };

    // person_id via crm_contacts.person_id (may not exist — fall back to persons table lookup by email)
    let person_id: Option<DbUuid> = {
        let by_contact: Option<DbUuid> = if let Some(ref contact_id) = deal.crm_contact_id {
            #[derive(sqlx::FromRow)] struct Row { person_id: Option<DbUuid> }
            sqlx::query_as::<_, Row>("SELECT person_id FROM crm_contacts WHERE id = ?")
                .bind(contact_id)
                .fetch_optional(pool).await.ok().flatten().and_then(|r| r.person_id)
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

    Ok(Json(ApiResponse::success(deal)))
}

/// PATCH /crm/deals/:id - Update deal
async fn update_deal(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(mut data): Json<UpdateCrmDeal>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    // Sanitize amount — reject Infinity/NaN
    if let Some(amt) = data.amount {
        if !amt.is_finite() {
            data.amount = None;
        }
    }
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    let deal = CrmDeal::update(pool, &id, data).await?;
    Ok(Json(ApiResponse::success(deal)))
}

/// PATCH /crm/deals/:id/stage - Move deal to new stage (drag-drop)
/// When a Sales pipeline deal moves to a Won stage, auto-create a Delivery pipeline deal
async fn move_deal_stage(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<MoveDealRequest>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
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

    // Stage-aware auto-triggers (8-stage Clients pipeline with human gates)
    let stage_name_lower = target_stage
        .as_ref()
        .map(|s| s.name.to_lowercase())
        .unwrap_or_default();

    // "Lead" stage → auto-trigger "Who Is" person + company research
    if stage_name_lower == "lead" {
        let pool_bg = pool.clone();
        let deal_id = deal.id.clone();
        let contact_id = deal.crm_contact_id.clone();
        tokio::spawn(async move {
            trigger_who_is_research(&pool_bg, deal_id, contact_id).await;
        });
    }

    // Stages that need a human review task before advancing
    let review_stages = [
        ("business analysis", "Review Who Is research and verify lead quality"),
        ("discovery", "Review business analysis report and confirm discovery readiness"),
        ("build proposal", "Confirm discovery knowledge is proposal-ready"),
        ("polish", "Verify proposed services and budgets are accurate"),
    ];

    for (stage_key, task_desc) in &review_stages {
        if stage_name_lower == *stage_key {
            create_review_task_if_needed(pool, &deal, task_desc).await;
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
async fn generate_phase1_business_report(
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

    #[derive(sqlx::FromRow)]
    struct PersonDataRow {
        person_id: Option<DbUuid>,
        company_name: Option<String>,
    }
    let contact_data = sqlx::query_as::<_, PersonDataRow>(
        "SELECT person_id, company_name FROM crm_contacts WHERE id = ?",
    )
    .bind(&contact_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let person_id = contact_data.as_ref().and_then(|c| c.person_id.clone());
    let company_name = contact_data.as_ref().and_then(|c| c.company_name.clone());

    #[derive(sqlx::FromRow)]
    struct PersonIntelRow {
        id: DbUuid,
        full_name: Option<String>,
        intelligence_summary: Option<String>,
        email: Option<String>,
        title: Option<String>,
    }
    let person_intel = if let Some(ref pid) = person_id {
        sqlx::query_as::<_, PersonIntelRow>(
            "SELECT id, full_name, intelligence_summary, email, title FROM persons WHERE id = ?",
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
        if let Some(ref title) = pi.title { parts.push(format!("**Title:** {}", title)); }
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
    let person_uuid = person_intel.as_ref().and_then(|p| uuid::Uuid::parse_str(p.id.as_str()).ok());
    let company_uuid = company_intel.as_ref().and_then(|c| uuid::Uuid::parse_str(c.id.as_str()).ok());
    let deal_uuid = uuid::Uuid::parse_str(deal_id.as_str()).ok();

    let report_id = uuid::Uuid::new_v4();
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

/// Create a review task for a deal if none exists
async fn create_review_task_if_needed(pool: &sqlx::SqlitePool, deal: &CrmDeal, description: &str) {
    let existing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&deal.id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if existing_count == 0 {
        let task_id = DbUuid::new();
        let task_title = format!("Review & approve: {}", deal.name);
        let _ = sqlx::query(
            r#"
            INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, created_at, updated_at)
            VALUES (?, ?, ?, 'todo', ?, ?, datetime('now','subsec'), datetime('now','subsec'))
            "#,
        )
        .bind(&task_id)
        .bind(&task_title)
        .bind(description)
        .bind(&deal.id)
        .bind(&deal.project_id)
        .execute(pool)
        .await;
    }
}

/// POST /crm/deals/:id/advance - Advance deal to next stage after review approval
///
/// Auth: This handler is behind the `require_auth` middleware layer applied to all
/// `protected_routes` in `routes/mod.rs`, which rejects unauthenticated requests
/// with 401. Per-deal ownership checks are not performed here (consistent with all
/// other CRM deal handlers: create, update, move_deal_stage, delete).
async fn advance_deal(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);

    let deal = CrmDeal::find_by_id(pool, &id).await?;
    let current_stage_id = deal.crm_stage_id.clone()
        .ok_or_else(|| ApiError::BadRequest("Deal has no stage assigned".to_string()))?;

    // Validate: active review tasks for this deal must be done
    let pending_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if pending_tasks > 0 {
        return Err(ApiError::BadRequest(
            format!("Cannot advance: {} pending review task(s) must be completed first", pending_tasks),
        ));
    }

    // Find next stage by position + 1 within the same pipeline
    let current_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, &current_stage_id).await
        .map_err(|_| ApiError::NotFound("Current stage not found".to_string()))?;

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

    // Create review task for stages that need one
    let review_stages = [
        ("business analysis", "Review Who Is research and verify lead quality"),
        ("discovery", "Review business analysis report and confirm discovery readiness"),
        ("build proposal", "Confirm discovery knowledge is proposal-ready"),
        ("polish", "Verify proposed services and budgets are accurate"),
    ];
    for (stage_key, task_desc) in &review_stages {
        if new_stage_name == *stage_key {
            create_review_task_if_needed(pool, &deal, task_desc).await;
            break;
        }
    }

    // Generate Phase 1 business analysis report on Business Analysis stage entry
    if new_stage_name == "business analysis" {
        let pool_bg = pool.clone();
        let deal_id_bg = deal.id.clone();
        let contact_id_bg = deal.crm_contact_id.clone();
        let project_id_bg = deal.project_id.clone();
        tokio::spawn(async move {
            generate_phase1_business_report(&pool_bg, deal_id_bg, contact_id_bg, project_id_bg).await;
        });
    }

    // Auto-trigger research on Lead entry
    if new_stage_name == "lead" {
        let pool_bg = pool.clone();
        let deal_id = deal.id.clone();
        let contact_id = deal.crm_contact_id.clone();
        tokio::spawn(async move {
            trigger_who_is_research(&pool_bg, deal_id, contact_id).await;
        });
    }

    Ok(Json(ApiResponse::success(deal)))
}

/// GET /organizations/:org_id/crm/deals - List deals for an organization
async fn list_org_deals(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<CrmDeal>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_id = DbUuid::from(org_id);
    let deals = CrmDeal::find_by_organization(pool, &org_id).await?;
    Ok(Json(ApiResponse::success(deals)))
}


/// DELETE /crm/deals/:id - Delete deal
async fn delete_deal(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
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
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<MetricsQuery>,
) -> Result<Json<ApiResponse<PipelineMetrics>>, ApiError> {
    let pool = &deployment.db().pool;

    // Get all deals for the project (optionally filtered by pipeline)
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

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/crm/deals", get(list_deals))
        .route("/crm/deals", post(create_deal))
        .route("/crm/deals/metrics", get(get_metrics))
        .route("/crm/deals/kanban/{pipeline_id}", get(get_kanban_data))
        .route("/crm/deals/{id}", get(get_deal))
        .route("/crm/deals/{id}", patch(update_deal))
        .route("/crm/deals/{id}", delete(delete_deal))
        .route("/crm/deals/{id}/rich", get(get_deal_rich))
        .route("/crm/deals/{id}/stage", patch(move_deal_stage))
        .route("/crm/deals/{id}/advance", post(advance_deal))
        // Org-scoped CRM deal routes
        .route("/organizations/{org_id}/crm/deals", get(list_org_deals))

}

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

    // If the target stage name contains "Analysis Done", create a review task
    let stage_name_lower = target_stage
        .as_ref()
        .map(|s| s.name.to_lowercase())
        .unwrap_or_default();
    if stage_name_lower.contains("analysis done") || stage_name_lower.contains("analysis complete") {
        // Only create if no existing active task for this deal
        let existing_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ? AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL"
        )
        .bind(&deal.id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        let existing = if existing_count > 0 { Some(()) } else { None };

        if existing.is_none() {
            let task_id = Uuid::new_v4();
            let task_title = format!("Review intelligence: {}", deal.name);
            let task_desc = "Review the Who Is? research and business report for this lead before advancing to proposal.";
            let _ = sqlx::query(
                r#"
                INSERT INTO tasks (id, title, description, status, crm_deal_id, project_id, created_at, updated_at)
                VALUES (?, ?, ?, 'todo', ?, ?, datetime('now','subsec'), datetime('now','subsec'))
                "#,
            )
            .bind(task_id)
            .bind(&task_title)
            .bind(task_desc)
            .bind(&deal.id)
            .bind(&deal.project_id)
            .execute(pool)
            .await;
        }
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
        // Org-scoped CRM deal routes
        .route("/organizations/{org_id}/crm/deals", get(list_org_deals))

}

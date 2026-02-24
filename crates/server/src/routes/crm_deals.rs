//! CRM Deal Management Routes
//!
//! Handles deal CRUD operations and Kanban board data.

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post, delete},
    Json, Router,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};
use db::models::crm_deal::{
    CrmDeal, CreateCrmDeal, KanbanBoardData, UpdateCrmDeal,
};

#[derive(Debug, Deserialize)]
pub struct ListDealsQuery {
    pub project_id: Option<Uuid>,
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
        CrmDeal::find_by_pipeline(pool, pipeline_id).await?
    } else if let Some(stage_id) = query.stage_id {
        CrmDeal::find_by_stage(pool, stage_id).await?
    } else if let Some(contact_id) = query.contact_id {
        CrmDeal::find_by_contact(pool, contact_id).await?
    } else if let Some(project_id) = query.project_id {
        CrmDeal::find_by_project(pool, project_id).await?
    } else {
        return Err(ApiError::BadRequest(
            "Must provide project_id, pipeline_id, stage_id, or contact_id".to_string(),
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
    let kanban_data = CrmDeal::get_kanban_data(pool, pipeline_id).await?;
    Ok(Json(ApiResponse::success(kanban_data)))
}

/// GET /crm/deals/:id - Get single deal
async fn get_deal(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CrmDeal>>, ApiError> {
    let pool = &deployment.db().pool;
    let deal = CrmDeal::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(deal)))
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
            id, project_id, crm_contact_id, crm_deal_id, activity_type,
            subject, activity_at
        )
        VALUES (?1, ?2, ?3, ?4, 'deal_created', ?5, datetime('now', 'subsec'))
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(deal.project_id)
    .bind(deal.crm_contact_id)
    .bind(deal.id)
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
    let deal = CrmDeal::update(pool, id, data).await?;
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

    // Check if the target stage is a "won" stage
    let target_stage =
        db::models::crm_pipeline::CrmPipelineStage::find_by_id(pool, data.stage_id).await.ok();
    let is_won = target_stage
        .as_ref()
        .map(|s| s.is_won.unwrap_or(0) == 1)
        .unwrap_or(false);

    // Move the deal
    let deal = CrmDeal::move_to_stage(pool, id, data.stage_id, data.position).await?;

    // If won, check if this is a sales pipeline deal and auto-create delivery deal
    if is_won {
        if let Some(pipeline_id) = deal.crm_pipeline_id {
            if let Ok(pipeline) =
                db::models::crm_pipeline::CrmPipeline::find_by_id(pool, pipeline_id).await
            {
                if pipeline.pipeline_type == "sales" {
                    // Find the delivery pipeline for this project
                    if let Ok(Some(delivery_pipeline)) =
                        db::models::crm_pipeline::CrmPipeline::find_by_type(
                            pool,
                            deal.project_id,
                            db::models::crm_pipeline::PipelineType::Delivery,
                        )
                        .await
                    {
                        // Check if a delivery deal already exists for this contact
                        let has_delivery_deal = if let Some(contact_id) = deal.crm_contact_id {
                            let contact_deals =
                                CrmDeal::find_by_contact(pool, contact_id).await.unwrap_or_default();
                            contact_deals
                                .iter()
                                .any(|d| d.crm_pipeline_id == Some(delivery_pipeline.id))
                        } else {
                            false
                        };

                        if !has_delivery_deal {
                            // Get the first stage (Onboarding) of the delivery pipeline
                            let delivery_stages =
                                db::models::crm_pipeline::CrmPipelineStage::find_by_pipeline(
                                    pool,
                                    delivery_pipeline.id,
                                )
                                .await
                                .unwrap_or_default();

                            let onboarding_stage = delivery_stages.first();

                            // Create delivery deal
                            let _ = CrmDeal::create(
                                pool,
                                CreateCrmDeal {
                                    project_id: deal.project_id,
                                    crm_contact_id: deal.crm_contact_id,
                                    crm_pipeline_id: Some(delivery_pipeline.id),
                                    crm_stage_id: onboarding_stage.map(|s| s.id),
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

    Ok(Json(ApiResponse::success(deal)))
}

/// DELETE /crm/deals/:id - Delete deal
async fn delete_deal(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmDeal::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub project_id: Uuid,
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
        CrmDeal::find_by_pipeline(pool, pipeline_id).await?
    } else {
        CrmDeal::find_by_project(pool, query.project_id).await?
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
        if let Some(stage_id) = deal.crm_stage_id {
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
        .route("/crm/deals/{id}/stage", patch(move_deal_stage))
}

//! CRM Pipeline Management Routes
//!
//! Handles pipeline and stage CRUD operations for Kanban boards.

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
use db::models::crm_pipeline::{
    CrmPipeline, CrmPipelineStage, CrmPipelineWithStages,
    CreateCrmPipeline, CreateCrmPipelineStage, PipelineType,
    UpdateCrmPipeline, UpdateCrmPipelineStage,
};

#[derive(Debug, Deserialize)]
pub struct ListPipelinesQuery {
    pub project_id: Uuid,
    pub pipeline_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderStagesRequest {
    pub stage_ids: Vec<Uuid>,
}

/// GET /crm/pipelines - List pipelines for a project
async fn list_pipelines(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListPipelinesQuery>,
) -> Result<Json<ApiResponse<Vec<CrmPipeline>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Ensure default pipelines exist
    CrmPipeline::ensure_defaults(pool, query.project_id).await?;

    let pipelines = if let Some(type_str) = query.pipeline_type {
        let pipeline_type: PipelineType = type_str
            .parse()
            .map_err(|_| ApiError::BadRequest(format!("Invalid pipeline type: {}", type_str)))?;

        CrmPipeline::find_by_type(pool, query.project_id, pipeline_type)
            .await?
            .into_iter()
            .collect()
    } else {
        CrmPipeline::find_by_project(pool, query.project_id).await?
    };

    Ok(Json(ApiResponse::success(pipelines)))
}

/// GET /crm/pipelines/:id - Get pipeline with stages
async fn get_pipeline(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CrmPipelineWithStages>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline = CrmPipeline::find_with_stages(pool, id).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// POST /crm/pipelines - Create pipeline
async fn create_pipeline(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateCrmPipeline>,
) -> Result<Json<ApiResponse<CrmPipeline>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline = CrmPipeline::create(pool, data).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// PATCH /crm/pipelines/:id - Update pipeline
async fn update_pipeline(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateCrmPipeline>,
) -> Result<Json<ApiResponse<CrmPipeline>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline = CrmPipeline::update(pool, id, data).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// DELETE /crm/pipelines/:id - Delete pipeline
async fn delete_pipeline(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmPipeline::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /crm/pipelines/:id/stages - Get stages for a pipeline
async fn list_stages(
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<CrmPipelineStage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let stages = CrmPipelineStage::find_by_pipeline(pool, pipeline_id).await?;
    Ok(Json(ApiResponse::success(stages)))
}

/// POST /crm/pipelines/:id/stages - Add stage to pipeline
async fn create_stage(
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<Uuid>,
    Json(mut data): Json<CreateCrmPipelineStage>,
) -> Result<Json<ApiResponse<CrmPipelineStage>>, ApiError> {
    let pool = &deployment.db().pool;
    data.pipeline_id = pipeline_id;
    let stage = CrmPipelineStage::create(pool, data).await?;
    Ok(Json(ApiResponse::success(stage)))
}

/// PATCH /crm/pipelines/:pipeline_id/stages/:stage_id - Update stage
async fn update_stage(
    State(deployment): State<DeploymentImpl>,
    Path((_pipeline_id, stage_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<UpdateCrmPipelineStage>,
) -> Result<Json<ApiResponse<CrmPipelineStage>>, ApiError> {
    let pool = &deployment.db().pool;
    let stage = CrmPipelineStage::update(pool, stage_id, data).await?;
    Ok(Json(ApiResponse::success(stage)))
}

/// DELETE /crm/pipelines/:pipeline_id/stages/:stage_id - Delete stage
async fn delete_stage(
    State(deployment): State<DeploymentImpl>,
    Path((_pipeline_id, stage_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmPipelineStage::delete(pool, stage_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/pipelines/:id/stages/reorder - Reorder stages
async fn reorder_stages(
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<Uuid>,
    Json(data): Json<ReorderStagesRequest>,
) -> Result<Json<ApiResponse<Vec<CrmPipelineStage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let stages = CrmPipelineStage::reorder(pool, pipeline_id, data.stage_ids).await?;
    Ok(Json(ApiResponse::success(stages)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/crm/pipelines", get(list_pipelines))
        .route("/crm/pipelines", post(create_pipeline))
        .route("/crm/pipelines/{id}", get(get_pipeline))
        .route("/crm/pipelines/{id}", patch(update_pipeline))
        .route("/crm/pipelines/{id}", delete(delete_pipeline))
        .route("/crm/pipelines/{id}/stages", get(list_stages))
        .route("/crm/pipelines/{id}/stages", post(create_stage))
        .route("/crm/pipelines/{pipeline_id}/stages/{stage_id}", patch(update_stage))
        .route("/crm/pipelines/{pipeline_id}/stages/{stage_id}", delete(delete_stage))
        .route("/crm/pipelines/{id}/stages/reorder", post(reorder_stages))
}

//! CRM Pipeline Management Routes
//!
//! Handles pipeline and stage CRUD operations for Kanban boards.

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};
use db::{
    db_uuid::DbUuid,
    models::crm_pipeline::{
        CreateCrmPipeline, CreateCrmPipelineStage, CrmPipeline, CrmPipelineStage,
        CrmPipelineWithStages, PipelineType, UpdateCrmPipeline, UpdateCrmPipelineStage,
    },
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

#[derive(Debug, Deserialize)]
pub struct ListPipelinesQuery {
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub pipeline_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderStagesRequest {
    pub stage_ids: Vec<Uuid>,
}

/// Helper: load a pipeline and verify org membership
async fn require_pipeline_org_access(
    access: &AccessContext,
    pool: &sqlx::SqlitePool,
    pipeline_id: &DbUuid,
) -> Result<CrmPipeline, ApiError> {
    let pipeline = CrmPipeline::find_by_id(pool, pipeline_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    if let Some(ref org_id) = pipeline.organization_id {
        access.require_org_membership(pool, org_id.as_str()).await?;
    }
    Ok(pipeline)
}

/// GET /crm/pipelines - List pipelines for an organization
async fn list_pipelines(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListPipelinesQuery>,
) -> Result<Json<ApiResponse<Vec<CrmPipeline>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Resolve organization_id — either directly provided or looked up from project_id
    let org_id: DbUuid = if let Some(oid) = query.organization_id {
        DbUuid::from(oid)
    } else if let Some(pid) = query.project_id {
        // Find the org that owns this project via existing crm_pipelines or clients table
        let org_str: Option<String> = sqlx::query_scalar(
            "SELECT organization_id FROM crm_pipelines WHERE project_id = ? AND organization_id IS NOT NULL LIMIT 1"
        )
        .bind(DbUuid::from(pid))
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some(s) = org_str {
            DbUuid::from_string(s)
        } else {
            return Err(ApiError::BadRequest(
                "No pipelines found for the given project_id".to_string(),
            ));
        }
    } else {
        return Err(ApiError::BadRequest(
            "Must provide organization_id or project_id".to_string(),
        ));
    };

    access_context
        .require_org_membership(pool, org_id.as_str())
        .await?;

    // Ensure default pipelines exist for this org
    CrmPipeline::ensure_defaults_for_org(pool, &org_id).await?;

    let pipelines = if let Some(type_str) = query.pipeline_type {
        let pipeline_type: PipelineType = type_str
            .parse()
            .map_err(|_| ApiError::BadRequest(format!("Invalid pipeline type: {}", type_str)))?;
        CrmPipeline::find_by_type_for_org(pool, &org_id, pipeline_type)
            .await?
            .into_iter()
            .collect()
    } else {
        CrmPipeline::find_by_organization(pool, &org_id, None).await?
    };

    Ok(Json(ApiResponse::success(pipelines)))
}

/// GET /crm/pipelines/:id - Get pipeline with stages
async fn get_pipeline(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmPipelineWithStages>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_pipeline_org_access(&access_context, pool, &id).await?;
    let pipeline = CrmPipeline::find_with_stages(pool, &id).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// POST /crm/pipelines - Create pipeline
async fn create_pipeline(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateCrmPipeline>,
) -> Result<Json<ApiResponse<CrmPipeline>>, ApiError> {
    let pool = &deployment.db().pool;
    if let Some(ref org_id) = data.organization_id {
        access_context
            .require_org_membership(pool, org_id.as_str())
            .await?;
    }
    let pipeline = CrmPipeline::create(pool, data).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// PATCH /crm/pipelines/:id - Update pipeline
async fn update_pipeline(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<UpdateCrmPipeline>,
) -> Result<Json<ApiResponse<CrmPipeline>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_pipeline_org_access(&access_context, pool, &id).await?;
    let pipeline = CrmPipeline::update(pool, &id, data).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// DELETE /crm/pipelines/:id - Delete pipeline
async fn delete_pipeline(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_pipeline_org_access(&access_context, pool, &id).await?;
    CrmPipeline::delete(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /crm/pipelines/:id/stages - Get stages for a pipeline
async fn list_stages(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<CrmPipelineStage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline_id = DbUuid::from(pipeline_id);
    require_pipeline_org_access(&access_context, pool, &pipeline_id).await?;
    let stages = CrmPipelineStage::find_by_pipeline(pool, &pipeline_id).await?;
    Ok(Json(ApiResponse::success(stages)))
}

/// POST /crm/pipelines/:id/stages - Add stage to pipeline
async fn create_stage(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<String>,
    Json(mut data): Json<CreateCrmPipelineStage>,
) -> Result<Json<ApiResponse<CrmPipelineStage>>, ApiError> {
    let pool = &deployment.db().pool;
    let pid = DbUuid::from(pipeline_id);
    require_pipeline_org_access(&access_context, pool, &pid).await?;
    data.pipeline_id = pid;
    let stage = CrmPipelineStage::create(pool, data).await?;
    Ok(Json(ApiResponse::success(stage)))
}

/// PATCH /crm/pipelines/:pipeline_id/stages/:stage_id - Update stage
async fn update_stage(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((pipeline_id, stage_id)): Path<(String, String)>,
    Json(data): Json<UpdateCrmPipelineStage>,
) -> Result<Json<ApiResponse<CrmPipelineStage>>, ApiError> {
    let pool = &deployment.db().pool;
    let pid = DbUuid::from(pipeline_id);
    require_pipeline_org_access(&access_context, pool, &pid).await?;
    let stage_id = DbUuid::from(stage_id);
    let stage = CrmPipelineStage::update(pool, &stage_id, data).await?;
    Ok(Json(ApiResponse::success(stage)))
}

/// DELETE /crm/pipelines/:pipeline_id/stages/:stage_id - Delete stage
async fn delete_stage(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((pipeline_id, stage_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let pid = DbUuid::from(pipeline_id);
    require_pipeline_org_access(&access_context, pool, &pid).await?;
    let stage_id = DbUuid::from(stage_id);
    CrmPipelineStage::delete(pool, &stage_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/pipelines/:id/stages/reorder - Reorder stages
async fn reorder_stages(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(pipeline_id): Path<String>,
    Json(data): Json<ReorderStagesRequest>,
) -> Result<Json<ApiResponse<Vec<CrmPipelineStage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let pipeline_id = DbUuid::from(pipeline_id);
    require_pipeline_org_access(&access_context, pool, &pipeline_id).await?;
    let stages = CrmPipelineStage::reorder(
        pool,
        &pipeline_id,
        data.stage_ids.into_iter().map(DbUuid::from).collect(),
    )
    .await?;
    Ok(Json(ApiResponse::success(stages)))
}

#[derive(Debug, Deserialize)]
pub struct ListOrgPipelinesQuery {
    pub pipeline_type: Option<String>,
}

/// GET /organizations/:org_id/crm/pipelines - List pipelines across all org projects
async fn list_org_pipelines(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(query): Query<ListOrgPipelinesQuery>,
) -> Result<Json<ApiResponse<Vec<CrmPipeline>>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context.require_org_membership(pool, &org_id).await?;
    let org_id = DbUuid::from(org_id);

    // Ensure default pipelines exist for the organization
    let _ = CrmPipeline::ensure_defaults_for_org(pool, &org_id).await;

    let pipeline_type_filter =
        if let Some(ref type_str) = query.pipeline_type {
            Some(type_str.parse::<PipelineType>().map_err(|_| {
                ApiError::BadRequest(format!("Invalid pipeline type: {}", type_str))
            })?)
        } else {
            None
        };

    let pipelines = CrmPipeline::find_by_organization(pool, &org_id, pipeline_type_filter).await?;

    Ok(Json(ApiResponse::success(pipelines)))
}

/// GET /organizations/:org_id/crm/pipelines/:pipeline_id - Get pipeline with stages (org-scoped)
async fn get_org_pipeline(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((org_id, pipeline_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<CrmPipelineWithStages>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context.require_org_membership(pool, &org_id).await?;
    let pipeline_id = DbUuid::from(pipeline_id);
    let pipeline = CrmPipeline::find_with_stages(pool, &pipeline_id).await?;
    Ok(Json(ApiResponse::success(pipeline)))
}

/// GET /organizations/:org_id/crm/pipelines/:pipeline_id/kanban - Kanban data aggregated across org
async fn get_org_kanban(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((org_id, pipeline_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<db::models::crm_deal::KanbanBoardData>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context.require_org_membership(pool, &org_id).await?;
    let org_id = DbUuid::from(org_id);
    let pipeline_id = DbUuid::from(pipeline_id);
    let kanban_data =
        db::models::crm_deal::CrmDeal::get_kanban_by_organization(pool, &org_id, &pipeline_id)
            .await?;
    Ok(Json(ApiResponse::success(kanban_data)))
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
        .route(
            "/crm/pipelines/{pipeline_id}/stages/{stage_id}",
            patch(update_stage),
        )
        .route(
            "/crm/pipelines/{pipeline_id}/stages/{stage_id}",
            delete(delete_stage),
        )
        .route("/crm/pipelines/{id}/stages/reorder", post(reorder_stages))
        // Org-scoped CRM routes
        .route(
            "/organizations/{org_id}/crm/pipelines",
            get(list_org_pipelines),
        )
        .route(
            "/organizations/{org_id}/crm/pipelines/{pipeline_id}",
            get(get_org_pipeline),
        )
        .route(
            "/organizations/{org_id}/crm/pipelines/{pipeline_id}/kanban",
            get(get_org_kanban),
        )
}

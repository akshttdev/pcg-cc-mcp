use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use db::models::project_knowledge_source::{
    KnowledgeSourceType, ProjectKnowledgeCompleteness, ProjectKnowledgeSource,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Grouped knowledge sources response
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ProjectKnowledgeResponse {
    pub project_id: String,
    pub completeness: Option<ProjectKnowledgeCompleteness>,
    pub total_sources: usize,
    pub stale_count: usize,
    pub sources_by_type: std::collections::HashMap<String, Vec<ProjectKnowledgeSource>>,
}

/// GET /api/projects/:project_id/knowledge
async fn get_project_knowledge(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<ProjectKnowledgeResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let sources = ProjectKnowledgeSource::find_by_project(pool, project_id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to fetch knowledge sources: {}", e))
        })?;

    let completeness = ProjectKnowledgeSource::get_completeness(pool, project_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch completeness: {}", e)))?;

    let stale_count = sources.iter().filter(|s| s.is_stale).count();
    let total_sources = sources.len();

    let mut sources_by_type: std::collections::HashMap<String, Vec<ProjectKnowledgeSource>> =
        std::collections::HashMap::new();
    for source in sources {
        sources_by_type
            .entry(source.source_type.clone())
            .or_default()
            .push(source);
    }

    Ok(Json(ApiResponse::success(ProjectKnowledgeResponse {
        project_id: project_id.to_string(),
        completeness,
        total_sources,
        stale_count,
        sources_by_type,
    })))
}

/// Create knowledge source request
#[derive(Debug, Deserialize)]
pub struct CreateKnowledgeSourceRequest {
    pub source_type: String,
    pub source_title: String,
    pub source_summary: Option<String>,
    pub coverage_score: Option<f64>,
}

/// POST /api/projects/:project_id/knowledge
async fn create_knowledge_source(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateKnowledgeSourceRequest>,
) -> Result<Json<ApiResponse<ProjectKnowledgeSource>>, ApiError> {
    let pool = &deployment.db().pool;

    let source_type: KnowledgeSourceType = body
        .source_type
        .parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;

    let source_id = Uuid::new_v4().to_string();
    let coverage = body.coverage_score.unwrap_or(0.0);

    ProjectKnowledgeSource::upsert_source(
        pool,
        project_id,
        &source_type,
        &source_id,
        &body.source_title,
        body.source_summary.as_deref(),
        coverage,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create knowledge source: {}", e)))?;

    // Fetch the newly created source back
    let sources = ProjectKnowledgeSource::find_by_project(pool, project_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch source: {}", e)))?;

    let created = sources
        .into_iter()
        .find(|s| s.source_id == source_id)
        .ok_or_else(|| ApiError::InternalError("Source created but not found".to_string()))?;

    Ok(Json(ApiResponse::success(created)))
}

/// POST /api/projects/:project_id/knowledge/:source_id/refresh
async fn refresh_source(
    Path((project_id, source_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let _ = project_id; // validated by path

    ProjectKnowledgeSource::mark_refreshed(pool, source_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to refresh source: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// POST /api/projects/:project_id/knowledge/:source_id/stale
async fn mark_source_stale(
    Path((project_id, source_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let _ = project_id;

    ProjectKnowledgeSource::mark_stale(pool, source_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to mark source stale: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/projects/{project_id}/knowledge",
            get(get_project_knowledge).post(create_knowledge_source),
        )
        .route(
            "/projects/{project_id}/knowledge/{source_id}/refresh",
            post(refresh_source),
        )
        .route(
            "/projects/{project_id}/knowledge/{source_id}/stale",
            post(mark_source_stale),
        )
}

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use db::models::project_knowledge_source::{
    ProjectKnowledgeSource, ProjectKnowledgeCompleteness,
};
use deployment::Deployment;
use serde::Serialize;
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
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch knowledge sources: {}", e)))?;

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
            get(get_project_knowledge),
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

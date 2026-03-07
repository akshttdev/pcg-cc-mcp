use axum::{
    Router,
    extract::{Path, State},
    routing::{delete, get, patch, post},
    Json,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::deliverable::{CreateDeliverable, Deliverable, UpdateDeliverable};
use db::models::project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource};

// ── Helpers ───────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct MoveStatusBody {
    pub status: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/projects/:project_id/deliverables
async fn list_deliverables(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<Deliverable>>>, ApiError> {
    let items = Deliverable::list_for_project(&d.db().pool, project_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/projects/:project_id/deliverables
async fn create_deliverable(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(mut body): Json<CreateDeliverable>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    // Ensure the path project_id takes precedence
    body.project_id = project_id;
    let item = Deliverable::create(&d.db().pool, body).await?;
    Ok(Json(ApiResponse::success(item)))
}

/// GET /api/deliverables/:id
async fn get_deliverable(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    Deliverable::find_by_id(&d.db().pool, id)
        .await?
        .map(|x| Json(ApiResponse::success(x)))
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))
}

/// PATCH /api/deliverables/:id
async fn update_deliverable(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateDeliverable>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    Deliverable::update(&d.db().pool, id, body)
        .await?
        .map(|x| Json(ApiResponse::success(x)))
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))
}

/// PATCH /api/deliverables/:id/status
async fn move_deliverable_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveStatusBody>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    let pool = &d.db().pool;
    let deliverable = Deliverable::move_status(pool, id, &body.status)
        .await?
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))?;

    // Auto-register in knowledge graph when marked done
    if body.status == "done" {
        let source_id = id.to_string();
        let source_title = format!("{} ({})", deliverable.title, deliverable.deliverable_type);
        let summary = deliverable.description.as_str();
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            deliverable.project_id,
            &KnowledgeSourceType::Artifact,
            &source_id,
            &source_title,
            Some(summary),
            1.0,
        )
        .await;
        tracing::info!("Deliverable {} registered in knowledge graph as artifact", id);
    }

    Ok(Json(ApiResponse::success(deliverable)))
}

/// DELETE /api/deliverables/:id
async fn delete_deliverable(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let deleted = Deliverable::delete(&d.db().pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound("Deliverable not found".into()))
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Scoped under project
        .route(
            "/projects/{project_id}/deliverables",
            get(list_deliverables).post(create_deliverable),
        )
        // Flat by ID
        .route(
            "/deliverables/{id}",
            get(get_deliverable)
                .patch(update_deliverable)
                .delete(delete_deliverable),
        )
        .route("/deliverables/{id}/status", patch(move_deliverable_status))
        .with_state(deployment.clone())
}

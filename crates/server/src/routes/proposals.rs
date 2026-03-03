use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::proposal::{CreateProposal, Proposal, UpdateProposal};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListProposalsParams {
    pub status: Option<String>,
    pub lead_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MoveStatusBody {
    pub status: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/proposals
async fn list_proposals(
    State(d): State<DeploymentImpl>,
    Query(p): Query<ListProposalsParams>,
) -> Result<Json<ApiResponse<Vec<Proposal>>>, ApiError> {
    let proposals = Proposal::list(
        &d.db().pool,
        p.status.as_deref(),
        p.lead_id,
        p.project_id,
        p.owner_id,
        p.limit,
    )
    .await?;
    Ok(Json(ApiResponse::success(proposals)))
}

/// POST /api/proposals
async fn create_proposal(
    State(d): State<DeploymentImpl>,
    Json(body): Json<CreateProposal>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    let proposal = Proposal::create(&d.db().pool, body).await?;
    Ok(Json(ApiResponse::success(proposal)))
}

/// GET /api/proposals/:id
async fn get_proposal(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    Proposal::find_by_id(&d.db().pool, id)
        .await?
        .map(|p| Json(ApiResponse::success(p)))
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))
}

/// PATCH /api/proposals/:id
async fn update_proposal(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProposal>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    Proposal::update(&d.db().pool, id, body)
        .await?
        .map(|p| Json(ApiResponse::success(p)))
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))
}

/// PATCH /api/proposals/:id/status
async fn move_proposal_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveStatusBody>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    Proposal::move_status(&d.db().pool, id, &body.status)
        .await?
        .map(|p| Json(ApiResponse::success(p)))
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))
}

/// DELETE /api/proposals/:id
async fn delete_proposal(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let deleted = Proposal::delete(&d.db().pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound("Proposal not found".into()))
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/proposals", get(list_proposals).post(create_proposal))
        .route(
            "/proposals/{id}",
            get(get_proposal)
                .patch(update_proposal)
                .delete(delete_proposal),
        )
        .route("/proposals/{id}/status", patch(move_proposal_status))
        .with_state(deployment.clone())
}

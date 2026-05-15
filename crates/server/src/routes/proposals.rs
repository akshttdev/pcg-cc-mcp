use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch},
};
use db::{
    db_uuid::DbUuid,
    models::proposal::{CreateProposal, Proposal, UpdateProposal},
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListProposalsParams {
    pub status: Option<String>,
    pub lead_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
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
        p.organization_id,
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
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
    Proposal::find_by_id(&d.db().pool, id_uuid)
        .await?
        .map(|p| Json(ApiResponse::success(p)))
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))
}

/// PATCH /api/proposals/:id
async fn update_proposal(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProposal>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
    Proposal::update(&d.db().pool, id_uuid, body)
        .await?
        .map(|p| Json(ApiResponse::success(p)))
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))
}

/// PATCH /api/proposals/:id/status
async fn move_proposal_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<MoveStatusBody>,
) -> Result<Json<ApiResponse<Proposal>>, ApiError> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
    let pool = &d.db().pool;
    let proposal = Proposal::move_status(pool, id_uuid, &body.status)
        .await?
        .ok_or_else(|| ApiError::NotFound("Proposal not found".into()))?;

    // Slack notify on terminal states. Soft-fail.
    if let Some(org_uuid) = proposal.organization_id {
        let event = match body.status.as_str() {
            "approved" | "contract_signed" => {
                Some(db::models::slack_channel_route::SlackEventType::ProposalApproved)
            }
            "declined" => Some(db::models::slack_channel_route::SlackEventType::ProposalRejected),
            _ => None,
        };
        if let Some(event) = event {
            let amount_usd = (proposal.quote_amount_vibe as f64) / 100.0; // 1 VIBE = $0.01
            let payload = serde_json::json!({
                "client_name": proposal.title,
                "deal_value_usd": amount_usd,
                "deliverables_count": serde_json::Value::Null,
                "project_url": proposal.project_id.map(|p| {
                    format!("/organizations/{}/projects/{}", org_uuid, p)
                }),
                "reason": serde_json::Value::Null,
            });
            let org_db = DbUuid::from_string(org_uuid.to_string());
            let _ = services::services::slack::dispatch_event(pool, &org_db, event, &payload).await;
        }
    }

    Ok(Json(ApiResponse::success(proposal)))
}

/// DELETE /api/proposals/:id
async fn delete_proposal(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
    let deleted = Proposal::delete(&d.db().pool, id_uuid).await?;
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

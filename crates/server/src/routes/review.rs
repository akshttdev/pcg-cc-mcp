//! Client Review routes — public shareable links (Shade-equivalent).
//!
//! GET  /review/{token}/data        — validate token, return deliverable + comments
//! POST /review/{token}/comments    — add comment (name, email, timecode, content)
//! PATCH /review/{token}/comments/{comment_id}/resolve — admin marks resolved

use axum::{
    Router,
    extract::{Path, State},
    routing::{get, patch, post},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::deliverable::Deliverable;
use db::models::review_comment::{CreateReviewComment, ReviewComment};
use db::models::review_token::ReviewToken;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ReviewData {
    pub token: ReviewToken,
    pub deliverable: Deliverable,
    pub comments: Vec<ReviewComment>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveBody {
    pub resolved_by: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /review/{token}/data — public endpoint
async fn get_review_data(
    State(d): State<DeploymentImpl>,
    Path(token): Path<String>,
) -> Result<Json<ApiResponse<ReviewData>>, ApiError> {
    let pool = &d.db().pool;

    let (tok, deliverable_id) = ReviewToken::validate_and_increment(pool, &token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Review link is invalid or expired".into()))?;

    let deliverable = Deliverable::find_by_id(pool, deliverable_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))?;

    let comments = ReviewComment::find_by_deliverable(pool, deliverable_id).await?;

    Ok(Json(ApiResponse::success(ReviewData {
        token: tok,
        deliverable,
        comments,
    })))
}

/// POST /review/{token}/comments — public endpoint
async fn add_comment(
    State(d): State<DeploymentImpl>,
    Path(token): Path<String>,
    Json(body): Json<CreateReviewComment>,
) -> Result<Json<ApiResponse<ReviewComment>>, ApiError> {
    let pool = &d.db().pool;

    let (tok, deliverable_id) = ReviewToken::validate_and_increment(pool, &token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Review link is invalid or expired".into()))?;

    let comment =
        ReviewComment::create(pool, deliverable_id, Some(tok.id), body).await?;

    Ok(Json(ApiResponse::success(comment)))
}

/// PATCH /review/{token}/comments/{comment_id}/resolve — public endpoint
async fn resolve_comment(
    State(d): State<DeploymentImpl>,
    Path((token, comment_id)): Path<(String, Uuid)>,
    Json(body): Json<ResolveBody>,
) -> Result<Json<ApiResponse<ReviewComment>>, ApiError> {
    let pool = &d.db().pool;

    // Validate the token is still active (but don't increment view count)
    let valid: Option<ReviewToken> = sqlx::query_as(
        "SELECT * FROM review_tokens WHERE token = ? AND is_active = 1",
    )
    .bind(&token)
    .fetch_optional(pool)
    .await?;

    valid.ok_or_else(|| ApiError::NotFound("Invalid review token".into()))?;

    let resolved_by = body.resolved_by.as_deref().unwrap_or("admin");
    let comment = ReviewComment::resolve(pool, comment_id, resolved_by)
        .await?
        .ok_or_else(|| ApiError::NotFound("Comment not found".into()))?;

    Ok(Json(ApiResponse::success(comment)))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/review/{token}/data", get(get_review_data))
        .route("/review/{token}/comments", post(add_comment))
        .route(
            "/review/{token}/comments/{comment_id}/resolve",
            patch(resolve_comment),
        )
        .with_state(deployment.clone())
}

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use db::models::artifact_review::{
    ArtifactReview, CreateArtifactReview, ResolveReview, ReviewStatus, ReviewType,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct CreateReviewPayload {
    pub artifact_id: Uuid,
    pub reviewer_id: Option<String>,
    pub reviewer_agent_id: Option<Uuid>,
    pub reviewer_name: Option<String>,
    pub review_type: ReviewType,
    pub feedback_text: Option<String>,
    pub rating: Option<i32>,
    pub revision_notes: Option<serde_json::Value>,
    pub revision_deadline: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListReviewsQuery {
    pub artifact_id: Option<Uuid>,
    pub reviewer_id: Option<String>,
    pub status: Option<ReviewStatus>,
    pub pending_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveReviewPayload {
    pub status: ReviewStatus,
    pub feedback_text: Option<String>,
    pub rating: Option<i32>,
    pub resolved_by: String,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/artifact-reviews", get(list_reviews).post(create_review))
        .route("/artifact-reviews/pending", get(list_pending_reviews))
        .route(
            "/artifact-reviews/{review_id}",
            get(get_review).delete(delete_review),
        )
        .route("/artifact-reviews/{review_id}/resolve", post(resolve_review))
        .with_state(deployment.clone())
}

async fn list_reviews(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListReviewsQuery>,
) -> Result<Json<ApiResponse<Vec<ArtifactReview>>>, ApiError> {
    let pool = &deployment.db().pool;

    let reviews = if let Some(artifact_id) = query.artifact_id {
        ArtifactReview::find_by_artifact(pool, artifact_id).await?
    } else if let Some(reviewer_id) = query.reviewer_id {
        ArtifactReview::find_pending_for_reviewer(pool, &reviewer_id).await?
    } else if query.pending_only.unwrap_or(false) {
        ArtifactReview::find_all_pending(pool).await?
    } else {
        // Return recent reviews
        sqlx::query_as::<_, ArtifactReview>(
            r#"SELECT * FROM artifact_reviews ORDER BY created_at DESC LIMIT 100"#,
        )
        .fetch_all(pool)
        .await?
    };

    Ok(Json(ApiResponse::success(reviews)))
}

async fn list_pending_reviews(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ArtifactReview>>>, ApiError> {
    let reviews = ArtifactReview::find_all_pending(&deployment.db().pool).await?;
    Ok(Json(ApiResponse::success(reviews)))
}

async fn create_review(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateReviewPayload>,
) -> Result<(StatusCode, Json<ApiResponse<ArtifactReview>>), ApiError> {
    let review = ArtifactReview::create(
        &deployment.db().pool,
        CreateArtifactReview {
            artifact_id: payload.artifact_id,
            reviewer_id: payload.reviewer_id,
            reviewer_agent_id: payload.reviewer_agent_id,
            reviewer_name: payload.reviewer_name,
            review_type: payload.review_type,
            feedback_text: payload.feedback_text,
            rating: payload.rating,
            revision_notes: payload.revision_notes,
            revision_deadline: payload.revision_deadline,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(review))))
}

async fn get_review(
    State(deployment): State<DeploymentImpl>,
    Path(review_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ArtifactReview>>, ApiError> {
    let review = ArtifactReview::find_by_id(&deployment.db().pool, review_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Artifact review not found".into()))?;

    Ok(Json(ApiResponse::success(review)))
}

async fn delete_review(
    State(deployment): State<DeploymentImpl>,
    Path(review_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify review exists
    ArtifactReview::find_by_id(pool, review_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Artifact review not found".into()))?;

    sqlx::query(r#"DELETE FROM artifact_reviews WHERE id = ?1"#)
        .bind(review_id)
        .execute(pool)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

async fn resolve_review(
    State(deployment): State<DeploymentImpl>,
    Path(review_id): Path<Uuid>,
    Json(payload): Json<ResolveReviewPayload>,
) -> Result<Json<ApiResponse<ArtifactReview>>, ApiError> {
    let review = ArtifactReview::resolve(
        &deployment.db().pool,
        review_id,
        ResolveReview {
            status: payload.status,
            feedback_text: payload.feedback_text,
            rating: payload.rating,
            resolved_by: payload.resolved_by,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(review)))
}

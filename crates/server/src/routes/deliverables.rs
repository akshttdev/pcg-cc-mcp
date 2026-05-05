use axum::{
    extract::{Path, State},
    routing::{get, patch},
    Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::{
        deliverable::{CreateDeliverable, Deliverable, UpdateDeliverable},
        project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
        review_token::ReviewToken,
        social_post::SocialPost,
    },
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

// ── Helpers ───────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct MoveStatusBody {
    pub status: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/projects/:project_id/deliverables
async fn list_deliverables(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Deliverable>>>, ApiError> {
    let project_id = DbUuid::parse(&project_id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    let items = Deliverable::list_for_project(&d.db().pool, project_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/projects/:project_id/deliverables
async fn create_deliverable(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<String>,
    Json(mut body): Json<CreateDeliverable>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    let project_id = DbUuid::parse(&project_id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    // Ensure the path project_id takes precedence
    body.project_id = project_id;
    let item = Deliverable::create(&d.db().pool, body).await?;
    Ok(Json(ApiResponse::success(item)))
}

/// GET /api/deliverables/:id
async fn get_deliverable(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    Deliverable::find_by_id(&d.db().pool, id)
        .await?
        .map(|x| Json(ApiResponse::success(x)))
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))
}

/// PATCH /api/deliverables/:id
async fn update_deliverable(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<UpdateDeliverable>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    Deliverable::update(&d.db().pool, id, body)
        .await?
        .map(|x| Json(ApiResponse::success(x)))
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))
}

/// PATCH /api/deliverables/:id/status
async fn move_deliverable_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<MoveStatusBody>,
) -> Result<Json<ApiResponse<Deliverable>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    let pool = &d.db().pool;
    let deliverable = Deliverable::move_status(pool, id, &body.status)
        .await?
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))?;

    // Auto-generate review link when moved to client_review
    if body.status == "client_review" {
        match ReviewToken::get_or_create(pool, id, None).await {
            Ok(tok) => tracing::info!("Review link: /review/{}", tok.token),
            Err(e) => tracing::warn!("Could not generate review token: {}", e),
        }
    }

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
        tracing::info!(
            "Deliverable {} registered in knowledge graph as artifact",
            id
        );

        // Auto-advance linked social post draft → pending_review
        if let Ok(Some(linked_post)) = SocialPost::find_by_deliverable(pool, id).await {
            if linked_post.status == "draft" {
                match SocialPost::advance_to_review(pool, linked_post.id, "", &[]).await {
                    Ok(updated) => {
                        let app_base = std::env::var("APP_BASE_URL")
                            .unwrap_or_else(|_| "http://localhost:3001".into());
                        let review_url = updated
                            .review_token
                            .as_deref()
                            .map(|t| format!("{}/api/social/review/{}", app_base, t))
                            .unwrap_or_default();
                        tracing::info!(
                            "Social post {} advanced to pending_review. Review: {}",
                            linked_post.id,
                            review_url
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Could not advance social post {}: {}", linked_post.id, e)
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(deliverable)))
}

/// GET /api/deliverables/:id/review-link — return active token URL
async fn get_review_link(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    let pool = &d.db().pool;
    let token = ReviewToken::get_or_create(pool, id, None).await?;
    let app_base = std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".into());
    let url = format!("{}/review/{}", app_base, token.token);
    Ok(Json(ApiResponse::success(serde_json::json!({
        "token": token.token,
        "url": url,
        "view_count": token.view_count,
        "is_active": token.is_active,
    }))))
}

/// DELETE /api/deliverables/:id
async fn delete_deliverable(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
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
        .route("/deliverables/{id}/review-link", get(get_review_link))
        .with_state(deployment.clone())
}

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, patch, post},
};
use cinematics::{CinematicsConfig, CinematicsService, Cinematographer};
use db::{
    db_uuid::DbUuid,
    models::{
        cinematic_brief::CreateCinematicBrief,
        deliverable::{CreateDeliverable, Deliverable, UpdateDeliverable},
        project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
        review_comment::ReviewComment,
        review_token::ReviewToken,
    },
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

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

/// GET /api/deliverables/:id/comments — return all review comments for a deliverable
/// Comments may be stored under deliverable_id or artifact_id (review page uses artifact_id as scope)
async fn get_deliverable_comments(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ReviewComment>>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    let pool = &d.db().pool;

    // Try comments by deliverable_id first
    let mut comments = ReviewComment::find_by_deliverable(pool, id).await?;

    // If none found, check if deliverable has an artifact_id and search by that
    // (the review page stores comments under artifact_id as the scope)
    if comments.is_empty() {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT artifact_id FROM deliverables WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?;

        if let Some((artifact_bytes,)) = row {
            if artifact_bytes.len() == 16 {
                if let Ok(artifact_id) = uuid::Uuid::from_slice(&artifact_bytes) {
                    comments = ReviewComment::find_by_deliverable(pool, artifact_id).await?;
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(comments)))
}

/// POST /api/deliverables/:id/dispatch-revision
/// Collects review comments, builds a revision brief, dispatches Editron, and moves status to revision.
async fn dispatch_revision(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let id = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?
        .to_uuid();
    let pool = &d.db().pool;

    // Load deliverable
    let deliverable = Deliverable::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))?;

    // Gather comments (try deliverable_id, fall back to artifact_id)
    let mut comments = ReviewComment::find_by_deliverable(pool, id).await?;
    if comments.is_empty() {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT artifact_id FROM deliverables WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?;
        if let Some((artifact_bytes,)) = row {
            if artifact_bytes.len() == 16 {
                if let Ok(artifact_id) = uuid::Uuid::from_slice(&artifact_bytes) {
                    comments = ReviewComment::find_by_deliverable(pool, artifact_id).await?;
                }
            }
        }
    }

    // Build revision script from comments
    let revision_notes: Vec<String> = comments
        .iter()
        .map(|c| {
            let tc = c
                .timecode_seconds
                .map(|s| {
                    let m = (s as u64) / 60;
                    let sec = (s as u64) % 60;
                    format!("[{m}:{sec:02}] ")
                })
                .unwrap_or_else(|| "[General] ".to_string());
            format!("{}{}", tc, c.content)
        })
        .collect();

    let next_version = deliverable.revision_rounds_used + 2; // current is v2, next is v3 etc
    let brief_title = format!("REVISION: {} — v{}", deliverable.title, next_version);
    let brief_summary = format!(
        "Revision pass for: {}\n\nOperator feedback:\n{}\n\nSirak Studios colour philosophy: enhance reality, not fabricate it.",
        deliverable.title,
        revision_notes.join("\n")
    );

    // Parse source_clips to extract asset IDs
    let asset_ids: Vec<uuid::Uuid> = {
        let clips_raw: Option<(String,)> =
            sqlx::query_as("SELECT source_clips FROM deliverables WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?;

        if let Some((clips_json,)) = clips_raw {
            serde_json::from_str::<serde_json::Value>(&clips_json)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|clip| {
                    clip["proxy"].as_str().and_then(|p| {
                        // extract artifact UUID from /api/artifacts/{uuid}/files/...
                        p.split('/')
                            .nth(3)
                            .and_then(|s| uuid::Uuid::parse_str(s).ok())
                    })
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        } else {
            vec![]
        }
    };

    // Dispatch Editron via CinematicsService
    let svc = CinematicsService::new(pool.clone(), CinematicsConfig::default());
    let brief = svc
        .create_brief(CreateCinematicBrief {
            project_id: deliverable.project_id,
            requester_id: "editron".to_string(),
            nora_session_id: None,
            title: brief_title.clone(),
            summary: brief_summary,
            script: None,
            asset_ids,
            duration_seconds: None,
            fps: None,
            style_tags: vec!["revision".to_string(), "interview".to_string()],
            metadata: Some(serde_json::json!({
                "deliverable_id": id.to_string(),
                "revision_round": deliverable.revision_rounds_used + 1,
                "source": "review_comments",
            })),
        })
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Auto-trigger render
    let brief = svc
        .trigger_render(brief.id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Move deliverable to revision (increments revision counter)
    Deliverable::move_status(pool, id, "revision").await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "brief_id": brief.id,
        "brief_title": brief_title,
        "status": brief.status,
        "comment_count": comments.len(),
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
        .route("/deliverables/{id}/comments", get(get_deliverable_comments))
        .route(
            "/deliverables/{id}/dispatch-revision",
            post(dispatch_revision),
        )
        .with_state(deployment.clone())
}

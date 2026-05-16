//! Client Review routes — public shareable links for deliverables and task artifacts.
//!
//! GET  /review/{token}/data        — validate token, return review payload + comments
//! POST /review/{token}/comments    — add comment (name, email, timecode, content)
//! PATCH /review/{token}/comments/{comment_id}/resolve — admin marks resolved
//! POST /api/artifacts/{artifact_id}/review-link — create/get artifact review token (auth)

use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use db::models::{
    deliverable::Deliverable,
    execution_artifact::ExecutionArtifact,
    review_comment::{CreateReviewComment, ReviewComment},
    review_token::ReviewToken,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::{assets::asset_dir, response::ApiResponse};
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ── Response types ────────────────────────────────────────────────────────────

/// Unified deliverable-like payload returned by both deliverable and artifact reviews.
#[derive(Debug, Serialize)]
pub struct ReviewDeliverablePayload {
    pub id: String,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub working_file_url: Option<String>,
    pub final_link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewData {
    pub token: ReviewToken,
    pub deliverable: ReviewDeliverablePayload,
    pub comments: Vec<ReviewComment>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveBody {
    pub resolved_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewLinkResponse {
    pub token: String,
    pub url: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Try to derive the video URL from the artifact's content JSON.
fn artifact_video_url_from_content(artifact: &ExecutionArtifact) -> Option<String> {
    let content = artifact.content.as_deref()?;
    let data: serde_json::Value = serde_json::from_str(content).ok()?;
    let items = data
        .get("deliverables")
        .or_else(|| data.get("edits"))
        .and_then(|v| v.as_array())?;
    let first = items.first()?;
    let file = first.get("file").and_then(|v| v.as_str()).or_else(|| {
        first
            .get("path")
            .and_then(|v| v.as_str())
            .and_then(|p| p.split('/').last())
    })?;
    Some(format!("/api/artifacts/{}/files/{}", artifact.id, file))
}

/// Find the most recently modified video file in the artifact's file_path directory.
/// Returns the URL `/api/artifacts/{id}/files/{filename}`.
fn artifact_video_url_from_dir(artifact: &ExecutionArtifact) -> Option<String> {
    let file_path = artifact.file_path.as_deref()?;
    let dir = {
        let p = std::path::Path::new(file_path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            asset_dir().join(file_path)
        }
    };
    if !dir.is_dir() {
        // file_path is a file itself — check if it's a video
        if dir.is_file() {
            let ext = dir
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if matches!(ext.as_str(), "mp4" | "mov" | "webm" | "avi" | "mkv") {
                let filename = dir.file_name()?.to_string_lossy().to_string();
                return Some(format!("/api/artifacts/{}/files/{}", artifact.id, filename));
            }
        }
        return None;
    }
    // List directory, pick latest video file by modification time
    let mut best: Option<(std::time::SystemTime, String)> = None;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !matches!(ext.as_str(), "mp4" | "mov" | "webm" | "avi" | "mkv") {
                continue;
            }
            let mtime = entry.metadata().ok()?.modified().ok()?;
            let fname = path.file_name()?.to_string_lossy().to_string();
            if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
                best = Some((mtime, fname));
            }
        }
    }
    let (_, filename) = best?;
    Some(format!("/api/artifacts/{}/files/{}", artifact.id, filename))
}

/// Resolve the best available video URL for an artifact.
fn resolve_artifact_video_url(artifact: &ExecutionArtifact) -> Option<String> {
    artifact_video_url_from_content(artifact).or_else(|| artifact_video_url_from_dir(artifact))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /review/{token}/data — public endpoint
async fn get_review_data(
    State(d): State<DeploymentImpl>,
    Path(token): Path<String>,
) -> Result<Json<ApiResponse<ReviewData>>, ApiError> {
    let pool = &d.db().pool;

    let tok = ReviewToken::validate_and_increment(pool, &token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Review link is invalid or expired".into()))?;

    let comments;
    let payload;

    if let Some(artifact_id) = tok.artifact_id {
        // Artifact-based review
        let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Artifact not found".into()))?;

        let video_url = tok
            .artifact_video_url
            .clone()
            .or_else(|| resolve_artifact_video_url(&artifact));

        payload = ReviewDeliverablePayload {
            id: artifact.id.to_string(),
            title: tok
                .artifact_title
                .clone()
                .unwrap_or_else(|| artifact.title.clone()),
            status: "active".to_string(),
            description: None,
            working_file_url: video_url,
            final_link: None,
        };

        // Use artifact_id as comment scope (stored as deliverable_id in review_comments)
        comments = ReviewComment::find_by_deliverable(pool, artifact_id).await?;
    } else if let Some(deliverable_id) = tok.deliverable_id {
        let deliverable = Deliverable::find_by_id(pool, deliverable_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Deliverable not found".into()))?;

        payload = ReviewDeliverablePayload {
            id: deliverable.id.to_string(),
            title: deliverable.title.clone(),
            status: deliverable.status.clone(),
            description: Some(deliverable.description.clone()),
            working_file_url: deliverable.working_file_url.clone(),
            final_link: deliverable.final_link.clone(),
        };

        comments = ReviewComment::find_by_deliverable(pool, deliverable_id).await?;
    } else {
        return Err(ApiError::InternalError(
            "Review token has no linked resource".into(),
        ));
    }

    Ok(Json(ApiResponse::success(ReviewData {
        token: tok,
        deliverable: payload,
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

    let tok = ReviewToken::validate_and_increment(pool, &token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Review link is invalid or expired".into()))?;

    // Use artifact_id or deliverable_id as the comment scope
    let scope_id = tok
        .artifact_id
        .or(tok.deliverable_id)
        .ok_or_else(|| ApiError::InternalError("Review token has no linked resource".into()))?;

    let comment = ReviewComment::create(pool, scope_id, Some(tok.id), body).await?;
    Ok(Json(ApiResponse::success(comment)))
}

/// PATCH /review/{token}/comments/{comment_id}/resolve — public endpoint
async fn resolve_comment(
    State(d): State<DeploymentImpl>,
    Path((token, comment_id)): Path<(String, Uuid)>,
    Json(body): Json<ResolveBody>,
) -> Result<Json<ApiResponse<ReviewComment>>, ApiError> {
    let pool = &d.db().pool;

    let valid: Option<ReviewToken> =
        sqlx::query_as("SELECT * FROM review_tokens WHERE token = ? AND is_active = 1")
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

/// POST /api/artifacts/{artifact_id}/review-link — authenticated, creates/gets review token
async fn get_artifact_review_link(
    State(d): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ReviewLinkResponse>>, ApiError> {
    let pool = &d.db().pool;

    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    let video_url = resolve_artifact_video_url(&artifact)
        .unwrap_or_else(|| format!("/api/artifacts/{}/download", artifact_id));

    let title = artifact.title.clone();

    let token =
        ReviewToken::get_or_create_for_artifact(pool, artifact_id, &video_url, &title, None)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create review token: {}", e))
            })?;

    let base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let url = format!("{}/review/{}", base_url, token.token);

    Ok(Json(ApiResponse::success(ReviewLinkResponse {
        token: token.token,
        url,
    })))
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

/// Protected routes (require session auth)
pub fn protected_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/artifacts/{artifact_id}/review-link",
            post(get_artifact_review_link),
        )
        .with_state(deployment.clone())
}

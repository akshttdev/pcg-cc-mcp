//! Social Post Management Routes
//!
//! Handles content CRUD, scheduling, and publishing operations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::{delete, get, patch, post},
    Json, Router,
};
use db::models::{
    social_approval_event::SocialApprovalEvent,
    social_post::{CreateSocialPost, SocialPost, UpdateSocialPost},
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    pub project_id: Option<Uuid>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub limit: Option<i64>,
}

/// GET /social/posts - List posts with filters
async fn list_posts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListPostsQuery>,
) -> Result<Json<ApiResponse<Vec<SocialPost>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Fetch all posts for the project (or all projects), then filter in-process
    let all_posts = if let Some(pid) = query.project_id {
        SocialPost::find_all_for_project(pool, pid).await?
    } else {
        SocialPost::find_all(pool).await?
    };

    let posts: Vec<SocialPost> = all_posts
        .into_iter()
        .filter(|p| query.status.as_deref().map_or(true, |s| p.status == s))
        .filter(|p| {
            query
                .category
                .as_deref()
                .map_or(true, |c| p.category.as_deref() == Some(c))
        })
        .take(query.limit.unwrap_or(500) as usize)
        .collect();

    Ok(Json(ApiResponse::success(posts)))
}

/// GET /social/posts/:id - Get single post
async fn get_post(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(post)))
}

/// POST /social/posts - Create new post
async fn create_post(
    State(deployment): State<DeploymentImpl>,
    Json(create): Json<CreateSocialPost>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::create(pool, create).await?;
    Ok(Json(ApiResponse::success(post)))
}

/// PATCH /social/posts/:id - Update post
async fn update_post(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateSocialPost>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(post)))
}

/// DELETE /social/posts/:id - Delete post
async fn delete_post(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    SocialPost::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /social/posts/due - Get posts due for publishing
async fn get_due_posts(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<SocialPost>>>, ApiError> {
    let pool = &deployment.db().pool;
    let posts = SocialPost::find_due_for_publish(pool).await?;
    Ok(Json(ApiResponse::success(posts)))
}

/// PATCH /social/posts/:id/status — FSM-guarded status transition
async fn transition_post_status(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<TransitionStatusBody>,
) -> Result<Json<ApiResponse<SocialPost>>, ApiError> {
    let pool = &deployment.db().pool;
    let post = SocialPost::transition_status(pool, id, &body.status, body.by.as_deref()).await?;

    // Log the approval event
    let _ = SocialApprovalEvent::create(
        pool,
        db::models::social_approval_event::CreateSocialApprovalEvent {
            post_id: id,
            actor: body.by.unwrap_or_else(|| "system".to_string()),
            from_status: post.status.clone(),
            to_status: body.status.clone(),
            note: body.note,
        },
    )
    .await;

    Ok(Json(ApiResponse::success(post)))
}

#[derive(Debug, Deserialize)]
pub struct TransitionStatusBody {
    pub status: String,
    pub by: Option<String>,
    pub note: Option<String>,
}

/// GET /social/review/:token — public review page (no auth)
pub async fn review_page(
    State(deployment): State<DeploymentImpl>,
    Path(token): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let pool = &deployment.db().pool;
    let post = SocialPost::find_by_review_token(pool, &token)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                "Review link not found or expired".to_string(),
            )
        })?;

    let media_html = post.media_urls.as_deref()
        .and_then(|m| serde_json::from_str::<Vec<String>>(m).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|url| format!(r#"<img src="{}" style="width:100%;max-width:540px;border-radius:8px;margin-bottom:12px;" />"#, html_escape(&url)))
        .collect::<Vec<_>>()
        .join("\n");

    let caption = post.caption.as_deref().unwrap_or("").replace('\n', "<br/>");
    let scheduled = post
        .scheduled_for
        .map(|d| d.format("%b %d, %Y at %H:%M UTC").to_string())
        .unwrap_or_else(|| "Not scheduled".to_string());
    let review_note = post.review_note.as_deref().unwrap_or("");

    let status_badge = match post.status.as_str() {
        "pending_review" => {
            r#"<span style="background:#FEF3C7;color:#92400E;padding:4px 12px;border-radius:20px;font-size:13px;font-weight:600;">⏳ Awaiting Review</span>"#
        }
        "approved" => {
            r#"<span style="background:#D1FAE5;color:#065F46;padding:4px 12px;border-radius:20px;font-size:13px;font-weight:600;">✓ Approved</span>"#
        }
        "draft" => {
            r#"<span style="background:#E0E7FF;color:#3730A3;padding:4px 12px;border-radius:20px;font-size:13px;font-weight:600;">✏ Changes Requested</span>"#
        }
        other => &format!(
            r#"<span style="background:#F3F4F6;color:#6B7280;padding:4px 12px;border-radius:20px;font-size:13px;">{}</span>"#,
            html_escape(other)
        ),
    };

    let action_buttons = if post.status == "pending_review" {
        format!(
            r#"
        <form method="POST" action="/api/social/review" style="display:flex;gap:12px;flex-wrap:wrap;">
          <input type="hidden" name="token" value="{token}" />
          <button name="action" value="approve"
            style="background:#005A9B;color:#fff;border:none;padding:12px 32px;border-radius:8px;font-size:16px;font-weight:600;cursor:pointer;flex:1;min-width:140px;">
            ✓ Approve &amp; Schedule
          </button>
          <div style="flex:1;min-width:200px;">
            <input name="note" placeholder="Change request note (optional)"
              style="width:100%;padding:10px 12px;border:1px solid #D1D5DB;border-radius:8px;font-size:14px;box-sizing:border-box;margin-bottom:8px;" />
            <button name="action" value="changes"
              style="background:#fff;color:#374151;border:1px solid #D1D5DB;padding:10px 20px;border-radius:8px;font-size:14px;cursor:pointer;width:100%;">
              ✗ Request Changes
            </button>
          </div>
        </form>
        "#
        )
    } else {
        String::new()
    };

    let prior_note_html = if !review_note.is_empty() {
        format!(
            r#"<div style="background:#FEF2F2;border:1px solid #FECACA;padding:12px 16px;border-radius:8px;margin-bottom:16px;font-size:14px;color:#991B1B;"><strong>Previous feedback:</strong> {}</div>"#,
            html_escape(review_note)
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <title>Review Post — ORCHA</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #F9FAFB; color: #111827; }}
    .container {{ max-width: 620px; margin: 0 auto; padding: 24px 16px 60px; }}
    .header {{ display: flex; align-items: center; gap: 12px; margin-bottom: 24px; padding-bottom: 16px; border-bottom: 1px solid #E5E7EB; }}
    .header-logo {{ width: 32px; height: 32px; background: #005A9B; border-radius: 6px; display: flex; align-items: center; justify-content: center; color: #fff; font-weight: 800; font-size: 14px; }}
    .card {{ background: #fff; border: 1px solid #E5E7EB; border-radius: 12px; padding: 20px; margin-bottom: 16px; }}
    .meta-row {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }}
    .platform {{ font-size: 12px; color: #6B7280; text-transform: uppercase; letter-spacing: 0.05em; }}
    .caption {{ font-size: 15px; line-height: 1.6; color: #374151; margin-bottom: 12px; white-space: pre-wrap; }}
    .scheduled {{ font-size: 13px; color: #6B7280; margin-top: 12px; }}
    .action-card {{ background: #fff; border: 1px solid #E5E7EB; border-radius: 12px; padding: 20px; }}
    .action-title {{ font-size: 14px; font-weight: 600; margin-bottom: 16px; color: #374151; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div class="header-logo">O</div>
      <div>
        <div style="font-size:16px;font-weight:700;">ORCHA — Content Review</div>
        <div style="font-size:12px;color:#6B7280;">Review and approve this social post before it publishes</div>
      </div>
    </div>

    {prior_note_html}

    <div class="card">
      <div class="meta-row">
        <span class="platform">LinkedIn Post</span>
        {status_badge}
      </div>
      {media_html}
      <div class="caption">{caption}</div>
      <div class="scheduled">📅 Scheduled: {scheduled}</div>
    </div>

    <div class="action-card">
      <div class="action-title">Your Decision</div>
      {action_buttons}
    </div>
  </div>
</body>
</html>"#
    );

    Ok(Html(html))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// POST /api/social/review — process approve/reject from review page form
pub async fn submit_review(
    State(deployment): State<DeploymentImpl>,
    body: axum::extract::Form<ReviewSubmitForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let pool = &deployment.db().pool;
    let post = SocialPost::find_by_review_token(pool, &body.token)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                "Review link not found or expired".to_string(),
            )
        })?;

    if post.status != "pending_review" {
        let msg = match post.status.as_str() {
            "approved" => "This post has already been approved.",
            "published" => "This post has already been published.",
            "cancelled" => "This post has been cancelled.",
            _ => "This post is no longer awaiting review.",
        };
        return Ok(Html(confirmation_page(msg, "#065F46", "#D1FAE5")));
    }

    match body.action.as_str() {
        "approve" => {
            let reviewer = body
                .reviewer
                .clone()
                .unwrap_or_else(|| "reviewer".to_string());
            SocialPost::approve_post(pool, post.id, &reviewer)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let _ = SocialApprovalEvent::create(
                pool,
                db::models::social_approval_event::CreateSocialApprovalEvent {
                    post_id: post.id,
                    actor: reviewer,
                    from_status: "pending_review".to_string(),
                    to_status: "approved".to_string(),
                    note: None,
                },
            )
            .await;

            Ok(Html(confirmation_page(
                "Post approved! It will publish at the scheduled time.",
                "#065F46",
                "#D1FAE5",
            )))
        }
        "changes" => {
            let note = body.note.clone().unwrap_or_default();
            let reviewer = body
                .reviewer
                .clone()
                .unwrap_or_else(|| "reviewer".to_string());
            SocialPost::request_changes(pool, post.id, &note, &reviewer)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let _ = SocialApprovalEvent::create(
                pool,
                db::models::social_approval_event::CreateSocialApprovalEvent {
                    post_id: post.id,
                    actor: reviewer,
                    from_status: "pending_review".to_string(),
                    to_status: "draft".to_string(),
                    note: Some(note),
                },
            )
            .await;

            Ok(Html(confirmation_page(
                "Changes requested. The team has been notified.",
                "#92400E",
                "#FEF3C7",
            )))
        }
        _ => Err((StatusCode::BAD_REQUEST, "Invalid action".to_string())),
    }
}

fn confirmation_page(msg: &str, text_color: &str, bg_color: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"/><title>Review Complete</title>
<style>body{{font-family:-apple-system,sans-serif;background:#F9FAFB;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0;}}</style>
</head>
<body>
  <div style="background:{bg_color};border-radius:12px;padding:40px;max-width:400px;text-align:center;">
    <div style="font-size:48px;margin-bottom:16px;">✓</div>
    <div style="font-size:18px;font-weight:600;color:{text_color};">{msg}</div>
    <div style="font-size:13px;color:#6B7280;margin-top:12px;">You can close this window.</div>
  </div>
</body>
</html>"#
    )
}

#[derive(Debug, Deserialize)]
pub struct ReviewSubmitForm {
    pub token: String,
    pub action: String,
    pub note: Option<String>,
    pub reviewer: Option<String>,
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/posts", get(list_posts))
        .route("/social/posts", post(create_post))
        .route("/social/posts/due", get(get_due_posts))
        .route("/social/posts/{id}", get(get_post))
        .route("/social/posts/{id}", patch(update_post))
        .route("/social/posts/{id}", delete(delete_post))
        .route("/social/posts/{id}/status", patch(transition_post_status))
}

pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/review/{token}", get(review_page))
        .route("/social/review", post(submit_review))
}

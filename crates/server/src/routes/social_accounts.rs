//! Social Account Management Routes
//!
//! Handles OAuth connections, account management, and platform integrations.

use axum::{
    Router,
    extract::{Path, Query, State},
    response::Html,
    routing::{get, delete, patch},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::social_account::{SocialAccount, UpdateSocialAccount};

#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    pub project_id: Option<Uuid>,
    pub platform: Option<String>,
    pub active_only: Option<bool>,
}

/// GET /social/accounts - List social accounts
async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<ApiResponse<Vec<SocialAccount>>>, ApiError> {
    let pool = &deployment.db().pool;

    let accounts = if let Some(project_id) = query.project_id {
        SocialAccount::find_by_project(pool, project_id).await?
    } else {
        SocialAccount::find_active(pool).await?
    };

    // Filter by platform if specified
    let accounts = if let Some(platform) = query.platform {
        accounts
            .into_iter()
            .filter(|a| a.platform == platform)
            .collect()
    } else {
        accounts
    };

    Ok(Json(ApiResponse::success(accounts)))
}

/// GET /social/accounts/:id - Get single account
async fn get_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SocialAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    let account = SocialAccount::find_by_id(pool, id_uuid).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// PATCH /social/accounts/:id - Update account
async fn update_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(update): Json<UpdateSocialAccount>,
) -> Result<Json<ApiResponse<SocialAccount>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    let account = SocialAccount::update(pool, id_uuid, update).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// DELETE /social/accounts/:id - Disconnect account
async fn delete_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    SocialAccount::delete(pool, id_uuid).await?;
    Ok(Json(ApiResponse::success(())))
}

/// Best time slot for posting
#[derive(Debug, Serialize)]
pub struct BestTimeSlot {
    pub day_of_week: i64,   // 0=Sunday, 6=Saturday
    pub hour_of_day: i64,
    pub post_count: i64,
    pub avg_engagement: f64,
}

/// GET /social/accounts/{id}/best-times — suggest optimal posting times
async fn get_best_times(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<BestTimeSlot>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;

    #[derive(sqlx::FromRow)]
    struct Row {
        day: i64,
        hour: i64,
        cnt: i64,
        eng: f64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"SELECT
           CAST(strftime('%w', published_at) AS INTEGER) AS day,
           CAST(strftime('%H', published_at) AS INTEGER) AS hour,
           COUNT(*) AS cnt,
           AVG(COALESCE(engagement_rate, 0.0)) AS eng
           FROM social_posts
           WHERE social_account_id = ? AND status = 'published' AND impressions > 0
           GROUP BY day, hour
           ORDER BY eng DESC
           LIMIT 10"#,
    )
    .bind(id_uuid)
    .fetch_all(pool)
    .await?;

    let slots = rows
        .into_iter()
        .map(|r| BestTimeSlot {
            day_of_week: r.day,
            hour_of_day: r.hour,
            post_count: r.cnt,
            avg_engagement: r.eng,
        })
        .collect();

    Ok(Json(ApiResponse::success(slots)))
}

/// GET /bio/{username} — public Link in Bio page
pub async fn bio_page(
    State(deployment): State<DeploymentImpl>,
    Path(username): Path<String>,
) -> Result<Html<String>, ApiError> {
    let pool = &deployment.db().pool;

    // Find social account by username or project slug
    let account: Option<SocialAccount> = sqlx::query_as(
        "SELECT * FROM social_accounts WHERE username = ? AND status = 'active' LIMIT 1",
    )
    .bind(&username)
    .fetch_optional(pool)
    .await?;

    let Some(acct) = account else {
        return Err(ApiError::NotFound(format!("No bio found for @{}", username)));
    };

    // Get recent published posts
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct PostRow {
        platform_post_id: Option<String>,
        platform_url: Option<String>,
        caption: Option<String>,
    }

    let posts: Vec<PostRow> = sqlx::query_as(
        "SELECT platform_post_id, platform_url, caption FROM social_posts \
         WHERE social_account_id = ? AND status = 'published' \
         ORDER BY published_at DESC LIMIT 3",
    )
    .bind(acct.id)
    .fetch_all(pool)
    .await?;

    let display_name = acct.display_name.as_deref().unwrap_or(&username);
    let avatar_url = acct.avatar_url.as_deref().unwrap_or("");
    let platform = acct.platform.as_str();

    let post_links: String = posts
        .iter()
        .filter_map(|p| p.platform_url.as_deref())
        .enumerate()
        .map(|(i, url)| {
            format!(
                r#"<a href="{}" target="_blank" rel="noopener" class="post-link">Post {}</a>"#,
                url,
                i + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let profile_url = acct
        .profile_url
        .as_deref()
        .unwrap_or("#");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>@{username} | Link in Bio</title>
<style>
  body {{font-family:system-ui,sans-serif;max-width:480px;margin:0 auto;padding:2rem;text-align:center;background:#0f0f0f;color:#fff}}
  img {{width:80px;height:80px;border-radius:50%;object-fit:cover;margin-bottom:1rem}}
  h1 {{font-size:1.5rem;margin:0 0 .25rem}}
  .platform {{color:#aaa;font-size:.9rem;margin-bottom:1.5rem}}
  .post-link {{display:block;padding:.75rem 1rem;margin:.5rem 0;background:#1e1e1e;border-radius:.5rem;color:#fff;text-decoration:none}}
  .post-link:hover {{background:#2a2a2a}}
  .profile-link {{margin-top:1.5rem;color:#6c8cef;text-decoration:none;font-size:.9rem}}
</style>
</head>
<body>
{avatar_html}
<h1>{display_name}</h1>
<div class="platform">@{username} on {platform}</div>
{post_links}
<a class="profile-link" href="{profile_url}" target="_blank" rel="noopener">View {platform} profile →</a>
</body>
</html>"#,
        username = username,
        display_name = display_name,
        platform = platform,
        avatar_html = if avatar_url.is_empty() {
            String::new()
        } else {
            format!(r#"<img src="{}" alt="{}" />"#, avatar_url, display_name)
        },
        post_links = post_links,
        profile_url = profile_url,
    );

    Ok(Html(html))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/accounts", get(list_accounts))
        .route("/social/accounts/{id}", get(get_account))
        .route("/social/accounts/{id}", patch(update_account))
        .route("/social/accounts/{id}", delete(delete_account))
        .route("/social/accounts/{id}/best-times", get(get_best_times))
        .with_state(_deployment.clone())
}

/// Standalone public router for bio pages (no auth)
pub fn bio_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/bio/{username}", get(bio_page))
        .with_state(deployment.clone())
}

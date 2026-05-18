//! Social Account Management Routes
//!
//! Handles OAuth connections, account management, and platform integrations.

use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::{delete, get, patch, post},
    Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::social_account::{SocialAccount, UpdateSocialAccount},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::workflow_llm::WorkflowLLMService;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    pub project_id: Option<Uuid>,
    pub organization_id: Option<String>,
    pub platform: Option<String>,
    pub active_only: Option<bool>,
}

/// GET /social/accounts - List social accounts
async fn list_accounts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<ApiResponse<Vec<SocialAccount>>>, ApiError> {
    let pool = &deployment.db().pool;

    let accounts = if let Some(org_id) = query.organization_id.as_deref() {
        SocialAccount::find_by_organization(pool, org_id).await?
    } else if let Some(project_id) = query.project_id {
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
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
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
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
    let account = SocialAccount::update(pool, id_uuid, update).await?;
    Ok(Json(ApiResponse::success(account)))
}

/// DELETE /social/accounts/:id - Disconnect account
async fn delete_account(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();
    SocialAccount::delete(pool, id_uuid).await?;
    Ok(Json(ApiResponse::success(())))
}

/// Best time slot for posting
#[derive(Debug, Serialize)]
pub struct BestTimeSlot {
    pub day_of_week: i64, // 0=Sunday, 6=Saturday
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
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();

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

// ── Analytics endpoint ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub project_id: Uuid,
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummary {
    pub total_impressions: i64,
    pub total_reach: i64,
    pub total_likes: i64,
    pub total_comments: i64,
    pub total_shares: i64,
    pub total_saves: i64,
    pub total_clicks: i64,
    pub posts_published: i64,
    pub avg_engagement_rate: f64,
    pub daily: Vec<DailyMetrics>,
    pub by_platform: Vec<PlatformMetrics>,
}

#[derive(Debug, Serialize)]
pub struct DailyMetrics {
    pub date: String,
    pub impressions: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub posts: i64,
}

#[derive(Debug, Serialize)]
pub struct PlatformMetrics {
    pub platform: String,
    pub posts_published: i64,
    pub total_impressions: i64,
    pub total_likes: i64,
    pub avg_engagement_rate: f64,
}

async fn get_analytics(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<AnalyticsSummary>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = query.days.unwrap_or(30);

    #[derive(sqlx::FromRow)]
    struct TotalsRow {
        total_impressions: i64,
        total_reach: i64,
        total_likes: i64,
        total_comments: i64,
        total_shares: i64,
        total_saves: i64,
        total_clicks: i64,
        posts_published: i64,
        avg_engagement_rate: f64,
    }

    let totals: TotalsRow = sqlx::query_as(
        r#"SELECT
            COALESCE(SUM(impressions),0) AS total_impressions,
            COALESCE(SUM(reach),0) AS total_reach,
            COALESCE(SUM(likes),0) AS total_likes,
            COALESCE(SUM(comments),0) AS total_comments,
            COALESCE(SUM(shares),0) AS total_shares,
            COALESCE(SUM(saves),0) AS total_saves,
            COALESCE(SUM(clicks),0) AS total_clicks,
            COUNT(*) AS posts_published,
            COALESCE(AVG(engagement_rate),0.0) AS avg_engagement_rate
           FROM social_posts
           WHERE project_id = ?1 AND status = 'published'
             AND published_at >= datetime('now', '-' || ?2 || ' days')"#,
    )
    .bind(query.project_id.to_string())
    .bind(days)
    .fetch_one(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct DailyRow {
        date: String,
        impressions: i64,
        likes: i64,
        comments: i64,
        shares: i64,
        posts: i64,
    }

    let daily_rows: Vec<DailyRow> = sqlx::query_as(
        r#"SELECT
            date(published_at) AS date,
            COALESCE(SUM(impressions),0) AS impressions,
            COALESCE(SUM(likes),0) AS likes,
            COALESCE(SUM(comments),0) AS comments,
            COALESCE(SUM(shares),0) AS shares,
            COUNT(*) AS posts
           FROM social_posts
           WHERE project_id = ?1 AND status = 'published'
             AND published_at >= datetime('now', '-' || ?2 || ' days')
           GROUP BY date(published_at)
           ORDER BY date ASC"#,
    )
    .bind(query.project_id.to_string())
    .bind(days)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct PlatformRow {
        platform: String,
        posts_published: i64,
        total_impressions: i64,
        total_likes: i64,
        avg_engagement_rate: f64,
    }

    // social_posts.platforms is a JSON array; extract first platform name for grouping
    let platform_rows: Vec<PlatformRow> = sqlx::query_as(
        r#"SELECT
            json_extract(platforms, '$[0]') AS platform,
            COUNT(*) AS posts_published,
            COALESCE(SUM(impressions),0) AS total_impressions,
            COALESCE(SUM(likes),0) AS total_likes,
            COALESCE(AVG(engagement_rate),0.0) AS avg_engagement_rate
           FROM social_posts
           WHERE project_id = ?1 AND status = 'published'
             AND published_at >= datetime('now', '-' || ?2 || ' days')
           GROUP BY json_extract(platforms, '$[0]')
           ORDER BY posts_published DESC"#,
    )
    .bind(query.project_id.to_string())
    .bind(days)
    .fetch_all(pool)
    .await?;

    Ok(Json(ApiResponse::success(AnalyticsSummary {
        total_impressions: totals.total_impressions,
        total_reach: totals.total_reach,
        total_likes: totals.total_likes,
        total_comments: totals.total_comments,
        total_shares: totals.total_shares,
        total_saves: totals.total_saves,
        total_clicks: totals.total_clicks,
        posts_published: totals.posts_published,
        avg_engagement_rate: totals.avg_engagement_rate,
        daily: daily_rows
            .into_iter()
            .map(|r| DailyMetrics {
                date: r.date,
                impressions: r.impressions,
                likes: r.likes,
                comments: r.comments,
                shares: r.shares,
                posts: r.posts,
            })
            .collect(),
        by_platform: platform_rows
            .into_iter()
            .map(|r| PlatformMetrics {
                platform: r.platform,
                posts_published: r.posts_published,
                total_impressions: r.total_impressions,
                total_likes: r.total_likes,
                avg_engagement_rate: r.avg_engagement_rate,
            })
            .collect(),
    })))
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
        return Err(ApiError::NotFound(format!(
            "No bio found for @{}",
            username
        )));
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

    let profile_url = acct.profile_url.as_deref().unwrap_or("#");

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

// ── Top posts analytics endpoint ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TopPostsQuery {
    pub project_id: Option<Uuid>,
    pub organization_id: Option<String>,
    pub limit: Option<i64>,
    /// One of: impressions, likes, comments, shares, saves, clicks, engagement_rate
    pub metric: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TopPost {
    // BLOB primary key — decoded as Uuid via sqlx uuid feature, serialised to string in JSON
    pub id: Uuid,
    pub caption: Option<String>,
    pub platform_url: Option<String>,
    pub platforms: String,
    pub published_at: Option<String>,
    pub impressions: i64,
    pub reach: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub saves: i64,
    pub clicks: i64,
    pub engagement_rate: f64,
}

/// GET /social/analytics/top-posts?project_id=&limit=&metric=
async fn get_top_posts(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<TopPostsQuery>,
) -> Result<Json<ApiResponse<Vec<TopPost>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = q.limit.unwrap_or(10).min(100);

    // Validate (and sanitise) the sort metric to prevent SQL injection
    let order_col = match q.metric.as_deref().unwrap_or("impressions") {
        "likes" => "likes",
        "comments" => "comments",
        "shares" => "shares",
        "saves" => "saves",
        "clicks" => "clicks",
        "engagement_rate" => "engagement_rate",
        "reach" => "reach",
        _ => "impressions",
    };

    let sql = format!(
        "SELECT id, caption, platform_url, platforms, \
                date(published_at) AS published_at, \
                impressions, reach, likes, comments, shares, saves, clicks, engagement_rate \
         FROM social_posts \
         WHERE status = 'published' \
           AND ({owner_filter}) \
         ORDER BY {order_col} DESC \
         LIMIT ?3",
        owner_filter = "project_id = ?1 OR organization_id = ?2",
        order_col = order_col,
    );

    let project_id_str = q.project_id.map(|id| id.to_string());
    let rows: Vec<TopPost> = sqlx::query_as(&sql)
        .bind(&project_id_str)
        .bind(&q.organization_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    Ok(Json(ApiResponse::success(rows)))
}

// ── Post growth curve endpoint ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GrowthQuery {
    pub post_id: Uuid,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MetricsSnapshot {
    pub captured_at: String,
    pub impressions: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub saves: i64,
    pub clicks: i64,
    pub engagement_rate: f64,
}

/// GET /social/analytics/growth?post_id=
///
/// Returns the time-series snapshot rows for a single post, ordered ASC by
/// capture time so the caller can draw a growth curve.
async fn get_post_growth(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<GrowthQuery>,
) -> Result<Json<ApiResponse<Vec<MetricsSnapshot>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows: Vec<MetricsSnapshot> = sqlx::query_as(
        "SELECT captured_at, impressions, likes, comments, shares, saves, clicks, engagement_rate \
         FROM social_post_metrics_snapshots \
         WHERE post_id = ? \
         ORDER BY captured_at ASC",
    )
    .bind(q.post_id)
    .fetch_all(pool)
    .await?;
    Ok(Json(ApiResponse::success(rows)))
}

// ── Benchmark heatmap endpoint ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BenchmarkHeatmapEntry {
    pub day_of_week: i64,
    pub hour_of_day: i64,
    pub avg_saves: f64,
    pub avg_engagement_rate: f64,
    pub avg_reach: f64,
    pub sample_count: i64,
    pub confidence: f64,
}

/// GET /social/accounts/:id/benchmarks — learned optimal posting time heatmap
async fn get_benchmarks(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<BenchmarkHeatmapEntry>>>, ApiError> {
    use db::models::social_performance_benchmark::SocialPerformanceBenchmark;

    let pool = &deployment.db().pool;
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| ApiError::BadRequest("Invalid ID".into()))?
        .to_uuid();

    let rows = SocialPerformanceBenchmark::get_heatmap(pool, &id_uuid.to_string())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let entries = rows
        .into_iter()
        .map(|r| BenchmarkHeatmapEntry {
            day_of_week: r.day_of_week,
            hour_of_day: r.hour_of_day,
            avg_saves: r.avg_saves,
            avg_engagement_rate: r.avg_engagement_rate,
            avg_reach: r.avg_reach,
            sample_count: r.sample_count,
            confidence: r.confidence,
        })
        .collect();

    Ok(Json(ApiResponse::success(entries)))
}

// ── Nora AI insights endpoint ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct InsightsRequest {
    project_id: Option<Uuid>,
    org_id: Option<String>,
    days: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InsightsSummary {
    summary: String,
    top_finding: String,
    recommendations: Vec<String>,
    generated_at: String,
}

/// POST /social/analytics/insights — AI-generated social media insights
async fn get_insights(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<InsightsRequest>,
) -> Result<Json<ApiResponse<InsightsSummary>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = body.days.unwrap_or(30);

    let project_id_str = body.project_id.map(|u| u.to_string()).unwrap_or_default();
    let org_id_str = body.org_id.clone().unwrap_or_default();

    // ── Aggregate totals ──────────────────────────────────────────────────────

    #[derive(sqlx::FromRow)]
    struct TotalsRow {
        total_posts: i64,
        total_impressions: i64,
        total_likes: i64,
        total_comments: i64,
        avg_engagement: f64,
    }

    let totals: Option<TotalsRow> = sqlx::query_as(
        r#"SELECT
            COUNT(*) AS total_posts,
            COALESCE(SUM(impressions), 0) AS total_impressions,
            COALESCE(SUM(likes), 0) AS total_likes,
            COALESCE(SUM(comments), 0) AS total_comments,
            COALESCE(AVG(COALESCE(engagement_rate, 0.0)), 0.0) AS avg_engagement
           FROM social_posts
           WHERE status = 'published'
             AND (project_id = ?1 OR organization_id = ?2)
             AND published_at >= datetime('now', '-' || ?3 || ' days')"#,
    )
    .bind(&project_id_str)
    .bind(&org_id_str)
    .bind(days)
    .fetch_optional(pool)
    .await?;

    let totals = totals.unwrap_or(TotalsRow {
        total_posts: 0,
        total_impressions: 0,
        total_likes: 0,
        total_comments: 0,
        avg_engagement: 0.0,
    });

    // ── Best performing platform ──────────────────────────────────────────────

    #[derive(sqlx::FromRow)]
    struct PlatformRow {
        platform: Option<String>,
    }

    let best_platform: Option<PlatformRow> = sqlx::query_as(
        r#"SELECT json_extract(platforms, '$[0]') AS platform
           FROM social_posts
           WHERE status = 'published'
             AND (project_id = ?1 OR organization_id = ?2)
             AND published_at >= datetime('now', '-' || ?3 || ' days')
           GROUP BY json_extract(platforms, '$[0]')
           ORDER BY AVG(COALESCE(engagement_rate, 0.0)) DESC
           LIMIT 1"#,
    )
    .bind(&project_id_str)
    .bind(&org_id_str)
    .bind(days)
    .fetch_optional(pool)
    .await?;

    let best_platform_name = best_platform
        .and_then(|r| r.platform)
        .unwrap_or_else(|| "unknown".to_string());

    // ── Best day of week ──────────────────────────────────────────────────────

    #[derive(sqlx::FromRow)]
    struct DayRow {
        day: Option<i64>,
    }

    let best_day: Option<DayRow> = sqlx::query_as(
        r#"SELECT CAST(strftime('%w', published_at) AS INTEGER) AS day
           FROM social_posts
           WHERE status = 'published'
             AND (project_id = ?1 OR organization_id = ?2)
             AND published_at >= datetime('now', '-' || ?3 || ' days')
             AND published_at IS NOT NULL
           GROUP BY day
           ORDER BY AVG(COALESCE(engagement_rate, 0.0)) DESC
           LIMIT 1"#,
    )
    .bind(&project_id_str)
    .bind(&org_id_str)
    .bind(days)
    .fetch_optional(pool)
    .await?;

    let day_names = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let best_day_name = best_day
        .and_then(|r| r.day)
        .map(|d| day_names.get(d as usize).copied().unwrap_or("unknown"))
        .unwrap_or("unknown");

    // ── Best hour of day ──────────────────────────────────────────────────────

    #[derive(sqlx::FromRow)]
    struct HourRow {
        hour: Option<i64>,
    }

    let best_hour: Option<HourRow> = sqlx::query_as(
        r#"SELECT CAST(strftime('%H', published_at) AS INTEGER) AS hour
           FROM social_posts
           WHERE status = 'published'
             AND (project_id = ?1 OR organization_id = ?2)
             AND published_at >= datetime('now', '-' || ?3 || ' days')
             AND published_at IS NOT NULL
           GROUP BY hour
           ORDER BY AVG(COALESCE(engagement_rate, 0.0)) DESC
           LIMIT 1"#,
    )
    .bind(&project_id_str)
    .bind(&org_id_str)
    .bind(days)
    .fetch_optional(pool)
    .await?;

    let best_hour_val = best_hour.and_then(|r| r.hour).unwrap_or(12);

    // ── Top post caption ──────────────────────────────────────────────────────

    #[derive(sqlx::FromRow)]
    struct TopCaptionRow {
        caption: Option<String>,
    }

    let top_post: Option<TopCaptionRow> = sqlx::query_as(
        r#"SELECT caption
           FROM social_posts
           WHERE status = 'published'
             AND (project_id = ?1 OR organization_id = ?2)
             AND published_at >= datetime('now', '-' || ?3 || ' days')
           ORDER BY COALESCE(engagement_rate, 0.0) DESC
           LIMIT 1"#,
    )
    .bind(&project_id_str)
    .bind(&org_id_str)
    .bind(days)
    .fetch_optional(pool)
    .await?;

    let top_caption = top_post
        .and_then(|r| r.caption)
        .unwrap_or_else(|| "(no caption)".to_string());

    // Truncate caption for prompt to avoid token bloat
    let top_caption_short = if top_caption.len() > 200 {
        format!("{}...", &top_caption[..200])
    } else {
        top_caption.clone()
    };

    // ── Build AI prompt ───────────────────────────────────────────────────────

    let data_prompt = format!(
        "Social media analytics for the last {days} days:\n\
         - Total posts published: {total_posts}\n\
         - Total impressions: {total_impressions}\n\
         - Total likes: {total_likes}\n\
         - Total comments: {total_comments}\n\
         - Average engagement rate: {avg_engagement:.2}%\n\
         - Best performing platform: {best_platform}\n\
         - Best day of week: {best_day}\n\
         - Best hour of day: {best_hour}:00\n\
         - Top post caption: \"{top_caption}\"\n\n\
         Provide concrete, actionable insights based on these numbers.",
        days = days,
        total_posts = totals.total_posts,
        total_impressions = totals.total_impressions,
        total_likes = totals.total_likes,
        total_comments = totals.total_comments,
        avg_engagement = totals.avg_engagement * 100.0,
        best_platform = best_platform_name,
        best_day = best_day_name,
        best_hour = best_hour_val,
        top_caption = top_caption_short,
    );

    let messages = vec![
        WorkflowLLMService::system_message(
            "You are a social media analytics expert. Analyze the metrics and give concrete, \
             actionable insights in JSON format with keys: summary (2-3 sentences), \
             top_finding (1 sentence), recommendations (array of exactly 3 short action items). \
             Be specific with numbers. Respond ONLY with valid JSON.",
        ),
        WorkflowLLMService::user_message(&data_prompt),
    ];

    let (text, _) = WorkflowLLMService::completion(
        pool,
        messages,
        Some("claude-haiku-4-5-20251001"),
        Some(512),
        Some(0.3),
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("AI insights failed: {e}")))?;

    // ── Parse JSON response ───────────────────────────────────────────────────

    #[derive(Deserialize)]
    struct AiResponse {
        summary: String,
        top_finding: String,
        recommendations: Vec<String>,
    }

    // Strip markdown code fences if present
    let clean = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let summary = match serde_json::from_str::<AiResponse>(clean) {
        Ok(ai) => InsightsSummary {
            summary: ai.summary,
            top_finding: ai.top_finding,
            recommendations: ai.recommendations,
            generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        },
        Err(_) => InsightsSummary {
            summary: text,
            top_finding: String::new(),
            recommendations: vec![],
            generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        },
    };

    Ok(Json(ApiResponse::success(summary)))
}

// ── Project-level best times heatmap endpoint ─────────────────────────────────

#[derive(Debug, Deserialize)]
struct ProjectBestTimesQuery {
    project_id: Option<Uuid>,
    org_id: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct HeatmapCell {
    day_of_week: i64,
    hour_of_day: i64,
    post_count: i64,
    avg_engagement: f64,
}

/// GET /social/analytics/best-times?project_id= — 7×24 heatmap for project/org
async fn get_project_best_times(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<ProjectBestTimesQuery>,
) -> Result<Json<ApiResponse<Vec<HeatmapCell>>>, ApiError> {
    let pool = &deployment.db().pool;

    let project_id_str = q.project_id.map(|u| u.to_string()).unwrap_or_default();
    let org_id_str = q.org_id.unwrap_or_default();

    let rows: Vec<HeatmapCell> = sqlx::query_as(
        r#"SELECT
            CAST(strftime('%w', published_at) AS INTEGER) AS day_of_week,
            CAST(strftime('%H', published_at) AS INTEGER) AS hour_of_day,
            COUNT(*) AS post_count,
            AVG(COALESCE(engagement_rate, 0.0)) AS avg_engagement
           FROM social_posts
           WHERE (project_id = ?1 OR organization_id = ?2)
             AND status = 'published'
             AND published_at IS NOT NULL
           GROUP BY day_of_week, hour_of_day
           ORDER BY day_of_week, hour_of_day"#,
    )
    .bind(&project_id_str)
    .bind(&org_id_str)
    .fetch_all(pool)
    .await?;

    Ok(Json(ApiResponse::success(rows)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/social/accounts", get(list_accounts))
        .route("/social/accounts/{id}", get(get_account))
        .route("/social/accounts/{id}", patch(update_account))
        .route("/social/accounts/{id}", delete(delete_account))
        .route("/social/accounts/{id}/best-times", get(get_best_times))
        .route("/social/accounts/{id}/benchmarks", get(get_benchmarks))
        .route("/social/analytics", get(get_analytics))
        .route("/social/analytics/top-posts", get(get_top_posts))
        .route("/social/analytics/growth", get(get_post_growth))
        .route("/social/analytics/insights", post(get_insights))
        .route("/social/analytics/best-times", get(get_project_best_times))
        .with_state(_deployment.clone())
}

/// Standalone public router for bio pages (no auth)
pub fn bio_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/bio/{username}", get(bio_page))
        .with_state(deployment.clone())
}

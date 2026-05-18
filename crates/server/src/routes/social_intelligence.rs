//! Social Intelligence Routes
//!
//! Analytics, content opportunities, audience insights, share of voice,
//! tracked entities, and client reports.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use db::models::social_post::SocialPost;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ─────────────────────────────────────────────────────────────
// Client Report
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ClientReportQuery {
    pub project_id: Option<Uuid>,
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ClientReportAccountSummary {
    pub platform: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub follower_count: Option<i64>,
    pub follower_delta: i64,
    pub follower_delta_pct: f64,
    pub reach_total: i64,
    pub impressions_total: i64,
    pub avg_engagement_rate: f64,
    pub posts_published: i64,
    pub top_posts: Vec<ClientReportPost>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ClientReportPost {
    pub id: String,
    pub caption: Option<String>,
    pub platform_url: Option<String>,
    pub published_at: Option<String>,
    pub likes: i64,
    pub saves: i64,
    pub reach: i64,
    pub engagement_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct ClientReport {
    pub organization_id: String,
    pub project_id: Option<String>,
    pub period_days: i64,
    pub accounts: Vec<ClientReportAccountSummary>,
}

/// GET /organizations/:id/social-report — curated client-facing performance report
async fn get_client_report(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(q): Query<ClientReportQuery>,
) -> Result<Json<ApiResponse<ClientReport>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = q.days.unwrap_or(30);

    // Get accounts scoped to this org (or project)
    #[derive(sqlx::FromRow)]
    struct AccountRow {
        id: String,
        platform: String,
        username: Option<String>,
        avatar_url: Option<String>,
        follower_count: Option<i64>,
    }

    let accounts: Vec<AccountRow> = if let Some(pid) = q.project_id {
        sqlx::query_as(
            "SELECT id, platform, username, avatar_url, follower_count \
             FROM social_accounts WHERE project_id = ?1 AND status = 'active'",
        )
        .bind(pid.to_string())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, platform, username, avatar_url, follower_count \
             FROM social_accounts WHERE organization_id = ?1 AND status = 'active'",
        )
        .bind(&org_id)
        .fetch_all(pool)
        .await?
    };

    let mut account_summaries = Vec::new();

    for account in &accounts {
        // Aggregate metrics from published posts in period
        #[derive(sqlx::FromRow)]
        struct MetricsRow {
            total_reach: i64,
            total_impressions: i64,
            avg_engagement_rate: f64,
            posts_published: i64,
        }

        let metrics: Option<MetricsRow> = sqlx::query_as(
            r#"SELECT
                COALESCE(SUM(reach), 0)           AS total_reach,
                COALESCE(SUM(impressions), 0)     AS total_impressions,
                COALESCE(AVG(engagement_rate), 0) AS avg_engagement_rate,
                COUNT(*)                          AS posts_published
               FROM social_posts
               WHERE social_account_id = ?1
                 AND status = 'published'
                 AND published_at >= datetime('now', '-' || ?2 || ' days')"#,
        )
        .bind(&account.id)
        .bind(days)
        .fetch_optional(pool)
        .await?;

        let m = metrics.unwrap_or(MetricsRow {
            total_reach: 0,
            total_impressions: 0,
            avg_engagement_rate: 0.0,
            posts_published: 0,
        });

        // Follower delta: compare current vs snapshot at period start
        #[derive(sqlx::FromRow)]
        struct SnapshotRow {
            follower_count: Option<i64>,
        }
        let old_snapshot: Option<SnapshotRow> = sqlx::query_as(
            "SELECT follower_count FROM social_account_snapshots \
             WHERE account_id = ?1 AND snapshot_date <= date('now', '-' || ?2 || ' days') \
             ORDER BY snapshot_date DESC LIMIT 1",
        )
        .bind(&account.id)
        .bind(days)
        .fetch_optional(pool)
        .await?;

        let current = account.follower_count.unwrap_or(0);
        let old = old_snapshot
            .and_then(|s| s.follower_count)
            .unwrap_or(current);
        let delta = current - old;
        let delta_pct = if old > 0 {
            (delta as f64 / old as f64) * 100.0
        } else {
            0.0
        };

        // Top 3 posts by composite score (saves * 0.6 + reach * 0.0001)
        let top_posts: Vec<ClientReportPost> = sqlx::query_as(
            r#"SELECT
                CAST(id AS TEXT) AS id,
                caption,
                platform_url,
                date(published_at) AS published_at,
                COALESCE(likes, 0) AS likes,
                COALESCE(saves, 0) AS saves,
                COALESCE(reach, 0) AS reach,
                COALESCE(engagement_rate, 0.0) AS engagement_rate
               FROM social_posts
               WHERE social_account_id = ?1
                 AND status = 'published'
                 AND published_at >= datetime('now', '-' || ?2 || ' days')
               ORDER BY (COALESCE(saves, 0) * 0.6 + COALESCE(reach, 0) * 0.0001) DESC
               LIMIT 3"#,
        )
        .bind(&account.id)
        .bind(days)
        .fetch_all(pool)
        .await?;

        account_summaries.push(ClientReportAccountSummary {
            platform: account.platform.clone(),
            username: account.username.clone(),
            avatar_url: account.avatar_url.clone(),
            follower_count: account.follower_count,
            follower_delta: delta,
            follower_delta_pct: delta_pct,
            reach_total: m.total_reach,
            impressions_total: m.total_impressions,
            avg_engagement_rate: m.avg_engagement_rate,
            posts_published: m.posts_published,
            top_posts,
        });
    }

    Ok(Json(ApiResponse::success(ClientReport {
        organization_id: org_id,
        project_id: q.project_id.map(|u| u.to_string()),
        period_days: days,
        accounts: account_summaries,
    })))
}

// ─────────────────────────────────────────────────────────────
// Tracked Entities (Sprint 2 — stubs, fully wired in Sprint 2)
// ─────────────────────────────────────────────────────────────

/// GET /organizations/:id/intelligence/tracked-entities
async fn list_tracked_entities(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(q): Query<TrackedEntityQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = &deployment.db().pool;

    let rows: Vec<serde_json::Value> = match q.entity_type.as_deref() {
        Some(et) => {
            #[derive(sqlx::FromRow)]
            struct Row {
                data: String,
            }
            let items: Vec<Row> = sqlx::query_as(
                "SELECT json_object('id',id,'name',name,'entity_type',entity_type,\
                 'instagram_handle',instagram_handle,'twitter_handle',twitter_handle,\
                 'linkedin_url',linkedin_url,'niche_relevance',niche_relevance,\
                 'is_active',is_active) AS data \
                 FROM social_tracked_entities \
                 WHERE organization_id = ?1 AND entity_type = ?2 AND is_active = 1 \
                 ORDER BY name",
            )
            .bind(&org_id)
            .bind(et)
            .fetch_all(pool)
            .await?;
            items
                .into_iter()
                .filter_map(|r| serde_json::from_str(&r.data).ok())
                .collect()
        }
        None => {
            #[derive(sqlx::FromRow)]
            struct Row {
                data: String,
            }
            let items: Vec<Row> = sqlx::query_as(
                "SELECT json_object('id',id,'name',name,'entity_type',entity_type,\
                 'instagram_handle',instagram_handle,'twitter_handle',twitter_handle,\
                 'linkedin_url',linkedin_url,'niche_relevance',niche_relevance,\
                 'is_active',is_active) AS data \
                 FROM social_tracked_entities \
                 WHERE organization_id = ?1 AND is_active = 1 \
                 ORDER BY entity_type, name",
            )
            .bind(&org_id)
            .fetch_all(pool)
            .await?;
            items
                .into_iter()
                .filter_map(|r| serde_json::from_str(&r.data).ok())
                .collect()
        }
    };

    Ok(Json(ApiResponse::success(rows)))
}

#[derive(Debug, Deserialize)]
pub struct TrackedEntityQuery {
    pub entity_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTrackedEntity {
    pub entity_type: String,
    pub name: String,
    pub description: Option<String>,
    pub instagram_handle: Option<String>,
    pub twitter_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub tiktok_handle: Option<String>,
    pub youtube_channel: Option<String>,
    pub website_url: Option<String>,
    pub crm_contact_id: Option<String>,
    pub follower_estimate: Option<i64>,
    pub niche_relevance: Option<f64>,
    pub notes: Option<String>,
    pub project_id: Option<Uuid>,
}

/// POST /organizations/:id/intelligence/tracked-entities
async fn create_tracked_entity(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Json(body): Json<CreateTrackedEntity>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO social_tracked_entities \
         (id, organization_id, project_id, entity_type, name, description, \
          instagram_handle, twitter_handle, linkedin_url, tiktok_handle, \
          youtube_channel, website_url, crm_contact_id, follower_estimate, \
          niche_relevance, notes) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
    )
    .bind(&id)
    .bind(&org_id)
    .bind(body.project_id.map(|u| u.to_string()))
    .bind(&body.entity_type)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.instagram_handle)
    .bind(&body.twitter_handle)
    .bind(&body.linkedin_url)
    .bind(&body.tiktok_handle)
    .bind(&body.youtube_channel)
    .bind(&body.website_url)
    .bind(&body.crm_contact_id)
    .bind(body.follower_estimate)
    .bind(body.niche_relevance.unwrap_or(0.5))
    .bind(&body.notes)
    .execute(pool)
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({ "id": id }))))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTrackedEntity {
    pub name: Option<String>,
    pub description: Option<String>,
    pub instagram_handle: Option<String>,
    pub twitter_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub tiktok_handle: Option<String>,
    pub youtube_channel: Option<String>,
    pub website_url: Option<String>,
    pub niche_relevance: Option<f64>,
    pub is_active: Option<bool>,
    pub notes: Option<String>,
}

/// PATCH /social/tracked-entities/:id
async fn update_tracked_entity(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTrackedEntity>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let is_active = body.is_active.map(|b| if b { 1i64 } else { 0i64 });

    sqlx::query(
        "UPDATE social_tracked_entities SET \
         name             = COALESCE(?2, name), \
         description      = COALESCE(?3, description), \
         instagram_handle = COALESCE(?4, instagram_handle), \
         twitter_handle   = COALESCE(?5, twitter_handle), \
         linkedin_url     = COALESCE(?6, linkedin_url), \
         tiktok_handle    = COALESCE(?7, tiktok_handle), \
         youtube_channel  = COALESCE(?8, youtube_channel), \
         website_url      = COALESCE(?9, website_url), \
         niche_relevance  = COALESCE(?10, niche_relevance), \
         is_active        = COALESCE(?11, is_active), \
         notes            = COALESCE(?12, notes), \
         updated_at       = datetime('now','subsec') \
         WHERE id = ?1",
    )
    .bind(&id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.instagram_handle)
    .bind(&body.twitter_handle)
    .bind(&body.linkedin_url)
    .bind(&body.tiktok_handle)
    .bind(&body.youtube_channel)
    .bind(&body.website_url)
    .bind(body.niche_relevance)
    .bind(is_active)
    .bind(&body.notes)
    .execute(pool)
    .await?;

    Ok(Json(ApiResponse::success(())))
}

/// DELETE /social/tracked-entities/:id
async fn delete_tracked_entity(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query("DELETE FROM social_tracked_entities WHERE id = ?1")
        .bind(&id)
        .execute(pool)
        .await?;
    Ok(Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────
// Content Opportunities (Sprint 3 — stubs, fully wired in S3)
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub status: Option<String>,
    pub opportunity_type: Option<String>,
    pub limit: Option<i64>,
}

/// GET /organizations/:id/intelligence/opportunities
async fn list_opportunities(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(q): Query<OpportunityQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = q.limit.unwrap_or(20).min(100);

    let rows: Vec<serde_json::Value> = {
        #[derive(sqlx::FromRow)]
        struct Row {
            data: String,
        }
        let items: Vec<Row> = sqlx::query_as(
            "SELECT json_object('id',id,'opportunity_type',opportunity_type,'title',title,\
             'brief',brief,'suggested_format',suggested_format,'suggested_slot',suggested_slot,\
             'status',status,'relevance_score',relevance_score,'expires_at',expires_at,\
             'draft_post_id',draft_post_id,'created_at',created_at) AS data \
             FROM social_content_opportunities \
             WHERE organization_id = ?1 \
               AND (?2 IS NULL OR status = ?2) \
               AND (?3 IS NULL OR opportunity_type = ?3) \
             ORDER BY relevance_score DESC, created_at DESC \
             LIMIT ?4",
        )
        .bind(&org_id)
        .bind(&q.status)
        .bind(&q.opportunity_type)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        items
            .into_iter()
            .filter_map(|r| serde_json::from_str(&r.data).ok())
            .collect()
    };

    Ok(Json(ApiResponse::success(rows)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateOpportunity {
    pub status: String,
    pub review_notes: Option<String>,
}

/// PATCH /social/opportunities/:id
async fn update_opportunity(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<UpdateOpportunity>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Validate status transition
    let allowed = [
        "approved",
        "rejected",
        "briefed",
        "draft_created",
        "scheduled",
        "expired",
    ];
    if !allowed.contains(&body.status.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid status: {}",
            body.status
        )));
    }

    sqlx::query(
        "UPDATE social_content_opportunities SET \
         status = ?2, review_notes = COALESCE(?3, review_notes), \
         reviewed_at = datetime('now','subsec'), \
         updated_at  = datetime('now','subsec') \
         WHERE id = ?1",
    )
    .bind(&id)
    .bind(&body.status)
    .bind(&body.review_notes)
    .execute(pool)
    .await?;

    // If approving: advance draft post to pending_review
    if body.status == "approved" {
        sqlx::query(
            "UPDATE social_posts SET status = 'pending_review', updated_at = datetime('now','subsec') \
             WHERE id = (SELECT draft_post_id FROM social_content_opportunities WHERE id = ?1) \
               AND status = 'draft'"
        )
        .bind(&id)
        .execute(pool)
        .await?;
    }

    Ok(Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────
// Audience Insights (Sprint 4 — stubs)
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AudienceInsightQuery {
    pub period_days: Option<i64>,
    pub insight_type: Option<String>,
}

/// GET /organizations/:id/intelligence/audience-insights
async fn list_audience_insights(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(q): Query<AudienceInsightQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = q.period_days.unwrap_or(30);

    #[derive(sqlx::FromRow)]
    struct Row {
        data: String,
    }
    let items: Vec<Row> = sqlx::query_as(
        "SELECT json_object('id',id,'insight_type',insight_type,'topic',topic,\
         'mention_count',mention_count,'sentiment',sentiment,'sentiment_score',sentiment_score,\
         'suggested_content_angle',suggested_content_angle,'actioned',actioned,\
         'period_start',period_start,'period_end',period_end) AS data \
         FROM social_audience_insights \
         WHERE organization_id = ?1 \
           AND (?2 IS NULL OR insight_type = ?2) \
           AND period_end >= datetime('now', '-' || ?3 || ' days') \
           AND dismissed = 0 \
         ORDER BY mention_count DESC \
         LIMIT 50",
    )
    .bind(&org_id)
    .bind(&q.insight_type)
    .bind(days)
    .fetch_all(pool)
    .await?;

    let rows = items
        .into_iter()
        .filter_map(|r| serde_json::from_str(&r.data).ok())
        .collect();
    Ok(Json(ApiResponse::success(rows)))
}

// ─────────────────────────────────────────────────────────────
// Share of Voice (Sprint 5 — stubs)
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SovQuery {
    pub period_days: Option<i64>,
    pub platform: Option<String>,
}

/// GET /organizations/:id/intelligence/share-of-voice
async fn get_share_of_voice(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(q): Query<SovQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = q.period_days.unwrap_or(30);

    #[derive(sqlx::FromRow)]
    struct Row {
        data: String,
    }
    let items: Vec<Row> = sqlx::query_as(
        "SELECT json_object('id',id,'period_start',period_start,'period_end',period_end,\
         'platform',platform,'own_mention_count',own_mention_count,\
         'total_niche_mention_count',total_niche_mention_count,\
         'share_of_voice_pct',share_of_voice_pct,'top_keywords',top_keywords,\
         'competitor_summary',competitor_summary) AS data \
         FROM social_share_of_voice \
         WHERE organization_id = ?1 \
           AND (?2 IS NULL OR platform = ?2) \
           AND period_end >= datetime('now', '-' || ?3 || ' days') \
         ORDER BY period_end DESC \
         LIMIT 12",
    )
    .bind(&org_id)
    .bind(&q.platform)
    .bind(days)
    .fetch_all(pool)
    .await?;

    let rows = items
        .into_iter()
        .filter_map(|r| serde_json::from_str(&r.data).ok())
        .collect();
    Ok(Json(ApiResponse::success(rows)))
}

// ─────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Client report (org-scoped)
        .route(
            "/organizations/{org_id}/social-report",
            get(get_client_report),
        )
        // Tracked entities
        .route(
            "/organizations/{org_id}/intelligence/tracked-entities",
            get(list_tracked_entities).post(create_tracked_entity),
        )
        .route(
            "/social/tracked-entities/{id}",
            patch(update_tracked_entity).delete(delete_tracked_entity),
        )
        // Content opportunities
        .route(
            "/organizations/{org_id}/intelligence/opportunities",
            get(list_opportunities),
        )
        .route("/social/opportunities/{id}", patch(update_opportunity))
        // Audience insights
        .route(
            "/organizations/{org_id}/intelligence/audience-insights",
            get(list_audience_insights),
        )
        // Share of voice
        .route(
            "/organizations/{org_id}/intelligence/share-of-voice",
            get(get_share_of_voice),
        )
        .with_state(deployment.clone())
}

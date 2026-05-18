#![allow(dead_code)]
//! MCP tools for social intelligence — content opportunities, audience insights,
//! share of voice, KOL signals, and tracked entity management.

use rmcp::{handler::server::tool::Parameters, model::CallToolResult, schemars, tool, ErrorData};
use serde::Deserialize;

use super::{helpers::*, TaskServer};

// ─── Request types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetContentOpportunitiesRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(
        description = "Status filter: detected, briefed, draft_created, approved, rejected, expired"
    )]
    pub status: Option<String>,
    #[schemars(
        description = "Type filter: trending_topic, news_event, audience_question, kol_signal, competitor_gap"
    )]
    pub opportunity_type: Option<String>,
    #[schemars(description = "Max results (default: 10, max: 50)")]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApproveOpportunityRequest {
    #[schemars(description = "Content opportunity UUID to approve")]
    pub opportunity_id: String,
    #[schemars(description = "Optional review notes")]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RejectOpportunityRequest {
    #[schemars(description = "Content opportunity UUID to reject")]
    pub opportunity_id: String,
    #[schemars(description = "Optional rejection reason")]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSocialPerformanceRequest {
    #[schemars(description = "Organization UUID (optional — use with project_id for scoping)")]
    pub org_id: Option<String>,
    #[schemars(description = "Project UUID (optional)")]
    pub project_id: Option<String>,
    #[schemars(description = "Lookback window in days (default: 30)")]
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAudienceInsightsRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(description = "Lookback window in days (default: 30)")]
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetShareOfVoiceRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(description = "Lookback window in days (default: 30)")]
    pub days: Option<i64>,
    #[schemars(description = "Filter by platform (e.g. 'instagram', 'twitter')")]
    pub platform: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetKolSignalsRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(description = "Max results (default: 5, max: 20)")]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTrackedEntitiesRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(
        description = "Filter by type: competitor, kol_influencer, thought_leader, trade_press"
    )]
    pub entity_type: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddTrackedEntityRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(description = "Display name of the entity")]
    pub name: String,
    #[schemars(description = "Type: competitor, kol_influencer, thought_leader, trade_press")]
    pub entity_type: String,
    #[schemars(description = "Instagram handle (without @)")]
    pub instagram_handle: Option<String>,
    #[schemars(description = "Twitter/X handle (without @)")]
    pub twitter_handle: Option<String>,
    #[schemars(description = "LinkedIn profile URL")]
    pub linkedin_url: Option<String>,
    #[schemars(description = "Niche relevance score 0.0–1.0 (default: 0.5)")]
    pub niche_relevance: Option<f64>,
    #[schemars(description = "Internal notes about this entity")]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateClientReportRequest {
    #[schemars(description = "Organization UUID")]
    pub org_id: String,
    #[schemars(description = "Project UUID (optional)")]
    pub project_id: Option<String>,
    #[schemars(description = "Report period in days (default: 30)")]
    pub days: Option<i64>,
}

// ─── Tool implementations ────────────────────────────────────────────────────

impl TaskServer {
    #[tool(
        description = "List open content opportunities detected by the social intelligence engine. Opportunities are signals the system has turned into actionable content ideas — trending topics, news events, KOL activity, competitor gaps, audience questions. Use status='detected' or 'draft_created' to find items awaiting review."
    )]
    pub(super) async fn get_content_opportunities(
        &self,
        Parameters(req): Parameters<GetContentOpportunitiesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = req.limit.unwrap_or(10).min(50);

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            opportunity_type: String,
            title: String,
            brief: Option<String>,
            suggested_format: Option<String>,
            suggested_slot: Option<String>,
            status: String,
            relevance_score: f64,
            expires_at: Option<String>,
            draft_post_id: Option<String>,
            created_at: String,
        }

        let rows: Vec<Row> = match sqlx::query_as(
            "SELECT id, opportunity_type, title, brief, suggested_format, suggested_slot, \
             status, relevance_score, expires_at, draft_post_id, created_at \
             FROM social_content_opportunities \
             WHERE organization_id = ?1 \
               AND (?2 IS NULL OR status = ?2) \
               AND (?3 IS NULL OR opportunity_type = ?3) \
             ORDER BY relevance_score DESC, created_at DESC \
             LIMIT ?4",
        )
        .bind(&req.org_id)
        .bind(&req.status)
        .bind(&req.opportunity_type)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error fetching opportunities",
                    Some(&e.to_string()),
                ))
            }
        };

        let items: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "type": r.opportunity_type,
                    "title": r.title,
                    "brief": r.brief,
                    "suggested_format": r.suggested_format,
                    "suggested_slot": r.suggested_slot,
                    "status": r.status,
                    "relevance_score": r.relevance_score,
                    "expires_at": r.expires_at,
                    "has_draft": r.draft_post_id.is_some(),
                    "draft_post_id": r.draft_post_id,
                    "created_at": r.created_at,
                })
            })
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "count": items.len(),
            "opportunities": items,
            "tip": if items.is_empty() {
                "No opportunities found. Signal processors run hourly — check back after content is collected."
            } else {
                "Use approve_content_opportunity to advance to pending review."
            }
        })))
    }

    #[tool(
        description = "Approve a content opportunity. Marks it approved and advances its draft post to 'pending_review' so it can be scheduled. Returns confirmation. Use get_content_opportunities to find opportunity IDs."
    )]
    pub(super) async fn approve_content_opportunity(
        &self,
        Parameters(req): Parameters<ApproveOpportunityRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = sqlx::query(
            "UPDATE social_content_opportunities SET \
             status = 'approved', review_notes = COALESCE(?2, review_notes), \
             reviewed_at = datetime('now','subsec'), updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(&req.opportunity_id)
        .bind(&req.notes)
        .execute(&self.pool)
        .await
        {
            return Ok(error_result(
                "Failed to approve opportunity",
                Some(&e.to_string()),
            ));
        }

        // Advance associated draft post to pending_review
        let _ = sqlx::query(
            "UPDATE social_posts SET status = 'pending_review', \
             updated_at = datetime('now','subsec') \
             WHERE id = (SELECT draft_post_id FROM social_content_opportunities WHERE id = ?1) \
               AND status = 'draft'",
        )
        .bind(&req.opportunity_id)
        .execute(&self.pool)
        .await;

        Ok(success_json(&serde_json::json!({
            "success": true,
            "approved": true,
            "opportunity_id": req.opportunity_id,
            "message": "Opportunity approved. Draft post moved to pending review queue."
        })))
    }

    #[tool(
        description = "Reject a content opportunity — it will not be drafted or scheduled. Optionally provide a reason for team context."
    )]
    pub(super) async fn reject_content_opportunity(
        &self,
        Parameters(req): Parameters<RejectOpportunityRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = sqlx::query(
            "UPDATE social_content_opportunities SET \
             status = 'rejected', review_notes = COALESCE(?2, review_notes), \
             reviewed_at = datetime('now','subsec'), updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(&req.opportunity_id)
        .bind(&req.reason)
        .execute(&self.pool)
        .await
        {
            return Ok(error_result(
                "Failed to reject opportunity",
                Some(&e.to_string()),
            ));
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "rejected": true,
            "opportunity_id": req.opportunity_id
        })))
    }

    #[tool(
        description = "Get a social performance summary for an organization or project: total posts published, reach, impressions, likes, saves, avg engagement rate, and the top 3 performing posts by composite score (saves + reach). Provide org_id and/or project_id to scope."
    )]
    pub(super) async fn get_social_performance_summary(
        &self,
        Parameters(req): Parameters<GetSocialPerformanceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let days = req.days.unwrap_or(30);

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            total_posts: i64,
            total_reach: i64,
            total_impressions: i64,
            total_likes: i64,
            total_saves: i64,
            avg_engagement_rate: f64,
        }

        let summary: Option<SummaryRow> = match sqlx::query_as(
            "SELECT COUNT(*) AS total_posts, \
             COALESCE(SUM(reach),0) AS total_reach, \
             COALESCE(SUM(impressions),0) AS total_impressions, \
             COALESCE(SUM(likes),0) AS total_likes, \
             COALESCE(SUM(saves),0) AS total_saves, \
             COALESCE(AVG(engagement_rate),0) AS avg_engagement_rate \
             FROM social_posts \
             WHERE status = 'published' \
               AND published_at >= datetime('now', '-' || ?3 || ' days') \
               AND (?1 IS NULL OR organization_id = ?1) \
               AND (?2 IS NULL OR project_id = ?2)",
        )
        .bind(&req.org_id)
        .bind(&req.project_id)
        .bind(days)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error fetching performance summary",
                    Some(&e.to_string()),
                ))
            }
        };

        #[derive(sqlx::FromRow)]
        struct TopPost {
            caption: Option<String>,
            saves: i64,
            reach: i64,
            engagement_rate: f64,
            published_at: Option<String>,
        }

        let top_posts: Vec<TopPost> = sqlx::query_as(
            "SELECT caption, COALESCE(saves,0) AS saves, COALESCE(reach,0) AS reach, \
             COALESCE(engagement_rate,0) AS engagement_rate, date(published_at) AS published_at \
             FROM social_posts \
             WHERE status = 'published' \
               AND published_at >= datetime('now', '-' || ?3 || ' days') \
               AND (?1 IS NULL OR organization_id = ?1) \
               AND (?2 IS NULL OR project_id = ?2) \
             ORDER BY (COALESCE(saves,0) * 0.6 + COALESCE(reach,0) * 0.0001) DESC \
             LIMIT 3",
        )
        .bind(&req.org_id)
        .bind(&req.project_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let s = summary.unwrap_or(SummaryRow {
            total_posts: 0,
            total_reach: 0,
            total_impressions: 0,
            total_likes: 0,
            total_saves: 0,
            avg_engagement_rate: 0.0,
        });

        Ok(success_json(&serde_json::json!({
            "success": true,
            "period_days": days,
            "total_posts_published": s.total_posts,
            "total_reach": s.total_reach,
            "total_impressions": s.total_impressions,
            "total_likes": s.total_likes,
            "total_saves": s.total_saves,
            "avg_engagement_rate_pct": format!("{:.2}", s.avg_engagement_rate),
            "top_posts": top_posts.iter().map(|p| serde_json::json!({
                "caption_preview": p.caption.as_deref().map(|c| &c[..c.len().min(80)]),
                "saves": p.saves,
                "reach": p.reach,
                "engagement_rate": format!("{:.2}%", p.engagement_rate),
                "published_at": p.published_at,
            })).collect::<Vec<_>>()
        })))
    }

    #[tool(
        description = "Get audience insights derived from mention clustering — frequently asked questions, objections, praise topics, sentiment shifts, and content angle suggestions based on what the audience is actually saying."
    )]
    pub(super) async fn get_audience_insights(
        &self,
        Parameters(req): Parameters<GetAudienceInsightsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let days = req.days.unwrap_or(30);

        #[derive(sqlx::FromRow)]
        struct InsightRow {
            insight_type: String,
            topic: String,
            mention_count: i64,
            sentiment: Option<String>,
            suggested_content_angle: Option<String>,
        }

        let rows: Vec<InsightRow> = match sqlx::query_as(
            "SELECT insight_type, topic, mention_count, sentiment, suggested_content_angle \
             FROM social_audience_insights \
             WHERE organization_id = ?1 AND dismissed = 0 \
               AND period_end >= datetime('now', '-' || ?2 || ' days') \
             ORDER BY mention_count DESC \
             LIMIT 20",
        )
        .bind(&req.org_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error fetching audience insights",
                    Some(&e.to_string()),
                ))
            }
        };

        Ok(success_json(&serde_json::json!({
            "success": true,
            "period_days": days,
            "count": rows.len(),
            "insights": rows.iter().map(|r| serde_json::json!({
                "type": r.insight_type,
                "topic": r.topic,
                "mention_count": r.mention_count,
                "sentiment": r.sentiment,
                "content_angle": r.suggested_content_angle,
            })).collect::<Vec<_>>(),
            "note": if rows.is_empty() {
                "No insights yet. Mention clustering runs hourly when mentions exist."
            } else { "" }
        })))
    }

    #[tool(
        description = "Get share of voice data — what percentage of niche/keyword conversation the organization owns vs. the total. Returns weekly periods with own mention count, total niche mentions, and SOV percentage."
    )]
    pub(super) async fn get_share_of_voice(
        &self,
        Parameters(req): Parameters<GetShareOfVoiceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let days = req.days.unwrap_or(30);

        #[derive(sqlx::FromRow)]
        struct SovRow {
            period_start: String,
            period_end: String,
            own_mention_count: i64,
            total_niche_mention_count: i64,
            share_of_voice_pct: Option<f64>,
            top_keywords: Option<String>,
            competitor_summary: Option<String>,
        }

        let rows: Vec<SovRow> = match sqlx::query_as(
            "SELECT period_start, period_end, own_mention_count, total_niche_mention_count, \
             share_of_voice_pct, top_keywords, competitor_summary \
             FROM social_share_of_voice \
             WHERE organization_id = ?1 \
               AND (?2 IS NULL OR platform = ?2) \
               AND period_end >= datetime('now', '-' || ?3 || ' days') \
             ORDER BY period_end DESC \
             LIMIT 4",
        )
        .bind(&req.org_id)
        .bind(&req.platform)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error fetching share of voice",
                    Some(&e.to_string()),
                ))
            }
        };

        Ok(success_json(&serde_json::json!({
            "success": true,
            "periods": rows.iter().map(|r| serde_json::json!({
                "period_start": r.period_start,
                "period_end": r.period_end,
                "own_mentions": r.own_mention_count,
                "total_niche_mentions": r.total_niche_mention_count,
                "share_of_voice_pct": r.share_of_voice_pct.map(|v| format!("{v:.1}%")),
                "top_keywords": r.top_keywords.as_deref()
                    .and_then(|k| serde_json::from_str::<serde_json::Value>(k).ok()),
                "competitors": r.competitor_summary.as_deref()
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok()),
            })).collect::<Vec<_>>(),
            "note": if rows.is_empty() { "SOV calculation runs weekly on Sundays." } else { "" }
        })))
    }

    #[tool(
        description = "Get recent high-signal KOL and thought leader content — these are opportunities generated when tracked influencers/thought leaders post content relevant to the org's niche. Returns the title, brief, relevance score, and linked tracked entity."
    )]
    pub(super) async fn get_kol_signals(
        &self,
        Parameters(req): Parameters<GetKolSignalsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = req.limit.unwrap_or(5).min(20);

        #[derive(sqlx::FromRow)]
        struct Row {
            title: String,
            brief: Option<String>,
            relevance_score: f64,
            tracked_entity_id: Option<String>,
            expires_at: Option<String>,
            created_at: String,
        }

        let rows: Vec<Row> = match sqlx::query_as(
            "SELECT title, brief, relevance_score, tracked_entity_id, expires_at, created_at \
             FROM social_content_opportunities \
             WHERE organization_id = ?1 \
               AND opportunity_type = 'kol_signal' \
               AND status NOT IN ('rejected', 'expired') \
             ORDER BY relevance_score DESC, created_at DESC \
             LIMIT ?2",
        )
        .bind(&req.org_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error fetching KOL signals",
                    Some(&e.to_string()),
                ))
            }
        };

        Ok(success_json(&serde_json::json!({
            "success": true,
            "count": rows.len(),
            "kol_signals": rows.iter().map(|r| serde_json::json!({
                "title": r.title,
                "brief": r.brief,
                "relevance_score": r.relevance_score,
                "tracked_entity_id": r.tracked_entity_id,
                "expires_at": r.expires_at,
                "detected_at": r.created_at,
            })).collect::<Vec<_>>()
        })))
    }

    #[tool(
        description = "List tracked entities (competitors, KOL influencers, thought leaders, trade press) for an organization. These are the signal sources the pulse engine monitors. Optionally filter by entity_type."
    )]
    pub(super) async fn get_tracked_entities(
        &self,
        Parameters(req): Parameters<GetTrackedEntitiesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            entity_type: String,
            name: String,
            instagram_handle: Option<String>,
            twitter_handle: Option<String>,
            linkedin_url: Option<String>,
            niche_relevance: f64,
            notes: Option<String>,
        }

        let rows: Vec<Row> = match sqlx::query_as(
            "SELECT id, entity_type, name, instagram_handle, twitter_handle, linkedin_url, \
             niche_relevance, notes \
             FROM social_tracked_entities \
             WHERE organization_id = ?1 AND is_active = 1 \
               AND (?2 IS NULL OR entity_type = ?2) \
             ORDER BY entity_type, niche_relevance DESC",
        )
        .bind(&req.org_id)
        .bind(&req.entity_type)
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error fetching tracked entities",
                    Some(&e.to_string()),
                ))
            }
        };

        Ok(success_json(&serde_json::json!({
            "success": true,
            "count": rows.len(),
            "entities": rows.iter().map(|r| serde_json::json!({
                "id": r.id,
                "type": r.entity_type,
                "name": r.name,
                "instagram": r.instagram_handle,
                "twitter": r.twitter_handle,
                "linkedin": r.linkedin_url,
                "relevance": r.niche_relevance,
                "notes": r.notes,
            })).collect::<Vec<_>>()
        })))
    }

    #[tool(
        description = "Add a tracked entity to monitor for signal intelligence. entity_type must be one of: competitor, kol_influencer, thought_leader, trade_press. Provide any known social handles so the pulse engine can collect their content."
    )]
    pub(super) async fn add_tracked_entity(
        &self,
        Parameters(req): Parameters<AddTrackedEntityRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let valid_types = [
            "competitor",
            "kol_influencer",
            "thought_leader",
            "trade_press",
        ];
        if !valid_types.contains(&req.entity_type.as_str()) {
            return Ok(error_result(
                "Invalid entity_type",
                Some(&format!("Must be one of: {}", valid_types.join(", "))),
            ));
        }

        let id = uuid::Uuid::new_v4().to_string();

        if let Err(e) = sqlx::query(
            "INSERT INTO social_tracked_entities \
             (id, organization_id, entity_type, name, instagram_handle, twitter_handle, \
              linkedin_url, niche_relevance, notes) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        )
        .bind(&id)
        .bind(&req.org_id)
        .bind(&req.entity_type)
        .bind(&req.name)
        .bind(&req.instagram_handle)
        .bind(&req.twitter_handle)
        .bind(&req.linkedin_url)
        .bind(req.niche_relevance.unwrap_or(0.5))
        .bind(&req.notes)
        .execute(&self.pool)
        .await
        {
            return Ok(error_result(
                "Failed to add tracked entity",
                Some(&e.to_string()),
            ));
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "created": true,
            "id": id,
            "message": format!(
                "Added {} '{}' to tracking. Pulse collection will monitor this source.",
                req.entity_type, req.name
            )
        })))
    }

    #[tool(
        description = "Generate a curated client report summary showing published post performance, reach, engagement, and top content for the given period. This is the client-safe view — no pulse intelligence, competitor data, or share of voice included."
    )]
    pub(super) async fn generate_client_report_summary(
        &self,
        Parameters(req): Parameters<GenerateClientReportRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let days = req.days.unwrap_or(30);

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            total_posts: i64,
            total_reach: i64,
            total_impressions: i64,
            total_likes: i64,
            total_saves: i64,
            avg_engagement_rate: f64,
        }

        let summary: Option<SummaryRow> = match sqlx::query_as(
            "SELECT COUNT(*) AS total_posts, \
             COALESCE(SUM(reach),0) AS total_reach, \
             COALESCE(SUM(impressions),0) AS total_impressions, \
             COALESCE(SUM(likes),0) AS total_likes, \
             COALESCE(SUM(saves),0) AS total_saves, \
             COALESCE(AVG(engagement_rate),0) AS avg_engagement_rate \
             FROM social_posts \
             WHERE status = 'published' \
               AND published_at >= datetime('now', '-' || ?3 || ' days') \
               AND organization_id = ?1 \
               AND (?2 IS NULL OR project_id = ?2)",
        )
        .bind(&req.org_id)
        .bind(&req.project_id)
        .bind(days)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "DB error generating client report",
                    Some(&e.to_string()),
                ))
            }
        };

        #[derive(sqlx::FromRow)]
        struct TopPost {
            caption: Option<String>,
            saves: i64,
            reach: i64,
            engagement_rate: f64,
            published_at: Option<String>,
        }

        let top_posts: Vec<TopPost> = sqlx::query_as(
            "SELECT caption, COALESCE(saves,0) AS saves, COALESCE(reach,0) AS reach, \
             COALESCE(engagement_rate,0) AS engagement_rate, date(published_at) AS published_at \
             FROM social_posts \
             WHERE status = 'published' \
               AND published_at >= datetime('now', '-' || ?3 || ' days') \
               AND organization_id = ?1 \
               AND (?2 IS NULL OR project_id = ?2) \
             ORDER BY (COALESCE(saves,0) * 0.6 + COALESCE(reach,0) * 0.0001) DESC \
             LIMIT 3",
        )
        .bind(&req.org_id)
        .bind(&req.project_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let s = summary.unwrap_or(SummaryRow {
            total_posts: 0,
            total_reach: 0,
            total_impressions: 0,
            total_likes: 0,
            total_saves: 0,
            avg_engagement_rate: 0.0,
        });

        Ok(success_json(&serde_json::json!({
            "success": true,
            "report_type": "curated_client_summary",
            "organization_id": req.org_id,
            "period_days": days,
            "total_posts_published": s.total_posts,
            "total_reach": s.total_reach,
            "total_impressions": s.total_impressions,
            "total_likes": s.total_likes,
            "total_saves": s.total_saves,
            "avg_engagement_rate_pct": format!("{:.2}", s.avg_engagement_rate),
            "top_posts": top_posts.iter().map(|p| serde_json::json!({
                "caption_preview": p.caption.as_deref().map(|c| &c[..c.len().min(80)]),
                "saves": p.saves,
                "reach": p.reach,
                "engagement_rate": format!("{:.2}%", p.engagement_rate),
                "published_at": p.published_at,
            })).collect::<Vec<_>>(),
            "note": "Client-safe view. Pulse intel and competitor data excluded."
        })))
    }
}

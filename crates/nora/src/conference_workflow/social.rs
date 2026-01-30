//! Social Post Creation and Scheduling
//!
//! Creates SocialPost records from workflow content and schedules them
//! for optimal posting times.

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use db::models::{
    conference_workflow::ConferenceWorkflow,
    social_post::{CreateSocialPost, ContentType, SocialPost},
};

use crate::{NoraError, Result};

use super::{
    engine::ResearchFlowResult,
    parallel::{ArticleContent, ContentResult, GraphicsResult},
};

/// Social post creator
pub struct SocialPostCreator {
    pool: SqlitePool,
}

impl SocialPostCreator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create social posts for a workflow from research results
    pub async fn create_posts_for_workflow(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<Vec<SocialPost>> {
        tracing::info!(
            "[SOCIAL] Creating posts for workflow: {}",
            workflow.conference_name
        );

        let mut posts = Vec::new();

        // Get project ID from board
        let board = db::models::project_board::ProjectBoard::find_by_id(&self.pool, workflow.conference_board_id)
            .await
            .map_err(NoraError::DatabaseError)?
            .ok_or_else(|| NoraError::ConfigError("Board not found".to_string()))?;

        let project_id = board.project_id;

        // Create pre-event announcement post
        match self.create_announcement_post(workflow, project_id).await {
            Ok(post) => posts.push(post),
            Err(e) => tracing::warn!("[SOCIAL] Failed to create announcement post: {}", e),
        }

        // Create speaker spotlight posts
        for entity in &research.entities {
            if entity.entity_type == db::models::entity::EntityType::Speaker {
                match self.create_speaker_spotlight(workflow, project_id, entity).await {
                    Ok(post) => posts.push(post),
                    Err(e) => tracing::warn!("[SOCIAL] Failed to create speaker spotlight: {}", e),
                }
            }
        }

        // Create side events post
        if !research.side_events.is_empty() {
            match self.create_side_events_post(workflow, project_id, &research.side_events).await {
                Ok(post) => posts.push(post),
                Err(e) => tracing::warn!("[SOCIAL] Failed to create side events post: {}", e),
            }
        }

        tracing::info!("[SOCIAL] Created {} posts", posts.len());

        Ok(posts)
    }

    /// Create social posts from content + graphics results
    pub async fn create_posts_from_content(
        &self,
        workflow: &ConferenceWorkflow,
        content: &ContentResult,
        graphics: &GraphicsResult,
    ) -> Result<Vec<SocialPost>> {
        let mut posts = Vec::new();

        let board = db::models::project_board::ProjectBoard::find_by_id(&self.pool, workflow.conference_board_id)
            .await
            .map_err(NoraError::DatabaseError)?
            .ok_or_else(|| NoraError::ConfigError("Board not found".to_string()))?;

        let project_id = board.project_id;

        for article in &content.articles {
            let thumbnail = graphics.thumbnails.get(&article.article_type);

            let media_urls = thumbnail.map(|t| vec![t.url.clone()]);

            match self.create_article_post(workflow, project_id, article, media_urls).await {
                Ok(post) => posts.push(post),
                Err(e) => tracing::warn!("[SOCIAL] Failed to create article post: {}", e),
            }
        }

        Ok(posts)
    }

    /// Create pre-event announcement post
    async fn create_announcement_post(
        &self,
        workflow: &ConferenceWorkflow,
        project_id: Uuid,
    ) -> Result<SocialPost> {
        let caption = format!(
            "🎯 We're heading to {}! \n\n\
            📅 {} - {}\n\
            📍 {}\n\n\
            Stay tuned for speaker spotlights, side event guides, and live coverage! 🚀\n\n\
            #{}",
            workflow.conference_name,
            workflow.start_date,
            workflow.end_date,
            workflow.location.as_deref().unwrap_or("TBD"),
            slugify(&workflow.conference_name)
        );

        let scheduled_for = self.calculate_optimal_time(&workflow.start_date, -5);

        let create = CreateSocialPost {
            project_id,
            social_account_id: None,
            task_id: None,
            content_type: Some(ContentType::Post),
            caption: Some(caption),
            content_blocks: None,
            media_urls: None,
            hashtags: Some(vec![
                slugify(&workflow.conference_name),
                "conference".to_string(),
                "event".to_string(),
            ]),
            mentions: None,
            platforms: vec![], // Will be populated from project social accounts
            platform_specific: None,
            scheduled_for,
            category: Some("announcement".to_string()),
            is_evergreen: Some(false),
            recycle_after_days: None,
            created_by_agent_id: None,
        };

        SocialPost::create(&self.pool, create)
            .await
            .map_err(|e| match e {
                db::models::social_post::SocialPostError::Database(err) => NoraError::DatabaseError(err),
                db::models::social_post::SocialPostError::NotFound => NoraError::ConfigError("Social post not found".to_string()),
            })
    }

    /// Create speaker spotlight post
    async fn create_speaker_spotlight(
        &self,
        workflow: &ConferenceWorkflow,
        project_id: Uuid,
        speaker: &db::models::entity::Entity,
    ) -> Result<SocialPost> {
        let title_line = match (&speaker.title, &speaker.company) {
            (Some(title), Some(company)) => format!("{} at {}", title, company),
            (Some(title), None) => title.clone(),
            (None, Some(company)) => format!("at {}", company),
            (None, None) => String::new(),
        };

        let caption = format!(
            "🎤 Speaker Spotlight: {}\n\n\
            {}\n\n\
            {}\n\n\
            Catch them at {}! 🚀\n\n\
            #{}",
            speaker.canonical_name,
            title_line,
            speaker.bio.as_deref().unwrap_or("").chars().take(200).collect::<String>(),
            workflow.conference_name,
            slugify(&workflow.conference_name)
        );

        let media_urls = speaker.photo_url.as_ref().map(|url| vec![url.clone()]);

        let scheduled_for = self.calculate_optimal_time(&workflow.start_date, -3);

        let create = CreateSocialPost {
            project_id,
            social_account_id: None,
            task_id: None,
            content_type: Some(ContentType::Post),
            caption: Some(caption),
            content_blocks: None,
            media_urls,
            hashtags: Some(vec![
                slugify(&workflow.conference_name),
                "speaker".to_string(),
                slugify(&speaker.canonical_name),
            ]),
            mentions: None,
            platforms: vec![],
            platform_specific: None,
            scheduled_for,
            category: Some("speaker_spotlight".to_string()),
            is_evergreen: Some(false),
            recycle_after_days: None,
            created_by_agent_id: None,
        };

        SocialPost::create(&self.pool, create)
            .await
            .map_err(|e| match e {
                db::models::social_post::SocialPostError::Database(err) => NoraError::DatabaseError(err),
                db::models::social_post::SocialPostError::NotFound => NoraError::ConfigError("Social post not found".to_string()),
            })
    }

    /// Create side events compilation post
    async fn create_side_events_post(
        &self,
        workflow: &ConferenceWorkflow,
        project_id: Uuid,
        side_events: &[db::models::side_event::SideEvent],
    ) -> Result<SocialPost> {
        let event_list: Vec<String> = side_events
            .iter()
            .take(5)
            .filter(|e| e.relevance_score.map(|s| s > 0.5).unwrap_or(false))
            .map(|e| {
                let date = e.event_date.as_deref().unwrap_or("TBD");
                format!("• {} ({})", e.name, date)
            })
            .collect();

        let caption = format!(
            "🎉 Side Events Guide for {}\n\n\
            Don't miss these satellite events:\n\n\
            {}\n\n\
            More events in our full guide! Link in bio 🔗\n\n\
            #{}",
            workflow.conference_name,
            event_list.join("\n"),
            slugify(&workflow.conference_name)
        );

        let scheduled_for = self.calculate_optimal_time(&workflow.start_date, -2);

        let create = CreateSocialPost {
            project_id,
            social_account_id: None,
            task_id: None,
            content_type: Some(ContentType::Post),
            caption: Some(caption),
            content_blocks: None,
            media_urls: None,
            hashtags: Some(vec![
                slugify(&workflow.conference_name),
                "sideevents".to_string(),
                "networking".to_string(),
            ]),
            mentions: None,
            platforms: vec![],
            platform_specific: None,
            scheduled_for,
            category: Some("side_events".to_string()),
            is_evergreen: Some(false),
            recycle_after_days: None,
            created_by_agent_id: None,
        };

        SocialPost::create(&self.pool, create)
            .await
            .map_err(|e| match e {
                db::models::social_post::SocialPostError::Database(err) => NoraError::DatabaseError(err),
                db::models::social_post::SocialPostError::NotFound => NoraError::ConfigError("Social post not found".to_string()),
            })
    }

    /// Create post from article content
    async fn create_article_post(
        &self,
        workflow: &ConferenceWorkflow,
        project_id: Uuid,
        article: &ArticleContent,
        media_urls: Option<Vec<String>>,
    ) -> Result<SocialPost> {
        let scheduled_for = self.calculate_optimal_time(&workflow.start_date, -2);

        let create = CreateSocialPost {
            project_id,
            social_account_id: None,
            task_id: article.task_id,
            content_type: Some(ContentType::Article),
            caption: Some(article.social_caption.clone()),
            content_blocks: None,
            media_urls,
            hashtags: Some(article.hashtags.clone()),
            mentions: None,
            platforms: vec![],
            platform_specific: None,
            scheduled_for,
            category: Some(article.article_type.clone()),
            is_evergreen: Some(false),
            recycle_after_days: None,
            created_by_agent_id: None,
        };

        SocialPost::create(&self.pool, create)
            .await
            .map_err(|e| match e {
                db::models::social_post::SocialPostError::Database(err) => NoraError::DatabaseError(err),
                db::models::social_post::SocialPostError::NotFound => NoraError::ConfigError("Social post not found".to_string()),
            })
    }

    /// Calculate optimal posting time based on conference date and offset
    fn calculate_optimal_time(&self, conference_date_str: &str, days_offset: i64) -> Option<DateTime<Utc>> {
        let date = chrono::NaiveDate::parse_from_str(conference_date_str, "%Y-%m-%d").ok()?;

        // Schedule for 10 AM UTC
        let time = chrono::NaiveTime::from_hms_opt(10, 0, 0)?;
        let datetime = date.and_time(time);

        let scheduled = Utc.from_utc_datetime(&datetime) + Duration::days(days_offset);

        Some(scheduled)
    }
}

/// Simple slugify helper
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

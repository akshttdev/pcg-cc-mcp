//! Parallel Content + Graphics Orchestration
//!
//! Runs content creation and graphics generation workflows in parallel
//! using specialized agents (Muse for content, Maci for graphics via ComfyUI).

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use db::models::conference_workflow::ConferenceWorkflow;
use cinematics::CinematicsService;

use crate::{
    execution::{research::ResearchTools, ExecutionEngine},
    NoraError, Result,
};

use super::graphics::GraphicsComposer;

use super::engine::ResearchFlowResult;

/// Result from content creation workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentResult {
    pub articles: Vec<ArticleContent>,
    pub social_captions: Vec<String>,
    pub execution_id: Option<Uuid>,
}

/// Individual article content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleContent {
    pub task_id: Option<Uuid>,
    pub article_type: String,
    pub title: String,
    pub body: String,
    pub social_caption: String,
    pub hashtags: Vec<String>,
    pub agent_id: String,
}

/// Result from graphics creation workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsResult {
    pub thumbnails: std::collections::HashMap<String, ThumbnailAsset>,
    pub social_graphics: Vec<SocialGraphic>,
    pub execution_id: Option<Uuid>,
}

/// Individual thumbnail asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailAsset {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// Social media graphic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialGraphic {
    pub platform: String,
    pub aspect_ratio: String,
    pub url: String,
}

/// Parallel orchestrator for content + graphics workflows
pub struct ParallelOrchestrator {
    pool: SqlitePool,
    #[allow(dead_code)]
    execution_engine: Arc<ExecutionEngine>,
    research_tools: ResearchTools,
    cinematics: Option<Arc<CinematicsService>>,
    graphics_composer: GraphicsComposer,
}

impl ParallelOrchestrator {
    pub fn new(
        pool: SqlitePool,
        execution_engine: Arc<ExecutionEngine>,
        cinematics: Option<Arc<CinematicsService>>,
    ) -> Self {
        Self {
            pool: pool.clone(),
            execution_engine,
            research_tools: ResearchTools::new(),
            cinematics: cinematics.clone(),
            graphics_composer: GraphicsComposer::new(pool, cinematics),
        }
    }

    /// Run content and graphics creation in parallel
    pub async fn run_parallel_creation(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<(ContentResult, GraphicsResult)> {
        tracing::info!(
            "[PARALLEL] Starting parallel content + graphics for: {}",
            workflow.conference_name
        );

        // Run content and graphics workflows in parallel
        let content_future = self.run_content_workflow(workflow, research);
        let graphics_future = self.run_graphics_workflow(workflow, research);

        let (content_result, graphics_result) = tokio::join!(content_future, graphics_future);

        let content = content_result?;
        let graphics = graphics_result?;

        tracing::info!(
            "[PARALLEL] Completed: {} articles, {} thumbnails",
            content.articles.len(),
            graphics.thumbnails.len()
        );

        Ok((content, graphics))
    }

    /// Run content creation workflow (Muse agent)
    async fn run_content_workflow(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<ContentResult> {
        tracing::info!("[PARALLEL] Starting content workflow with LLM generation");

        let mut articles = Vec::new();

        // Generate Speakers Article
        match self.generate_speakers_article(workflow, research).await {
            Ok(article) => {
                tracing::info!("[PARALLEL] Generated speakers article: {} chars", article.body.len());
                articles.push(article);
            }
            Err(e) => tracing::warn!("[PARALLEL] Failed to generate speakers article: {}", e),
        }

        // Generate Side Events Article
        if !research.side_events.is_empty() {
            match self.generate_side_events_article(workflow, research).await {
                Ok(article) => {
                    tracing::info!("[PARALLEL] Generated side events article: {} chars", article.body.len());
                    articles.push(article);
                }
                Err(e) => tracing::warn!("[PARALLEL] Failed to generate side events article: {}", e),
            }
        }

        // Generate Press Release
        match self.generate_press_release(workflow, research).await {
            Ok(article) => {
                tracing::info!("[PARALLEL] Generated press release: {} chars", article.body.len());
                articles.push(article);
            }
            Err(e) => tracing::warn!("[PARALLEL] Failed to generate press release: {}", e),
        }

        tracing::info!("[PARALLEL] Content workflow complete: {} articles", articles.len());

        // Collect social captions from generated articles
        let social_captions: Vec<String> = articles
            .iter()
            .map(|article| article.social_caption.clone())
            .filter(|caption| !caption.is_empty())
            .collect();

        Ok(ContentResult {
            articles,
            social_captions,
            execution_id: None,
        })
    }

    /// Generate Speakers Article using LLM
    async fn generate_speakers_article(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<ArticleContent> {
        let speakers: Vec<_> = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Speaker)
            .collect();

        let speaker_data: Vec<String> = speakers
            .iter()
            .map(|s| {
                format!(
                    "- **{}**: {} at {}. {}",
                    s.canonical_name,
                    s.title.as_deref().unwrap_or("Speaker"),
                    s.company.as_deref().unwrap_or(""),
                    s.bio.as_deref().unwrap_or("").chars().take(200).collect::<String>()
                )
            })
            .collect();

        let system_prompt = r#"You are an expert tech journalist writing engaging conference coverage articles.

Write a compelling article about the speakers at this conference. The article should:
1. Have an engaging headline that captures attention
2. Open with a hook about why this speaker lineup matters
3. Profile key speakers with their backgrounds and expertise
4. Highlight what attendees can learn from each speaker
5. End with a call-to-action

Output a JSON object with these exact fields:
{
  "title": "string (compelling headline)",
  "body": "string (full article body in markdown, 800-1200 words)",
  "social_caption": "string (engaging social media caption, 280 chars max)",
  "hashtags": ["array", "of", "relevant", "hashtags"]
}

Write in an energetic, professional tone. Return ONLY valid JSON."#;

        let user_prompt = format!(
            r#"Write a speakers article for {}:

Conference: {}
Dates: {} to {}
Location: {}

Speakers:
{}

Create an engaging article profiling these speakers and what attendees can expect."#,
            workflow.conference_name,
            workflow.conference_name,
            workflow.start_date,
            workflow.end_date,
            workflow.location.as_deref().unwrap_or("TBD"),
            speaker_data.join("\n")
        );

        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| NoraError::ExecutionError(format!("LLM article generation failed: {}", e)))?;

        let parsed = parse_article_response(
            &response,
            &format!("Meet the Speakers at {}", workflow.conference_name),
            &format!("Check out the incredible speakers at {}!", workflow.conference_name),
        );

        let mut hashtags = parsed.hashtags;
        if hashtags.is_empty() {
            hashtags = vec![slugify(&workflow.conference_name), "speakers".to_string()];
        }

        Ok(ArticleContent {
            task_id: None,
            article_type: "speakers".to_string(),
            title: parsed.title,
            body: parsed.body,
            social_caption: parsed.social_caption,
            hashtags,
            agent_id: "muse-creative".to_string(),
        })
    }

    /// Generate Side Events Article using LLM
    async fn generate_side_events_article(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<ArticleContent> {
        let events_data: Vec<String> = research
            .side_events
            .iter()
            .map(|e| {
                format!(
                    "- **{}**: {} at {}. URL: {}",
                    e.name,
                    e.event_date.as_deref().unwrap_or("TBD"),
                    e.venue_name.as_deref().unwrap_or("TBD"),
                    e.event_url.as_deref().unwrap_or("")
                )
            })
            .collect();

        let system_prompt = r#"You are an expert tech journalist writing engaging conference coverage articles.

Write a compelling guide to side events happening around this conference. The article should:
1. Have an engaging headline about satellite events and networking opportunities
2. Explain why side events matter for maximizing conference value
3. Categorize events (networking, parties, workshops, meetups)
4. Highlight must-attend events with dates and venues
5. Include practical tips for navigating multiple events

Output a JSON object with these exact fields:
{
  "title": "string (compelling headline)",
  "body": "string (full article body in markdown, 600-1000 words)",
  "social_caption": "string (engaging social media caption, 280 chars max)",
  "hashtags": ["array", "of", "relevant", "hashtags"]
}

Write in an energetic, helpful tone. Return ONLY valid JSON."#;

        let user_prompt = format!(
            r#"Write a side events guide for {}:

Conference: {}
Dates: {} to {}
Location: {}

Side Events:
{}

Create an engaging guide to these satellite events and networking opportunities."#,
            workflow.conference_name,
            workflow.conference_name,
            workflow.start_date,
            workflow.end_date,
            workflow.location.as_deref().unwrap_or("TBD"),
            events_data.join("\n")
        );

        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| NoraError::ExecutionError(format!("LLM article generation failed: {}", e)))?;

        let parsed = parse_article_response(
            &response,
            &format!("Your Guide to {} Side Events", workflow.conference_name),
            &format!("Don't miss these {} side events!", research.side_events.len()),
        );

        let mut hashtags = parsed.hashtags;
        if hashtags.is_empty() {
            hashtags = vec![slugify(&workflow.conference_name), "sideevents".to_string()];
        }

        Ok(ArticleContent {
            task_id: None,
            article_type: "side_events".to_string(),
            title: parsed.title,
            body: parsed.body,
            social_caption: parsed.social_caption,
            hashtags,
            agent_id: "muse-creative".to_string(),
        })
    }

    /// Generate Press Release using LLM
    async fn generate_press_release(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<ArticleContent> {
        // Gather key data points
        let speaker_count = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Speaker)
            .count();

        let sponsor_count = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Sponsor)
            .count();

        let top_speakers: Vec<String> = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Speaker)
            .take(5)
            .map(|s| {
                format!(
                    "{} ({} at {})",
                    s.canonical_name,
                    s.title.as_deref().unwrap_or("Speaker"),
                    s.company.as_deref().unwrap_or("")
                )
            })
            .collect();

        let system_prompt = r#"You are a PR professional writing press releases for tech media coverage.

Write a professional press release announcing coverage of this conference. The press release should:
1. Have a strong headline suitable for media pickup
2. Include a dateline (city, date format)
3. Lead with the most newsworthy angle
4. Include quotes (you can create attributed quotes)
5. Provide key facts: dates, location, speaker count, sponsors
6. End with boilerplate about the publication/outlet

Output a JSON object with these exact fields:
{
  "title": "string (press release headline)",
  "body": "string (full press release in standard PR format, 400-600 words)",
  "social_caption": "string (announcement for social media, 280 chars max)",
  "hashtags": ["array", "of", "relevant", "hashtags"]
}

Write in professional PR style. Return ONLY valid JSON."#;

        let user_prompt = format!(
            r#"Write a press release announcing our coverage of {}:

Conference: {}
Dates: {} to {}
Location: {}
Speaker Count: {}
Sponsor Count: {}
Side Events: {}

Top Speakers: {}

Create a professional press release suitable for media distribution."#,
            workflow.conference_name,
            workflow.conference_name,
            workflow.start_date,
            workflow.end_date,
            workflow.location.as_deref().unwrap_or("TBD"),
            speaker_count,
            sponsor_count,
            research.side_events.len(),
            top_speakers.join(", ")
        );

        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| NoraError::ExecutionError(format!("LLM press release generation failed: {}", e)))?;

        let parsed = parse_article_response(
            &response,
            &format!("Comprehensive Coverage Announced for {}", workflow.conference_name),
            &format!("Announcing our coverage of {}!", workflow.conference_name),
        );

        let mut hashtags = parsed.hashtags;
        if hashtags.is_empty() {
            hashtags = vec![slugify(&workflow.conference_name), "pressrelease".to_string()];
        }

        Ok(ArticleContent {
            task_id: None,
            article_type: "press_release".to_string(),
            title: parsed.title,
            body: parsed.body,
            social_caption: parsed.social_caption,
            hashtags,
            agent_id: "muse-creative".to_string(),
        })
    }

    /// Run graphics creation workflow using GraphicsComposer
    ///
    /// Composes thumbnails using real assets from the knowledge graph:
    /// - Speaker photos for speaker articles
    /// - Sponsor logos for side events
    /// - AI-generated backgrounds only when needed
    async fn run_graphics_workflow(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> Result<GraphicsResult> {
        tracing::info!("[PARALLEL] Starting graphics workflow with asset composition");

        let mut thumbnails = std::collections::HashMap::new();
        let mut social_graphics = Vec::new();

        // Collect all available assets from the knowledge graph
        let assets = self.graphics_composer.collect_assets(research);
        tracing::info!(
            "[PARALLEL] Collected assets: {} speaker photos, {} sponsor logos",
            assets.speaker_photos.len(),
            assets.sponsor_logos.len()
        );

        // Generate article titles (we'll use these in the compositions)
        let speakers_title = format!("Meet the Speakers at {}", workflow.conference_name);
        let side_events_title = format!("Side Events Guide: {}", workflow.conference_name);
        let press_title = format!("Coverage Announcement: {}", workflow.conference_name);

        // Compose Speakers Article Thumbnail
        match self.graphics_composer
            .compose_speakers_thumbnail(&workflow.conference_name, &speakers_title, &assets)
            .await
        {
            Ok(composition) => {
                tracing::info!(
                    "[PARALLEL] Composed speakers thumbnail with {} assets",
                    composition.assets.len()
                );
                // Render the composition (for now returns background URL)
                match self.graphics_composer.render_composition(&composition).await {
                    Ok(url) => {
                        thumbnails.insert("speakers".to_string(), ThumbnailAsset {
                            url,
                            width: composition.dimensions.width,
                            height: composition.dimensions.height,
                            format: "png".to_string(),
                        });
                    }
                    Err(e) => tracing::warn!("[PARALLEL] Failed to render speakers thumbnail: {}", e),
                }
            }
            Err(e) => tracing::warn!("[PARALLEL] Failed to compose speakers thumbnail: {}", e),
        }

        // Compose Side Events Article Thumbnail (with sponsor logos)
        if !research.side_events.is_empty() {
            match self.graphics_composer
                .compose_side_events_thumbnail(
                    &workflow.conference_name,
                    &side_events_title,
                    &assets,
                    research.side_events.len(),
                )
                .await
            {
                Ok(composition) => {
                    tracing::info!(
                        "[PARALLEL] Composed side events thumbnail with {} sponsor logos",
                        composition.assets.len()
                    );
                    match self.graphics_composer.render_composition(&composition).await {
                        Ok(url) => {
                            thumbnails.insert("side_events".to_string(), ThumbnailAsset {
                                url,
                                width: composition.dimensions.width,
                                height: composition.dimensions.height,
                                format: "png".to_string(),
                            });
                        }
                        Err(e) => tracing::warn!("[PARALLEL] Failed to render side events thumbnail: {}", e),
                    }
                }
                Err(e) => tracing::warn!("[PARALLEL] Failed to compose side events thumbnail: {}", e),
            }
        }

        // Compose Press Release Thumbnail
        match self.graphics_composer
            .compose_press_release_thumbnail(&workflow.conference_name, &press_title, &assets)
            .await
        {
            Ok(composition) => {
                tracing::info!("[PARALLEL] Composed press release thumbnail");
                match self.graphics_composer.render_composition(&composition).await {
                    Ok(url) => {
                        thumbnails.insert("press_release".to_string(), ThumbnailAsset {
                            url,
                            width: composition.dimensions.width,
                            height: composition.dimensions.height,
                            format: "png".to_string(),
                        });
                    }
                    Err(e) => tracing::warn!("[PARALLEL] Failed to render press release thumbnail: {}", e),
                }
            }
            Err(e) => tracing::warn!("[PARALLEL] Failed to compose press release thumbnail: {}", e),
        }

        // Generate social media graphic via Maci/ComfyUI
        match self.generate_social_graphic(workflow).await {
            Ok(graphic) => {
                social_graphics.push(graphic);
            }
            Err(e) => tracing::warn!("[PARALLEL] Failed to generate social graphic: {}", e),
        }

        tracing::info!(
            "[PARALLEL] Graphics workflow complete: {} thumbnails, {} social graphics",
            thumbnails.len(),
            social_graphics.len()
        );

        Ok(GraphicsResult {
            thumbnails,
            social_graphics,
            execution_id: None,
        })
    }

    /// Generate social media graphic (square format) via Maci/ComfyUI
    async fn generate_social_graphic(
        &self,
        workflow: &ConferenceWorkflow,
    ) -> Result<SocialGraphic> {
        let prompt = format!(
            "Eye-catching social media announcement graphic for '{}' conference. \
            Bold, modern design with vibrant colors and tech aesthetic. \
            Abstract geometric patterns suggesting innovation and connection. \
            Perfect for Instagram/Twitter share. Location: {}. \
            Minimalist style, no text - image only.",
            workflow.conference_name,
            workflow.location.as_deref().unwrap_or("tech venue")
        );

        // Use Maci/ComfyUI for image generation (1024x1024 square)
        let url = if let Some(ref cinematics) = self.cinematics {
            cinematics.generate_static_image_url(&prompt, 1024, 1024).await
                .map_err(|e| NoraError::ExecutionError(format!("Maci graphic generation failed: {}", e)))?
        } else {
            return Err(NoraError::ExecutionError(
                "CinematicsService (Maci) not available for social graphic generation".to_string()
            ));
        };

        Ok(SocialGraphic {
            platform: "universal".to_string(),
            aspect_ratio: "1:1".to_string(),
            url,
        })
    }

    /// Prepare inputs for content workflow
    fn prepare_content_inputs(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> std::collections::HashMap<String, serde_json::Value> {
        let mut inputs = std::collections::HashMap::new();

        inputs.insert("conference_name".to_string(), serde_json::json!(workflow.conference_name));
        inputs.insert("start_date".to_string(), serde_json::json!(workflow.start_date));
        inputs.insert("end_date".to_string(), serde_json::json!(workflow.end_date));
        inputs.insert("location".to_string(), serde_json::json!(workflow.location));

        // Add entity data
        let speakers: Vec<_> = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Speaker)
            .map(|e| serde_json::json!({
                "name": e.canonical_name,
                "title": e.title,
                "company": e.company,
                "bio": e.bio,
            }))
            .collect();
        inputs.insert("speakers".to_string(), serde_json::json!(speakers));

        // Add side events
        let side_events: Vec<_> = research
            .side_events
            .iter()
            .map(|e| serde_json::json!({
                "name": e.name,
                "date": e.event_date,
                "venue": e.venue_name,
                "url": e.event_url,
            }))
            .collect();
        inputs.insert("side_events".to_string(), serde_json::json!(side_events));

        inputs
    }

    /// Prepare inputs for graphics workflow
    fn prepare_graphics_inputs(
        &self,
        workflow: &ConferenceWorkflow,
        research: &ResearchFlowResult,
    ) -> std::collections::HashMap<String, serde_json::Value> {
        let mut inputs = std::collections::HashMap::new();

        inputs.insert("conference_name".to_string(), serde_json::json!(workflow.conference_name));
        inputs.insert("location".to_string(), serde_json::json!(workflow.location));

        // Add speaker photos
        let speaker_photos: Vec<_> = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Speaker && e.photo_url.is_some())
            .map(|e| serde_json::json!({
                "name": e.canonical_name,
                "photo_url": e.photo_url,
            }))
            .collect();
        inputs.insert("speaker_photos".to_string(), serde_json::json!(speaker_photos));

        // Add brand logos
        let brand_logos: Vec<_> = research
            .entities
            .iter()
            .filter(|e| e.entity_type == db::models::entity::EntityType::Sponsor && e.photo_url.is_some())
            .map(|e| serde_json::json!({
                "name": e.canonical_name,
                "logo_url": e.photo_url,
            }))
            .collect();
        inputs.insert("brand_logos".to_string(), serde_json::json!(brand_logos));

        inputs
    }
}

/// Extract JSON from LLM response that may have text before/after the JSON
fn extract_json_from_response(response: &str) -> Option<&str> {
    // Find the first { and last } to extract the JSON object
    let start = response.find('{')?;
    let end = response.rfind('}')?;
    if start < end {
        Some(&response[start..=end])
    } else {
        None
    }
}

/// Parse JSON from LLM response with fallback
fn parse_article_response(response: &str, fallback_title: &str, fallback_caption: &str) -> ArticleResponse {
    // Try to extract JSON from the response
    let json_str = extract_json_from_response(response).unwrap_or(response);

    serde_json::from_str(json_str).unwrap_or_else(|e| {
        tracing::warn!("[PARALLEL] Failed to parse article JSON: {}", e);
        // Check if response looks like it might be partially valid
        let body = if response.len() > 100 && !response.starts_with('{') {
            // Response might be the article body directly
            response.to_string()
        } else {
            format!("Content generation completed. Raw output available for review.\n\n{}",
                &response[..response.len().min(500)])
        };
        ArticleResponse {
            title: fallback_title.to_string(),
            body,
            social_caption: fallback_caption.to_string(),
            hashtags: vec![],
        }
    })
}

/// Article response structure for JSON parsing
#[derive(Debug, Clone, Deserialize)]
struct ArticleResponse {
    title: String,
    body: String,
    social_caption: String,
    hashtags: Vec<String>,
}

/// Simple slugify helper
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

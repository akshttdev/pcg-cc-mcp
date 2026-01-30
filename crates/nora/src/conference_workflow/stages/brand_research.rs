//! Stage 3: Brand Research
//!
//! Parallel research of sponsors and brands:
//! - Company information
//! - Social media presence
//! - Logo/branding assets
//! - Sponsorship level

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use services::services::image::ImageService;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

use db::models::{
    conference_workflow::ConferenceWorkflow,
    entity::{CreateEntity, Entity, EntityType},
    entity_appearance::{AppearanceType, CreateEntityAppearance, EntityAppearance},
};

use crate::{
    execution::{research::ResearchTools, ExecutionEngine},
    NoraError, Result,
};

use super::{ResearchStage, ResearchStageResult};

/// Discovered brand/sponsor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandInfo {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo_url: Option<String>,
    pub industry: Option<String>,
    pub headquarters: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub sponsorship_level: Option<String>,
}

/// Brand Research stage
pub struct BrandResearchStage {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    research_tools: ResearchTools,
}

impl BrandResearchStage {
    pub fn new(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            pool,
            execution_engine,
            research_tools: ResearchTools::new(),
        }
    }

    /// Execute brand research in parallel
    pub async fn execute(
        &self,
        workflow: &ConferenceWorkflow,
        intel_result: &ResearchStageResult,
        parallelism_limit: usize,
    ) -> Result<Vec<ResearchStageResult>> {
        // Extract sponsor names from intel result
        let sponsor_names: Vec<String> = intel_result.data
            .get("sponsor_names")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        if sponsor_names.is_empty() {
            tracing::info!("[BRAND_RESEARCH] No sponsors to research");
            return Ok(vec![]);
        }

        tracing::info!(
            "[BRAND_RESEARCH] Researching {} brands with parallelism {}",
            sponsor_names.len(),
            parallelism_limit
        );

        let semaphore = Arc::new(Semaphore::new(parallelism_limit));
        let pool = self.pool.clone();
        let board_id = workflow.conference_board_id;

        let mut handles = Vec::new();

        for brand_name in sponsor_names {
            let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
                NoraError::ExecutionError(format!("Semaphore error: {}", e))
            })?;

            let pool = pool.clone();
            let name = brand_name.clone();
            let conf_name = workflow.conference_name.clone();
            // Create new ResearchTools for each task (they're lightweight)
            let tools = ResearchTools::new();

            let handle = tokio::spawn(async move {
                let result = research_single_brand(&pool, &tools, &name, &conf_name, board_id).await;
                drop(permit);
                result
            });

            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    tracing::error!("[BRAND_RESEARCH] Brand research failed: {}", e);
                }
                Err(e) => {
                    tracing::error!("[BRAND_RESEARCH] Task panicked: {}", e);
                }
            }
        }

        tracing::info!(
            "[BRAND_RESEARCH] Completed {} brand profiles",
            results.len()
        );

        Ok(results)
    }
}

/// Research a single brand
async fn research_single_brand(
    pool: &SqlitePool,
    research_tools: &ResearchTools,
    brand_name: &str,
    conference_name: &str,
    board_id: Uuid,
) -> Result<ResearchStageResult> {
    let started_at = Utc::now();
    let mut result = ResearchStageResult::new(ResearchStage::BrandResearch, started_at);

    tracing::debug!("[BRAND_RESEARCH] Researching brand: {}", brand_name);

    // Check for existing entity with fresh research
    let existing = Entity::find_by_name(pool, brand_name)
        .await
        .map_err(NoraError::DatabaseError)?;

    if let Some(entity) = existing {
        if entity.is_research_fresh(Duration::days(30)) {
            tracing::debug!(
                "[BRAND_RESEARCH] Using cached research for: {}",
                brand_name
            );

            EntityAppearance::find_or_create(
                pool,
                entity.id,
                board_id,
                AppearanceType::Sponsor,
            )
            .await
            .map_err(NoraError::DatabaseError)?;

            let summary = format!("Reused existing profile for {}", brand_name);
            result = result
                .complete(summary, serde_json::json!({"reused": true}))
                .with_entity(entity);

            return Ok(result);
        }
    }

    // Perform new research using LLM
    let mut brand_info = research_brand_profile(research_tools, brand_name, conference_name).await?;

    // Download brand logo if URL found
    let local_logo_url = if let Some(ref remote_url) = brand_info.logo_url {
        match ImageService::new(pool.clone()) {
            Ok(image_service) => {
                match image_service.download_image_from_url(remote_url).await {
                    Some(image) => {
                        let local_url = format!("/api/images/{}/file", image.id);
                        tracing::info!(
                            "[BRAND_RESEARCH] Downloaded logo for {}: {} -> {}",
                            brand_name,
                            remote_url,
                            local_url
                        );
                        Some(local_url)
                    }
                    None => {
                        tracing::warn!(
                            "[BRAND_RESEARCH] Failed to download logo for {}: {}",
                            brand_name,
                            remote_url
                        );
                        Some(remote_url.clone())
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[BRAND_RESEARCH] ImageService unavailable: {}", e);
                Some(remote_url.clone())
            }
        }
    } else {
        None
    };

    // Update brand_info with local logo URL
    if local_logo_url.is_some() {
        brand_info.logo_url = local_logo_url.clone();
    }

    // Create or update entity
    let entity = Entity::find_or_create(pool, EntityType::Sponsor, brand_name)
        .await
        .map_err(NoraError::DatabaseError)?;

    // Update entity with research
    let update = db::models::entity::UpdateEntity {
        canonical_name: None,
        external_ids: Some(db::models::entity::ExternalIds {
            linkedin: brand_info.linkedin_url.clone(),
            twitter: brand_info.twitter_handle.clone(),
            website: brand_info.website.clone(),
            youtube: None,
            github: None,
            crunchbase: None,
        }),
        bio: brand_info.description.clone(),
        title: brand_info.industry.clone(),
        company: Some(brand_name.to_string()),
        photo_url: local_logo_url,
        social_profiles: None,
        social_analysis: None,
        data_completeness: Some(calculate_completeness(&brand_info)),
    };

    let updated_entity = Entity::update(pool, entity.id, &update)
        .await
        .map_err(NoraError::DatabaseError)?;

    Entity::mark_researched(pool, entity.id)
        .await
        .map_err(NoraError::DatabaseError)?;

    // Create appearance
    let appearance = CreateEntityAppearance {
        entity_id: updated_entity.id,
        conference_board_id: board_id,
        appearance_type: AppearanceType::Sponsor,
        talk_title: None,
        talk_description: None,
        talk_slot: None,
    };

    EntityAppearance::create(pool, &appearance)
        .await
        .map_err(NoraError::DatabaseError)?;

    let summary = format!(
        "Researched {} - {}",
        brand_name,
        brand_info.industry.as_deref().unwrap_or("Unknown industry")
    );

    let data = serde_json::to_value(&brand_info).unwrap_or_default();
    result = result.complete(summary, data).with_entity(updated_entity);

    Ok(result)
}

/// Research brand profile using LLM and web search
async fn research_brand_profile(
    tools: &ResearchTools,
    name: &str,
    conference_name: &str,
) -> Result<BrandInfo> {
    tracing::info!("[BRAND_RESEARCH] Researching profile for: {}", name);

    // Step 1: Web search for brand information
    let search_queries = vec![
        format!("{} company official website", name),
        format!("{} sponsor {} conference", name, conference_name),
        format!("{} company LinkedIn about", name),
    ];

    let mut all_search_results = Vec::new();
    for query in &search_queries {
        match tools.web_search(query, 5).await {
            Ok(results) => {
                tracing::debug!("[BRAND_RESEARCH] Found {} results for: {}", results.len(), query);
                all_search_results.extend(results);
            }
            Err(e) => {
                tracing::warn!("[BRAND_RESEARCH] Search failed for '{}': {}", query, e);
            }
        }
    }

    // Step 2: Build LLM prompt for analysis
    let system_prompt = r#"You are an expert researcher building company/brand profiles for conference coverage.

Based on the provided search results, extract comprehensive information about this brand/sponsor.

Output a JSON object with these exact fields:
{
  "name": "string (company name)",
  "description": "string or null (company description, 2-3 sentences)",
  "website": "string or null (official website URL)",
  "logo_url": "string or null (URL to company logo if found)",
  "industry": "string or null (primary industry/sector)",
  "headquarters": "string or null (HQ location)",
  "linkedin_url": "string or null (company LinkedIn page)",
  "twitter_handle": "string or null (@handle format)",
  "sponsorship_level": "string or null (if mentioned: platinum, gold, silver, etc.)"
}

Return ONLY valid JSON, no markdown formatting."#;

    let search_context = if all_search_results.is_empty() {
        format!("No search results available. Use your knowledge about {} if known.", name)
    } else {
        all_search_results
            .iter()
            .take(10)
            .map(|r| format!("**{}**\nURL: {}\n{}\n", r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n---\n")
    };

    let user_prompt = format!(
        r#"Research this brand/sponsor and extract their profile:

## Brand Details
- Name: {}
- Conference: {}

## Search Results
{}

Extract all available information and return a comprehensive JSON profile."#,
        name,
        conference_name,
        search_context
    );

    // Step 3: Call LLM for analysis
    let response = tools.research_llm(system_prompt, &user_prompt).await
        .map_err(|e| NoraError::ExecutionError(format!("LLM research failed: {}", e)))?;

    // Step 4: Parse the response - extract JSON from potential markdown fences
    let json_str = extract_json_from_response(&response);
    let brand_info: BrandInfo = serde_json::from_str(json_str)
        .unwrap_or_else(|e| {
            tracing::warn!("[BRAND_RESEARCH] Failed to parse LLM response for {}: {}. JSON: {}", name, e, &json_str[..json_str.len().min(200)]);
            // Return basic info
            BrandInfo {
                name: name.to_string(),
                description: None,
                website: None,
                logo_url: None,
                industry: None,
                headquarters: None,
                linkedin_url: None,
                twitter_handle: None,
                sponsorship_level: None,
            }
        });

    tracing::info!(
        "[BRAND_RESEARCH] Profile for {}: {} - {}",
        name,
        brand_info.industry.as_deref().unwrap_or("?"),
        brand_info.description.as_ref().map(|d| format!("{} chars", d.len())).unwrap_or_else(|| "no description".to_string())
    );

    Ok(brand_info)
}

/// Calculate data completeness score
fn calculate_completeness(info: &BrandInfo) -> f64 {
    let mut score = 0.0;
    let mut total = 0.0;

    let fields = [
        (info.name.is_empty(), 0.15),
        (info.description.is_some(), 0.20),
        (info.website.is_some(), 0.15),
        (info.logo_url.is_some(), 0.15),
        (info.industry.is_some(), 0.10),
        (info.linkedin_url.is_some(), 0.15),
        (info.twitter_handle.is_some(), 0.10),
    ];

    for (has_value, weight) in fields {
        total += weight;
        if has_value {
            score += weight;
        }
    }

    score / total
}

/// QA checklist for brand research
pub fn qa_checklist() -> Vec<&'static str> {
    vec![
        "Company description present",
        "Website URL captured",
        "Logo URL available",
        "Social handles found",
        "Industry identified",
    ]
}

/// Extract JSON from LLM response that may be wrapped in markdown code fences
fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();

    if trimmed.starts_with("```") {
        if let Some(start) = trimmed.find('\n') {
            let after_fence = &trimmed[start + 1..];
            if let Some(end) = after_fence.rfind("```") {
                return after_fence[..end].trim();
            }
        }
    }

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if start < end {
                return &trimmed[start..=end];
            }
        }
    }

    trimmed
}

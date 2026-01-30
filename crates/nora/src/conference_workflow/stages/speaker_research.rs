//! Stage 2: Speaker Research
//!
//! Parallel research of all discovered speakers:
//! - Bio and credentials
//! - Social media presence
//! - Photo/headshot
//! - Talk topics and past presentations

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

/// Discovered speaker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerInfo {
    pub name: String,
    pub title: Option<String>,
    pub company: Option<String>,
    pub bio: Option<String>,
    pub talk_title: Option<String>,
    pub talk_description: Option<String>,
    pub photo_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub website: Option<String>,
    pub expertise: Vec<String>,
    pub past_talks: Vec<String>,
}

/// Speaker Research stage
pub struct SpeakerResearchStage {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    research_tools: ResearchTools,
}

impl SpeakerResearchStage {
    pub fn new(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            pool,
            execution_engine,
            research_tools: ResearchTools::new(),
        }
    }

    /// Execute speaker research in parallel
    pub async fn execute(
        &self,
        workflow: &ConferenceWorkflow,
        intel_result: &ResearchStageResult,
        parallelism_limit: usize,
    ) -> Result<Vec<ResearchStageResult>> {
        // Extract speaker names from intel result
        let speaker_names: Vec<String> = intel_result.data
            .get("speaker_names")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        if speaker_names.is_empty() {
            tracing::info!("[SPEAKER_RESEARCH] No speakers to research");
            return Ok(vec![]);
        }

        tracing::info!(
            "[SPEAKER_RESEARCH] Researching {} speakers with parallelism {}",
            speaker_names.len(),
            parallelism_limit
        );

        let semaphore = Arc::new(Semaphore::new(parallelism_limit));
        let pool = self.pool.clone();
        let board_id = workflow.conference_board_id;
        let conference_name = workflow.conference_name.clone();

        let mut handles = Vec::new();

        for speaker_name in speaker_names {
            let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
                NoraError::ExecutionError(format!("Semaphore error: {}", e))
            })?;

            let pool = pool.clone();
            let name = speaker_name.clone();
            let conf_name = conference_name.clone();
            // Create new ResearchTools for each task (they're lightweight)
            let tools = ResearchTools::new();

            let handle = tokio::spawn(async move {
                let result = research_single_speaker(&pool, &tools, &name, &conf_name, board_id).await;
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
                    tracing::error!("[SPEAKER_RESEARCH] Speaker research failed: {}", e);
                }
                Err(e) => {
                    tracing::error!("[SPEAKER_RESEARCH] Task panicked: {}", e);
                }
            }
        }

        tracing::info!(
            "[SPEAKER_RESEARCH] Completed {} speaker profiles",
            results.len()
        );

        Ok(results)
    }
}

/// Research a single speaker
async fn research_single_speaker(
    pool: &SqlitePool,
    research_tools: &ResearchTools,
    speaker_name: &str,
    conference_name: &str,
    board_id: Uuid,
) -> Result<ResearchStageResult> {
    let started_at = Utc::now();
    let mut result = ResearchStageResult::new(ResearchStage::SpeakerResearch, started_at);

    tracing::debug!("[SPEAKER_RESEARCH] Researching speaker: {}", speaker_name);

    // Check for existing entity with fresh research
    let existing = Entity::find_by_name(pool, speaker_name)
        .await
        .map_err(NoraError::DatabaseError)?;

    if let Some(entity) = existing {
        if entity.is_research_fresh(Duration::days(30)) {
            tracing::debug!(
                "[SPEAKER_RESEARCH] Using cached research for: {}",
                speaker_name
            );

            // Create appearance linking
            EntityAppearance::find_or_create(
                pool,
                entity.id,
                board_id,
                AppearanceType::Speaker,
            )
            .await
            .map_err(NoraError::DatabaseError)?;

            let summary = format!("Reused existing profile for {}", speaker_name);
            result = result
                .complete(summary, serde_json::json!({"reused": true}))
                .with_entity(entity);

            return Ok(result);
        }
    }

    // Perform new research using LLM
    let mut speaker_info = research_speaker_profile(research_tools, speaker_name, conference_name).await?;

    // Download speaker photo if URL found
    let local_photo_url = if let Some(ref remote_url) = speaker_info.photo_url {
        match ImageService::new(pool.clone()) {
            Ok(image_service) => {
                match image_service.download_image_from_url(remote_url).await {
                    Some(image) => {
                        let local_url = format!("/api/images/{}/file", image.id);
                        tracing::info!(
                            "[SPEAKER_RESEARCH] Downloaded photo for {}: {} -> {}",
                            speaker_name,
                            remote_url,
                            local_url
                        );
                        Some(local_url)
                    }
                    None => {
                        tracing::warn!(
                            "[SPEAKER_RESEARCH] Failed to download photo for {}: {}",
                            speaker_name,
                            remote_url
                        );
                        // Keep the remote URL as fallback
                        Some(remote_url.clone())
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[SPEAKER_RESEARCH] ImageService unavailable: {}", e);
                Some(remote_url.clone())
            }
        }
    } else {
        None
    };

    // Update speaker_info with local photo URL if downloaded
    if local_photo_url.is_some() {
        speaker_info.photo_url = local_photo_url.clone();
    }

    // Create or update entity
    let entity = Entity::find_or_create(pool, EntityType::Speaker, speaker_name)
        .await
        .map_err(NoraError::DatabaseError)?;

    // Update entity with research
    let update = db::models::entity::UpdateEntity {
        canonical_name: None,
        external_ids: Some(db::models::entity::ExternalIds {
            linkedin: speaker_info.linkedin_url.clone(),
            twitter: speaker_info.twitter_handle.clone(),
            website: speaker_info.website.clone(),
            youtube: None,
            github: None,
            crunchbase: None,
        }),
        bio: speaker_info.bio.clone(),
        title: speaker_info.title.clone(),
        company: speaker_info.company.clone(),
        photo_url: local_photo_url,
        social_profiles: None,
        social_analysis: None,
        data_completeness: Some(calculate_completeness(&speaker_info)),
    };

    let updated_entity = Entity::update(pool, entity.id, &update)
        .await
        .map_err(NoraError::DatabaseError)?;

    Entity::mark_researched(pool, entity.id)
        .await
        .map_err(NoraError::DatabaseError)?;

    // Create appearance with talk info
    let appearance = CreateEntityAppearance {
        entity_id: updated_entity.id,
        conference_board_id: board_id,
        appearance_type: AppearanceType::Speaker,
        talk_title: speaker_info.talk_title.clone(),
        talk_description: speaker_info.talk_description.clone(),
        talk_slot: None,
    };

    EntityAppearance::create(pool, &appearance)
        .await
        .map_err(NoraError::DatabaseError)?;

    let summary = format!(
        "Researched {} - {} at {}",
        speaker_name,
        speaker_info.title.as_deref().unwrap_or("Unknown title"),
        speaker_info.company.as_deref().unwrap_or("Unknown company")
    );

    let data = serde_json::to_value(&speaker_info).unwrap_or_default();
    result = result.complete(summary, data).with_entity(updated_entity);

    Ok(result)
}

/// Research speaker profile using LLM and web search
async fn research_speaker_profile(
    tools: &ResearchTools,
    name: &str,
    conference_name: &str,
) -> Result<SpeakerInfo> {
    tracing::info!("[SPEAKER_RESEARCH] Researching profile for: {}", name);

    // Step 1: Web search for speaker information
    let search_queries = vec![
        format!("{} speaker {} conference", name, conference_name),
        format!("{} LinkedIn profile", name),
        format!("{} Twitter bio tech speaker", name),
    ];

    let mut all_search_results = Vec::new();
    for query in &search_queries {
        match tools.web_search(query, 5).await {
            Ok(results) => {
                tracing::debug!("[SPEAKER_RESEARCH] Found {} results for: {}", results.len(), query);
                all_search_results.extend(results);
            }
            Err(e) => {
                tracing::warn!("[SPEAKER_RESEARCH] Search failed for '{}': {}", query, e);
            }
        }
    }

    // Step 2: Build LLM prompt for analysis
    let system_prompt = r#"You are an expert researcher building speaker profiles for conference coverage.

Based on the provided search results, extract comprehensive information about this speaker.

Output a JSON object with these exact fields:
{
  "name": "string (full name)",
  "title": "string or null (job title)",
  "company": "string or null (current company/organization)",
  "bio": "string or null (2-3 sentence professional bio)",
  "talk_title": "string or null (if speaking at this specific conference)",
  "talk_description": "string or null",
  "photo_url": "string or null (URL to professional headshot if found)",
  "linkedin_url": "string or null (full LinkedIn URL)",
  "twitter_handle": "string or null (@handle format)",
  "website": "string or null (personal website)",
  "expertise": ["array", "of", "expertise", "areas"],
  "past_talks": ["array", "of", "past", "talk", "titles"]
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
        r#"Research this speaker and extract their profile:

## Speaker Details
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
    let speaker_info: SpeakerInfo = serde_json::from_str(json_str)
        .unwrap_or_else(|e| {
            tracing::warn!("[SPEAKER_RESEARCH] Failed to parse LLM response for {}: {}. JSON: {}", name, e, &json_str[..json_str.len().min(200)]);
            // Return basic info
            SpeakerInfo {
                name: name.to_string(),
                title: None,
                company: None,
                bio: None,
                talk_title: None,
                talk_description: None,
                photo_url: None,
                linkedin_url: None,
                twitter_handle: None,
                website: None,
                expertise: vec![],
                past_talks: vec![],
            }
        });

    tracing::info!(
        "[SPEAKER_RESEARCH] Profile for {}: {} at {}, bio: {} chars",
        name,
        speaker_info.title.as_deref().unwrap_or("?"),
        speaker_info.company.as_deref().unwrap_or("?"),
        speaker_info.bio.as_ref().map(|b| b.len()).unwrap_or(0)
    );

    Ok(speaker_info)
}

/// Calculate data completeness score
fn calculate_completeness(info: &SpeakerInfo) -> f64 {
    let mut score = 0.0;
    let mut total = 0.0;

    // Weight different fields
    let fields = [
        (!info.name.is_empty(), 0.15),
        (info.bio.is_some(), 0.20),
        (info.title.is_some(), 0.10),
        (info.company.is_some(), 0.10),
        (info.photo_url.is_some(), 0.15),
        (info.linkedin_url.is_some(), 0.10),
        (info.twitter_handle.is_some(), 0.10),
        (info.talk_title.is_some(), 0.10),
    ];

    for (has_value, weight) in fields {
        total += weight;
        if has_value {
            score += weight;
        }
    }

    if total > 0.0 { score / total } else { 0.0 }
}

/// QA checklist for speaker research
pub fn qa_checklist() -> Vec<&'static str> {
    vec![
        "Speaker bio present",
        "Social handles found",
        "Photo URL available",
        "Talk topic captured",
        "Company information",
    ]
}

/// Extract JSON from LLM response that may be wrapped in markdown code fences
fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();

    // Check for ```json ... ``` or ``` ... ``` wrapping
    if trimmed.starts_with("```") {
        if let Some(start) = trimmed.find('\n') {
            let after_fence = &trimmed[start + 1..];
            if let Some(end) = after_fence.rfind("```") {
                return after_fence[..end].trim();
            }
        }
    }

    // Try to find JSON object boundaries
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if start < end {
                return &trimmed[start..=end];
            }
        }
    }

    trimmed
}

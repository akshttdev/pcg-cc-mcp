//! Stage 1: Conference Intel
//!
//! Initial research stage that gathers comprehensive information about the conference:
//! - Conference name, dates, location
//! - Website validation and content extraction
//! - Theme/track identification
//! - Initial speaker/sponsor lists

use chrono::Utc;
use serde::{Deserialize, Serialize};
use services::services::image::ImageService;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use db::models::conference_workflow::ConferenceWorkflow;

use crate::{
    execution::{research::ResearchTools, ExecutionEngine},
    NoraError, Result,
};

use super::{ResearchStage, ResearchStageResult};

/// Extracted conference intelligence
///
/// All Vec fields use `#[serde(default)]` to handle missing fields in LLM responses
/// gracefully, avoiding deserialization failures that would result in empty data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceIntel {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub dates: String,
    #[serde(default)]
    pub location: String,
    pub website: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub tracks: Vec<String>,
    #[serde(default)]
    pub speaker_names: Vec<String>,
    #[serde(default)]
    pub sponsor_names: Vec<String>,
    pub venue_name: Option<String>,
    pub venue_address: Option<String>,
    pub ticket_info: Option<String>,
    #[serde(default)]
    pub social_handles: Vec<SocialHandle>,
    pub expected_attendance: Option<u32>,
    #[serde(default)]
    pub hashtags: Vec<String>,
    /// Conference cover/hero image (downloaded locally)
    #[serde(default)]
    pub cover_image_url: Option<String>,
    /// Additional discovered images (downloaded locally)
    #[serde(default)]
    pub additional_images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialHandle {
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
}

/// Conference Intel research stage
pub struct ConferenceIntelStage {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    research_tools: ResearchTools,
}

impl ConferenceIntelStage {
    pub fn new(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            pool,
            execution_engine,
            research_tools: ResearchTools::new(),
        }
    }

    /// Execute the conference intel stage
    pub async fn execute(&self, workflow: &ConferenceWorkflow) -> Result<ResearchStageResult> {
        let started_at = Utc::now();
        let mut result = ResearchStageResult::new(ResearchStage::ConferenceIntel, started_at);

        tracing::info!(
            "[CONFERENCE_INTEL] Researching conference: {}",
            workflow.conference_name
        );

        // Execute real research
        match self.execute_research(workflow).await {
            Ok(intel) => {
                let summary = format!(
                    "Discovered {} themes, {} speakers, {} sponsors for {}",
                    intel.themes.len(),
                    intel.speaker_names.len(),
                    intel.sponsor_names.len(),
                    workflow.conference_name
                );

                let data = serde_json::to_value(&intel).unwrap_or_default();
                result = result.complete(summary, data);

                tracing::info!(
                    "[CONFERENCE_INTEL] Completed: {} speakers, {} sponsors found",
                    intel.speaker_names.len(),
                    intel.sponsor_names.len()
                );
            }
            Err(e) => {
                tracing::error!("[CONFERENCE_INTEL] Research failed: {}", e);
                result = result.fail(&e.to_string());
            }
        }

        Ok(result)
    }

    /// Execute research using LLM and web search
    async fn execute_research(&self, workflow: &ConferenceWorkflow) -> Result<ConferenceIntel> {
        // Step 1: Web search for conference information
        let search_queries = vec![
            format!("{} conference {} official", workflow.conference_name, extract_year(&workflow.start_date)),
            format!("{} conference speakers agenda schedule", workflow.conference_name),
            format!("{} conference sponsors partners", workflow.conference_name),
        ];

        let mut all_search_results = Vec::new();
        for query in &search_queries {
            tracing::debug!("[CONFERENCE_INTEL] Searching: {}", query);
            match self.research_tools.web_search(query, 5).await {
                Ok(results) => {
                    tracing::debug!("[CONFERENCE_INTEL] Found {} results for query", results.len());
                    all_search_results.extend(results);
                }
                Err(e) => {
                    tracing::warn!("[CONFERENCE_INTEL] Search failed for '{}': {}", query, e);
                }
            }
        }

        tracing::info!(
            "[CONFERENCE_INTEL] Gathered {} total search results",
            all_search_results.len()
        );

        // Step 2: Try to fetch the conference website if provided
        let (website_content, website_html_raw) = if let Some(ref url) = workflow.website {
            // First fetch raw HTML for image extraction
            match reqwest::Client::new()
                .get(url)
                .header("User-Agent", "Mozilla/5.0 (compatible; Scout/1.0)")
                .send()
                .await
            {
                Ok(response) => {
                    if let Ok(html) = response.text().await {
                        tracing::info!("[CONFERENCE_INTEL] Fetched raw HTML: {} chars", html.len());
                        // Also get cleaned text for LLM
                        let text = self.research_tools.fetch_url(url).await.ok();
                        (text, Some(html))
                    } else {
                        (self.research_tools.fetch_url(url).await.ok(), None)
                    }
                }
                Err(e) => {
                    tracing::warn!("[CONFERENCE_INTEL] Failed to fetch website {}: {}", url, e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        // Step 2b: Extract and download images from website
        let mut downloaded_images: Vec<String> = Vec::new();
        if let (Some(ref html), Some(ref base_url)) = (&website_html_raw, &workflow.website) {
            let image_urls = self.research_tools.extract_images_from_html(html, base_url);
            tracing::info!("[CONFERENCE_INTEL] Found {} image URLs in website", image_urls.len());

            // Download up to 5 images
            if let Ok(image_service) = ImageService::new(self.pool.clone()) {
                for (i, img_url) in image_urls.iter().take(5).enumerate() {
                    if let Some(image) = image_service.download_image_from_url(img_url).await {
                        let local_url = format!("/api/images/{}/file", image.id);
                        tracing::info!(
                            "[CONFERENCE_INTEL] Downloaded image {}: {} -> {}",
                            i + 1,
                            img_url,
                            local_url
                        );
                        downloaded_images.push(local_url);
                    }
                }
            }
        }

        // Step 3: Build comprehensive prompt for LLM analysis
        let system_prompt = r#"You are an expert conference analyst. Your task is to analyze information about a conference and extract comprehensive intelligence.

Based on the provided search results and website content, extract:
1. Official conference name and tagline
2. Conference description (2-3 sentences)
3. Main themes and focus areas
4. Conference tracks or session categories
5. List of confirmed/announced speakers (names only)
6. List of sponsors and partners
7. Venue information
8. Expected attendance (if mentioned)
9. Official hashtags and social handles

Output a JSON object with these exact fields:
{
  "name": "string (official conference name)",
  "dates": "string (formatted date range)",
  "location": "string (city, venue)",
  "website": "string or null",
  "description": "string (2-3 sentence description)",
  "themes": ["array", "of", "themes"],
  "tracks": ["array", "of", "tracks"],
  "speaker_names": ["array", "of", "speaker", "names"],
  "sponsor_names": ["array", "of", "sponsor", "names"],
  "venue_name": "string or null",
  "venue_address": "string or null",
  "ticket_info": "string or null",
  "social_handles": [{"platform": "twitter", "handle": "@handle", "url": "optional"}],
  "expected_attendance": number or null,
  "hashtags": ["conference2026", "techweek", "example"]
}

Return ONLY valid JSON, no markdown formatting or explanation."#;

        // Format search results for the prompt
        let search_context = if all_search_results.is_empty() {
            "No search results available. Use your knowledge about this conference.".to_string()
        } else {
            all_search_results
                .iter()
                .take(15) // Limit to avoid token overflow
                .map(|r| format!("**{}**\nURL: {}\n{}\n", r.title, r.url, r.snippet))
                .collect::<Vec<_>>()
                .join("\n---\n")
        };

        let website_context = website_content
            .map(|c| {
                let truncated: String = c.chars().take(8000).collect();
                format!("\n\n## Website Content:\n{}", truncated)
            })
            .unwrap_or_default();

        let user_prompt = format!(
            r#"Analyze this conference and extract comprehensive intelligence:

## Known Conference Details
- Name: {}
- Dates: {} to {}
- Location: {}
- Website: {}

## Search Results
{}
{}

Extract all available information and return a comprehensive JSON object."#,
            workflow.conference_name,
            workflow.start_date,
            workflow.end_date,
            workflow.location.as_deref().unwrap_or("TBD"),
            workflow.website.as_deref().unwrap_or("Not provided"),
            search_context,
            website_context
        );

        // Step 4: Call LLM for analysis
        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| NoraError::ExecutionError(format!("LLM analysis failed: {}", e)))?;

        tracing::debug!("[CONFERENCE_INTEL] LLM response: {} chars", response.len());

        // Step 5: Parse the response - extract JSON from potential markdown fences
        let json_str = extract_json_from_response(&response);
        tracing::debug!("[CONFERENCE_INTEL] Extracted JSON: {} chars", json_str.len());

        let mut intel: ConferenceIntel = serde_json::from_str(json_str)
            .unwrap_or_else(|e| {
                tracing::warn!("[CONFERENCE_INTEL] Failed to parse LLM response: {}. JSON: {}", e, &json_str[..json_str.len().min(500)]);
                // Return fallback with basic info from workflow
                ConferenceIntel {
                    name: workflow.conference_name.clone(),
                    dates: format!("{} to {}", workflow.start_date, workflow.end_date),
                    location: workflow.location.clone().unwrap_or_else(|| "TBD".to_string()),
                    website: workflow.website.clone(),
                    description: Some(format!(
                        "{} is a conference taking place from {} to {} in {}.",
                        workflow.conference_name,
                        workflow.start_date,
                        workflow.end_date,
                        workflow.location.as_deref().unwrap_or("TBD")
                    )),
                    themes: vec!["Technology".to_string(), "Innovation".to_string()],
                    tracks: vec![],
                    speaker_names: vec![],
                    sponsor_names: vec![],
                    venue_name: None,
                    venue_address: None,
                    ticket_info: None,
                    social_handles: vec![],
                    expected_attendance: None,
                    hashtags: vec![format!("#{}", slugify(&workflow.conference_name))],
                    cover_image_url: None,
                    additional_images: vec![],
                }
            });

        // Add downloaded images to intel
        if !downloaded_images.is_empty() {
            intel.cover_image_url = Some(downloaded_images[0].clone());
            intel.additional_images = downloaded_images[1..].to_vec();
            tracing::info!(
                "[CONFERENCE_INTEL] Attached {} downloaded images (cover + {} additional)",
                downloaded_images.len(),
                downloaded_images.len() - 1
            );
        }

        tracing::info!(
            "[CONFERENCE_INTEL] Extracted: {} themes, {} tracks, {} speakers, {} sponsors",
            intel.themes.len(),
            intel.tracks.len(),
            intel.speaker_names.len(),
            intel.sponsor_names.len()
        );

        Ok(intel)
    }
}

/// Extract year from date string (YYYY-MM-DD)
fn extract_year(date: &str) -> &str {
    date.split('-').next().unwrap_or("2026")
}

/// Simple slugify helper for hashtags
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// QA checklist for conference intel stage
pub fn qa_checklist() -> Vec<&'static str> {
    vec![
        "Conference name captured correctly",
        "Dates captured and validated",
        "Location captured",
        "Website validated (if provided)",
        "At least one theme identified",
        "Speaker list extracted (if available)",
        "Sponsor list extracted (if available)",
    ]
}

/// Extract JSON from LLM response that may be wrapped in markdown code fences
fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();

    // Check for ```json ... ``` or ``` ... ``` wrapping
    if trimmed.starts_with("```") {
        // Find the end of the opening fence line
        if let Some(start) = trimmed.find('\n') {
            let after_fence = &trimmed[start + 1..];
            // Find closing ```
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

    // Return as-is if no extraction possible
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year("2026-02-20"), "2026");
        assert_eq!(extract_year("2025-12-01"), "2025");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("ETHDenver 2026"), "ethdenver2026");
        assert_eq!(slugify("Token2049 Singapore"), "token2049singapore");
    }
}

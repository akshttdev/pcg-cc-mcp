//! Stage 4: Production Team Research
//!
//! Research the production company and team behind the conference:
//! - Production company details
//! - Key contacts
//! - Past events produced
//! - Media/press contacts

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use db::models::{
    conference_workflow::ConferenceWorkflow,
    entity::{Entity, EntityType},
};

use crate::{
    execution::{research::ResearchTools, ExecutionEngine},
    NoraError, Result,
};

use super::{ResearchStage, ResearchStageResult};

/// Production team information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionTeamInfo {
    pub company_name: Option<String>,
    pub company_website: Option<String>,
    pub key_contacts: Vec<ProductionContact>,
    pub past_events: Vec<String>,
    pub media_contact: Option<ProductionContact>,
    pub press_kit_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionContact {
    pub name: String,
    pub role: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub linkedin: Option<String>,
}

/// Production Team research stage
pub struct ProductionTeamStage {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    research_tools: ResearchTools,
}

impl ProductionTeamStage {
    pub fn new(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            pool,
            execution_engine,
            research_tools: ResearchTools::new(),
        }
    }

    /// Execute production team research
    pub async fn execute(&self, workflow: &ConferenceWorkflow) -> Result<ResearchStageResult> {
        let started_at = Utc::now();
        let mut result = ResearchStageResult::new(ResearchStage::ProductionTeam, started_at);

        tracing::info!(
            "[PRODUCTION_TEAM] Researching production team for: {}",
            workflow.conference_name
        );

        match self.execute_research(workflow).await {
            Ok(info) => {
                // Create entity for production company if found
                if let Some(company_name) = &info.company_name {
                    if let Ok(entity) = Entity::find_or_create(
                        &self.pool,
                        EntityType::ProductionCompany,
                        company_name,
                    ).await {
                        result = result.with_entity(entity);
                    }
                }

                let summary = format!(
                    "Found production info: {} contacts, {} past events",
                    info.key_contacts.len(),
                    info.past_events.len()
                );

                let data = serde_json::to_value(&info).unwrap_or_default();
                result = result.complete(summary, data);

                tracing::info!(
                    "[PRODUCTION_TEAM] Completed: {} contacts found",
                    info.key_contacts.len()
                );
            }
            Err(e) => {
                tracing::error!("[PRODUCTION_TEAM] Research failed: {}", e);
                result = result.fail(&e.to_string());
            }
        }

        Ok(result)
    }

    /// Execute research using LLM and web search
    async fn execute_research(&self, workflow: &ConferenceWorkflow) -> Result<ProductionTeamInfo> {
        tracing::info!(
            "[PRODUCTION_TEAM] Researching production team for: {}",
            workflow.conference_name
        );

        // Step 1: Web search for production company information
        let search_queries = vec![
            format!("{} conference organizer producer", workflow.conference_name),
            format!("{} event production company", workflow.conference_name),
            format!("{} conference press contact media", workflow.conference_name),
        ];

        let mut all_search_results = Vec::new();
        for query in &search_queries {
            match self.research_tools.web_search(query, 5).await {
                Ok(results) => {
                    tracing::debug!("[PRODUCTION_TEAM] Found {} results for: {}", results.len(), query);
                    all_search_results.extend(results);
                }
                Err(e) => {
                    tracing::warn!("[PRODUCTION_TEAM] Search failed for '{}': {}", query, e);
                }
            }
        }

        // Try to fetch the conference website for production info
        let website_content = if let Some(ref url) = workflow.website {
            match self.research_tools.fetch_url(url).await {
                Ok(content) => {
                    tracing::debug!("[PRODUCTION_TEAM] Fetched website: {} chars", content.len());
                    Some(content)
                }
                Err(e) => {
                    tracing::warn!("[PRODUCTION_TEAM] Failed to fetch website: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Step 2: Build LLM prompt for analysis
        let system_prompt = r#"You are an expert researcher identifying production teams behind conferences.

Based on the provided search results and website content, extract information about the production company and team.

Output a JSON object with these exact fields:
{
  "company_name": "string or null (production company name)",
  "company_website": "string or null (production company website URL)",
  "key_contacts": [
    {
      "name": "string (contact name)",
      "role": "string or null (their role)",
      "email": "string or null",
      "phone": "string or null",
      "linkedin": "string or null"
    }
  ],
  "past_events": ["array", "of", "past", "event", "names"],
  "media_contact": {
    "name": "string",
    "role": "string or null",
    "email": "string or null",
    "phone": "string or null",
    "linkedin": "string or null"
  } or null,
  "press_kit_url": "string or null"
}

Return ONLY valid JSON, no markdown formatting."#;

        let search_context = if all_search_results.is_empty() {
            "No search results available.".to_string()
        } else {
            all_search_results
                .iter()
                .take(10)
                .map(|r| format!("**{}**\nURL: {}\n{}\n", r.title, r.url, r.snippet))
                .collect::<Vec<_>>()
                .join("\n---\n")
        };

        let website_context = website_content
            .map(|c| {
                let truncated: String = c.chars().take(5000).collect();
                format!("\n\n## Website Content:\n{}", truncated)
            })
            .unwrap_or_default();

        let user_prompt = format!(
            r#"Research the production team for this conference:

## Conference Details
- Name: {}
- Website: {}
- Location: {}

## Search Results
{}
{}

Identify the production company, key contacts, and press/media contacts."#,
            workflow.conference_name,
            workflow.website.as_deref().unwrap_or("Not provided"),
            workflow.location.as_deref().unwrap_or("Unknown"),
            search_context,
            website_context
        );

        // Step 3: Call LLM for analysis
        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| NoraError::ExecutionError(format!("LLM research failed: {}", e)))?;

        // Step 4: Parse the response - extract JSON from potential markdown fences
        let json_str = extract_json_from_response(&response);
        let production_info: ProductionTeamInfo = serde_json::from_str(json_str)
            .unwrap_or_else(|e| {
                tracing::warn!("[PRODUCTION_TEAM] Failed to parse LLM response: {}. JSON: {}...", e, &json_str[..json_str.len().min(200)]);
                ProductionTeamInfo {
                    company_name: None,
                    company_website: None,
                    key_contacts: vec![],
                    past_events: vec![],
                    media_contact: None,
                    press_kit_url: None,
                }
            });

        tracing::info!(
            "[PRODUCTION_TEAM] Found: {} contacts, {} past events",
            production_info.key_contacts.len(),
            production_info.past_events.len()
        );

        Ok(production_info)
    }
}

/// QA checklist for production team
pub fn qa_checklist() -> Vec<&'static str> {
    vec![
        "Production company identified",
        "Contact information found",
        "Past events listed",
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

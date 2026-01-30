//! Stage 5: Competitive Intel
//!
//! Analyze competitive landscape:
//! - Similar conferences
//! - Competitor coverage
//! - Market positioning
//! - Content gaps and opportunities

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use db::models::conference_workflow::ConferenceWorkflow;

use crate::{
    execution::{research::ResearchTools, ExecutionEngine},
    NoraError, Result,
};

use super::{ResearchStage, ResearchStageResult};

/// Competitive intelligence findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitiveIntel {
    pub similar_conferences: Vec<SimilarConference>,
    pub competitor_coverage: Vec<CompetitorCoverage>,
    pub market_positioning: Option<String>,
    pub content_gaps: Vec<String>,
    pub opportunities: Vec<String>,
    pub trending_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarConference {
    pub name: String,
    pub dates: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub overlap_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorCoverage {
    pub outlet_name: String,
    pub article_title: Option<String>,
    pub article_url: Option<String>,
    pub publish_date: Option<String>,
    pub coverage_type: String,
}

/// Competitive Intel research stage
pub struct CompetitiveIntelStage {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    research_tools: ResearchTools,
}

impl CompetitiveIntelStage {
    pub fn new(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            pool,
            execution_engine,
            research_tools: ResearchTools::new(),
        }
    }

    /// Execute competitive intel research
    pub async fn execute(&self, workflow: &ConferenceWorkflow) -> Result<ResearchStageResult> {
        let started_at = Utc::now();
        let mut result = ResearchStageResult::new(ResearchStage::CompetitiveIntel, started_at);

        tracing::info!(
            "[COMPETITIVE_INTEL] Analyzing competitive landscape for: {}",
            workflow.conference_name
        );

        match self.execute_research(workflow).await {
            Ok(intel) => {
                let summary = format!(
                    "Found {} similar conferences, {} content gaps, {} opportunities",
                    intel.similar_conferences.len(),
                    intel.content_gaps.len(),
                    intel.opportunities.len()
                );

                let data = serde_json::to_value(&intel).unwrap_or_default();
                result = result.complete(summary, data);

                tracing::info!(
                    "[COMPETITIVE_INTEL] Completed: {} opportunities identified",
                    intel.opportunities.len()
                );
            }
            Err(e) => {
                tracing::error!("[COMPETITIVE_INTEL] Research failed: {}", e);
                result = result.fail(&e.to_string());
            }
        }

        Ok(result)
    }

    /// Execute research using LLM and web search
    async fn execute_research(&self, workflow: &ConferenceWorkflow) -> Result<CompetitiveIntel> {
        tracing::info!(
            "[COMPETITIVE_INTEL] Researching competitive landscape for: {}",
            workflow.conference_name
        );

        // Step 1: Web search for competitive information
        let search_queries = vec![
            format!("{} similar conferences 2026", workflow.conference_name),
            format!("{} competitors coverage news", workflow.conference_name),
            format!("{} conference industry trends", workflow.conference_name),
            format!("{} press coverage media", workflow.conference_name),
        ];

        let mut all_search_results = Vec::new();
        for query in &search_queries {
            match self.research_tools.web_search(query, 5).await {
                Ok(results) => {
                    tracing::debug!("[COMPETITIVE_INTEL] Found {} results for: {}", results.len(), query);
                    all_search_results.extend(results);
                }
                Err(e) => {
                    tracing::warn!("[COMPETITIVE_INTEL] Search failed for '{}': {}", query, e);
                }
            }
        }

        // Step 2: Build LLM prompt for analysis
        let system_prompt = r#"You are a competitive intelligence analyst researching the conference landscape.

Based on the provided search results, analyze the competitive landscape and identify opportunities.

Output a JSON object with these exact fields:
{
  "similar_conferences": [
    {
      "name": "string (conference name)",
      "dates": "string or null (date range)",
      "location": "string or null",
      "website": "string or null",
      "overlap_score": number (0.0-1.0, how similar/competitive)
    }
  ],
  "competitor_coverage": [
    {
      "outlet_name": "string (media outlet name)",
      "article_title": "string or null",
      "article_url": "string or null",
      "publish_date": "string or null",
      "coverage_type": "string (preview, recap, interview, etc.)"
    }
  ],
  "market_positioning": "string or null (how this conference is positioned in the market)",
  "content_gaps": ["array", "of", "content", "gaps", "to", "fill"],
  "opportunities": ["array", "of", "unique", "coverage", "opportunities"],
  "trending_topics": ["array", "of", "trending", "industry", "topics"]
}

Return ONLY valid JSON, no markdown formatting."#;

        let search_context = if all_search_results.is_empty() {
            "No search results available. Use your general knowledge.".to_string()
        } else {
            all_search_results
                .iter()
                .take(15)
                .map(|r| format!("**{}**\nURL: {}\n{}\n", r.title, r.url, r.snippet))
                .collect::<Vec<_>>()
                .join("\n---\n")
        };

        let user_prompt = format!(
            r#"Analyze the competitive landscape for this conference:

## Conference Details
- Name: {}
- Dates: {} to {}
- Location: {}
- Website: {}

## Search Results
{}

Identify similar conferences, competitor coverage, content gaps, and unique opportunities for our coverage."#,
            workflow.conference_name,
            workflow.start_date,
            workflow.end_date,
            workflow.location.as_deref().unwrap_or("Unknown"),
            workflow.website.as_deref().unwrap_or("Not provided"),
            search_context
        );

        // Step 3: Call LLM for analysis
        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| NoraError::ExecutionError(format!("LLM research failed: {}", e)))?;

        // Step 4: Parse the response - extract JSON from potential markdown fences
        let json_str = extract_json_from_response(&response);
        let competitive_intel: CompetitiveIntel = serde_json::from_str(json_str)
            .unwrap_or_else(|e| {
                tracing::warn!("[COMPETITIVE_INTEL] Failed to parse LLM response: {}. JSON: {}...", e, &json_str[..json_str.len().min(200)]);
                CompetitiveIntel {
                    similar_conferences: vec![],
                    competitor_coverage: vec![],
                    market_positioning: None,
                    content_gaps: vec![],
                    opportunities: vec![],
                    trending_topics: vec![],
                }
            });

        tracing::info!(
            "[COMPETITIVE_INTEL] Found: {} similar conferences, {} opportunities, {} trending topics",
            competitive_intel.similar_conferences.len(),
            competitive_intel.opportunities.len(),
            competitive_intel.trending_topics.len()
        );

        Ok(competitive_intel)
    }
}

/// QA checklist for competitive intel
pub fn qa_checklist() -> Vec<&'static str> {
    vec![
        "Similar conferences identified",
        "Content gaps analyzed",
        "Opportunities documented",
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

//! Eventbrite Event Scraper
//!
//! Discovers events from Eventbrite platform using web search + LLM analysis

use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

use crate::execution::research::ResearchTools;
use super::{calculate_relevance, ScrapedEvent, ScraperError};

/// Eventbrite event scraper
pub struct EventbriteScraper {
    client: Client,
    research_tools: ResearchTools,
}

impl EventbriteScraper {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            research_tools: ResearchTools::new(),
        }
    }

    /// Search for events on Eventbrite using web search + LLM analysis
    pub async fn search(
        &self,
        location: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        conference_name: &str,
    ) -> Result<Vec<ScrapedEvent>, ScraperError> {
        tracing::info!(
            "[EVENTBRITE_SCRAPER] Searching Eventbrite for {} side events in {} ({} to {})",
            conference_name,
            location,
            start_date,
            end_date
        );

        // Step 1: Web search for Eventbrite events
        let search_queries = vec![
            format!("site:eventbrite.com {} {} {}", conference_name, location, start_date.format("%B %Y")),
            format!("site:eventbrite.com {} side event networking", conference_name),
            format!("eventbrite {} {} tech crypto blockchain", location, start_date.format("%B %Y")),
        ];

        let mut all_search_results = Vec::new();
        for query in &search_queries {
            match self.research_tools.web_search(query, 10).await {
                Ok(results) => {
                    tracing::debug!("[EVENTBRITE_SCRAPER] Found {} results for: {}", results.len(), query);
                    all_search_results.extend(results);
                }
                Err(e) => {
                    tracing::warn!("[EVENTBRITE_SCRAPER] Search failed for '{}': {}", query, e);
                }
            }
        }

        if all_search_results.is_empty() {
            tracing::info!("[EVENTBRITE_SCRAPER] No Eventbrite events found via search");
            return Ok(vec![]);
        }

        // Dedupe by URL
        all_search_results.sort_by(|a, b| a.url.cmp(&b.url));
        all_search_results.dedup_by(|a, b| a.url == b.url);

        tracing::info!(
            "[EVENTBRITE_SCRAPER] Found {} unique Eventbrite search results",
            all_search_results.len()
        );

        // Step 2: Use LLM to extract event information
        let system_prompt = r#"You are an expert at extracting event information from search results.

Given search results for Eventbrite events, extract structured event data.

For each event found, return a JSON array with objects containing:
{
  "platform_id": "string (Eventbrite event ID from URL if visible)",
  "name": "string (event title)",
  "description": "string or null (brief description)",
  "event_date": "string or null (YYYY-MM-DD format)",
  "start_time": "string or null (HH:MM format, 24hr)",
  "end_time": "string or null (HH:MM format)",
  "venue_name": "string or null",
  "venue_address": "string or null (city, state)",
  "event_url": "string (full Eventbrite URL)",
  "organizer_name": "string or null",
  "is_free": "boolean or null",
  "price_info": "string or null (e.g., '$25', 'Free', '$50-$100')"
}

Return ONLY a valid JSON array, no markdown formatting. If no events are found, return []."#;

        let search_context: String = all_search_results
            .iter()
            .take(15)
            .map(|r| format!("**{}**\nURL: {}\n{}\n", r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n---\n");

        let user_prompt = format!(
            r#"Extract Eventbrite events from these search results related to {} in {} (dates: {} to {}):

## Search Results
{}

Extract all events and return as JSON array."#,
            conference_name,
            location,
            start_date,
            end_date,
            search_context
        );

        let response = self.research_tools.research_llm(system_prompt, &user_prompt).await
            .map_err(|e| ScraperError::ApiError(format!("LLM analysis failed: {}", e)))?;

        // Step 3: Parse LLM response
        let parsed_events: Vec<EventbriteExtractedEvent> = serde_json::from_str(&response)
            .unwrap_or_else(|e| {
                tracing::warn!("[EVENTBRITE_SCRAPER] Failed to parse LLM response: {}", e);
                vec![]
            });

        // Step 4: Convert to ScrapedEvent with relevance scoring
        let events: Vec<ScrapedEvent> = parsed_events
            .into_iter()
            .map(|event| {
                let mut scraped: ScrapedEvent = event.into();
                let (score, reason) = calculate_relevance(
                    &scraped.name,
                    scraped.description.as_deref(),
                    conference_name,
                );
                scraped.relevance_score = Some(score);
                scraped.relevance_reason = Some(reason);
                scraped
            })
            .filter(|e| e.relevance_score.unwrap_or(0.0) > 0.1)
            .collect();

        tracing::info!(
            "[EVENTBRITE_SCRAPER] Extracted {} relevant events from Eventbrite",
            events.len()
        );

        Ok(events)
    }
}

impl Default for EventbriteScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracted event from LLM analysis
#[derive(Debug, Deserialize)]
struct EventbriteExtractedEvent {
    platform_id: Option<String>,
    name: String,
    description: Option<String>,
    event_date: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    venue_name: Option<String>,
    venue_address: Option<String>,
    event_url: Option<String>,
    organizer_name: Option<String>,
    is_free: Option<bool>,
    price_info: Option<String>,
}

impl From<EventbriteExtractedEvent> for ScrapedEvent {
    fn from(event: EventbriteExtractedEvent) -> Self {
        let mut scraped = ScrapedEvent::new(event.name);

        scraped.platform_id = event.platform_id;
        scraped.description = event.description;
        scraped.event_date = event.event_date;
        scraped.start_time = event.start_time;
        scraped.end_time = event.end_time;
        scraped.venue_name = event.venue_name;
        scraped.venue_address = event.venue_address;
        scraped.event_url = event.event_url.clone();
        scraped.registration_url = event.event_url;
        scraped.organizer_name = event.organizer_name;
        scraped.is_free = event.is_free;
        scraped.price_info = event.price_info;

        scraped
    }
}

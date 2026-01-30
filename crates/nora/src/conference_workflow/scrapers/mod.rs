//! Side Event Scrapers
//!
//! Scrapers for discovering side events from various platforms:
//! - Lu.ma
//! - Eventbrite
//! - Partiful
//!
//! Uses web search (Exa API) + LLM analysis for discovery since these
//! platforms either require authentication or lack public APIs.

pub mod luma;
pub mod eventbrite;
pub mod partiful;

pub use luma::LumaScraper;
pub use eventbrite::EventbriteScraper;
pub use partiful::PartifulScraper;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::execution::research::ResearchTools;

/// Common structure for scraped events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedEvent {
    pub platform_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub event_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub venue_name: Option<String>,
    pub venue_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub event_url: Option<String>,
    pub registration_url: Option<String>,
    pub organizer_name: Option<String>,
    pub organizer_url: Option<String>,
    pub relevance_score: Option<f64>,
    pub relevance_reason: Option<String>,
    pub capacity: Option<i64>,
    pub registered_count: Option<i64>,
    pub is_free: Option<bool>,
    pub price_info: Option<String>,
}

impl ScrapedEvent {
    pub fn new(name: String) -> Self {
        Self {
            platform_id: None,
            name,
            description: None,
            event_date: None,
            start_time: None,
            end_time: None,
            venue_name: None,
            venue_address: None,
            latitude: None,
            longitude: None,
            event_url: None,
            registration_url: None,
            organizer_name: None,
            organizer_url: None,
            relevance_score: None,
            relevance_reason: None,
            capacity: None,
            registered_count: None,
            is_free: None,
            price_info: None,
        }
    }
}

/// Trait for event scrapers
pub trait EventScraper {
    /// Search for events in a location during a date range
    fn search(
        &self,
        location: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        conference_name: &str,
    ) -> impl std::future::Future<Output = Result<Vec<ScrapedEvent>, ScraperError>> + Send;
}

/// Errors that can occur during scraping
#[derive(Debug, thiserror::Error)]
pub enum ScraperError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("API error: {0}")]
    ApiError(String),
}

/// Calculate relevance score based on keywords
pub fn calculate_relevance(
    event_name: &str,
    event_description: Option<&str>,
    conference_name: &str,
) -> (f64, String) {
    let name_lower = event_name.to_lowercase();
    let conf_lower = conference_name.to_lowercase();
    let desc_lower = event_description.map(|d| d.to_lowercase()).unwrap_or_default();

    let mut score: f64 = 0.0;
    let mut reasons = Vec::new();

    // Check for conference name mention
    if name_lower.contains(&conf_lower) || desc_lower.contains(&conf_lower) {
        score += 0.4;
        reasons.push("Conference name mentioned");
    }

    // Check for common side event keywords
    let side_event_keywords = [
        "side event", "afterparty", "after party", "networking", "meetup",
        "happy hour", "breakfast", "dinner", "brunch", "hackathon",
        "workshop", "summit", "panel", "fireside", "mixer",
    ];

    for keyword in &side_event_keywords {
        if name_lower.contains(keyword) || desc_lower.contains(keyword) {
            score += 0.2;
            reasons.push("Side event keyword match");
            break;
        }
    }

    // Check for tech/crypto keywords (common for crypto conferences)
    let tech_keywords = [
        "web3", "crypto", "blockchain", "defi", "nft", "ethereum",
        "bitcoin", "solana", "developer", "builder", "founder",
    ];

    for keyword in &tech_keywords {
        if name_lower.contains(keyword) || desc_lower.contains(keyword) {
            score += 0.2;
            reasons.push("Industry keyword match");
            break;
        }
    }

    // Cap at 1.0
    score = score.min(1.0);

    let reason = if reasons.is_empty() {
        "Location/date match only".to_string()
    } else {
        reasons.join("; ")
    };

    (score, reason)
}

//! Stage 6: Side Events Discovery
//!
//! Discover side events from multiple platforms:
//! - Lu.ma
//! - Eventbrite
//! - Partiful
//! - Meetup

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use db::models::{
    conference_workflow::ConferenceWorkflow,
    side_event::{CreateSideEvent, SideEvent, SideEventPlatform},
};

use crate::{NoraError, Result};

use super::{ResearchStage, ResearchStageResult};
use crate::conference_workflow::scrapers::{
    EventbriteScraper, LumaScraper, PartifulScraper, ScrapedEvent,
};

/// Side Events discovery stage
pub struct SideEventsStage {
    pool: SqlitePool,
    luma_scraper: LumaScraper,
    eventbrite_scraper: EventbriteScraper,
    partiful_scraper: PartifulScraper,
}

impl SideEventsStage {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            luma_scraper: LumaScraper::new(),
            eventbrite_scraper: EventbriteScraper::new(),
            partiful_scraper: PartifulScraper::new(),
        }
    }

    /// Execute side events discovery in parallel across platforms
    pub async fn execute(&self, workflow: &ConferenceWorkflow) -> Result<Vec<SideEvent>> {
        tracing::info!(
            "[SIDE_EVENTS] Discovering side events for: {}",
            workflow.conference_name
        );

        let location = workflow.location.as_deref().unwrap_or("");
        let start_date = NaiveDate::parse_from_str(&workflow.start_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive());
        let end_date = NaiveDate::parse_from_str(&workflow.end_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        // Search all platforms in parallel
        let (luma_result, eventbrite_result, partiful_result) = tokio::join!(
            self.luma_scraper.search(location, start_date, end_date, &workflow.conference_name),
            self.eventbrite_scraper.search(location, start_date, end_date, &workflow.conference_name),
            self.partiful_scraper.search(location, start_date, end_date, &workflow.conference_name),
        );

        let mut all_events = Vec::new();

        // Process Lu.ma results
        if let Ok(events) = luma_result {
            tracing::info!("[SIDE_EVENTS] Found {} events from Lu.ma", events.len());
            for event in events {
                if let Ok(side_event) = self.save_scraped_event(workflow.id, SideEventPlatform::Luma, event).await {
                    all_events.push(side_event);
                }
            }
        }

        // Process Eventbrite results
        if let Ok(events) = eventbrite_result {
            tracing::info!("[SIDE_EVENTS] Found {} events from Eventbrite", events.len());
            for event in events {
                if let Ok(side_event) = self.save_scraped_event(workflow.id, SideEventPlatform::Eventbrite, event).await {
                    all_events.push(side_event);
                }
            }
        }

        // Process Partiful results
        if let Ok(events) = partiful_result {
            tracing::info!("[SIDE_EVENTS] Found {} events from Partiful", events.len());
            for event in events {
                if let Ok(side_event) = self.save_scraped_event(workflow.id, SideEventPlatform::Partiful, event).await {
                    all_events.push(side_event);
                }
            }
        }

        tracing::info!(
            "[SIDE_EVENTS] Total: {} side events discovered",
            all_events.len()
        );

        Ok(all_events)
    }

    /// Save a scraped event to the database
    async fn save_scraped_event(
        &self,
        workflow_id: Uuid,
        platform: SideEventPlatform,
        event: ScrapedEvent,
    ) -> Result<SideEvent> {
        let create = CreateSideEvent {
            conference_workflow_id: workflow_id,
            platform: Some(platform),
            platform_event_id: event.platform_id,
            name: event.name,
            description: event.description,
            event_date: event.event_date,
            start_time: event.start_time,
            end_time: event.end_time,
            venue_name: event.venue_name,
            venue_address: event.venue_address,
            latitude: event.latitude,
            longitude: event.longitude,
            event_url: event.event_url,
            registration_url: event.registration_url,
            organizer_name: event.organizer_name,
            organizer_url: event.organizer_url,
            relevance_score: event.relevance_score,
            relevance_reason: event.relevance_reason,
            capacity: event.capacity,
            registered_count: event.registered_count,
            is_featured: Some(event.relevance_score.map(|s| s > 0.8).unwrap_or(false)),
            requires_registration: Some(true),
            is_free: event.is_free,
            price_info: event.price_info,
        };

        SideEvent::create(&self.pool, &create)
            .await
            .map_err(NoraError::DatabaseError)
    }
}

/// QA checklist for side events
pub fn qa_checklist() -> Vec<&'static str> {
    vec![
        "Events discovered from Lu.ma",
        "Events discovered from Eventbrite",
        "Events discovered from Partiful",
        "Event dates/times captured",
        "Venue information available",
        "Event URLs captured",
    ]
}

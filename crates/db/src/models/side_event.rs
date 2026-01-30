//! Side Event model for tracking events discovered from Lu.ma, Eventbrite, Partiful, etc.
//!
//! Side events are satellite events that happen around a main conference,
//! like networking events, hackathons, parties, and meetups.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Platform where the event was discovered
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "side_event_platform", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SideEventPlatform {
    Luma,
    Eventbrite,
    Partiful,
    Meetup,
    Manual,
    Other,
}

impl std::fmt::Display for SideEventPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SideEventPlatform::Luma => write!(f, "luma"),
            SideEventPlatform::Eventbrite => write!(f, "eventbrite"),
            SideEventPlatform::Partiful => write!(f, "partiful"),
            SideEventPlatform::Meetup => write!(f, "meetup"),
            SideEventPlatform::Manual => write!(f, "manual"),
            SideEventPlatform::Other => write!(f, "other"),
        }
    }
}

/// Status of the side event
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "side_event_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SideEventStatus {
    Discovered,
    Validated,
    Included,
    Excluded,
    Attended,
}

impl Default for SideEventStatus {
    fn default() -> Self {
        Self::Discovered
    }
}

impl std::fmt::Display for SideEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SideEventStatus::Discovered => write!(f, "discovered"),
            SideEventStatus::Validated => write!(f, "validated"),
            SideEventStatus::Included => write!(f, "included"),
            SideEventStatus::Excluded => write!(f, "excluded"),
            SideEventStatus::Attended => write!(f, "attended"),
        }
    }
}

/// Full Side Event record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SideEvent {
    pub id: Uuid,
    pub conference_workflow_id: Uuid,
    pub platform: Option<SideEventPlatform>,
    pub platform_event_id: Option<String>,
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
    pub is_featured: bool,
    pub requires_registration: bool,
    pub is_free: bool,
    pub price_info: Option<String>,
    pub status: SideEventStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new side event
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateSideEvent {
    pub conference_workflow_id: Uuid,
    pub platform: Option<SideEventPlatform>,
    pub platform_event_id: Option<String>,
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
    pub is_featured: Option<bool>,
    pub requires_registration: Option<bool>,
    pub is_free: Option<bool>,
    pub price_info: Option<String>,
}

/// Update a side event
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSideEvent {
    pub description: Option<String>,
    pub event_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub venue_name: Option<String>,
    pub venue_address: Option<String>,
    pub relevance_score: Option<f64>,
    pub relevance_reason: Option<String>,
    pub is_featured: Option<bool>,
    pub status: Option<SideEventStatus>,
}

/// Side event summary for lists
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SideEventBrief {
    pub id: Uuid,
    pub conference_workflow_id: Uuid,
    pub platform: Option<SideEventPlatform>,
    pub name: String,
    pub event_date: Option<String>,
    pub start_time: Option<String>,
    pub venue_name: Option<String>,
    pub relevance_score: Option<f64>,
    pub is_featured: bool,
    pub status: SideEventStatus,
}

impl From<SideEvent> for SideEventBrief {
    fn from(e: SideEvent) -> Self {
        Self {
            id: e.id,
            conference_workflow_id: e.conference_workflow_id,
            platform: e.platform,
            name: e.name,
            event_date: e.event_date,
            start_time: e.start_time,
            venue_name: e.venue_name,
            relevance_score: e.relevance_score,
            is_featured: e.is_featured,
            status: e.status,
        }
    }
}

impl SideEvent {
    /// Find all side events
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            SideEvent,
            r#"SELECT
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM side_events
            ORDER BY event_date ASC, start_time ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find side events by workflow
    pub async fn find_by_workflow(pool: &SqlitePool, workflow_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            SideEvent,
            r#"SELECT
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM side_events
            WHERE conference_workflow_id = $1
            ORDER BY relevance_score DESC NULLS LAST, event_date ASC"#,
            workflow_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find featured side events for a workflow
    pub async fn find_featured(pool: &SqlitePool, workflow_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            SideEvent,
            r#"SELECT
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM side_events
            WHERE conference_workflow_id = $1 AND is_featured = 1
            ORDER BY event_date ASC, start_time ASC"#,
            workflow_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find side event by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            SideEvent,
            r#"SELECT
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM side_events
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find or create by platform event ID
    pub async fn find_or_create_by_platform(
        pool: &SqlitePool,
        workflow_id: Uuid,
        platform: SideEventPlatform,
        platform_event_id: &str,
        name: &str,
    ) -> Result<Self, sqlx::Error> {
        let platform_str = platform.to_string();

        // Try to find existing
        if let Some(existing) = sqlx::query_as!(
            SideEvent,
            r#"SELECT
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM side_events
            WHERE conference_workflow_id = $1 AND platform = $2 AND platform_event_id = $3"#,
            workflow_id,
            platform_str,
            platform_event_id
        )
        .fetch_optional(pool)
        .await? {
            return Ok(existing);
        }

        // Create new
        Self::create(pool, &CreateSideEvent {
            conference_workflow_id: workflow_id,
            platform: Some(platform),
            platform_event_id: Some(platform_event_id.to_string()),
            name: name.to_string(),
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
            is_featured: None,
            requires_registration: None,
            is_free: None,
            price_info: None,
        }).await
    }

    /// Create a new side event
    pub async fn create(pool: &SqlitePool, data: &CreateSideEvent) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let platform_str = data.platform.as_ref().map(|p| p.to_string());
        let status = SideEventStatus::default().to_string();
        let is_featured = data.is_featured.unwrap_or(false) as i32;
        let requires_registration = data.requires_registration.unwrap_or(true) as i32;
        let is_free = data.is_free.unwrap_or(false) as i32;

        sqlx::query_as!(
            SideEvent,
            r#"INSERT INTO side_events (
                id, conference_workflow_id, platform, platform_event_id, name,
                description, event_date, start_time, end_time,
                venue_name, venue_address, latitude, longitude,
                event_url, registration_url, organizer_name, organizer_url,
                relevance_score, relevance_reason,
                capacity, registered_count,
                is_featured, requires_registration, is_free, price_info, status
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26
            )
            RETURNING
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.conference_workflow_id,
            platform_str,
            data.platform_event_id,
            data.name,
            data.description,
            data.event_date,
            data.start_time,
            data.end_time,
            data.venue_name,
            data.venue_address,
            data.latitude,
            data.longitude,
            data.event_url,
            data.registration_url,
            data.organizer_name,
            data.organizer_url,
            data.relevance_score,
            data.relevance_reason,
            data.capacity,
            data.registered_count,
            is_featured,
            requires_registration,
            is_free,
            data.price_info,
            status
        )
        .fetch_one(pool)
        .await
    }

    /// Update a side event
    pub async fn update(pool: &SqlitePool, id: Uuid, data: &UpdateSideEvent) -> Result<Self, sqlx::Error> {
        let status_str = data.status.as_ref().map(|s| s.to_string());
        let is_featured = data.is_featured.map(|b| b as i32);

        sqlx::query_as!(
            SideEvent,
            r#"UPDATE side_events SET
                description = COALESCE($2, description),
                event_date = COALESCE($3, event_date),
                start_time = COALESCE($4, start_time),
                end_time = COALESCE($5, end_time),
                venue_name = COALESCE($6, venue_name),
                venue_address = COALESCE($7, venue_address),
                relevance_score = COALESCE($8, relevance_score),
                relevance_reason = COALESCE($9, relevance_reason),
                is_featured = COALESCE($10, is_featured),
                status = COALESCE($11, status),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                conference_workflow_id as "conference_workflow_id!: Uuid",
                platform as "platform: SideEventPlatform",
                platform_event_id,
                name,
                description,
                event_date,
                start_time,
                end_time,
                venue_name,
                venue_address,
                latitude,
                longitude,
                event_url,
                registration_url,
                organizer_name,
                organizer_url,
                relevance_score,
                relevance_reason,
                capacity,
                registered_count,
                is_featured as "is_featured!: bool",
                requires_registration as "requires_registration!: bool",
                is_free as "is_free!: bool",
                price_info,
                status as "status!: SideEventStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.description,
            data.event_date,
            data.start_time,
            data.end_time,
            data.venue_name,
            data.venue_address,
            data.relevance_score,
            data.relevance_reason,
            is_featured,
            status_str
        )
        .fetch_one(pool)
        .await
    }

    /// Update status
    pub async fn update_status(pool: &SqlitePool, id: Uuid, status: SideEventStatus) -> Result<(), sqlx::Error> {
        let status_str = status.to_string();
        sqlx::query!(
            r#"UPDATE side_events SET
                status = $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            status_str
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a side event
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM side_events WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Count side events by workflow
    pub async fn count_by_workflow(pool: &SqlitePool, workflow_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM side_events WHERE conference_workflow_id = $1",
            workflow_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count as i64)
    }

    /// Count included side events
    pub async fn count_included(pool: &SqlitePool, workflow_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM side_events
            WHERE conference_workflow_id = $1 AND status = 'included'"#,
            workflow_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count as i64)
    }
}

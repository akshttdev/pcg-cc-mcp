use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Error)]
pub enum CalendarEventError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("calendar event not found")]
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CalendarProvider {
    GoogleCalendar,
    OutlookCalendar,
}

impl std::fmt::Display for CalendarProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CalendarProvider::GoogleCalendar => "google_calendar",
            CalendarProvider::OutlookCalendar => "outlook_calendar",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CalendarEvent {
    pub id: DbUuid,
    pub organization_id: DbUuid,
    pub calendar_account_id: DbUuid,
    pub provider: String,
    pub external_event_id: String,
    pub external_calendar_id: Option<String>,
    pub status: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    /// ISO-8601 UTC string
    pub start_at: String,
    /// ISO-8601 UTC string
    pub end_at: String,
    pub all_day: i64,
    pub timezone: Option<String>,
    pub organizer_email: Option<String>,
    /// JSON array `[{ "email": "...", "name": "...", "response_status": "..." }]`
    pub attendees: String,
    pub meeting_url: Option<String>,
    pub crm_activity_id: Option<DbUuid>,
    /// JSON array of `crm_contacts.id`
    pub crm_contact_ids: String,
    pub raw: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertCalendarEvent {
    pub organization_id: DbUuid,
    pub calendar_account_id: DbUuid,
    pub provider: CalendarProvider,
    pub external_event_id: String,
    pub external_calendar_id: Option<String>,
    pub status: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub all_day: bool,
    pub timezone: Option<String>,
    pub organizer_email: Option<String>,
    /// Raw JSON value the caller serialized from provider attendees list
    pub attendees_json: String,
    pub meeting_url: Option<String>,
    pub raw: Option<String>,
}

impl CalendarEvent {
    /// Upsert by `(calendar_account_id, external_event_id)`. CRM-link fields
    /// are preserved on update — they're set separately by the auto-link pass.
    pub async fn upsert(
        pool: &SqlitePool,
        data: UpsertCalendarEvent,
    ) -> Result<Self, CalendarEventError> {
        let id = DbUuid::new();
        let provider = data.provider.to_string();
        let all_day = if data.all_day { 1 } else { 0 };

        let row = sqlx::query_as::<_, CalendarEvent>(
            r#"
            INSERT INTO calendar_events (
                id, organization_id, calendar_account_id, provider,
                external_event_id, external_calendar_id, status,
                title, description, location, start_at, end_at, all_day,
                timezone, organizer_email, attendees, meeting_url, raw
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(calendar_account_id, external_event_id) DO UPDATE SET
                status          = excluded.status,
                title           = excluded.title,
                description     = excluded.description,
                location        = excluded.location,
                start_at        = excluded.start_at,
                end_at          = excluded.end_at,
                all_day         = excluded.all_day,
                timezone        = excluded.timezone,
                organizer_email = excluded.organizer_email,
                attendees       = excluded.attendees,
                meeting_url     = excluded.meeting_url,
                raw             = excluded.raw,
                updated_at      = datetime('now','subsec')
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&data.organization_id)
        .bind(&data.calendar_account_id)
        .bind(&provider)
        .bind(&data.external_event_id)
        .bind(&data.external_calendar_id)
        .bind(&data.status)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.location)
        .bind(&data.start_at)
        .bind(&data.end_at)
        .bind(all_day)
        .bind(&data.timezone)
        .bind(&data.organizer_email)
        .bind(&data.attendees_json)
        .bind(&data.meeting_url)
        .bind(&data.raw)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn list_for_org(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        from: Option<&str>,
        to: Option<&str>,
        limit: i32,
    ) -> Result<Vec<Self>, CalendarEventError> {
        let rows = sqlx::query_as::<_, CalendarEvent>(
            r#"
            SELECT * FROM calendar_events
            WHERE organization_id = ?1
              AND (?2 IS NULL OR start_at >= ?2)
              AND (?3 IS NULL OR start_at <= ?3)
            ORDER BY start_at ASC
            LIMIT ?4
            "#,
        )
        .bind(organization_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_for_contact(
        pool: &SqlitePool,
        crm_contact_id: &DbUuid,
        limit: i32,
    ) -> Result<Vec<Self>, CalendarEventError> {
        // crm_contact_ids is stored as a JSON array of strings; use LIKE to match
        let needle = format!("%\"{}\"%", crm_contact_id.as_str());
        let rows = sqlx::query_as::<_, CalendarEvent>(
            r#"
            SELECT * FROM calendar_events
            WHERE crm_contact_ids LIKE ?1
            ORDER BY start_at DESC
            LIMIT ?2
            "#,
        )
        .bind(needle)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Persist the CRM links produced by `services::calendar::sync::link_to_crm`.
    pub async fn set_crm_links(
        pool: &SqlitePool,
        id: &DbUuid,
        crm_activity_id: Option<&DbUuid>,
        crm_contact_ids_json: &str,
    ) -> Result<(), CalendarEventError> {
        sqlx::query(
            r#"
            UPDATE calendar_events SET
                crm_activity_id = ?2,
                crm_contact_ids = ?3,
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(crm_activity_id)
        .bind(crm_contact_ids_json)
        .execute(pool)
        .await?;
        Ok(())
    }
}

/// Per-account sync cursor + last-sync timestamps. Lives in its own table so
/// we can rotate cursors (e.g., Google's `nextSyncToken` expires after ~30d
/// inactivity) without touching `integration_connections`.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CalendarSyncCursor {
    pub calendar_account_id: DbUuid,
    pub cursor: Option<String>,
    pub last_full_sync_at: Option<DateTime<Utc>>,
    pub last_delta_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl CalendarSyncCursor {
    pub async fn get(
        pool: &SqlitePool,
        account_id: &DbUuid,
    ) -> Result<Option<Self>, CalendarEventError> {
        let row = sqlx::query_as::<_, CalendarSyncCursor>(
            r#"SELECT * FROM calendar_sync_cursor WHERE calendar_account_id = ?1"#,
        )
        .bind(account_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    pub async fn upsert_cursor(
        pool: &SqlitePool,
        account_id: &DbUuid,
        cursor: Option<&str>,
        is_full_sync: bool,
    ) -> Result<(), CalendarEventError> {
        let full_marker = if is_full_sync {
            "datetime('now','subsec')"
        } else {
            "last_full_sync_at"
        };
        // Inline date function in SQL to avoid binding NULL-vs-NOW conditionally.
        let sql = format!(
            r#"
            INSERT INTO calendar_sync_cursor (
                calendar_account_id, cursor, last_full_sync_at, last_delta_sync_at, updated_at
            )
            VALUES (?1, ?2, {full_marker}, datetime('now','subsec'), datetime('now','subsec'))
            ON CONFLICT(calendar_account_id) DO UPDATE SET
                cursor             = excluded.cursor,
                last_full_sync_at  = COALESCE(excluded.last_full_sync_at, last_full_sync_at),
                last_delta_sync_at = excluded.last_delta_sync_at,
                last_error         = NULL,
                updated_at         = datetime('now','subsec')
            "#,
        );
        sqlx::query(&sql)
            .bind(account_id)
            .bind(cursor)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn record_error(
        pool: &SqlitePool,
        account_id: &DbUuid,
        error: &str,
    ) -> Result<(), CalendarEventError> {
        sqlx::query(
            r#"
            INSERT INTO calendar_sync_cursor (calendar_account_id, last_error, updated_at)
            VALUES (?1, ?2, datetime('now','subsec'))
            ON CONFLICT(calendar_account_id) DO UPDATE SET
                last_error = excluded.last_error,
                updated_at = datetime('now','subsec')
            "#,
        )
        .bind(account_id)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }
}

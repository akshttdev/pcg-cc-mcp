//! Pull → normalize → upsert → CRM-link, in one pass per account.
//!
//! Entry: `sync_account(pool, integration_connection_id)`. Reads provider
//! from the row, dispatches to the right connector, normalizes each event,
//! and for any attendee email matching a `crm_contacts.email` (org-scoped)
//! creates a `crm_activities` row of type `meeting_scheduled`.
//!
//! Idempotent: re-running upserts the same `(account_id, external_event_id)`
//! pair, so safe to call repeatedly without duplicating.

use db::{
    db_uuid::DbUuid,
    models::{
        calendar_event::{
            CalendarEvent, CalendarEventError, CalendarProvider, CalendarSyncCursor,
            UpsertCalendarEvent,
        },
        crm_activity::{CreateCrmActivity, CrmActivity, CrmActivityError, CrmActivityType},
        crm_contact::CrmContact,
        integration_connection::IntegrationConnection,
    },
};
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{google, outlook, CalendarError, CalendarSyncStats};
use crate::services::oauth_token_manager;

const PROVIDER_GOOGLE: &str = "google_calendar";
const PROVIDER_OUTLOOK: &str = "outlook_calendar";

/// 15 minutes — matches the spec ("Calendar sync loop every 15 min") and the
/// cadence of `storage::sync_worker`.
const SYNC_INTERVAL_SECS: u64 = 900;

pub async fn sync_account(
    pool: &SqlitePool,
    connection_id: Uuid,
) -> Result<CalendarSyncStats, CalendarError> {
    let connection = IntegrationConnection::find_by_id(pool, connection_id)
        .await?
        .ok_or_else(|| CalendarError::AccountNotFound(connection_id.to_string()))?;

    let account_id = DbUuid::from_string(connection.id.to_string());
    let org_id = DbUuid::from_string(connection.organization_id.to_string());
    let provider = connection.provider.as_str();

    let stats = match provider {
        PROVIDER_GOOGLE => sync_google(pool, connection_id, &account_id, &org_id).await,
        PROVIDER_OUTLOOK => sync_outlook(pool, connection_id, &account_id, &org_id).await,
        other => Err(CalendarError::UnsupportedProvider(other.to_string())),
    };

    if let Err(ref e) = stats {
        let _ = CalendarSyncCursor::record_error(pool, &account_id, &e.to_string()).await;
        warn!(account_id = %account_id, error = %e, "calendar sync failed");
    } else {
        let _ = IntegrationConnection::touch_last_sync(pool, connection_id).await;
    }

    stats
}

// ─── Google ─────────────────────────────────────────────────────────────────

async fn sync_google(
    pool: &SqlitePool,
    connection_id: Uuid,
    account_id: &DbUuid,
    org_id: &DbUuid,
) -> Result<CalendarSyncStats, CalendarError> {
    let access_token = oauth_token_manager::get_access_token(pool, connection_id).await?;
    let client = google::GoogleCalendarClient::new(access_token);

    let prev = CalendarSyncCursor::get(pool, account_id).await?;
    let prev_token = prev.as_ref().and_then(|c| c.cursor.as_deref());
    let pull = google::pull_all(&client, prev_token).await?;

    let mut stats = CalendarSyncStats {
        used_full_sync: pull.used_full_sync,
        ..Default::default()
    };
    stats.events_seen = pull.events.len() as i64;

    for event in &pull.events {
        match handle_google_event(pool, account_id, org_id, event, &mut stats).await {
            Ok(()) => {}
            Err(e) => warn!(error = %e, "skipped Google calendar event"),
        }
    }

    CalendarSyncCursor::upsert_cursor(
        pool,
        account_id,
        pull.next_sync_token.as_deref(),
        pull.used_full_sync,
    )
    .await?;

    info!(
        account_id = %account_id,
        seen = stats.events_seen,
        upserted = stats.events_upserted,
        crm_activities = stats.crm_activities_created,
        full_sync = stats.used_full_sync,
        "Google Calendar sync done"
    );
    Ok(stats)
}

async fn handle_google_event(
    pool: &SqlitePool,
    account_id: &DbUuid,
    org_id: &DbUuid,
    event: &Value,
    stats: &mut CalendarSyncStats,
) -> Result<(), CalendarError> {
    let external_event_id = match event["id"].as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Ok(()),
    };
    let status = event["status"].as_str().unwrap_or("confirmed").to_string();

    if status == "cancelled" {
        // Google emits cancellations as deltas with status=cancelled; we keep
        // the row so downstream queries can show "Cancelled" — counted but not auto-linked.
        let upsert = UpsertCalendarEvent {
            organization_id: org_id.clone(),
            calendar_account_id: account_id.clone(),
            provider: CalendarProvider::GoogleCalendar,
            external_event_id,
            external_calendar_id: None,
            status,
            title: event["summary"]
                .as_str()
                .unwrap_or("(cancelled)")
                .to_string(),
            description: None,
            location: None,
            start_at: extract_google_datetime(&event["start"]).unwrap_or_default(),
            end_at: extract_google_datetime(&event["end"]).unwrap_or_default(),
            all_day: event["start"]["date"].is_string(),
            timezone: event["start"]["timeZone"].as_str().map(|s| s.to_string()),
            organizer_email: event["organizer"]["email"].as_str().map(|s| s.to_string()),
            attendees_json: "[]".into(),
            meeting_url: None,
            raw: Some(event.to_string()),
        };
        CalendarEvent::upsert(pool, upsert).await?;
        stats.events_upserted += 1;
        stats.events_deleted += 1;
        return Ok(());
    }

    let attendees = normalize_google_attendees(&event["attendees"]);
    let attendees_json = serde_json::to_string(&attendees).unwrap_or_else(|_| "[]".into());
    let meeting_url = event["hangoutLink"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| extract_meeting_url(event["description"].as_str().unwrap_or("")));

    let upsert = UpsertCalendarEvent {
        organization_id: org_id.clone(),
        calendar_account_id: account_id.clone(),
        provider: CalendarProvider::GoogleCalendar,
        external_event_id,
        external_calendar_id: None,
        status: status.clone(),
        title: event["summary"]
            .as_str()
            .unwrap_or("(no title)")
            .to_string(),
        description: event["description"].as_str().map(|s| s.to_string()),
        location: event["location"].as_str().map(|s| s.to_string()),
        start_at: extract_google_datetime(&event["start"]).unwrap_or_default(),
        end_at: extract_google_datetime(&event["end"]).unwrap_or_default(),
        all_day: event["start"]["date"].is_string(),
        timezone: event["start"]["timeZone"].as_str().map(|s| s.to_string()),
        organizer_email: event["organizer"]["email"].as_str().map(|s| s.to_string()),
        attendees_json,
        meeting_url,
        raw: Some(event.to_string()),
    };
    let stored = CalendarEvent::upsert(pool, upsert).await?;
    stats.events_upserted += 1;

    let attendee_emails: Vec<String> = attendees
        .iter()
        .filter_map(|a| a["email"].as_str().map(|s| s.to_lowercase()))
        .collect();
    link_event_to_crm(pool, &stored, org_id, &attendee_emails, stats).await?;
    Ok(())
}

fn extract_google_datetime(node: &Value) -> Option<String> {
    node["dateTime"]
        .as_str()
        .or_else(|| node["date"].as_str())
        .map(|s| s.to_string())
}

fn normalize_google_attendees(node: &Value) -> Vec<Value> {
    node.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let email = a["email"].as_str()?;
                    Some(serde_json::json!({
                        "email": email,
                        "name": a["displayName"].as_str(),
                        "response_status": a["responseStatus"].as_str(),
                    }))
                })
                .collect()
        })
        .unwrap_or_default()
}

// ─── Outlook ────────────────────────────────────────────────────────────────

async fn sync_outlook(
    pool: &SqlitePool,
    connection_id: Uuid,
    account_id: &DbUuid,
    org_id: &DbUuid,
) -> Result<CalendarSyncStats, CalendarError> {
    let access_token = oauth_token_manager::get_access_token(pool, connection_id).await?;
    let client = outlook::OutlookCalendarClient::new(access_token);

    let prev = CalendarSyncCursor::get(pool, account_id).await?;
    let prev_link = prev.as_ref().and_then(|c| c.cursor.as_deref());
    let pull = outlook::pull_all(&client, prev_link).await?;

    let mut stats = CalendarSyncStats {
        used_full_sync: pull.used_full_sync,
        ..Default::default()
    };
    stats.events_seen = pull.events.len() as i64;

    for event in &pull.events {
        match handle_outlook_event(pool, account_id, org_id, event, &mut stats).await {
            Ok(()) => {}
            Err(e) => warn!(error = %e, "skipped Outlook calendar event"),
        }
    }

    CalendarSyncCursor::upsert_cursor(
        pool,
        account_id,
        pull.next_delta_link.as_deref(),
        pull.used_full_sync,
    )
    .await?;

    info!(
        account_id = %account_id,
        seen = stats.events_seen,
        upserted = stats.events_upserted,
        crm_activities = stats.crm_activities_created,
        full_sync = stats.used_full_sync,
        "Outlook Calendar sync done"
    );
    Ok(stats)
}

async fn handle_outlook_event(
    pool: &SqlitePool,
    account_id: &DbUuid,
    org_id: &DbUuid,
    event: &Value,
    stats: &mut CalendarSyncStats,
) -> Result<(), CalendarError> {
    // MS Graph delta marks deletions with @removed
    if event.get("@removed").is_some() {
        stats.events_deleted += 1;
        return Ok(());
    }
    let external_event_id = match event["id"].as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Ok(()),
    };

    let attendees = normalize_outlook_attendees(&event["attendees"]);
    let attendees_json = serde_json::to_string(&attendees).unwrap_or_else(|_| "[]".into());

    let online = &event["onlineMeeting"];
    let meeting_url = online["joinUrl"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| event["onlineMeetingUrl"].as_str().map(|s| s.to_string()))
        .or_else(|| extract_meeting_url(event["bodyPreview"].as_str().unwrap_or("")));

    let status_text = if event["isCancelled"].as_bool().unwrap_or(false) {
        "cancelled".to_string()
    } else {
        event["showAs"].as_str().unwrap_or("confirmed").to_string()
    };

    let upsert = UpsertCalendarEvent {
        organization_id: org_id.clone(),
        calendar_account_id: account_id.clone(),
        provider: CalendarProvider::OutlookCalendar,
        external_event_id,
        external_calendar_id: None,
        status: status_text.clone(),
        title: event["subject"]
            .as_str()
            .unwrap_or("(no title)")
            .to_string(),
        description: event["bodyPreview"].as_str().map(|s| s.to_string()),
        location: event["location"]["displayName"]
            .as_str()
            .map(|s| s.to_string()),
        start_at: event["start"]["dateTime"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        end_at: event["end"]["dateTime"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        all_day: event["isAllDay"].as_bool().unwrap_or(false),
        timezone: event["start"]["timeZone"].as_str().map(|s| s.to_string()),
        organizer_email: event["organizer"]["emailAddress"]["address"]
            .as_str()
            .map(|s| s.to_string()),
        attendees_json,
        meeting_url,
        raw: Some(event.to_string()),
    };
    let stored = CalendarEvent::upsert(pool, upsert).await?;
    stats.events_upserted += 1;

    if status_text == "cancelled" {
        return Ok(());
    }

    let attendee_emails: Vec<String> = attendees
        .iter()
        .filter_map(|a| a["email"].as_str().map(|s| s.to_lowercase()))
        .collect();
    link_event_to_crm(pool, &stored, org_id, &attendee_emails, stats).await?;
    Ok(())
}

fn normalize_outlook_attendees(node: &Value) -> Vec<Value> {
    node.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let email = a["emailAddress"]["address"].as_str()?;
                    Some(serde_json::json!({
                        "email": email,
                        "name": a["emailAddress"]["name"].as_str(),
                        "response_status": a["status"]["response"].as_str(),
                    }))
                })
                .collect()
        })
        .unwrap_or_default()
}

// ─── CRM auto-link ──────────────────────────────────────────────────────────

async fn link_event_to_crm(
    pool: &SqlitePool,
    event: &CalendarEvent,
    org_id: &DbUuid,
    attendee_emails: &[String],
    stats: &mut CalendarSyncStats,
) -> Result<(), CalendarError> {
    if attendee_emails.is_empty() {
        return Ok(());
    }

    let mut matched_contact_ids: Vec<String> = Vec::new();
    for email in attendee_emails {
        match CrmContact::find_by_email(pool, org_id, email).await {
            Ok(Some(c)) => matched_contact_ids.push(c.id.as_str().to_string()),
            Ok(None) => {}
            Err(e) => warn!(email = %email, error = %e, "crm contact lookup failed"),
        }
    }
    if matched_contact_ids.is_empty() {
        return Ok(());
    }
    stats.contacts_matched += matched_contact_ids.len() as i64;

    // One activity per event, attached to the first matched contact (we keep
    // the full list on the event so the UI can fan out the relationship).
    let first_id = match DbUuid::parse(&matched_contact_ids[0]) {
        Ok(u) => u.to_uuid(),
        Err(_) => return Ok(()),
    };

    let activity = CrmActivity::create(
        pool,
        CreateCrmActivity {
            organization_id: Some(org_id.to_uuid()),
            client_id: None,
            crm_contact_id: Some(first_id),
            crm_deal_id: None,
            activity_type: CrmActivityType::MeetingScheduled,
            subject: Some(event.title.clone()),
            description: event.description.clone(),
            outcome: None,
            email_message_id: None,
            social_mention_id: None,
            task_id: None,
            performed_by_user: None,
            performed_by_agent_id: None,
            metadata: Some(serde_json::json!({
                "calendar_event_id": event.id.as_str(),
                "calendar_account_id": event.calendar_account_id.as_str(),
                "start_at": event.start_at,
                "end_at": event.end_at,
                "meeting_url": event.meeting_url,
            })),
            duration_minutes: None,
        },
    )
    .await
    .map_err(CalendarError::CrmActivity)?;

    stats.crm_activities_created += 1;

    let activity_id = DbUuid::from_string(activity.id.to_string());
    let contact_ids_json = serde_json::to_string(&matched_contact_ids).unwrap_or("[]".into());
    CalendarEvent::set_crm_links(pool, &event.id, Some(&activity_id), &contact_ids_json).await?;
    Ok(())
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Pull a Zoom/Meet/Teams URL out of free-form description text. Best-effort —
/// returns the first match or None.
fn extract_meeting_url(text: &str) -> Option<String> {
    const HOSTS: &[&str] = &[
        "zoom.us/j/",
        "meet.google.com/",
        "teams.microsoft.com/l/meetup-join/",
        "teams.live.com/meet/",
    ];
    let lower = text.to_lowercase();
    for host in HOSTS {
        if let Some(start) = lower.find(host) {
            let tail = &text[start..];
            let end = tail
                .find(|c: char| c.is_whitespace() || c == '"' || c == '<' || c == '>')
                .unwrap_or(tail.len());
            return Some(format!("https://{}", &tail[..end]));
        }
    }
    None
}

#[allow(dead_code)] // surfaced below for re-export
type _CalendarEventErrorAlias = CalendarEventError;
#[allow(dead_code)]
type _CrmActivityErrorAlias = CrmActivityError;

// ─── Background ticker ──────────────────────────────────────────────────────

/// Spawn a detached Tokio task that calls `sync_account` every 15 minutes
/// for each connected calendar whose `last_sync_at` is null or older than
/// the threshold. Pattern mirrors `storage::sync_worker::spawn_sync_loop`.
pub fn spawn_sync_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        let mut ticker = interval(std::time::Duration::from_secs(SYNC_INTERVAL_SECS));
        ticker.tick().await; // discard immediate fire so the first sync happens at t+15m
        loop {
            ticker.tick().await;
            if let Err(e) = run_due_syncs(&pool).await {
                error!("Calendar sync loop error: {e}");
            }
        }
    });
    info!(
        "Calendar sync worker spawned ({}-min interval)",
        SYNC_INTERVAL_SECS / 60
    );
}

/// Find every active calendar connection with a stale (or missing) `last_sync_at`
/// and run `sync_account` for it. Errors per account are logged and skipped —
/// one bad account never blocks the others.
async fn run_due_syncs(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let due_ids = find_due_calendar_account_ids(pool).await?;
    if due_ids.is_empty() {
        return Ok(());
    }
    info!(count = due_ids.len(), "running due calendar syncs");
    for id in due_ids {
        match sync_account(pool, id).await {
            Ok(stats) => info!(
                account_id = %id,
                upserted = stats.events_upserted,
                "calendar account synced"
            ),
            Err(e) => warn!(account_id = %id, error = %e, "calendar account sync failed"),
        }
    }
    Ok(())
}

/// Scan `integration_connections` for active Google/Outlook calendar rows whose
/// `last_sync_at` is NULL or older than 15 minutes. Returns the connection IDs.
async fn find_due_calendar_account_ids(pool: &SqlitePool) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT id FROM integration_connections
        WHERE provider IN ('google_calendar', 'outlook_calendar')
          AND status = 'active'
          AND (
            last_sync_at IS NULL
            OR datetime(last_sync_at) < datetime('now', '-' || ?1 || ' minutes')
          )
        "#,
    )
    .bind(15_i64)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(id,)| Uuid::parse_str(&id).ok())
        .collect())
}

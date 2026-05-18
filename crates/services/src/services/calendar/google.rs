//! Google Calendar API v3 client.
//!
//! Endpoints:
//!   GET  /calendar/v3/calendars/primary/events
//!   GET  /calendar/v3/calendars/primary/events?syncToken=...
//!
//! Delta sync:
//! 1. First sync: full pull with `singleEvents=true&showDeleted=false`,
//!    paginate via `pageToken`. The terminal page response carries
//!    `nextSyncToken` — store it.
//! 2. Subsequent sync: pass `syncToken=<previous>`. Response includes only
//!    changed/created/cancelled events plus a fresh `nextSyncToken`.
//! 3. If Google returns 410 Gone, the token expired: drop it and full-resync.

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use super::CalendarError;

const API_BASE: &str = "https://www.googleapis.com/calendar/v3";
const DEFAULT_CALENDAR_ID: &str = "primary";

#[derive(Debug, Clone)]
pub struct GoogleCalendarClient {
    http: Client,
    access_token: String,
}

#[derive(Debug, Clone, Default)]
pub struct EventListPage {
    pub events: Vec<Value>,
    pub next_page_token: Option<String>,
    pub next_sync_token: Option<String>,
    /// True if Google returned 410 — caller should drop its sync token and full-resync.
    pub sync_token_expired: bool,
}

impl GoogleCalendarClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            access_token: access_token.into(),
        }
    }

    /// Fetch one page. `cursor` is either a `pageToken` (during full pagination)
    /// or a `syncToken` (delta). They're mutually exclusive in Google's API
    /// — callers pass only one at a time.
    pub async fn list_events(
        &self,
        calendar_id: Option<&str>,
        page_token: Option<&str>,
        sync_token: Option<&str>,
    ) -> Result<EventListPage, CalendarError> {
        let cal = calendar_id.unwrap_or(DEFAULT_CALENDAR_ID);
        let mut url = format!("{API_BASE}/calendars/{cal}/events?singleEvents=true&maxResults=250");
        match (page_token, sync_token) {
            (Some(t), _) => url.push_str(&format!("&pageToken={}", urlencoding::encode(t))),
            (None, Some(t)) => url.push_str(&format!("&syncToken={}", urlencoding::encode(t))),
            (None, None) => {
                // First-ever full pull — restrict to the last 90 days going forward
                // to avoid pulling a decade of history. Subsequent pulls use the
                // sync token regardless of time window.
                let since = chrono::Utc::now() - chrono::Duration::days(90);
                url.push_str(&format!(
                    "&timeMin={}",
                    urlencoding::encode(&since.to_rfc3339())
                ));
            }
        }

        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/json")
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::GONE {
            return Ok(EventListPage {
                sync_token_expired: true,
                ..EventListPage::default()
            });
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CalendarError::Api {
                status: status.as_u16(),
                body,
            });
        }

        #[derive(Deserialize)]
        struct EventsResponse {
            #[serde(default)]
            items: Vec<Value>,
            #[serde(rename = "nextPageToken")]
            next_page_token: Option<String>,
            #[serde(rename = "nextSyncToken")]
            next_sync_token: Option<String>,
        }

        let parsed: EventsResponse = resp.json().await?;
        Ok(EventListPage {
            events: parsed.items,
            next_page_token: parsed.next_page_token,
            next_sync_token: parsed.next_sync_token,
            sync_token_expired: false,
        })
    }
}

/// Pull every event the cursor exposes. Walks `nextPageToken` until exhausted
/// and returns the terminal `nextSyncToken` for the next delta run.
pub struct PullResult {
    pub events: Vec<Value>,
    pub next_sync_token: Option<String>,
    pub used_full_sync: bool,
}

pub async fn pull_all(
    client: &GoogleCalendarClient,
    sync_token: Option<&str>,
) -> Result<PullResult, CalendarError> {
    let mut events: Vec<Value> = Vec::new();
    let mut page_token: Option<String> = None;
    let mut current_sync_token = sync_token.map(|s| s.to_string());
    let mut used_full_sync = sync_token.is_none();
    let next_sync_token;

    loop {
        let page = client
            .list_events(None, page_token.as_deref(), current_sync_token.as_deref())
            .await?;

        if page.sync_token_expired {
            // Drop expired token and restart with a full sync.
            current_sync_token = None;
            page_token = None;
            used_full_sync = true;
            events.clear();
            continue;
        }

        events.extend(page.events);

        if let Some(next) = page.next_page_token {
            // Once paginating with a pageToken, the syncToken is no longer
            // included until the terminal page.
            page_token = Some(next);
            current_sync_token = None;
            continue;
        }

        next_sync_token = page.next_sync_token;
        break;
    }

    Ok(PullResult {
        events,
        next_sync_token,
        used_full_sync,
    })
}

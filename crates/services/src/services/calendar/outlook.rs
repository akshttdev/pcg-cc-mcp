//! Microsoft Graph Calendar client.
//!
//! Endpoints:
//!   GET /v1.0/me/calendarview/delta?startDateTime=...&endDateTime=...
//!
//! Delta sync: Graph returns either `@odata.nextLink` (more pages of the same
//! window) or `@odata.deltaLink` (terminal — store and use as the next sync
//! cursor). Passing the deltaLink back returns only events that changed since.

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::CalendarError;

const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
const DEFAULT_LOOKAHEAD_DAYS: i64 = 90;
const DEFAULT_LOOKBACK_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct OutlookCalendarClient {
    http: Client,
    access_token: String,
}

#[derive(Debug, Default)]
pub struct PullResult {
    pub events: Vec<Value>,
    pub next_delta_link: Option<String>,
    pub used_full_sync: bool,
}

impl OutlookCalendarClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            access_token: access_token.into(),
        }
    }

    /// One HTTP call. Pass either a fresh `delta` URL (from the cursor) or
    /// `None` to start a new full sync window.
    async fn fetch(&self, url: &str) -> Result<DeltaPage, CalendarError> {
        let resp = self
            .http
            .get(url)
            .header("Accept", "application/json")
            .header("Prefer", "outlook.timezone=\"UTC\"")
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CalendarError::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(resp.json::<DeltaPage>().await?)
    }
}

#[derive(Debug, Deserialize)]
struct DeltaPage {
    #[serde(default)]
    value: Vec<Value>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink")]
    delta_link: Option<String>,
}

/// Walk all pages until we get `@odata.deltaLink` (terminal). Returns the
/// concatenated event list and the next sync cursor.
pub async fn pull_all(
    client: &OutlookCalendarClient,
    sync_cursor: Option<&str>,
) -> Result<PullResult, CalendarError> {
    let used_full_sync = sync_cursor.is_none();
    let mut url = match sync_cursor {
        Some(u) => u.to_string(),
        None => initial_delta_url(),
    };

    let mut events: Vec<Value> = Vec::new();
    let next_delta_link;

    loop {
        let page = client.fetch(&url).await?;
        events.extend(page.value);

        if let Some(next) = page.next_link {
            url = next;
            continue;
        }
        next_delta_link = page.delta_link;
        break;
    }

    Ok(PullResult {
        events,
        next_delta_link,
        used_full_sync,
    })
}

fn initial_delta_url() -> String {
    let now = chrono::Utc::now();
    let start = now - chrono::Duration::days(DEFAULT_LOOKBACK_DAYS);
    let end = now + chrono::Duration::days(DEFAULT_LOOKAHEAD_DAYS);
    format!(
        "{GRAPH_BASE}/me/calendarview/delta?startDateTime={}&endDateTime={}",
        urlencoding::encode(&start.to_rfc3339()),
        urlencoding::encode(&end.to_rfc3339()),
    )
}

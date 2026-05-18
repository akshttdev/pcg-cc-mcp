//! Calendar integration: Google Calendar + Outlook (Microsoft Graph).
//!
//! - `google`  — REST client for Calendar API v3 with delta sync via `nextSyncToken`.
//! - `outlook` — REST client for `/me/calendarview/delta` with `@odata.deltaLink`.
//! - `sync`    — entry point that fans the right connector for an account,
//!   normalizes events into `calendar_events`, and links matched
//!   attendees to `crm_contacts` + `crm_activities`.
//!
//! Tokens come from `integration_connections` via `oauth_token_manager`; this
//! module never touches token storage directly.

pub mod google;
pub mod outlook;
pub mod sync;

use thiserror::Error;

use crate::services::oauth_token_manager::TokenManagerError;

#[derive(Debug, Error)]
pub enum CalendarError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    CalendarEvent(#[from] db::models::calendar_event::CalendarEventError),
    #[error(transparent)]
    CrmActivity(#[from] db::models::crm_activity::CrmActivityError),
    #[error(transparent)]
    Token(#[from] TokenManagerError),
    #[error("calendar account not found: {0}")]
    AccountNotFound(String),
    #[error("provider {0} is not a calendar provider")]
    UnsupportedProvider(String),
    #[error("Calendar credentials not configured: {0}")]
    NotConfigured(&'static str),
    #[error("calendar API error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("network error talking to calendar provider: {0}")]
    Network(#[from] reqwest::Error),
    #[error("could not parse calendar response: {0}")]
    Parse(String),
}

/// Stats returned from one sync pass — surfaced to the trigger-sync route
/// and the dashboard widget.
#[derive(Debug, Default, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct CalendarSyncStats {
    pub events_seen: i64,
    pub events_upserted: i64,
    pub events_deleted: i64,
    pub crm_activities_created: i64,
    pub contacts_matched: i64,
    pub used_full_sync: bool,
}

pub use sync::{spawn_sync_loop, sync_account};

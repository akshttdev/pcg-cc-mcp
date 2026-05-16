//! Slack integration: workspace install, channel routing, message dispatch.
//!
//! - `client`     — REST client for chat.postMessage + conversations.list.
//! - `formatter`  — Block Kit builders for the spec's 4 notification templates.
//! - `dispatcher` — orchestrates: load routes → format → post → log.
//!
//! Slack OAuth tokens live in `integration_connections` with provider='slack',
//! managed by the existing `oauth_token_manager`. Per-event channel mappings
//! live in `slack_channel_routing` (one row per org + event_type + channel).
//!
//! All public dispatch functions take a `serde_json::Value` payload + the org
//! id. Whether or not the org has Slack connected is handled internally —
//! callers (deal-stage-change handler, invoice-paid handler, …) don't need
//! to gate on that.

pub mod client;
pub mod dispatcher;
pub mod formatter;

use thiserror::Error;

use crate::services::oauth_token_manager::TokenManagerError;

#[derive(Debug, Error)]
pub enum SlackError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    SlackRoute(#[from] db::models::slack_channel_route::SlackRouteError),
    #[error(transparent)]
    Token(#[from] TokenManagerError),
    #[error("Slack credentials not configured: {0}")]
    NotConfigured(&'static str),
    #[error("organization {0} has no connected Slack workspace")]
    NoWorkspace(String),
    #[error("Slack API error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("network error talking to Slack: {0}")]
    Network(#[from] reqwest::Error),
    #[error("could not parse Slack response: {0}")]
    Parse(String),
}

pub use client::SlackClient;
pub use dispatcher::dispatch_event;
pub use formatter::format_event;

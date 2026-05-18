//! Event → routes → format → post → log.
//!
//! Callers (deal-stage handler, invoice handler, …) just call:
//!
//! ```ignore
//! dispatch_event(pool, &org_id, SlackEventType::DealStageChanged, &payload).await;
//! ```
//!
//! The dispatcher silently no-ops if the org has no Slack workspace connected
//! or no route configured for this event — it never raises a hard error to
//! the caller. The hot path (deal stage move) shouldn't fail on a Slack outage.
//!
//! Every dispatch attempt is logged to `slack_dispatch_log`, success or fail,
//! so operators can audit "did Slack actually receive this?".

use db::{
    db_uuid::DbUuid,
    models::{
        integration_connection::IntegrationConnection,
        slack_channel_route::{record_dispatch, SlackChannelRoute, SlackEventType},
    },
};
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{info, warn};

use super::{client::SlackClient, formatter::format_event, SlackError};
use crate::services::oauth_token_manager;

const PROVIDER_SLACK: &str = "slack";

/// Look up active routes for `(org, event_type)` and post the formatted message
/// to each. Soft-fails: returns `Ok(0)` when the org has no Slack workspace
/// or no route configured. Returns the number of channels we successfully
/// posted to.
pub async fn dispatch_event(
    pool: &SqlitePool,
    organization_id: &DbUuid,
    event_type: SlackEventType,
    payload: &Value,
) -> Result<usize, SlackError> {
    let routes = SlackChannelRoute::find_active(pool, organization_id, event_type).await?;
    if routes.is_empty() {
        return Ok(0);
    }

    // Resolve the org's Slack connection. We need an integration_connections
    // row with provider='slack' to pull the bot token from.
    let connection = match find_slack_connection(pool, organization_id).await? {
        Some(c) => c,
        None => {
            // Routes exist but the workspace was disconnected — log a warning
            // so operators notice their notifications are silently failing.
            warn!(
                org = %organization_id,
                "slack routes configured but no slack workspace connected"
            );
            return Ok(0);
        }
    };

    // Decrypt the token. If this fails (revoked, ciphertext drift, missing key),
    // log a row per configured channel so operators can see the dispatcher
    // *tried* — silently failing here is the worst possible behavior.
    let access_token = match oauth_token_manager::get_access_token(pool, connection.id).await {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("token unavailable: {e}");
            for route in &routes {
                let _ = record_dispatch(
                    pool,
                    organization_id,
                    event_type.as_str(),
                    &route.channel_id,
                    None,
                    false,
                    Some(&msg),
                    None,
                )
                .await;
            }
            warn!(org = %organization_id, error = %e, "slack dispatch aborted: token unavailable");
            return Ok(0);
        }
    };
    let client = SlackClient::new(access_token);
    let (text, blocks) = format_event(event_type, payload);
    let summary = blocks_summary(&blocks);

    let mut delivered = 0;
    for route in routes {
        match client.post_message(&route.channel_id, &text, &blocks).await {
            Ok(resp) => {
                let _ = record_dispatch(
                    pool,
                    organization_id,
                    event_type.as_str(),
                    &route.channel_id,
                    resp.ts.as_deref(),
                    true,
                    None,
                    Some(&summary),
                )
                .await;
                delivered += 1;
                info!(
                    org = %organization_id,
                    channel = %route.channel_id,
                    event = event_type.as_str(),
                    ts = ?resp.ts,
                    "slack message delivered"
                );
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = record_dispatch(
                    pool,
                    organization_id,
                    event_type.as_str(),
                    &route.channel_id,
                    None,
                    false,
                    Some(&msg),
                    Some(&summary),
                )
                .await;
                warn!(
                    org = %organization_id,
                    channel = %route.channel_id,
                    event = event_type.as_str(),
                    error = %msg,
                    "slack message failed"
                );
            }
        }
    }
    Ok(delivered)
}

/// Send a one-off message bypassing the routing table — used by the
/// "Send test notification" button so admins can verify a channel works.
pub async fn dispatch_test(
    pool: &SqlitePool,
    organization_id: &DbUuid,
    channel_id: &str,
    text: &str,
) -> Result<String, SlackError> {
    let connection = find_slack_connection(pool, organization_id)
        .await?
        .ok_or_else(|| SlackError::NoWorkspace(organization_id.as_str().to_string()))?;
    let access_token = oauth_token_manager::get_access_token(pool, connection.id).await?;
    let client = SlackClient::new(access_token);
    let blocks = serde_json::json!([{
        "type": "section",
        "text": { "type": "mrkdwn", "text": text },
    }]);
    let resp = client.post_message(channel_id, text, &blocks).await?;
    let _ = record_dispatch(
        pool,
        organization_id,
        "test",
        channel_id,
        resp.ts.as_deref(),
        true,
        None,
        Some(text),
    )
    .await;
    Ok(resp.ts.unwrap_or_default())
}

/// Find the org's connected Slack workspace. Returns the most-recently
/// connected if more than one exists (rare but possible if a user reconnected).
async fn find_slack_connection(
    pool: &SqlitePool,
    organization_id: &DbUuid,
) -> Result<Option<IntegrationConnection>, SlackError> {
    let org_uuid = uuid::Uuid::parse_str(organization_id.as_str())
        .map_err(|e| SlackError::Parse(format!("invalid org uuid: {e}")))?;
    let mut rows =
        IntegrationConnection::find_by_org_and_provider(pool, org_uuid, PROVIDER_SLACK).await?;
    Ok(rows.pop())
}

/// Best-effort one-line summary of a Block Kit message for the dispatch log.
fn blocks_summary(blocks: &Value) -> String {
    blocks
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|first| first["text"]["text"].as_str())
        .map(|s| {
            s.split('\n')
                .next()
                .unwrap_or(s)
                .chars()
                .take(120)
                .collect::<String>()
        })
        .unwrap_or_default()
}

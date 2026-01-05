use axum::{
    Router,
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
    routing::get,
};
use chrono::{DateTime, Utc};
use db::models::agent_flow_event::AgentFlowEvent;
use deployment::Deployment;
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};
use uuid::Uuid;

use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct EventStreamQuery {
    /// Only return events after this timestamp
    pub since: Option<DateTime<Utc>>,
}

/// Stream agent flow events for a specific flow via SSE
pub async fn stream_flow_events(
    Path(flow_id): Path<Uuid>,
    Query(query): Query<EventStreamQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pool = deployment.db().pool.clone();
    let last_event_time = query.since.unwrap_or_else(|| Utc::now() - chrono::Duration::hours(1));

    let stream = stream::unfold(
        (pool, flow_id, last_event_time),
        |(pool, flow_id, mut since)| async move {
            // Poll every 500ms
            tokio::time::sleep(Duration::from_millis(500)).await;

            match AgentFlowEvent::find_since(&pool, flow_id, since).await {
                Ok(events) if !events.is_empty() => {
                    // Update the since timestamp to the last event
                    if let Some(last) = events.last() {
                        since = last.created_at;
                    }

                    // Serialize events to JSON
                    let json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string());
                    let event = Event::default().data(json).event("flow_events");

                    Some((Ok(event), (pool, flow_id, since)))
                }
                Ok(_) => {
                    // No new events, send keepalive
                    let event = Event::default().comment("keepalive");
                    Some((Ok(event), (pool, flow_id, since)))
                }
                Err(e) => {
                    // Error occurred, log and continue
                    tracing::error!("Error fetching flow events: {}", e);
                    let event = Event::default().comment("error");
                    Some((Ok(event), (pool, flow_id, since)))
                }
            }
        },
    );

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// Stream all recent flow events across all flows
pub async fn stream_all_events(
    Query(query): Query<EventStreamQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pool = deployment.db().pool.clone();
    let last_check = query.since.unwrap_or_else(|| Utc::now() - chrono::Duration::minutes(5));

    let stream = stream::unfold((pool, last_check), |(pool, mut since)| async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        match AgentFlowEvent::find_latest(&pool, 20).await {
            Ok(events) => {
                // Filter to only events newer than our last check
                let new_events: Vec<_> = events
                    .into_iter()
                    .filter(|e| e.created_at > since)
                    .collect();

                if !new_events.is_empty() {
                    // Update since to the newest event
                    if let Some(newest) = new_events.first() {
                        since = newest.created_at;
                    }

                    let json =
                        serde_json::to_string(&new_events).unwrap_or_else(|_| "[]".to_string());
                    let event = Event::default().data(json).event("all_events");
                    Some((Ok(event), (pool, since)))
                } else {
                    let event = Event::default().comment("keepalive");
                    Some((Ok(event), (pool, since)))
                }
            }
            Err(e) => {
                tracing::error!("Error fetching all events: {}", e);
                let event = Event::default().comment("error");
                Some((Ok(event), (pool, since)))
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/events/flows/{flow_id}", get(stream_flow_events))
        .route("/events/all", get(stream_all_events))
}

//! SSE endpoint for real-time pipeline deal changes.
//!
//! Polls MAX(updated_at) from crm_deals every 1s and emits a "pipeline_changed"
//! event when any deal in the org is created, moved, or updated. Frontend subscribes
//! to invalidate kanban queries — no polling needed on the client.

use std::{convert::Infallible, time::Duration};

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use deployment::Deployment;
use futures::stream::{self, Stream};
use serde::Deserialize;

use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct PipelineEventsQuery {
    pub org_id: Option<String>,
}

/// SSE stream that emits "pipeline_changed" whenever deals in the org are modified.
pub async fn stream_pipeline_events(
    Query(query): Query<PipelineEventsQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pool = deployment.db().pool.clone();
    let org_id = query.org_id.unwrap_or_default();

    let stream = stream::unfold(
        (pool, org_id, String::new()),
        |(pool, org_id, mut last_timestamp)| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let current: Option<String> = if org_id.is_empty() {
                sqlx::query_scalar("SELECT MAX(updated_at) FROM crm_deals")
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
            } else {
                sqlx::query_scalar(
                    "SELECT MAX(updated_at) FROM crm_deals WHERE organization_id = ?1",
                )
                .bind(&org_id)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
            };

            let timestamp = current.unwrap_or_default();
            if timestamp != last_timestamp && !last_timestamp.is_empty() {
                last_timestamp = timestamp;
                let event = Event::default()
                    .event("pipeline_changed")
                    .data(serde_json::json!({"org_id": org_id}).to_string());
                Some((Ok(event), (pool, org_id, last_timestamp)))
            } else {
                if last_timestamp.is_empty() {
                    last_timestamp = timestamp;
                }
                // No change — emit comment to keep connection alive
                let event = Event::default().comment("ping");
                Some((Ok(event), (pool, org_id, last_timestamp)))
            }
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/events/pipeline", get(stream_pipeline_events))
}

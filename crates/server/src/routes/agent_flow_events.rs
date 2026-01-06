use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use db::models::agent_flow_event::{
    AgentFlowEvent, CreateFlowEvent, FlowEventPayload, FlowEventType,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct CreateEventPayload {
    pub event_type: FlowEventType,
    pub event_data: FlowEventPayload,
}

#[derive(Debug, Deserialize)]
pub struct ListEventsQuery {
    pub event_type: Option<FlowEventType>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<i32>,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/agent-flows/{flow_id}/events", get(list_events).post(create_event))
        .route("/agent-flow-events/latest", get(list_latest_events))
        .with_state(deployment.clone())
}

async fn list_events(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<ApiResponse<Vec<AgentFlowEvent>>>, ApiError> {
    let pool = &deployment.db().pool;

    let events = if let Some(since) = query.since {
        AgentFlowEvent::find_since(pool, flow_id, since).await?
    } else if let Some(event_type) = query.event_type {
        AgentFlowEvent::find_by_type(pool, flow_id, event_type).await?
    } else {
        AgentFlowEvent::find_by_flow(pool, flow_id).await?
    };

    Ok(Json(ApiResponse::success(events)))
}

async fn create_event(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
    Json(payload): Json<CreateEventPayload>,
) -> Result<(StatusCode, Json<ApiResponse<AgentFlowEvent>>), ApiError> {
    let event = AgentFlowEvent::create(
        &deployment.db().pool,
        CreateFlowEvent {
            agent_flow_id: flow_id,
            event_type: payload.event_type,
            event_data: payload.event_data,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(event))))
}

async fn list_latest_events(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<ApiResponse<Vec<AgentFlowEvent>>>, ApiError> {
    let limit = query.limit.unwrap_or(50);
    let events = AgentFlowEvent::find_latest(&deployment.db().pool, limit).await?;
    Ok(Json(ApiResponse::success(events)))
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i32>,
}

// Helper functions for emitting events from other modules

use db::models::agent_flow_event::AgentFlowEventError;

pub async fn emit_phase_started(
    pool: &sqlx::SqlitePool,
    flow_id: Uuid,
    phase: &str,
    agent_id: Option<Uuid>,
) -> Result<AgentFlowEvent, AgentFlowEventError> {
    AgentFlowEvent::emit_phase_started(pool, flow_id, phase, agent_id).await
}

pub async fn emit_artifact_created(
    pool: &sqlx::SqlitePool,
    flow_id: Uuid,
    artifact_id: Uuid,
    artifact_type: &str,
    title: &str,
    phase: &str,
) -> Result<AgentFlowEvent, AgentFlowEventError> {
    AgentFlowEvent::emit_artifact_created(pool, flow_id, artifact_id, artifact_type, title, phase)
        .await
}

pub async fn emit_subagent_progress(
    pool: &sqlx::SqlitePool,
    flow_id: Uuid,
    session_id: Uuid,
    subagent_id: Uuid,
    subagent_index: i32,
    target_item: &str,
    status: &str,
    result_artifact_id: Option<Uuid>,
) -> Result<AgentFlowEvent, AgentFlowEventError> {
    AgentFlowEvent::emit_subagent_progress(
        pool,
        flow_id,
        session_id,
        subagent_id,
        subagent_index,
        target_item,
        status,
        result_artifact_id,
    )
    .await
}

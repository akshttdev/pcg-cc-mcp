use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use db::models::wide_research::{
    CreateWideResearchSession, ResearchSessionStatus, ResearchTarget,
    WideResearchSession, WideResearchSubagent,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct CreateSessionPayload {
    pub agent_flow_id: Option<Uuid>,
    pub parent_agent_id: Option<Uuid>,
    pub task_description: String,
    pub targets: Vec<TargetPayload>,
    pub parallelism_limit: Option<i32>,
    pub timeout_per_subagent: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct TargetPayload {
    pub target_item: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub agent_flow_id: Option<Uuid>,
    pub status: Option<ResearchSessionStatus>,
}

#[derive(Debug, Deserialize)]
pub struct StartSubagentPayload {
    pub execution_process_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CompleteSubagentPayload {
    pub result_artifact_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct FailSubagentPayload {
    pub error_message: String,
}

#[derive(Debug, Deserialize)]
pub struct SetAggregatedResultPayload {
    pub artifact_id: Uuid,
}

#[derive(Debug, serde::Serialize)]
pub struct SessionWithSubagents {
    #[serde(flatten)]
    pub session: WideResearchSession,
    pub subagents: Vec<WideResearchSubagent>,
    pub progress_percent: f64,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/wide-research", get(list_sessions).post(create_session))
        .route(
            "/wide-research/{session_id}",
            get(get_session).delete(delete_session),
        )
        .route("/wide-research/{session_id}/subagents", get(list_subagents))
        .route(
            "/wide-research/{session_id}/subagents/next",
            get(get_next_pending),
        )
        .route(
            "/wide-research/{session_id}/subagents/{subagent_id}/start",
            post(start_subagent),
        )
        .route(
            "/wide-research/{session_id}/subagents/{subagent_id}/complete",
            post(complete_subagent),
        )
        .route(
            "/wide-research/{session_id}/subagents/{subagent_id}/fail",
            post(fail_subagent),
        )
        .route(
            "/wide-research/{session_id}/status",
            post(update_session_status),
        )
        .route(
            "/wide-research/{session_id}/aggregated-result",
            post(set_aggregated_result),
        )
        .with_state(deployment.clone())
}

async fn list_sessions(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<ApiResponse<Vec<WideResearchSession>>>, ApiError> {
    let pool = &deployment.db().pool;

    let sessions = if let Some(flow_id) = query.agent_flow_id {
        WideResearchSession::find_by_flow(pool, flow_id).await?
    } else {
        // Return recent sessions
        sqlx::query_as::<_, WideResearchSession>(
            r#"SELECT * FROM wide_research_sessions ORDER BY created_at DESC LIMIT 50"#,
        )
        .fetch_all(pool)
        .await?
    };

    Ok(Json(ApiResponse::success(sessions)))
}

async fn create_session(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateSessionPayload>,
) -> Result<(StatusCode, Json<ApiResponse<SessionWithSubagents>>), ApiError> {
    let pool = &deployment.db().pool;

    let targets: Vec<ResearchTarget> = payload
        .targets
        .into_iter()
        .map(|t| ResearchTarget {
            target_item: t.target_item,
            metadata: t.metadata,
        })
        .collect();

    let session = WideResearchSession::create(
        pool,
        CreateWideResearchSession {
            agent_flow_id: payload.agent_flow_id,
            parent_agent_id: payload.parent_agent_id,
            task_description: payload.task_description,
            targets,
            parallelism_limit: payload.parallelism_limit,
            timeout_per_subagent: payload.timeout_per_subagent,
        },
    )
    .await?;

    let subagents = WideResearchSession::get_subagents(pool, session.id).await?;
    let progress = session.progress_percent();

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(SessionWithSubagents {
            session,
            subagents,
            progress_percent: progress,
        })),
    ))
}

async fn get_session(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SessionWithSubagents>>, ApiError> {
    let pool = &deployment.db().pool;

    let session = WideResearchSession::find_by_id(pool, session_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Wide research session not found".into()))?;

    let subagents = WideResearchSession::get_subagents(pool, session_id).await?;
    let progress = session.progress_percent();

    Ok(Json(ApiResponse::success(SessionWithSubagents {
        session,
        subagents,
        progress_percent: progress,
    })))
}

async fn delete_session(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify session exists
    WideResearchSession::find_by_id(pool, session_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Wide research session not found".into()))?;

    sqlx::query(r#"DELETE FROM wide_research_sessions WHERE id = ?1"#)
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

async fn list_subagents(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<WideResearchSubagent>>>, ApiError> {
    let subagents = WideResearchSession::get_subagents(&deployment.db().pool, session_id).await?;
    Ok(Json(ApiResponse::success(subagents)))
}

async fn get_next_pending(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<ApiResponse<Vec<WideResearchSubagent>>>, ApiError> {
    let limit = query.limit.unwrap_or(10);
    let subagents =
        WideResearchSession::get_next_pending_subagents(&deployment.db().pool, session_id, limit)
            .await?;
    Ok(Json(ApiResponse::success(subagents)))
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i32>,
}

async fn start_subagent(
    State(deployment): State<DeploymentImpl>,
    Path((session_id, subagent_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<StartSubagentPayload>,
) -> Result<Json<ApiResponse<WideResearchSubagent>>, ApiError> {
    let pool = &deployment.db().pool;

    // Update session status to in_progress if still spawning
    let session = WideResearchSession::find_by_id(pool, session_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Session not found".into()))?;

    if session.status == ResearchSessionStatus::Spawning {
        WideResearchSession::update_status(pool, session_id, ResearchSessionStatus::InProgress)
            .await?;
    }

    let subagent =
        WideResearchSubagent::start(pool, subagent_id, payload.execution_process_id).await?;
    Ok(Json(ApiResponse::success(subagent)))
}

async fn complete_subagent(
    State(deployment): State<DeploymentImpl>,
    Path((session_id, subagent_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CompleteSubagentPayload>,
) -> Result<Json<ApiResponse<WideResearchSubagent>>, ApiError> {
    let pool = &deployment.db().pool;

    let subagent =
        WideResearchSubagent::complete(pool, subagent_id, payload.result_artifact_id).await?;

    // Increment completed counter
    WideResearchSession::increment_completed(pool, session_id).await?;

    Ok(Json(ApiResponse::success(subagent)))
}

async fn fail_subagent(
    State(deployment): State<DeploymentImpl>,
    Path((session_id, subagent_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<FailSubagentPayload>,
) -> Result<Json<ApiResponse<WideResearchSubagent>>, ApiError> {
    let pool = &deployment.db().pool;

    let subagent = WideResearchSubagent::fail(pool, subagent_id, &payload.error_message).await?;

    // Increment failed counter
    WideResearchSession::increment_failed(pool, session_id).await?;

    Ok(Json(ApiResponse::success(subagent)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusPayload {
    pub status: ResearchSessionStatus,
}

async fn update_session_status(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<UpdateStatusPayload>,
) -> Result<Json<ApiResponse<WideResearchSession>>, ApiError> {
    let session =
        WideResearchSession::update_status(&deployment.db().pool, session_id, payload.status)
            .await?;
    Ok(Json(ApiResponse::success(session)))
}

async fn set_aggregated_result(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<SetAggregatedResultPayload>,
) -> Result<Json<ApiResponse<WideResearchSession>>, ApiError> {
    let session =
        WideResearchSession::set_aggregated_result(&deployment.db().pool, session_id, payload.artifact_id)
            .await?;
    Ok(Json(ApiResponse::success(session)))
}

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use db::models::agent_flow::{
    AgentFlow, AgentPhase, CreateAgentFlow, FlowStatus, FlowType, UpdateAgentFlow,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct CreateFlowPayload {
    pub task_id: Uuid,
    pub flow_type: FlowType,
    pub planner_agent_id: Option<Uuid>,
    pub executor_agent_id: Option<Uuid>,
    pub verifier_agent_id: Option<Uuid>,
    pub flow_config: Option<serde_json::Value>,
    pub human_approval_required: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListFlowsQuery {
    pub task_id: Option<Uuid>,
    pub status: Option<FlowStatus>,
}

#[derive(Debug, Deserialize)]
pub struct TransitionPhasePayload {
    pub phase: AgentPhase,
}

#[derive(Debug, Deserialize)]
pub struct CompleteFlowPayload {
    pub verification_score: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveFlowPayload {
    pub approved_by: String,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/agent-flows", get(list_flows).post(create_flow))
        .route("/agent-flows/awaiting-approval", get(list_awaiting_approval))
        .route(
            "/agent-flows/{flow_id}",
            get(get_flow).patch(update_flow).delete(delete_flow),
        )
        .route("/agent-flows/{flow_id}/transition", post(transition_phase))
        .route("/agent-flows/{flow_id}/complete", post(complete_flow))
        .route("/agent-flows/{flow_id}/request-approval", post(request_approval))
        .route("/agent-flows/{flow_id}/approve", post(approve_flow))
        .with_state(deployment.clone())
}

async fn list_flows(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListFlowsQuery>,
) -> Result<Json<ApiResponse<Vec<AgentFlow>>>, ApiError> {
    let pool = &deployment.db().pool;

    let flows = if let Some(task_id) = query.task_id {
        AgentFlow::find_by_task(pool, task_id).await?
    } else if let Some(status) = query.status {
        AgentFlow::find_by_status(pool, status).await?
    } else {
        // Return recent flows (limit 100)
        sqlx::query_as::<_, AgentFlow>(
            r#"SELECT * FROM agent_flows ORDER BY created_at DESC LIMIT 100"#,
        )
        .fetch_all(pool)
        .await?
    };

    Ok(Json(ApiResponse::success(flows)))
}

async fn list_awaiting_approval(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<AgentFlow>>>, ApiError> {
    let flows = AgentFlow::find_awaiting_approval(&deployment.db().pool).await?;
    Ok(Json(ApiResponse::success(flows)))
}

async fn create_flow(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateFlowPayload>,
) -> Result<(StatusCode, Json<ApiResponse<AgentFlow>>), ApiError> {
    let flow = AgentFlow::create(
        &deployment.db().pool,
        CreateAgentFlow {
            task_id: payload.task_id,
            flow_type: payload.flow_type,
            planner_agent_id: payload.planner_agent_id,
            executor_agent_id: payload.executor_agent_id,
            verifier_agent_id: payload.verifier_agent_id,
            flow_config: payload.flow_config,
            human_approval_required: payload.human_approval_required,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(flow))))
}

async fn get_flow(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
) -> Result<Json<ApiResponse<AgentFlow>>, ApiError> {
    let flow = AgentFlow::find_by_id(&deployment.db().pool, flow_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent flow not found".into()))?;

    Ok(Json(ApiResponse::success(flow)))
}

async fn update_flow(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
    Json(payload): Json<UpdateAgentFlow>,
) -> Result<Json<ApiResponse<AgentFlow>>, ApiError> {
    // Verify flow exists
    AgentFlow::find_by_id(&deployment.db().pool, flow_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent flow not found".into()))?;

    // Build update query dynamically based on provided fields
    let pool = &deployment.db().pool;

    let flow = sqlx::query_as::<_, AgentFlow>(
        r#"
        UPDATE agent_flows
        SET status = COALESCE(?2, status),
            current_phase = COALESCE(?3, current_phase),
            planner_agent_id = COALESCE(?4, planner_agent_id),
            executor_agent_id = COALESCE(?5, executor_agent_id),
            verifier_agent_id = COALESCE(?6, verifier_agent_id),
            handoff_instructions = COALESCE(?7, handoff_instructions),
            verification_score = COALESCE(?8, verification_score),
            approved_by = COALESCE(?9, approved_by),
            updated_at = datetime('now', 'subsec')
        WHERE id = ?1
        RETURNING *
        "#,
    )
    .bind(flow_id)
    .bind(payload.status.map(|s| s.to_string()))
    .bind(payload.current_phase.map(|p| p.to_string()))
    .bind(payload.planner_agent_id)
    .bind(payload.executor_agent_id)
    .bind(payload.verifier_agent_id)
    .bind(&payload.handoff_instructions)
    .bind(payload.verification_score)
    .bind(&payload.approved_by)
    .fetch_one(pool)
    .await?;

    Ok(Json(ApiResponse::success(flow)))
}

async fn delete_flow(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify flow exists
    AgentFlow::find_by_id(pool, flow_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent flow not found".into()))?;

    sqlx::query(r#"DELETE FROM agent_flows WHERE id = ?1"#)
        .bind(flow_id)
        .execute(pool)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

async fn transition_phase(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
    Json(payload): Json<TransitionPhasePayload>,
) -> Result<Json<ApiResponse<AgentFlow>>, ApiError> {
    let flow = AgentFlow::transition_to_phase(&deployment.db().pool, flow_id, payload.phase).await?;
    Ok(Json(ApiResponse::success(flow)))
}

async fn complete_flow(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
    Json(payload): Json<CompleteFlowPayload>,
) -> Result<Json<ApiResponse<AgentFlow>>, ApiError> {
    let flow =
        AgentFlow::complete(&deployment.db().pool, flow_id, payload.verification_score).await?;
    Ok(Json(ApiResponse::success(flow)))
}

async fn request_approval(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
) -> Result<Json<ApiResponse<AgentFlow>>, ApiError> {
    let flow = AgentFlow::request_approval(&deployment.db().pool, flow_id).await?;
    Ok(Json(ApiResponse::success(flow)))
}

async fn approve_flow(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<Uuid>,
    Json(payload): Json<ApproveFlowPayload>,
) -> Result<Json<ApiResponse<AgentFlow>>, ApiError> {
    let flow = AgentFlow::approve(&deployment.db().pool, flow_id, &payload.approved_by).await?;
    Ok(Json(ApiResponse::success(flow)))
}

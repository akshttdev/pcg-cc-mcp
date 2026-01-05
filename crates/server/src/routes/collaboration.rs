//! Human-Agent Collaboration API Routes
//!
//! REST endpoints for pause/resume, context injection, and handoffs.

use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::{
    context_injection::{ContextInjection, InjectionType},
    execution_handoff::ExecutionHandoff,
    execution_pause_history::ExecutionPauseHistory,
};
use deployment::Deployment;
use serde::Deserialize;
use serde_json::Value;
use services::services::execution_control::{
    CollaborationState, ExecutionControlService, PauseRequest, ResumeRequest,
    ReturnControlRequest, TakeoverRequest,
};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ========== Request Types ==========

#[derive(Debug, Deserialize, TS)]
pub struct PauseExecutionRequest {
    pub reason: Option<String>,
    pub initiated_by: String,
    pub initiated_by_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ResumeExecutionRequest {
    pub initiated_by: String,
    pub initiated_by_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct TakeoverExecutionRequest {
    pub human_id: String,
    pub human_name: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ReturnControlToAgentRequest {
    pub human_id: String,
    pub human_name: Option<String>,
    pub to_agent_id: String,
    pub to_agent_name: Option<String>,
    pub context_notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct InjectContextRequest {
    pub injector_id: String,
    pub injector_name: Option<String>,
    pub injection_type: InjectionType,
    pub content: String,
    pub metadata: Option<Value>,
}

// ========== Pause/Resume Endpoints ==========

/// Pause an execution
pub async fn pause_execution(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<PauseExecutionRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionPauseHistory>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let entry = control
        .pause(PauseRequest {
            execution_process_id: execution_id,
            reason: req.reason,
            initiated_by: req.initiated_by,
            initiated_by_name: req.initiated_by_name,
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(entry)))
}

/// Resume an execution
pub async fn resume_execution(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<ResumeExecutionRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionPauseHistory>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let entry = control
        .resume(ResumeRequest {
            execution_process_id: execution_id,
            initiated_by: req.initiated_by,
            initiated_by_name: req.initiated_by_name,
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(entry)))
}

/// Get pause history for an execution
pub async fn get_pause_history(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionPauseHistory>>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let history = control
        .get_pause_history(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(history)))
}

// ========== Takeover Endpoints ==========

/// Human takes over control from agent
pub async fn takeover_execution(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<TakeoverExecutionRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionHandoff>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let handoff = control
        .human_takeover(TakeoverRequest {
            execution_process_id: execution_id,
            human_id: req.human_id,
            human_name: req.human_name,
            reason: req.reason,
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(handoff)))
}

/// Return control from human to agent
pub async fn return_control(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<ReturnControlToAgentRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionHandoff>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let handoff = control
        .return_control(ReturnControlRequest {
            execution_process_id: execution_id,
            human_id: req.human_id,
            human_name: req.human_name,
            to_agent_id: req.to_agent_id,
            to_agent_name: req.to_agent_name,
            context_notes: req.context_notes,
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(handoff)))
}

/// Get handoff history for an execution
pub async fn get_handoffs(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionHandoff>>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let handoffs = control
        .get_handoffs(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(handoffs)))
}

// ========== Context Injection Endpoints ==========

/// Inject context/note into an execution
pub async fn inject_context(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<InjectContextRequest>,
) -> Result<ResponseJson<ApiResponse<ContextInjection>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let injection = control
        .inject_context(
            execution_id,
            req.injector_id,
            req.injector_name,
            req.injection_type,
            req.content,
            req.metadata,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(injection)))
}

/// Get all context injections for an execution
pub async fn get_injections(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ContextInjection>>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let injections = control
        .get_injections(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(injections)))
}

/// Get pending (unacknowledged) injections for an execution
pub async fn get_pending_injections(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ContextInjection>>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let injections = control
        .get_pending_injections(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(injections)))
}

/// Acknowledge a context injection
pub async fn acknowledge_injection(
    Path(injection_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ContextInjection>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let injection = control
        .acknowledge_injection(injection_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(injection)))
}

/// Acknowledge all injections for an execution
pub async fn acknowledge_all_injections(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<u64>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let count = control
        .acknowledge_all_injections(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(count)))
}

// ========== State Endpoint ==========

/// Get full collaboration state for an execution
pub async fn get_collaboration_state(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CollaborationState>>, ApiError> {
    let control = ExecutionControlService::new(deployment.db().clone());

    let state = control
        .get_collaboration_state(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(state)))
}

// ========== Router ==========

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Pause/Resume
        .route("/executions/{execution_id}/pause", post(pause_execution))
        .route("/executions/{execution_id}/resume", post(resume_execution))
        .route("/executions/{execution_id}/pause-history", get(get_pause_history))
        // Takeover
        .route("/executions/{execution_id}/takeover", post(takeover_execution))
        .route("/executions/{execution_id}/return-control", post(return_control))
        .route("/executions/{execution_id}/handoffs", get(get_handoffs))
        // Context Injections
        .route("/executions/{execution_id}/inject", post(inject_context))
        .route("/executions/{execution_id}/injections", get(get_injections))
        .route("/executions/{execution_id}/injections/pending", get(get_pending_injections))
        .route("/executions/{execution_id}/injections/acknowledge-all", post(acknowledge_all_injections))
        .route("/injections/{injection_id}/acknowledge", post(acknowledge_injection))
        // Full State
        .route("/executions/{execution_id}/collaboration", get(get_collaboration_state))
}

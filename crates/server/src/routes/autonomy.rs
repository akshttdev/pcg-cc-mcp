//! Autonomy Modes API Routes
//!
//! REST endpoints for checkpoints, approval gates, and autonomy mode configuration.

use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post, put},
};
use db::models::{
    approval_gate::{ApprovalGate, GateApproval, GateType, PendingGate, SubmitApproval},
    checkpoint_definition::{CheckpointDefinition, CheckpointType, CreateCheckpointDefinition, UpdateCheckpointDefinition},
    execution_checkpoint::{CheckpointStatus, ExecutionCheckpoint, ReviewCheckpoint},
};
use deployment::Deployment;
use serde::Deserialize;
use serde_json::Value;
use services::services::autonomy::{AutonomyMode, AutonomyService, PendingApprovalsSummary};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ========== Request Types ==========

#[derive(Debug, Deserialize, TS)]
pub struct SetAutonomyModeRequest {
    pub mode: AutonomyMode,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCheckpointDefinitionRequest {
    pub project_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub checkpoint_type: CheckpointType,
    pub config: Option<Value>,
    #[serde(default = "default_true")]
    pub requires_approval: bool,
    pub auto_approve_after_minutes: Option<i32>,
    #[serde(default)]
    pub priority: i32,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCheckpointDefinitionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<Value>,
    pub requires_approval: Option<bool>,
    pub auto_approve_after_minutes: Option<Option<i32>>,
    pub priority: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
pub struct TriggerCheckpointRequest {
    pub checkpoint_definition_id: Option<Uuid>,
    pub checkpoint_data: Value,
    pub trigger_reason: Option<String>,
    pub auto_approve_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ReviewCheckpointRequest {
    pub reviewer_id: String,
    pub reviewer_name: Option<String>,
    pub decision: CheckpointStatus,
    pub review_note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateApprovalGateRequest {
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub name: String,
    pub gate_type: GateType,
    pub required_approvers: Vec<String>,
    #[serde(default = "default_one")]
    pub min_approvals: i32,
    pub conditions: Option<Value>,
}

fn default_one() -> i32 {
    1
}

#[derive(Debug, Deserialize, TS)]
pub struct TriggerGateRequest {
    pub approval_gate_id: Uuid,
    pub trigger_context: Option<Value>,
}

// ========== Autonomy Mode Endpoints ==========

/// Get autonomy mode for a task
pub async fn get_task_autonomy_mode(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<AutonomyMode>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let mode = autonomy
        .get_task_autonomy_mode(task_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(mode)))
}

/// Set autonomy mode for a task
pub async fn set_task_autonomy_mode(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<SetAutonomyModeRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    autonomy
        .set_task_autonomy_mode(task_id, req.mode)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(())))
}

// ========== Checkpoint Definition Endpoints ==========

/// Create a checkpoint definition
pub async fn create_checkpoint_definition(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<CreateCheckpointDefinitionRequest>,
) -> Result<ResponseJson<ApiResponse<CheckpointDefinition>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let def = autonomy
        .create_checkpoint_definition(CreateCheckpointDefinition {
            project_id: req.project_id,
            name: req.name,
            description: req.description,
            checkpoint_type: req.checkpoint_type,
            config: req.config,
            requires_approval: req.requires_approval,
            auto_approve_after_minutes: req.auto_approve_after_minutes,
            priority: req.priority,
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(def)))
}

/// Get checkpoint definitions for a project
pub async fn get_checkpoint_definitions(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CheckpointDefinition>>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let defs = autonomy
        .get_checkpoint_definitions(project_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(defs)))
}

/// Update a checkpoint definition
pub async fn update_checkpoint_definition(
    Path(definition_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<UpdateCheckpointDefinitionRequest>,
) -> Result<ResponseJson<ApiResponse<CheckpointDefinition>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let def = autonomy
        .update_checkpoint_definition(
            definition_id,
            UpdateCheckpointDefinition {
                name: req.name,
                description: req.description,
                config: req.config,
                requires_approval: req.requires_approval,
                auto_approve_after_minutes: req.auto_approve_after_minutes,
                priority: req.priority,
                is_active: req.is_active,
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(def)))
}

/// Delete a checkpoint definition
pub async fn delete_checkpoint_definition(
    Path(definition_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    autonomy
        .delete_checkpoint_definition(definition_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(())))
}

// ========== Execution Checkpoint Endpoints ==========

/// Trigger a checkpoint
pub async fn trigger_checkpoint(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<TriggerCheckpointRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionCheckpoint>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let checkpoint = autonomy
        .trigger_checkpoint(
            execution_id,
            req.checkpoint_definition_id,
            req.checkpoint_data,
            req.trigger_reason,
            req.auto_approve_minutes,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(checkpoint)))
}

/// Get checkpoints for an execution
pub async fn get_execution_checkpoints(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionCheckpoint>>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let checkpoints = autonomy
        .get_execution_checkpoints(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(checkpoints)))
}

/// Get pending checkpoints for an execution
pub async fn get_pending_checkpoints(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionCheckpoint>>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let checkpoints = autonomy
        .get_pending_checkpoints(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(checkpoints)))
}

/// Review a checkpoint
pub async fn review_checkpoint(
    Path(checkpoint_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<ReviewCheckpointRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionCheckpoint>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let checkpoint = autonomy
        .review_checkpoint(
            checkpoint_id,
            ReviewCheckpoint {
                reviewer_id: req.reviewer_id,
                reviewer_name: req.reviewer_name,
                decision: req.decision,
                review_note: req.review_note,
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(checkpoint)))
}

/// Skip a checkpoint
pub async fn skip_checkpoint(
    Path(checkpoint_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ExecutionCheckpoint>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let checkpoint = autonomy
        .skip_checkpoint(checkpoint_id, None)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(checkpoint)))
}

// ========== Approval Gate Endpoints ==========

/// Create an approval gate
pub async fn create_approval_gate(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<CreateApprovalGateRequest>,
) -> Result<ResponseJson<ApiResponse<ApprovalGate>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let gate = autonomy
        .create_approval_gate(db::models::approval_gate::CreateApprovalGate {
            project_id: req.project_id,
            task_id: req.task_id,
            name: req.name,
            gate_type: req.gate_type,
            required_approvers: req.required_approvers,
            min_approvals: req.min_approvals,
            conditions: req.conditions,
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(gate)))
}

/// Get approval gates for a project
pub async fn get_project_gates(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ApprovalGate>>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let gates = autonomy
        .get_project_gates(project_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(gates)))
}

/// Delete an approval gate
pub async fn delete_approval_gate(
    Path(gate_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    autonomy
        .delete_approval_gate(gate_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(())))
}

// ========== Pending Gate Endpoints ==========

/// Trigger a gate for an execution
pub async fn trigger_gate(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<TriggerGateRequest>,
) -> Result<ResponseJson<ApiResponse<PendingGate>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let pending = autonomy
        .trigger_gate(req.approval_gate_id, execution_id, req.trigger_context)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(pending)))
}

/// Get pending gates for an execution
pub async fn get_pending_gates(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<PendingGate>>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let gates = autonomy
        .get_pending_gates(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(gates)))
}

/// Submit an approval for a pending gate
pub async fn submit_gate_approval(
    Path(pending_gate_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<SubmitApproval>,
) -> Result<ResponseJson<ApiResponse<GateApproval>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let approval = autonomy
        .submit_gate_approval(pending_gate_id, req)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(approval)))
}

/// Bypass a pending gate
pub async fn bypass_gate(
    Path(pending_gate_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<PendingGate>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let pending = autonomy
        .bypass_gate(pending_gate_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(pending)))
}

// ========== Summary Endpoint ==========

/// Get pending approvals summary
pub async fn get_pending_approvals_summary(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<PendingApprovalsSummary>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let summary = autonomy
        .get_pending_approvals_summary()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(summary)))
}

/// Check if execution can proceed
pub async fn can_execution_proceed(
    Path(execution_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<bool>>, ApiError> {
    let autonomy = AutonomyService::new(deployment.db().clone());

    let can_proceed = autonomy
        .can_execution_proceed(execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(can_proceed)))
}

// ========== Router ==========

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Autonomy mode
        .route("/tasks/{task_id}/autonomy-mode", get(get_task_autonomy_mode))
        .route("/tasks/{task_id}/autonomy-mode", put(set_task_autonomy_mode))
        // Checkpoint definitions
        .route("/autonomy/checkpoint-definitions", post(create_checkpoint_definition))
        .route("/autonomy/checkpoint-definitions/{definition_id}", put(update_checkpoint_definition))
        .route("/autonomy/checkpoint-definitions/{definition_id}", delete(delete_checkpoint_definition))
        .route("/projects/{project_id}/checkpoint-definitions", get(get_checkpoint_definitions))
        // Execution checkpoints
        .route("/executions/{execution_id}/checkpoints", get(get_execution_checkpoints))
        .route("/executions/{execution_id}/checkpoints", post(trigger_checkpoint))
        .route("/executions/{execution_id}/checkpoints/pending", get(get_pending_checkpoints))
        .route("/checkpoints/{checkpoint_id}/review", post(review_checkpoint))
        .route("/checkpoints/{checkpoint_id}/skip", post(skip_checkpoint))
        // Approval gates
        .route("/autonomy/approval-gates", post(create_approval_gate))
        .route("/autonomy/approval-gates/{gate_id}", delete(delete_approval_gate))
        .route("/projects/{project_id}/approval-gates", get(get_project_gates))
        // Pending gates
        .route("/executions/{execution_id}/gates", get(get_pending_gates))
        .route("/executions/{execution_id}/gates", post(trigger_gate))
        .route("/pending-gates/{pending_gate_id}/approve", post(submit_gate_approval))
        .route("/pending-gates/{pending_gate_id}/bypass", post(bypass_gate))
        // Summary
        .route("/autonomy/pending-approvals", get(get_pending_approvals_summary))
        .route("/executions/{execution_id}/can-proceed", get(can_execution_proceed))
}

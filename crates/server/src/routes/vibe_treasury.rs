use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use db::models::vibe_deposit::{
    CreateVibeDeposit, CreateVibeWithdrawal, VibeDeposit, VibeWithdrawal,
};
use db::models::vibe_transaction::{VibeSourceType, VibeTransaction};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectVibeBalance {
    pub project_id: Uuid,
    pub total_deposited: i64,
    pub total_withdrawn: i64,
    pub total_spent: i64,
    pub available_balance: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct RecordDepositRequest {
    pub tx_hash: String,
    pub sender_address: String,
    pub amount_vibe: i64,
    pub block_height: Option<i64>,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Project VIBE balance
        .route(
            "/projects/{project_id}/vibe/balance",
            get(get_project_balance),
        )
        // Deposit management
        .route(
            "/projects/{project_id}/vibe/deposits",
            get(list_deposits).post(record_deposit),
        )
        .route(
            "/projects/{project_id}/vibe/deposits/{deposit_id}/confirm",
            post(confirm_deposit),
        )
        .route(
            "/projects/{project_id}/vibe/deposits/{deposit_id}/credit",
            post(credit_deposit),
        )
        // Withdrawal management
        .route(
            "/projects/{project_id}/vibe/withdrawals",
            get(list_withdrawals).post(request_withdrawal),
        )
        // Transaction history (spending)
        .route(
            "/projects/{project_id}/vibe/transactions",
            get(list_transactions),
        )
        .with_state(deployment.clone())
}

/// GET /api/projects/:project_id/vibe/balance - Get project's VIBE balance
async fn get_project_balance(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<ProjectVibeBalance>>, ApiError> {
    let pool = &deployment.db().pool;

    // Get totals
    let total_deposited = VibeDeposit::total_deposited(pool, project_id).await?;
    let total_withdrawn = VibeWithdrawal::total_withdrawn(pool, project_id).await?;

    // Get total spent from transactions
    let summary = VibeTransaction::sum_by_source(pool, VibeSourceType::Project, project_id, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get spending: {}", e)))?;
    let total_spent = summary.total_vibe;

    let available_balance = total_deposited - total_withdrawn - total_spent;

    Ok(Json(ApiResponse::success(ProjectVibeBalance {
        project_id,
        total_deposited,
        total_withdrawn,
        total_spent,
        available_balance,
    })))
}

/// GET /api/projects/:project_id/vibe/deposits - List deposits for a project
async fn list_deposits(
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<VibeDeposit>>>, ApiError> {
    let deposits = VibeDeposit::list_by_project(&deployment.db().pool, project_id, query.limit)
        .await?;
    Ok(Json(ApiResponse::success(deposits)))
}

/// POST /api/projects/:project_id/vibe/deposits - Record a new deposit
async fn record_deposit(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<RecordDepositRequest>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    // Check if deposit already exists
    if let Some(existing) =
        VibeDeposit::find_by_tx_hash(&deployment.db().pool, &payload.tx_hash).await?
    {
        return Ok(Json(ApiResponse::success(existing)));
    }

    let deposit = VibeDeposit::create(
        &deployment.db().pool,
        CreateVibeDeposit {
            project_id,
            tx_hash: payload.tx_hash,
            sender_address: payload.sender_address,
            amount_vibe: payload.amount_vibe,
            block_height: payload.block_height,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(deposit)))
}

/// POST /api/projects/:project_id/vibe/deposits/:deposit_id/confirm - Mark deposit as confirmed
async fn confirm_deposit(
    Path((project_id, deposit_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    // Verify deposit belongs to project
    let deposit = VibeDeposit::find_by_id(&deployment.db().pool, deposit_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Deposit not found".to_string()))?;

    if deposit.project_id != project_id {
        return Err(ApiError::NotFound("Deposit not found for this project".to_string()));
    }

    let updated = VibeDeposit::mark_confirmed(&deployment.db().pool, deposit_id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// POST /api/projects/:project_id/vibe/deposits/:deposit_id/credit - Mark deposit as credited
async fn credit_deposit(
    Path((project_id, deposit_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    // Verify deposit belongs to project
    let deposit = VibeDeposit::find_by_id(&deployment.db().pool, deposit_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Deposit not found".to_string()))?;

    if deposit.project_id != project_id {
        return Err(ApiError::NotFound("Deposit not found for this project".to_string()));
    }

    let updated = VibeDeposit::mark_credited(&deployment.db().pool, deposit_id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// GET /api/projects/:project_id/vibe/withdrawals - List withdrawals for a project
async fn list_withdrawals(
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<VibeWithdrawal>>>, ApiError> {
    let withdrawals =
        VibeWithdrawal::list_by_project(&deployment.db().pool, project_id, query.limit).await?;
    Ok(Json(ApiResponse::success(withdrawals)))
}

/// POST /api/projects/:project_id/vibe/withdrawals - Request a withdrawal
async fn request_withdrawal(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateVibeWithdrawal>,
) -> Result<Json<ApiResponse<VibeWithdrawal>>, ApiError> {
    // Verify the project_id matches
    if payload.project_id != project_id {
        return Err(ApiError::BadRequest("Project ID mismatch".to_string()));
    }

    // Check available balance
    let pool = &deployment.db().pool;
    let total_deposited = VibeDeposit::total_deposited(pool, project_id).await?;
    let total_withdrawn = VibeWithdrawal::total_withdrawn(pool, project_id).await?;
    let summary = VibeTransaction::sum_by_source(pool, VibeSourceType::Project, project_id, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get spending: {}", e)))?;
    let available = total_deposited - total_withdrawn - summary.total_vibe;

    if payload.amount_vibe > available {
        return Err(ApiError::BadRequest(format!(
            "Insufficient balance. Available: {} VIBE, Requested: {} VIBE",
            available, payload.amount_vibe
        )));
    }

    let withdrawal = VibeWithdrawal::create(&deployment.db().pool, payload).await?;
    Ok(Json(ApiResponse::success(withdrawal)))
}

/// GET /api/projects/:project_id/vibe/transactions - List VIBE spending transactions
async fn list_transactions(
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<VibeTransaction>>>, ApiError> {
    let transactions = VibeTransaction::list_by_source(
        &deployment.db().pool,
        VibeSourceType::Project,
        project_id,
        query.limit,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to get transactions: {}", e)))?;

    Ok(Json(ApiResponse::success(transactions)))
}

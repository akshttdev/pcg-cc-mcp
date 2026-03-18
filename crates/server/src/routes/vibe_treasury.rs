use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use db::models::vibe_deposit::{
    CreateVibeDeposit, CreateVibeWithdrawal, VibeDeposit, VibeWithdrawal,
};
use db::models::vibe_transaction::{VibeSourceType, VibeTransaction};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::aptos::AptosService;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;
use db::db_uuid::DbUuid;

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

#[derive(Debug, Deserialize)]
pub struct VerifyDepositRequest {
    pub project_id: Uuid,
    pub tx_hash: String,
    /// User-provided amount to credit. We verify tx exists and succeeded on-chain.
    pub amount_vibe: i64,
}

#[derive(Debug, Deserialize)]
pub struct FaucetRequest {
    pub project_id: Uuid,
    pub amount_vibe: i64,
    pub note: Option<String>,
}

/// Protected routes (require session auth)
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

/// VIBE network configuration returned to the frontend
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct VibeConfig {
    pub revenue_address: String,
    pub network: String,
    pub vibe_token_address: String,
}

/// Public routes (no session required — faucet checks admin key, verify checks on-chain)
pub fn public_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // On-chain deposit verification (user proves they sent VIBE)
        .route("/vibe/deposit/verify", post(verify_deposit))
        // Admin faucet for seeding project balances (admin key required)
        .route("/vibe/faucet", post(admin_faucet))
        // Network config (revenue wallet address, chain info)
        .route("/vibe/config", get(get_vibe_config))
        .with_state(deployment.clone())
}

/// GET /api/projects/:project_id/vibe/balance - Get project's VIBE balance
async fn get_project_balance(
    Path(project_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<ProjectVibeBalance>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
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
    Path(project_id): Path<String>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<VibeDeposit>>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
    let deposits = VibeDeposit::list_by_project(&deployment.db().pool, project_id, query.limit)
        .await?;
    Ok(Json(ApiResponse::success(deposits)))
}

/// POST /api/projects/:project_id/vibe/deposits - Record a new deposit
async fn record_deposit(
    Path(project_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<RecordDepositRequest>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
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
            payment_method: None,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(deposit)))
}

/// POST /api/projects/:project_id/vibe/deposits/:deposit_id/confirm - Mark deposit as confirmed
async fn confirm_deposit(
    Path((project_id, deposit_id)): Path<(String, String)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
    let deposit_id = DbUuid::parse(&deposit_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
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
    Path((project_id, deposit_id)): Path<(String, String)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
    let deposit_id = DbUuid::parse(&deposit_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
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
    Path(project_id): Path<String>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<VibeWithdrawal>>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
    let withdrawals =
        VibeWithdrawal::list_by_project(&deployment.db().pool, project_id, query.limit).await?;
    Ok(Json(ApiResponse::success(withdrawals)))
}

/// POST /api/projects/:project_id/vibe/withdrawals - Request a withdrawal
async fn request_withdrawal(
    Path(project_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateVibeWithdrawal>,
) -> Result<Json<ApiResponse<VibeWithdrawal>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
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
    Path(project_id): Path<String>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<VibeTransaction>>>, ApiError> {
    let project_id = DbUuid::parse(&project_id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid();
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

/// POST /api/vibe/deposit/verify — user proves they sent VIBE on-chain; gets project credited
async fn verify_deposit(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<VerifyDepositRequest>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    let pool = &deployment.db().pool;

    // Idempotency: if already recorded, return it
    if let Some(existing) = VibeDeposit::find_by_tx_hash(pool, &req.tx_hash).await? {
        return Ok(Json(ApiResponse::success(existing)));
    }

    // Verify on Aptos testnet
    let aptos = AptosService::testnet();
    let tx = aptos
        .get_transaction_by_hash(&req.tx_hash)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to query Aptos: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Transaction not found on Aptos testnet".into()))?;

    if !tx.success {
        return Err(ApiError::BadRequest(
            "Transaction failed on-chain. Only successful transactions can be credited.".into(),
        ));
    }

    // Create deposit record and immediately confirm + credit
    let deposit = VibeDeposit::create(
        pool,
        CreateVibeDeposit {
            project_id: req.project_id,
            tx_hash: req.tx_hash,
            sender_address: tx.sender,
            amount_vibe: req.amount_vibe,
            block_height: None,
            payment_method: None,
        },
    )
    .await?;

    let deposit = VibeDeposit::mark_confirmed(pool, deposit.id).await?;
    let deposit = VibeDeposit::mark_credited(pool, deposit.id).await?;

    tracing::info!(
        "[VIBE] Deposit verified and credited: {} VIBE to project {}",
        deposit.amount_vibe,
        deposit.project_id
    );

    Ok(Json(ApiResponse::success(deposit)))
}

/// GET /api/vibe/config — public endpoint returning network config (revenue wallet, chain info)
async fn get_vibe_config() -> Json<ApiResponse<VibeConfig>> {
    Json(ApiResponse::success(VibeConfig {
        revenue_address: std::env::var("PLATFORM_REVENUE_ADDRESS").unwrap_or_default(),
        network: "aptos_testnet".to_string(),
        vibe_token_address: "0x24cb561c64c32942eb8600d5135f0185c23bcd06cd8cf33422ce2f9b77d65388"
            .to_string(),
    }))
}

/// POST /api/vibe/faucet — admin-only endpoint to seed project balances for testing
async fn admin_faucet(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<FaucetRequest>,
) -> Result<Json<ApiResponse<VibeDeposit>>, ApiError> {
    // Check admin key
    let expected_key = std::env::var("ADMIN_API_KEY").unwrap_or_default();
    let provided_key = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if expected_key.is_empty() || provided_key != expected_key {
        return Err(ApiError::Unauthorized(
            "Admin key required for faucet".into(),
        ));
    }

    let pool = &deployment.db().pool;
    let faucet_tx_hash = format!("faucet-{}-{}", req.project_id, Uuid::new_v4());

    let deposit = VibeDeposit::create(
        pool,
        CreateVibeDeposit {
            project_id: req.project_id,
            tx_hash: faucet_tx_hash,
            sender_address: format!(
                "platform-faucet{}",
                req.note
                    .as_deref()
                    .map(|n| format!(": {}", n))
                    .unwrap_or_default()
            ),
            amount_vibe: req.amount_vibe,
            block_height: None,
            payment_method: None,
        },
    )
    .await?;

    let deposit = VibeDeposit::mark_confirmed(pool, deposit.id).await?;
    let deposit = VibeDeposit::mark_credited(pool, deposit.id).await?;

    tracing::info!(
        "[VIBE] Faucet: credited {} VIBE to project {}",
        deposit.amount_vibe,
        deposit.project_id
    );

    Ok(Json(ApiResponse::success(deposit)))
}

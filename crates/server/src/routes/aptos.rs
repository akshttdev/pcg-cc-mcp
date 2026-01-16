use axum::{
    Router,
    extract::{Path, Query},
    response::Json as ResponseJson,
    routing::{get, post},
};
use serde::Deserialize;
use services::services::aptos::{
    AptosService, AptosBalance, AptosTransaction, FaucetResponse,
    SendTransactionResponse, EstimateGasResponse, VibeBalance, VibeTransferResponse,
};
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/aptos/balance/{address}", get(get_balance))
        .route("/aptos/transactions/{address}", get(get_transactions))
        .route("/aptos/faucet/{address}", post(fund_from_faucet))
        .route("/aptos/exists/{address}", get(check_account_exists))
        .route("/aptos/send", post(send_apt))
        .route("/aptos/estimate-gas/{address}", get(estimate_gas))
        // VIBE token endpoints
        .route("/vibe/balance/{address}", get(get_vibe_balance))
        .route("/vibe/send", post(send_vibe))
}

#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct FaucetQuery {
    pub amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub sender_private_key: String,
    pub sender_address: String,
    pub recipient_address: String,
    pub amount_apt: f64,
}

#[derive(Debug, Deserialize)]
pub struct SendVibeRequest {
    pub sender_private_key: String,
    pub sender_address: String,
    pub recipient_address: String,
    pub amount_vibe: u64,
}

/// GET /api/aptos/balance/:address - Get account balance from Aptos testnet
async fn get_balance(
    Path(address): Path<String>,
) -> Result<ResponseJson<ApiResponse<AptosBalance>>, ApiError> {
    let service = AptosService::testnet();

    let balance = service
        .get_balance(&address)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get balance: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(balance)))
}

/// GET /api/aptos/transactions/:address - Get recent transactions for an account
async fn get_transactions(
    Path(address): Path<String>,
    Query(params): Query<TransactionsQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<AptosTransaction>>>, ApiError> {
    let service = AptosService::testnet();

    let transactions = service
        .get_transactions(&address, params.limit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get transactions: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(transactions)))
}

/// POST /api/aptos/faucet/:address - Fund account from testnet faucet
async fn fund_from_faucet(
    Path(address): Path<String>,
    Query(params): Query<FaucetQuery>,
) -> Result<ResponseJson<ApiResponse<FaucetResponse>>, ApiError> {
    let service = AptosService::testnet();

    let result = service
        .fund_from_faucet(&address, params.amount)
        .await
        .map_err(|e| ApiError::InternalError(format!("Faucet funding failed: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(result)))
}

/// GET /api/aptos/exists/:address - Check if account exists on chain
async fn check_account_exists(
    Path(address): Path<String>,
) -> Result<ResponseJson<ApiResponse<bool>>, ApiError> {
    let service = AptosService::testnet();

    let exists = service
        .account_exists(&address)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check account: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(exists)))
}

/// POST /api/aptos/send - Send APT to another address
async fn send_apt(
    ResponseJson(request): ResponseJson<SendRequest>,
) -> Result<ResponseJson<ApiResponse<SendTransactionResponse>>, ApiError> {
    let service = AptosService::testnet();

    let result = service
        .send_apt(
            &request.sender_private_key,
            &request.sender_address,
            &request.recipient_address,
            request.amount_apt,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Transaction failed: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(result)))
}

/// GET /api/aptos/estimate-gas/:address - Estimate gas for a transfer
async fn estimate_gas(
    Path(address): Path<String>,
) -> Result<ResponseJson<ApiResponse<EstimateGasResponse>>, ApiError> {
    let service = AptosService::testnet();

    let estimate = service
        .estimate_gas(&address)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to estimate gas: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(estimate)))
}

// ========================================
// VIBE Token Endpoints
// ========================================

/// GET /api/vibe/balance/:address - Get VIBE token balance
async fn get_vibe_balance(
    Path(address): Path<String>,
) -> Result<ResponseJson<ApiResponse<VibeBalance>>, ApiError> {
    let service = AptosService::testnet();

    let balance = service
        .get_vibe_balance(&address)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get VIBE balance: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(balance)))
}

/// POST /api/vibe/send - Send VIBE tokens to another address
async fn send_vibe(
    ResponseJson(request): ResponseJson<SendVibeRequest>,
) -> Result<ResponseJson<ApiResponse<VibeTransferResponse>>, ApiError> {
    let service = AptosService::testnet();

    let result = service
        .transfer_vibe(
            &request.sender_private_key,
            &request.sender_address,
            &request.recipient_address,
            request.amount_vibe,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("VIBE transfer failed: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(result)))
}

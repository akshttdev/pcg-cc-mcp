use axum::{
    Json, Router,
    extract::{Path, State},
};
use db::models::agent_wallet::{
    AgentWallet, AgentWalletTransaction, CreateWalletTransaction, UpsertAgentWallet,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Deserialize)]
pub struct TransactionsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    25
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/agent-wallets",
            axum::routing::get(list_wallets).post(upsert_wallet),
        )
        .route(
            "/agent-wallets/{profile_key}",
            axum::routing::put(upsert_wallet_by_key),
        )
        .route(
            "/agent-wallets/{profile_key}/transactions",
            axum::routing::get(list_transactions).post(create_transaction),
        )
        .with_state(deployment.clone())
}

async fn list_wallets(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<AgentWallet>>>, ApiError> {
    let wallets = AgentWallet::list(&deployment.db().pool).await?;
    Ok(Json(ApiResponse::success(wallets)))
}

async fn upsert_wallet(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpsertAgentWallet>,
) -> Result<Json<ApiResponse<AgentWallet>>, ApiError> {
    let wallet = AgentWallet::upsert(&deployment.db().pool, &payload).await?;
    Ok(Json(ApiResponse::success(wallet)))
}

async fn upsert_wallet_by_key(
    Path(profile_key): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(mut payload): Json<UpsertAgentWallet>,
) -> Result<Json<ApiResponse<AgentWallet>>, ApiError> {
    payload.profile_key = profile_key;
    let wallet = AgentWallet::upsert(&deployment.db().pool, &payload).await?;
    Ok(Json(ApiResponse::success(wallet)))
}

async fn list_transactions(
    Path(profile_key): Path<String>,
    State(deployment): State<DeploymentImpl>,
    axum::extract::Query(query): axum::extract::Query<TransactionsQuery>,
) -> Result<Json<ApiResponse<Vec<AgentWalletTransaction>>>, ApiError> {
    let wallet = AgentWallet::find_by_profile_key(&deployment.db().pool, &profile_key)
        .await?
        .ok_or_else(|| ApiError::NotFound("Wallet not found".to_string()))?;

    let transactions =
        AgentWalletTransaction::list_by_wallet(&deployment.db().pool, wallet.id, query.limit)
            .await?;

    Ok(Json(ApiResponse::success(transactions)))
}

async fn create_transaction(
    Path(profile_key): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateWalletTransaction>,
) -> Result<Json<ApiResponse<AgentWalletTransaction>>, ApiError> {
    let wallet = AgentWallet::find_by_profile_key(&deployment.db().pool, &profile_key)
        .await?
        .ok_or_else(|| ApiError::NotFound("Wallet not found".to_string()))?;

    if payload.wallet_id != wallet.id {
        return Err(ApiError::BadRequest(
            "Wallet ID mismatch for provided profile".to_string(),
        ));
    }

    if !matches!(payload.direction.as_str(), "debit" | "credit") {
        return Err(ApiError::BadRequest(
            "Direction must be 'debit' or 'credit'".to_string(),
        ));
    }

    if payload.amount <= 0 {
        return Err(ApiError::BadRequest(
            "Amount must be greater than zero".to_string(),
        ));
    }

    let txn = AgentWalletTransaction::create(&deployment.db().pool, &payload).await?;
    Ok(Json(ApiResponse::success(txn)))
}

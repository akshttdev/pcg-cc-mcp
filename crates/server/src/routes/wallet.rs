//! Wallet import endpoint for linking Aptos wallet addresses to users

use axum::{
    extract::State,
    response::Json as ResponseJson,
    routing::post,
    Router,
};
use db::models::user::User;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{
    error::ApiError,
    middleware::access_control::AccessContext,
    DeploymentImpl,
};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ImportWalletRequest {
    pub wallet_address: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ImportWalletResponse {
    pub message: String,
    pub wallet_address: String,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/wallet/import", post(import_wallet))
}

/// POST /api/wallet/import
/// Link an Aptos wallet address to the current user
async fn import_wallet(
    State(deployment): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    ResponseJson(req): ResponseJson<ImportWalletRequest>,
) -> Result<ResponseJson<ApiResponse<ImportWalletResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Validate Aptos address format (0x followed by 64 hex characters)
    let addr = req.wallet_address.trim();
    if !addr.starts_with("0x") || addr.len() < 10 {
        return Err(ApiError::BadRequest(
            "Invalid Aptos wallet address format. Must start with '0x'.".to_string(),
        ));
    }

    // Check if this wallet is already linked to another user
    let existing: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM users WHERE wallet_address = ? AND id != ?",
    )
    .bind(addr)
    .bind(access_ctx.user_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if existing.is_some() {
        return Err(ApiError::BadRequest(
            "This wallet address is already linked to another account.".to_string(),
        ));
    }

    // Set wallet address on user
    User::set_wallet_address(pool, access_ctx.user_id, addr)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to save wallet address: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(ImportWalletResponse {
        message: "Wallet address linked successfully".to_string(),
        wallet_address: addr.to_string(),
    })))
}

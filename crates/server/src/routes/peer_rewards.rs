use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use db::models::peer_node::PeerNode;
use db::models::peer_reward::{PeerReward, PeerRewardSummary};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PeerBalanceResponse {
    pub node_id: String,
    pub wallet_address: String,
    #[ts(type = "number")]
    pub pending_vibe: i64,
    #[ts(type = "number")]
    pub distributed_vibe: i64,
    #[ts(type = "number")]
    pub confirmed_vibe: i64,
    #[ts(type = "number")]
    pub total_earned: i64,
    pub reward_count: i64,
    #[ts(type = "number")]
    pub pending_usd: f64,
    #[ts(type = "number")]
    pub total_earned_usd: f64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PeerStatsResponse {
    pub node_id: String,
    pub wallet_address: String,
    pub cpu_cores: Option<i64>,
    pub ram_mb: Option<i64>,
    pub storage_gb: Option<i64>,
    pub gpu_available: bool,
    pub gpu_model: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub is_active: bool,
    #[ts(type = "number")]
    pub total_earned: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct NetworkStatsResponse {
    pub total_peers: i64,
    pub active_peers: i64,
    #[ts(type = "number")]
    pub total_rewards_distributed: i64,
    #[ts(type = "number")]
    pub total_pending_rewards: i64,
    pub total_batches: i64,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Peer-specific endpoints
        .route("/peers/:wallet/balance", get(get_peer_balance))
        .route("/peers/:wallet/rewards", get(list_peer_rewards))
        .route("/peers/:wallet/stats", get(get_peer_stats))
        // Network-wide endpoints
        .route("/peers", get(list_all_peers))
        .route("/network/stats", get(get_network_stats))
        .with_state(deployment.clone())
}

/// GET /api/peers/:wallet/balance - Get peer's VIBE balance and earnings
async fn get_peer_balance(
    Path(wallet): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<PeerBalanceResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Find peer by wallet
    let peer = PeerNode::find_by_wallet(pool, &wallet)
        .await?
        .ok_or_else(|| ApiError::NotFound("Peer not found".to_string()))?;

    // Get reward summary
    let summary = PeerReward::get_summary(pool, peer.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get summary: {}", e)))?;

    // Calculate USD values (1 VIBE = $0.01)
    let vibe_price = 0.01;
    let pending_usd = (summary.pending_vibe as f64 / 100_000_000.0) * vibe_price;
    let total_earned_usd = (summary.total_earned as f64 / 100_000_000.0) * vibe_price;

    Ok(Json(ApiResponse::success(PeerBalanceResponse {
        node_id: summary.node_id,
        wallet_address: summary.wallet_address,
        pending_vibe: summary.pending_vibe,
        distributed_vibe: summary.distributed_vibe,
        confirmed_vibe: summary.confirmed_vibe,
        total_earned: summary.total_earned,
        reward_count: summary.reward_count,
        pending_usd,
        total_earned_usd,
    })))
}

/// GET /api/peers/:wallet/rewards - List rewards for a peer
async fn list_peer_rewards(
    Path(wallet): Path<String>,
    Query(query): Query<ListQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<db::models::peer_reward::PeerReward>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Find peer by wallet
    let peer = PeerNode::find_by_wallet(pool, &wallet)
        .await?
        .ok_or_else(|| ApiError::NotFound("Peer not found".to_string()))?;

    // Get rewards
    let rewards = PeerReward::list_by_peer(pool, peer.id, query.limit).await?;

    Ok(Json(ApiResponse::success(rewards)))
}

/// GET /api/peers/:wallet/stats - Get peer node stats
async fn get_peer_stats(
    Path(wallet): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<PeerStatsResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Find peer by wallet
    let peer = PeerNode::find_by_wallet(pool, &wallet)
        .await?
        .ok_or_else(|| ApiError::NotFound("Peer not found".to_string()))?;

    // Get reward summary for total earned
    let summary = PeerReward::get_summary(pool, peer.id).await.ok();
    let total_earned = summary.map(|s| s.total_earned).unwrap_or(0);

    Ok(Json(ApiResponse::success(PeerStatsResponse {
        node_id: peer.node_id,
        wallet_address: peer.wallet_address,
        cpu_cores: peer.cpu_cores,
        ram_mb: peer.ram_mb,
        storage_gb: peer.storage_gb,
        gpu_available: peer.gpu_available,
        gpu_model: peer.gpu_model,
        last_heartbeat_at: peer.last_heartbeat_at.map(|dt| dt.to_rfc3339()),
        is_active: peer.is_active,
        total_earned,
    })))
}

/// GET /api/peers - List all active peers
async fn list_all_peers(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<PeerStatsResponse>>>, ApiError> {
    let pool = &deployment.db().pool;

    let peers = PeerNode::list_active(pool).await?;

    let mut response = Vec::new();
    for peer in peers {
        let summary = PeerReward::get_summary(pool, peer.id).await.ok();
        let total_earned = summary.map(|s| s.total_earned).unwrap_or(0);

        response.push(PeerStatsResponse {
            node_id: peer.node_id,
            wallet_address: peer.wallet_address,
            cpu_cores: peer.cpu_cores,
            ram_mb: peer.ram_mb,
            storage_gb: peer.storage_gb,
            gpu_available: peer.gpu_available,
            gpu_model: peer.gpu_model,
            last_heartbeat_at: peer.last_heartbeat_at.map(|dt| dt.to_rfc3339()),
            is_active: peer.is_active,
            total_earned,
        });
    }

    Ok(Json(ApiResponse::success(response)))
}

/// GET /api/network/stats - Get network-wide reward statistics
async fn get_network_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<NetworkStatsResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Count total and active peers
    let (total_peers,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM peer_nodes",
    )
    .fetch_one(pool)
    .await?;

    let (active_peers,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM peer_nodes WHERE is_active = 1 AND is_banned = 0",
    )
    .fetch_one(pool)
    .await?;

    // Total rewards distributed
    let (total_distributed,): (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(final_amount), 0)
        FROM peer_rewards
        WHERE status IN ('distributed', 'confirmed')
        "#,
    )
    .fetch_one(pool)
    .await?;

    // Total pending rewards
    let total_pending = PeerReward::total_pending_amount(pool).await?;

    // Total batches
    let (total_batches,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reward_batches",
    )
    .fetch_one(pool)
    .await?;

    Ok(Json(ApiResponse::success(NetworkStatsResponse {
        total_peers,
        active_peers,
        total_rewards_distributed: total_distributed,
        total_pending_rewards: total_pending,
        total_batches,
    })))
}

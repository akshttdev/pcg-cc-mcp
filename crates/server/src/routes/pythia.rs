//! Pythia Routes - Proxy endpoints for Pythia Layer 3 intelligence service
//!
//! These routes proxy requests to the Pythia service (localhost:8100) which
//! handles VIBE economics, task routing, and network intelligence.
//!
//! ```text
//! Dashboard frontend  -->  pythia routes  -->  pythia-client  -->  Pythia (localhost:8100)
//! ```
//!
//! When Pythia is not available, endpoints return graceful fallback responses.

use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

fn pythia_client() -> pythia_client::PythiaClient {
    let url = std::env::var("PYTHIA_URL").unwrap_or_else(|_| "http://localhost:8100".to_string());
    pythia_client::PythiaClient::new(&url)
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Health / status
        .route("/pythia/health", get(pythia_health))
        // Task routing
        .route("/pythia/tasks/submit", post(submit_task))
        .route("/pythia/tasks/{id}/status", get(task_status))
        .route("/pythia/tasks", get(all_tasks))
        // Economics
        .route("/pythia/economics/stats", get(economics_stats))
        .route("/pythia/economics/reputation", get(node_reputation))
        // Rewards
        .route("/pythia/rewards/balance", get(wallet_balance))
        .route("/pythia/rewards/rates", get(reward_rates))
        .route("/pythia/rewards/transactions", get(reward_transactions))
        // Network (from Pythia's perspective)
        .route("/pythia/network/nodes", get(network_nodes))
        .route("/pythia/network/stats", get(network_stats))
        // Cost estimation
        .route("/pythia/cost/estimate", get(cost_estimate))
}

// ─── Health ───────────────────────────────────────────────────────────────

async fn pythia_health(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.health_check().await {
        Ok(health) => Ok(Json(ApiResponse::success(serde_json::json!({
            "status": health.status,
            "version": health.version,
            "peer_count": health.peer_count,
            "pythia_connected": true,
        })))),
        Err(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "status": "offline",
            "pythia_connected": false,
            "message": "Pythia service not running at localhost:8100",
        })))),
    }
}

// ─── Task Submission ──────────────────────────────────────────────────────

async fn submit_task(
    State(_deployment): State<DeploymentImpl>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    let req = pythia_client::TaskSubmitRequest {
        agent: body["agent"].as_str().unwrap_or("").to_string(),
        params: body["params"].clone(),
        input: body["input"].as_str().map(String::from),
        prefer_local: body["prefer_local"].as_bool().unwrap_or(true),
        max_vibe_cost: body["max_vibe_cost"].as_f64(),
        submitter_node_id: body["submitter_node_id"].as_str().unwrap_or("").to_string(),
        submitter_wallet: body["submitter_wallet"].as_str().unwrap_or("").to_string(),
    };

    match client.submit_task(req).await {
        Ok(resp) => Ok(Json(ApiResponse::success(serde_json::json!({
            "task_id": resp.task_id,
            "target_node_id": resp.target_node_id,
            "estimated_vibe_cost": resp.estimated_vibe_cost,
            "routing_reason": resp.routing_reason,
            "is_local": resp.is_local,
        })))),
        Err(e) => Err(ApiError::InternalError(format!("Pythia error: {}", e))),
    }
}

async fn task_status(
    State(_deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_task_status(&id).await {
        Ok(status) => Ok(Json(ApiResponse::success(status))),
        Err(e) => Err(ApiError::NotFound(format!("Task not found: {}", e))),
    }
}

async fn all_tasks(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_all_tasks().await {
        Ok(tasks) => Ok(Json(ApiResponse::success(tasks))),
        Err(e) => Err(ApiError::InternalError(format!("Pythia error: {}", e))),
    }
}

// ─── Economics ────────────────────────────────────────────────────────────

async fn economics_stats(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_economics_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(serde_json::json!(stats)))),
        Err(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "pythia_connected": false,
            "message": "Pythia not available",
        })))),
    }
}

#[derive(Deserialize)]
struct WalletQuery {
    wallet: String,
}

async fn node_reputation(
    State(_deployment): State<DeploymentImpl>,
    Query(q): Query<WalletQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_reputation(&q.wallet).await {
        Ok(rep) => Ok(Json(ApiResponse::success(serde_json::json!(rep)))),
        Err(e) => Err(ApiError::NotFound(format!("Reputation not found: {}", e))),
    }
}

// ─── Rewards ──────────────────────────────────────────────────────────────

async fn wallet_balance(
    State(_deployment): State<DeploymentImpl>,
    Query(q): Query<WalletQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_balance(&q.wallet).await {
        Ok(bal) => Ok(Json(ApiResponse::success(serde_json::json!(bal)))),
        Err(e) => Err(ApiError::NotFound(format!("Wallet not found: {}", e))),
    }
}

async fn reward_rates(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_reward_rates().await {
        Ok(rates) => Ok(Json(ApiResponse::success(rates))),
        Err(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "pythia_connected": false,
        })))),
    }
}

#[derive(Deserialize)]
struct TransactionQuery {
    wallet: Option<String>,
    limit: Option<usize>,
}

async fn reward_transactions(
    State(_deployment): State<DeploymentImpl>,
    Query(q): Query<TransactionQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_transactions(q.wallet.as_deref(), q.limit).await {
        Ok(txs) => Ok(Json(ApiResponse::success(serde_json::json!(txs)))),
        Err(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "count": 0,
            "transactions": [],
            "pythia_connected": false,
        })))),
    }
}

// ─── Network ──────────────────────────────────────────────────────────────

async fn network_nodes(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_network_nodes().await {
        Ok(nodes) => Ok(Json(ApiResponse::success(serde_json::json!(nodes)))),
        Err(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "node_count": 0,
            "nodes": [],
            "pythia_connected": false,
        })))),
    }
}

async fn network_stats(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.get_network_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "active_nodes": 0,
            "pythia_connected": false,
        })))),
    }
}

// ─── Cost Estimation ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CostQuery {
    agent: String,
    node_id: String,
}

async fn cost_estimate(
    State(_deployment): State<DeploymentImpl>,
    Query(q): Query<CostQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let client = pythia_client();

    match client.estimate_cost(&q.agent, &q.node_id).await {
        Ok(est) => Ok(Json(ApiResponse::success(serde_json::json!(est)))),
        Err(e) => Err(ApiError::InternalError(format!("Pythia error: {}", e))),
    }
}

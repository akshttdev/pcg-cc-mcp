//! Mesh Network Routes - API endpoints for APN mesh network stats and operations
//!
//! Provides real-time mesh network statistics, peer information, and
//! transaction logs for the Alpha Protocol Network integration.
//!
//! ## Architecture Change (v3.0)
//!
//! These routes now consume APN Core's API via the `apn-client` crate
//! instead of parsing log files. APN Core (Layer 0) is the single source
//! of truth for network state.
//!
//! ```text
//! Dashboard mesh routes  -->  apn-client  -->  APN Core (localhost:8000)
//! ```

use axum::{
    Router,
    extract::State,
    response::Json as ResponseJson,
    routing::get,
};
use chrono::{DateTime, Utc};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Mesh network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshStats {
    pub node_id: String,
    pub status: String,
    pub peers_connected: usize,
    pub peers: Vec<PeerInfo>,
    pub bandwidth: BandwidthStats,
    pub resources: ResourceStats,
    pub relay_connected: bool,
    pub uptime: u64,
    pub vibe_balance: f64,
    pub transactions: Vec<TransactionLog>,
    pub active_tasks: u32,
    pub completed_tasks_today: u32,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub latency_ms: Option<u64>,
    pub connection_type: String,
    pub bandwidth_mbps: Option<f64>,
    pub reputation: f64,
    pub capabilities: Vec<String>,
}

/// Bandwidth statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthStats {
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_rate: f64,
    pub download_rate: f64,
}

/// Resource statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStats {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub disk_percent: f64,
    pub available_compute: f64,
}

/// Transaction log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub tx_type: String,
    pub description: String,
    pub vibe_amount: Option<f64>,
    pub peer_node: Option<String>,
    pub task_id: Option<Uuid>,
}

/// Fetch peers from APN Core API (replaces log file parsing)
async fn fetch_peers_from_apn_core() -> (Vec<PeerInfo>, bool, String, u64) {
    let client = apn_client::ApnClient::new();

    // Try to get identity and peers from APN Core
    let node_id = match client.get_identity().await {
        Ok(identity) => identity.node_id,
        Err(_) => {
            // Fall back to hostname if APN Core not available
            let hostname = hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            format!("omega-{}", &hostname[..hostname.len().min(8)])
        }
    };

    let (peers, relay_connected, uptime) = match client.get_network_stats().await {
        Ok(stats) => {
            let relay = stats.status == "online";
            let uptime = stats.uptime_seconds;

            let peers = match client.get_peers().await {
                Ok(peer_list) => {
                    peer_list.peers.into_iter().map(|p| PeerInfo {
                        peer_id: p.node_id,
                        address: p.wallet_address,
                        latency_ms: None,
                        connection_type: p.connection_type,
                        bandwidth_mbps: None,
                        reputation: 1.0,
                        capabilities: p.capabilities,
                    }).collect()
                }
                Err(_) => vec![],
            };

            (peers, relay, uptime)
        }
        Err(_) => {
            // APN Core not available - fall back to log parsing for backwards compat
            let (legacy_peers, legacy_relay) = fetch_peers_from_log_fallback().await;
            (legacy_peers, legacy_relay, 0)
        }
    };

    (peers, relay_connected, node_id, uptime)
}

/// Legacy fallback: parse peers from log file (used when APN Core is not running)
async fn fetch_peers_from_log_fallback() -> (Vec<PeerInfo>, bool) {
    use std::io::{BufRead, BufReader};
    use std::fs::File;
    use regex::Regex;

    let log_path = "/tmp/apn_node.log";
    let mut peers = std::collections::HashMap::new();
    let mut relay_connected = false;

    if let Ok(file) = File::open(log_path) {
        let reader = BufReader::new(file);

        let peer_regex = Regex::new(
            r#"Message from apn\.discovery \(([^)]+)\): PeerAnnouncement \{ wallet_address: "([^"]+)", capabilities: \[([^\]]+)\], resources: (.+?) \}"#
        ).unwrap();

        let relay_regex = Regex::new(r"Relay connected").unwrap();

        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        for line in lines.iter().rev().take(1000) {
            if relay_regex.is_match(line) {
                relay_connected = true;
            }

            if let Some(caps) = peer_regex.captures(line) {
                let node_id = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let wallet = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let caps_str = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                let capabilities: Vec<String> = caps_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect();

                if !peers.contains_key(node_id) {
                    peers.insert(node_id.to_string(), PeerInfo {
                        peer_id: node_id.to_string(),
                        address: wallet.to_string(),
                        latency_ms: None,
                        connection_type: "NATS".to_string(),
                        bandwidth_mbps: None,
                        reputation: 1.0,
                        capabilities,
                    });
                }
            }
        }
    }

    (peers.into_values().collect(), relay_connected)
}

/// Get mesh network statistics
pub async fn get_mesh_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MeshStats>>, ApiError> {
    // Get peers and identity from APN Core
    let (peers, relay_connected, node_id, uptime) = fetch_peers_from_apn_core().await;

    // Get real system metrics
    let sys_info = sysinfo::System::new_all();

    let cpu_percent = sys_info.global_cpu_usage() as f64;
    let memory_percent = if sys_info.total_memory() > 0 {
        (sys_info.used_memory() as f64 / sys_info.total_memory() as f64) * 100.0
    } else {
        0.0
    };

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk_percent = if let Some(disk) = disks.list().first() {
        let total = disk.total_space();
        let available = disk.available_space();
        if total > 0 {
            ((total - available) as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Get transaction logs from recent activity
    let pool = &deployment.db().pool;
    let recent_flows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        r#"
        SELECT
            af.id,
            af.flow_type,
            af.status,
            af.planning_started_at
        FROM agent_flows af
        ORDER BY af.planning_started_at DESC
        LIMIT 20
        "#
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let transactions: Vec<TransactionLog> = recent_flows
        .into_iter()
        .map(|(id, flow_type, status, timestamp_opt)| {
            let tx_type = match status.as_str() {
                "completed" => "execution_completed",
                "failed" => "execution_failed",
                "planning" | "executing" => "task_received",
                _ => "task_distributed",
            };

            let timestamp = timestamp_opt
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            TransactionLog {
                id: id.clone(),
                timestamp,
                tx_type: tx_type.to_string(),
                description: format!("{} workflow: {}",
                    if status == "completed" { "Completed" }
                    else if status == "failed" { "Failed" }
                    else { "Processing" },
                    flow_type
                ),
                vibe_amount: Some(if status == "completed" { 10.0 } else { 0.0 }),
                peer_node: Some("local".to_string()),
                task_id: Uuid::parse_str(&id).ok(),
            }
        })
        .collect();

    let active_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM agent_flows WHERE status IN ('planning', 'executing')"
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    let completed_today: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM agent_flows
        WHERE status = 'completed'
        AND date(verification_completed_at) = date('now')
        "#
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    let vibe_balance = completed_today.0 as f64 * 10.0;

    let stats = MeshStats {
        node_id,
        status: if relay_connected { "online".to_string() } else { "offline".to_string() },
        peers_connected: peers.len(),
        peers,
        bandwidth: BandwidthStats {
            upload_bytes: 0,
            download_bytes: 0,
            upload_rate: 0.0,
            download_rate: 0.0,
        },
        resources: ResourceStats {
            cpu_percent,
            memory_percent,
            disk_percent,
            available_compute: 100.0 - cpu_percent,
        },
        relay_connected,
        uptime,
        vibe_balance,
        transactions,
        active_tasks: active_count.0 as u32,
        completed_tasks_today: completed_today.0 as u32,
    };

    Ok(ResponseJson(ApiResponse::success(stats)))
}

/// Get mesh peers (now from APN Core)
pub async fn get_mesh_peers(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<PeerInfo>>>, ApiError> {
    let (peers, _, _, _) = fetch_peers_from_apn_core().await;
    Ok(ResponseJson(ApiResponse::success(peers)))
}

/// Get APN Core identity (proxy endpoint for frontend)
pub async fn get_apn_identity(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_identity().await {
        Ok(identity) => {
            let value = serde_json::json!({
                "node_id": identity.node_id,
                "wallet_address": identity.wallet_address,
                "public_key": identity.public_key,
                "apn_core_connected": true,
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(_) => {
            let value = serde_json::json!({
                "node_id": null,
                "wallet_address": null,
                "public_key": null,
                "apn_core_connected": false,
                "message": "APN Core not running at localhost:8000",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get transaction history
pub async fn get_transactions(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TransactionLog>>>, ApiError> {
    let pool = &deployment.db().pool;

    let flows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        r#"
        SELECT
            af.id,
            af.flow_type,
            af.status,
            af.planning_started_at
        FROM agent_flows af
        ORDER BY af.planning_started_at DESC
        LIMIT 50
        "#
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let transactions: Vec<TransactionLog> = flows
        .into_iter()
        .map(|(id, flow_type, status, timestamp_opt)| {
            let tx_type = match status.as_str() {
                "completed" => "execution_completed",
                "failed" => "execution_failed",
                "planning" | "executing" => "task_received",
                _ => "task_distributed",
            };

            let timestamp = timestamp_opt
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            TransactionLog {
                id: id.clone(),
                timestamp,
                tx_type: tx_type.to_string(),
                description: format!("{} workflow: {}",
                    if status == "completed" { "Completed" }
                    else if status == "failed" { "Failed" }
                    else { "Processing" },
                    flow_type
                ),
                vibe_amount: Some(if status == "completed" { 10.0 } else { 0.0 }),
                peer_node: Some("local".to_string()),
                task_id: Uuid::parse_str(&id).ok(),
            }
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(transactions)))
}

// ============= File Transfer Proxy Routes =============

/// Send a file to a peer node (proxies to APN Core)
pub async fn send_file(
    State(_deployment): State<DeploymentImpl>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    let target = body["target_node_id"].as_str().unwrap_or("");
    let path = body["file_path"].as_str().unwrap_or("");

    match client.send_file(target, path).await {
        Ok(resp) => {
            let value = serde_json::to_value(resp.transfer).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(e) => {
            let value = serde_json::json!({
                "error": format!("{}", e),
                "service_status": "error",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get active file transfers
pub async fn get_active_transfers(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_active_transfers().await {
        Ok(resp) => {
            let value = serde_json::to_value(resp).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(_) => {
            let value = serde_json::json!({
                "active": [],
                "service_status": "not_running",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get a specific transfer status
pub async fn get_transfer_status(
    State(_deployment): State<DeploymentImpl>,
    axum::extract::Path(transfer_id): axum::extract::Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_transfer(&transfer_id).await {
        Ok(resp) => {
            let value = serde_json::to_value(resp.transfer).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(e) => {
            let value = serde_json::json!({
                "error": format!("{}", e),
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get file transfer history
pub async fn get_file_transfer_history(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_transfer_history(50).await {
        Ok(resp) => {
            let value = serde_json::to_value(resp).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(_) => {
            let value = serde_json::json!({
                "history": [],
                "service_status": "not_running",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Accept a pending transfer
pub async fn accept_file_transfer(
    State(_deployment): State<DeploymentImpl>,
    axum::extract::Path(transfer_id): axum::extract::Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.accept_transfer(&transfer_id).await {
        Ok(resp) => Ok(ResponseJson(ApiResponse::success(resp))),
        Err(e) => {
            let value = serde_json::json!({ "error": format!("{}", e) });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Cancel an active transfer
pub async fn cancel_file_transfer(
    State(_deployment): State<DeploymentImpl>,
    axum::extract::Path(transfer_id): axum::extract::Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.cancel_transfer(&transfer_id).await {
        Ok(resp) => Ok(ResponseJson(ApiResponse::success(resp))),
        Err(e) => {
            let value = serde_json::json!({ "error": format!("{}", e) });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

// ============= Cloud Import Proxy Routes =============

/// Import a file from a cloud URL (Google Drive, OneDrive, Dropbox)
pub async fn cloud_import(
    State(_deployment): State<DeploymentImpl>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    let url = body["url"].as_str().unwrap_or("");
    let file_name = body["file_name"].as_str();

    match client.cloud_import(url, file_name).await {
        Ok(resp) => {
            let value = serde_json::to_value(resp.import_job).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(e) => {
            let value = serde_json::json!({
                "error": format!("{}", e),
                "service_status": "error",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get active cloud imports
pub async fn get_cloud_imports(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_active_imports().await {
        Ok(resp) => {
            let value = serde_json::to_value(resp).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(_) => {
            let value = serde_json::json!({
                "active": [],
                "service_status": "not_running",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get cloud import history
pub async fn get_cloud_import_history(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_import_history(50).await {
        Ok(resp) => {
            let value = serde_json::to_value(resp).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(_) => {
            let value = serde_json::json!({
                "history": [],
                "service_status": "not_running",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Get download cache stats
pub async fn get_cloud_cache(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    match client.get_cache_stats().await {
        Ok(resp) => {
            let value = serde_json::to_value(resp).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(_) => {
            let value = serde_json::json!({
                "cache": null,
                "service_status": "not_running",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// Resolve a cloud URL to direct download URL
pub async fn resolve_cloud_url(
    State(_deployment): State<DeploymentImpl>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let client = apn_client::ApnClient::new();

    let url = params.get("url").map(|s| s.as_str()).unwrap_or("");

    match client.resolve_cloud_url(url).await {
        Ok(resp) => {
            let value = serde_json::to_value(resp).unwrap_or_default();
            Ok(ResponseJson(ApiResponse::success(value)))
        }
        Err(e) => {
            let value = serde_json::json!({ "error": format!("{}", e) });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/mesh/stats", get(get_mesh_stats))
        .route("/mesh/peers", get(get_mesh_peers))
        .route("/mesh/transactions", get(get_transactions))
        .route("/apn/identity", get(get_apn_identity))
        // File transfer routes (proxy to APN Core)
        .route("/files/send", axum::routing::post(send_file))
        .route("/files/transfers", get(get_active_transfers))
        .route("/files/transfers/{id}", get(get_transfer_status))
        .route("/files/transfers/{id}/accept", axum::routing::post(accept_file_transfer))
        .route("/files/transfers/{id}/cancel", axum::routing::post(cancel_file_transfer))
        .route("/files/history", get(get_file_transfer_history))
        // Cloud import routes (proxy to APN Core)
        .route("/cloud/import", axum::routing::post(cloud_import))
        .route("/cloud/imports", get(get_cloud_imports))
        .route("/cloud/history", get(get_cloud_import_history))
        .route("/cloud/cache", get(get_cloud_cache))
        .route("/cloud/resolve", get(resolve_cloud_url))
}

//! Mesh Network Routes — native APN peer discovery (no external bridge)
//!
//! Reads from the ApnPeerManager singleton which maintains a live NATS-based
//! peer map under the single Pythia Master Node identity (apn_814d37f4).

use axum::{extract::State, response::Json as ResponseJson, routing::get, Router};
use chrono::{DateTime, Utc};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{apn_peer_manager, error::ApiError, DeploymentImpl};
use crate::{error::ApiError, DeploymentImpl};

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
    pub device_name: Option<String>,
    pub resources: Option<serde_json::Value>,
    pub last_seen: Option<String>,
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

/// Resolve this node's APN identity without APN Core running.
/// Priority: APN_NODE_ID env → ~/.apn/node_identity.json → hostname fallback
fn resolve_node_id_fallback() -> String {
    // 1. Check env var (set in .env)
    if let Ok(id) = std::env::var("APN_NODE_ID") {
        if !id.is_empty() {
            return id;
        }
    }

    // 2. Read ~/.apn/node_identity.json
    if let Some(home) = dirs::home_dir() {
        let identity_path = home.join(".apn").join("node_identity.json");
        if let Ok(contents) = std::fs::read_to_string(&identity_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(id) = json.get("node_id").and_then(|v| v.as_str()) {
                    if !id.is_empty() {
                        return id.to_string();
                    }
                }
            }
        }
    }

    // 3. Hostname fallback (last resort)
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    format!("apn_{}", &hostname.replace('.', "-"))
}

/// Fetch peers from APN Core API (replaces log file parsing)
async fn fetch_peers_from_apn_core() -> (Vec<PeerInfo>, bool, String, u64) {
    let client = apn_client::ApnClient::new();

    // Try to get identity from APN Core, then env/node_identity.json, then hostname
    let node_id = match client.get_identity().await {
        Ok(identity) => identity.node_id,
        Err(_) => resolve_node_id_fallback(),
    };

    let (peers, relay_connected, uptime) = match client.get_network_stats().await {
        Ok(stats) => {
            let relay = stats.status == "online";
            let uptime = stats.uptime_seconds;
            let peers = match client.get_peers().await {
                Ok(peer_list) => peer_list
                    .peers
                    .into_iter()
                    .map(|p| PeerInfo {
                        peer_id: p.node_id,
                        address: p.wallet_address,
                        latency_ms: None,
                        connection_type: p.connection_type,
                        bandwidth_mbps: None,
                        reputation: 1.0,
                        capabilities: p.capabilities,
                        device_name: None,
                        resources: None,
                        last_seen: None,
                    })
                    .collect(),
                Err(_) => vec![],
            };
            (peers, relay, uptime)
        }
        Err(_) => {
            let (legacy_peers, legacy_relay) = fetch_peers_from_log_fallback().await;
            (legacy_peers, legacy_relay, 0)
        }
    };

    (peers, relay_connected, node_id, uptime)
}

/// Legacy fallback: parse peers from log file (used when APN Core is not running)
async fn fetch_peers_from_log_fallback() -> (Vec<PeerInfo>, bool) {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

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
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

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
                    peers.insert(
                        node_id.to_string(),
                        PeerInfo {
                            peer_id: node_id.to_string(),
                            address: wallet.to_string(),
                            latency_ms: None,
                            connection_type: "NATS".to_string(),
                            bandwidth_mbps: None,
                            reputation: 1.0,
                            capabilities,
                            device_name: None,
                            resources: None,
                            last_seen: None,
                        },
                    );
                }
            }
        }
    }

    (peers.into_values().collect(), relay_connected)
}

fn get_local_resources() -> ResourceStats {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let cpu_percent = sys.global_cpu_usage() as f64;
    let memory_percent = if sys.total_memory() > 0 {
        sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0
    } else {
        0.0
    };
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk_percent = disks
        .list()
        .first()
        .map(|d| {
            let total = d.total_space();
            if total > 0 {
                (total - d.available_space()) as f64 / total as f64 * 100.0
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);
    ResourceStats {
        cpu_percent,
        memory_percent,
        disk_percent,
        available_compute: 100.0 - cpu_percent,
    }
}

/// GET /api/mesh/stats
pub async fn get_mesh_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MeshStats>>, ApiError> {
    let pool = &deployment.db().pool;

    let (node_id, peers, relay_connected, uptime) = if let Some(mgr) = apn_peer_manager::global() {
        let discovered = mgr.get_peers().await;
        let peers: Vec<PeerInfo> = discovered
            .into_iter()
            .map(|p| PeerInfo {
                peer_id: p.peer_id,
                address: p.wallet_address,
                latency_ms: None,
                connection_type: "NATS".to_string(),
                bandwidth_mbps: None,
                reputation: 1.0,
                capabilities: p.capabilities,
                device_name: Some(p.device_name),
                resources: p.resources,
                last_seen: Some(p.last_seen),
            })
            .collect();
        (
            mgr.node_id.clone(),
            peers,
            mgr.is_relay_connected().await,
            mgr.uptime_seconds(),
        )
    } else {
        (
            std::env::var("APN_NODE_ID").unwrap_or_else(|_| "apn_unknown".to_string()),
            vec![],
            false,
            0,
        )
    };

    let resources = get_local_resources();

    let recent_flows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        r#"
        SELECT af.id, af.flow_type, af.status, af.planning_started_at
        FROM agent_flows af
        ORDER BY af.planning_started_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let transactions: Vec<TransactionLog> = recent_flows
        .into_iter()
        .map(|(id, flow_type, status, ts)| {
            let tx_type = match status.as_str() {
                "completed" => "execution_completed",
                "failed" => "execution_failed",
                "planning" | "executing" => "task_received",
                _ => "task_distributed",
            };
            let timestamp = ts
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            TransactionLog {
                id: id.clone(),
                timestamp,
                tx_type: tx_type.to_string(),
                description: format!(
                    "{} workflow: {}",
                    match status.as_str() {
                        "completed" => "Completed",
                        "failed" => "Failed",
                        _ => "Processing",
                    },
                    flow_type
                ),
                vibe_amount: Some(if status == "completed" { 10.0 } else { 0.0 }),
                peer_node: Some("local".to_string()),
                task_id: Uuid::parse_str(&id).ok(),
            }
        })
        .collect();

    let active_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM agent_flows WHERE status IN ('planning', 'executing')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    let completed_today: (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM agent_flows
           WHERE status = 'completed' AND date(verification_completed_at) = date('now')"#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    let stats = MeshStats {
        node_id,
        status: if relay_connected {
            "online".to_string()
        } else {
            "offline".to_string()
        },
        peers_connected: peers.len(),
        peers,
        bandwidth: BandwidthStats {
            upload_bytes: 0,
            download_bytes: 0,
            upload_rate: 0.0,
            download_rate: 0.0,
        },
        resources,
        relay_connected,
        uptime,
        vibe_balance: completed_today.0 as f64 * 10.0,
        transactions,
        active_tasks: active_count.0 as u32,
        completed_tasks_today: completed_today.0 as u32,
    };

    Ok(ResponseJson(ApiResponse::success(stats)))
}

/// GET /api/mesh/peers
pub async fn get_mesh_peers(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<PeerInfo>>>, ApiError> {
    let peers = if let Some(mgr) = apn_peer_manager::global() {
        mgr.get_peers()
            .await
            .into_iter()
            .map(|p| PeerInfo {
                peer_id: p.peer_id,
                address: p.wallet_address,
                latency_ms: None,
                connection_type: "NATS".to_string(),
                bandwidth_mbps: None,
                reputation: 1.0,
                capabilities: p.capabilities,
                device_name: Some(p.device_name),
                resources: p.resources,
                last_seen: Some(p.last_seen),
            })
            .collect()
    } else {
        vec![]
    };
    Ok(ResponseJson(ApiResponse::success(peers)))
}

/// GET /api/apn/identity
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
            let node_id = resolve_node_id_fallback();
            let mut wallet = std::env::var("APN_WALLET_ADDRESS").ok();
            let mut device_name = std::env::var("APN_DEVICE_NAME").ok();
            let mut public_key: Option<String> = None;

            if let Some(home) = dirs::home_dir() {
                let identity_path = home.join(".apn").join("node_identity.json");
                if let Ok(contents) = std::fs::read_to_string(&identity_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                        let str_field =
                            |key: &str| json.get(key).and_then(|v| v.as_str()).map(String::from);
                        wallet = wallet.or_else(|| str_field("wallet_address"));
                        public_key = str_field("public_key");
                        device_name = device_name.or_else(|| str_field("device_name"));
                    }
                }
            }

            let value = serde_json::json!({
                "node_id": node_id,
                "device_name": device_name,
                "wallet_address": wallet,
                "public_key": public_key,
                "apn_core_connected": false,
                "message": "Identity resolved from local config (APN Core not running)",
            });
            Ok(ResponseJson(ApiResponse::success(value)))
        }
    }
}

/// GET /api/mesh/transactions
pub async fn get_transactions(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TransactionLog>>>, ApiError> {
    let pool = &deployment.db().pool;
    let flows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        r#"
        SELECT af.id, af.flow_type, af.status, af.planning_started_at
        FROM agent_flows af
        ORDER BY af.planning_started_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let transactions: Vec<TransactionLog> = flows
        .into_iter()
        .map(|(id, flow_type, status, ts)| {
            let tx_type = match status.as_str() {
                "completed" => "execution_completed",
                "failed" => "execution_failed",
                "planning" | "executing" => "task_received",
                _ => "task_distributed",
            };
            let timestamp = ts
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            TransactionLog {
                id: id.clone(),
                timestamp,
                tx_type: tx_type.to_string(),
                description: format!(
                    "{} workflow: {}",
                    match status.as_str() {
                        "completed" => "Completed",
                        "failed" => "Failed",
                        _ => "Processing",
                    },
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

// ─── File transfer & cloud import routes (stubbed — bridge no longer proxies) ─

pub async fn send_file(
    State(_): State<DeploymentImpl>,
    axum::extract::Json(_body): axum::extract::Json<serde_json::Value>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "status": "not_implemented",
        "message": "File transfer via APN native P2P — coming soon"
    }))))
}

pub async fn get_active_transfers(
    State(_): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "active": [] }),
    )))
}

pub async fn get_transfer_status(
    State(_): State<DeploymentImpl>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "status": "not_found" }),
    )))
}

pub async fn get_file_transfer_history(
    State(_): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "history": [] }),
    )))
}

pub async fn accept_file_transfer(
    State(_): State<DeploymentImpl>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "status": "not_implemented" }),
    )))
}

pub async fn cancel_file_transfer(
    State(_): State<DeploymentImpl>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "status": "not_implemented" }),
    )))
}

pub async fn cloud_import(
    State(_): State<DeploymentImpl>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    // Cloud import now handled via sovereign storage — stub for API compat
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "status": "not_implemented",
        "url": body.get("url"),
        "message": "Cloud import via APN native — coming soon"
    }))))
}

pub async fn get_cloud_imports(
    State(_): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "active": [] }),
    )))
}

pub async fn get_cloud_import_history(
    State(_): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "history": [] }),
    )))
}

pub async fn get_cloud_cache(
    State(_): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "cache": null }),
    )))
}

pub async fn resolve_cloud_url(
    State(_): State<DeploymentImpl>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "url": params.get("url"),
        "resolved": null,
        "status": "not_implemented"
    }))))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/mesh/stats", get(get_mesh_stats))
        .route("/mesh/peers", get(get_mesh_peers))
        .route("/mesh/transactions", get(get_transactions))
        .route("/apn/identity", get(get_apn_identity))
        .route("/files/send", axum::routing::post(send_file))
        .route("/files/transfers", get(get_active_transfers))
        .route("/files/transfers/{id}", get(get_transfer_status))
        .route(
            "/files/transfers/{id}/accept",
            axum::routing::post(accept_file_transfer),
        )
        .route(
            "/files/transfers/{id}/cancel",
            axum::routing::post(cancel_file_transfer),
        )
        .route("/cloud/import", axum::routing::post(cloud_import))
        .route("/cloud/imports", get(get_cloud_imports))
        .route("/cloud/history", get(get_cloud_import_history))
        .route("/cloud/cache", get(get_cloud_cache))
        .route("/cloud/resolve", get(resolve_cloud_url))
}

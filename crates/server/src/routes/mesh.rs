//! Mesh Network Routes - API endpoints for APN mesh network stats and operations
//!
//! Provides real-time mesh network statistics, peer information, and
//! transaction logs for the Alpha Protocol Network integration.

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
use std::time::Duration;

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

/// Peer announcement from NATS (matches alpha_protocol_core::relay::PeerAnnouncement)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NatsPeerAnnouncement {
    node_id: String,
    wallet_address: String,
    capabilities: Vec<String>,
    timestamp: String,
    resources: Option<alpha_protocol_core::wire::NodeResources>,
}

/// Fetch peers from master node log
async fn fetch_peers_from_log() -> (Vec<PeerInfo>, bool) {
    use std::io::{BufRead, BufReader};
    use std::fs::File;
    use regex::Regex;

    let log_path = "/tmp/apn_node.log";
    let mut peers = std::collections::HashMap::new();
    let mut relay_connected = false;

    if let Ok(file) = File::open(log_path) {
        let reader = BufReader::new(file);

        // Regex to match peer announcements with resources
        let peer_regex = Regex::new(
            r#"Message from apn\.discovery \(([^)]+)\): PeerAnnouncement \{ wallet_address: "([^"]+)", capabilities: \[([^\]]+)\], resources: (.+?) \}"#
        ).unwrap();

        // Check for relay connection
        let relay_regex = Regex::new(r"Relay connected|🌐 Relay connected").unwrap();

        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        for line in lines.iter().rev().take(1000) {
            // Check relay status
            if relay_regex.is_match(line) {
                relay_connected = true;
            }

            // Parse peer announcements
            if let Some(caps) = peer_regex.captures(line) {
                let node_id = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let wallet = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let caps_str = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                // Parse capabilities
                let capabilities: Vec<String> = caps_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect();

                // Only add if not already present (keep most recent)
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
    // Get real system metrics
    let sys_info = sysinfo::System::new_all();

    let cpu_percent = sys_info.global_cpu_usage() as f64;
    let memory_percent = if sys_info.total_memory() > 0 {
        (sys_info.used_memory() as f64 / sys_info.total_memory() as f64) * 100.0
    } else {
        0.0
    };

    // Get disk usage
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

    // Count active and completed tasks
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

    // Calculate Vibe balance from completed tasks
    let vibe_balance = completed_today.0 as f64 * 10.0;

    // Fetch real peer data from master node log
    let (peers, relay_connected) = fetch_peers_from_log().await;

    // Try to get node ID from log, fallback to hostname
    let node_id = if let Ok(log_content) = tokio::fs::read_to_string("/tmp/apn_node.log").await {
        if let Some(line) = log_content.lines().find(|l| l.contains("Node ID:")) {
            if let Some(id) = line.split("Node ID:").nth(1) {
                id.trim().to_string()
            } else {
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                format!("omega-{}", &hostname[..hostname.len().min(8)])
            }
        } else {
            let hostname = hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            format!("omega-{}", &hostname[..hostname.len().min(8)])
        }
    } else {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!("omega-{}", &hostname[..hostname.len().min(8)])
    };

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
        uptime: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        vibe_balance,
        transactions,
        active_tasks: active_count.0 as u32,
        completed_tasks_today: completed_today.0 as u32,
    };

    Ok(ResponseJson(ApiResponse::success(stats)))
}

/// Get mesh peers
pub async fn get_mesh_peers(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<PeerInfo>>>, ApiError> {
    let (peers, _) = fetch_peers_from_log().await;
    Ok(ResponseJson(ApiResponse::success(peers)))
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

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/mesh/stats", get(get_mesh_stats))
        .route("/mesh/peers", get(get_mesh_peers))
        .route("/mesh/transactions", get(get_transactions))
}

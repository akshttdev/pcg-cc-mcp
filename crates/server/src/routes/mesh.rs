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

    // Generate node ID from hostname
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let node_id = format!("omega-{}", &hostname[..hostname.len().min(8)]);

    let stats = MeshStats {
        node_id,
        status: "online".to_string(),
        peers_connected: 0, // Will be populated when APN is fully integrated
        peers: vec![],
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
        relay_connected: false, // Will be true when NATS relay is connected
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
    // Will be populated when APN is fully integrated
    Ok(ResponseJson(ApiResponse::success(vec![])))
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

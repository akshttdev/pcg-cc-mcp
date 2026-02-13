//! Type definitions for APN Core API responses
//!
//! These types mirror the JSON responses from APN Core's FastAPI server.
//! They are the canonical types used by Dashboard and Pythia to interact
//! with the network substrate.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health check response from APN Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub node_id: String,
    pub uptime_seconds: u64,
}

/// Node identity - THE single source of truth for this device on the APN network.
/// Dashboard and Pythia use this instead of generating their own identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub node_id: String,
    pub wallet_address: String,
    pub public_key: String,
    pub identity_file: Option<String>,
    pub created_at: Option<String>,
}

/// Version information from APN Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub protocol: Option<String>,
    pub layer: Option<u8>,
    pub layer_name: Option<String>,
    pub node_id: String,
    pub wallet_address: String,
    pub uptime_seconds: Option<u64>,
}

/// A peer node on the APN network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerNode {
    pub node_id: String,
    pub wallet_address: String,
    pub capabilities: Vec<String>,
    pub resources: Option<serde_json::Value>,
    pub agents: Vec<String>,
    pub software: HashMap<String, serde_json::Value>,
    pub last_seen: String,
    pub connection_type: String,
}

/// List of peers from APN Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerList {
    pub node_id: String,
    pub peer_count: usize,
    pub peers: Vec<PeerNode>,
    pub timestamp: String,
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub node_id: String,
    pub status: String,
    pub peers_connected: usize,
    pub relay_url: String,
    pub uptime_seconds: u64,
    pub resources: serde_json::Value,
    pub timestamp: String,
}

/// Node capabilities (agents, software, contribution types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub agents: Vec<String>,
    pub software: HashMap<String, serde_json::Value>,
    pub contribution: Vec<String>,
    pub updated_at: Option<String>,
}

/// Capabilities response from APN Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesResponse {
    pub node_id: String,
    pub capabilities: NodeCapabilities,
    pub timestamp: String,
}

/// Request to update capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesUpdate {
    pub agents: Vec<String>,
    pub software: HashMap<String, serde_json::Value>,
    pub contribution: Vec<String>,
}

/// System resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesResponse {
    pub node_id: String,
    pub resources: SystemResources,
    pub timestamp: String,
}

/// Detailed system resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub cpu_cores: Option<u32>,
    pub cpu_percent: Option<f64>,
    pub ram_mb: Option<u64>,
    pub ram_used_mb: Option<u64>,
    pub ram_percent: Option<f64>,
    pub storage_gb: Option<u64>,
    pub storage_used_gb: Option<u64>,
    pub storage_percent: Option<f64>,
    pub gpu_available: Option<bool>,
    pub gpu_model: Option<String>,
    pub platform: Option<String>,
    pub hostname: Option<String>,
}

/// Contribution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionStatus {
    pub node_id: String,
    pub wallet_address: String,
    pub contribution: ContributionSettings,
    pub relay_url: String,
}

/// Contribution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionSettings {
    pub enabled: bool,
    pub relay_enabled: Option<bool>,
    pub compute_enabled: Option<bool>,
    pub storage_enabled: Option<bool>,
}

/// Request to update contribution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionSettingsUpdate {
    pub enabled: bool,
    pub relay_enabled: bool,
    pub compute_enabled: bool,
    pub storage_enabled: bool,
}

// ============= File Transfer Types =============

/// Request to send a file to another node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSendRequest {
    pub target_node_id: String,
    pub file_path: String,
}

/// A file transfer record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferInfo {
    pub transfer_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: String,
    pub source_node: String,
    pub target_node: String,
    pub direction: String,
    pub status: String,
    pub chunks_total: u32,
    pub chunks_transferred: u32,
    pub bytes_transferred: u64,
    pub started_at: f64,
    pub completed_at: f64,
    pub error: Option<String>,
    pub local_path: Option<String>,
    pub progress_pct: f64,
    pub speed_bps: f64,
}

/// Response for active transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTransfersResponse {
    pub active: Vec<FileTransferInfo>,
    pub service_status: String,
}

/// Response for transfer history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferHistoryResponse {
    pub history: Vec<FileTransferInfo>,
    pub service_status: String,
}

/// Response wrapping a single transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponse {
    pub transfer: FileTransferInfo,
}

// ============= Cloud Import Types =============

/// Request to import a file from a cloud URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudImportRequest {
    pub url: String,
    pub file_name: Option<String>,
}

/// A cloud import job record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudImportJob {
    pub job_id: String,
    pub source_url: String,
    pub provider: String,
    pub resolved_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    pub file_hash: Option<String>,
    pub local_path: Option<String>,
    pub status: String,
    pub progress_pct: f64,
    pub bytes_downloaded: u64,
    pub speed_bps: f64,
    pub started_at: f64,
    pub completed_at: f64,
    pub error: Option<String>,
    pub cached: bool,
}

/// Response for active imports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveImportsResponse {
    pub active: Vec<CloudImportJob>,
    pub service_status: String,
}

/// Response for import history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportHistoryResponse {
    pub history: Vec<CloudImportJob>,
    pub service_status: String,
}

/// Response wrapping a single import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResponse {
    #[serde(rename = "import")]
    pub import_job: CloudImportJob,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    pub cache: Option<CacheStats>,
    pub service_status: String,
}

/// Cache stats detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cached_files: u32,
    pub cached_urls: u32,
    pub total_size_bytes: u64,
    pub cache_dir: String,
}

/// URL resolution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveUrlResponse {
    pub source_url: String,
    pub provider: String,
    pub resolved_url: String,
}

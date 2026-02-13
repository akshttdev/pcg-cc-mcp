//! APN Client - Interface to APN Core (Layer 0: Network Substrate)
//!
//! This crate provides a typed HTTP client for consuming the APN Core API.
//! It is used by:
//! - **Dashboard (Layer 2)**: To get node identity, display network peers, register capabilities
//! - **Pythia (Layer 3)**: To query network state for task routing and reward distribution
//!
//! # Architecture
//!
//! ```text
//! Dashboard/Pythia  -->  ApnClient  -->  APN Core (localhost:8000)
//!                        (this crate)     (always-running service)
//! ```
//!
//! APN Core owns the single device identity. All other services consume it via this client.

mod types;

pub use types::*;

use anyhow::{Context, Result};
use tracing::{debug, warn, info};

/// Default APN Core API URL
pub const DEFAULT_APN_CORE_URL: &str = "http://localhost:8000";

/// Client for communicating with APN Core (Layer 0)
#[derive(Debug, Clone)]
pub struct ApnClient {
    base_url: String,
    client: reqwest::Client,
}

/// Error types for APN Client operations
#[derive(Debug, thiserror::Error)]
pub enum ApnClientError {
    #[error("APN Core not reachable at {url}: {source}")]
    NotReachable {
        url: String,
        source: reqwest::Error,
    },

    #[error("APN Core returned error {status}: {body}")]
    ApiError {
        status: u16,
        body: String,
    },

    #[error("Failed to parse APN Core response: {0}")]
    ParseError(#[from] reqwest::Error),
}

impl ApnClient {
    /// Create a new APN Client connecting to the default URL (localhost:8000)
    pub fn new() -> Self {
        Self::with_url(DEFAULT_APN_CORE_URL)
    }

    /// Create a new APN Client connecting to a specific URL
    pub fn with_url(url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Check if APN Core is running and healthy
    pub async fn health_check(&self) -> Result<HealthResponse> {
        let resp = self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| ApnClientError::NotReachable {
                url: self.base_url.clone(),
                source: e,
            })?;

        let health: HealthResponse = resp.json().await?;
        Ok(health)
    }

    /// Check if APN Core is running (returns false if not reachable)
    pub async fn is_running(&self) -> bool {
        match self.health_check().await {
            Ok(h) => h.status == "healthy",
            Err(_) => false,
        }
    }

    // ============= Identity Endpoints =============

    /// Get this node's identity from APN Core.
    /// This is THE single source of truth for device identity on the APN network.
    pub async fn get_identity(&self) -> Result<NodeIdentity> {
        let resp = self.client
            .get(format!("{}/api/identity", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for identity")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        let identity: NodeIdentity = resp.json().await?;
        debug!("Got identity from APN Core: {}", identity.node_id);
        Ok(identity)
    }

    /// Get version information
    pub async fn get_version(&self) -> Result<VersionInfo> {
        let resp = self.client
            .get(format!("{}/api/version", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for version")?;

        Ok(resp.json().await?)
    }

    // ============= Network Endpoints =============

    /// Get all known peers on the APN network
    pub async fn get_peers(&self) -> Result<PeerList> {
        let resp = self.client
            .get(format!("{}/api/network/peers", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for peers")?;

        Ok(resp.json().await?)
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> Result<NetworkStats> {
        let resp = self.client
            .get(format!("{}/api/network/stats", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for network stats")?;

        Ok(resp.json().await?)
    }

    // ============= Capability Endpoints =============

    /// Get this node's capabilities
    pub async fn get_capabilities(&self) -> Result<CapabilitiesResponse> {
        let resp = self.client
            .get(format!("{}/api/capabilities", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for capabilities")?;

        Ok(resp.json().await?)
    }

    /// Register/update this node's capabilities
    pub async fn update_capabilities(&self, caps: &CapabilitiesUpdate) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/capabilities", self.base_url))
            .json(caps)
            .send()
            .await
            .context("Failed to reach APN Core to update capabilities")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    // ============= Resource Endpoints =============

    /// Get current system resource usage
    pub async fn get_resources(&self) -> Result<ResourcesResponse> {
        let resp = self.client
            .get(format!("{}/api/resources", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for resources")?;

        Ok(resp.json().await?)
    }

    // ============= Contribution Endpoints =============

    /// Get contribution status
    pub async fn get_contribution_status(&self) -> Result<ContributionStatus> {
        let resp = self.client
            .get(format!("{}/api/contribution/status", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for contribution status")?;

        Ok(resp.json().await?)
    }

    /// Update contribution settings
    pub async fn update_contribution_settings(&self, settings: &ContributionSettingsUpdate) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/contribution/settings", self.base_url))
            .json(settings)
            .send()
            .await
            .context("Failed to reach APN Core to update contribution")?;

        Ok(resp.json().await?)
    }

    // ============= File Transfer Endpoints =============

    /// Send a file to another node via P2P transfer
    pub async fn send_file(&self, target_node_id: &str, file_path: &str) -> Result<TransferResponse> {
        let req = FileSendRequest {
            target_node_id: target_node_id.to_string(),
            file_path: file_path.to_string(),
        };

        let resp = self.client
            .post(format!("{}/api/files/send", self.base_url))
            .json(&req)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .context("Failed to reach APN Core to send file")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    /// Get all active file transfers
    pub async fn get_active_transfers(&self) -> Result<ActiveTransfersResponse> {
        let resp = self.client
            .get(format!("{}/api/files/transfers", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for transfers")?;

        Ok(resp.json().await?)
    }

    /// Get a specific transfer by ID
    pub async fn get_transfer(&self, transfer_id: &str) -> Result<TransferResponse> {
        let resp = self.client
            .get(format!("{}/api/files/transfers/{}", self.base_url, transfer_id))
            .send()
            .await
            .context("Failed to reach APN Core for transfer status")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    /// Get file transfer history
    pub async fn get_transfer_history(&self, limit: u32) -> Result<TransferHistoryResponse> {
        let resp = self.client
            .get(format!("{}/api/files/history?limit={}", self.base_url, limit))
            .send()
            .await
            .context("Failed to reach APN Core for transfer history")?;

        Ok(resp.json().await?)
    }

    /// Accept a pending incoming transfer
    pub async fn accept_transfer(&self, transfer_id: &str) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/files/transfers/{}/accept", self.base_url, transfer_id))
            .send()
            .await
            .context("Failed to reach APN Core to accept transfer")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    // ============= Cloud Import Endpoints =============

    /// Import a file from a cloud storage URL
    pub async fn cloud_import(&self, url: &str, file_name: Option<&str>) -> Result<ImportResponse> {
        let req = CloudImportRequest {
            url: url.to_string(),
            file_name: file_name.map(String::from),
        };

        let resp = self.client
            .post(format!("{}/api/cloud/import", self.base_url))
            .json(&req)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .context("Failed to reach APN Core for cloud import")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    /// Get active cloud imports
    pub async fn get_active_imports(&self) -> Result<ActiveImportsResponse> {
        let resp = self.client
            .get(format!("{}/api/cloud/imports", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for imports")?;

        Ok(resp.json().await?)
    }

    /// Get a specific import status
    pub async fn get_import(&self, job_id: &str) -> Result<ImportResponse> {
        let resp = self.client
            .get(format!("{}/api/cloud/imports/{}", self.base_url, job_id))
            .send()
            .await
            .context("Failed to reach APN Core for import status")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    /// Get cloud import history
    pub async fn get_import_history(&self, limit: u32) -> Result<ImportHistoryResponse> {
        let resp = self.client
            .get(format!("{}/api/cloud/history?limit={}", self.base_url, limit))
            .send()
            .await
            .context("Failed to reach APN Core for import history")?;

        Ok(resp.json().await?)
    }

    /// Get download cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStatsResponse> {
        let resp = self.client
            .get(format!("{}/api/cloud/cache", self.base_url))
            .send()
            .await
            .context("Failed to reach APN Core for cache stats")?;

        Ok(resp.json().await?)
    }

    /// Resolve a cloud URL to a direct download URL
    pub async fn resolve_cloud_url(&self, url: &str) -> Result<ResolveUrlResponse> {
        let resp = self.client
            .get(format!("{}/api/cloud/resolve?url={}", self.base_url, urlencoding::encode(url)))
            .send()
            .await
            .context("Failed to reach APN Core to resolve URL")?;

        Ok(resp.json().await?)
    }

    /// Cancel an active transfer
    pub async fn cancel_transfer(&self, transfer_id: &str) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/files/transfers/{}/cancel", self.base_url, transfer_id))
            .send()
            .await
            .context("Failed to reach APN Core to cancel transfer")?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApnClientError::ApiError { status, body }.into());
        }

        Ok(resp.json().await?)
    }
}

impl Default for ApnClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Try to connect to APN Core and return identity, or log a warning if unavailable.
/// Used during Dashboard startup.
pub async fn try_connect_to_apn_core() -> Option<(ApnClient, NodeIdentity)> {
    let client = ApnClient::new();

    match client.get_identity().await {
        Ok(identity) => {
            info!(
                "Connected to APN Core - Node: {} Wallet: {}",
                identity.node_id,
                &identity.wallet_address[..10]
            );
            Some((client, identity))
        }
        Err(e) => {
            warn!(
                "APN Core not available at {} - running in standalone mode. Error: {}",
                DEFAULT_APN_CORE_URL, e
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ApnClient::new();
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_client_custom_url() {
        let client = ApnClient::with_url("http://192.168.1.100:9000");
        assert_eq!(client.base_url, "http://192.168.1.100:9000");
    }
}

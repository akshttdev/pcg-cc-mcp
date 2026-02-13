//! Pythia Client - HTTP client for consuming the Pythia Layer 3 API
//!
//! Used by the PCG Dashboard (Layer 2) to:
//! - Submit tasks for routing
//! - Query VIBE balances and economics
//! - Get network statistics and node reputation
//! - Estimate task costs before submission

use reqwest::Client;
use tracing::{info, warn};

pub mod types;
pub use types::*;

/// Error types for Pythia client operations
#[derive(Debug, thiserror::Error)]
pub enum PythiaClientError {
    #[error("Pythia service not reachable at {0}")]
    NotReachable(String),
    #[error("Pythia API error: {0}")]
    ApiError(String),
    #[error("Failed to parse Pythia response: {0}")]
    ParseError(String),
}

/// Client for the Pythia Layer 3 API
#[derive(Clone)]
pub struct PythiaClient {
    base_url: String,
    client: Client,
}

impl PythiaClient {
    /// Create a new Pythia client with the given base URL
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    /// Check if Pythia is reachable
    pub async fn is_running(&self) -> bool {
        self.health_check().await.is_ok()
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthResponse, PythiaClientError> {
        let url = format!("{}/health", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|_| PythiaClientError::NotReachable(self.base_url.clone()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    // ─── Task Operations ──────────────────────────────────────────────────

    /// Submit a task for routing
    pub async fn submit_task(&self, req: TaskSubmitRequest) -> Result<TaskSubmitResponse, PythiaClientError> {
        let url = format!("{}/api/tasks/submit", self.base_url);
        let resp = self.client.post(&url).json(&req).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await
                .unwrap_or_else(|_| serde_json::json!({"error": "Unknown error"}));
            return Err(PythiaClientError::ApiError(
                err["error"].as_str().unwrap_or("Unknown error").to_string()
            ));
        }

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Result<serde_json::Value, PythiaClientError> {
        let url = format!("{}/api/tasks/{}/status", self.base_url, task_id);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PythiaClientError::ApiError("Task not found".to_string()));
        }

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Result<serde_json::Value, PythiaClientError> {
        let url = format!("{}/api/tasks", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    // ─── Rewards & Economics ──────────────────────────────────────────────

    /// Get VIBE balance for a wallet
    pub async fn get_balance(&self, wallet: &str) -> Result<WalletBalance, PythiaClientError> {
        let url = format!("{}/api/rewards/balance?wallet={}", self.base_url, wallet);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PythiaClientError::ApiError("Wallet not found".to_string()));
        }

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Get current reward rates
    pub async fn get_reward_rates(&self) -> Result<serde_json::Value, PythiaClientError> {
        let url = format!("{}/api/rewards/rates", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Get recent transactions (optionally filtered by wallet)
    pub async fn get_transactions(
        &self,
        wallet: Option<&str>,
        limit: Option<usize>,
    ) -> Result<TransactionList, PythiaClientError> {
        let mut url = format!("{}/api/rewards/transactions?", self.base_url);
        if let Some(w) = wallet {
            url.push_str(&format!("wallet={}&", w));
        }
        if let Some(l) = limit {
            url.push_str(&format!("limit={}", l));
        }

        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Trigger reward flush (flush pending heartbeat rewards)
    pub async fn flush_rewards(&self) -> Result<serde_json::Value, PythiaClientError> {
        let url = format!("{}/api/rewards/flush", self.base_url);
        let resp = self.client.post(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    // ─── Economics ────────────────────────────────────────────────────────

    /// Get aggregate economics statistics
    pub async fn get_economics_stats(&self) -> Result<EconomicsStats, PythiaClientError> {
        let url = format!("{}/api/economics/stats", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Get reputation for a node wallet
    pub async fn get_reputation(&self, wallet: &str) -> Result<NodeReputation, PythiaClientError> {
        let url = format!("{}/api/economics/reputation?wallet={}", self.base_url, wallet);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PythiaClientError::ApiError("Node not found".to_string()));
        }

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    // ─── Network ──────────────────────────────────────────────────────────

    /// Get all active network nodes
    pub async fn get_network_nodes(&self) -> Result<NetworkNodes, PythiaClientError> {
        let url = format!("{}/api/network/nodes", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> Result<serde_json::Value, PythiaClientError> {
        let url = format!("{}/api/network/stats", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }

    // ─── Cost Estimation ──────────────────────────────────────────────────

    /// Estimate cost for running an agent
    pub async fn estimate_cost(&self, agent: &str, node_id: &str) -> Result<CostEstimate, PythiaClientError> {
        let url = format!(
            "{}/api/cost/estimate?agent={}&node_id={}",
            self.base_url, agent, node_id
        );
        let resp = self.client.get(&url).send().await
            .map_err(|e| PythiaClientError::ApiError(e.to_string()))?;

        resp.json().await
            .map_err(|e| PythiaClientError::ParseError(e.to_string()))
    }
}

/// Try to connect to Pythia service, returning a client if successful
pub async fn try_connect_to_pythia() -> Option<PythiaClient> {
    let url = std::env::var("PYTHIA_URL").unwrap_or_else(|_| "http://localhost:8100".to_string());
    let client = PythiaClient::new(&url);

    match client.health_check().await {
        Ok(health) => {
            info!(
                "Connected to Pythia v{} ({} peers)",
                health.version, health.peer_count
            );
            Some(client)
        }
        Err(e) => {
            warn!("Pythia not available at {}: {}", url, e);
            None
        }
    }
}

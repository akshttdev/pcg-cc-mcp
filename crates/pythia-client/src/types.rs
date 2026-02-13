//! Response types for the Pythia API

use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub peer_count: usize,
}

/// Task submission request
#[derive(Debug, Clone, Serialize)]
pub struct TaskSubmitRequest {
    pub agent: String,
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    pub prefer_local: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_vibe_cost: Option<f64>,
    pub submitter_node_id: String,
    pub submitter_wallet: String,
}

/// Task submission response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskSubmitResponse {
    pub task_id: String,
    pub target_node_id: String,
    pub estimated_vibe_cost: f64,
    pub routing_reason: String,
    pub is_local: bool,
}

/// VIBE wallet balance
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalletBalance {
    pub wallet_address: String,
    pub node_id: String,
    pub balance_vibe: f64,
    pub total_earned: f64,
    pub total_spent: f64,
    pub pending_rewards: f64,
    pub heartbeat_count: u64,
    pub last_updated: String,
}

/// Transaction list response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionList {
    pub count: usize,
    pub transactions: Vec<RewardTransaction>,
}

/// A single reward transaction
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RewardTransaction {
    pub id: String,
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount_vibe: f64,
    pub reason: String,
    pub reward_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub multiplier: f64,
    pub timestamp: String,
    pub status: String,
}

/// Aggregate economics statistics
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EconomicsStats {
    pub total_wallets: usize,
    pub total_vibe_distributed: f64,
    pub total_vibe_spent: f64,
    pub total_pending_rewards: f64,
    pub total_transactions: usize,
    pub total_heartbeats_tracked: u64,
    pub active_reward_peers: usize,
}

/// Node reputation info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeReputation {
    pub node_id: String,
    pub total_contribution: u64,
    pub success_rate: u8,
    pub avg_response_time_ms: u32,
    pub age_hours: u64,
    pub stake_amount: f64,
    pub slash_count: u32,
    pub reputation_score: u32,
}

/// Network nodes response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkNodes {
    pub node_count: usize,
    pub nodes: Vec<serde_json::Value>,
}

/// Cost estimate response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CostEstimate {
    pub agent: String,
    pub local_available: bool,
    pub local_cost: f64,
    pub cheapest_network_cost: f64,
    pub network_nodes_available: usize,
}

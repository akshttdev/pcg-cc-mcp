//! Task Distributor - Routes tasks to capable mesh nodes
//!
//! Handles the logic of finding suitable nodes for task execution,
//! bidding/negotiation, and task assignment.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

use super::types::*;

/// Task distribution service
pub struct TaskDistributor {
    /// Known capable peers
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Pending task distributions awaiting bids
    pending: Arc<RwLock<HashMap<Uuid, PendingDistribution>>>,
    /// Distribution settings
    settings: DistributorSettings,
}

#[derive(Debug, Clone)]
struct PendingDistribution {
    request: TaskDistributionRequest,
    bids: Vec<TaskBid>,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TaskBid {
    pub node_id: String,
    pub estimated_time_ms: u64,
    pub bid_vibe: f64,
    pub capabilities: Vec<String>,
    pub received_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DistributorSettings {
    /// Maximum time to wait for bids
    pub bid_timeout_ms: u64,
    /// Minimum number of bids before selection
    pub min_bids: usize,
    /// Prefer local execution over remote
    pub prefer_local: bool,
    /// Maximum task queue depth per peer
    pub max_peer_queue: usize,
}

impl Default for DistributorSettings {
    fn default() -> Self {
        Self {
            bid_timeout_ms: 5000,
            min_bids: 1,
            prefer_local: true,
            max_peer_queue: 4,
        }
    }
}

impl TaskDistributor {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
            settings: DistributorSettings::default(),
        }
    }

    pub fn with_settings(settings: DistributorSettings) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
            settings,
        }
    }

    /// Register a peer as capable of task execution
    pub async fn register_peer(&self, peer: PeerInfo) {
        let mut peers = self.peers.write().await;
        peers.insert(peer.node_id.clone(), peer);
    }

    /// Remove a peer
    pub async fn remove_peer(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(node_id);
    }

    /// Get all known peers
    pub async fn peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Find peers capable of executing a task
    pub async fn find_capable_peers(&self, requirements: &ResourceRequirements) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;

        peers
            .values()
            .filter(|peer| {
                // Check if peer has all required capabilities
                requirements
                    .required_capabilities
                    .iter()
                    .all(|cap| peer.capabilities.contains(cap))
            })
            .cloned()
            .collect()
    }

    /// Distribute a task to the network
    pub async fn distribute(&self, request: TaskDistributionRequest) -> anyhow::Result<TaskDistributionResult> {
        // Find capable peers
        let capable_peers = self.find_capable_peers(&request.resource_requirements).await;

        if capable_peers.is_empty() {
            // No remote peers available - this is actually OK for now
            // In a real implementation, we'd either:
            // 1. Execute locally
            // 2. Wait for peers to become available
            // 3. Return an error

            // For now, return a "local" execution result
            return Ok(TaskDistributionResult {
                task_id: request.task_id,
                assigned_node: "local".to_string(),
                estimated_start_time: Some(Utc::now()),
                agreed_reward: 0.0, // Local execution doesn't cost vibe
            });
        }

        // Select the best peer based on:
        // 1. Reputation
        // 2. Latency
        // 3. Available capacity
        let best_peer = capable_peers
            .iter()
            .max_by(|a, b| {
                // Higher reputation is better
                let rep_cmp = a.reputation.partial_cmp(&b.reputation).unwrap_or(std::cmp::Ordering::Equal);
                if rep_cmp != std::cmp::Ordering::Equal {
                    return rep_cmp;
                }

                // Lower latency is better
                let a_lat = a.latency_ms.unwrap_or(u64::MAX);
                let b_lat = b.latency_ms.unwrap_or(u64::MAX);
                b_lat.cmp(&a_lat)
            })
            .ok_or_else(|| anyhow::anyhow!("No suitable peer found"))?;

        Ok(TaskDistributionResult {
            task_id: request.task_id,
            assigned_node: best_peer.node_id.clone(),
            estimated_start_time: Some(Utc::now()),
            agreed_reward: request.reward_vibe,
        })
    }

    /// Handle an incoming bid for a pending task
    pub async fn receive_bid(&self, task_id: Uuid, bid: TaskBid) -> anyhow::Result<()> {
        let mut pending = self.pending.write().await;

        if let Some(distribution) = pending.get_mut(&task_id) {
            distribution.bids.push(bid);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No pending distribution for task {}", task_id))
        }
    }

    /// Check if a task should be executed locally vs distributed
    pub fn should_distribute(&self, _requirements: &ResourceRequirements) -> bool {
        // For now, always try local first if prefer_local is set
        // In the future, this would consider:
        // - Local resource availability
        // - Task priority/urgency
        // - Cost optimization
        // - Load balancing
        !self.settings.prefer_local
    }
}

impl Default for TaskDistributor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_peer() {
        let distributor = TaskDistributor::new();

        let peer = PeerInfo {
            node_id: "test_node".to_string(),
            address: "127.0.0.1".to_string(),
            capabilities: vec!["compute".to_string()],
            reputation: 1.0,
            latency_ms: Some(50),
            available_bandwidth_mbps: Some(100.0),
            last_seen: Utc::now(),
        };

        distributor.register_peer(peer.clone()).await;

        let peers = distributor.peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "test_node");
    }

    #[tokio::test]
    async fn test_find_capable_peers() {
        let distributor = TaskDistributor::new();

        // Register peers with different capabilities
        distributor.register_peer(PeerInfo {
            node_id: "compute_node".to_string(),
            address: "127.0.0.1".to_string(),
            capabilities: vec!["compute".to_string()],
            reputation: 1.0,
            latency_ms: Some(50),
            available_bandwidth_mbps: Some(100.0),
            last_seen: Utc::now(),
        }).await;

        distributor.register_peer(PeerInfo {
            node_id: "gpu_node".to_string(),
            address: "127.0.0.2".to_string(),
            capabilities: vec!["compute".to_string(), "gpu".to_string()],
            reputation: 1.0,
            latency_ms: Some(100),
            available_bandwidth_mbps: Some(50.0),
            last_seen: Utc::now(),
        }).await;

        // Find peers that can do compute
        let compute_peers = distributor.find_capable_peers(&ResourceRequirements {
            required_capabilities: vec!["compute".to_string()],
            ..Default::default()
        }).await;
        assert_eq!(compute_peers.len(), 2);

        // Find peers that can do GPU compute
        let gpu_peers = distributor.find_capable_peers(&ResourceRequirements {
            required_capabilities: vec!["compute".to_string(), "gpu".to_string()],
            ..Default::default()
        }).await;
        assert_eq!(gpu_peers.len(), 1);
        assert_eq!(gpu_peers[0].node_id, "gpu_node");
    }
}

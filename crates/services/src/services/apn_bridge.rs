//! APN Bridge - Connects PCG Dashboard to Alpha Protocol Network
//!
//! This service bridges the PCG Dashboard (Topsi/Pythia orchestrator)
//! to the APN network, enabling:
//! - Task distribution to remote Vibe Nodes
//! - Resource discovery across the mesh
//! - Vibe token accounting for task completion
//! - Real-time network status monitoring

use anyhow::Result;
use async_nats::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Default NATS relay for APN
pub const APN_NATS_RELAY: &str = "nats://nonlocal.info:4222";

/// APN Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APNBridgeConfig {
    /// NATS relay URL
    pub relay_url: String,
    /// Local node ID (for identification)
    pub node_id: String,
    /// Enable task distribution
    pub enable_distribution: bool,
}

impl Default for APNBridgeConfig {
    fn default() -> Self {
        Self {
            relay_url: APN_NATS_RELAY.to_string(),
            node_id: format!("pcg-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            enable_distribution: true,
        }
    }
}

/// Remote Vibe Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeNode {
    pub node_id: String,
    pub wallet_address: String,
    pub capabilities: Vec<String>,
    pub resources: NodeResources,
    pub reputation: f64,
    pub last_seen: i64,
    pub vibe_stake: f64,
}

/// Node resource capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeResources {
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_available: bool,
    pub gpu_model: Option<String>,
    pub bandwidth_mbps: f64,
}

/// Task for distribution to APN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributableTask {
    pub task_id: String,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub required_capabilities: Vec<String>,
    pub min_reputation: f64,
    pub vibe_reward: f64,
    pub deadline_unix: i64,
    pub priority: u8,
}

/// Task result from remote node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub worker_node: String,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub vibe_earned: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
    Timeout,
}

/// APN Bridge service
pub struct APNBridge {
    config: APNBridgeConfig,
    client: Option<Client>,
    /// Known Vibe Nodes (node_id -> VibeNode)
    nodes: Arc<RwLock<HashMap<String, VibeNode>>>,
    /// Pending tasks (task_id -> DistributableTask)
    pending_tasks: Arc<RwLock<HashMap<String, DistributableTask>>>,
    /// Task results (task_id -> TaskResult)
    results: Arc<RwLock<HashMap<String, TaskResult>>>,
    /// Total Vibe distributed
    vibe_distributed: Arc<RwLock<f64>>,
}

impl APNBridge {
    /// Create a new APN Bridge
    pub fn new(config: APNBridgeConfig) -> Self {
        Self {
            config,
            client: None,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            pending_tasks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            vibe_distributed: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Connect to the APN relay
    pub async fn connect(&mut self) -> Result<()> {
        let client = async_nats::connect(&self.config.relay_url).await?;
        tracing::info!("APNBridge connected to {}", self.config.relay_url);
        self.client = Some(client);

        // Subscribe to discovery channel
        self.subscribe_to_discovery().await?;

        Ok(())
    }

    /// Subscribe to node discovery
    async fn subscribe_to_discovery(&self) -> Result<()> {
        let client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let mut subscriber = client.subscribe("apn.discovery").await?;

        let nodes = self.nodes.clone();

        tokio::spawn(async move {
            while let Some(msg) = subscriber.next().await {
                if let Ok(announcement) = serde_json::from_slice::<PeerAnnouncement>(&msg.payload) {
                    let node = VibeNode {
                        node_id: announcement.node_id.clone(),
                        wallet_address: announcement.wallet_address,
                        capabilities: announcement.capabilities,
                        resources: announcement.resources.unwrap_or_default(),
                        reputation: 100.0, // Default reputation
                        last_seen: chrono::Utc::now().timestamp(),
                        vibe_stake: 0.0,
                    };

                    nodes.write().await.insert(announcement.node_id, node);
                }
            }
        });

        Ok(())
    }

    /// Distribute a task to the network
    pub async fn distribute_task(&self, task: DistributableTask) -> Result<()> {
        let client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Find eligible nodes
        let nodes = self.nodes.read().await;
        let eligible: Vec<_> = nodes.values()
            .filter(|n| {
                n.reputation >= task.min_reputation &&
                task.required_capabilities.iter().all(|cap| n.capabilities.contains(cap))
            })
            .collect();

        if eligible.is_empty() {
            tracing::warn!("No eligible nodes for task {}", task.task_id);
            return Ok(());
        }

        // Publish task to network
        let payload = serde_json::to_vec(&task)?;
        client.publish("apn.tasks", payload.into()).await?;

        // Store in pending
        self.pending_tasks.write().await.insert(task.task_id.clone(), task);

        tracing::info!("Task distributed to {} eligible nodes", eligible.len());
        Ok(())
    }

    /// Get all known nodes
    pub async fn get_nodes(&self) -> Vec<VibeNode> {
        self.nodes.read().await.values().cloned().collect()
    }

    /// Get node count
    pub async fn node_count(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Get pending task count
    pub async fn pending_task_count(&self) -> usize {
        self.pending_tasks.read().await.len()
    }

    /// Get total Vibe distributed
    pub async fn total_vibe_distributed(&self) -> f64 {
        *self.vibe_distributed.read().await
    }

    /// Record task completion and distribute Vibe
    pub async fn record_completion(&self, result: TaskResult) -> Result<()> {
        if result.status == TaskStatus::Completed {
            let mut vibe = self.vibe_distributed.write().await;
            *vibe += result.vibe_earned;
        }

        self.results.write().await.insert(result.task_id.clone(), result);
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Get configuration
    pub fn config(&self) -> &APNBridgeConfig {
        &self.config
    }
}

/// Peer announcement message (matches APN format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PeerAnnouncement {
    pub node_id: String,
    pub wallet_address: String,
    pub capabilities: Vec<String>,
    pub timestamp: String,
    #[serde(default)]
    pub resources: Option<NodeResources>,
}

use futures::StreamExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = APNBridgeConfig::default();
        assert!(config.node_id.starts_with("pcg-"));
        assert!(config.enable_distribution);
    }

    #[test]
    fn test_task_status() {
        let status = TaskStatus::Completed;
        assert_eq!(status, TaskStatus::Completed);
    }
}

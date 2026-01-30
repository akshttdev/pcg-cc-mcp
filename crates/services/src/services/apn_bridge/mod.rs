//! APN Bridge - Integration between PCG Dashboard and Alpha Protocol Network
//!
//! This module provides the bridge layer that connects the workflow execution
//! system to the distributed mesh network, enabling:
//! - Task distribution to remote nodes
//! - Execution log streaming from remote executions
//! - Resource accounting (bandwidth, compute)
//! - Economic settlement (Vibe tokens)

mod types;
mod task_distributor;
mod execution_relay;
mod resource_accounting;

pub use types::*;
pub use task_distributor::TaskDistributor;
pub use execution_relay::ExecutionRelay;
pub use resource_accounting::ResourceAccounting;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// The main APN Bridge service that coordinates all mesh operations
pub struct APNBridge {
    /// Task distribution service
    task_distributor: Arc<TaskDistributor>,
    /// Execution relay for remote log streaming
    execution_relay: Arc<ExecutionRelay>,
    /// Resource accounting
    resource_accounting: Arc<ResourceAccounting>,
    /// Event broadcaster for UI updates
    event_tx: broadcast::Sender<APNEvent>,
    /// Connection state
    state: Arc<RwLock<BridgeState>>,
}

#[derive(Debug, Clone, Default)]
pub struct BridgeState {
    pub connected: bool,
    pub node_id: Option<String>,
    pub peer_count: usize,
    pub vibe_balance: f64,
    pub active_distributed_tasks: usize,
    pub active_received_tasks: usize,
}

impl APNBridge {
    /// Create a new APN Bridge
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            task_distributor: Arc::new(TaskDistributor::new()),
            execution_relay: Arc::new(ExecutionRelay::new()),
            resource_accounting: Arc::new(ResourceAccounting::new()),
            event_tx,
            state: Arc::new(RwLock::new(BridgeState::default())),
        }
    }

    /// Subscribe to APN events
    pub fn subscribe(&self) -> broadcast::Receiver<APNEvent> {
        self.event_tx.subscribe()
    }

    /// Get current bridge state
    pub async fn state(&self) -> BridgeState {
        self.state.read().await.clone()
    }

    /// Distribute a task to the mesh network
    pub async fn distribute_task(&self, request: TaskDistributionRequest) -> anyhow::Result<TaskDistributionResult> {
        let result = self.task_distributor.distribute(request.clone()).await?;

        // Record transaction
        self.resource_accounting.record_task_distributed(&request, &result).await;

        // Emit event
        let _ = self.event_tx.send(APNEvent::TaskDistributed {
            task_id: request.task_id,
            target_node: result.assigned_node.clone(),
            reward_vibe: request.reward_vibe,
        });

        // Update state
        {
            let mut state = self.state.write().await;
            state.active_distributed_tasks += 1;
        }

        Ok(result)
    }

    /// Handle an incoming task from the network
    pub async fn receive_task(&self, task: IncomingTask) -> anyhow::Result<()> {
        // Update state
        {
            let mut state = self.state.write().await;
            state.active_received_tasks += 1;
        }

        // Emit event
        let _ = self.event_tx.send(APNEvent::TaskReceived {
            task_id: task.task_id,
            from_node: task.from_node.clone(),
            reward_vibe: task.reward_vibe,
        });

        Ok(())
    }

    /// Report task completion
    pub async fn complete_task(&self, task_id: Uuid, success: bool, vibe_earned: f64) -> anyhow::Result<()> {
        // Update balance
        self.resource_accounting.credit_vibe(vibe_earned).await;

        // Update state
        {
            let mut state = self.state.write().await;
            if state.active_received_tasks > 0 {
                state.active_received_tasks -= 1;
            }
            state.vibe_balance += vibe_earned;
        }

        // Emit event
        let _ = self.event_tx.send(APNEvent::TaskCompleted {
            task_id,
            success,
            vibe_earned,
        });

        Ok(())
    }

    /// Get the task distributor
    pub fn task_distributor(&self) -> &Arc<TaskDistributor> {
        &self.task_distributor
    }

    /// Get the execution relay
    pub fn execution_relay(&self) -> &Arc<ExecutionRelay> {
        &self.execution_relay
    }

    /// Get resource accounting
    pub fn resource_accounting(&self) -> &Arc<ResourceAccounting> {
        &self.resource_accounting
    }

    /// Get recent transactions for the UI
    pub async fn recent_transactions(&self) -> Vec<TransactionLog> {
        self.resource_accounting.recent_transactions().await
    }

    /// Get current vibe balance
    pub async fn vibe_balance(&self) -> f64 {
        self.resource_accounting.balance().await
    }
}

impl Default for APNBridge {
    fn default() -> Self {
        Self::new()
    }
}

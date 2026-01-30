//! Resource Accounting - Tracks bandwidth, compute, and Vibe token economics
//!
//! Maintains records of resource contributions and consumption,
//! and manages the economic settlement via Vibe tokens.

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

use super::types::*;

/// Resource accounting service
pub struct ResourceAccounting {
    /// Current Vibe balance
    balance: Arc<RwLock<f64>>,
    /// Transaction history
    transactions: Arc<RwLock<Vec<TransactionLog>>>,
    /// Bandwidth stats
    bandwidth_stats: Arc<RwLock<BandwidthStats>>,
    /// Compute stats
    compute_stats: Arc<RwLock<ComputeStats>>,
    /// Settings
    settings: AccountingSettings,
}

#[derive(Debug, Clone, Default)]
pub struct BandwidthStats {
    pub total_contributed_bytes: u64,
    pub total_consumed_bytes: u64,
    pub current_upload_rate: f64,   // bytes/sec
    pub current_download_rate: f64, // bytes/sec
}

#[derive(Debug, Clone, Default)]
pub struct ComputeStats {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_cpu_seconds: f64,
    pub total_memory_gb_seconds: f64,
}

#[derive(Debug, Clone)]
pub struct AccountingSettings {
    /// Max transactions to keep in memory
    pub max_transaction_history: usize,
    /// Vibe rate per GB of bandwidth relayed
    pub bandwidth_vibe_per_gb: f64,
    /// Base Vibe rate per task completed
    pub task_base_vibe: f64,
}

impl Default for AccountingSettings {
    fn default() -> Self {
        Self {
            max_transaction_history: 1000,
            bandwidth_vibe_per_gb: 0.01,
            task_base_vibe: 10.0,
        }
    }
}

impl ResourceAccounting {
    pub fn new() -> Self {
        Self {
            balance: Arc::new(RwLock::new(0.0)),
            transactions: Arc::new(RwLock::new(Vec::new())),
            bandwidth_stats: Arc::new(RwLock::new(BandwidthStats::default())),
            compute_stats: Arc::new(RwLock::new(ComputeStats::default())),
            settings: AccountingSettings::default(),
        }
    }

    pub fn with_settings(settings: AccountingSettings) -> Self {
        Self {
            balance: Arc::new(RwLock::new(0.0)),
            transactions: Arc::new(RwLock::new(Vec::new())),
            bandwidth_stats: Arc::new(RwLock::new(BandwidthStats::default())),
            compute_stats: Arc::new(RwLock::new(ComputeStats::default())),
            settings,
        }
    }

    /// Get current Vibe balance
    pub async fn balance(&self) -> f64 {
        *self.balance.read().await
    }

    /// Credit Vibe to the account
    pub async fn credit_vibe(&self, amount: f64) {
        let mut balance = self.balance.write().await;
        *balance += amount;

        // Record transaction
        self.record_transaction(TransactionLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            tx_type: TransactionType::VibeEarned,
            description: format!("Earned {:.2} VIBE", amount),
            vibe_amount: Some(amount),
            peer_node: None,
            task_id: None,
        }).await;
    }

    /// Debit Vibe from the account
    pub async fn debit_vibe(&self, amount: f64) -> anyhow::Result<()> {
        let mut balance = self.balance.write().await;

        if *balance < amount {
            return Err(anyhow::anyhow!("Insufficient Vibe balance"));
        }

        *balance -= amount;

        // Record transaction
        drop(balance); // Release lock before recording
        self.record_transaction(TransactionLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            tx_type: TransactionType::VibeSpent,
            description: format!("Spent {:.2} VIBE", amount),
            vibe_amount: Some(-amount),
            peer_node: None,
            task_id: None,
        }).await;

        Ok(())
    }

    /// Record a task distribution
    pub async fn record_task_distributed(
        &self,
        request: &TaskDistributionRequest,
        result: &TaskDistributionResult,
    ) {
        self.record_transaction(TransactionLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            tx_type: TransactionType::TaskDistributed,
            description: format!("Task distributed to {}", result.assigned_node),
            vibe_amount: Some(-result.agreed_reward),
            peer_node: Some(result.assigned_node.clone()),
            task_id: Some(request.task_id),
        }).await;
    }

    /// Record a task received
    pub async fn record_task_received(&self, task: &IncomingTask) {
        self.record_transaction(TransactionLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            tx_type: TransactionType::TaskReceived,
            description: format!("Task received from {}", task.from_node),
            vibe_amount: Some(task.reward_vibe),
            peer_node: Some(task.from_node.clone()),
            task_id: Some(task.task_id),
        }).await;
    }

    /// Record task completion
    pub async fn record_task_completed(
        &self,
        task_id: Uuid,
        executor_node: &str,
        success: bool,
        vibe_earned: f64,
    ) {
        let tx_type = if success {
            TransactionType::ExecutionCompleted
        } else {
            TransactionType::ExecutionFailed
        };

        let description = if success {
            format!("Task completed successfully, earned {:.2} VIBE", vibe_earned)
        } else {
            "Task execution failed".to_string()
        };

        self.record_transaction(TransactionLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            tx_type,
            description,
            vibe_amount: if success { Some(vibe_earned) } else { None },
            peer_node: Some(executor_node.to_string()),
            task_id: Some(task_id),
        }).await;

        // Update compute stats
        {
            let mut stats = self.compute_stats.write().await;
            if success {
                stats.tasks_completed += 1;
            } else {
                stats.tasks_failed += 1;
            }
        }
    }

    /// Record bandwidth contribution
    pub async fn record_bandwidth_contribution(
        &self,
        bytes: u64,
        peer_node: &str,
        purpose: &str,
    ) {
        // Calculate Vibe earned
        let gb = bytes as f64 / 1_073_741_824.0;
        let vibe_earned = gb * self.settings.bandwidth_vibe_per_gb;

        // Credit balance
        {
            let mut balance = self.balance.write().await;
            *balance += vibe_earned;
        }

        // Update bandwidth stats
        {
            let mut stats = self.bandwidth_stats.write().await;
            stats.total_contributed_bytes += bytes;
        }

        self.record_transaction(TransactionLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            tx_type: TransactionType::BandwidthContributed,
            description: format!("Relayed {}MB for {}", bytes / 1_048_576, purpose),
            vibe_amount: Some(vibe_earned),
            peer_node: Some(peer_node.to_string()),
            task_id: None,
        }).await;
    }

    /// Record a transaction
    async fn record_transaction(&self, tx: TransactionLog) {
        let mut transactions = self.transactions.write().await;
        transactions.push(tx);

        // Trim to max history
        if transactions.len() > self.settings.max_transaction_history {
            transactions.remove(0);
        }
    }

    /// Get recent transactions
    pub async fn recent_transactions(&self) -> Vec<TransactionLog> {
        let transactions = self.transactions.read().await;
        transactions.clone()
    }

    /// Get recent transactions (limited)
    pub async fn recent_transactions_limited(&self, limit: usize) -> Vec<TransactionLog> {
        let transactions = self.transactions.read().await;
        transactions
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get bandwidth stats
    pub async fn bandwidth_stats(&self) -> BandwidthStats {
        self.bandwidth_stats.read().await.clone()
    }

    /// Get compute stats
    pub async fn compute_stats(&self) -> ComputeStats {
        self.compute_stats.read().await.clone()
    }
}

impl Default for ResourceAccounting {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vibe_balance() {
        let accounting = ResourceAccounting::new();

        assert_eq!(accounting.balance().await, 0.0);

        accounting.credit_vibe(100.0).await;
        assert_eq!(accounting.balance().await, 100.0);

        accounting.debit_vibe(30.0).await.unwrap();
        assert_eq!(accounting.balance().await, 70.0);

        // Try to overdraw
        let result = accounting.debit_vibe(100.0).await;
        assert!(result.is_err());
        assert_eq!(accounting.balance().await, 70.0);
    }

    #[tokio::test]
    async fn test_transaction_history() {
        let settings = AccountingSettings {
            max_transaction_history: 5,
            ..Default::default()
        };
        let accounting = ResourceAccounting::with_settings(settings);

        // Add 10 transactions
        for i in 0..10 {
            accounting.credit_vibe(i as f64).await;
        }

        // Should only keep last 5
        let transactions = accounting.recent_transactions().await;
        assert_eq!(transactions.len(), 5);
    }

    #[tokio::test]
    async fn test_bandwidth_contribution() {
        let accounting = ResourceAccounting::new();

        // Contribute 1 GB
        accounting.record_bandwidth_contribution(
            1_073_741_824,
            "peer_node",
            "task_relay"
        ).await;

        let stats = accounting.bandwidth_stats().await;
        assert_eq!(stats.total_contributed_bytes, 1_073_741_824);

        // Should have earned 0.01 VIBE
        assert!((accounting.balance().await - 0.01).abs() < 0.001);
    }
}

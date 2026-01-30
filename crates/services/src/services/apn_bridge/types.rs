//! APN Bridge Types - Data structures for mesh network integration

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Request to distribute a task to the mesh network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistributionRequest {
    pub task_id: Uuid,
    pub task_attempt_id: Uuid,
    pub executor_profile: String,
    pub prompt: String,
    pub project_id: Uuid,
    pub project_path: Option<String>,
    pub resource_requirements: ResourceRequirements,
    pub reward_vibe: f64,
}

/// Resource requirements for task execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    pub min_cpu_cores: Option<u32>,
    pub min_memory_gb: Option<u32>,
    pub min_storage_gb: Option<u32>,
    pub required_capabilities: Vec<String>,
    pub max_execution_time_secs: Option<u64>,
}

/// Result of task distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistributionResult {
    pub task_id: Uuid,
    pub assigned_node: String,
    pub estimated_start_time: Option<DateTime<Utc>>,
    pub agreed_reward: f64,
}

/// An incoming task received from the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingTask {
    pub task_id: Uuid,
    pub task_attempt_id: Uuid,
    pub from_node: String,
    pub executor_profile: String,
    pub prompt: String,
    pub project_context: Option<String>,
    pub reward_vibe: f64,
    pub received_at: DateTime<Utc>,
}

/// Execution progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub execution_process_id: Uuid,
    pub task_id: Uuid,
    pub stage: ExecutionStage,
    pub progress_percent: u8,
    pub current_action: String,
    pub files_modified: u32,
    pub timestamp: DateTime<Utc>,
}

/// Execution stages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStage {
    Setup,
    Coding,
    Testing,
    Review,
    Cleanup,
    Completed,
    Failed,
}

impl std::fmt::Display for ExecutionStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStage::Setup => write!(f, "setup"),
            ExecutionStage::Coding => write!(f, "coding"),
            ExecutionStage::Testing => write!(f, "testing"),
            ExecutionStage::Review => write!(f, "review"),
            ExecutionStage::Cleanup => write!(f, "cleanup"),
            ExecutionStage::Completed => write!(f, "completed"),
            ExecutionStage::Failed => write!(f, "failed"),
        }
    }
}

/// Log chunk from remote execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogChunk {
    pub execution_process_id: Uuid,
    pub log_type: LogType,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Types of execution logs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogType {
    Stdout,
    Stderr,
    Agent,
    System,
}

/// Execution completion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: Uuid,
    pub execution_process_id: Uuid,
    pub executor_node: String,
    pub success: bool,
    pub files_modified: u32,
    pub files_created: u32,
    pub files_deleted: u32,
    pub execution_time_ms: u64,
    pub git_diff: Option<String>,
    pub error: Option<String>,
    pub completed_at: DateTime<Utc>,
}

/// Transaction log entry for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub tx_type: TransactionType,
    pub description: String,
    pub vibe_amount: Option<f64>,
    pub peer_node: Option<String>,
    pub task_id: Option<Uuid>,
}

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    TaskDistributed,
    TaskReceived,
    ExecutionStarted,
    ExecutionCompleted,
    ExecutionFailed,
    BandwidthContributed,
    VibeEarned,
    VibeSpent,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::TaskDistributed => write!(f, "task_distributed"),
            TransactionType::TaskReceived => write!(f, "task_received"),
            TransactionType::ExecutionStarted => write!(f, "execution_started"),
            TransactionType::ExecutionCompleted => write!(f, "execution_completed"),
            TransactionType::ExecutionFailed => write!(f, "execution_failed"),
            TransactionType::BandwidthContributed => write!(f, "bandwidth_contributed"),
            TransactionType::VibeEarned => write!(f, "vibe_earned"),
            TransactionType::VibeSpent => write!(f, "vibe_spent"),
        }
    }
}

/// Events emitted by the APN Bridge
#[derive(Debug, Clone)]
pub enum APNEvent {
    /// Task distributed to remote node
    TaskDistributed {
        task_id: Uuid,
        target_node: String,
        reward_vibe: f64,
    },
    /// Task received from network
    TaskReceived {
        task_id: Uuid,
        from_node: String,
        reward_vibe: f64,
    },
    /// Execution started
    ExecutionStarted {
        task_id: Uuid,
        execution_process_id: Uuid,
        node: String,
    },
    /// Execution progress update
    ExecutionProgress {
        task_id: Uuid,
        progress: ExecutionProgress,
    },
    /// Execution completed
    TaskCompleted {
        task_id: Uuid,
        success: bool,
        vibe_earned: f64,
    },
    /// Peer connected
    PeerConnected {
        node_id: String,
        capabilities: Vec<String>,
    },
    /// Peer disconnected
    PeerDisconnected {
        node_id: String,
    },
    /// Vibe balance changed
    VibeBalanceChanged {
        new_balance: f64,
        delta: f64,
    },
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
    pub capabilities: Vec<String>,
    pub reputation: f64,
    pub latency_ms: Option<u64>,
    pub available_bandwidth_mbps: Option<f64>,
    pub last_seen: DateTime<Utc>,
}

/// Capability flags for nodes
pub mod capabilities {
    pub const COMPUTE: &str = "compute";
    pub const RELAY: &str = "relay";
    pub const STORAGE: &str = "storage";
    pub const GPU: &str = "gpu";
    pub const HIGH_MEMORY: &str = "high_memory";
}

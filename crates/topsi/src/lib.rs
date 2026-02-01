//! # Topsi - Topological Super Intelligence
//!
//! A master controller that reasons about the project ecosystem as a living topology
//! of agents, tasks, resources, and connections. Topsi doesn't just manage tasks -
//! it maintains and sculpts the topology of the project.
//!
//! ## Core Capabilities
//!
//! - **Global Awareness**: Maintains a live topology graph of nodes, edges, and clusters
//! - **Pattern Detection**: Identifies bottlenecks, holes, and emerging patterns
//! - **Dynamic Re-Routing**: Finds alternate paths when routes degrade
//! - **Collective Intelligence**: Forms and dissolves agent teams dynamically
//! - **Policy Enforcement**: Uses topological invariants to enforce rules
//! - **Graceful Scaling**: Seamlessly integrates new agents and resources

pub mod agent;
pub mod config;
pub mod context;
pub mod prioritization;
pub mod topology;
pub mod tools;

pub use agent::{
    TopsiAgent, TopsiRequest, TopsiRequestType,
    access_control::{AccessControl, AccessScope, UserContext, ProjectAccess, ProjectRole},
};
pub use config::TopsiConfig;
pub use context::TopologyContext;
pub use topology::{
    graph::{TopologyGraph, GraphNode, GraphEdge, ClusterInfo, RouteInfo, ProjectTopology},
    engine::TopologyEngine,
    patterns::PatternDetector,
    routing::RoutePlanner,
    clusters::ClusterManager,
    invariants::{InvariantChecker, InvariantViolation},
    voice::VoiceTopology,
};
pub use prioritization::{
    Goal as PrioritizationGoal, GoalState, GoalType,
    ExpectedFreeEnergy, EFECalculator,
    PriorityScore, PriorityCalculator, PriorityLevel,
    Recommendation, PriorityRecommender,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Main error types for Topsi operations
#[derive(Debug, thiserror::Error)]
pub enum TopsiError {
    #[error("Topology error: {0}")]
    TopologyError(String),

    #[error("Routing error: {0}")]
    RoutingError(String),

    #[error("No path found from {from} to {to}")]
    NoPathFound { from: Uuid, to: Uuid },

    #[error("Node not found: {0}")]
    NodeNotFound(Uuid),

    #[error("Edge not found: {0}")]
    EdgeNotFound(Uuid),

    #[error("Cluster error: {0}")]
    ClusterError(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Pattern detection error: {0}")]
    PatternError(String),

    #[error("Tool execution error: {0}")]
    ToolError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("LLM error: {0}")]
    LLMError(String),
}

pub type Result<T> = std::result::Result<T, TopsiError>;

/// Response from Topsi processing
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiResponse {
    /// The text response
    pub message: String,
    /// Tool calls made during processing
    pub tool_calls: Vec<ToolCallResult>,
    /// Topology changes made
    pub topology_changes: Vec<TopologyChange>,
    /// Current topology summary
    pub topology_summary: Option<TopologySummary>,
    /// Detected issues
    pub issues: Vec<DetectedIssue>,
    /// Token usage
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Result of a tool call
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: serde_json::Value,
    pub success: bool,
}

/// A change made to the topology
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TopologyChange {
    NodeAdded { node_id: Uuid, node_type: String },
    NodeRemoved { node_id: Uuid },
    NodeStatusChanged { node_id: Uuid, old_status: String, new_status: String },
    EdgeAdded { edge_id: Uuid, from: Uuid, to: Uuid },
    EdgeRemoved { edge_id: Uuid },
    EdgeStatusChanged { edge_id: Uuid, old_status: String, new_status: String },
    ClusterFormed { cluster_id: Uuid, name: String, node_count: usize },
    ClusterDissolved { cluster_id: Uuid },
    RouteCreated { route_id: Uuid, goal: String, path_length: usize },
    RouteCompleted { route_id: Uuid },
    RouteFailed { route_id: Uuid, reason: String },
}

/// Summary of the current topology state
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopologySummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub cluster_count: usize,
    pub active_routes: usize,
    pub unresolved_issues: usize,
    pub nodes_by_type: Vec<(String, usize)>,
    pub edges_by_type: Vec<(String, usize)>,
    pub health_score: f64,
}

impl Default for TopologySummary {
    fn default() -> Self {
        Self {
            node_count: 0,
            edge_count: 0,
            cluster_count: 0,
            active_routes: 0,
            unresolved_issues: 0,
            nodes_by_type: Vec::new(),
            edges_by_type: Vec::new(),
            health_score: 1.0,
        }
    }
}

/// A detected issue in the topology
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DetectedIssue {
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub affected_nodes: Vec<Uuid>,
    pub suggested_action: Option<String>,
}

/// Goal for route planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Goal {
    /// Execute a specific task
    ExecuteTask(Uuid),
    /// Reach a capability from current position
    ReachCapability(String),
    /// Connect two nodes
    ConnectNodes { from: Uuid, to: Uuid },
    /// Find an agent with specific capabilities
    FindAgent(Vec<String>),
    /// Execute a workflow
    ExecuteWorkflow(String),
}

/// Initialize Topsi with configuration
pub async fn initialize_topsi(config: TopsiConfig) -> Result<TopsiAgent> {
    tracing::info!("Initializing Topsi (Topological Super Intelligence)...");

    let agent = TopsiAgent::new(config).await?;

    tracing::info!("Topsi initialized successfully");
    Ok(agent)
}

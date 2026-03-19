//! Topology module - Graph model and algorithms for project topology

pub mod clusters;
pub mod engine;
pub mod graph;
pub mod invariants;
pub mod patterns;
pub mod routing;
pub mod voice;

pub use clusters::ClusterManager;
pub use engine::{Path, TopologyEngine};
pub use graph::{ClusterInfo, GraphEdge, GraphNode, ProjectTopology, RouteInfo, TopologyGraph};
pub use invariants::InvariantChecker;
pub use patterns::PatternDetector;
pub use routing::RoutePlanner;
pub use voice::VoiceTopology;

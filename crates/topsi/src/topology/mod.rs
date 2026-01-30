//! Topology module - Graph model and algorithms for project topology

pub mod graph;
pub mod engine;
pub mod patterns;
pub mod routing;
pub mod clusters;
pub mod invariants;

pub use graph::{ProjectTopology, TopologyGraph, GraphNode, GraphEdge, ClusterInfo, RouteInfo};
pub use engine::{TopologyEngine, Path};
pub use patterns::PatternDetector;
pub use routing::RoutePlanner;
pub use clusters::ClusterManager;
pub use invariants::InvariantChecker;

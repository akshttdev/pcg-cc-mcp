//! Graph model for project topology

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// A node in the topology graph
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: Uuid,
    pub node_type: String,
    pub ref_id: String,
    pub capabilities: Vec<String>,
    pub status: String,
    pub weight: f64,
    pub metadata: Option<serde_json::Value>,
}

impl GraphNode {
    pub fn new(
        id: Uuid,
        node_type: impl Into<String>,
        ref_id: impl Into<String>,
    ) -> Self {
        Self {
            id,
            node_type: node_type.into(),
            ref_id: ref_id.into(),
            capabilities: Vec::new(),
            status: "active".to_string(),
            weight: 1.0,
            metadata: None,
        }
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

/// An edge in the topology graph
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub edge_type: String,
    pub weight: f64,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
}

impl GraphEdge {
    pub fn new(
        id: Uuid,
        from_node_id: Uuid,
        to_node_id: Uuid,
        edge_type: impl Into<String>,
    ) -> Self {
        Self {
            id,
            from_node_id,
            to_node_id,
            edge_type: edge_type.into(),
            weight: 1.0,
            status: "active".to_string(),
            metadata: None,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
}

/// Information about a cluster
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClusterInfo {
    pub id: Uuid,
    pub name: String,
    pub purpose: Option<String>,
    pub node_ids: Vec<Uuid>,
    pub leader_node_id: Option<Uuid>,
    pub is_active: bool,
    pub formed_at: DateTime<Utc>,
}

impl ClusterInfo {
    pub fn new(id: Uuid, name: impl Into<String>, node_ids: Vec<Uuid>) -> Self {
        Self {
            id,
            name: name.into(),
            purpose: None,
            node_ids,
            leader_node_id: None,
            is_active: true,
            formed_at: Utc::now(),
        }
    }

    pub fn with_purpose(mut self, purpose: impl Into<String>) -> Self {
        self.purpose = Some(purpose.into());
        self
    }

    pub fn with_leader(mut self, leader_id: Uuid) -> Self {
        self.leader_node_id = Some(leader_id);
        self
    }
}

/// Information about a route
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RouteInfo {
    pub id: Uuid,
    pub goal: String,
    pub path: Vec<Uuid>,
    pub edges: Vec<Uuid>,
    pub total_weight: f64,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl RouteInfo {
    pub fn new(id: Uuid, goal: impl Into<String>, path: Vec<Uuid>, edges: Vec<Uuid>) -> Self {
        let total_weight = 0.0; // Will be calculated
        Self {
            id,
            goal: goal.into(),
            path,
            edges,
            total_weight,
            status: "planned".to_string(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.total_weight = weight;
        self
    }
}

/// The topology graph structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologyGraph {
    pub nodes: IndexMap<Uuid, GraphNode>,
    pub edges: IndexMap<Uuid, GraphEdge>,
}

impl TopologyGraph {
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            edges: IndexMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id, node);
    }

    /// Remove a node and its edges
    pub fn remove_node(&mut self, node_id: Uuid) -> Option<GraphNode> {
        // Remove all edges involving this node
        self.edges.retain(|_, e| e.from_node_id != node_id && e.to_node_id != node_id);
        self.nodes.swap_remove(&node_id)
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.insert(edge.id, edge);
    }

    /// Remove an edge
    pub fn remove_edge(&mut self, edge_id: Uuid) -> Option<GraphEdge> {
        self.edges.swap_remove(&edge_id)
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: Uuid) -> Option<&GraphNode> {
        self.nodes.get(&node_id)
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, node_id: Uuid) -> Option<&mut GraphNode> {
        self.nodes.get_mut(&node_id)
    }

    /// Get an edge by ID
    pub fn get_edge(&self, edge_id: Uuid) -> Option<&GraphEdge> {
        self.edges.get(&edge_id)
    }

    /// Get all edges from a node
    pub fn edges_from(&self, node_id: Uuid) -> Vec<&GraphEdge> {
        self.edges
            .values()
            .filter(|e| e.from_node_id == node_id && e.is_active())
            .collect()
    }

    /// Get all edges to a node
    pub fn edges_to(&self, node_id: Uuid) -> Vec<&GraphEdge> {
        self.edges
            .values()
            .filter(|e| e.to_node_id == node_id && e.is_active())
            .collect()
    }

    /// Get neighbors of a node (nodes reachable via outgoing edges)
    pub fn neighbors(&self, node_id: Uuid) -> Vec<Uuid> {
        self.edges_from(node_id)
            .iter()
            .map(|e| e.to_node_id)
            .collect()
    }

    /// Get predecessors of a node (nodes with edges to this node)
    pub fn predecessors(&self, node_id: Uuid) -> Vec<Uuid> {
        self.edges_to(node_id)
            .iter()
            .map(|e| e.from_node_id)
            .collect()
    }

    /// Get nodes by type
    pub fn nodes_of_type(&self, node_type: &str) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| n.node_type == node_type)
            .collect()
    }

    /// Get active nodes
    pub fn active_nodes(&self) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.is_active()).collect()
    }

    /// Get nodes with a specific capability
    pub fn nodes_with_capability(&self, capability: &str) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| n.is_active() && n.has_capability(capability))
            .collect()
    }

    /// Calculate in-degree (number of incoming edges)
    pub fn in_degree(&self, node_id: Uuid) -> usize {
        self.edges_to(node_id).len()
    }

    /// Calculate out-degree (number of outgoing edges)
    pub fn out_degree(&self, node_id: Uuid) -> usize {
        self.edges_from(node_id).len()
    }

    /// Check if an edge exists between two nodes
    pub fn has_edge(&self, from: Uuid, to: Uuid) -> bool {
        self.edges
            .values()
            .any(|e| e.from_node_id == from && e.to_node_id == to)
    }

    /// Find edge between two nodes
    pub fn find_edge(&self, from: Uuid, to: Uuid) -> Option<&GraphEdge> {
        self.edges
            .values()
            .find(|e| e.from_node_id == from && e.to_node_id == to)
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// The complete project topology including clusters and routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTopology {
    pub project_id: Uuid,
    pub graph: TopologyGraph,
    pub clusters: Vec<ClusterInfo>,
    pub routes: Vec<RouteInfo>,
    pub last_updated: DateTime<Utc>,
}

impl ProjectTopology {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project_id,
            graph: TopologyGraph::new(),
            clusters: Vec::new(),
            routes: Vec::new(),
            last_updated: Utc::now(),
        }
    }

    /// Update the last_updated timestamp
    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
    }

    /// Add a cluster
    pub fn add_cluster(&mut self, cluster: ClusterInfo) {
        self.clusters.push(cluster);
        self.touch();
    }

    /// Remove a cluster by ID
    pub fn remove_cluster(&mut self, cluster_id: Uuid) -> Option<ClusterInfo> {
        let idx = self.clusters.iter().position(|c| c.id == cluster_id)?;
        self.touch();
        Some(self.clusters.remove(idx))
    }

    /// Get a cluster by ID
    pub fn get_cluster(&self, cluster_id: Uuid) -> Option<&ClusterInfo> {
        self.clusters.iter().find(|c| c.id == cluster_id)
    }

    /// Get active clusters
    pub fn active_clusters(&self) -> Vec<&ClusterInfo> {
        self.clusters.iter().filter(|c| c.is_active).collect()
    }

    /// Add a route
    pub fn add_route(&mut self, route: RouteInfo) {
        self.routes.push(route);
        self.touch();
    }

    /// Get a route by ID
    pub fn get_route(&self, route_id: Uuid) -> Option<&RouteInfo> {
        self.routes.iter().find(|r| r.id == route_id)
    }

    /// Get a mutable route by ID
    pub fn get_route_mut(&mut self, route_id: Uuid) -> Option<&mut RouteInfo> {
        self.routes.iter_mut().find(|r| r.id == route_id)
    }

    /// Get executing routes
    pub fn executing_routes(&self) -> Vec<&RouteInfo> {
        self.routes.iter().filter(|r| r.status == "executing").collect()
    }
}

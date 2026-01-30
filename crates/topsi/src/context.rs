//! Topology-enhanced context building

use crate::{TopologySummary, DetectedIssue};
use crate::topology::{ProjectTopology, TopologyGraph};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Context for topology-aware operations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopologyContext {
    /// Project ID
    pub project_id: Uuid,
    /// Current topology summary
    pub summary: TopologySummary,
    /// Active issues
    pub issues: Vec<DetectedIssue>,
    /// Key nodes (most connected/important)
    pub key_nodes: Vec<NodeSummary>,
    /// Active clusters
    pub active_clusters: Vec<ClusterSummary>,
    /// Executing routes
    pub executing_routes: Vec<RouteSummary>,
    /// Recent changes
    pub recent_changes: Vec<String>,
}

/// Summary of a node for context
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NodeSummary {
    pub id: Uuid,
    pub node_type: String,
    pub ref_id: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub in_degree: usize,
    pub out_degree: usize,
}

/// Summary of a cluster for context
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClusterSummary {
    pub id: Uuid,
    pub name: String,
    pub purpose: Option<String>,
    pub node_count: usize,
    pub leader_id: Option<Uuid>,
}

/// Summary of a route for context
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RouteSummary {
    pub id: Uuid,
    pub goal: String,
    pub path_length: usize,
    pub status: String,
    pub progress: Option<f64>,
}

impl TopologyContext {
    /// Build context from a project topology
    pub fn from_topology(
        project_id: Uuid,
        topology: &ProjectTopology,
        issues: Vec<DetectedIssue>,
        max_nodes: usize,
    ) -> Self {
        let graph = &topology.graph;

        // Get summary
        let summary = TopologySummary {
            node_count: graph.nodes.len(),
            edge_count: graph.edges.len(),
            cluster_count: topology.clusters.len(),
            active_routes: topology.routes.iter().filter(|r| r.status == "executing").count(),
            unresolved_issues: issues.len(),
            nodes_by_type: Self::count_nodes_by_type(graph),
            edges_by_type: Self::count_edges_by_type(graph),
            health_score: Self::calculate_health_score(graph, &issues),
        };

        // Get key nodes (sorted by connectivity)
        let mut key_nodes: Vec<NodeSummary> = graph.nodes.values()
            .map(|n| {
                let in_degree = graph.edges.values()
                    .filter(|e| e.to_node_id == n.id)
                    .count();
                let out_degree = graph.edges.values()
                    .filter(|e| e.from_node_id == n.id)
                    .count();
                NodeSummary {
                    id: n.id,
                    node_type: n.node_type.clone(),
                    ref_id: n.ref_id.clone(),
                    status: n.status.clone(),
                    capabilities: n.capabilities.clone(),
                    in_degree,
                    out_degree,
                }
            })
            .collect();

        // Sort by total degree descending
        key_nodes.sort_by(|a, b| {
            let a_total = a.in_degree + a.out_degree;
            let b_total = b.in_degree + b.out_degree;
            b_total.cmp(&a_total)
        });
        key_nodes.truncate(max_nodes);

        // Get active clusters
        let active_clusters: Vec<ClusterSummary> = topology.clusters.iter()
            .filter(|c| c.is_active)
            .map(|c| ClusterSummary {
                id: c.id,
                name: c.name.clone(),
                purpose: c.purpose.clone(),
                node_count: c.node_ids.len(),
                leader_id: c.leader_node_id,
            })
            .collect();

        // Get executing routes
        let executing_routes: Vec<RouteSummary> = topology.routes.iter()
            .filter(|r| r.status == "executing")
            .map(|r| RouteSummary {
                id: r.id,
                goal: r.goal.clone(),
                path_length: r.path.len(),
                status: r.status.clone(),
                progress: None, // Would need execution state to calculate
            })
            .collect();

        Self {
            project_id,
            summary,
            issues,
            key_nodes,
            active_clusters,
            executing_routes,
            recent_changes: Vec::new(),
        }
    }

    fn count_nodes_by_type(graph: &TopologyGraph) -> Vec<(String, usize)> {
        let mut counts = std::collections::HashMap::new();
        for node in graph.nodes.values() {
            *counts.entry(node.node_type.clone()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    fn count_edges_by_type(graph: &TopologyGraph) -> Vec<(String, usize)> {
        let mut counts = std::collections::HashMap::new();
        for edge in graph.edges.values() {
            *counts.entry(edge.edge_type.clone()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    fn calculate_health_score(graph: &TopologyGraph, issues: &[DetectedIssue]) -> f64 {
        if graph.nodes.is_empty() {
            return 1.0;
        }

        let mut score = 1.0;

        // Deduct for issues based on severity
        for issue in issues {
            match issue.severity.as_str() {
                "critical" => score -= 0.25,
                "error" => score -= 0.15,
                "warning" => score -= 0.05,
                _ => score -= 0.01,
            }
        }

        // Deduct for inactive/degraded nodes
        let inactive_count = graph.nodes.values()
            .filter(|n| n.status != "active")
            .count();
        let inactive_ratio = inactive_count as f64 / graph.nodes.len() as f64;
        score -= inactive_ratio * 0.3;

        // Deduct for degraded edges
        let degraded_edges = graph.edges.values()
            .filter(|e| e.status != "active")
            .count();
        if !graph.edges.is_empty() {
            let edge_ratio = degraded_edges as f64 / graph.edges.len() as f64;
            score -= edge_ratio * 0.2;
        }

        score.max(0.0).min(1.0)
    }

    /// Generate a text description of the context for LLM
    pub fn to_prompt_context(&self) -> String {
        let mut context = String::new();

        context.push_str(&format!(
            "## Current Topology State (Project: {})\n\n",
            self.project_id
        ));

        context.push_str(&format!(
            "**Health Score**: {:.0}%\n",
            self.summary.health_score * 100.0
        ));

        context.push_str(&format!(
            "**Nodes**: {} total ({} types)\n",
            self.summary.node_count,
            self.summary.nodes_by_type.len()
        ));
        for (node_type, count) in &self.summary.nodes_by_type {
            context.push_str(&format!("  - {}: {}\n", node_type, count));
        }

        context.push_str(&format!(
            "**Edges**: {} connections\n",
            self.summary.edge_count
        ));

        context.push_str(&format!(
            "**Active Clusters**: {}\n",
            self.summary.cluster_count
        ));
        for cluster in &self.active_clusters {
            context.push_str(&format!(
                "  - {} ({} nodes): {}\n",
                cluster.name,
                cluster.node_count,
                cluster.purpose.as_deref().unwrap_or("general")
            ));
        }

        context.push_str(&format!(
            "**Executing Routes**: {}\n",
            self.summary.active_routes
        ));
        for route in &self.executing_routes {
            context.push_str(&format!(
                "  - {}: {} steps, status: {}\n",
                route.goal, route.path_length, route.status
            ));
        }

        if !self.issues.is_empty() {
            context.push_str(&format!(
                "\n**Issues Detected**: {}\n",
                self.issues.len()
            ));
            for issue in &self.issues {
                context.push_str(&format!(
                    "  - [{}] {}: {}\n",
                    issue.severity.to_uppercase(),
                    issue.issue_type,
                    issue.description
                ));
                if let Some(action) = &issue.suggested_action {
                    context.push_str(&format!("    Suggested: {}\n", action));
                }
            }
        }

        context.push_str("\n**Key Nodes** (most connected):\n");
        for node in self.key_nodes.iter().take(10) {
            context.push_str(&format!(
                "  - {} ({}, {}): in={}, out={}, caps=[{}]\n",
                node.ref_id,
                node.node_type,
                node.status,
                node.in_degree,
                node.out_degree,
                node.capabilities.join(", ")
            ));
        }

        context
    }
}

/// Builder for topology context
pub struct TopologyContextBuilder {
    project_id: Uuid,
    include_issues: bool,
    include_clusters: bool,
    include_routes: bool,
    max_nodes: usize,
}

impl TopologyContextBuilder {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project_id,
            include_issues: true,
            include_clusters: true,
            include_routes: true,
            max_nodes: 50,
        }
    }

    pub fn include_issues(mut self, include: bool) -> Self {
        self.include_issues = include;
        self
    }

    pub fn include_clusters(mut self, include: bool) -> Self {
        self.include_clusters = include;
        self
    }

    pub fn include_routes(mut self, include: bool) -> Self {
        self.include_routes = include;
        self
    }

    pub fn max_nodes(mut self, max: usize) -> Self {
        self.max_nodes = max;
        self
    }

    pub fn build(self, topology: &ProjectTopology, issues: Vec<DetectedIssue>) -> TopologyContext {
        TopologyContext::from_topology(
            self.project_id,
            topology,
            if self.include_issues { issues } else { Vec::new() },
            self.max_nodes,
        )
    }
}

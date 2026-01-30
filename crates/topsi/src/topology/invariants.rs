//! Topological Invariants - Rules that must always hold true
//!
//! This module defines and enforces invariants across the topology,
//! ensuring consistency and preventing invalid states.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use super::graph::TopologyGraph;

/// A violation of a topological invariant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantViolation {
    pub invariant_name: String,
    pub severity: ViolationSeverity,
    pub message: String,
    pub affected_nodes: Vec<Uuid>,
    pub affected_edges: Vec<Uuid>,
    pub suggested_fix: Option<String>,
}

/// Severity levels for invariant violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Informational - doesn't break anything but worth noting
    Info,
    /// Warning - could cause issues, should be addressed
    Warning,
    /// Error - violates a required invariant, needs immediate attention
    Error,
    /// Critical - system is in an invalid state
    Critical,
}

/// Invariant checker for topology validation
pub struct InvariantChecker {
    /// Enable strict mode (all warnings become errors)
    strict_mode: bool,
}

impl Default for InvariantChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl InvariantChecker {
    /// Create a new invariant checker
    pub fn new() -> Self {
        Self {
            strict_mode: false,
        }
    }

    /// Enable strict mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Check all invariants against a topology graph
    pub fn check_all(&self, graph: &TopologyGraph) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        // Check built-in invariants
        violations.extend(self.check_no_orphan_edges(graph));
        violations.extend(self.check_no_self_loops(graph));
        violations.extend(self.check_node_status_consistency(graph));
        violations.extend(self.check_edge_weight_validity(graph));

        // In strict mode, upgrade warnings to errors
        if self.strict_mode {
            for v in &mut violations {
                if v.severity == ViolationSeverity::Warning {
                    v.severity = ViolationSeverity::Error;
                }
            }
        }

        violations
    }

    /// Check that all edges reference existing nodes
    fn check_no_orphan_edges(&self, graph: &TopologyGraph) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        let node_ids: HashSet<Uuid> = graph.nodes.keys().copied().collect();

        for (edge_id, edge) in &graph.edges {
            let mut orphan_refs = Vec::new();

            if !node_ids.contains(&edge.from_node_id) {
                orphan_refs.push(edge.from_node_id);
            }
            if !node_ids.contains(&edge.to_node_id) {
                orphan_refs.push(edge.to_node_id);
            }

            if !orphan_refs.is_empty() {
                violations.push(InvariantViolation {
                    invariant_name: "no_orphan_edges".to_string(),
                    severity: ViolationSeverity::Error,
                    message: format!(
                        "Edge {} references non-existent nodes: {:?}",
                        edge_id, orphan_refs
                    ),
                    affected_nodes: orphan_refs,
                    affected_edges: vec![*edge_id],
                    suggested_fix: Some("Remove the edge or create the missing nodes".to_string()),
                });
            }
        }

        violations
    }

    /// Check for self-referential edges
    fn check_no_self_loops(&self, graph: &TopologyGraph) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        for (edge_id, edge) in &graph.edges {
            if edge.from_node_id == edge.to_node_id {
                violations.push(InvariantViolation {
                    invariant_name: "no_self_loops".to_string(),
                    severity: ViolationSeverity::Warning,
                    message: format!("Edge {} is a self-loop on node {}", edge_id, edge.from_node_id),
                    affected_nodes: vec![edge.from_node_id],
                    affected_edges: vec![*edge_id],
                    suggested_fix: Some("Remove the self-referential edge".to_string()),
                });
            }
        }

        violations
    }

    /// Check that node statuses are valid
    fn check_node_status_consistency(&self, graph: &TopologyGraph) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        let valid_statuses = ["active", "idle", "busy", "degraded", "offline", "pending"];

        for (node_id, node) in &graph.nodes {
            if !valid_statuses.contains(&node.status.as_str()) {
                violations.push(InvariantViolation {
                    invariant_name: "valid_node_status".to_string(),
                    severity: ViolationSeverity::Warning,
                    message: format!(
                        "Node {} has invalid status: {}",
                        node_id, node.status
                    ),
                    affected_nodes: vec![*node_id],
                    affected_edges: vec![],
                    suggested_fix: Some(format!(
                        "Set status to one of: {:?}",
                        valid_statuses
                    )),
                });
            }
        }

        violations
    }

    /// Check that edge weights are valid
    fn check_edge_weight_validity(&self, graph: &TopologyGraph) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        for (edge_id, edge) in &graph.edges {
            if edge.weight < 0.0 {
                violations.push(InvariantViolation {
                    invariant_name: "positive_edge_weights".to_string(),
                    severity: ViolationSeverity::Error,
                    message: format!(
                        "Edge {} has negative weight: {}",
                        edge_id, edge.weight
                    ),
                    affected_nodes: vec![edge.from_node_id, edge.to_node_id],
                    affected_edges: vec![*edge_id],
                    suggested_fix: Some("Set weight to a non-negative value".to_string()),
                });
            }

            if edge.weight > 1000.0 {
                violations.push(InvariantViolation {
                    invariant_name: "reasonable_edge_weights".to_string(),
                    severity: ViolationSeverity::Info,
                    message: format!(
                        "Edge {} has unusually high weight: {}",
                        edge_id, edge.weight
                    ),
                    affected_nodes: vec![edge.from_node_id, edge.to_node_id],
                    affected_edges: vec![*edge_id],
                    suggested_fix: Some("Consider if this weight is intentional".to_string()),
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn create_empty_graph() -> TopologyGraph {
        TopologyGraph {
            nodes: IndexMap::new(),
            edges: IndexMap::new(),
        }
    }

    #[test]
    fn test_empty_graph_has_no_violations() {
        let checker = InvariantChecker::new();
        let graph = create_empty_graph();
        let violations = checker.check_all(&graph);
        assert!(violations.is_empty());
    }
}

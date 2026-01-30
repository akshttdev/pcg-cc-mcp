//! Pattern detection - Find bottlenecks, holes, and emerging patterns in the topology

use super::graph::{ProjectTopology, TopologyGraph};
use super::engine::TopologyEngine;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use std::collections::HashMap;

/// A bottleneck in the topology (overloaded node)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Bottleneck {
    pub node_id: Uuid,
    pub node_ref: String,
    pub in_degree: usize,
    pub out_degree: usize,
    pub capacity_ratio: f64,
    pub severity: BottleneckSeverity,
    pub description: String,
}

/// Severity of a bottleneck
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "lowercase")]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl From<f64> for BottleneckSeverity {
    fn from(ratio: f64) -> Self {
        if ratio >= 0.9 {
            BottleneckSeverity::Critical
        } else if ratio >= 0.75 {
            BottleneckSeverity::High
        } else if ratio >= 0.5 {
            BottleneckSeverity::Medium
        } else {
            BottleneckSeverity::Low
        }
    }
}

/// A hole in the topology (missing capability)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Hole {
    pub capability: String,
    pub required_by: Vec<Uuid>,
    pub severity: HoleSeverity,
    pub description: String,
}

/// Severity of a hole
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "lowercase")]
pub enum HoleSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

impl From<usize> for HoleSeverity {
    fn from(required_count: usize) -> Self {
        if required_count >= 5 {
            HoleSeverity::Critical
        } else if required_count >= 3 {
            HoleSeverity::Major
        } else if required_count >= 2 {
            HoleSeverity::Moderate
        } else {
            HoleSeverity::Minor
        }
    }
}

/// An emerging pattern in the topology
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EmergingPattern {
    pub pattern_type: PatternType,
    pub involved_nodes: Vec<Uuid>,
    pub frequency: usize,
    pub description: String,
}

/// Type of pattern detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Hub pattern - single node with many connections
    Hub,
    /// Chain pattern - linear sequence of nodes
    Chain,
    /// Star pattern - central node with spokes
    Star,
    /// Cluster pattern - densely connected group
    DenseCluster,
    /// Bridge pattern - single node connecting components
    Bridge,
    /// Fork pattern - node splitting to multiple paths
    Fork,
    /// Join pattern - multiple paths converging
    Join,
}

/// Detector for patterns in the topology
pub struct PatternDetector {
    /// Threshold for in-degree to consider a node a potential bottleneck
    bottleneck_threshold: usize,
    /// Minimum degree to consider a node a hub
    hub_threshold: usize,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self {
            bottleneck_threshold: 5,
            hub_threshold: 4,
        }
    }
}

impl PatternDetector {
    pub fn new(bottleneck_threshold: usize, hub_threshold: usize) -> Self {
        Self {
            bottleneck_threshold,
            hub_threshold,
        }
    }

    /// Detect bottlenecks in the topology
    pub fn detect_bottlenecks(&self, graph: &TopologyGraph) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        for (node_id, node) in &graph.nodes {
            if !node.is_active() {
                continue;
            }

            let in_degree = graph.in_degree(*node_id);
            let out_degree = graph.out_degree(*node_id);

            // A bottleneck has high in-degree relative to out-degree
            if in_degree >= self.bottleneck_threshold && in_degree > out_degree * 2 {
                let capacity_ratio = if node.weight > 0.0 {
                    in_degree as f64 / (node.weight * 10.0) // Assuming weight * 10 is capacity
                } else {
                    1.0
                };

                let severity = BottleneckSeverity::from(capacity_ratio.min(1.0));

                bottlenecks.push(Bottleneck {
                    node_id: *node_id,
                    node_ref: node.ref_id.clone(),
                    in_degree,
                    out_degree,
                    capacity_ratio,
                    severity,
                    description: format!(
                        "Node '{}' has {} incoming edges but only {} outgoing, creating a potential bottleneck",
                        node.ref_id, in_degree, out_degree
                    ),
                });
            }
        }

        // Sort by severity (critical first)
        bottlenecks.sort_by(|a, b| {
            let a_score = match a.severity {
                BottleneckSeverity::Critical => 4,
                BottleneckSeverity::High => 3,
                BottleneckSeverity::Medium => 2,
                BottleneckSeverity::Low => 1,
            };
            let b_score = match b.severity {
                BottleneckSeverity::Critical => 4,
                BottleneckSeverity::High => 3,
                BottleneckSeverity::Medium => 2,
                BottleneckSeverity::Low => 1,
            };
            b_score.cmp(&a_score)
        });

        bottlenecks
    }

    /// Detect holes (missing capabilities) in the topology
    pub fn detect_holes(&self, graph: &TopologyGraph, required_capabilities: &[String]) -> Vec<Hole> {
        let mut holes = Vec::new();

        // Collect all available capabilities from active nodes
        let mut available_capabilities: HashMap<String, Vec<Uuid>> = HashMap::new();
        for (node_id, node) in &graph.nodes {
            if node.is_active() {
                for cap in &node.capabilities {
                    available_capabilities
                        .entry(cap.clone())
                        .or_default()
                        .push(*node_id);
                }
            }
        }

        // Check which required capabilities are missing or under-provisioned
        for required in required_capabilities {
            match available_capabilities.get(required) {
                None => {
                    // Completely missing capability
                    holes.push(Hole {
                        capability: required.clone(),
                        required_by: Vec::new(),
                        severity: HoleSeverity::Critical,
                        description: format!(
                            "Capability '{}' is required but no active nodes provide it",
                            required
                        ),
                    });
                }
                Some(providers) if providers.len() == 1 => {
                    // Single point of failure
                    holes.push(Hole {
                        capability: required.clone(),
                        required_by: providers.clone(),
                        severity: HoleSeverity::Moderate,
                        description: format!(
                            "Capability '{}' has only one provider - single point of failure",
                            required
                        ),
                    });
                }
                _ => {}
            }
        }

        holes
    }

    /// Detect emerging patterns in the topology
    pub fn detect_patterns(&self, graph: &TopologyGraph) -> Vec<EmergingPattern> {
        let mut patterns = Vec::new();

        // Detect hubs (nodes with many connections)
        for (node_id, node) in &graph.nodes {
            let total_degree = graph.in_degree(*node_id) + graph.out_degree(*node_id);
            if total_degree >= self.hub_threshold {
                patterns.push(EmergingPattern {
                    pattern_type: PatternType::Hub,
                    involved_nodes: vec![*node_id],
                    frequency: total_degree,
                    description: format!(
                        "Node '{}' is a hub with {} total connections",
                        node.ref_id, total_degree
                    ),
                });
            }
        }

        // Detect stars (central node with many outgoing but few incoming)
        for (node_id, node) in &graph.nodes {
            let in_degree = graph.in_degree(*node_id);
            let out_degree = graph.out_degree(*node_id);
            if out_degree >= self.hub_threshold && in_degree <= 1 {
                let neighbors: Vec<Uuid> = graph.neighbors(*node_id);
                let mut involved = vec![*node_id];
                involved.extend(neighbors);
                patterns.push(EmergingPattern {
                    pattern_type: PatternType::Star,
                    involved_nodes: involved,
                    frequency: out_degree,
                    description: format!(
                        "Node '{}' is a star center with {} spokes",
                        node.ref_id, out_degree
                    ),
                });
            }
        }

        // Detect bridges (nodes that connect otherwise disconnected components)
        patterns.extend(self.detect_bridges(graph));

        // Detect forks (nodes that split to multiple paths)
        for (node_id, node) in &graph.nodes {
            let in_degree = graph.in_degree(*node_id);
            let out_degree = graph.out_degree(*node_id);
            if in_degree == 1 && out_degree >= 3 {
                patterns.push(EmergingPattern {
                    pattern_type: PatternType::Fork,
                    involved_nodes: vec![*node_id],
                    frequency: out_degree,
                    description: format!(
                        "Node '{}' is a fork point splitting into {} paths",
                        node.ref_id, out_degree
                    ),
                });
            }
        }

        // Detect joins (multiple paths converging)
        for (node_id, node) in &graph.nodes {
            let in_degree = graph.in_degree(*node_id);
            let out_degree = graph.out_degree(*node_id);
            if in_degree >= 3 && out_degree == 1 {
                patterns.push(EmergingPattern {
                    pattern_type: PatternType::Join,
                    involved_nodes: vec![*node_id],
                    frequency: in_degree,
                    description: format!(
                        "Node '{}' is a join point converging {} paths",
                        node.ref_id, in_degree
                    ),
                });
            }
        }

        patterns
    }

    fn detect_bridges(&self, graph: &TopologyGraph) -> Vec<EmergingPattern> {
        let mut patterns = Vec::new();

        // Simple bridge detection: find nodes whose removal would increase components
        let original_components = TopologyEngine::find_connected_components(graph);

        for (node_id, node) in &graph.nodes {
            // Create a copy without this node
            let mut test_graph = graph.clone();
            test_graph.remove_node(*node_id);

            let new_components = TopologyEngine::find_connected_components(&test_graph);

            // If removing this node creates more components, it's a bridge
            if new_components.len() > original_components.len() {
                patterns.push(EmergingPattern {
                    pattern_type: PatternType::Bridge,
                    involved_nodes: vec![*node_id],
                    frequency: new_components.len() - original_components.len(),
                    description: format!(
                        "Node '{}' is a bridge connecting {} otherwise disconnected components",
                        node.ref_id,
                        new_components.len() - original_components.len() + 1
                    ),
                });
            }
        }

        patterns
    }

    /// Detect orphan nodes (no connections)
    pub fn detect_orphans(&self, graph: &TopologyGraph) -> Vec<Uuid> {
        TopologyEngine::find_orphans(graph)
    }

    /// Detect dead-end nodes (no outgoing paths)
    pub fn detect_dead_ends(&self, graph: &TopologyGraph) -> Vec<Uuid> {
        TopologyEngine::find_dead_ends(graph)
    }

    /// Detect degraded paths (paths with failing or degraded edges)
    pub fn detect_degraded_paths(&self, graph: &TopologyGraph) -> Vec<(Uuid, Uuid, Vec<Uuid>)> {
        let mut degraded = Vec::new();

        // Find edges that are degraded
        let degraded_edges: Vec<Uuid> = graph.edges
            .values()
            .filter(|e| e.status == "degraded")
            .map(|e| e.id)
            .collect();

        if degraded_edges.is_empty() {
            return degraded;
        }

        // For each pair of important nodes, check if the path includes degraded edges
        let active_agents: Vec<Uuid> = graph.nodes
            .iter()
            .filter(|(_, n)| n.node_type == "agent" && n.is_active())
            .map(|(id, _)| *id)
            .collect();

        let tasks: Vec<Uuid> = graph.nodes
            .iter()
            .filter(|(_, n)| n.node_type == "task" && n.is_active())
            .map(|(id, _)| *id)
            .collect();

        for agent in &active_agents {
            for task in &tasks {
                if let Some(path) = TopologyEngine::find_shortest_path(graph, *agent, *task) {
                    let path_degraded: Vec<Uuid> = path.edges
                        .iter()
                        .filter(|e| degraded_edges.contains(e))
                        .copied()
                        .collect();

                    if !path_degraded.is_empty() {
                        degraded.push((*agent, *task, path_degraded));
                    }
                }
            }
        }

        degraded
    }

    /// Run all pattern detection and return a comprehensive report
    pub fn full_analysis(&self, topology: &ProjectTopology, required_capabilities: &[String]) -> PatternAnalysisReport {
        let graph = &topology.graph;

        PatternAnalysisReport {
            bottlenecks: self.detect_bottlenecks(graph),
            holes: self.detect_holes(graph, required_capabilities),
            patterns: self.detect_patterns(graph),
            orphans: self.detect_orphans(graph),
            dead_ends: self.detect_dead_ends(graph),
            cycles: TopologyEngine::find_cycles(graph),
            connected_components: TopologyEngine::find_connected_components(graph).len(),
            graph_diameter: TopologyEngine::calculate_diameter(graph),
            average_path_length: TopologyEngine::average_path_length(graph),
        }
    }
}

/// Comprehensive pattern analysis report
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PatternAnalysisReport {
    pub bottlenecks: Vec<Bottleneck>,
    pub holes: Vec<Hole>,
    pub patterns: Vec<EmergingPattern>,
    pub orphans: Vec<Uuid>,
    pub dead_ends: Vec<Uuid>,
    pub cycles: Vec<Vec<Uuid>>,
    pub connected_components: usize,
    pub graph_diameter: usize,
    pub average_path_length: f64,
}

impl PatternAnalysisReport {
    /// Check if the topology is healthy
    pub fn is_healthy(&self) -> bool {
        self.bottlenecks.iter().all(|b| b.severity != BottleneckSeverity::Critical)
            && self.holes.iter().all(|h| h.severity != HoleSeverity::Critical)
            && self.cycles.is_empty()
            && self.orphans.is_empty()
    }

    /// Get a health score from 0.0 to 1.0
    pub fn health_score(&self) -> f64 {
        let mut score = 1.0;

        // Deduct for bottlenecks
        for bottleneck in &self.bottlenecks {
            match bottleneck.severity {
                BottleneckSeverity::Critical => score -= 0.2,
                BottleneckSeverity::High => score -= 0.1,
                BottleneckSeverity::Medium => score -= 0.05,
                BottleneckSeverity::Low => score -= 0.02,
            }
        }

        // Deduct for holes
        for hole in &self.holes {
            match hole.severity {
                HoleSeverity::Critical => score -= 0.2,
                HoleSeverity::Major => score -= 0.1,
                HoleSeverity::Moderate => score -= 0.05,
                HoleSeverity::Minor => score -= 0.02,
            }
        }

        // Deduct for cycles
        score -= self.cycles.len() as f64 * 0.15;

        // Deduct for orphans
        score -= self.orphans.len() as f64 * 0.05;

        // Deduct for dead ends
        score -= self.dead_ends.len() as f64 * 0.03;

        // Deduct if graph is disconnected
        if self.connected_components > 1 {
            score -= (self.connected_components - 1) as f64 * 0.1;
        }

        score.max(0.0).min(1.0)
    }
}

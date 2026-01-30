//! Cluster management - Dynamic team formation and dissolution

use super::graph::{ClusterInfo, ProjectTopology, TopologyGraph};
use super::engine::TopologyEngine;
use crate::{Result, TopsiError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::collections::HashSet;
use uuid::Uuid;

/// Requirements for forming a cluster
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClusterRequirements {
    pub capabilities: Vec<String>,
    pub min_nodes: usize,
    pub max_nodes: Option<usize>,
    pub required_node_types: Vec<String>,
    pub purpose: String,
}

impl ClusterRequirements {
    pub fn new(purpose: impl Into<String>, capabilities: Vec<String>) -> Self {
        Self {
            capabilities,
            min_nodes: 1,
            max_nodes: None,
            required_node_types: Vec::new(),
            purpose: purpose.into(),
        }
    }

    pub fn with_min_nodes(mut self, min: usize) -> Self {
        self.min_nodes = min;
        self
    }

    pub fn with_max_nodes(mut self, max: usize) -> Self {
        self.max_nodes = Some(max);
        self
    }

    pub fn with_node_types(mut self, types: Vec<String>) -> Self {
        self.required_node_types = types;
        self
    }
}

/// Result of cluster formation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClusterFormationResult {
    pub cluster: ClusterInfo,
    pub capability_coverage: f64,
    pub missing_capabilities: Vec<String>,
    pub warnings: Vec<String>,
}

/// Manager for dynamic cluster operations
pub struct ClusterManager;

impl ClusterManager {
    /// Form a cluster based on requirements
    pub fn form_cluster(
        topology: &mut ProjectTopology,
        requirements: &ClusterRequirements,
        name: impl Into<String>,
    ) -> Result<ClusterFormationResult> {
        let graph = &topology.graph;
        let name = name.into();

        // Find nodes that match the requirements
        let mut candidate_nodes: Vec<(Uuid, f64)> = Vec::new(); // (node_id, score)

        for (node_id, node) in &graph.nodes {
            if !node.is_active() {
                continue;
            }

            // Check node type if required
            if !requirements.required_node_types.is_empty() {
                if !requirements.required_node_types.contains(&node.node_type) {
                    continue;
                }
            }

            // Calculate score based on capability match
            let matching_caps = requirements.capabilities.iter()
                .filter(|c| node.has_capability(c))
                .count();

            if matching_caps > 0 || requirements.capabilities.is_empty() {
                let score = if requirements.capabilities.is_empty() {
                    node.weight
                } else {
                    matching_caps as f64 / requirements.capabilities.len() as f64 * node.weight
                };
                candidate_nodes.push((*node_id, score));
            }
        }

        // Sort by score descending
        candidate_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Select nodes
        let max_nodes = requirements.max_nodes.unwrap_or(candidate_nodes.len());
        let selected: Vec<Uuid> = candidate_nodes.iter()
            .take(max_nodes.max(requirements.min_nodes))
            .map(|(id, _)| *id)
            .collect();

        if selected.len() < requirements.min_nodes {
            return Err(TopsiError::ClusterError(format!(
                "Cannot form cluster '{}': only {} nodes match requirements, {} required",
                name, selected.len(), requirements.min_nodes
            )));
        }

        // Check capability coverage
        let mut covered_capabilities = HashSet::new();
        for node_id in &selected {
            if let Some(node) = graph.get_node(*node_id) {
                for cap in &node.capabilities {
                    if requirements.capabilities.contains(cap) {
                        covered_capabilities.insert(cap.clone());
                    }
                }
            }
        }

        let missing_capabilities: Vec<String> = requirements.capabilities.iter()
            .filter(|c| !covered_capabilities.contains(*c))
            .cloned()
            .collect();

        let coverage = if requirements.capabilities.is_empty() {
            1.0
        } else {
            covered_capabilities.len() as f64 / requirements.capabilities.len() as f64
        };

        // Select leader (highest weighted node)
        let leader_id = selected.iter()
            .max_by(|a, b| {
                let a_weight = graph.get_node(**a).map(|n| n.weight).unwrap_or(0.0);
                let b_weight = graph.get_node(**b).map(|n| n.weight).unwrap_or(0.0);
                a_weight.partial_cmp(&b_weight).unwrap()
            })
            .copied();

        // Create cluster
        let cluster = ClusterInfo::new(Uuid::new_v4(), &name, selected)
            .with_purpose(&requirements.purpose);

        let cluster = if let Some(leader) = leader_id {
            cluster.with_leader(leader)
        } else {
            cluster
        };

        // Warnings
        let mut warnings = Vec::new();
        if !missing_capabilities.is_empty() {
            warnings.push(format!(
                "Missing capabilities: {}",
                missing_capabilities.join(", ")
            ));
        }
        if coverage < 0.8 {
            warnings.push(format!(
                "Low capability coverage: {:.0}%",
                coverage * 100.0
            ));
        }

        // Add cluster to topology
        topology.add_cluster(cluster.clone());

        Ok(ClusterFormationResult {
            cluster,
            capability_coverage: coverage,
            missing_capabilities,
            warnings,
        })
    }

    /// Dissolve a cluster
    pub fn dissolve_cluster(
        topology: &mut ProjectTopology,
        cluster_id: Uuid,
    ) -> Result<ClusterInfo> {
        let cluster = topology.remove_cluster(cluster_id)
            .ok_or_else(|| TopsiError::ClusterError(format!(
                "Cluster {} not found",
                cluster_id
            )))?;

        Ok(cluster)
    }

    /// Add a node to an existing cluster
    pub fn add_to_cluster(
        topology: &mut ProjectTopology,
        cluster_id: Uuid,
        node_id: Uuid,
    ) -> Result<()> {
        // Verify node exists and is active
        if let Some(node) = topology.graph.get_node(node_id) {
            if !node.is_active() {
                return Err(TopsiError::ClusterError(format!(
                    "Node {} is not active",
                    node_id
                )));
            }
        } else {
            return Err(TopsiError::NodeNotFound(node_id));
        }

        // Find and update cluster
        let cluster = topology.clusters.iter_mut()
            .find(|c| c.id == cluster_id)
            .ok_or_else(|| TopsiError::ClusterError(format!(
                "Cluster {} not found",
                cluster_id
            )))?;

        if !cluster.node_ids.contains(&node_id) {
            cluster.node_ids.push(node_id);
        }

        topology.touch();
        Ok(())
    }

    /// Remove a node from a cluster
    pub fn remove_from_cluster(
        topology: &mut ProjectTopology,
        cluster_id: Uuid,
        node_id: Uuid,
    ) -> Result<()> {
        let cluster = topology.clusters.iter_mut()
            .find(|c| c.id == cluster_id)
            .ok_or_else(|| TopsiError::ClusterError(format!(
                "Cluster {} not found",
                cluster_id
            )))?;

        cluster.node_ids.retain(|id| *id != node_id);

        // If the removed node was the leader, select a new one
        if cluster.leader_node_id == Some(node_id) {
            cluster.leader_node_id = cluster.node_ids.first().copied();
        }

        // If cluster is empty, mark as inactive
        if cluster.node_ids.is_empty() {
            cluster.is_active = false;
        }

        topology.touch();
        Ok(())
    }

    /// Discover natural clusters using community detection
    pub fn discover_clusters(topology: &ProjectTopology) -> Vec<ClusterSuggestion> {
        let graph = &topology.graph;
        let mut suggestions = Vec::new();

        // Use connected components as a simple community detection
        let components = TopologyEngine::find_connected_components(graph);

        for (idx, component) in components.iter().enumerate() {
            if component.len() < 2 {
                continue; // Skip single-node components
            }

            // Analyze the component
            let node_types: HashSet<String> = component.iter()
                .filter_map(|id| graph.get_node(*id))
                .map(|n| n.node_type.clone())
                .collect();

            let all_capabilities: HashSet<String> = component.iter()
                .filter_map(|id| graph.get_node(*id))
                .flat_map(|n| n.capabilities.clone())
                .collect();

            // Determine suggested purpose based on dominant node type
            let purpose = if node_types.contains("agent") && node_types.len() == 1 {
                "agent_pool"
            } else if node_types.contains("task") && node_types.len() == 1 {
                "task_batch"
            } else if node_types.contains("agent") && node_types.contains("task") {
                "execution_team"
            } else {
                "resource_group"
            };

            suggestions.push(ClusterSuggestion {
                node_ids: component.iter().copied().collect(),
                suggested_name: format!("Cluster_{}", idx + 1),
                suggested_purpose: purpose.to_string(),
                node_types: node_types.into_iter().collect(),
                shared_capabilities: all_capabilities.into_iter().collect(),
                cohesion_score: Self::calculate_cohesion(graph, component),
            });
        }

        // Sort by cohesion score
        suggestions.sort_by(|a, b| b.cohesion_score.partial_cmp(&a.cohesion_score).unwrap());

        suggestions
    }

    /// Calculate cluster cohesion (density of internal edges)
    fn calculate_cohesion(graph: &TopologyGraph, nodes: &HashSet<Uuid>) -> f64 {
        if nodes.len() < 2 {
            return 1.0;
        }

        let mut internal_edges = 0;
        let max_edges = nodes.len() * (nodes.len() - 1); // Directed graph

        for edge in graph.edges.values() {
            if nodes.contains(&edge.from_node_id) && nodes.contains(&edge.to_node_id) {
                internal_edges += 1;
            }
        }

        if max_edges == 0 {
            0.0
        } else {
            internal_edges as f64 / max_edges as f64
        }
    }

    /// Get clusters that a node belongs to
    pub fn get_node_clusters(topology: &ProjectTopology, node_id: Uuid) -> Vec<&ClusterInfo> {
        topology.clusters.iter()
            .filter(|c| c.is_active && c.node_ids.contains(&node_id))
            .collect()
    }

    /// Get all active clusters
    pub fn get_active_clusters(topology: &ProjectTopology) -> Vec<&ClusterInfo> {
        topology.active_clusters()
    }

    /// Merge two clusters
    pub fn merge_clusters(
        topology: &mut ProjectTopology,
        cluster_a_id: Uuid,
        cluster_b_id: Uuid,
        new_name: impl Into<String>,
    ) -> Result<ClusterInfo> {
        let cluster_a = topology.get_cluster(cluster_a_id)
            .ok_or_else(|| TopsiError::ClusterError(format!("Cluster {} not found", cluster_a_id)))?
            .clone();

        let cluster_b = topology.get_cluster(cluster_b_id)
            .ok_or_else(|| TopsiError::ClusterError(format!("Cluster {} not found", cluster_b_id)))?
            .clone();

        // Combine node IDs
        let mut merged_nodes: Vec<Uuid> = cluster_a.node_ids.clone();
        for node_id in cluster_b.node_ids {
            if !merged_nodes.contains(&node_id) {
                merged_nodes.push(node_id);
            }
        }

        // Create new cluster
        let merged = ClusterInfo::new(
            Uuid::new_v4(),
            new_name,
            merged_nodes,
        )
        .with_purpose(format!(
            "Merged from {} and {}",
            cluster_a.name, cluster_b.name
        ))
        .with_leader(cluster_a.leader_node_id.or(cluster_b.leader_node_id).unwrap_or(Uuid::nil()));

        // Remove old clusters
        topology.remove_cluster(cluster_a_id);
        topology.remove_cluster(cluster_b_id);

        // Add merged cluster
        topology.add_cluster(merged.clone());

        Ok(merged)
    }

    /// Split a cluster into two
    pub fn split_cluster(
        topology: &mut ProjectTopology,
        cluster_id: Uuid,
        split_nodes: Vec<Uuid>,
        new_cluster_name: impl Into<String>,
    ) -> Result<(ClusterInfo, ClusterInfo)> {
        let original = topology.get_cluster(cluster_id)
            .ok_or_else(|| TopsiError::ClusterError(format!("Cluster {} not found", cluster_id)))?
            .clone();

        // Validate split nodes are in original cluster
        for node_id in &split_nodes {
            if !original.node_ids.contains(node_id) {
                return Err(TopsiError::ClusterError(format!(
                    "Node {} is not in cluster {}",
                    node_id, cluster_id
                )));
            }
        }

        // Create remaining nodes list
        let remaining_nodes: Vec<Uuid> = original.node_ids.iter()
            .filter(|n| !split_nodes.contains(n))
            .copied()
            .collect();

        if remaining_nodes.is_empty() {
            return Err(TopsiError::ClusterError(
                "Cannot split: all nodes would be moved to new cluster".to_string()
            ));
        }

        // Update original cluster
        topology.remove_cluster(cluster_id);

        let updated_original = ClusterInfo::new(
            original.id,
            &original.name,
            remaining_nodes,
        )
        .with_purpose(original.purpose.unwrap_or_default());

        let new_cluster = ClusterInfo::new(
            Uuid::new_v4(),
            new_cluster_name,
            split_nodes,
        )
        .with_purpose(format!("Split from {}", original.name));

        topology.add_cluster(updated_original.clone());
        topology.add_cluster(new_cluster.clone());

        Ok((updated_original, new_cluster))
    }
}

/// Suggestion for a potential cluster
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClusterSuggestion {
    pub node_ids: Vec<Uuid>,
    pub suggested_name: String,
    pub suggested_purpose: String,
    pub node_types: Vec<String>,
    pub shared_capabilities: Vec<String>,
    pub cohesion_score: f64,
}

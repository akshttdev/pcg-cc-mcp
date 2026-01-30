//! Topology engine - Graph algorithms for pathfinding and analysis

use super::graph::{GraphEdge, GraphNode, TopologyGraph};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::cmp::Ordering;
use uuid::Uuid;

/// A path through the topology
#[derive(Debug, Clone)]
pub struct Path {
    pub nodes: Vec<Uuid>,
    pub edges: Vec<Uuid>,
    pub total_weight: f64,
}

impl Path {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            total_weight: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

/// State for Dijkstra's algorithm
#[derive(Clone, PartialEq)]
struct DijkstraState {
    cost: f64,
    node_id: Uuid,
}

impl Eq for DijkstraState {}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Engine for topology operations and algorithms
pub struct TopologyEngine;

impl TopologyEngine {
    /// Find the shortest path between two nodes using Dijkstra's algorithm
    pub fn find_shortest_path(
        graph: &TopologyGraph,
        from: Uuid,
        to: Uuid,
    ) -> Option<Path> {
        if !graph.nodes.contains_key(&from) || !graph.nodes.contains_key(&to) {
            return None;
        }

        if from == to {
            return Some(Path {
                nodes: vec![from],
                edges: Vec::new(),
                total_weight: 0.0,
            });
        }

        let mut dist: HashMap<Uuid, f64> = HashMap::new();
        let mut prev: HashMap<Uuid, (Uuid, Uuid)> = HashMap::new(); // (prev_node, edge_id)
        let mut heap = BinaryHeap::new();

        dist.insert(from, 0.0);
        heap.push(DijkstraState { cost: 0.0, node_id: from });

        while let Some(DijkstraState { cost, node_id }) = heap.pop() {
            if node_id == to {
                // Reconstruct path
                let mut path = Path::new();
                let mut current = to;
                path.total_weight = cost;

                while current != from {
                    path.nodes.push(current);
                    if let Some((prev_node, edge_id)) = prev.get(&current) {
                        path.edges.push(*edge_id);
                        current = *prev_node;
                    } else {
                        break;
                    }
                }
                path.nodes.push(from);
                path.nodes.reverse();
                path.edges.reverse();

                return Some(path);
            }

            if cost > *dist.get(&node_id).unwrap_or(&f64::INFINITY) {
                continue;
            }

            for edge in graph.edges_from(node_id) {
                // Skip inactive edges
                if !edge.is_active() {
                    continue;
                }

                // Skip if target node is inactive
                if let Some(target_node) = graph.get_node(edge.to_node_id) {
                    if !target_node.is_active() {
                        continue;
                    }
                } else {
                    continue;
                }

                let next_cost = cost + edge.weight;
                if next_cost < *dist.get(&edge.to_node_id).unwrap_or(&f64::INFINITY) {
                    dist.insert(edge.to_node_id, next_cost);
                    prev.insert(edge.to_node_id, (node_id, edge.id));
                    heap.push(DijkstraState {
                        cost: next_cost,
                        node_id: edge.to_node_id,
                    });
                }
            }
        }

        None
    }

    /// Find all paths between two nodes (up to a limit)
    pub fn find_all_paths(
        graph: &TopologyGraph,
        from: Uuid,
        to: Uuid,
        max_paths: usize,
        max_depth: usize,
    ) -> Vec<Path> {
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        let mut current_path = Path::new();

        Self::dfs_paths(
            graph,
            from,
            to,
            &mut visited,
            &mut current_path,
            &mut paths,
            max_paths,
            max_depth,
            0,
        );

        paths
    }

    fn dfs_paths(
        graph: &TopologyGraph,
        current: Uuid,
        target: Uuid,
        visited: &mut HashSet<Uuid>,
        current_path: &mut Path,
        paths: &mut Vec<Path>,
        max_paths: usize,
        max_depth: usize,
        depth: usize,
    ) {
        if paths.len() >= max_paths || depth > max_depth {
            return;
        }

        visited.insert(current);
        current_path.nodes.push(current);

        if current == target {
            paths.push(current_path.clone());
        } else {
            for edge in graph.edges_from(current) {
                if !visited.contains(&edge.to_node_id) && edge.is_active() {
                    if let Some(node) = graph.get_node(edge.to_node_id) {
                        if node.is_active() {
                            current_path.edges.push(edge.id);
                            current_path.total_weight += edge.weight;

                            Self::dfs_paths(
                                graph,
                                edge.to_node_id,
                                target,
                                visited,
                                current_path,
                                paths,
                                max_paths,
                                max_depth,
                                depth + 1,
                            );

                            current_path.edges.pop();
                            current_path.total_weight -= edge.weight;
                        }
                    }
                }
            }
        }

        visited.remove(&current);
        current_path.nodes.pop();
    }

    /// Find the best path considering weights and node status
    pub fn find_best_path(
        graph: &TopologyGraph,
        from: Uuid,
        to: Uuid,
    ) -> Option<Path> {
        // For now, best path is the shortest path
        // Could be extended to consider node weights, capacities, etc.
        Self::find_shortest_path(graph, from, to)
    }

    /// Check if a path exists between two nodes
    pub fn path_exists(graph: &TopologyGraph, from: Uuid, to: Uuid) -> bool {
        Self::find_shortest_path(graph, from, to).is_some()
    }

    /// Check if adding an edge would create a cycle
    pub fn would_create_cycle(
        graph: &TopologyGraph,
        from: Uuid,
        to: Uuid,
    ) -> bool {
        // A cycle would be created if there's already a path from 'to' to 'from'
        Self::path_exists(graph, to, from)
    }

    /// Perform topological sort (returns None if graph has cycles)
    pub fn topological_sort(graph: &TopologyGraph) -> Option<Vec<Uuid>> {
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        // Initialize in-degrees
        for node_id in graph.nodes.keys() {
            in_degree.insert(*node_id, 0);
        }

        for edge in graph.edges.values() {
            if edge.is_active() {
                *in_degree.entry(edge.to_node_id).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0
        for (node_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(*node_id);
            }
        }

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);

            for edge in graph.edges_from(node_id) {
                let target = edge.to_node_id;
                if let Some(degree) = in_degree.get_mut(&target) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(target);
                    }
                }
            }
        }

        if result.len() == graph.nodes.len() {
            Some(result)
        } else {
            None // Graph has a cycle
        }
    }

    /// Find all cycles in the graph
    pub fn find_cycles(graph: &TopologyGraph) -> Vec<Vec<Uuid>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node_id in graph.nodes.keys() {
            if !visited.contains(node_id) {
                Self::dfs_cycles(
                    graph,
                    *node_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_cycles(
        graph: &TopologyGraph,
        node_id: Uuid,
        visited: &mut HashSet<Uuid>,
        rec_stack: &mut HashSet<Uuid>,
        path: &mut Vec<Uuid>,
        cycles: &mut Vec<Vec<Uuid>>,
    ) {
        visited.insert(node_id);
        rec_stack.insert(node_id);
        path.push(node_id);

        for edge in graph.edges_from(node_id) {
            let target = edge.to_node_id;

            if !visited.contains(&target) {
                Self::dfs_cycles(graph, target, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(&target) {
                // Found a cycle - extract it from the path
                if let Some(start_idx) = path.iter().position(|&n| n == target) {
                    let cycle: Vec<Uuid> = path[start_idx..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(&node_id);
    }

    /// Find connected components in the graph (treating edges as undirected)
    pub fn find_connected_components(graph: &TopologyGraph) -> Vec<HashSet<Uuid>> {
        let mut components = Vec::new();
        let mut visited = HashSet::new();

        for node_id in graph.nodes.keys() {
            if !visited.contains(node_id) {
                let mut component = HashSet::new();
                Self::bfs_component(graph, *node_id, &mut visited, &mut component);
                components.push(component);
            }
        }

        components
    }

    fn bfs_component(
        graph: &TopologyGraph,
        start: Uuid,
        visited: &mut HashSet<Uuid>,
        component: &mut HashSet<Uuid>,
    ) {
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(node_id) = queue.pop_front() {
            component.insert(node_id);

            // Check outgoing edges
            for edge in graph.edges_from(node_id) {
                if !visited.contains(&edge.to_node_id) {
                    visited.insert(edge.to_node_id);
                    queue.push_back(edge.to_node_id);
                }
            }

            // Check incoming edges (treat as undirected)
            for edge in graph.edges_to(node_id) {
                if !visited.contains(&edge.from_node_id) {
                    visited.insert(edge.from_node_id);
                    queue.push_back(edge.from_node_id);
                }
            }
        }
    }

    /// Find orphan nodes (nodes with no connections)
    pub fn find_orphans(graph: &TopologyGraph) -> Vec<Uuid> {
        graph
            .nodes
            .keys()
            .filter(|&node_id| {
                graph.in_degree(*node_id) == 0 && graph.out_degree(*node_id) == 0
            })
            .copied()
            .collect()
    }

    /// Find dead-end nodes (nodes with no outgoing edges)
    pub fn find_dead_ends(graph: &TopologyGraph) -> Vec<Uuid> {
        graph
            .nodes
            .keys()
            .filter(|&node_id| {
                graph.in_degree(*node_id) > 0 && graph.out_degree(*node_id) == 0
            })
            .copied()
            .collect()
    }

    /// Calculate the diameter of the graph (longest shortest path)
    pub fn calculate_diameter(graph: &TopologyGraph) -> usize {
        let mut max_distance = 0;

        for from in graph.nodes.keys() {
            for to in graph.nodes.keys() {
                if from != to {
                    if let Some(path) = Self::find_shortest_path(graph, *from, *to) {
                        max_distance = max_distance.max(path.len() - 1);
                    }
                }
            }
        }

        max_distance
    }

    /// Calculate average path length
    pub fn average_path_length(graph: &TopologyGraph) -> f64 {
        let mut total_length = 0usize;
        let mut path_count = 0usize;

        for from in graph.nodes.keys() {
            for to in graph.nodes.keys() {
                if from != to {
                    if let Some(path) = Self::find_shortest_path(graph, *from, *to) {
                        total_length += path.len() - 1;
                        path_count += 1;
                    }
                }
            }
        }

        if path_count > 0 {
            total_length as f64 / path_count as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> TopologyGraph {
        let mut graph = TopologyGraph::new();

        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();
        let node_d = Uuid::new_v4();

        graph.add_node(GraphNode::new(node_a, "agent", "A"));
        graph.add_node(GraphNode::new(node_b, "agent", "B"));
        graph.add_node(GraphNode::new(node_c, "task", "C"));
        graph.add_node(GraphNode::new(node_d, "task", "D"));

        graph.add_edge(GraphEdge::new(Uuid::new_v4(), node_a, node_b, "can_execute"));
        graph.add_edge(GraphEdge::new(Uuid::new_v4(), node_b, node_c, "can_execute"));
        graph.add_edge(GraphEdge::new(Uuid::new_v4(), node_a, node_c, "can_execute").with_weight(3.0));
        graph.add_edge(GraphEdge::new(Uuid::new_v4(), node_c, node_d, "depends_on"));

        graph
    }

    #[test]
    fn test_find_shortest_path() {
        let graph = create_test_graph();
        let nodes: Vec<_> = graph.nodes.keys().collect();

        let path = TopologyEngine::find_shortest_path(&graph, *nodes[0], *nodes[2]);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3); // A -> B -> C (shorter than A -> C directly)
    }

    #[test]
    fn test_path_exists() {
        let graph = create_test_graph();
        let nodes: Vec<_> = graph.nodes.keys().collect();

        assert!(TopologyEngine::path_exists(&graph, *nodes[0], *nodes[3]));
        assert!(!TopologyEngine::path_exists(&graph, *nodes[3], *nodes[0]));
    }

    #[test]
    fn test_find_orphans() {
        let mut graph = TopologyGraph::new();
        let orphan = Uuid::new_v4();
        let connected = Uuid::new_v4();

        graph.add_node(GraphNode::new(orphan, "agent", "orphan"));
        graph.add_node(GraphNode::new(connected, "agent", "connected"));
        graph.add_edge(GraphEdge::new(Uuid::new_v4(), connected, connected, "self_loop"));

        let orphans = TopologyEngine::find_orphans(&graph);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0], orphan);
    }
}

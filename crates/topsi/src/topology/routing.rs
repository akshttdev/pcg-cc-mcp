//! Route planning - Find optimal paths through the topology

use super::graph::{ProjectTopology, TopologyGraph, RouteInfo};
use super::engine::{TopologyEngine, Path};
use crate::{Goal, Result, TopsiError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// A planned execution route
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPlan {
    pub route_id: Uuid,
    pub goal: String,
    pub path: Vec<RouteStep>,
    pub total_weight: f64,
    pub estimated_duration_ms: Option<u64>,
    pub alternatives: Vec<AlternativeRoute>,
}

/// A step in the execution route
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RouteStep {
    pub node_id: Uuid,
    pub node_ref: String,
    pub node_type: String,
    pub action: String,
    pub edge_id: Option<Uuid>,
    pub edge_type: Option<String>,
}

/// An alternative route
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeRoute {
    pub path: Vec<Uuid>,
    pub total_weight: f64,
    pub reason: String,
}

/// Route planner for finding optimal paths
pub struct RoutePlanner;

impl RoutePlanner {
    /// Plan a route for a goal
    pub fn plan_route(
        topology: &ProjectTopology,
        goal: &Goal,
    ) -> Result<ExecutionPlan> {
        match goal {
            Goal::ExecuteTask(task_id) => {
                Self::plan_task_execution(topology, *task_id)
            }
            Goal::ReachCapability(capability) => {
                Self::plan_capability_reach(topology, capability)
            }
            Goal::ConnectNodes { from, to } => {
                Self::plan_connection(topology, *from, *to)
            }
            Goal::FindAgent(capabilities) => {
                Self::plan_agent_search(topology, capabilities)
            }
            Goal::ExecuteWorkflow(workflow_id) => {
                Self::plan_workflow_execution(topology, workflow_id)
            }
        }
    }

    /// Plan execution path to a task
    fn plan_task_execution(
        topology: &ProjectTopology,
        task_id: Uuid,
    ) -> Result<ExecutionPlan> {
        let graph = &topology.graph;

        // Find the task node
        let task_node = graph.nodes.values()
            .find(|n| n.node_type == "task" && n.ref_id == task_id.to_string())
            .ok_or_else(|| TopsiError::NodeNotFound(task_id))?;

        // Find available agents
        let agents: Vec<_> = graph.nodes.values()
            .filter(|n| n.node_type == "agent" && n.is_active())
            .collect();

        if agents.is_empty() {
            return Err(TopsiError::RoutingError("No active agents available".to_string()));
        }

        // Find best path from any agent to the task
        let mut best_path: Option<Path> = None;
        let mut best_agent_id: Option<Uuid> = None;

        for agent in &agents {
            if let Some(path) = TopologyEngine::find_shortest_path(graph, agent.id, task_node.id) {
                if best_path.is_none() || path.total_weight < best_path.as_ref().unwrap().total_weight {
                    best_path = Some(path);
                    best_agent_id = Some(agent.id);
                }
            }
        }

        let path = best_path.ok_or_else(|| TopsiError::NoPathFound {
            from: agents[0].id,
            to: task_node.id,
        })?;

        // Build execution plan
        let steps = Self::path_to_steps(graph, &path)?;

        // Find alternatives
        let alternatives = Self::find_alternatives(graph, best_agent_id.unwrap(), task_node.id, &path);

        Ok(ExecutionPlan {
            route_id: Uuid::new_v4(),
            goal: format!("Execute task {}", task_id),
            path: steps,
            total_weight: path.total_weight,
            estimated_duration_ms: None,
            alternatives,
        })
    }

    /// Plan path to reach a capability
    fn plan_capability_reach(
        topology: &ProjectTopology,
        capability: &str,
    ) -> Result<ExecutionPlan> {
        let graph = &topology.graph;

        // Find nodes with the capability
        let target_nodes: Vec<_> = graph.nodes.values()
            .filter(|n| n.is_active() && n.has_capability(capability))
            .collect();

        if target_nodes.is_empty() {
            return Err(TopsiError::RoutingError(format!(
                "No nodes with capability '{}' found",
                capability
            )));
        }

        // Find agents as starting points
        let agents: Vec<_> = graph.nodes.values()
            .filter(|n| n.node_type == "agent" && n.is_active())
            .collect();

        if agents.is_empty() {
            return Err(TopsiError::RoutingError("No active agents available".to_string()));
        }

        // Find best path from any agent to any target
        let mut best_path: Option<Path> = None;
        let mut from_id: Option<Uuid> = None;
        let mut to_id: Option<Uuid> = None;

        for agent in &agents {
            for target in &target_nodes {
                if let Some(path) = TopologyEngine::find_shortest_path(graph, agent.id, target.id) {
                    if best_path.is_none() || path.total_weight < best_path.as_ref().unwrap().total_weight {
                        best_path = Some(path);
                        from_id = Some(agent.id);
                        to_id = Some(target.id);
                    }
                }
            }
        }

        let path = best_path.ok_or_else(|| TopsiError::RoutingError(format!(
            "No path to capability '{}' found",
            capability
        )))?;

        let steps = Self::path_to_steps(graph, &path)?;
        let alternatives = Self::find_alternatives(graph, from_id.unwrap(), to_id.unwrap(), &path);

        Ok(ExecutionPlan {
            route_id: Uuid::new_v4(),
            goal: format!("Reach capability '{}'", capability),
            path: steps,
            total_weight: path.total_weight,
            estimated_duration_ms: None,
            alternatives,
        })
    }

    /// Plan a connection between two nodes
    fn plan_connection(
        topology: &ProjectTopology,
        from: Uuid,
        to: Uuid,
    ) -> Result<ExecutionPlan> {
        let graph = &topology.graph;

        let path = TopologyEngine::find_shortest_path(graph, from, to)
            .ok_or(TopsiError::NoPathFound { from, to })?;

        let steps = Self::path_to_steps(graph, &path)?;
        let alternatives = Self::find_alternatives(graph, from, to, &path);

        Ok(ExecutionPlan {
            route_id: Uuid::new_v4(),
            goal: format!("Connect {} to {}", from, to),
            path: steps,
            total_weight: path.total_weight,
            estimated_duration_ms: None,
            alternatives,
        })
    }

    /// Plan to find an agent with specific capabilities
    fn plan_agent_search(
        topology: &ProjectTopology,
        capabilities: &[String],
    ) -> Result<ExecutionPlan> {
        let graph = &topology.graph;

        // Find agents with all required capabilities
        let matching_agents: Vec<_> = graph.nodes.values()
            .filter(|n| {
                n.node_type == "agent" && n.is_active() &&
                capabilities.iter().all(|c| n.has_capability(c))
            })
            .collect();

        if matching_agents.is_empty() {
            // Try to find agents with partial matches
            let partial_matches: Vec<_> = graph.nodes.values()
                .filter(|n| {
                    n.node_type == "agent" && n.is_active() &&
                    capabilities.iter().any(|c| n.has_capability(c))
                })
                .collect();

            if partial_matches.is_empty() {
                return Err(TopsiError::RoutingError(format!(
                    "No agents found with capabilities: {:?}",
                    capabilities
                )));
            }

            // Return partial match
            let best = partial_matches[0];
            let steps = vec![RouteStep {
                node_id: best.id,
                node_ref: best.ref_id.clone(),
                node_type: best.node_type.clone(),
                action: "partial_match".to_string(),
                edge_id: None,
                edge_type: None,
            }];

            return Ok(ExecutionPlan {
                route_id: Uuid::new_v4(),
                goal: format!("Find agent with capabilities {:?} (partial match)", capabilities),
                path: steps,
                total_weight: 0.0,
                estimated_duration_ms: None,
                alternatives: Vec::new(),
            });
        }

        // Return the best matching agent (highest weight)
        let best = matching_agents.iter()
            .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap())
            .unwrap();

        let steps = vec![RouteStep {
            node_id: best.id,
            node_ref: best.ref_id.clone(),
            node_type: best.node_type.clone(),
            action: "found".to_string(),
            edge_id: None,
            edge_type: None,
        }];

        let alternatives: Vec<AlternativeRoute> = matching_agents.iter()
            .filter(|a| a.id != best.id)
            .take(3)
            .map(|a| AlternativeRoute {
                path: vec![a.id],
                total_weight: a.weight,
                reason: format!("Alternative agent: {}", a.ref_id),
            })
            .collect();

        Ok(ExecutionPlan {
            route_id: Uuid::new_v4(),
            goal: format!("Find agent with capabilities {:?}", capabilities),
            path: steps,
            total_weight: 0.0,
            estimated_duration_ms: None,
            alternatives,
        })
    }

    /// Plan workflow execution
    fn plan_workflow_execution(
        topology: &ProjectTopology,
        workflow_id: &str,
    ) -> Result<ExecutionPlan> {
        let graph = &topology.graph;

        // Find workflow node
        let workflow_node = graph.nodes.values()
            .find(|n| n.node_type == "workflow" && n.ref_id == workflow_id)
            .ok_or_else(|| TopsiError::RoutingError(format!(
                "Workflow '{}' not found",
                workflow_id
            )))?;

        // Find all connected nodes in execution order
        let mut execution_order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(workflow_node.id);
        visited.insert(workflow_node.id);

        while let Some(node_id) = queue.pop_front() {
            execution_order.push(node_id);

            for edge in graph.edges_from(node_id) {
                if !visited.contains(&edge.to_node_id) {
                    visited.insert(edge.to_node_id);
                    queue.push_back(edge.to_node_id);
                }
            }
        }

        // Build steps
        let steps: Vec<RouteStep> = execution_order.iter()
            .filter_map(|id| {
                graph.get_node(*id).map(|n| RouteStep {
                    node_id: n.id,
                    node_ref: n.ref_id.clone(),
                    node_type: n.node_type.clone(),
                    action: "execute".to_string(),
                    edge_id: None,
                    edge_type: None,
                })
            })
            .collect();

        Ok(ExecutionPlan {
            route_id: Uuid::new_v4(),
            goal: format!("Execute workflow '{}'", workflow_id),
            path: steps,
            total_weight: 0.0,
            estimated_duration_ms: None,
            alternatives: Vec::new(),
        })
    }

    /// Convert a path to route steps
    fn path_to_steps(graph: &TopologyGraph, path: &Path) -> Result<Vec<RouteStep>> {
        let mut steps = Vec::new();

        for (i, node_id) in path.nodes.iter().enumerate() {
            let node = graph.get_node(*node_id)
                .ok_or_else(|| TopsiError::NodeNotFound(*node_id))?;

            let (edge_id, edge_type) = if i < path.edges.len() {
                let edge = graph.get_edge(path.edges[i]);
                (edge.map(|e| e.id), edge.map(|e| e.edge_type.clone()))
            } else {
                (None, None)
            };

            let action = if i == 0 {
                "start"
            } else if i == path.nodes.len() - 1 {
                "execute"
            } else {
                "traverse"
            };

            steps.push(RouteStep {
                node_id: node.id,
                node_ref: node.ref_id.clone(),
                node_type: node.node_type.clone(),
                action: action.to_string(),
                edge_id,
                edge_type,
            });
        }

        Ok(steps)
    }

    /// Find alternative routes
    fn find_alternatives(
        graph: &TopologyGraph,
        from: Uuid,
        to: Uuid,
        exclude: &Path,
    ) -> Vec<AlternativeRoute> {
        let all_paths = TopologyEngine::find_all_paths(graph, from, to, 4, 10);

        all_paths.into_iter()
            .filter(|p| p.nodes != exclude.nodes)
            .take(3)
            .map(|p| AlternativeRoute {
                path: p.nodes,
                total_weight: p.total_weight,
                reason: "Alternative path".to_string(),
            })
            .collect()
    }

    /// Find a new route avoiding a failed node
    pub fn reroute_avoiding(
        topology: &ProjectTopology,
        original_route: &RouteInfo,
        failed_node: Uuid,
    ) -> Result<ExecutionPlan> {
        let mut graph = topology.graph.clone();

        // Mark the failed node as inactive
        if let Some(node) = graph.get_node_mut(failed_node) {
            node.status = "failed".to_string();
        }

        // Find new path from start to end of original route
        if original_route.path.len() < 2 {
            return Err(TopsiError::RoutingError("Route too short to reroute".to_string()));
        }

        let from = original_route.path[0];
        let to = *original_route.path.last().unwrap();

        let path = TopologyEngine::find_shortest_path(&graph, from, to)
            .ok_or(TopsiError::NoPathFound { from, to })?;

        let steps = Self::path_to_steps(&graph, &path)?;

        Ok(ExecutionPlan {
            route_id: Uuid::new_v4(),
            goal: format!("Reroute: {}", original_route.goal),
            path: steps,
            total_weight: path.total_weight,
            estimated_duration_ms: None,
            alternatives: Vec::new(),
        })
    }
}

// Path is exported from the engine module, re-exported via topology/mod.rs

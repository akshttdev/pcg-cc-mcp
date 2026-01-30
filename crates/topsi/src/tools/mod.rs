//! Topsi Tools - MCP-compatible tools for topology operations
//!
//! This module provides tools that Topsi can use during conversations
//! to interact with and modify the topology.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Tool definition for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Get tool schemas in OpenAI function calling format
///
/// Returns a vector of tool definitions compatible with OpenAI's function calling API.
/// These are used by the LLM to understand what tools are available.
pub fn get_tool_schemas() -> Vec<Value> {
    vec![
        // ==================== PROJECT TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "list_projects",
                "description": "List all projects the user has access to. Returns project names, IDs, paths, and VIBE budget information. Use this when users ask about their projects or what they can work on.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_nodes",
                "description": "List all nodes in the topology, optionally filtered by type or status. Use this to explore what agents, tasks, and resources exist in the system.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to list nodes for (optional, defaults to all accessible projects)"
                        },
                        "node_type": {
                            "type": "string",
                            "description": "Filter by node type: 'agent', 'task', 'resource', 'project'"
                        },
                        "status": {
                            "type": "string",
                            "description": "Filter by status: 'active', 'idle', 'busy', 'degraded'"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_edges",
                "description": "List all edges (connections) between nodes in the topology. Edges represent dependencies, assignments, and relationships.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to list edges for"
                        },
                        "edge_type": {
                            "type": "string",
                            "description": "Filter by edge type: 'depends_on', 'assigned_to', 'contains', 'communicates_with'"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "find_path",
                "description": "Find the optimal path between two nodes in the topology. Useful for routing work or understanding dependencies.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "from_node_id": {
                            "type": "string",
                            "description": "Starting node ID"
                        },
                        "to_node_id": {
                            "type": "string",
                            "description": "Target node ID"
                        },
                        "constraints": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Routing constraints: 'avoid_busy', 'prefer_fast', 'minimize_hops'"
                        }
                    },
                    "required": ["from_node_id", "to_node_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "detect_issues",
                "description": "Detect issues in the topology such as bottlenecks, cycles, isolated nodes, or overloaded agents.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to analyze (optional for all accessible projects)"
                        },
                        "issue_types": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Types of issues to detect: 'bottleneck', 'cycle', 'isolated', 'overloaded', 'stale'"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "get_topology_summary",
                "description": "Get a summary of the current topology state including node counts, edge counts, clusters, and health score.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to summarize (optional for system-wide summary)"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "create_cluster",
                "description": "Create a new cluster (team or group) of nodes for coordinated work.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the new cluster"
                        },
                        "node_ids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Node IDs to include in the cluster"
                        },
                        "cluster_type": {
                            "type": "string",
                            "description": "Type of cluster: 'team', 'workflow', 'resource_pool'"
                        }
                    },
                    "required": ["name", "node_ids"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "verify_access",
                "description": "Verify if a user has access to a specific resource. Use this to check permissions before operations.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "user_id": {
                            "type": "string",
                            "description": "User ID to check"
                        },
                        "resource_type": {
                            "type": "string",
                            "description": "Type of resource: 'project', 'task', 'agent', 'cluster'"
                        },
                        "resource_id": {
                            "type": "string",
                            "description": "Resource ID to check access for"
                        },
                        "action": {
                            "type": "string",
                            "description": "Action to check: 'read', 'write', 'delete', 'execute'"
                        }
                    },
                    "required": ["user_id", "resource_type", "resource_id", "action"]
                }
            }
        }),
        // Chat/response tool for conversational replies
        json!({
            "type": "function",
            "function": {
                "name": "respond_to_user",
                "description": "Generate a conversational response to the user. Use this when you want to speak directly to the user, introduce yourself, answer questions about yourself, or provide information that doesn't require other tools.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The message to send to the user. This should be natural, conversational text."
                        }
                    },
                    "required": ["message"]
                }
            }
        }),
    ]
}

/// Get all available Topsi tools
pub fn get_topsi_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_nodes".to_string(),
            description: "List all nodes in the topology, optionally filtered by type or status".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID to list nodes for (optional, defaults to current context)"
                    },
                    "node_type": {
                        "type": "string",
                        "description": "Filter by node type (agent, task, resource, etc.)"
                    },
                    "status": {
                        "type": "string",
                        "description": "Filter by status (active, idle, busy, degraded)"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "list_edges".to_string(),
            description: "List all edges (connections) in the topology".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID to list edges for"
                    },
                    "edge_type": {
                        "type": "string",
                        "description": "Filter by edge type (depends_on, assigned_to, etc.)"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "find_path".to_string(),
            description: "Find the optimal path between two nodes in the topology".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from_node_id": {
                        "type": "string",
                        "description": "Starting node ID"
                    },
                    "to_node_id": {
                        "type": "string",
                        "description": "Target node ID"
                    },
                    "constraints": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Routing constraints (avoid_busy, prefer_fast, etc.)"
                    }
                },
                "required": ["from_node_id", "to_node_id"]
            }),
        },
        ToolDefinition {
            name: "detect_issues".to_string(),
            description: "Detect issues in the topology (bottlenecks, cycles, isolated nodes)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID to analyze"
                    },
                    "issue_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Types of issues to detect (bottleneck, cycle, isolated, overloaded)"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "get_topology_summary".to_string(),
            description: "Get a summary of the current topology state".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID to summarize (optional for all projects)"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "create_cluster".to_string(),
            description: "Create a new cluster of nodes for coordinated work".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Cluster name"
                    },
                    "node_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Node IDs to include in the cluster"
                    },
                    "cluster_type": {
                        "type": "string",
                        "description": "Type of cluster (team, workflow, resource_pool)"
                    }
                },
                "required": ["name", "node_ids"]
            }),
        },
        ToolDefinition {
            name: "verify_access".to_string(),
            description: "Verify if a user has access to a specific resource".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "user_id": {
                        "type": "string",
                        "description": "User ID to check"
                    },
                    "resource_type": {
                        "type": "string",
                        "description": "Type of resource (project, task, agent)"
                    },
                    "resource_id": {
                        "type": "string",
                        "description": "Resource ID to check access for"
                    },
                    "action": {
                        "type": "string",
                        "description": "Action to check (read, write, delete)"
                    }
                },
                "required": ["user_id", "resource_type", "resource_id", "action"]
            }),
        },
    ]
}

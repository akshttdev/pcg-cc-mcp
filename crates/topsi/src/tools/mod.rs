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
        // ==================== PROJECT MANAGEMENT TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "create_project",
                "description": "Create a new project for the user. Projects are workspaces where tasks and agents operate.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Project name (e.g., 'My Test Project', 'Website Redesign')"
                        },
                        "path": {
                            "type": "string",
                            "description": "Optional: Git repository path. If not provided, will create in ~/projects/<name>"
                        }
                    },
                    "required": ["name"]
                }
            }
        }),
        // ==================== ORGANIZATION & CLIENT TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "list_organizations",
                "description": "List all organizations on the platform. Use this to find an organization's ID when a user wants to assign a project to an org (e.g., 'this belongs to Sirak Studios').",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "update_project",
                "description": "Update a project's metadata such as name, organization assignment, or client. Use this when a user wants to reassign a project to an organization or rename it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "UUID of the project to update"
                        },
                        "name": {
                            "type": "string",
                            "description": "New project name (optional)"
                        },
                        "organization_id": {
                            "type": "string",
                            "description": "Organization UUID to assign this project to (optional). Get from list_organizations."
                        }
                    },
                    "required": ["project_id"]
                }
            }
        }),
        // ==================== TASK & AGENT ORCHESTRATION TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "create_task",
                "description": "Create a new task and optionally assign it to an agent. When agent_name is provided, execution starts automatically — do NOT also call start_task_execution. SMART CONTEXT: If project_id is not specified, Topsi will automatically: (1) use the only project if user has exactly one, (2) create a new project if user has none, or (3) analyze available projects and suggest the best match if multiple exist.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to create the task in. OPTIONAL - omit to let Topsi intelligently choose or create a project based on context."
                        },
                        "title": {
                            "type": "string",
                            "description": "Task title (brief description)"
                        },
                        "description": {
                            "type": "string",
                            "description": "Detailed task description with instructions"
                        },
                        "agent_name": {
                            "type": "string",
                            "description": "Agent to assign and auto-execute the task with (e.g., 'claude', 'Scout', 'Nora', 'Maci'). Execution starts immediately."
                        },
                        "auto_execute": {
                            "type": "boolean",
                            "description": "Whether to auto-start execution when agent_name is set. Defaults to true."
                        },
                        "status": {
                            "type": "string",
                            "description": "Initial status: 'todo', 'in_progress', or 'done'. Defaults to 'todo'."
                        }
                    },
                    "required": ["title", "description"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_agents",
                "description": "List all available agents in the system. Use this to see what agents can be orchestrated.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        // ==================== EXECUTION & ORCHESTRATION TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "start_task_execution",
                "description": "Start executing a task by spawning a coding agent. Creates a task attempt and triggers the executor pipeline. The agent will work in an isolated git worktree and stream logs in real-time.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "UUID of the task to execute"
                        },
                        "agent_name": {
                            "type": "string",
                            "description": "Agent to use: 'claude' (default), 'gemini', 'amp', 'codex'"
                        },
                        "additional_prompt": {
                            "type": "string",
                            "description": "Optional extra instructions for the agent beyond the task description"
                        }
                    },
                    "required": ["task_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "get_task_status",
                "description": "Get the current status of a task including its latest execution attempt, recent log output, and completion state. Use to monitor running tasks.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "UUID of the task to check"
                        },
                        "include_logs": {
                            "type": "boolean",
                            "description": "Whether to include recent log lines (default: true)"
                        },
                        "log_lines": {
                            "type": "integer",
                            "description": "Number of recent log lines to include (default: 20)"
                        }
                    },
                    "required": ["task_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "update_task",
                "description": "Update a task's status, description, priority, or assignment. Use to mark tasks done, reassign, or add context.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "UUID of the task to update"
                        },
                        "status": {
                            "type": "string",
                            "description": "New status: 'todo', 'in_progress', 'done', 'cancelled'"
                        },
                        "title": {
                            "type": "string",
                            "description": "Updated title"
                        },
                        "description": {
                            "type": "string",
                            "description": "Updated description"
                        },
                        "priority": {
                            "type": "string",
                            "description": "Priority: 'low', 'medium', 'high', 'critical'"
                        },
                        "assigned_agent": {
                            "type": "string",
                            "description": "Agent name to assign/reassign to"
                        }
                    },
                    "required": ["task_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_tasks",
                "description": "List tasks for a project with filtering by status, priority, or agent. Returns titles, statuses, and latest execution info.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Project UUID (optional — inferred if user has one project)"
                        },
                        "status": {
                            "type": "string",
                            "description": "Filter: 'todo', 'in_progress', 'done', 'in_review'"
                        },
                        "assigned_agent": {
                            "type": "string",
                            "description": "Filter by agent name"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results (default 20)"
                        }
                    }
                }
            }
        }),
        // ==================== DESTRUCTIVE / BULK TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "delete_task",
                "description": "Delete a task permanently. This is destructive — it cannot be undone. Consider cancelling instead (update_task with status='cancelled').",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "UUID of the task to delete"
                        }
                    },
                    "required": ["task_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "bulk_update_tasks",
                "description": "Update multiple tasks at once with the same field values. Useful for batch status changes, priority updates, or reassignments.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_ids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Array of task UUIDs to update (max 100)"
                        },
                        "status": {
                            "type": "string",
                            "description": "New status for all tasks: 'todo', 'in_progress', 'done', 'cancelled'"
                        },
                        "priority": {
                            "type": "string",
                            "description": "New priority: 'low', 'medium', 'high', 'critical'"
                        },
                        "assigned_agent": {
                            "type": "string",
                            "description": "Agent name to assign all tasks to"
                        }
                    },
                    "required": ["task_ids"]
                }
            }
        }),
        // Web access tools
        json!({
            "type": "function",
            "function": {
                "name": "search_web",
                "description": "Search the internet in real-time. Use when asked to find, look up, or research anything online.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Number of results to return (default: 5)"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "fetch_web_page",
                "description": "Fetch and read the content of any URL. Use when asked to open, read, or access a specific web page.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch"
                        }
                    },
                    "required": ["url"]
                }
            }
        }),
        // ==================== CRM & WORKFLOW TOOLS ====================
        json!({
            "type": "function",
            "function": {
                "name": "get_project_detail",
                "description": "Get detailed project info including task counts, recent activity, organization, and client info.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "UUID of the project to get details for"
                        }
                    },
                    "required": ["project_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_crm_contacts",
                "description": "List CRM contacts for an organization. Returns names, emails, companies, lifecycle stage, lead score.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "organization_id": {
                            "type": "string",
                            "description": "UUID of the organization to list contacts for"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max number of contacts to return (default: 50)"
                        },
                        "lifecycle_stage": {
                            "type": "string",
                            "description": "Filter by lifecycle stage: 'subscriber', 'lead', 'mql', 'sql', 'opportunity', 'customer'"
                        },
                        "search_query": {
                            "type": "string",
                            "description": "Search by name, email, or company name"
                        }
                    },
                    "required": ["organization_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_crm_deals",
                "description": "List CRM deals for an organization or pipeline. Returns deal names, amounts, stages, contacts.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "organization_id": {
                            "type": "string",
                            "description": "UUID of the organization to list deals for"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "UUID of a specific pipeline to list deals for"
                        },
                        "stage_id": {
                            "type": "string",
                            "description": "UUID of a specific stage to list deals for"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_crm_pipelines",
                "description": "List CRM pipelines and their stages for an organization.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "organization_id": {
                            "type": "string",
                            "description": "UUID of the organization to list pipelines for"
                        },
                        "pipeline_type": {
                            "type": "string",
                            "description": "Filter by pipeline type: 'sales', 'clients', 'conferences', 'delivery', 'custom'"
                        }
                    },
                    "required": ["organization_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_workflow_definitions",
                "description": "List saved workflow definitions. Returns names, descriptions, owner info.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "get_workflow_definition",
                "description": "Get a full workflow definition including all nodes and connections.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "workflow_id": {
                            "type": "string",
                            "description": "UUID of the workflow definition to retrieve"
                        }
                    },
                    "required": ["workflow_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_entities",
                "description": "Search across projects, contacts, deals, and tasks by keyword.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search keyword to match against entity names, titles, emails, descriptions"
                        },
                        "entity_types": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Entity types to search: 'projects', 'contacts', 'deals', 'tasks'"
                        },
                        "organization_id": {
                            "type": "string",
                            "description": "Optional org UUID to scope contacts and deals search"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results per entity type (default: 20)"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        // ── Workflow execution tools ──────────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "list_workflow_runs",
                "description": "List recent workflow execution runs. Filter by workflow_id or organization_id.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "workflow_id": {
                            "type": "string",
                            "description": "Filter runs by workflow definition ID"
                        },
                        "organization_id": {
                            "type": "string",
                            "description": "Filter runs by organization UUID"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results (default: 20)"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "get_workflow_run_status",
                "description": "Get status and details of a specific workflow run, including staged record counts.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "UUID of the workflow run"
                        }
                    },
                    "required": ["run_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "review_staged_data",
                "description": "Show staged CRM/task records pending review for a workflow run or organization.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "UUID of the workflow run to review staged records for"
                        },
                        "organization_id": {
                            "type": "string",
                            "description": "UUID of the organization to show all pending staged records"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "approve_staged_records",
                "description": "Approve staged records from a workflow run. Approves valid non-duplicate records and rejects duplicates.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "UUID of the workflow run whose records to approve"
                        }
                    },
                    "required": ["run_id"]
                }
            }
        }),
        // ── CRM write tools ───────────────────────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "create_crm_contact",
                "description": "Create a new CRM contact in an organization.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "organization_id": { "type": "string", "description": "Organization UUID (required)" },
                        "first_name": { "type": "string" },
                        "last_name": { "type": "string" },
                        "email": { "type": "string" },
                        "phone": { "type": "string" },
                        "company_name": { "type": "string" },
                        "job_title": { "type": "string" },
                        "linkedin_url": { "type": "string" },
                        "lifecycle_stage": { "type": "string", "enum": ["subscriber","lead","mql","sql","opportunity","customer","evangelist"] }
                    },
                    "required": ["organization_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "create_crm_deal",
                "description": "Create a new CRM deal in an organization.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "organization_id": { "type": "string", "description": "Organization UUID (required)" },
                        "name": { "type": "string", "description": "Deal name (required)" },
                        "amount": { "type": "number" },
                        "currency": { "type": "string", "description": "e.g. USD, EUR" },
                        "pipeline_id": { "type": "string", "description": "Pipeline UUID" },
                        "stage_id": { "type": "string", "description": "Stage UUID" },
                        "contact_id": { "type": "string", "description": "Associated CRM contact UUID" },
                        "description": { "type": "string" },
                        "expected_close_date": { "type": "string", "description": "ISO date, e.g. 2026-06-01" }
                    },
                    "required": ["organization_id", "name"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "update_crm_deal",
                "description": "Update fields on an existing CRM deal.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "deal_id": { "type": "string", "description": "Deal UUID (required)" },
                        "name": { "type": "string" },
                        "amount": { "type": "number" },
                        "currency": { "type": "string" },
                        "stage_id": { "type": "string" },
                        "description": { "type": "string" },
                        "expected_close_date": { "type": "string" },
                        "lost_reason": { "type": "string" },
                        "win_reason": { "type": "string" }
                    },
                    "required": ["deal_id"]
                }
            }
        }),
        // ── Workflow builder delegation ──────────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "build_workflow",
                "description": "Delegate workflow creation or modification to the Workflow Builder specialist agent. Provide the user's request and any context you've gathered (existing workflows, CRM schema info, project details). The specialist will generate the node graph and save it. Use this when the user wants to create a new workflow or modify an existing one.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["create", "modify"],
                            "description": "'create' for new workflows, 'modify' to update existing"
                        },
                        "user_request": {
                            "type": "string",
                            "description": "The user's natural language description of what they want the workflow to do"
                        },
                        "context": {
                            "type": "string",
                            "description": "Any context you've gathered: existing workflow definitions, CRM data shapes, project info, org details. Include anything that helps the builder make good decisions."
                        },
                        "workflow_id": {
                            "type": "string",
                            "description": "For 'modify' action: the ID of the existing workflow to update"
                        },
                        "owner_id": {
                            "type": "string",
                            "description": "Organization UUID that should own this workflow"
                        }
                    },
                    "required": ["action", "user_request"]
                }
            }
        }),
        // Chat/response tool for conversational replies
        json!({
            "type": "function",
            "function": {
                "name": "respond_to_user",
                "description": "Send YOUR response to the user. IMPORTANT: You must compose and write out your complete response in the 'message' parameter. This is how you communicate with users - by writing your actual response text here. For example, if a user asks 'tell me a story', you should write an actual story in the message parameter, NOT just echo 'tell me a story'. Always use this tool to deliver your response to the user.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "YOUR complete response text that will be spoken/shown to the user. Write your actual response here - if asked for a story, write the story; if asked a question, write your answer; if greeting, write your greeting."
                        }
                    },
                    "required": ["message"]
                }
            }
        }),
    ]
}

/// Get all tool names from the canonical schema definitions.
///
/// Extracts the `"name"` field from each tool in `get_tool_schemas()`,
/// excluding `respond_to_user` (internal plumbing, not a real tool).
pub fn get_tool_names() -> Vec<String> {
    get_tool_schemas()
        .iter()
        .filter_map(|schema| {
            schema
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .filter(|&name| name != "respond_to_user")
                .map(|s| s.to_string())
        })
        .collect()
}

/// Get all available Topsi tools
pub fn get_topsi_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_nodes".to_string(),
            description: "List all nodes in the topology, optionally filtered by type or status"
                .to_string(),
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
            description: "Detect issues in the topology (bottlenecks, cycles, isolated nodes)"
                .to_string(),
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

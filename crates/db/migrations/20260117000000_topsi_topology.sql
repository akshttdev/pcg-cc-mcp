-- Topsi Topology Tables
-- Topological Super Intelligence - maintains a living topology of the project ecosystem

-- Project topology nodes
-- Represents all entities in the project: agents, tasks, resources, accounts, workflows
CREATE TABLE topology_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    node_type TEXT NOT NULL CHECK (node_type IN ('agent', 'task', 'resource', 'account', 'workflow')),
    ref_id TEXT NOT NULL,     -- Reference to actual entity (agent.id, task.id, etc.)
    capabilities TEXT,        -- JSON array of capability strings
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'degraded', 'failed')),
    metadata TEXT,            -- JSON object with additional node-specific data
    weight REAL DEFAULT 1.0,  -- Node importance/capacity weight
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_topology_nodes_project ON topology_nodes(project_id);
CREATE INDEX idx_topology_nodes_type ON topology_nodes(node_type);
CREATE INDEX idx_topology_nodes_ref ON topology_nodes(ref_id);
CREATE INDEX idx_topology_nodes_status ON topology_nodes(status);

-- Topology edges (connections between nodes)
-- Represents relationships: capabilities, dependencies, data flows, access permissions
CREATE TABLE topology_edges (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    from_node_id TEXT NOT NULL,
    to_node_id TEXT NOT NULL,
    edge_type TEXT NOT NULL CHECK (edge_type IN (
        'can_execute',      -- Agent can execute task type
        'has_access',       -- Agent has access to account/resource
        'depends_on',       -- Task depends on another task
        'produces_for',     -- Workflow produces output for another
        'belongs_to',       -- Node belongs to cluster
        'flows_to',         -- Data flows from source to sink
        'supervises',       -- Agent supervises another agent
        'triggers'          -- Event/workflow triggers another
    )),
    weight REAL DEFAULT 1.0,  -- Edge strength/capacity
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'degraded')),
    metadata TEXT,            -- JSON object with edge-specific data
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (from_node_id) REFERENCES topology_nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (to_node_id) REFERENCES topology_nodes(id) ON DELETE CASCADE
);

CREATE INDEX idx_topology_edges_project ON topology_edges(project_id);
CREATE INDEX idx_topology_edges_from ON topology_edges(from_node_id);
CREATE INDEX idx_topology_edges_to ON topology_edges(to_node_id);
CREATE INDEX idx_topology_edges_type ON topology_edges(edge_type);

-- Topology clusters (dynamic teams/groups)
-- Represents dynamically formed groups of nodes for coordinated work
CREATE TABLE topology_clusters (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    purpose TEXT,             -- Cluster purpose: "creative_team", "content_pipeline", etc.
    node_ids TEXT NOT NULL,   -- JSON array of node IDs
    leader_node_id TEXT,      -- Optional cluster leader
    is_active INTEGER NOT NULL DEFAULT 1,
    formed_at TEXT NOT NULL DEFAULT (datetime('now')),
    dissolved_at TEXT,
    metadata TEXT,            -- JSON object with cluster-specific data
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (leader_node_id) REFERENCES topology_nodes(id) ON DELETE SET NULL
);

CREATE INDEX idx_topology_clusters_project ON topology_clusters(project_id);
CREATE INDEX idx_topology_clusters_active ON topology_clusters(is_active);

-- Topology invariants (rules to enforce)
-- Defines rules the topology must satisfy
CREATE TABLE topology_invariants (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    invariant_type TEXT NOT NULL CHECK (invariant_type IN (
        'no_cycles',           -- Prevent circular dependencies
        'path_exists',         -- Ensure path exists between nodes
        'adjacency_required',  -- Require certain nodes to be connected
        'max_degree',          -- Limit connections to a node
        'min_capacity',        -- Ensure minimum capacity on paths
        'isolation'            -- Prevent certain connections
    )),
    rule TEXT NOT NULL,       -- JSON rule definition
    severity TEXT NOT NULL DEFAULT 'warning' CHECK (severity IN ('info', 'warning', 'error', 'critical')),
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_checked_at TEXT,
    last_violation_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_topology_invariants_project ON topology_invariants(project_id);
CREATE INDEX idx_topology_invariants_type ON topology_invariants(invariant_type);

-- Topology snapshots (for history/debugging)
-- Stores complete topology state at points in time
CREATE TABLE topology_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    snapshot TEXT NOT NULL,   -- Full topology as JSON
    trigger TEXT,             -- What caused the snapshot: "manual", "auto", "issue_detected", etc.
    node_count INTEGER,
    edge_count INTEGER,
    cluster_count INTEGER,
    issues_detected TEXT,     -- JSON array of detected issues
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_topology_snapshots_project ON topology_snapshots(project_id);
CREATE INDEX idx_topology_snapshots_created ON topology_snapshots(created_at);

-- Topology issues (detected problems)
-- Tracks detected issues in the topology
CREATE TABLE topology_issues (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    issue_type TEXT NOT NULL CHECK (issue_type IN (
        'bottleneck',          -- Node overloaded
        'hole',                -- Missing capability
        'cycle',               -- Circular dependency detected
        'orphan',              -- Disconnected node
        'degraded_path',       -- Path with failing edges
        'invariant_violation', -- Rule violation
        'capacity_exceeded',   -- Capacity limits exceeded
        'dead_end'             -- Node with no outgoing paths
    )),
    severity TEXT NOT NULL DEFAULT 'warning' CHECK (severity IN ('info', 'warning', 'error', 'critical')),
    affected_nodes TEXT,      -- JSON array of affected node IDs
    affected_edges TEXT,      -- JSON array of affected edge IDs
    description TEXT NOT NULL,
    suggested_action TEXT,    -- Recommended resolution
    resolved_at TEXT,
    resolution_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_topology_issues_project ON topology_issues(project_id);
CREATE INDEX idx_topology_issues_type ON topology_issues(issue_type);
CREATE INDEX idx_topology_issues_unresolved ON topology_issues(project_id, resolved_at) WHERE resolved_at IS NULL;

-- Topology routes (planned execution paths)
-- Stores planned routes through the topology for task execution
CREATE TABLE topology_routes (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    goal TEXT NOT NULL,       -- What this route accomplishes
    path TEXT NOT NULL,       -- JSON array of node IDs in order
    edges TEXT NOT NULL,      -- JSON array of edge IDs used
    total_weight REAL,        -- Sum of edge weights (cost)
    status TEXT NOT NULL DEFAULT 'planned' CHECK (status IN ('planned', 'executing', 'completed', 'failed', 'rerouted')),
    started_at TEXT,
    completed_at TEXT,
    rerouted_from TEXT,       -- ID of original route if this is a reroute
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (rerouted_from) REFERENCES topology_routes(id) ON DELETE SET NULL
);

CREATE INDEX idx_topology_routes_project ON topology_routes(project_id);
CREATE INDEX idx_topology_routes_status ON topology_routes(status);

-- Extend project_controller_config for Topsi
ALTER TABLE project_controller_config ADD COLUMN enabled_tools TEXT DEFAULT '[]';
ALTER TABLE project_controller_config ADD COLUMN topology_auto_refresh INTEGER DEFAULT 1;
ALTER TABLE project_controller_config ADD COLUMN autonomy_level TEXT DEFAULT 'supervised';

-- Extend project_controller_messages for topology-aware responses
ALTER TABLE project_controller_messages ADD COLUMN tool_call_id TEXT;
ALTER TABLE project_controller_messages ADD COLUMN tool_name TEXT;
ALTER TABLE project_controller_messages ADD COLUMN tool_arguments TEXT;
ALTER TABLE project_controller_messages ADD COLUMN tool_result TEXT;
ALTER TABLE project_controller_messages ADD COLUMN topology_context TEXT;
ALTER TABLE project_controller_messages ADD COLUMN route_taken TEXT;
ALTER TABLE project_controller_messages ADD COLUMN input_tokens INTEGER;
ALTER TABLE project_controller_messages ADD COLUMN output_tokens INTEGER;

-- Update default controller name to Topsi
UPDATE project_controller_config SET name = 'Topsi' WHERE name = 'Project Assistant' OR name = 'Controller';

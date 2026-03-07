-- Entity Graph Extension
-- Business entity graph nodes and edges for CRM topology visualization.
-- Separate from topology_nodes/edges (which are project-scoped execution graphs).
-- This graph covers the CRM domain: companies, persons, orgs, proposals, projects.

CREATE TABLE IF NOT EXISTS entity_graph_nodes (
    id        BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    node_type TEXT NOT NULL,  -- 'company'|'person'|'organization'|'proposal'|'project'
    ref_id    TEXT NOT NULL,  -- UUID hex of referenced entity
    ref_table TEXT NOT NULL,  -- 'companies'|'persons'|'organizations'|'proposals'|'projects'
    label     TEXT NOT NULL,
    metadata  TEXT,           -- JSON
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(node_type, ref_id)
);

CREATE TABLE IF NOT EXISTS entity_graph_edges (
    id           BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    from_node_id BLOB NOT NULL REFERENCES entity_graph_nodes(id) ON DELETE CASCADE,
    to_node_id   BLOB NOT NULL REFERENCES entity_graph_nodes(id) ON DELETE CASCADE,
    edge_type    TEXT NOT NULL,
    -- 'employs'     Company→Person
    -- 'targets'     Proposal→Company
    -- 'lead_on'     Person→Proposal
    -- 'part_of_org' Company→Organization
    -- 'manages'     Organization→Person (managing org)
    weight    REAL DEFAULT 1.0,
    metadata  TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(from_node_id, to_node_id, edge_type)
);

CREATE INDEX IF NOT EXISTS idx_egn_ref ON entity_graph_nodes(node_type, ref_id);
CREATE INDEX IF NOT EXISTS idx_ege_from ON entity_graph_edges(from_node_id);
CREATE INDEX IF NOT EXISTS idx_ege_to ON entity_graph_edges(to_node_id);

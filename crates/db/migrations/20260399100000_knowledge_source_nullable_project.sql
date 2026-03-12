-- Make project_id nullable in project_knowledge_sources
-- Required for company/org/user-owned knowledge sources (polymorphic owner_type)

-- Drop dependents first
DROP VIEW IF EXISTS v_project_knowledge_completeness;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_conversation;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_pulse_content;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_entity_appearance;
DROP TRIGGER IF EXISTS trg_knowledge_update_entity_appearance;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_topology_snapshot;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_artifact;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_context_injection;

CREATE TABLE project_knowledge_sources_new (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_title TEXT NOT NULL,
    source_summary TEXT,
    coverage_score REAL NOT NULL DEFAULT 0.0,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_stale INTEGER NOT NULL DEFAULT 0,
    auto_registered INTEGER NOT NULL DEFAULT 0,
    last_refreshed_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    owner_type TEXT NOT NULL DEFAULT 'project',
    owner_id TEXT,
    UNIQUE(project_id, source_type, source_id)
);

INSERT INTO project_knowledge_sources_new SELECT * FROM project_knowledge_sources;
DROP TABLE project_knowledge_sources;
ALTER TABLE project_knowledge_sources_new RENAME TO project_knowledge_sources;

-- Index for company/org/user owner lookups
CREATE INDEX IF NOT EXISTS idx_pks_owner ON project_knowledge_sources(owner_type, owner_id) WHERE owner_id IS NOT NULL;

-- Recreate view
CREATE VIEW v_project_knowledge_completeness AS
SELECT
    p.id AS project_id,
    COALESCE(src.total_sources, 0)   AS total_sources,
    COALESCE(src.fresh_sources, 0)   AS fresh_sources,
    COALESCE(src.avg_coverage, 0.0)  AS avg_coverage,
    COALESCE(src.type_count, 0)      AS type_count,
    ROUND(
        0.4 * (COALESCE(src.type_count, 0) / 6.0)
        + 0.6 * COALESCE(src.avg_coverage, 0.0),
        4
    ) AS knowledge_completeness
FROM projects p
LEFT JOIN (
    SELECT
        project_id,
        COUNT(*)                                              AS total_sources,
        SUM(CASE WHEN is_stale = 0 THEN 1 ELSE 0 END)       AS fresh_sources,
        AVG(coverage_score)                                   AS avg_coverage,
        COUNT(DISTINCT source_type)                           AS type_count
    FROM project_knowledge_sources
    WHERE is_active = 1 AND project_id IS NOT NULL
    GROUP BY project_id
) src ON src.project_id = p.id;

-- Recreate triggers
CREATE TRIGGER trg_knowledge_auto_register_conversation
AFTER INSERT ON agent_conversations
WHEN NEW.project_id IS NOT NULL
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, coverage_score, auto_registered)
    VALUES (randomblob(16), NEW.project_id, 'conversation', hex(NEW.id), COALESCE(NEW.title, 'Agent Conversation'), 0.3, 1)
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_auto_register_pulse_content
AFTER INSERT ON pulse_content_items
WHEN NEW.project_id IS NOT NULL
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    VALUES (randomblob(16), NEW.project_id, 'pulse_content', hex(NEW.id), COALESCE(NEW.title, 'Pulse Content'), NEW.summary, 0.4, 1)
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_auto_register_entity_appearance
AFTER INSERT ON entity_appearances
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    SELECT randomblob(16), pb.project_id, 'entity', hex(NEW.id),
        COALESCE(e.canonical_name || ' (' || NEW.appearance_type || ')', 'Entity Appearance'),
        COALESCE(NEW.talk_title, e.bio),
        CASE NEW.appearance_type WHEN 'keynote' THEN 0.7 WHEN 'speaker' THEN 0.6 WHEN 'panelist' THEN 0.5 WHEN 'sponsor' THEN 0.5 WHEN 'workshop_leader' THEN 0.6 ELSE 0.4 END,
        1
    FROM project_boards pb JOIN entities e ON e.id = NEW.entity_id
    WHERE pb.id = NEW.conference_board_id AND pb.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_update_entity_appearance
AFTER UPDATE OF status ON entity_appearances
WHEN NEW.status IN ('researched', 'verified', 'content_created')
BEGIN
    UPDATE project_knowledge_sources SET
        coverage_score = CASE NEW.status WHEN 'researched' THEN MAX(coverage_score, 0.7) WHEN 'verified' THEN MAX(coverage_score, 0.85) WHEN 'content_created' THEN MAX(coverage_score, 1.0) ELSE coverage_score END,
        is_stale = 0,
        last_refreshed_at = datetime('now', 'subsec'),
        updated_at = datetime('now', 'subsec')
    WHERE source_type = 'entity' AND source_id = hex(NEW.id);
END;

CREATE TRIGGER trg_knowledge_auto_register_topology_snapshot
AFTER INSERT ON topology_snapshots
WHEN NEW.project_id IS NOT NULL
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    VALUES (randomblob(16), unhex(REPLACE(NEW.project_id, '-', '')), 'topology_snapshot', NEW.id,
        'Topology Snapshot' || CASE WHEN NEW.trigger IS NOT NULL THEN ' (' || NEW.trigger || ')' ELSE '' END,
        CASE WHEN NEW.issues_detected IS NOT NULL THEN NEW.issues_detected ELSE 'Nodes: ' || COALESCE(NEW.node_count, 0) || ', Edges: ' || COALESCE(NEW.edge_count, 0) || ', Clusters: ' || COALESCE(NEW.cluster_count, 0) END,
        0.5, 1)
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        last_refreshed_at = datetime('now', 'subsec'),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_auto_register_artifact
AFTER INSERT ON execution_artifacts
WHEN NEW.artifact_type IN ('research_report','strategy_document','content_calendar','competitor_analysis','content_draft','visual_brief','verification_report','aggregated_research')
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    SELECT randomblob(16), t.project_id, 'artifact', hex(NEW.id), COALESCE(NEW.title, 'Artifact'), SUBSTR(COALESCE(NEW.content, ''), 1, 500),
        CASE NEW.artifact_type WHEN 'research_report' THEN 0.6 WHEN 'strategy_document' THEN 0.7 WHEN 'competitor_analysis' THEN 0.6 WHEN 'aggregated_research' THEN 0.7 WHEN 'content_draft' THEN 0.4 WHEN 'visual_brief' THEN 0.4 WHEN 'content_calendar' THEN 0.5 WHEN 'verification_report' THEN 0.5 ELSE 0.3 END, 1
    FROM execution_processes ep JOIN task_attempts ta ON ta.id = ep.task_attempt_id JOIN tasks t ON t.id = ta.task_id
    WHERE ep.id = NEW.execution_process_id AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_auto_register_context_injection
AFTER INSERT ON context_injections
WHEN NEW.injection_type IN ('note', 'correction', 'directive', 'answer')
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    SELECT randomblob(16), t.project_id, 'context_injection', hex(NEW.id),
        COALESCE(NEW.injector_name || ': ' || NEW.injection_type, 'Context: ' || NEW.injection_type),
        SUBSTR(NEW.content, 1, 500),
        CASE NEW.injection_type WHEN 'correction' THEN 0.8 WHEN 'directive' THEN 0.7 WHEN 'answer' THEN 0.6 WHEN 'note' THEN 0.5 ELSE 0.4 END, 1
    FROM execution_processes ep JOIN task_attempts ta ON ta.id = ep.task_attempt_id JOIN tasks t ON t.id = ta.task_id
    WHERE ep.id = NEW.execution_process_id AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

-- Project Knowledge Sources (Knowledge Sheaf)
-- Unifies all per-project knowledge into one queryable structure
-- with auto-registration via triggers (functorial mapping from source categories)

CREATE TABLE IF NOT EXISTS project_knowledge_sources (
    id              BLOB PRIMARY KEY NOT NULL,
    project_id      BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_type     TEXT NOT NULL CHECK (source_type IN (
        'conversation', 'artifact', 'pulse_content',
        'context_injection', 'entity', 'topology_snapshot'
    )),
    source_id       TEXT NOT NULL,           -- ID of the source row (as hex string for BLOB PKs)
    source_title    TEXT NOT NULL DEFAULT '',
    source_summary  TEXT,
    coverage_score  REAL NOT NULL DEFAULT 0.0 CHECK (coverage_score >= 0.0 AND coverage_score <= 1.0),
    is_active       INTEGER NOT NULL DEFAULT 1,
    is_stale        INTEGER NOT NULL DEFAULT 0,
    auto_registered INTEGER NOT NULL DEFAULT 0,
    last_refreshed_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Unique constraint for upsert support
CREATE UNIQUE INDEX IF NOT EXISTS idx_pks_project_source
    ON project_knowledge_sources(project_id, source_type, source_id);

-- Query indexes
CREATE INDEX IF NOT EXISTS idx_pks_project_id
    ON project_knowledge_sources(project_id);
CREATE INDEX IF NOT EXISTS idx_pks_source_type
    ON project_knowledge_sources(source_type);
CREATE INDEX IF NOT EXISTS idx_pks_is_active
    ON project_knowledge_sources(is_active);
CREATE INDEX IF NOT EXISTS idx_pks_is_stale
    ON project_knowledge_sources(is_stale);

-- View: per-project knowledge completeness
-- 40% breadth (how many of 6 source types present) + 60% depth (avg coverage_score)
CREATE VIEW IF NOT EXISTS v_project_knowledge_completeness AS
SELECT
    p.id AS project_id,
    COALESCE(src.total_sources, 0)   AS total_sources,
    COALESCE(src.fresh_sources, 0)   AS fresh_sources,
    COALESCE(src.avg_coverage, 0.0)  AS avg_coverage,
    COALESCE(src.type_count, 0)      AS type_count,
    -- Completeness = 40% breadth + 60% depth
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
    WHERE is_active = 1
    GROUP BY project_id
) src ON src.project_id = p.id;

-- Trigger: auto-register agent_conversations as knowledge sources on INSERT
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_conversation
AFTER INSERT ON agent_conversations
WHEN NEW.project_id IS NOT NULL
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title, coverage_score, auto_registered
    )
    VALUES (
        randomblob(16),
        NEW.project_id,
        'conversation',
        hex(NEW.id),
        COALESCE(NEW.title, 'Agent Conversation'),
        0.3,
        1
    )
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        updated_at = datetime('now', 'subsec');
END;

-- Trigger: auto-register pulse_content_items as knowledge sources on INSERT
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_pulse_content
AFTER INSERT ON pulse_content_items
WHEN NEW.project_id IS NOT NULL
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered
    )
    VALUES (
        randomblob(16),
        NEW.project_id,
        'pulse_content',
        hex(NEW.id),
        COALESCE(NEW.title, 'Pulse Content'),
        NEW.summary,
        0.4,
        1
    )
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        updated_at = datetime('now', 'subsec');
END;

-- Complete the Knowledge Sheaf: add auto-registration triggers for all 6 source types
--
-- Currently only 2/6 source types have triggers (conversation, pulse_content).
-- This migration adds triggers for the remaining 4:
--   - execution_artifacts → 'artifact' (via execution_processes → tasks → project_id)
--   - context_injections  → 'context_injection' (via execution_processes → tasks → project_id)
--   - entity_appearances  → 'entity' (via project_boards → project_id)
--   - topology_snapshots  → 'topology_snapshot' (direct project_id, TEXT→BLOB conversion)
--
-- Each trigger implements a functorial mapping from its source category into the
-- unified project_knowledge_sources sheaf, preserving the project_id relationship
-- through whatever intermediate joins are required.

-- ============================================================================
-- 1. execution_artifacts → 'artifact'
--    Path: execution_artifacts.execution_process_id
--        → execution_processes.task_id → tasks.project_id
-- ============================================================================
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_artifact
AFTER INSERT ON execution_artifacts
WHEN NEW.artifact_type IN (
    'research_report', 'strategy_document', 'content_calendar',
    'competitor_analysis', 'content_draft', 'visual_brief',
    'verification_report', 'aggregated_research'
)
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title,
        source_summary, coverage_score, auto_registered
    )
    SELECT
        randomblob(16),
        t.project_id,
        'artifact',
        hex(NEW.id),
        COALESCE(NEW.title, 'Artifact'),
        SUBSTR(COALESCE(NEW.content, ''), 1, 500),
        CASE NEW.artifact_type
            WHEN 'research_report' THEN 0.6
            WHEN 'strategy_document' THEN 0.7
            WHEN 'competitor_analysis' THEN 0.6
            WHEN 'aggregated_research' THEN 0.7
            WHEN 'content_draft' THEN 0.4
            WHEN 'visual_brief' THEN 0.4
            WHEN 'content_calendar' THEN 0.5
            WHEN 'verification_report' THEN 0.5
            ELSE 0.3
        END,
        1
    FROM execution_processes ep
    JOIN tasks t ON t.id = ep.task_id
    WHERE ep.id = NEW.execution_process_id
      AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

-- ============================================================================
-- 2. context_injections → 'context_injection'
--    Path: context_injections.execution_process_id
--        → execution_processes.task_id → tasks.project_id
--    Human-validated knowledge (corrections, directives, notes) gets high coverage.
-- ============================================================================
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_context_injection
AFTER INSERT ON context_injections
WHEN NEW.injection_type IN ('note', 'correction', 'directive', 'answer')
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title,
        source_summary, coverage_score, auto_registered
    )
    SELECT
        randomblob(16),
        t.project_id,
        'context_injection',
        hex(NEW.id),
        COALESCE(
            NEW.injector_name || ': ' || NEW.injection_type,
            'Context: ' || NEW.injection_type
        ),
        SUBSTR(NEW.content, 1, 500),
        CASE NEW.injection_type
            WHEN 'correction' THEN 0.8
            WHEN 'directive' THEN 0.7
            WHEN 'answer' THEN 0.6
            WHEN 'note' THEN 0.5
            ELSE 0.4
        END,
        1
    FROM execution_processes ep
    JOIN tasks t ON t.id = ep.task_id
    WHERE ep.id = NEW.execution_process_id
      AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

-- ============================================================================
-- 3. entity_appearances → 'entity'
--    Path: entity_appearances.conference_board_id
--        → project_boards.project_id
--    Registers discovered entities (speakers, sponsors, etc.) as project knowledge.
-- ============================================================================
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_entity_appearance
AFTER INSERT ON entity_appearances
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title,
        source_summary, coverage_score, auto_registered
    )
    SELECT
        randomblob(16),
        pb.project_id,
        'entity',
        hex(NEW.id),
        COALESCE(
            e.canonical_name || ' (' || NEW.appearance_type || ')',
            'Entity Appearance'
        ),
        COALESCE(NEW.talk_title, e.bio),
        CASE NEW.appearance_type
            WHEN 'keynote' THEN 0.7
            WHEN 'speaker' THEN 0.6
            WHEN 'panelist' THEN 0.5
            WHEN 'sponsor' THEN 0.5
            WHEN 'workshop_leader' THEN 0.6
            ELSE 0.4
        END,
        1
    FROM project_boards pb
    JOIN entities e ON e.id = NEW.entity_id
    WHERE pb.id = NEW.conference_board_id
      AND pb.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

-- Also register when entity research status advances (status UPDATE)
CREATE TRIGGER IF NOT EXISTS trg_knowledge_update_entity_appearance
AFTER UPDATE OF status ON entity_appearances
WHEN NEW.status IN ('researched', 'verified', 'content_created')
BEGIN
    UPDATE project_knowledge_sources
    SET coverage_score = CASE NEW.status
            WHEN 'researched' THEN MAX(coverage_score, 0.7)
            WHEN 'verified' THEN MAX(coverage_score, 0.85)
            WHEN 'content_created' THEN MAX(coverage_score, 1.0)
            ELSE coverage_score
        END,
        is_stale = 0,
        last_refreshed_at = datetime('now', 'subsec'),
        updated_at = datetime('now', 'subsec')
    WHERE source_type = 'entity'
      AND source_id = hex(NEW.id);
END;

-- ============================================================================
-- 4. topology_snapshots → 'topology_snapshot'
--    topology_snapshots.project_id is TEXT (hex), while
--    project_knowledge_sources.project_id is BLOB. We use unhex() to convert.
-- ============================================================================
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_topology_snapshot
AFTER INSERT ON topology_snapshots
WHEN NEW.project_id IS NOT NULL
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title,
        source_summary, coverage_score, auto_registered
    )
    VALUES (
        randomblob(16),
        unhex(REPLACE(NEW.project_id, '-', '')),
        'topology_snapshot',
        NEW.id,
        'Topology Snapshot' || CASE
            WHEN NEW.trigger IS NOT NULL THEN ' (' || NEW.trigger || ')'
            ELSE ''
        END,
        CASE
            WHEN NEW.issues_detected IS NOT NULL THEN NEW.issues_detected
            ELSE 'Nodes: ' || COALESCE(NEW.node_count, 0) ||
                 ', Edges: ' || COALESCE(NEW.edge_count, 0) ||
                 ', Clusters: ' || COALESCE(NEW.cluster_count, 0)
        END,
        0.5,
        1
    )
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        last_refreshed_at = datetime('now', 'subsec'),
        updated_at = datetime('now', 'subsec');
END;

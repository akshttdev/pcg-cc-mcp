-- Fix triggers that incorrectly reference ep.task_id
-- execution_processes does NOT have task_id; the correct join is:
--   execution_processes.task_attempt_id -> task_attempts.task_id -> tasks.id

DROP TRIGGER IF EXISTS trg_knowledge_auto_register_artifact;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_context_injection;

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
    JOIN task_attempts ta ON ta.id = ep.task_attempt_id
    JOIN tasks t ON t.id = ta.task_id
    WHERE ep.id = NEW.execution_process_id
      AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

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
    JOIN task_attempts ta ON ta.id = ep.task_attempt_id
    JOIN tasks t ON t.id = ta.task_id
    WHERE ep.id = NEW.execution_process_id
      AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

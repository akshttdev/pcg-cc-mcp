-- Seed Le Chateau Editron data for dashboard visibility
-- Backfills media_batches, media_files, execution_artifacts, task, and task_artifacts
-- All inserts use INSERT OR IGNORE with deterministic IDs — safe to re-run.

-- ============================================================================
-- 0. FIX: trg_knowledge_auto_register_artifact references ep.task_id which
--    does not exist on execution_processes (should go through task_attempts).
--    Drop and recreate with the corrected join so artifact INSERTs don't fail.
-- ============================================================================

DROP TRIGGER IF EXISTS trg_knowledge_auto_register_artifact;

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

-- ============================================================================
-- 1. MEDIA BATCH (TEXT PK)
-- ============================================================================

INSERT OR IGNORE INTO media_batches (
    id, project_id, reference_name, source_url, storage_tier,
    checksum_required, status, file_count, total_size_bytes,
    metadata, created_at, updated_at
) VALUES (
    'le-chateau-batch-001',
    'b0b1b2b3-b4b5-b6b7-b8b9-babbbcbdbebf',
    'Le Chateau Source Footage',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage',
    'hot',
    0,
    'analyzed',
    10,
    95995141,
    '{"client":"le-chateau","event_type":"venue_showcase","backfill":true}',
    '2026-02-26T14:12:00',
    '2026-02-26T14:19:00'
);

-- ============================================================================
-- 2. MEDIA FILES (10 rows, TEXT PKs)
-- ============================================================================

INSERT OR IGNORE INTO media_files (
    id, batch_id, filename, file_path, size_bytes,
    duration_seconds, resolution, codec, fps, metadata, created_at
) VALUES
(
    'le-chateau-file-01',
    'le-chateau-batch-001',
    '01-reel-DOlpGVbCEx6.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/01-reel-DOlpGVbCEx6.mp4',
    2218388,
    15.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-02',
    'le-chateau-batch-001',
    '02-main-DVESRjNCErn.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/02-main-DVESRjNCErn.mp4',
    2018469,
    12.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_post","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-03',
    'le-chateau-batch-001',
    '03-main-DTqHDYLiG2I.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/03-main-DTqHDYLiG2I.mp4',
    20470712,
    60.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_post","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-04',
    'le-chateau-batch-001',
    '04-zoom-in-DMDczhPMlwh.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/04-zoom-in-DMDczhPMlwh.mp4',
    4298689,
    20.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","content":"zoom_detail","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-05',
    'le-chateau-batch-001',
    '05-red-carpet-DM5fxbQobN4.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/05-red-carpet-DM5fxbQobN4.mp4',
    7184713,
    25.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","content":"red_carpet","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-06',
    'le-chateau-batch-001',
    '06-red-carpet-DMS2E0go_sd.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/06-red-carpet-DMS2E0go_sd.mp4',
    18418826,
    55.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","content":"red_carpet","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-07',
    'le-chateau-batch-001',
    '07-night-light-DLxbKX7I72m.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/07-night-light-DLxbKX7I72m.mp4',
    1147934,
    8.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","content":"night_ambiance","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-08',
    'le-chateau-batch-001',
    '08-drone-garden-DKXP83womBl.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/08-drone-garden-DKXP83womBl.mp4',
    15555179,
    45.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","content":"drone_aerial","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-09',
    'le-chateau-batch-001',
    '09-hero-construction-DJzR-aFIcch.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/09-hero-construction-DJzR-aFIcch.mp4',
    17361463,
    48.0, '1080x1920', 'vp9', 24.0,
    '{"source":"instagram_reel","content":"construction_progress","backfill":true}',
    '2026-02-26T14:12:00'
),
(
    'le-chateau-file-10',
    'le-chateau-batch-001',
    '10-hosting-DGa711WIHC6.mp4',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage/10-hosting-DGa711WIHC6.mp4',
    7320768,
    30.0, '1080x1920', 'vp9', 30.0,
    '{"source":"instagram_reel","content":"hosting_event","backfill":true}',
    '2026-02-26T14:12:00'
);

-- ============================================================================
-- 3. EXECUTION ARTIFACTS (4 rows, BLOB PKs — deterministic hex UUIDs)
-- ============================================================================

-- 3a. Ingest manifest
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, file_path, metadata, phase, created_at
) VALUES (
    X'1ec0a1ea000040008000000000000001',
    NULL,
    'media_ingest_manifest',
    'Ingest: Le Chateau (10 files)',
    '{"batch_id":"le-chateau-batch-001","file_count":10,"total_bytes":95995141,"source_dir":"/home/spaceterminal/topos/sirak-studios/clients/le-chateau/source-footage","files":["01-reel-DOlpGVbCEx6.mp4","02-main-DVESRjNCErn.mp4","03-main-DTqHDYLiG2I.mp4","04-zoom-in-DMDczhPMlwh.mp4","05-red-carpet-DM5fxbQobN4.mp4","06-red-carpet-DMS2E0go_sd.mp4","07-night-light-DLxbKX7I72m.mp4","08-drone-garden-DKXP83womBl.mp4","09-hero-construction-DJzR-aFIcch.mp4","10-hosting-DGa711WIHC6.mp4"]}',
    NULL,
    '{"phase":"execution","le-chateau":true,"backfill":true}',
    'execution',
    '2026-02-26T14:12:00'
);

-- 3b. Analysis report
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, file_path, metadata, phase, created_at
) VALUES (
    X'1ec0a1ea000040008000000000000002',
    NULL,
    'media_analysis_report',
    'Analysis: Le Chateau venue showcase',
    '{"batch_id":"le-chateau-batch-001","total_clips":10,"usable_clips":10,"content_types":{"red_carpet":2,"drone_aerial":1,"night_ambiance":1,"zoom_detail":1,"construction_progress":1,"hosting_event":1,"reel":1,"post":2},"avg_energy":0.65,"dominant_resolution":"1080x1920","dominant_codec":"vp9"}',
    NULL,
    '{"phase":"execution","le-chateau":true,"backfill":true}',
    'execution',
    '2026-02-26T14:15:00'
);

-- 3c. Edit session
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, file_path, metadata, phase, created_at
) VALUES (
    X'1ec0a1ea000040008000000000000003',
    NULL,
    'video_edit_session',
    'Edit Session: Le Chateau (3 edits)',
    '{"batch_id":"le-chateau-batch-001","edits":[{"name":"59s-FOMO-Edit","duration_seconds":59.0,"segments":10,"file":"Le-Chateau-59s-FOMO-Edit.mp4"},{"name":"29s-Condensed-Edit","duration_seconds":29.0,"segments":8,"file":"Le-Chateau-29s-Condensed-Edit.mp4"},{"name":"14.5s-Teaser-Edit","duration_seconds":14.5,"segments":6,"file":"Le-Chateau-14.5s-Teaser-Edit.mp4"}]}',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits',
    '{"phase":"execution","le-chateau":true,"backfill":true}',
    'execution',
    '2026-02-26T14:17:00'
);

-- 3d. Render deliverables
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, file_path, metadata, phase, created_at
) VALUES (
    X'1ec0a1ea000040008000000000000004',
    NULL,
    'render_deliverable',
    'Deliverables: Le Chateau (3 MP4s)',
    '{"batch_id":"le-chateau-batch-001","deliverables":[{"file":"Le-Chateau-59s-FOMO-Edit.mp4","path":"/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits/Le-Chateau-59s-FOMO-Edit.mp4","size_bytes":64695121,"duration_seconds":59.0,"resolution":"1080x1920","codec":"h264"},{"file":"Le-Chateau-29s-Condensed-Edit.mp4","path":"/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits/Le-Chateau-29s-Condensed-Edit.mp4","size_bytes":31393162,"duration_seconds":29.0,"resolution":"1080x1920","codec":"h264"},{"file":"Le-Chateau-14.5s-Teaser-Edit.mp4","path":"/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits/Le-Chateau-14.5s-Teaser-Edit.mp4","size_bytes":29816124,"duration_seconds":14.5,"resolution":"1080x1920","codec":"h264"}]}',
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits',
    '{"phase":"execution","le-chateau":true,"backfill":true}',
    'execution',
    '2026-02-26T14:19:00'
);

-- ============================================================================
-- 4. TASK (1 row, BLOB PK)
-- ============================================================================

INSERT OR IGNORE INTO tasks (
    id, project_id, title, description, status,
    assigned_agent, tags, custom_properties,
    created_by, created_at, updated_at
) VALUES (
    X'1ec0a1ea000040008000000000000010',
    X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf',
    'Le Chateau — Event Recap Video Production',
    'Ingest 10 source clips from Le Chateau venue, run scene analysis, assemble 3 edits (59s FOMO, 29s Condensed, 14.5s Teaser), and render final MP4 deliverables.',
    'done',
    'editron',
    '["editron","media"]',
    '{"editron_batch_id":"le-chateau-batch-001","client":"le-chateau","backfill":true}',
    'editron-backfill',
    '2026-02-26T14:12:00',
    '2026-02-26T14:19:00'
);

-- ============================================================================
-- 5. TASK–ARTIFACT LINKS (4 rows)
-- ============================================================================

-- Render deliverable: primary, pinned
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'1ec0a1ea000040008000000000000010',
    X'1ec0a1ea000040008000000000000004',
    'primary', 1, 1, '2026-02-26T14:19:00', 'editron-backfill'
);

-- Edit session: primary
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'1ec0a1ea000040008000000000000010',
    X'1ec0a1ea000040008000000000000003',
    'primary', 2, 0, '2026-02-26T14:17:00', 'editron-backfill'
);

-- Ingest manifest: supporting
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'1ec0a1ea000040008000000000000010',
    X'1ec0a1ea000040008000000000000001',
    'supporting', 3, 0, '2026-02-26T14:12:00', 'editron-backfill'
);

-- Analysis report: supporting
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'1ec0a1ea000040008000000000000010',
    X'1ec0a1ea000040008000000000000002',
    'supporting', 4, 0, '2026-02-26T14:15:00', 'editron-backfill'
);

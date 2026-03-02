-- Backfill Le Chateau activity logs and VIBE transaction charges
-- These were never recorded because the workflow ran outside the NORA pipeline.
--
-- VIBE cost schedule (from EditronVibeCosts in editron_tracking.rs):
--   Ingest:  500 base + 10/file  = 500 + 10*10 = 600
--   Analyze: 2000 base + 500/pass = 2000 + 500*1 = 2500
--   Edit:    3000 base + 200/ratio = 3000 + 200*1 = 3200
--   Render:  5000 base + 1000/format = 5000 + 1000*3 = 8000
--   Total:   14,300 VIBE
--
-- All inserts use INSERT OR IGNORE with deterministic IDs — safe to re-run.

-- Task ID reference: X'1ec0a1ea000040008000000000000010'
-- Task ID as UUID text: '1ec0a1ea-0000-4000-8000-000000000010'
-- Project ID: X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf'

-- ============================================================================
-- 1. ACTIVITY LOGS (4 rows — TEXT id, TEXT task_id)
-- ============================================================================

-- 1a. Ingest activity
INSERT OR IGNORE INTO activity_logs (
    id, task_id, actor_id, actor_type, action,
    previous_state, new_state, metadata, timestamp
) VALUES (
    '1ec0a1ea-0000-4000-8000-000000000101',
    '1ec0a1ea-0000-4000-8000-000000000010',
    'editron', 'agent', 'media_ingest',
    NULL, NULL,
    '{"vibe_cost":600,"message":"Ingested 10 source clips from Le Chateau (91.6 MB)","batch_id":"le-chateau-batch-001","file_count":10,"total_bytes":95995141,"client":"le-chateau"}',
    '2026-02-26T14:12:00'
);

-- 1b. Scene analysis activity
INSERT OR IGNORE INTO activity_logs (
    id, task_id, actor_id, actor_type, action,
    previous_state, new_state, metadata, timestamp
) VALUES (
    '1ec0a1ea-0000-4000-8000-000000000102',
    '1ec0a1ea-0000-4000-8000-000000000010',
    'editron', 'agent', 'media_analysis',
    NULL, NULL,
    '{"vibe_cost":2500,"message":"Scene analysis complete: 10/10 clips usable, venue showcase content","batch_id":"le-chateau-batch-001","passes":1,"usable_clips":10,"dominant_content":"venue_showcase"}',
    '2026-02-26T14:15:00'
);

-- 1c. Edit assembly activity
INSERT OR IGNORE INTO activity_logs (
    id, task_id, actor_id, actor_type, action,
    previous_state, new_state, metadata, timestamp
) VALUES (
    '1ec0a1ea-0000-4000-8000-000000000103',
    '1ec0a1ea-0000-4000-8000-000000000010',
    'editron', 'agent', 'video_edit',
    NULL, NULL,
    '{"vibe_cost":3200,"message":"Assembled 3 edits: 59s FOMO (10 seg), 29s Condensed (8 seg), 14.5s Teaser (6 seg)","batch_id":"le-chateau-batch-001","edit_count":3,"aspect_ratios":1,"edits":[{"name":"59s-FOMO","duration":59.0,"segments":10},{"name":"29s-Condensed","duration":29.0,"segments":8},{"name":"14.5s-Teaser","duration":14.5,"segments":6}]}',
    '2026-02-26T14:17:00'
);

-- 1d. Render activity
INSERT OR IGNORE INTO activity_logs (
    id, task_id, actor_id, actor_type, action,
    previous_state, new_state, metadata, timestamp
) VALUES (
    '1ec0a1ea-0000-4000-8000-000000000104',
    '1ec0a1ea-0000-4000-8000-000000000010',
    'editron', 'agent', 'render_deliverable',
    NULL, NULL,
    '{"vibe_cost":8000,"message":"Rendered 3 MP4 deliverables (120 MB total)","batch_id":"le-chateau-batch-001","format_count":3,"deliverables":["Le-Chateau-59s-FOMO-Edit.mp4","Le-Chateau-29s-Condensed-Edit.mp4","Le-Chateau-14.5s-Teaser-Edit.mp4"],"total_size_bytes":125904407}',
    '2026-02-26T14:19:00'
);

-- ============================================================================
-- 2. VIBE TRANSACTIONS (4 rows — BLOB id, BLOB source_id, BLOB task_id)
--    source_type = 'project', provider = 'editron'
-- ============================================================================

-- 2a. Ingest charge: 600 VIBE
INSERT OR IGNORE INTO vibe_transactions (
    id, source_type, source_id, amount_vibe,
    input_tokens, output_tokens, model, provider,
    calculated_cost_cents, task_id, task_attempt_id, process_id,
    description, metadata, aptos_tx_status, created_at, updated_at, on_chain_synced
) VALUES (
    X'1ec0a1ea000040008000000000000201',
    'project',
    X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf',
    600,
    NULL, NULL,
    'editron-ingest', 'editron',
    600,
    X'1ec0a1ea000040008000000000000010',
    NULL, NULL,
    'Le Chateau — Media ingest (10 files, 91.6 MB)',
    '{"batch_id":"le-chateau-batch-001","operation":"ingest","file_count":10,"base_cost":500,"per_file_cost":10,"backfill":true}',
    'pending',
    '2026-02-26T14:12:00', '2026-02-26T14:12:00', 0
);

-- 2b. Analysis charge: 2500 VIBE
INSERT OR IGNORE INTO vibe_transactions (
    id, source_type, source_id, amount_vibe,
    input_tokens, output_tokens, model, provider,
    calculated_cost_cents, task_id, task_attempt_id, process_id,
    description, metadata, aptos_tx_status, created_at, updated_at, on_chain_synced
) VALUES (
    X'1ec0a1ea000040008000000000000202',
    'project',
    X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf',
    2500,
    NULL, NULL,
    'editron-analyze', 'editron',
    2500,
    X'1ec0a1ea000040008000000000000010',
    NULL, NULL,
    'Le Chateau — Scene analysis (1 pass, 10 clips)',
    '{"batch_id":"le-chateau-batch-001","operation":"analyze","passes":1,"base_cost":2000,"per_pass_cost":500,"backfill":true}',
    'pending',
    '2026-02-26T14:15:00', '2026-02-26T14:15:00', 0
);

-- 2c. Edit charge: 3200 VIBE
INSERT OR IGNORE INTO vibe_transactions (
    id, source_type, source_id, amount_vibe,
    input_tokens, output_tokens, model, provider,
    calculated_cost_cents, task_id, task_attempt_id, process_id,
    description, metadata, aptos_tx_status, created_at, updated_at, on_chain_synced
) VALUES (
    X'1ec0a1ea000040008000000000000203',
    'project',
    X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf',
    3200,
    NULL, NULL,
    'editron-edit', 'editron',
    3200,
    X'1ec0a1ea000040008000000000000010',
    NULL, NULL,
    'Le Chateau — Video assembly (3 edits, 1 aspect ratio)',
    '{"batch_id":"le-chateau-batch-001","operation":"generate","edit_count":3,"aspect_ratios":1,"base_cost":3000,"per_ratio_cost":200,"backfill":true}',
    'pending',
    '2026-02-26T14:17:00', '2026-02-26T14:17:00', 0
);

-- 2d. Render charge: 8000 VIBE
INSERT OR IGNORE INTO vibe_transactions (
    id, source_type, source_id, amount_vibe,
    input_tokens, output_tokens, model, provider,
    calculated_cost_cents, task_id, task_attempt_id, process_id,
    description, metadata, aptos_tx_status, created_at, updated_at, on_chain_synced
) VALUES (
    X'1ec0a1ea000040008000000000000204',
    'project',
    X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf',
    8000,
    NULL, NULL,
    'editron-render', 'editron',
    8000,
    X'1ec0a1ea000040008000000000000010',
    NULL, NULL,
    'Le Chateau — Render deliverables (3 MP4s, standard priority)',
    '{"batch_id":"le-chateau-batch-001","operation":"render","format_count":3,"is_rush":false,"base_cost":5000,"per_format_cost":1000,"backfill":true}',
    'pending',
    '2026-02-26T14:19:00', '2026-02-26T14:19:00', 0
);

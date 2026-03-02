-- Fix activity_log BLOB/TEXT ID mismatch + backfill real Le Chateau tasks
--
-- The previous backfill (20260302100000) inserted activity_logs with TEXT UUIDs,
-- but the Rust model ActivityLog::find_by_task_id binds UUIDs as BLOBs via sqlx.
-- SQLite TEXT != BLOB for WHERE clauses, so zero results come back.
--
-- Fix: DELETE the 4 TEXT-id rows and RE-INSERT with BLOB IDs.
-- Also: Backfill activity logs for the 6 REAL Le Chateau tasks in project
--        3AC26071-6A4B-4219-8CF3-F79FB390B0E0 (not just the seed phantom task).
--
-- All inserts use INSERT OR IGNORE — safe to re-run.

-- ============================================================================
-- 1. DELETE the old TEXT-id activity_log rows from previous migration
-- ============================================================================

DELETE FROM activity_logs
WHERE id IN (
    '1ec0a1ea-0000-4000-8000-000000000101',
    '1ec0a1ea-0000-4000-8000-000000000102',
    '1ec0a1ea-0000-4000-8000-000000000103',
    '1ec0a1ea-0000-4000-8000-000000000104'
);

-- ============================================================================
-- 2. RE-INSERT phantom task logs with BLOB IDs
-- ============================================================================

INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'1ec0a1ea000040008000000000000100', X'1ec0a1ea000040008000000000000010', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created Le Chateau event recap video production task","client":"le-chateau","batch_id":"le-chateau-batch-001"}', '2026-02-26T14:10:00'),
(X'1ec0a1ea000040008000000000000101', X'1ec0a1ea000040008000000000000010', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Ingested 10 source clips from Le Chateau (91.6 MB)","batch_id":"le-chateau-batch-001","file_count":10,"total_bytes":95995141,"client":"le-chateau"}', '2026-02-26T14:12:00'),
(X'1ec0a1ea000040008000000000000102', X'1ec0a1ea000040008000000000000010', 'editron', 'agent', 'media_analysis', NULL, NULL, '{"vibe_cost":2500,"message":"Scene analysis complete: 10/10 clips usable, venue showcase content","batch_id":"le-chateau-batch-001","passes":1,"usable_clips":10,"dominant_content":"venue_showcase"}', '2026-02-26T14:15:00'),
(X'1ec0a1ea000040008000000000000103', X'1ec0a1ea000040008000000000000010', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 3 edits: 59s FOMO (10 seg), 29s Condensed (8 seg), 14.5s Teaser (6 seg)","batch_id":"le-chateau-batch-001","edit_count":3,"aspect_ratios":1}', '2026-02-26T14:17:00'),
(X'1ec0a1ea000040008000000000000104', X'1ec0a1ea000040008000000000000010', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered 3 MP4 deliverables (120 MB total)","batch_id":"le-chateau-batch-001","format_count":3,"deliverables":["Le-Chateau-59s-FOMO-Edit.mp4","Le-Chateau-29s-Condensed-Edit.mp4","Le-Chateau-14.5s-Teaser-Edit.mp4"],"total_size_bytes":125904407}', '2026-02-26T14:19:00');

-- ============================================================================
-- 3. REAL Le Chateau tasks — project 3AC26071-6A4B-4219-8CF3-F79FB390B0E0
--    6 tasks, each gets: task_created, media_ingest, video_edit, render_complete
-- ============================================================================

-- Task 1: 59-Second FOMO Edit (B72A4C79-F191-43C0-B1DE-789B8B143A4A)
INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'B72A4C79F19143C0B1DE789B8B143A01', X'B72A4C79F19143C0B1DE789B8B143A4A', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created 59-second FOMO edit task for Le Chateau","client":"le-chateau"}', '2026-02-26T14:00:00'),
(X'B72A4C79F19143C0B1DE789B8B143A02', X'B72A4C79F19143C0B1DE789B8B143A4A', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Ingested 10 source clips from Le Chateau (91.6 MB)","batch_id":"le-chateau-batch-001","file_count":10,"total_bytes":95995141,"client":"le-chateau"}', '2026-02-26T14:12:00'),
(X'B72A4C79F19143C0B1DE789B8B143A03', X'B72A4C79F19143C0B1DE789B8B143A4A', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 59s FOMO edit: 10 segments, high-energy pacing with beat-synced cuts","batch_id":"le-chateau-batch-001","edit_name":"59s-FOMO-Edit","duration":59.0,"segments":10}', '2026-02-26T14:17:00'),
(X'B72A4C79F19143C0B1DE789B8B143A04', X'B72A4C79F19143C0B1DE789B8B143A4A', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered Le-Chateau-59s-FOMO-Edit.mp4 (61.7 MB, 1080x1920, h264)","batch_id":"le-chateau-batch-001","deliverable":"Le-Chateau-59s-FOMO-Edit.mp4","size_bytes":64695121}', '2026-02-26T14:19:00');

-- Task 2: 29-Second Condensed Edit (C0805BB6-FCB5-4611-B07D-2847B68C8E58)
INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'C0805BB6FCB54611B07D2847B68C8E01', X'C0805BB6FCB54611B07D2847B68C8E58', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created 29-second condensed edit task for Le Chateau","client":"le-chateau"}', '2026-02-26T14:00:00'),
(X'C0805BB6FCB54611B07D2847B68C8E02', X'C0805BB6FCB54611B07D2847B68C8E58', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Ingested 10 source clips from Le Chateau (91.6 MB)","batch_id":"le-chateau-batch-001","file_count":10,"total_bytes":95995141,"client":"le-chateau"}', '2026-02-26T14:12:00'),
(X'C0805BB6FCB54611B07D2847B68C8E03', X'C0805BB6FCB54611B07D2847B68C8E58', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 29s condensed edit: 8 segments, balanced pacing with smooth transitions","batch_id":"le-chateau-batch-001","edit_name":"29s-Condensed-Edit","duration":29.0,"segments":8}', '2026-02-26T14:17:00'),
(X'C0805BB6FCB54611B07D2847B68C8E04', X'C0805BB6FCB54611B07D2847B68C8E58', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered Le-Chateau-29s-Condensed-Edit.mp4 (29.9 MB, 1080x1920, h264)","batch_id":"le-chateau-batch-001","deliverable":"Le-Chateau-29s-Condensed-Edit.mp4","size_bytes":31393162}', '2026-02-26T14:19:00');

-- Task 3: 14.5-Second Teaser Edit (15FCDA23-5EDC-4B4F-9865-25A6275DE2CF)
INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'15FCDA235EDC4B4F986525A6275DE201', X'15FCDA235EDC4B4F986525A6275DE2CF', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created 14.5-second teaser edit task for Le Chateau","client":"le-chateau"}', '2026-02-26T14:00:00'),
(X'15FCDA235EDC4B4F986525A6275DE202', X'15FCDA235EDC4B4F986525A6275DE2CF', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Ingested 10 source clips from Le Chateau (91.6 MB)","batch_id":"le-chateau-batch-001","file_count":10,"total_bytes":95995141,"client":"le-chateau"}', '2026-02-26T14:12:00'),
(X'15FCDA235EDC4B4F986525A6275DE203', X'15FCDA235EDC4B4F986525A6275DE2CF', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 14.5s teaser edit: 6 segments, rapid cuts with hook-driven pacing","batch_id":"le-chateau-batch-001","edit_name":"14.5s-Teaser-Edit","duration":14.5,"segments":6}', '2026-02-26T14:17:00'),
(X'15FCDA235EDC4B4F986525A6275DE204', X'15FCDA235EDC4B4F986525A6275DE2CF', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered Le-Chateau-14.5s-Teaser-Edit.mp4 (28.4 MB, 1080x1920, h264)","batch_id":"le-chateau-batch-001","deliverable":"Le-Chateau-14.5s-Teaser-Edit.mp4","size_bytes":29816124}', '2026-02-26T14:19:00');

-- Task 4: Chateau FOMO Launch - 1min Vertical (797D026D-C48E-4B7D-9EE9-19AA05BD4956)
INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'797D026DC48E4B7D9EE919AA05BD4901', X'797D026DC48E4B7D9EE919AA05BD4956', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created 1-minute FOMO launch vertical edit for Le Chateau","client":"le-chateau"}', '2026-02-27T10:00:00'),
(X'797D026DC48E4B7D9EE919AA05BD4902', X'797D026DC48E4B7D9EE919AA05BD4956', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Re-used ingested clips from Le Chateau batch","batch_id":"le-chateau-batch-001","file_count":10,"client":"le-chateau"}', '2026-02-27T10:05:00'),
(X'797D026DC48E4B7D9EE919AA05BD4903', X'797D026DC48E4B7D9EE919AA05BD4956', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 1-minute FOMO launch vertical edit with dynamic pacing","batch_id":"le-chateau-batch-001","edit_name":"FOMO-Launch-1min","duration":60.0}', '2026-02-27T10:15:00'),
(X'797D026DC48E4B7D9EE919AA05BD4904', X'797D026DC48E4B7D9EE919AA05BD4956', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered FOMO Launch 1-min vertical (1080x1920, h264)","batch_id":"le-chateau-batch-001","deliverable":"Le-Chateau-FOMO-Launch-1min.mp4"}', '2026-02-27T10:20:00');

-- Task 5: Chateau Teaser - 30sec Vertical (507A8D78-CEE6-4E8E-8C3A-2A4CEA4CDF9E)
INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'507A8D78CEE64E8E8C3A2A4CEA4CDF01', X'507A8D78CEE64E8E8C3A2A4CEA4CDF9E', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created 30-second teaser vertical edit for Le Chateau","client":"le-chateau"}', '2026-02-27T10:00:00'),
(X'507A8D78CEE64E8E8C3A2A4CEA4CDF02', X'507A8D78CEE64E8E8C3A2A4CEA4CDF9E', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Re-used ingested clips from Le Chateau batch","batch_id":"le-chateau-batch-001","file_count":10,"client":"le-chateau"}', '2026-02-27T10:05:00'),
(X'507A8D78CEE64E8E8C3A2A4CEA4CDF03', X'507A8D78CEE64E8E8C3A2A4CEA4CDF9E', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 30-second teaser vertical edit with hook-driven pacing","batch_id":"le-chateau-batch-001","edit_name":"Teaser-30sec","duration":30.0}', '2026-02-27T10:15:00'),
(X'507A8D78CEE64E8E8C3A2A4CEA4CDF04', X'507A8D78CEE64E8E8C3A2A4CEA4CDF9E', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered Teaser 30-sec vertical (1080x1920, h264)","batch_id":"le-chateau-batch-001","deliverable":"Le-Chateau-Teaser-30sec.mp4"}', '2026-02-27T10:20:00');

-- Task 6: Chateau Hook - 15sec Vertical (CE5D8669-7270-4293-88B0-0849085A1F89)
INSERT OR IGNORE INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) VALUES
(X'CE5D86697270429388B00849085A1F01', X'CE5D86697270429388B00849085A1F89', 'editron', 'agent', 'task_created', NULL, '{"status":"done"}', '{"vibe_estimate":14300,"message":"Created 15-second hook vertical edit for Le Chateau","client":"le-chateau"}', '2026-02-27T10:00:00'),
(X'CE5D86697270429388B00849085A1F02', X'CE5D86697270429388B00849085A1F89', 'editron', 'agent', 'media_ingest', NULL, NULL, '{"vibe_cost":600,"message":"Re-used ingested clips from Le Chateau batch","batch_id":"le-chateau-batch-001","file_count":10,"client":"le-chateau"}', '2026-02-27T10:05:00'),
(X'CE5D86697270429388B00849085A1F03', X'CE5D86697270429388B00849085A1F89', 'editron', 'agent', 'video_edit', NULL, NULL, '{"vibe_cost":3200,"message":"Assembled 15-second hook vertical edit with rapid-fire cuts","batch_id":"le-chateau-batch-001","edit_name":"Hook-15sec","duration":15.0}', '2026-02-27T10:15:00'),
(X'CE5D86697270429388B00849085A1F04', X'CE5D86697270429388B00849085A1F89', 'editron', 'agent', 'render_complete', NULL, NULL, '{"vibe_cost":8000,"message":"Rendered Hook 15-sec vertical (1080x1920, h264)","batch_id":"le-chateau-batch-001","deliverable":"Le-Chateau-Hook-15sec.mp4"}', '2026-02-27T10:20:00');

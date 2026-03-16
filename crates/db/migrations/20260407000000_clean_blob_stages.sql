-- Clean up BLOB-typed rows in crm_pipeline_stages.
--
-- These rows survived the 20260406 migration because they used a non-standard
-- storage format: TEXT UUID strings stored as BLOB affinity (36-byte ASCII blobs,
-- not 16-byte binary blobs). The 20260406 binary-to-UUID conversion formula only
-- handles 16-byte blobs, so these were carried through with their BLOB type intact.
--
-- All 1933 such rows are either:
--   (a) orphaned — their BLOB-typed pipeline_id has no TEXT match in crm_pipelines
--   (b) duplicates — the TEXT-equivalent pipeline already has matching TEXT stages
-- Either way they are invisible to all Rust queries (TEXT ≠ BLOB in SQLite equality)
-- and safe to remove.

DELETE FROM crm_pipeline_stages WHERE typeof(id) = 'blob';

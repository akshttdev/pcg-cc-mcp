-- Normalize all BLOB UUIDs to TEXT format across all tables.
-- DbUuid encodes as TEXT, but legacy seed data stored some IDs as 16-byte BLOBs.
-- BLOB != TEXT in SQLite equality checks, so queries silently miss BLOB rows.

UPDATE agent_flows SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE agents SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE invoices SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE task_templates SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE browser_allowlist SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE checkpoint_definitions SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE model_pricing SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE repos SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE user_platform_roles SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE pcg_router_models SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE oss_libraries SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE oss_library_updates SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;
UPDATE workflow_output_staging SET id = lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' || hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) WHERE typeof(id) = 'blob' AND length(id) = 16;

-- Auri coding task fields on tasks table
ALTER TABLE tasks ADD COLUMN github_repo TEXT;        -- e.g. "powerclubglobal/pcg-cc-mcp"
ALTER TABLE tasks ADD COLUMN github_branch TEXT;      -- branch Auri created / pushed to
ALTER TABLE tasks ADD COLUMN auri_pr_url TEXT;        -- PR URL after Auri finishes
ALTER TABLE tasks ADD COLUMN auri_log TEXT;           -- execution log (stdout from claude --print)
ALTER TABLE tasks ADD COLUMN session_id TEXT;         -- meeting/call session that spawned this task

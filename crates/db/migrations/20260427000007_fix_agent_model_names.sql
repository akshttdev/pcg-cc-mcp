-- Fix stale model IDs in agents table
-- claude-sonnet-4-20250514 → claude-sonnet-4-6 (canonical model ID)
UPDATE agents
SET default_model = 'claude-sonnet-4-6'
WHERE default_model IN (
    'claude-sonnet-4-20250514',
    'claude-sonnet-4',
    'claude-sonnet-4-6-20250514'
);

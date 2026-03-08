-- Phase 2b: Enhance Tags and add Scratch (UI state persistence)

-- Add content column to existing tags table for template-like functionality
ALTER TABLE tags ADD COLUMN content TEXT NOT NULL DEFAULT '';

-- Scratch: typed key-value store for UI state persistence
CREATE TABLE IF NOT EXISTS scratch (
    id          BLOB PRIMARY KEY NOT NULL,
    scratch_type TEXT NOT NULL,
    payload     TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_scratch_type ON scratch(scratch_type);

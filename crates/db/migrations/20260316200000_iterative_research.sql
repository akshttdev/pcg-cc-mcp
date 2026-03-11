-- Iterative research: each pass builds on the last, accumulating into KG
-- Migration: 20260316200000

-- Track individual research passes per person
CREATE TABLE IF NOT EXISTS person_research_passes (
    id                  BLOB NOT NULL PRIMARY KEY,
    person_id           BLOB NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    pass_number         INTEGER NOT NULL DEFAULT 1,
    -- 1=who_is, 2=market_position, 3=competitors, 4=target_clients, 5=deep_strategy

    research_focus      TEXT NOT NULL,
    -- 'identity' | 'market_position' | 'competitors' | 'target_clients' | 'deep_strategy' | 'custom'

    focus_prompt        TEXT,               -- custom focus override
    status              TEXT NOT NULL DEFAULT 'pending',
    -- 'pending' | 'running' | 'done' | 'failed'

    -- Results
    summary             TEXT,               -- markdown summary of this pass
    raw_results         TEXT,               -- full JSON from agent/search
    key_findings        TEXT DEFAULT '[]',  -- JSON [{finding, confidence, source}]
    search_queries      TEXT DEFAULT '[]',  -- JSON [string] — queries actually used

    -- Confidence increases with each pass
    confidence_delta    REAL DEFAULT 0.0,   -- how much this pass improved confidence

    agent_used          TEXT,               -- 'scout' | 'astra' | 'claude'
    tokens_used         INTEGER,
    error               TEXT,

    created_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    completed_at        TEXT
);

-- Index for loading all passes for a person in order
CREATE INDEX IF NOT EXISTS idx_research_passes_person ON person_research_passes(person_id, pass_number);

-- Add pass_count to persons for quick lookup
ALTER TABLE persons ADD COLUMN research_pass_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE persons ADD COLUMN research_depth TEXT NOT NULL DEFAULT 'shallow';
-- 'shallow' (0-1 passes) | 'moderate' (2-3) | 'deep' (4+)

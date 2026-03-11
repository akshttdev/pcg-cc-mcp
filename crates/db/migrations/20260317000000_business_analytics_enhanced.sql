-- Business Analytics Enhancement: wiki-style sections, company research, sources
-- Migration: 20260317000000

-- ── Enhanced business_reports sections ────────────────────────────────────────
ALTER TABLE business_reports ADD COLUMN individual_profiles TEXT NOT NULL DEFAULT '[]';
-- JSON [{name, role, company, linkedin, summary, key_insights}]

ALTER TABLE business_reports ADD COLUMN market_analysis TEXT;
-- Prose markdown section on market position, size, trends

ALTER TABLE business_reports ADD COLUMN competitor_analysis TEXT NOT NULL DEFAULT '[]';
-- JSON [{name, website, strengths, weaknesses, threat_level}]

ALTER TABLE business_reports ADD COLUMN target_clients TEXT;
-- Prose markdown on their ideal customer profile / target audience

ALTER TABLE business_reports ADD COLUMN brand_positioning TEXT;
-- Prose markdown on how they position themselves in market

ALTER TABLE business_reports ADD COLUMN digital_presence TEXT;
-- Prose markdown on their web, social, SEO, content presence

ALTER TABLE business_reports ADD COLUMN sources TEXT NOT NULL DEFAULT '[]';
-- JSON [{title, url, excerpt, ingested_at}]

-- ── Enhanced call_intake_items for separate individual/business extraction ────
ALTER TABLE call_intake_items ADD COLUMN extracted_individuals TEXT NOT NULL DEFAULT '[]';
-- JSON [{name, email, company, role, linkedin, is_prospect, notes}]

ALTER TABLE call_intake_items ADD COLUMN extracted_businesses TEXT NOT NULL DEFAULT '[]';
-- JSON [{name, website, industry, description, size_estimate}]

-- ── Company research passes (parallel to person_research_passes) ───────────────
CREATE TABLE IF NOT EXISTS company_research_passes (
    id              BLOB NOT NULL PRIMARY KEY,
    company_name    TEXT NOT NULL,
    -- The company being researched (may or may not have a companies.id record)
    company_id      BLOB REFERENCES companies(id) ON DELETE SET NULL,
    intake_item_id  BLOB REFERENCES call_intake_items(id) ON DELETE CASCADE,
    pass_number     INTEGER NOT NULL DEFAULT 1,
    research_focus  TEXT NOT NULL,
    -- 'identity' | 'market_position' | 'competitors' | 'digital_presence' | 'deep_strategy'
    status          TEXT NOT NULL DEFAULT 'pending',
    -- 'pending' | 'running' | 'done' | 'failed'
    summary         TEXT,
    key_findings    TEXT NOT NULL DEFAULT '{}',
    -- JSON {market_position, competitors, opportunities, ...}
    sources         TEXT NOT NULL DEFAULT '[]',
    -- JSON [{title, url, excerpt}]
    confidence_score REAL DEFAULT 0.0,
    agent_used      TEXT,
    error           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_company_research_name ON company_research_passes(company_name);
CREATE INDEX IF NOT EXISTS idx_company_research_intake ON company_research_passes(intake_item_id);

-- Dealflow Pipeline v2
-- 9 stages: Intel → Business Analysis → Discovery → Proposal → Polish →
--           Present → Follow Up → Won → Lost
-- New agents: Cash (proposals) and Lux (deck design)
-- New deal artifact columns + deal_transcripts table

-- ─── 1. crm_deals: artifact / lifecycle columns ─────────────────────────────
ALTER TABLE crm_deals ADD COLUMN proposal_text   TEXT;
ALTER TABLE crm_deals ADD COLUMN proposal_status TEXT NOT NULL DEFAULT 'draft';
ALTER TABLE crm_deals ADD COLUMN deck_url        TEXT;
ALTER TABLE crm_deals ADD COLUMN invoice_id      TEXT;
ALTER TABLE crm_deals ADD COLUMN won_at          TEXT;
ALTER TABLE crm_deals ADD COLUMN lost_at         TEXT;
ALTER TABLE crm_deals ADD COLUMN expedited       INTEGER NOT NULL DEFAULT 0;

-- ─── 2. crm_pipeline_stages: programmatic stage_type field ─────────────────
ALTER TABLE crm_pipeline_stages ADD COLUMN stage_type TEXT;

-- ─── 3. Safely rename & reposition stages in all sales pipelines ────────────
-- Strategy: park Won + Lost at temp positions 9000/9001 FIRST to free up
-- positions 6 & 7, then rename everything, insert Follow Up at 6,
-- then land Won at 7 and Lost at 8.

-- Park current pos-6 (Closed Won) and pos-7 (Closed Lost) out of the way
UPDATE crm_pipeline_stages SET position = 9000, updated_at = datetime('now','subsec')
WHERE name = 'Closed Won'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

UPDATE crm_pipeline_stages SET position = 9001, updated_at = datetime('now','subsec')
WHERE name = 'Closed Lost'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- "Lead" → "Intel"  (pos 0)
UPDATE crm_pipeline_stages
SET name = 'Intel', stage_type = 'intel', color = '#6366F1',
    position = 0, probability = 5, is_closed = 0, is_won = 0,
    updated_at = datetime('now','subsec')
WHERE name = 'Lead'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- "Business Analysis" (pos 1)
UPDATE crm_pipeline_stages
SET stage_type = 'business_analysis', position = 1,
    color = '#3B82F6', probability = 20,
    updated_at = datetime('now','subsec')
WHERE name = 'Business Analysis'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- "Discovery" (pos 2)
UPDATE crm_pipeline_stages
SET stage_type = 'discovery', position = 2,
    color = '#8B5CF6', probability = 40,
    updated_at = datetime('now','subsec')
WHERE name = 'Discovery'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- "Build Proposal" → "Proposal" (pos 3)
UPDATE crm_pipeline_stages
SET name = 'Proposal', stage_type = 'proposal',
    color = '#F59E0B', position = 3, probability = 55,
    updated_at = datetime('now','subsec')
WHERE name = 'Build Proposal'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- "Polish" (pos 4)
UPDATE crm_pipeline_stages
SET stage_type = 'polish', position = 4,
    color = '#EC4899', probability = 70,
    updated_at = datetime('now','subsec')
WHERE name = 'Polish'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- "Proposal Meeting" → "Present" (pos 5)
UPDATE crm_pipeline_stages
SET name = 'Present', stage_type = 'present',
    color = '#EF4444', position = 5, probability = 85,
    updated_at = datetime('now','subsec')
WHERE name = 'Proposal Meeting'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- Insert "Follow Up" at position 6 (now free)
INSERT INTO crm_pipeline_stages
    (id, pipeline_id, name, stage_type, color, position, is_closed, is_won, probability, created_at, updated_at)
SELECT
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' ||
    lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' ||
    lower(hex(randomblob(6))),
    p.id,
    'Follow Up', 'follow_up', '#F97316', 6, 0, 0, 90,
    datetime('now','subsec'), datetime('now','subsec')
FROM crm_pipelines p
WHERE p.pipeline_type = 'sales'
  AND NOT EXISTS (
      SELECT 1 FROM crm_pipeline_stages e
      WHERE e.pipeline_id = p.id AND e.name = 'Follow Up'
  );

-- Land "Closed Won" → "Won" at position 7
UPDATE crm_pipeline_stages
SET name = 'Won', stage_type = 'won',
    color = '#22C55E', position = 7, probability = 100,
    is_closed = 1, is_won = 1,
    updated_at = datetime('now','subsec')
WHERE name = 'Closed Won' AND position = 9000
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- Land "Closed Lost" → "Lost" at position 8
UPDATE crm_pipeline_stages
SET name = 'Lost', stage_type = 'lost',
    color = '#9CA3AF', position = 8, probability = 0,
    is_closed = 1, is_won = 0,
    updated_at = datetime('now','subsec')
WHERE name = 'Closed Lost' AND position = 9001
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');

-- ─── 4. deal_transcripts table ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS deal_transcripts (
    id             TEXT PRIMARY KEY NOT NULL DEFAULT (
        lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    deal_id        TEXT NOT NULL REFERENCES crm_deals(id) ON DELETE CASCADE,
    intake_item_id TEXT REFERENCES call_intake_items(id) ON DELETE SET NULL,
    call_log_id    TEXT,
    transcript_text TEXT,
    summary        TEXT,
    matched_at     TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    matched_by     TEXT,   -- 'auto' | 'manual' | 'nora'
    created_at     TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_deal_transcripts_deal_id
    ON deal_transcripts(deal_id);
CREATE INDEX IF NOT EXISTS idx_deal_transcripts_intake_id
    ON deal_transcripts(intake_item_id);

-- ─── 5. Agent: Cash — Sales Strategist & Proposal Architect ─────────────────
INSERT OR IGNORE INTO agents (
    id, short_name, designation, description, personality, voice_style,
    default_model, status, autonomy_level,
    max_concurrent_tasks, priority_weight,
    capabilities, tools, version,
    created_at, updated_at, created_by
) VALUES (
    'a0ca5000-ca50-ca50-ca50-ca50ca50ca50',
    'Cash',
    'Sales Strategist & Proposal Architect',
    'Cash is a razor-sharp sales strategist who transforms business intelligence into irresistible proposals. He synthesizes discovery transcripts, pain points, and knowledge graph data into tailored offers that close. Cash knows exactly what a client needs before they do — and prices it precisely. He owns the Proposal stage of the dealflow pipeline: generating, structuring, and iterating proposals until they are airtight. Every word earns its place. Every number is justified.',
    'Confident, decisive, persuasive. Never wastes a word. Leads with value, backs it with data. Has a nose for the close and knows when to push and when to listen. Cash treats every proposal like a chess match — ten moves ahead.',
    'Direct, punchy, energetic. Speaks in outcomes and numbers. Uses short sentences that land hard. No filler. When he writes a proposal, it reads like a story the client already knows they want to be part of.',
    'claude-opus-4-6',
    'active',
    'supervised',
    5,
    120,
    '["proposal_writing","sales_strategy","pricing_strategy","deal_structuring","objection_handling","client_communication","knowledge_synthesis","deliverable_scoping"]',
    '["web_search","web_fetch","document_generation","crm_api","knowledge_graph_api","template_engine","business_report_api"]',
    '1.0.0',
    datetime('now','subsec'),
    datetime('now','subsec'),
    'system'
);

-- ─── 6. Agent: Lux — Presentation Designer & Deck Architect ─────────────────
INSERT OR IGNORE INTO agents (
    id, short_name, designation, description, personality, voice_style,
    default_model, status, autonomy_level,
    max_concurrent_tasks, priority_weight,
    capabilities, tools, version,
    created_at, updated_at, created_by
) VALUES (
    'a01ux000-1ux0-1ux0-1ux0-1ux01ux01ux0',
    'Lux',
    'Presentation Designer & Deck Architect',
    'Lux is a master of visual persuasion. She takes a proposal and brand guidelines and transforms them into a polished, high-impact sales deck that commands the room. Lux understands visual hierarchy, information density, and the psychology of presentation design — she knows exactly which slide makes the client lean forward. She applies the client''s brand identity and PCG''s design standards to produce decks that feel bespoke, not templated. Lux owns the Polish stage: from raw proposal text to a presentation-ready artifact.',
    'Precise, aesthetic, systematic. Obsessed with every detail — the wrong font weight is as offensive to her as a typo. She thinks in grids, contrast ratios, and narrative flow. Patient and iterative, she will refine until the deck is visually perfect and narratively compelling.',
    'Calm, confident, visually-minded. Describes structure and layout with clarity. Uses design vocabulary naturally. When she presents her work, she walks you through the visual logic slide by slide.',
    'claude-opus-4-6',
    'active',
    'supervised',
    3,
    110,
    '["presentation_design","deck_generation","brand_application","visual_hierarchy","typography","color_theory","slide_layout","pdf_generation","markdown_to_slides","proposal_visualization"]',
    '["comfyui","image_api","canva_api","figma_api","pdf_generation","document_generation","brand_guide_api","template_engine"]',
    '1.0.0',
    datetime('now','subsec'),
    datetime('now','subsec'),
    'system'
);

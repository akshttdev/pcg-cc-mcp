-- Join table: links data sources from the data library to deals.
-- Multiple sources per deal. Used by agents for enriched context.
--
-- Dual scoping:
-- relevant_stages: JSON array of stage names. NULL = all stages.
--   e.g. '["discovery","proposal"]' = visible only in those stages
-- relevant_agents: JSON array of agent names. NULL = all agents.
--   e.g. '["astra","cash"]' = only Astra and Cash read this source
--
-- Both NULL = universal context available to everyone at every stage.

CREATE TABLE IF NOT EXISTS deal_data_sources (
    id TEXT PRIMARY KEY NOT NULL,
    deal_id TEXT NOT NULL REFERENCES crm_deals(id) ON DELETE CASCADE,
    data_source_id TEXT NOT NULL,
    relevant_stages TEXT,
    relevant_agents TEXT,
    linked_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_deal_data_sources_deal ON deal_data_sources(deal_id);
CREATE INDEX IF NOT EXISTS idx_deal_data_sources_source ON deal_data_sources(data_source_id);

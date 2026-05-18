-- Lux Creator Studio — Phase 1 groundwork
-- Link a crm_deal to its structured deck document authored by Lux.
-- The legacy deck_url (Phase 0 markdown output) remains for backwards
-- compatibility until Phase 1 cutover is complete and we can feature-flag it off.

ALTER TABLE crm_deals
    ADD COLUMN deck_document_id TEXT REFERENCES deck_documents(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_crm_deals_deck_document_id
    ON crm_deals(deck_document_id)
    WHERE deck_document_id IS NOT NULL;

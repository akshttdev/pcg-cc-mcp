-- Migrate proposals.lead_id (→ persons.id) to lead_contact_id (→ crm_contacts.id)
--
-- Background: `persons` is the legacy bridge table being retired in favor of
-- `crm_contacts` (see PC-6). `proposals.lead_id` is the last FK still pointing
-- at `persons`. This migration adds the new column, backfills from the existing
-- `persons.crm_contact_id` bridge, and leaves the legacy lead_id column in
-- place for backward compatibility during the transition window.
--
-- Additive only. No destructive drops. Downstream code (entity graph sync,
-- proposal detail UI) should prefer lead_contact_id when present.

ALTER TABLE proposals ADD COLUMN lead_contact_id TEXT REFERENCES crm_contacts(id) ON DELETE SET NULL;

-- Backfill: for every proposal with a lead_id, look up the matching
-- crm_contacts.id via the persons.crm_contact_id bridge.
UPDATE proposals
SET lead_contact_id = (
    SELECT p.crm_contact_id
    FROM persons p
    WHERE p.id = proposals.lead_id
      AND p.crm_contact_id IS NOT NULL
)
WHERE lead_id IS NOT NULL;

CREATE INDEX idx_proposals_lead_contact_id ON proposals(lead_contact_id);

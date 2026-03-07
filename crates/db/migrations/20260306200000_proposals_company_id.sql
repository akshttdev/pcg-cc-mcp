-- ============================================================
-- Migration: Scope proposals to companies (pipeline artifacts)
-- 2026-03-06
-- ============================================================
-- Proposals are acquisition-pipeline artifacts that belong to a
-- company (public entity), not to a project. Adding company_id
-- as the canonical FK for the subject company of any proposal.

ALTER TABLE proposals ADD COLUMN company_id BLOB REFERENCES companies(id);

CREATE INDEX IF NOT EXISTS idx_proposals_company_id ON proposals(company_id);

-- Back-fill: if the proposal's lead person already has a company_id set,
-- copy it over. This handles any previously provisioned companies.
UPDATE proposals
SET company_id = (
    SELECT p.company_id
    FROM persons p
    WHERE p.id = proposals.lead_id
      AND p.company_id IS NOT NULL
)
WHERE lead_id IS NOT NULL
  AND company_id IS NULL;

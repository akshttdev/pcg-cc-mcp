-- Migration: CRM contact follow-up tracking fields
-- Required by command center follow-up section

ALTER TABLE crm_contacts ADD COLUMN waiting_on_client INTEGER NOT NULL DEFAULT 0;
ALTER TABLE crm_contacts ADD COLUMN follow_up_attempts INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_crm_contacts_follow_up
    ON crm_contacts(follow_up_attempts, waiting_on_client)
    WHERE waiting_on_client = 1;

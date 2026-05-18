-- Link social_posts back to CRM deals and contacts for timeline correlation.
ALTER TABLE social_posts ADD COLUMN crm_deal_id TEXT REFERENCES crm_deals(id) ON DELETE SET NULL;
ALTER TABLE social_posts ADD COLUMN crm_contact_id TEXT REFERENCES crm_contacts(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_social_posts_crm_deal ON social_posts(crm_deal_id) WHERE crm_deal_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_social_posts_crm_contact ON social_posts(crm_contact_id) WHERE crm_contact_id IS NOT NULL;

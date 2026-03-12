-- Lead workflow: CRM deal linkage + human review checkpoint on business reports
ALTER TABLE call_intake_items ADD COLUMN IF NOT EXISTS crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL;
ALTER TABLE business_reports ADD COLUMN IF NOT EXISTS crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL;
ALTER TABLE business_reports ADD COLUMN IF NOT EXISTS review_status TEXT NOT NULL DEFAULT 'pending_review'
    CHECK (review_status IN ('pending_review', 'approved', 'rejected'));
ALTER TABLE business_reports ADD COLUMN IF NOT EXISTS reviewed_by BLOB REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE business_reports ADD COLUMN IF NOT EXISTS reviewed_at TEXT;
ALTER TABLE business_reports ADD COLUMN IF NOT EXISTS review_notes TEXT;

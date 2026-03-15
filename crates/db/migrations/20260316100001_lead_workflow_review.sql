-- Lead workflow: CRM deal linkage + human review checkpoint on business reports
ALTER TABLE call_intake_items ADD COLUMN crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL;
ALTER TABLE business_reports ADD COLUMN crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL;
ALTER TABLE business_reports ADD COLUMN review_status TEXT NOT NULL DEFAULT 'pending_review'
    CHECK (review_status IN ('pending_review', 'approved', 'rejected'));
ALTER TABLE business_reports ADD COLUMN reviewed_by BLOB REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE business_reports ADD COLUMN reviewed_at TEXT;
ALTER TABLE business_reports ADD COLUMN review_notes TEXT;

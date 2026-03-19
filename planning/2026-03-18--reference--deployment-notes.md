# Deployment Notes — Sloperation317 Integration (PR #50)

**Date**: 2026-03-18
**Branch**: `integration/sloperation317-quality`

## Dev DB Schema Fixes Required

The BLOB→TEXT migration (20260406) was designed for databases with existing BLOB-format UUIDs. On dev databases that were seeded before this migration, several columns are missing because the migration's table recreation fails on missing columns.

### Manual Fixes Applied to Dev DB

Run these on any dev database before deploying this branch:

```sql
-- Missing columns on persons table
ALTER TABLE persons ADD COLUMN intelligence_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE persons ADD COLUMN intelligence_agent TEXT;

-- Missing columns on crm_deals table
ALTER TABLE crm_deals ADD COLUMN proposal_text TEXT;
ALTER TABLE crm_deals ADD COLUMN proposal_status TEXT DEFAULT 'none';
ALTER TABLE crm_deals ADD COLUMN deck_url TEXT;
ALTER TABLE crm_deals ADD COLUMN deleted_at TEXT;
ALTER TABLE crm_deals ADD COLUMN invoice_id TEXT;
ALTER TABLE crm_deals ADD COLUMN won_at TEXT;
ALTER TABLE crm_deals ADD COLUMN lost_at TEXT;
ALTER TABLE crm_deals ADD COLUMN expedited INTEGER DEFAULT 0;

-- Missing column on crm_pipeline_stages
ALTER TABLE crm_pipeline_stages ADD COLUMN stage_type TEXT;
```

### Pipeline Stage Types

The dealflow automation requires `stage_type` values on pipeline stages. Update existing stages:

```sql
UPDATE crm_pipeline_stages SET stage_type = 'lead' WHERE name = 'Lead';
UPDATE crm_pipeline_stages SET stage_type = 'intel' WHERE name = 'Intel';
UPDATE crm_pipeline_stages SET stage_type = 'business_analysis' WHERE name = 'Business Analysis';
UPDATE crm_pipeline_stages SET stage_type = 'discovery' WHERE name = 'Discovery';
UPDATE crm_pipeline_stages SET stage_type = 'proposal' WHERE name LIKE '%Proposal%';
UPDATE crm_pipeline_stages SET stage_type = 'polish' WHERE name = 'Polish';
UPDATE crm_pipeline_stages SET stage_type = 'present' WHERE name LIKE '%Present%' OR name = 'Proposal Meeting';
UPDATE crm_pipeline_stages SET stage_type = 'won' WHERE name LIKE '%Won%';
UPDATE crm_pipeline_stages SET stage_type = 'lost' WHERE name LIKE '%Lost%';
UPDATE crm_pipeline_stages SET stage_type = 'invoice' WHERE name LIKE '%Invoice%';
UPDATE crm_pipeline_stages SET stage_type = 'negotiation' WHERE name LIKE '%Negotiat%';
```

### Admin Org Membership

Access control requires org membership. If admin user has no memberships:

```sql
INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role)
SELECT 'mem-admin-' || substr(id, 1, 8), id, X'07192211AE5CF20B42BD546422D71A23', 'admin'
FROM organizations;
```

## Production Deployment

For **production databases** that already went through the BLOB→TEXT migration normally:
- No manual fixes needed — the migration handles everything
- Pipeline stage types were set by migration 20260408 (dealflow_pipeline_v2)
- The new migration 20260412 (company_brand_profiles) should apply cleanly

## Environment Variables

New env vars for this branch:
- `DATABASE_URL`: Override SQLite path (default: `sqlite://dev_assets/db.sqlite`). Used for test isolation.
- `ANTHROPIC_API_KEY`: Required for LLM fallback in call_llm (Cash/Lux/Astra)
- `OPENAI_API_KEY`: Preferred for LLM calls (tried first, Anthropic is fallback)

## Migration Numbering

- `20260406000000_crm_blob_to_text.sql` — CRM BLOB→TEXT conversion (672 lines)
- `20260407000000_clean_blob_stages.sql` — Clean orphaned BLOB stages
- `20260412000000_company_brand_profiles.sql` — New table (renumbered from 20260409 to avoid collision with org_cloud)

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

-- Missing columns on crm_contacts table
ALTER TABLE crm_contacts ADD COLUMN person_id TEXT;
ALTER TABLE crm_contacts ADD COLUMN deleted_at TEXT;

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

### Test Seed Data (for E2E tests)

The E2E tests expect specific entities. Create them after schema fixes:

```sql
-- Hudson's Car Club company
INSERT OR IGNORE INTO companies (id, name, industry, organization_id, intelligence_status, created_at, updated_at)
VALUES ('5b3d9e7c-8d05-454d-a73f-b8e659396072', 'Hudson''s Car Club', 'Automotive & Luxury',
  '02020202-0202-0202-0202-020202020202', 'done', datetime('now'), datetime('now'));

-- Joshua Marotta person + CRM contact
INSERT OR IGNORE INTO persons (id, full_name, email, company_name, job_title, organization_id,
  company_id, crm_contact_id, intelligence_status, created_at, updated_at)
VALUES ('p-joshua-marotta-001', 'Joshua Marotta', 'joshua@hudsoncarclub.com', 'Hudson''s Car Club',
  'Founder', '02020202-0202-0202-0202-020202020202', '5b3d9e7c-8d05-454d-a73f-b8e659396072',
  'ct-joshua-marotta-001', 'done', datetime('now'), datetime('now'));

INSERT OR IGNORE INTO crm_contacts (id, organization_id, first_name, last_name, email, person_id, created_at, updated_at)
VALUES ('ct-joshua-marotta-001', '02020202-0202-0202-0202-020202020202', 'Joshua', 'Marotta',
  'joshua@hudsoncarclub.com', 'p-joshua-marotta-001', datetime('now'), datetime('now'));

-- Hudson deal (linked to Acquisition pipeline)
INSERT OR IGNORE INTO crm_deals (id, name, organization_id, crm_pipeline_id, crm_stage_id,
  crm_contact_id, stage, proposal_status, expedited, position, created_at, updated_at)
VALUES ('3b5de595-b2dc-0792-2284-e0349788dfd7', 'Hudson''s Car Club — Joshua Marotta',
  '02020202-0202-0202-0202-020202020202',
  (SELECT id FROM crm_pipelines WHERE name='Acquisition' AND organization_id='02020202-0202-0202-0202-020202020202'),
  (SELECT id FROM crm_pipeline_stages WHERE stage_type='intel' AND pipeline_id=(SELECT id FROM crm_pipelines WHERE name='Acquisition' AND organization_id='02020202-0202-0202-0202-020202020202')),
  'ct-joshua-marotta-001', 'intel', 'none', 0, 0, datetime('now'), datetime('now'));

-- Sirak user org membership (non-admin, for permission tests)
INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role)
VALUES ('mem-sirak-sirak', '02020202-0202-0202-0202-020202020202',
  X'A0A1A2A3A4A5A6A7A8A9AAABACADAEAF', 'member');
```

## E2E Test Results (post-DB fixes)

| Test | Pass | Fail | Skip | Notes |
|------|------|------|------|-------|
| pipeline-userflows (promoted) | 22 | 0 | 0 | Fully passing |
| dealflow-pipeline | 7 | ? | ? | Improved from 1/18 |
| hudson-as-sirak | 3 | ? | ? | Improved from 0/4 |
| hudson-exact-flow | 2 | ? | ? | Improved from 0/4 |
| company-links | 7 | ? | ? | First run |
| trace-company-nav | 6 | ? | ? | Improved from 4/7 |
| full-pipeline-walkthrough | TBD | | | LLM test |
| operator-walkthrough | TBD | | | LLM test |
| visual-pipeline-walkthrough | TBD | | | LLM test / demo |

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

## Final E2E Test Results (all DB fixes applied)

| Test | Pass | Fail | Notes |
|------|------|------|-------|
| pipeline-userflows (promoted) | 22 | 0 | Fully green |
| dealflow-pipeline | 19 | 0 | All passing after seed data + dynamic pipeline |
| hudson-as-sirak | 3 | 0 | All passing with Sirak org membership |
| hudson-exact-flow | 4 | 0 | All passing with seed data |
| company-links | 9 | 0 | All passing with company_name fix |
| trace-company-nav | 6 | 0 | All passing |
| **Total non-LLM** | **63** | **0** | |
| full-pipeline-walkthrough | TBD | | LLM required (Anthropic key available) |
| operator-walkthrough | TBD | | LLM required |
| visual-pipeline-walkthrough | TBD | | LLM required (demo-only) |

### Additional DB Fixes Applied
```sql
-- Contact data completeness
UPDATE crm_contacts SET company_name = 'Hudson''s Car Club', full_name = 'Joshua Marotta'
WHERE id = 'ct-joshua-marotta-001';
```

### Person ID Format Fix
Person IDs must be valid UUIDs for the person profile page to work. The seed data previously used custom string IDs (`p-joshua-marotta-001`) which caused "Person not found" errors.

```sql
-- Fix: use UUID format for person IDs
UPDATE persons SET id = 'a7b8c9d0-1234-5678-9abc-def012345678' WHERE id = 'p-joshua-marotta-001';
UPDATE crm_contacts SET person_id = 'a7b8c9d0-1234-5678-9abc-def012345678' WHERE person_id = 'p-joshua-marotta-001';

-- Set deal amount for invoice functionality
UPDATE crm_deals SET amount = 25000 WHERE id = '3b5de595-b2dc-0792-2284-e0349788dfd7';
```

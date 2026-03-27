# Audit: Persons vs Contacts Table Unification

**Date**: 2026-03-27
**Branch**: `feature/2026-03-24--sloperation-sprint`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Decision**: Merge `persons` INTO `crm_contacts` — contacts is the canonical table

---

## Executive Summary

The codebase has two tables representing the same real-world entity: `crm_contacts` (operational CRM record) and `persons` (identity + intelligence). They share 13 overlapping fields, are linked by a fragile one-way FK, and create a data gap where CRM-created contacts have no intelligence capabilities because they lack a `persons` record.

**Impact**: Scout agent completes research but results are invisible in the Intel tab because intelligence lives on `persons`, but the CRM pipeline operates entirely through `crm_contacts`.

**Decision**: Keep `crm_contacts` as the canonical table. Migrate intelligence fields from `persons` into `crm_contacts`. Retire `persons` over three phases.

---

## Current State

### Schema Comparison

| Field Category | `crm_contacts` | `persons` | Overlap? |
|---|---|---|---|
| **Identity** | first_name, last_name, full_name, email, phone, mobile | full_name, email, phone, emails[], phones[] | YES |
| **Company** | company_name (text) | company_name + company_id (FK) | YES |
| **Job** | job_title, department | job_title | YES |
| **Avatar** | avatar_url | avatar_url | YES |
| **Lifecycle** | lifecycle_stage, lead_score | lifecycle_stage, lead_score | YES |
| **Tags/Custom** | tags, custom_fields | tags, custom_fields | YES |
| **Address** | address_line1/2, city, state, postal_code, country | — | contacts only |
| **Activity** | last_activity_at, last_contacted_at, last_replied_at, email/meeting/deal counts | — | contacts only |
| **Consent** | email_opt_in, sms_opt_in, do_not_contact | — | contacts only |
| **External sync** | zoho_contact_id, gmail_contact_id, external_ids | — | contacts only |
| **Source tracking** | source, source_data_source_id, source_workflow_run_id | — | contacts only |
| **Social** | linkedin_url, twitter_handle | person_social_profiles table | split |
| **Intelligence** | — | intelligence_summary, intelligence_raw, intelligence_confidence, intelligence_last_run_at | persons only |
| **Research** | — | research_pass_count, research_depth + person_research_passes table | persons only |
| **Classification** | — | person_type, financial_role, client_profile, business_stage | persons only |
| **Platform user** | owner_user_id (ownership) | user_id (identity link) | different semantics |
| **Org scope** | organization_id (required) | organization_id (optional) | both |

### Link Mechanism

```
crm_deals.crm_contact_id ──→ crm_contacts.id
                                    ↑
                         persons.crm_contact_id (reverse FK, optional)
```

- `crm_contacts` has **NO** `person_id` column
- `persons.crm_contact_id` points back, but is optional and often NULL
- CRM-created contacts (via UI "Add Deal") never get a person record
- Intake-created persons may get auto-linked contacts (migration 20260404)

### The Data Gap

1. User creates deal with contact "Stephanie Bruno" via CRM UI
2. Contact record created in `crm_contacts` — no person record
3. Deal moves to Intel → Scout runs → tries to find person via `deal → contact → person` chain
4. No person exists → intelligence has nowhere to land
5. Intel tab reads `persons.intelligence_summary` → finds nothing → shows "No intelligence data yet"

---

## API Surface

### Contacts (operational — used by pipeline)
- `GET/POST /api/crm/contacts` — CRUD
- `GET /api/crm/contacts/search` — search
- `POST /api/crm/contacts/:id/activity` — activity tracking
- `POST /api/crm/contacts/:id/lead-score` — scoring
- **Scoped by organization_id** (required)

### Persons (intelligence — mostly backend)
- `GET/POST /api/persons` — CRUD
- `POST /api/persons/:id/research` — trigger intelligence
- `GET /api/persons/:id/intelligence-status` — research status
- `GET/POST /api/persons/:id/social-profiles` — social data
- `GET/POST /api/persons/:id/company-roles` — company associations
- **No org scoping enforced**

### Frontend Usage
- CRM pipeline, deal detail, contacts page → all use `crm_contacts` API
- Intel tab → reads from `persons` via deal JOIN
- `/people/:id` page exists but not navigated from pipeline flow

---

## Unification Plan

### Phase 1 — Add Intelligence to Contacts (unblocks Scout) — 1 day

**Migration**: Add columns to `crm_contacts`:
```sql
ALTER TABLE crm_contacts ADD COLUMN intelligence_summary TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_status TEXT DEFAULT 'idle';
ALTER TABLE crm_contacts ADD COLUMN intelligence_raw TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_confidence REAL DEFAULT 0.0;
ALTER TABLE crm_contacts ADD COLUMN intelligence_last_run_at TEXT;
ALTER TABLE crm_contacts ADD COLUMN research_pass_count INTEGER DEFAULT 0;
ALTER TABLE crm_contacts ADD COLUMN research_depth TEXT DEFAULT 'shallow';
ALTER TABLE crm_contacts ADD COLUMN company_id TEXT REFERENCES companies(id);
ALTER TABLE crm_contacts ADD COLUMN person_id TEXT;
```

**Backfill**: Copy intelligence from linked persons:
```sql
UPDATE crm_contacts SET
  intelligence_summary = (SELECT intelligence_summary FROM persons WHERE crm_contact_id = crm_contacts.id),
  intelligence_status = (SELECT intelligence_status FROM persons WHERE crm_contact_id = crm_contacts.id),
  ...
WHERE id IN (SELECT crm_contact_id FROM persons WHERE crm_contact_id IS NOT NULL);
```

**Code changes**:
- Scout writes to `crm_contacts.intelligence_summary` directly (no person lookup)
- Intel tab reads from contact fields instead of person JOIN
- `CrmDealWithContact` query updated to pull intel from contacts table
- `/api/crm/contacts/:id/research` — new endpoint (delegates to existing intelligence service)

### Phase 2 — Consolidate Overlapping Fields — 2 days

- Remove duplicate reads: stop querying persons for data that now lives on contacts
- Move `person_research_passes` FK to `crm_contact_id`
- Move `person_social_profiles` FK to `crm_contact_id`
- Intake pipeline: create contacts directly (with intelligence) instead of persons
- Add structured `emails[]`, `phones[]` JSON columns to contacts (from persons pattern)

### Phase 3 — Retire Persons Table — 1 day

- Migrate any remaining persons-only data (person_type, financial_role, etc.) to contact columns or custom_fields
- Update `/people/:id` route to read from contacts
- Drop persons-specific API routes or redirect to contacts
- Final migration: drop `persons` table
- Clean up unused models and imports

### Platform Users Consideration

`persons.user_id` links some person records to platform users (team members). Options:
- **Option A**: Add `user_id` to contacts, mark internal users with `person_type = 'team'`
- **Option B**: Keep a thin `user_profiles` table for platform identity (not CRM)
- **Recommendation**: Option A — simpler, one table for all people

---

## Risk Assessment

| Risk | Impact | Mitigation |
|---|---|---|
| Breaking deal FK chain | HIGH | Phase 1 is additive — no FK changes to deals |
| Intake pipeline | MEDIUM | Phase 2 — update intake to create contacts |
| E2E tests | LOW | Tests create contacts via API — already the right table |
| Intelligence service | MEDIUM | Phase 1 — add `/api/crm/contacts/:id/research` |
| `/people/:id` page | LOW | Phase 3 — redirect to contact detail or keep as alias |
| Data loss | LOW | Backfill migration copies all person data before any drops |

---

## Files Affected

### Phase 1 (critical path)
| File | Change |
|---|---|
| `crates/db/migrations/new` | Add intelligence columns to crm_contacts |
| `crates/db/src/models/crm_contact.rs` | Add intelligence fields to struct |
| `crates/db/src/models/crm_deal.rs` | Update `fetch_intel_data()` to read from contacts |
| `crates/server/src/agent_flow_executor.rs` | Scout writes to contacts, not persons |
| `frontend/src/components/crm/deal-detail/tabs/IntelTab.tsx` | No change needed if deal query updated |

### Phase 2-3 (follow-up sprint)
| File | Change |
|---|---|
| `crates/server/src/routes/persons.rs` | Deprecate or redirect |
| `crates/server/src/routes/intelligence.rs` | Target contacts instead of persons |
| `crates/db/src/models/person.rs` | Deprecate |
| Intake pipeline code | Create contacts instead of persons |
| `person_research_passes` migration | Re-FK to contacts |
| `person_social_profiles` migration | Re-FK to contacts |

# Phase 1: Intelligence on Contacts — Unblocks Scout + Intel Tab

**Date**: 2026-03-27
**Branch**: `feature/contacts-unification-phase1`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Base**: `feature/2026-03-24--sloperation-sprint`
**Duration**: 1 day
**Depends on**: Nothing — can start immediately
**Unblocks**: Scout research visible in Intel tab for CRM-created contacts

---

## Goal

Add intelligence fields to `crm_contacts` and rewire Scout to write there directly. After this phase, creating a deal via the CRM UI → moving to Intel → Scout runs → Intel tab shows results. No person record needed.

---

## W1: Migration — Add intelligence columns to crm_contacts

**Migration `20260327000000_contacts_intelligence.sql`**:

```sql
-- Add intelligence fields (mirroring persons table)
ALTER TABLE crm_contacts ADD COLUMN intelligence_summary TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_status TEXT DEFAULT 'idle';
ALTER TABLE crm_contacts ADD COLUMN intelligence_raw TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_confidence REAL DEFAULT 0.0;
ALTER TABLE crm_contacts ADD COLUMN intelligence_last_run_at TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_agent TEXT;
ALTER TABLE crm_contacts ADD COLUMN research_pass_count INTEGER DEFAULT 0;
ALTER TABLE crm_contacts ADD COLUMN research_depth TEXT DEFAULT 'shallow';
ALTER TABLE crm_contacts ADD COLUMN company_id TEXT REFERENCES companies(id);
ALTER TABLE crm_contacts ADD COLUMN person_id TEXT;

-- Backfill from linked persons
UPDATE crm_contacts SET
  intelligence_summary = (SELECT p.intelligence_summary FROM persons p WHERE p.crm_contact_id = crm_contacts.id),
  intelligence_status = COALESCE((SELECT p.intelligence_status FROM persons p WHERE p.crm_contact_id = crm_contacts.id), 'idle'),
  intelligence_raw = (SELECT p.intelligence_raw FROM persons p WHERE p.crm_contact_id = crm_contacts.id),
  intelligence_confidence = COALESCE((SELECT p.intelligence_confidence FROM persons p WHERE p.crm_contact_id = crm_contacts.id), 0.0),
  intelligence_last_run_at = (SELECT p.intelligence_last_run_at FROM persons p WHERE p.crm_contact_id = crm_contacts.id),
  intelligence_agent = (SELECT p.intelligence_agent FROM persons p WHERE p.crm_contact_id = crm_contacts.id),
  research_pass_count = COALESCE((SELECT p.research_pass_count FROM persons p WHERE p.crm_contact_id = crm_contacts.id), 0),
  research_depth = COALESCE((SELECT p.research_depth FROM persons p WHERE p.crm_contact_id = crm_contacts.id), 'shallow'),
  company_id = (SELECT p.company_id FROM persons p WHERE p.crm_contact_id = crm_contacts.id),
  person_id = (SELECT p.id FROM persons p WHERE p.crm_contact_id = crm_contacts.id)
WHERE id IN (SELECT crm_contact_id FROM persons WHERE crm_contact_id IS NOT NULL);
```

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260327000000_contacts_intelligence.sql` | CREATE |

---

## W2: Model — Update CrmContact struct + CrmDealWithContact query

**`crates/db/src/models/crm_contact.rs`**:
- Add intelligence fields to `CrmContact` struct
- Add `intelligence_status`, `intelligence_summary`, `intelligence_confidence`, `intelligence_raw`, `intelligence_last_run_at`, `intelligence_agent`, `research_pass_count`, `research_depth`, `company_id`, `person_id`

**`crates/db/src/models/crm_deal.rs`**:
- Update `CrmDealWithContact` to read intel from `crm_contacts` directly
- Remove the `persons` JOIN in `fetch_intel_data()` — clean break
- `person_id` comes from `crm_contacts.person_id` (backfilled) instead of querying persons table

### Files
| File | Action |
|------|--------|
| `crates/db/src/models/crm_contact.rs` | MODIFY — add intel fields to struct |
| `crates/db/src/models/crm_deal.rs` | MODIFY — remove persons JOIN, read from contacts |

---

## W3: Scout — Write intelligence to contacts

**`crates/server/src/agent_flow_executor.rs`**:
- Update `update_deal_person_intelligence()` → `update_deal_contact_intelligence()`
- New query: `deal → crm_contact_id → UPDATE crm_contacts SET intelligence_summary = ...`
- No person lookup needed — write directly to the contact
- Remove the persons-based method added in previous commit

### Files
| File | Action |
|------|--------|
| `crates/server/src/agent_flow_executor.rs` | MODIFY — write intel to contacts |

---

## W4: DbUuid fixes in touched files

Fix `uuid::Uuid` → `DbUuid` violations in files we're modifying:

| File | Violations |
|------|-----------|
| `crates/db/src/models/person.rs` | `use uuid::Uuid`, `Uuid::new_v4()` in create() |
| `crates/db/src/models/person_research_pass.rs` | `use uuid::Uuid`, struct fields, `Uuid::new_v4()` |
| `crates/server/src/routes/crm_contacts.rs` | `use uuid::Uuid` in query structs |
| `crates/server/src/routes/crm_deals.rs` | `use uuid::Uuid` in query structs |

---

## Verification

1. `cargo check --workspace` — 0 errors
2. `cd frontend && npx tsc --noEmit` — 0 errors
3. Restart server, create a new deal with contact via CRM UI
4. Move deal to Intel → Scout triggers → Run Now
5. Open Intel tab → should show intelligence summary (not "No intelligence data yet")
6. MCP walkthrough to verify

---

## Status: COMPLETE (2026-03-27)

All work items implemented and verified:
- W1: Migration applied, intelligence columns on crm_contacts ✅
- W2: CrmContact struct updated, persons JOIN removed ✅
- W3: Scout writes intelligence to contacts directly ✅
- W4: DbUuid fixes applied ✅
- All static checks pass (fmt, clippy, tsc, eslint, generate-types:check) ✅
- Regression audit: 7 issues found, all fixed ✅
- Functionality audit: 13/13 features WORKING/PARTIAL ✅
- Experience audit: 1 BLOCKER (review task links), 2 PAIN POINTS, 4 FRICTION items

## Experience Audit (2026-03-27)

**Report**: `planning/reviews/2026-03-27--review--experience-audit.md`
**Blockers**: 1 | **Pain Points**: 2 | **Friction**: 4 | **Polish**: 3
**Quick Wins**: 5 identified (all fixed) | **Investments**: 1 identified
**Journey Grades**: A (5), B (3), C (1), F→B (1 — task link fix applied)

## Quick Wins Applied (2026-03-27)

1. IntelTab.tsx TypeScript error — `deal.deal` → `deal.crm_contact_id` ✅
2. cargo fmt — intelligence.rs import grouping ✅
3. BusinessReport model — added crm_contact_id, list_by_contact(), migrated Uuid→&str ✅
4. Task link blank page — guarded empty project_id in my-tasks + global-tasks ✅
5. Raw JSON in intelligence — fixed root cause in agent_flow_executor.rs ✅
6. Duplicate toasts — removed redundant "Deal added to pipeline" toast ✅

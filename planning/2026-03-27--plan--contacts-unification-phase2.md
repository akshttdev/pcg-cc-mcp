# Phase 2: Route Rewiring + PCG Router + Re-FK Child Tables

**Date**: 2026-03-27
**Branch**: `feature/contacts-unification-phase2`
**Base**: Phase 1 branch (or main after Phase 1 merges)
**Duration**: 2 days
**Depends on**: Phase 1 (intelligence columns on contacts)
**Unblocks**: Research from Intel tab, cost-tracked LLM calls, child table cleanup

---

## Goal

Wire the intelligence research system to target contacts (not persons), replace hardcoded LLM calls with PCG Router, and re-FK child tables (research_passes, social_profiles, business_reports) to contacts.

---

## W1: Intelligence routes — target contacts instead of persons

**`crates/server/src/routes/intelligence.rs`**:
- New endpoints:
  - `POST /api/crm/contacts/:id/research` → trigger_contact_research()
  - `GET /api/crm/contacts/:id/intelligence-status` → get_contact_intelligence_status()
- `write_intelligence_results()` → writes to `crm_contacts` instead of `persons`
- Keep person endpoints as thin redirects (look up contact via `persons.crm_contact_id`)

**`crates/server/src/routes/crm_contacts.rs`**:
- Register new intelligence routes on the contacts router

### Files
| File | Action |
|------|--------|
| `crates/server/src/routes/intelligence.rs` | MODIFY — add contact-targeted endpoints, update write target |
| `crates/server/src/routes/crm_contacts.rs` | MODIFY — register intelligence routes |
| `crates/server/src/routes/crm_deals.rs` | MODIFY — register contact research route if needed |

---

## W2: PCG Router integration — replace hardcoded LLM calls

**`crates/server/src/routes/intelligence.rs`**:
- Replace `run_research_direct()` (400+ lines of manual OpenAI/Anthropic HTTP):
  - Use `WorkflowLLMService::completion_with_tools()` from `crates/services/src/services/workflow_llm.rs`
  - Gets automatic priority-based fallback (Anthropic → OpenAI → etc.)
  - Gets automatic cost tracking via `RoutingMetadata`
- Replace `run_company_research_direct()` same way
- Replace `run_research_pass()` same way
- **Keep tool definitions** (web_search) — just pass them through WorkflowLLMService

**Key pattern** (from existing workflow_llm.rs):
```rust
let (response, metadata) = WorkflowLLMService::completion_with_tools(
    pool, messages, tools, Some("claude-sonnet-4-6"), // model hint
).await?;
```

### Files
| File | Action |
|------|--------|
| `crates/server/src/routes/intelligence.rs` | MODIFY — replace HTTP calls with WorkflowLLMService |

---

## W3: Re-FK child tables to contacts

**Migration `20260327000001_child_tables_to_contacts.sql`**:

```sql
-- person_research_passes: re-FK to crm_contacts
ALTER TABLE person_research_passes RENAME TO contact_research_passes;
-- (SQLite: recreate table with new FK)
CREATE TABLE contact_research_passes_new ( ... crm_contact_id TEXT REFERENCES crm_contacts(id) ... );
INSERT INTO contact_research_passes_new SELECT ... ;
DROP TABLE contact_research_passes;
ALTER TABLE contact_research_passes_new RENAME TO contact_research_passes;

-- person_social_profiles: re-FK to crm_contacts
-- Same pattern: recreate with crm_contact_id FK

-- business_reports: add crm_contact_id, backfill from persons bridge
ALTER TABLE business_reports ADD COLUMN crm_contact_id TEXT REFERENCES crm_contacts(id);
UPDATE business_reports SET crm_contact_id = (
  SELECT crm_contact_id FROM persons WHERE persons.id = business_reports.person_id
) WHERE person_id IS NOT NULL;
```

**Model updates**:
- `person_research_pass.rs` → `contact_research_pass.rs` (rename + DbUuid fixes)
- `person.rs` social profile methods → move to `crm_contact.rs` or new `contact_social_profile.rs`
- `business_report.rs` → add `crm_contact_id` field, update queries

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260327000001_child_tables_to_contacts.sql` | CREATE |
| `crates/db/src/models/person_research_pass.rs` | RENAME + MODIFY → contact_research_pass.rs |
| `crates/db/src/models/person.rs` | MODIFY — social profile methods |
| `crates/db/src/models/business_report.rs` | MODIFY — add crm_contact_id |

---

## W4: DbUuid fixes — CRM cluster

Fix `uuid::Uuid` → `DbUuid` in CRM route handlers:

| File | Violations |
|------|-----------|
| `crates/server/src/routes/intelligence.rs` | `use uuid::Uuid`, Uuid::new_v4() throughout |
| `crates/server/src/routes/crm_pipelines.rs` | `use uuid::Uuid` in query structs |
| `crates/server/src/routes/crm_activities.rs` | `use uuid::Uuid` in query structs |
| `crates/server/src/routes/crm_deal_automations.rs` | Check for violations |
| `crates/server/src/routes/crm_deal_transitions.rs` | Check for violations |

---

## Verification

1. `cargo check --workspace` — 0 errors
2. Create deal → move to Intel → click "Trigger Research" in Intel tab
3. Verify research runs through PCG Router (check logs for `[WORKFLOW_LLM]` messages)
4. Verify research results written to crm_contacts (not persons)
5. Verify research_passes accessible via contact
6. Verify business_reports have crm_contact_id populated


---

## Status: IMPLEMENTED (2026-03-27)

Commits: `5a895d33f` (W1), `f5aa308f6` (W2), `1cdaff0ce` (W3). Functionality audit pending.

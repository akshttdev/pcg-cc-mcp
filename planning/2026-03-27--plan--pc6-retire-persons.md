# PC-6: Retire Persons Table

**Date**: 2026-03-27
**Branch**: `feature/contacts-unification-phase1`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Base**: current HEAD on this branch
**Depends on**: Phase 1-3 (all done), Phase 4 W1-W2 (user_profiles + custom_fields migration done)

---

## Goal

Remove all active code references to the `persons` table and its API, replacing with `crm_contacts` equivalents. Then drop the table via migration.

---

## Strategy

**Backend-first**: Fix Rust code to read/write `crm_contacts` instead of `persons`, then remove person routes and model. **Frontend second**: Replace `personsApi` with contacts equivalents, remove person pages.

---

## W1: Backend — Rewrite persons queries to use crm_contacts

### 1a. `intelligence.rs` — Person intelligence routes → contact routes
- Person routes (`/api/persons/:id/research`, etc.) already have contact equivalents from Phase 2
- Remove person-specific routes OR redirect them to contact equivalents
- All SQL `UPDATE persons SET intelligence_*` → already writes to `crm_contacts` (Phase 2)
- Remaining: Remove legacy person route registrations

### 1b. `stage_transition.rs` — Read intelligence_status from crm_contacts
- `SELECT intelligence_status FROM persons WHERE crm_contact_id = ?` → `SELECT intelligence_status FROM crm_contacts WHERE id = ?`
- Company intelligence JOIN: update to not go through persons

### 1c. `organizations/members.rs` — get_org_persons() → get_org_contacts()
- Replace `SELECT * FROM persons` with `SELECT * FROM crm_contacts`
- Return `CrmContactRecord` instead of `Person`
- Update route: `/api/organizations/:id/persons` → `/api/organizations/:id/contacts`

### 1d. `organizations/brand.rs` — INSERT INTO persons → INSERT INTO crm_contacts
- Brand onboarding creates contacts directly instead of persons

### 1e. `mcp/task_server/policy.rs` — Search persons → search crm_contacts
- Update MCP search to query crm_contacts instead of persons

### 1f. Remove `persons.rs` routes and `person.rs` model
- Remove route registration from routes/mod.rs
- Keep model file temporarily if needed for migration compat, but remove from active use

---

## W2: Frontend — Replace personsApi with contacts API

### 2a. `lib/api/intelligence.ts` — Point to contact endpoints
- `triggerResearch()` → POST `/api/crm/contacts/{id}/research`
- `getStatus()` → GET `/api/crm/contacts/{id}/intelligence-status`
- `listResearchPasses()` → GET `/api/crm/contacts/{id}/research-passes`
- etc.

### 2b. `lib/api/communication.ts` — Remove personsApi, use contactsApi
- Remove `PersonRecord` interface (use `CrmContactRecord` from shared types)
- Remove `personsApi` object
- Update all importers

### 2c. `pages/leads.tsx` — Use contacts API
- Replace `personsApi` calls with contacts equivalents

### 2d. Remove legacy person pages
- `person-detail.tsx` — already a redirect, can simplify or remove
- `person-intel.tsx` — remove, Intel tab lives in contact detail
- `person-profile.tsx` — remove, profile lives in contact detail
- `App.tsx` — remove `/persons/*` routes (or keep redirects)

### 2e. Update remaining frontend files
- `query-keys.ts` — update person-related keys
- `useOrgContacts.ts` — verify it uses contacts API
- `ScheduleMeetingDialog.tsx` — use CrmContactRecord
- `BreadcrumbNav.tsx` — remove persons references
- Company profile tabs — use contacts API

---

## W3: Migration — Drop persons table

- Migration: `DROP TABLE IF EXISTS persons; DROP TABLE IF EXISTS person_notes;`
- Only after all code references are removed and cargo check passes

---

## W4: Cleanup

- Remove unused imports across all modified files
- `cargo check --workspace`, `npx tsc --noEmit`
- Verify: `grep -r "persons" crates/ --include="*.rs" | grep -v test | grep -v migration | grep -v comment` → zero

---

## Verification

1. `cargo check --workspace` — 0 errors
2. `npx tsc --noEmit` — 0 errors
3. No active Rust code references persons table (migrations excluded)
4. Frontend builds clean with no person API references

## Status: NOT STARTED

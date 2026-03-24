# Sloperation317 Integration + Quality Sprint

**Date**: 2026-03-18
**Branch**: `integration/sloperation317-quality` (from main `445e6d7b8`)
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Status**: Complete — Ready for PR

## Context

`sloperation317` has 11 commits with dealflow pipeline automation, company brand profiles, E2E tests, and DB migrations. It's 3 PRs behind main (#46, #47, #48).

**Critical finding**: sloperation317 **reverts** the DbUuid migration (PR #48) — it changes `Path<String>` back to `Path<Uuid>`, reverts `AccessContext.user_id` from `DbUuid` to `uuid::Uuid`, removes `DbUuid::to_uuid()`, and deletes our WelcomeWizard, SetupProgress, file splits (data-sources/, virtual-environment/, project-tasks/, meeting-mode/), and Cargo.lock.

**Decision**: Keep DbUuid standards (PR #48). Do NOT merge the branch directly — cherry-pick features only, porting new code into main's architecture.

---

## What Sloperation317 Adds (features to preserve)

### Backend
- **CRM pipeline stages**: `advance-requirements` endpoint, `call_llm()` with OpenAI/Anthropic fallback, `trigger_deep_research_pass2`, `generate_proposal/deck`, `approve_proposal` — all in `crm_deals.rs`
- **Company brand profiles**: New table + model + routes (`get_company_brand_profile`, `upsert_company_brand_profile`) in `companies.rs`
- **Intelligence redesign**: `run_research_direct()` prefers OpenAI GPT-4o with Anthropic fallback, simplified prompts
- **DB migrations**: 20260406 (CRM BLOB→TEXT, 672 lines), 20260407 (clean BLOB stages), 20260409 (company_brand_profiles table)

### Frontend
- **CompanyBrandGuidePage.tsx** (202 lines) — new page for company brand profiles
- **Role gate changes**: `platform_member` → `org_viewer` for CRM/people/companies routes
- **New App.tsx routes**: `/companies/:companyId/brand-guide`

### E2E Tests (9 files, ~2,500 lines) — see Workstream B2 for quarantine plan
- `company-links.spec.ts`, `dealflow-pipeline.spec.ts` (modified), `full-pipeline-walkthrough.spec.ts`
- `hudson-as-sirak.spec.ts`, `hudson-exact-flow.spec.ts`, `operator-walkthrough.spec.ts`
- `pipeline-userflows.spec.ts`, `trace-company-nav.spec.ts`, `visual-pipeline-walkthrough.spec.ts`

---

## What We Discard from Sloperation317

- All UUID reversions (`Path<Uuid>`, `AccessContext: Uuid`, removal of DbUuid helpers)
- Monolith resurrections (`virtual-environment.tsx`, `data-sources.tsx`) — keep main's split directories
- Deletion of WelcomeWizard, SetupProgress, nora/assistant splits, meeting-mode splits
- Deletion of Cargo.lock
- Deletion of `crates/utils/src/volume.rs`

---

## PR #49 Deconfliction

PR #49 (`refactor/frontend-polish-sprint`) is a 23-commit, 231-file frontend polish sprint that will merge to main soon. Key changes:

- **Repo root cleanup**: ~207→28 items, docs/scripts moved to subdirs, 118MB tracked binaries removed
- **Query key fixes**: 10 cache invalidation bugs fixed, 8 more inline keys → factories
- **Mutations**: 22 safe-zone mutations → `useMutationWithToast`
- **Webhook triggers**: New feature — HMAC validation, execution audit trail, migration `20260411000000`
- **Topsi connection status**: New feature + voice hook extraction
- **Task card refactor**: Sub-components extracted, agent name badges
- **E2E demo enhancements**: GitHub PR navigation, .env auto-loading in playwright config
- **useTaskFormState split**: 794→632 lines
- **fetch() migration**: 28 raw `fetch()` → `makeRequest`

**Strategy**: Merge main after PR #49 lands, but start safe-zone work immediately.

**After PR #49 merges**:
1. `git fetch origin && git merge origin/main` into our branch
2. Resolve conflicts (planning docs likely, code unlikely)
3. Proceed with conflict-zone workstreams

### Safe Zones (can start NOW, before PR #49 merges)

These files/areas have **zero overlap** with PR #49's 231 changed files:

#### Backend Safe Zones
| Area | Files | Why Safe |
|------|-------|----------|
| **CRM deals routes** | `crates/server/src/routes/crm_deals.rs` | PR #49 doesn't touch CRM deal routes |
| **Companies routes** | `crates/server/src/routes/companies.rs` | PR #49 doesn't touch companies |
| **Intelligence routes** | `crates/server/src/routes/intelligence.rs` | PR #49 touches `frontend/src/lib/api/intelligence.ts` but NOT the backend route |
| **DB migrations** | `crates/db/migrations/2026040{6,7,9}*.sql` | New files, no collision (PR #49 uses 20260411) |
| **Company brand model** | `crates/db/src/models/company_brand_profile.rs` | New file |
| **DB lib (DATABASE_URL)** | `crates/db/src/lib.rs` | PR #49 doesn't touch this |

#### Frontend Safe Zones
| Area | Files | Why Safe |
|------|-------|----------|
| **CompanyBrandGuidePage** | `frontend/src/pages/brand-guide/CompanyBrandGuidePage.tsx` | New file (PR #49 touches `BrandSetupWizard.tsx` in same dir — different file) |
| **App.tsx routes** | `frontend/src/App.tsx` | PR #49 doesn't modify App.tsx |
| **E2E quarantine** | `e2e/quarantine/*.spec.ts` | New directory, PR #49 only touches `e2e/demos/` and `e2e/helpers/` |

#### Docs/Planning Safe Zones
| Area | Files | Why Safe |
|------|-------|----------|
| **Our planning doc** | `planning/2026-03-18--plan--sloperation317-integration.md` | PR #49 has its own planning doc |
| **UX observations** | `planning/reviews/2026-03-18--review--ux-observations.md` | New file |
| **Test seed script** | `scripts/create-test-seed.sh` | New file (PR #49 moves existing scripts, doesn't create new ones here) |

### Conflict Zones (wait for PR #49 merge)

| Area | Our Change | PR #49 Change | Resolution |
|------|-----------|---------------|------------|
| `crates/db/src/models/mod.rs` | Add `company_brand_profile` | Add `trigger_execution`, `workflow_trigger` | Both add lines — merge cleanly |
| `frontend/src/lib/query-keys.ts` | Add new CRM query keys | Adds 5 new factories, fixes 10 mismatches | Merge first, then add ours |
| `frontend/src/hooks/useCrmPipeline.ts` | May modify for new pipeline stages | 3 mutations → `useMutationWithToast` | Merge first, port on top |
| `frontend/src/components/tasks/TaskCard.tsx` | Sloperation317 may reference | Extracted to sub-components (396→259 lines) | Merge first, use new structure |
| `planning/BACKLOG--remaining-work.md` | Add test DB + UX items | PR #49 adds its own backlog items | Merge first, append ours |
| `e2e/helpers/*` | May add `seed.ts` | PR #49 adds `demo/`, `timing.ts`, `ui.ts` | Both additive — safe after merge |
| `playwright.config.ts` | Add quarantine testIgnore | PR #49 adds .env loading | Merge first, add quarantine |

### Pre-Merge Work Plan

**Start immediately** (Workstream A safe-zone items):
1. Port DB migrations (20260406, 20260407, 20260409) — all new files
2. Port `crm_deals.rs` new functions — PR #49 doesn't touch this file
3. Port `companies.rs` brand profile routes — PR #49 doesn't touch this file
4. Port `intelligence.rs` backend changes — PR #49 doesn't touch backend route
5. Add `company_brand_profile` model — new file
6. Modify `crates/db/src/lib.rs` for DATABASE_URL — PR #49 doesn't touch this

**Start immediately** (Workstream B2 safe-zone items):
7. Copy E2E tests to `e2e/quarantine/` — new directory
8. Add `CompanyBrandGuidePage.tsx` — new file

**Wait for PR #49 merge**:
9. `crates/db/src/models/mod.rs` — register new model
10. `frontend/src/lib/query-keys.ts` — add CRM keys
11. `frontend/src/hooks/useCrmPipeline.ts` — pipeline stage changes
12. `playwright.config.ts` — quarantine testIgnore
13. `planning/BACKLOG--remaining-work.md` — append items
14. `e2e/auth.setup.ts`, `e2e/helpers/seed.ts` — test infra

---

## Sprint Workstreams

### A. Cherry-Pick Pipeline Features into Main (Day 1-2)

Port the CRM pipeline automation code into main's codebase, converting to DbUuid standards:

1. **DB migrations**: Add 20260406 (CRM BLOB→TEXT), 20260407 (clean stages), 20260409 (company_brand_profiles). Renumber if collisions exist with main.
2. **crm_deals.rs**: Cherry-pick new functions (`call_llm`, `advance-requirements`, `generate_proposal/deck`, `approve_proposal`, `trigger_deep_research_pass2`). Convert any `Uuid::parse_str` → `DbUuid::parse`, any `Path<Uuid>` → `Path<String>`.
3. **companies.rs**: Add `get_company_brand_profile` + `upsert_company_brand_profile` routes with DbUuid patterns.
4. **DB model**: Add `company_brand_profiles` model (or check if one already exists from sloperation316 integration).
5. **intelligence.rs**: Port the OpenAI-first LLM fallback logic from `run_research_direct()` if it improves on main's version.

**Files**:
- Modify: `crates/server/src/routes/crm_deals.rs`
- Modify: `crates/server/src/routes/companies.rs`
- Modify: `crates/server/src/routes/intelligence.rs`
- New: `crates/db/migrations/2026040{6,7,9}*.sql` (renumbered if needed)
- New/Modify: `crates/db/src/models/company_brand_profile.rs` (if new)

### B. Cherry-Pick Frontend Features + Docs (Day 2-3)

#### B1. Frontend Features
1. **CompanyBrandGuidePage.tsx**: Add to `frontend/src/pages/brand-guide/`
2. **App.tsx**: Add `/companies/:companyId/brand-guide` route, update role gates (`platform_member` → `org_viewer` where appropriate)
3. Apply DbUuid/query key standards to any new frontend code
4. **Docs**: Copy `planning/2026-03-18--pipeline-status.md` to `planning/` as reference. Copy VIBE tokenomics doc to `planning/notes/`.

#### B2. E2E Test Quarantine

**Key finding**: Playwright config (`playwright.config.ts`) discovers tests from `./e2e/` (root), NOT `frontend/tests/e2e/`. The sloperation317 tests live at `frontend/tests/e2e/` which is **already outside the discovery path**. However, we want a proper quarantine with an explicit promotion workflow.

**Strategy**: Place all 9 tests in `e2e/quarantine/` with a `testIgnore` exclusion in Playwright config. Promote individually to `e2e/` after verification passes.

**Steps**:
1. Create `e2e/quarantine/` directory
2. Copy all 9 test files from sloperation317 into `e2e/quarantine/`
3. Update `playwright.config.ts` main project: add `quarantine/` to `testIgnore` pattern (alongside existing `demos/`)
4. Add `e2e/quarantine/README.md` explaining the promotion process
5. Verify quarantined tests individually (see F below), then move passing ones to `e2e/`

**Test review summary** (from sloperation317 analysis):

| File | Lines | LLM Required? | Seed Data? | Readiness |
|------|-------|---------------|------------|-----------|
| company-links.spec.ts | 383 | No | Hudson deal | Ready after seed verify |
| dealflow-pipeline.spec.ts | 241 | No | Agents, deal | Ready after seed verify |
| full-pipeline-walkthrough.spec.ts | 565 | **Yes** (OpenAI) | Creates own | Needs LLM backend |
| hudson-as-sirak.spec.ts | 134 | No | Hardcoded UUIDs | Ready after seed verify |
| hudson-exact-flow.spec.ts | 210 | No | Hardcoded UUIDs | Ready after seed verify |
| operator-walkthrough.spec.ts | 534 | **Yes** (OpenAI) | Creates own | Needs LLM backend, 15min timeout |
| pipeline-userflows.spec.ts | 187 | No | Hardcoded IDs | Ready after seed verify |
| trace-company-nav.spec.ts | 196 | No | Hardcoded UUIDs | Ready after seed verify |
| visual-pipeline-walkthrough.spec.ts | 558 | **Yes** (OpenAI) | Creates own | Demo-only, 5min timeout, not CI-suitable |

**Promotion order** (easiest to verify first):
1. `pipeline-userflows.spec.ts` — lenient assertions, most portable
2. `dealflow-pipeline.spec.ts` — already existed on main (modified)
3. `hudson-as-sirak.spec.ts` — short, documents known bug
4. `hudson-exact-flow.spec.ts` — focused bug reproduction
5. `company-links.spec.ts` — broader coverage, fragile selectors
6. `trace-company-nav.spec.ts` — 7 navigation methods, some xpath
7. `full-pipeline-walkthrough.spec.ts` — requires LLM
8. `operator-walkthrough.spec.ts` — requires LLM, long timeout
9. `visual-pipeline-walkthrough.spec.ts` — demo-only, move to `e2e/demos/` not `e2e/`

### C. Test Database Isolation (Day 2-3)

**Goal**: Separate test data from development data with copy-on-run isolation. Start with base schema + setup scripts (tests create their own data via API), with migration path to full seed later.

**Rationale**: Tests creating their own data doubles as API/infra testing. High-ROI seeding only — admin user, default org, pipeline config. Entity-specific data (deals, contacts) created by tests in `beforeAll`.

#### C1. Make DBService accept DATABASE_URL
- Modify `crates/db/src/lib.rs` `DBService::new()` to read `DATABASE_URL` env var
- Fallback to current hardcoded `asset_dir()/db.sqlite` if env var not set
- This is the only backend change needed

**File**: `crates/db/src/lib.rs` (line ~28-49)

#### C2. Create test seed database
- Create `dev_assets_seed/test-seed.sqlite` with:
  - Full schema (all migrations applied)
  - Admin user (`admin/admin123`)
  - Default org (`02020202-...`)
  - Pipeline stage definitions (needed for deal tests)
  - NO entity data (no deals, contacts, companies — tests create these)
- Script: `scripts/create-test-seed.sh` — runs migrations on empty DB, inserts minimal fixtures

#### C3. E2E infrastructure updates
- Update `e2e/auth.setup.ts`:
  - Before login, copy `dev_assets_seed/test-seed.sqlite` → `dev_assets/test-db.sqlite`
  - Set `DATABASE_URL=sqlite:dev_assets/test-db.sqlite` for test backend
- Update `playwright.config.ts` webServer (if applicable) to pass `DATABASE_URL`
- Add `e2e/helpers/seed.ts` with API helpers for common entity creation:
  - `createTestOrg()`, `createTestDeal()`, `createTestContact()`, `createTestCompany()`
  - All return created entity for assertions
  - All prefix with `[E2E]` for cleanup

#### C4. Migration path to full seed (future sprint)
- Document in `planning/BACKLOG--remaining-work.md`:
  - Once test suite stabilizes, snapshot a "golden" test DB with common entities
  - Move high-frequency test entities (Hudson deal, etc.) into seed
  - Keep API-creation pattern for test-specific data

**Files**:
- Modify: `crates/db/src/lib.rs`
- New: `dev_assets_seed/test-seed.sqlite`
- New: `scripts/create-test-seed.sh`
- Modify: `e2e/auth.setup.ts`
- New: `e2e/helpers/seed.ts`

### D. Quality Pass on Ported Code (Day 3-4)

Apply PR #48 standards to all new code:
- `Uuid::parse_str` → `DbUuid::parse` in all new route handlers
- `Path<Uuid>` → `Path<String>` in any new handlers
- `: any` → proper types in new frontend code
- Inline query keys → factory functions
- `catch(e: any)` → `catch(e: unknown)` with type guards
- Run eslint + prettier on changed frontend files

### E. UX Review & Observations (After Workstreams A-D, Day 3-4)

**Goal**: After backend porting is complete, do a thorough UX observation session using Playwright MCP. **Documentation is the priority** — fix all blocking UX issues in-sprint, move heavy lifts to a new sprint.

**Timing**: After workstream A (backend) and B (frontend) are ported. Run with dev servers so new endpoints are live.

**Process**:
1. Use Playwright MCP to navigate each section, taking screenshots at every step
2. Document all observations in `planning/reviews/2026-03-18--review--ux-observations.md` with timestamps and screenshots
3. Categorize each finding:
   - **In-sprint fix**: Anything that blocks a user flow, regardless of time estimate
   - **Next sprint**: Heavy lifts, design improvements, new feature ideas → `planning/BACKLOG--remaining-work.md`

**Sections to observe** (thorough review of each):

#### E1. CRM Pipeline
- Deal card layout and information density
- Stage transitions: visual feedback, loading states, error states
- Advance flow UX: button placement, confirmation dialogs, progress indicators
- Pipeline overview: column layout, scrolling, empty columns
- Deal detail panel: tabs, content layout, action buttons
- What happens when LLM calls fail or timeout

#### E2. Company Profiles & Navigation
- Company profile page: layout, data display, edit flow
- Brand guide page: empty state, form fields, save/cancel UX
- All 7 entry points to company pages:
  1. Kanban card company text
  2. Deal detail company link
  3. CRM contacts company column
  4. People directory company name
  5. Person profile company link
  6. Companies list page
  7. Sidebar company links
- Navigation consistency: do all paths land on the same page?

#### E3. Sidebar & Routing
- New routes accessible from sidebar
- Breadcrumbs correct for new pages
- Role-gated pages (`org_viewer`): verify no flash of unauthorized content
- Sidebar collapsed/expanded states with new items
- Deep linking: `/companies/:id/brand-guide` direct navigation
- Back button behavior from new pages

**Deliverables**:
- `planning/reviews/2026-03-18--review--ux-observations.md` — thorough observations with screenshots, organized by section
- Updates to `planning/BACKLOG--remaining-work.md` — new UX items with priority and sprint assignment

### F. Verification & QA (Day 4)

#### F1. Build verification
- `flox activate -- cargo check --workspace` — 0 errors
- `cd frontend && npx tsc --noEmit` — 0 errors
- Verify WelcomeWizard, SetupProgress, file splits all still intact

#### F2. E2E test promotion (quarantine → main suite)
Run each quarantined test individually against dev servers:
```bash
npx playwright test e2e/quarantine/<test>.spec.ts --reporter=list
```
- Tests that pass: move from `e2e/quarantine/` to `e2e/`
- Tests that fail on seed data: fix hardcoded UUIDs or add seed setup, then re-test
- Tests that fail on LLM: keep in quarantine, document in backlog with `@llm-required` note
- `visual-pipeline-walkthrough.spec.ts`: if passing, move to `e2e/demos/` (not main suite)

#### F3. Smoke tests
- Playwright smoke: login → CRM pipeline → advance deal → company brand guide
- Run promoted E2E tests as a batch to confirm no interaction issues

---

## Risk Assessment

| Area | Risk | Mitigation |
|------|------|-----------|
| CRM BLOB→TEXT migration | HIGH — 672 lines, irreversible | Test on dev DB copy first. DbUuid already reads both formats. |
| Pipeline LLM calls | MEDIUM — requires PCG Router / API keys | Feature works but untestable without LLM backend |
| Conflict resolution | LOW — cherry-pick avoids merge conflicts entirely | Manual porting is slower but precise |
| Monolith deletion avoidance | LOW — we simply don't copy the monolith files | Keep main's split directories |
| E2E test breakage | MEDIUM — hardcoded UUIDs, LLM deps | Quarantine dir prevents CI failures; promote individually after verification |
| PR #49 merge conflicts | LOW — migrations don't collide, file overlaps are additive | Merge main after PR #49, resolve planning doc conflicts |

## Verification

- `flox activate -- cargo check --workspace`
- `cd frontend && npx tsc --noEmit`
- `cd frontend && npx eslint src/ --max-warnings=0` on changed files
- Playwright smoke: login → CRM pipeline → company brand guide → data sources
- Run sloperation317's new E2E tests against merged code

### G. Sync Back to Sloperation317 (Day 4-5)

After cherry-picked features are merged to main via PR, merge main back into sloperation317 so the original branch picks up all DbUuid standards, file splits, onboarding wizard, and query key factories:

1. `git checkout sloperation317 && git merge origin/main`
2. Resolve any remaining conflicts (should be minimal since features are now on main)
3. Verify sloperation317 compiles and passes checks
4. This keeps the branch alive for any ongoing intermittent development

---

## Out of Scope

- Agent Flow Orchestration Engine (P0 — separate sprint)
- DbUuid Phase C/D (model batch conversion)
- VIBE tokenomics implementation (planning doc only)
- Virtual environment 3D features (already on main, not modified)

---

## Post-Integration Status (Updated 2026-03-24)

This integration plan was executed across PRs #55–#58. The pipeline was rebuilt into a 9-stage config-driven architecture. Several sloperation intents were not ported during the rebuild.

**Resolved out-of-scope items:**
- Agent Flow Orchestration Engine → fully built (`crates/server/src/agent_flow_executor.rs`)
- DbUuid migration → completed for agent_flows and 13 tables

**See also:**
- `planning/2026-03-18--reference--pipeline-status.md` — original intent document with resolution tracking
- `docs/PIPELINE.md` — canonical architecture doc with full gap analysis (audited 2026-03-24)
- `planning/2026-03-21--plan--pipeline-ops-sprint.md` — the sprint that built the StageTransitionProcessor + AgentFlowExecutor
- `planning/BACKLOG--remaining-work.md` — pipeline specs and remaining test coverage

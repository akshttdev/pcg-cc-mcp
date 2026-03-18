# UX Polish & Quality Sprint 2 — 7 Days

**Date**: 2026-03-18
**Branch**: `refactor/ux-polish-quality-sprint-2` (from main `556d7032a`)
**Status**: Planning

## Context

Post-PR #47 (dev velocity sprint), the codebase has strong backend foundations but frontend UX has gaps identified during smoke testing and exploration:

1. **Onboarding is fragmented** — 4 sequential first-login modals (safety notice → agent/editor config → GitHub → telemetry) plus a separate org-level 9-segment carousel. No way to return to skipped steps. Order doesn't match user priority. Blocks automated E2E testing.
2. **107 `: any` types remain** in org-profile files (from PR #46, not touched in PR #47)
3. **258 `.unwrap()` calls** remain in non-test Rust code
4. **341 `Path<Uuid>`** remain in non-CRM routes (PR #47 did CRM only)
5. **89 inline query keys** remain (fragile cache invalidation)
6. **8 files >900 lines** (3 are dead code from PR #46 splits)
7. **SSE fires before auth** — console errors on login page

**Sloperation branches** (`pipeline-progress`, `vibe-integration`) are active with intermittent development. We avoid their feature areas (CRM pipeline automation, company profiles, brand guides) but can do quality/convention work anywhere.

**Goal**: Unify onboarding into a coherent, dismissible, returnable experience. Continue quality velocity work in non-conflicting areas. Gather UX feedback on frontend features touched.

---

## Sprint Workstreams

### A. Onboarding Unification (Days 1-3)

**Why**: Current onboarding is the #1 UX pain point — 4 mandatory sequential modals that can't be returned to, plus a disconnected org carousel. Users can't skip and come back. Order is wrong (safety disclaimer before useful setup). Blocks E2E testing.

**Current state** (from exploration):
- **User-level**: 4 modals orchestrated by `AppShell.tsx`, flags stored in config JSON file (`disclaimer_acknowledged`, `onboarding_acknowledged`, `github_login_acknowledged`, `telemetry_acknowledged`)
- **Org-level**: 9-segment carousel on org overview page, state in `org_onboarding` DB table
- **No cross-talk**: User and org onboarding don't know about each other

**Scope**:
1. **Merge first-login modals into a single stepped dialog** — replace 4 separate modals with one `WelcomeWizard` component with tabbed/stepper navigation:
   - Step 1: Agent & Editor setup (most useful, do first)
   - Step 2: GitHub connect (optional, skippable)
   - Step 3: Privacy preferences (quick toggle)
   - Safety notice becomes an inline collapsible section, not a blocking modal
2. **Add "Skip for now" + "Return to setup" affordance** — persist partial completion, show a setup progress indicator in sidebar/settings that lets users return
3. **Add `SKIP_ONBOARDING=1` env var** for test environments — bypasses all dialogs
4. **Fix SSE pre-auth** — guard `useAgentDirectory` SSE hook with auth check so login page doesn't fire SSE connections

**Files**:
- Modify: `frontend/src/layouts/AppShell.tsx` (orchestrator)
- Modify: `frontend/src/components/config-provider.tsx` (add env var check)
- New: `frontend/src/components/onboarding/WelcomeWizard.tsx` (unified wizard)
- New: `frontend/src/components/onboarding/SetupProgress.tsx` (return-to-setup indicator)
- Modify: `frontend/src/hooks/useAgentDirectory.ts` (SSE auth guard)
- Keep: existing dialog files (comment out, don't delete per feedback)
- Keep: org carousel unchanged (separate concern, working well)

**Risk**: Medium — touches auth flow adjacent code. Validate with Playwright smoke test.

### B. Remaining `any` Type Elimination (Days 3-4)

**Why**: 107 `: any` remain in org-profile and workflow files. These were skipped in PR #47 because PR #46 was in-flight — now safe to touch.

**Scope** (top files):
- `PulseSection.tsx` (12 any)
- `data-source-detail.tsx` (11)
- `MemberAssignments.tsx` (8)
- `WorkflowsView.tsx` (7)
- `BreadcrumbNav.tsx` (4)
- `ArtifactsView.tsx` (3)
- `MembersTab.tsx` (3)
- `organization-profile/index.tsx` (3)
- `workflows/editor/index.tsx` (3)
- Remaining scattered (15)

**Target**: 107 → ~10 (keep only genuinely dynamic/RJSF types)

### C. Dead Code Cleanup + File Splits (Day 4-5)

**Why**: 3 old monolith files coexist with their split directories (PR #46 created splits but didn't delete originals). Plus 5 files still >900 lines.

**Scope**:
1. **Delete dead code**: `project-tasks.tsx` (split dir exists at `project-tasks/`)
2. **Verify + clean**: Check if `MeetingMode.tsx` and `NoraAssistant.tsx` have split directories — if so, delete originals
3. **Split remaining large files** (pick 2 highest-value):
   - `TopsiWidget.tsx` (963 lines) — good candidate, self-contained widget
   - `StagingReviewPanel.tsx` (933 lines) — workflow staging, complex but separable

### D. Path<Uuid> Phase B Continuation (Days 5-6)

**Why**: 341 `Path<Uuid>` remain in non-CRM routes. PR #47 did CRM routes only. Mechanical transformation, high ROI for consistency.

**Scope**: Target the highest-traffic route files:
- `tasks.rs`, `projects.rs`, `users.rs`, `notifications.rs`
- `invitations.rs`, `org_invitations.rs`
- `data_sources.rs`, `data_source_workflows.rs`
- `sidebar.rs`, `agents.rs`

**Target**: ~150 conversions (341 → ~190), focusing on files that interact with DbUuid models

### E. Inline Query Key Migration (Days 6-7)

**Why**: 89 inline query keys remain. These cause cache invalidation bugs — when a mutation invalidates `['deals']` but a query uses `['crm-deals', orgId]`, the cache doesn't clear.

**Scope**:
- Migrate remaining inline `queryKey: ['string']` to factory functions from `lib/query-keys.ts`
- Fix known key mismatches: `['brandProfile', orgId]` vs `['orgBrandProfile', orgId]`
- Fix `['workflowTemplates']` vs `['workflow-templates']`

**Target**: 89 → ~20 inline keys

### F. UX Polish Pass (Throughout)

**Why**: User requested UX feedback gathering on frontend features touched during the sprint.

**Scope** (gathered via exploration):
- CRM deal card tooltip for "Research needed" badge (#16)
- Remove orphaned project-level CRM routes from App.tsx (dead routes, no nav entry)
- Settings "Coming Soon" items: reduce visual clutter (collapse into single "Planned" section)

---

## Daily Schedule

| Day | Workstream | Deliverable |
|-----|-----------|-------------|
| 1 | A: Onboarding unification | WelcomeWizard component, SSE auth guard |
| 2 | A: Onboarding cont. | Skip/return affordance, env var bypass, AppShell integration |
| 3 | A+B: Finish onboarding + start any elimination | Onboarding complete, org-profile any types started |
| 4 | B+C: Finish any + dead code cleanup | 107→~10 any, dead files deleted, 1-2 file splits |
| 5 | D: Path<Uuid> Phase B cont. | ~150 Path<Uuid> converted in non-CRM routes |
| 6 | D+E: Finish Path<Uuid> + query keys | Query key factory migration |
| 7 | E+F: Finish query keys + UX polish + QA | Polish items, Playwright smoke tests, docs |

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Onboarding modals | 4 sequential blocking | 1 wizard, skippable, returnable |
| `: any` (frontend) | 107 | ~10 |
| `.unwrap()` (non-test) | 258 | 258 (not targeted this sprint) |
| `Path<Uuid>` non-CRM | 341 | ~190 |
| Inline query keys | 89 | ~20 |
| Files >900 lines | 8 | 5 |
| SSE pre-auth errors | Yes | Fixed |

## Verification

- `flox activate -- cargo check --workspace` after Rust changes
- `cd frontend && ./node_modules/.bin/tsc --noEmit` after TS changes
- Playwright Firefox MCP smoke tests: login → onboarding wizard → CRM pipeline → data sources
- Verify `SKIP_ONBOARDING=1` bypasses all dialogs

## Out of Scope

- Agent Flow Orchestration Engine (P0 architecture — separate sprint)
- Workflow trigger system (P1 feature)
- Sloperation branch features (CRM pipeline automation, company profiles)
- DbUuid Phase C (BLOB→TEXT migration) and Phase D (model batch convert)
- Org-level onboarding carousel changes (working well, separate concern)

## Conflict Avoidance

- **sloperation316-pipeline-progress**: Avoid `crm_deals.rs` advanced actions, company profiles, brand guides. Our work is in non-CRM routes, org-profile types, onboarding components.
- **sloperation316-vibe-integration**: Avoid VIBE tokenomics, Dockerfile changes. Our VIBELAND work was already done in PR #47.
- **feature/blob-to-text-scoped**: Avoid DB migration files. Our Path<Uuid> work is route-level only.

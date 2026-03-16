# Modularity Sprint 3 — Pattern Extraction & DRY Cross-Cutting Concerns

**Date:** 2026-03-16
**Branch:** `modularity/sprint-3` from `main` @ current HEAD
**PR:** TBD
**Status:** In Progress

## Context

Sprint 2 (PR #37) split 6 large files (~20K lines reorganized) and centralized formatters/EmptyState/fetch. Sprint 3 shifts focus from **file splits** to **pattern extraction** — eliminating cross-cutting redundancy in query keys, mutations, and raw fetch calls, then splitting the 4 remaining largest components.

## Scope

- Frontend only (Rust route splits deferred to Sprint 4)
- High-traffic raw fetch targets only (~60 of 172 instances)
- useMutationWithToast: create + migrate top 10 consumers
- Split all 4 largest components (TaskFormDialog, company-profile, CrmDealDetailPanel, EnhancedTaskDetailsPanel)

## Implementation Plan

### Day 1: Query Key Factory (Foundation)

**Goal:** Create centralized query key factory. Eliminates 20+ inconsistent key patterns, unblocks correct invalidation in Day 3's mutation wrapper.

1. Create `frontend/src/lib/query-keys.ts` (~120 lines)
   - Hierarchical key factory covering all domains
   - `queryKeys.tasks`, `queryKeys.crm`, `queryKeys.projects`, `queryKeys.organizations`, `queryKeys.sidebar`, `queryKeys.agents`, `queryKeys.workflows`, `queryKeys.executions`
   - Follow `useCrmPipeline.ts` `crmQueryKeys` pattern but centralized

2. Migrate ~14 consumer files
   - Replace inline query key arrays with factory calls
   - Absorb `crmQueryKeys` from `useCrmPipeline.ts`, re-export alias

**Verification:** `npx tsc --noEmit`. Grep `queryKey: ['crm` — expect 80%+ migrated.

### Day 2: API Modules for Raw Fetch Hotspots (~60 calls)

**Goal:** Create 4 new API modules + extend 2 existing ones. Migrate raw fetch from the 7 highest-traffic files.

New modules:
- `lib/api/topsi-routes.ts` (~80 lines) — 7 fetches from `pages/topsi.tsx`
- `lib/api/topiclips.ts` (~70 lines) — 7 fetches from `pages/topiclips.tsx`
- `lib/api/users.ts` (~60 lines) — 5 fetches from `settings/UsersSettings.tsx`
- `lib/api/permissions.ts` (~30 lines) — 4 fetches from `settings/ProjectsSettings.tsx`

Extend existing:
- `lib/api/nora.ts` (+40 lines) — 7 fetches from nora components
- `lib/api/system.ts` (+20 lines) — 4 fetches from `settings/NetworkSettings.tsx`

**Verification:** `npx tsc --noEmit`. Zero `fetch(` in the 7 consumer files.

### Day 3: `useMutationWithToast` Wrapper + Top 10 Migration

**Goal:** Eliminate repetitive mutation boilerplate (useMutation + toast + invalidateQueries) found in 30+ files.

1. Create `frontend/src/hooks/useMutationWithToast.ts` (~60 lines)
2. Migrate top 10 consumers (3+ mutations each):
   - UsersSettings, ProjectsSettings, CrmPipelineSettings, StagingReviewPanel, StagingTab, ApprovalPanel, MembersTab, invoices, proposals, crm

**Verification:** `npx tsc --noEmit`. Grep `toast.success` count drops ~50.

### Day 4: Large File Splits — TaskFormDialog + company-profile

1. `TaskFormDialog.tsx` (1,242 lines) → `task-form/` directory (7 files)
2. `company-profile.tsx` (1,218 lines) → `pages/company-profile/` directory (7 files)

**Verification:** `npx tsc --noEmit`. UI functional for task forms and company profiles.

### Day 5: CrmDealDetailPanel + EnhancedTaskDetailsPanel + Final Sweep

1. `CrmDealDetailPanel.tsx` (1,202 lines) → `crm/deal-detail/` directory (7 files)
2. `EnhancedTaskDetailsPanel.tsx` (1,040 lines) → `task-details/` directory (6 files)
3. Final verification sweep

**Verification:** `npx tsc --noEmit`, all splits functional.

## Expected Outcomes

- Raw `fetch()`: 172 → ~135 (22% reduction)
- Query key patterns: 20+ ad-hoc → 1 centralized factory
- Mutation boilerplate: eliminated in 10 highest-impact files (~50 mutations simplified)
- Components >1,000 lines: 10 → 6

## PR #39 Impact Notes (reviewed 2026-03-16)

PR #39 (UX engagement polish sprint 2) lands before this branch. Key items that affect our work:

### New query keys to include in factory (Day 1)
- `['notifications-inbox']` — NotificationCenter.tsx
- `['org-onboarding', orgId]` — useOrgOnboarding.ts (new hook, uses `ORG_ONBOARDING_KEY` constant)
- `['workflow-runs-builder']` — BuilderTab.tsx
- `['my-tasks']`, `['my-created-tasks']`, `['my-watched-tasks']` — my-tasks.tsx invalidations (prefix-matching pattern)
- Additional `['sidebarTree']` invalidations added to useTaskMutations.ts and my-tasks.tsx

### New raw fetch to extract (Day 2)
- `NotificationCenter.tsx` — `fetch(resolveApiUrl('/api/notifications/${item.id}/read'))` in handleDismiss
- Consider adding a `lib/api/notifications.ts` module (or fold into existing)

### New mutation candidates for useMutationWithToast (Day 3)
- `useOrgOnboarding.ts` — 4 mutations (startOnboarding, startSegment, completeSegment, skipSegment), currently lack onError handlers
- `my-tasks.tsx` — batch status change + batch delete use sequential `for...of await` with manual toast; good refactor target

### TaskFormDialog split (Day 4)
- PR #39 adds `initialStatus` prop (~10 lines changed). Low conflict risk — just include the new prop in the `types.ts` when splitting.

### Files NOT touched by PR #39 (clean for splitting)
- `company-profile.tsx`
- `CrmDealDetailPanel.tsx`
- `EnhancedTaskDetailsPanel.tsx`

### New files from PR #39 (already follow good patterns)
- `frontend/src/hooks/useOrgOnboarding.ts` — centralizes its own key constant, clean hook structure
- `frontend/src/lib/api/onboarding.ts` — uses makeRequest/handleApiResponse, re-exported from index.ts

## Deferred to Sprint 4

- Rust route splits (topsi.rs, organizations.rs, nora.rs)
- Remaining ~110 raw fetch calls
- Remaining 20+ mutation consumers
- MeetingMode.tsx, virtual-environment.tsx, useConversationHistory.ts / useAutonomy.ts

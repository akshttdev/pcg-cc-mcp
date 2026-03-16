# Modularity Sprint 3 — Pattern Extraction & DRY Cross-Cutting Concerns

**Date:** 2026-03-16
**Branch:** `modularity/sprint-3` from `main`
**PR:** TBD
**Status:** Complete — ready for PR

## Context

Sprint 2 (PR #37) split 6 large files (~20K lines reorganized) and centralized formatters/EmptyState/fetch. Sprint 3 shifts focus from **file splits** to **pattern extraction** — eliminating cross-cutting redundancy in query keys, mutations, and raw fetch calls, then splitting the 4 remaining largest components.

## What Was Delivered

### 8 commits, 53 files changed (+6,525 / -4,970 lines)

| Commit | Summary |
|--------|---------|
| 1 | Query key factory (`lib/query-keys.ts`) + 10 consumer migrations |
| 2 | `useMutationWithToast` hook + 4 consumer migrations (invoices, proposals, crm, StagingTab) |
| 3 | UsersSettings, ProjectsSettings, StagingReviewPanel → useMutationWithToast |
| 4 | API module extraction: `permissions.ts` new, `usersApi` extended (9 raw fetch eliminated) |
| 5 | TaskFormDialog (1,242 lines) → `task-form/` directory (6 files) |
| 6 | CrmDealDetailPanel (1,203 lines) → `deal-detail/` directory (8 files) |
| 7 | company-profile (1,219 lines) → `company-profile/` directory (7 files) |
| 8 | EnhancedTaskDetailsPanel (1,040 lines) → `enhanced/` directory (7 files) |

### Actual Outcomes vs Plan

| Metric | Planned | Actual |
|--------|---------|--------|
| Query key factory | 1 centralized | Done — ~280 lines, covers all domains |
| Consumer files migrated to factory | ~14 | 10 (high-traffic files prioritized) |
| `useMutationWithToast` consumers | 10 | 7 (15 mutations migrated) |
| API modules | 4 new + 2 extended | 1 new (`permissions.ts`) + 1 extended (`usersApi`) |
| Raw fetch eliminated | ~60 | ~9 (UsersSettings + ProjectsSettings only) |
| Components >1,000 lines split | 4 | 4 (all completed) |
| Components >1,000 lines remaining | 6 | 6 (virtual-env, MeetingMode, NoraAssistant, topsi, data-sources, project-tasks) |
| TypeScript errors | 0 | 0 |

### What Was Descoped (moved to Sprint 4)

- **API modules for topsi, topiclips, nora, NetworkSettings** — researched but not implemented; ~35 raw fetch calls remain in those files
- **Remaining mutation consumers** — CrmPipelineSettings, ApprovalPanel, MembersTab not migrated
- **Remaining query key migrations** — ~60% of inline keys still use string literals in non-migrated files

## PR #39 Merge Notes

PR #39 (UX engagement polish sprint 2) was squash-merged to main and merged into this branch. 3 conflicts resolved:

1. **TaskFormDialog.tsx** — PR #39 modified the monolithic file; we keep barrel re-export (our `task-form/` already has `initialStatus` prop)
2. **lib/api/index.ts** — both sides added exports; merged to include both `permissions` and `onboarding`
3. **my-tasks.tsx** — auto-merged cleanly; PR #39's batch handlers + our query key factory coexist

### Post-merge regression check: all clear
- `useOrgOnboarding` hook intact with `ORG_ONBOARDING_KEY`
- `onboarding.ts` API module exported from index
- `initialStatus` prop in `task-form/types.ts` and `useTaskFormState.ts`
- Notification quick actions in NotificationCenter unchanged
- my-tasks batch handlers using our `taskKeys`/`sidebarKeys` factory (not inline strings)
- `npx tsc --noEmit`: zero errors

## QA Review Findings (self-review 2026-03-16)

### Regression found & fixed
- `useTaskMutations.ts` — `if (projectId)` guard silently suppressed cache invalidation when projectId was undefined. Fixed in be4b674 by restoring original `['tasks', projectId]` inline pattern.

### Known issues (not blocking merge, queued for Sprint 4)
- `enhanced/index.tsx` is 749 lines (exceeds 500-line rule) — extract data-fetching into `useTaskPanelData.ts`
- `useTaskFormState.ts:472-484` — duplicate `useEffect` setting executor profile (pre-existing)
- `useAgents.ts` — partially migrated; `useActiveAgents`, `useAgent`, `useAgentByName` still bypass factory
- `NotificationCenter.tsx:50` — `['notifications']` activity query key not in factory
- `StagingTab.tsx:78` — `['stagingPending', orgId]` doesn't match factory's `workflowKeys.stagingPending()` (no orgId)
- `company-profile/tabs/OverviewTab.tsx` — duplicate `Tab` type definition (also in index.tsx)
- `deal-detail/tabs/ActivityTab.tsx:10` — `organization_id` used as `projectId` fallback (pre-existing bug)
- `enhanced/index.tsx:298` — `any` type usage in vibe transaction filter
- ~8 `console.log` debug statements in `useTaskFormState.ts` (pre-existing)

### Pre-existing patterns preserved (not regressions)
- Singular vs plural key prefixes (`['task', id]` vs `['tasks']`) — matches original inline keys
- `taskKeys.list` / `taskKeys.projectTasks` dual purpose — matches original codebase patterns

## New items for Sprint 4 (expanded from PR #39 + QA review)

**From QA review:**
- Extract `enhanced/index.tsx` data-fetching into `useTaskPanelData.ts` hook (749 → ~300 lines)
- Remove duplicate `useEffect` in `useTaskFormState.ts`
- Complete `useAgents.ts` factory migration
- Add `['notifications']` activity key to factory + migrate consumer
- Fix `StagingTab` to use factory key with orgId param
- Deduplicate `Tab` type in company-profile

**From PR #39:**
- `my-tasks.tsx` batch handlers — candidates for `useMutationWithToast`
- Remaining query key factory adoption (~40% of files still use inline strings)
- API modules: topsi-routes.ts, topiclips.ts, nora extensions, system extensions

**Larger scope:**
- Rust route splits: topsi.rs (2,800), organizations.rs (2,665), nora.rs (2,475)
- Large component splits: MeetingMode.tsx (1,207), virtual-environment.tsx (1,282)

## File Split Details

### TaskFormDialog.tsx → task-form/
```
task-form/
├── types.ts              (78)  — TaskFormDialogProps + TaskFormState interfaces
├── useTaskFormState.ts   (808) — 25+ useState, effects, handlers
├── BasicInfoSection.tsx  (68)  — title + description
├── FieldsSection.tsx     (242) — priority, board, due date, assignee, MCPs, tags
├── QuickstartSection.tsx (224) — images, templates, executor, branch
└── index.tsx             (189) — NiceModal wrapper, dialog layout
```

### company-profile.tsx → company-profile/
```
company-profile/
├── index.tsx                     (299) — page shell, tab routing, data hooks
├── components/helpers.tsx        (213) — StarRating, IntelBadge, ContactRail, etc.
├── tabs/OverviewTab.tsx          (308) — overview with contact rail, metrics
├── tabs/EditTab.tsx              (205) — company field editor
├── tabs/IntelligenceTab.tsx      (100) — research status, AI summary
├── tabs/ContactsTab.tsx          (82)  — contacts list
└── tabs/ProposalsTab.tsx         (42)  — proposals listing
```

### CrmDealDetailPanel.tsx → deal-detail/
```
deal-detail/
├── index.tsx                (151) — Sheet shell, tab routing
├── DealHeader.tsx           (225) — avatar, name, PipelineStepper
├── tabs/OverviewTab.tsx     (388) — deal fields, metrics, contact card
├── tabs/ReviewTab.tsx       (318) — stage checklist, advance button
├── tabs/IntelTab.tsx        (303) — AI summary, business report
├── tabs/ProjectsTab.tsx     (91)  — client projects list
├── tabs/ActivityTab.tsx     (16)  — timeline wrapper
└── hooks/useDealActions.ts  (81)  — mutations
```

### EnhancedTaskDetailsPanel.tsx → enhanced/
```
enhanced/
├── index.tsx                (749) — panel shell, data fetching
├── TaskHeader.tsx           (61)  — breadcrumb + header
├── tabs/OverviewTab.tsx     (192) — description, criteria, agent terminal
├── tabs/VibeBudgetTab.tsx   (145) — vibe balance + transactions
├── tabs/ArtifactsTab.tsx    (58)  — artifacts gallery
├── tabs/WorkflowTab.tsx     (57)  — workflow view
└── tabs/ActivityTab.tsx     (39)  — activity timeline
```

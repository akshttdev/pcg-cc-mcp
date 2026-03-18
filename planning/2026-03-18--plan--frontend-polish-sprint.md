# 10-Day Sprint: Frontend Polish, Features, and Repo Hygiene

**Date**: 2026-03-18
**Branch**: `refactor/frontend-polish-sprint`
**Base**: `main` (post-PR #46 + PR #45)
**Status**: In Progress

## Progress Tracker

| Day | Task | Status | Commit |
|-----|------|--------|--------|
| 0 | Branch setup + planning file | DONE | `de9073140` |
| 1 | Repo root cleanup (~100 file moves, 118MB binary removal) | DONE | `d9a190d5e` |
| 2 | Cherry-pick dev-velocity frontend work | DONE (provisional) | `48bab1ad5`, `c9dbfd4`, `6f433b1c` |
| 3 | Query key mismatch fixes (10 critical bugs + 30 inline→factory) | DONE | `67ddc5724` |
| 4 | Mutation standardization (22 safe-zone conversions) | DONE | `c443e5970` |
| 5 | TaskFormDialog split (useTaskFormState 794→632 lines, 3 hooks extracted) | DONE | `504791fdc` |
| 6 | CrmDealDetailPanel split | SKIPPED (sloperation overlap) | |
| 7 | Webhook triggers + execution audit trail | DONE | `7966ec184` |
| 8 | Topsi connection status + voice hook extraction (963→616 lines) | DONE | `fbe4ab240` |
| 9 | fetch() migration + query key sweep | NOT STARTED | |
| 9.5 | QA infrastructure (skills + rules) | DONE | `f0b2cc8af` |
| 10 | Sprint QA + retrospective | NOT STARTED | |
| -- | **PR #47 rebase** | **NEXT** — PR #47 merged, rebase needed | |

---

## PR #47 Deconfliction (added 2026-03-18)

PR #47 (`refactor/dev-velocity-sprint`) is OPEN and overlaps with **32 files** on our branch:
- **Identical content (cherry-picks)**: virtual-env split (9 files), data-sources split (8 files), `any` type fixes (13 files)
- **Divergent edits**: `client-overview.tsx`, `organization-overview.tsx`, `OutputNodeConfig.tsx` (we added query-key fixes on top of their `any` type fixes)

**Strategy: Continue in safe zones until PR #47 merges, then rebase.**

When PR #47 merges to main:
1. `git fetch origin && git rebase origin/main`
2. Drop 3 cherry-pick commits (content already on main)
3. Resolve 3 divergent file conflicts (keep our query-key additions, accept their `any` type base)

**Safe zones (no PR #47 overlap)**: hooks/, settings pages, task-form/, crm deal-detail tabs, TopsiWidget, workflows/StagingReviewPanel, workflows/StagingTab, workflows/BuilderTab, all dialog files, email/communications components, brand-guide, person-intel, review.tsx, project-deliverables, all root-level file moves (Day 1).

---

## Context

Post-quality sprint (PR #46 merged), post-sloperation316 integration (PR #45 merged). The codebase has strong modularity patterns but incomplete adoption. Two feature gaps (Workflow Triggers, Topsi connection status) and significant root directory clutter.

**Starting metrics:**
- ~270 `any` types (188 in safe zones)
- ~90 inline query keys remaining (down from 318 pre-PR #46)
- ~50 files with raw `useMutation` (vs `useMutationWithToast`)
- 7 files >900 lines in frontend
- ~87 raw `fetch()` calls (vs typed API modules)
- ~207 items in repo root
- 10+ query key string mismatches (factory vs usage)

---

## Workstreams

| ID | Workstream | Sprint Target |
|----|-----------|---------------|
| A | Repo Root Cleanup | Move ~100 files, remove tracked binaries |
| B | Cherry-pick dev-velocity frontend work | 182 `any` types -> 0, virtual-env + data-sources splits |
| C | Query Key + Mutation Completion | ~90 -> ~20 inline keys, ~15 more mutations, fix mismatches |
| D | Large File Splits | useTaskFormState + CrmDealDetailPanel |
| E | Workflow Trigger Hardening | Webhook triggers, retry/error handling |
| F | Topsi Connection Status | Connection indicator + voice gating + TopsiWidget split |
| G | Remaining `fetch()` -> API Module | ~20 more raw fetch calls migrated |

**Frontend/Backend split: ~75/25**

---

## Day-by-Day Plan

### Day 0 (Start): Branch Setup + Planning File -- DONE

1. Created branch `refactor/frontend-polish-sprint` from `main`
2. Wrote planning file
3. Committed

---

### Day 1: Repo Root Cleanup -- DONE

**Workstream A** -- 160 files changed in commit `d9a190d5e`

- Moved ~50 markdown docs to `docs/` subdirectories (apn, deployment, docker, sovereign-stack, architecture, core-features, getting-started, session-logs, status-logs, integration-guides)
- Moved ~45 shell/Python scripts to `scripts/`
- Moved .desktop files to `deploy/desktop/`, .service files to `deploy/systemd/`
- Untracked 118MB of binaries (flox.deb, test_send_message, pcg-transfer tarball, TopiClips webp, app.db)
- Updated references in Makefile, Dockerfile, Dockerfile.apn-bridge, README.md, CLAUDE.md, package.json, INDEX-OF-DOCS.md
- Root directory reduced from ~207 to ~28 items

---

### Day 2: Cherry-pick dev-velocity -- DONE (provisional, will be dropped on rebase after PR #47)

Cherry-picked 3 commits from `refactor/dev-velocity-sprint`:
1. `8b2d05387` -- 182 `: any` type annotations eliminated (13 files)
2. `c9dbfd484` -- virtual-environment.tsx split (1,282 -> 9 files)
3. `6f433b1cf` -- data-sources.tsx split (991 -> 7 files)

**NOTE**: These will be dropped when PR #47 merges and we rebase. The identical content will already be on main.

---

### Day 3: Query Key Mismatch Fixes -- DONE

**Workstream C** -- 21 files changed in commit `67ddc5724`

**10 critical cache invalidation bugs fixed:**
1. `brandProfile` vs `orgBrandProfile` (SocialAccountsView, SocialOverviewView)
2. `workflowTemplates` vs `workflow-templates` (WorkflowsView)
3. `staging-pending` vs `stagingPending` (RunWorkflowDialog)
4. `crm-kanban` vs `['crm','kanban']` (ProposalTab, DeckTab, useDealActions)
5. `crm-deal` vs `['crm','deal']` (ProposalTab)
6. `orgClients` vs `org-clients` (client-overview, organization-overview)

**30+ inline keys converted to factory usage** across: project-context, project-controller, projects, crm-contact-detail, OutputNodeConfig, OverviewTab, person-intel, organization-context, ClientProjectPanel, WorkflowRunsPanel, ArticlePreview, business-reports

---

### Day 4: Mutation Standardization -- IN PROGRESS

**Workstream C** -- Convert raw `useMutation` -> `useMutationWithToast` in safe-zone files.

**Best candidates (already have manual toast calls -- exact pattern match):**
- `components/dialogs/controller-settings-dialog.tsx` (1 mutation) -- SIMPLE
- `components/agents/AgentExecutionConfigPanel.tsx` (2 mutations) -- SIMPLE
- `components/crm/deal-detail/tabs/TranscriptsTab.tsx` (1 mutation) -- SIMPLE
- `pages/brand-guide/BrandSetupWizard.tsx` (2 mutations) -- SIMPLE
- `components/tasks/CommentInput.tsx` (1 mutation) -- MEDIUM (has toast + state reset + activity logging)

**Good candidates (no toasts yet -- adds UX improvement):**
- `hooks/useAgentFlows.ts` (4 mutations) -- SIMPLE, invalidation only
- `hooks/useCrmPipeline.ts` (3 of 4: createDeal, updateDeal, deleteDeal) -- SIMPLE. Skip `useMoveDeal` (optimistic update with onMutate/onSettled)
- `components/dialogs/project-members-dialog.tsx` (3 mutations) -- SIMPLE
- `components/dialogs/client-members-dialog.tsx` (2 mutations) -- SIMPLE
- `pages/workflows/tabs/StagingTab.tsx` (2 mutations) -- SIMPLE
- `pages/workflows/tabs/BuilderTab.tsx` (2 mutations) -- SIMPLE
- `components/workflows/WorkflowTriggersPanel.tsx` (3 mutations) -- SIMPLE
- `components/email/EmailInbox.tsx` (3 mutations) -- SIMPLE
- `components/communications/CommunicationsInbox.tsx` (2 mutations) -- SIMPLE

**Skip (complex patterns -- keep raw):**
- `hooks/useTaskMutations.ts` -- navigation + activity logging callbacks
- `hooks/useRebase.ts` -- custom typed error generics
- `hooks/useProfiles.ts` -- uses setQueryData, not invalidation
- `hooks/useDevServer.ts` -- async await on invalidation
- `hooks/useExecutionSummary.ts` -- uses setQueryData
- `hooks/useAttemptCreation.ts` -- setQueryData + navigation
- `hooks/useCrmPipeline.ts` `useMoveDeal` -- optimistic update with onMutate/onSettled
- `components/workflows/StagingReviewPanel.tsx` (5 of 6 mutations) -- conditional error state logic

**Overlap exclusions (PR #47):** oss-library-listener.tsx, data-sources/RunWorkflowFromSourceDialog, OutputNodeConfig.tsx, PulseWidget.tsx, pulse.tsx, WorkflowDetailPanel.tsx, organization-profile/index.tsx

**Target**: ~25 mutations converted across ~15 files

---

### Day 5: Large File Split -- `useTaskFormState.ts` (794 lines)

**Workstream D**

TaskFormDialog is already split into `task-form/` directory (6 files, 1,599 total lines). The monolithic hook `useTaskFormState.ts` (794 lines) is the splitting target.

**Split plan:**

| New File | Lines | Content |
|----------|-------|---------|
| `useFormReset.ts` | ~140 | Form reset effect (edit/duplicate/template/blank modes) |
| `useBoardLoader.ts` | ~100 | 3 board-related effects (load, validate, auto-select) |
| `useBranchLoader.ts` | ~65 | Templates + branches loader + parent attempt resolver |
| `buildTaskPayload.ts` | ~50 | Pure function extracting duplicated payload construction |
| `DiscardWarningDialog.tsx` | ~30 | Inline second Dialog from index.tsx |
| `useTaskFormState.ts` (after) | ~350 | Remaining state + handlers |

**Key coupling to preserve:**
- `handleSubmit` and `handleCreateAndStart` share payload-building logic -> consolidate in `buildTaskPayload()`
- Board selection effects must remain sequenced (load -> validate -> auto-select)
- `hasUnsavedChanges` closes over 11 state vars

---

### Day 6: Large File Split -- `CrmDealDetailPanel.tsx` (1,202 lines)

**Workstream D**

Check sloperation overlap first. If overlap, defer to `company-profile.tsx` (1,218 lines) instead.

---

### Day 7: Workflow Trigger Hardening -- Webhook Support

**Workstream E** (Backend)

1. Add `webhook_secret`, `webhook_url` fields to `workflow_triggers` model
2. Add `POST /api/webhooks/:trigger_id` endpoint with HMAC secret validation
3. Add retry logic: `last_error`, `retry_count`, `next_retry_at` on trigger execution failures
4. Add `trigger_executions` table for audit trail
5. Rate limiting: per-trigger cooldown
6. Frontend: webhook trigger type option + URL/secret display

---

### Day 8: Topsi Connection Status + Voice Feature Gating

**Workstream F**

1. Connection status indicator (colored dot, tooltip)
2. Voice feature gating (mic permission, backend availability)
3. TopsiWidget split: extract prompt pills + voice controls (~200 lines each)

---

### Day 9: Remaining `fetch()` Migration + Query Key Sweep

**Workstream G + C**

Target ~20 raw fetch -> typed API modules + inline keys under 30.

---

### Day 9.5: QA Infrastructure

Create `/qa-review` and `/playwright-smoke` skills. Update git-workflow and frontend-standards rules.

---

### Day 10: Sprint QA + Retrospective

Full QA pass: tsc, lint, cargo check, E2E tests, Playwright MCP smoke tests, regression diff vs main.

---

## Sprint Metrics

| Metric | Before | Current | Target | Stretch |
|--------|--------|---------|--------|---------|
| Root dir file count | ~207 | **~28** | ~30 | ~25 |
| Git-tracked binaries | 118MB | **0** | 0 | 0 |
| `: any` types | ~270 | ~80 (provisional) | ~80 | 0 |
| Inline query keys | ~90 | **~55** | ~30 | ~20 |
| Query key mismatches | 10+ | **0** | 0 | 0 |
| Raw `useMutation` (convertible) | ~50 | ~50 | ~25 | ~20 |
| Files >900 lines (FE) | 7 | 5 (provisional) | 3 | 2 |
| Webhook trigger support | No | No | Yes | Yes + UI |
| Topsi connection status | No | No | Yes | Yes |

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| PR #47 merge timing | Cherry-pick commits become conflicts on rebase | Drop cherry-pick commits, keep only our original work |
| Root dir moves break CI/scripts | Build failure | Already verified: updated Makefile, Dockerfile, package.json, CLAUDE.md |
| Sloperation branches drift further | Merge conflict later | Sprint avoids all sloperation overlap zones |
| Webhook triggers need auth design | Security gap | Use HMAC secret validation (standard pattern) |
| TopsiWidget split breaks voice state | Runtime error | Lift state to parent, pass as props |
| Mutation conversions miss edge cases | Toast spam or missing feedback | Skip criteria: optimistic updates, setQueryData, complex conditional logic stay raw |

## PR Strategy

All work on branch `refactor/frontend-polish-sprint`. After PR #47 merges, rebase and drop cherry-pick commits.

**PR 1 (Days 1-4)** -- `chore+refactor: repo cleanup, query keys, mutations`
- Repo root reorganization + binary removal
- Query key mismatches + inline->factory migration
- Mutation standardization
- (After rebase: cherry-pick commits dropped, `any` types + splits come from PR #47)

**PR 2 (Days 5-8)** -- `feat+refactor: file splits, webhook triggers, Topsi connection`
- useTaskFormState split + CrmDealDetailPanel split
- Webhook trigger backend + frontend
- Topsi connection status + voice gating + TopsiWidget split

**PR 3 (Days 9-10)** -- `refactor+docs: fetch migration, QA infrastructure, sprint wrap`
- Remaining fetch -> API module migration
- Final query key sweep
- QA skills + rules updates
- Full QA pass + retrospective

## Key Reference Files

- `frontend/src/lib/query-keys.ts` -- 36+ factory objects
- `frontend/src/hooks/useMutationWithToast.ts` -- target mutation pattern
- `frontend/src/lib/api/*.ts` -- typed API modules
- `frontend/src/components/dialogs/task-form/useTaskFormState.ts` -- 794-line splitting target
- `crates/db/src/models/workflow_trigger.rs` -- trigger model
- `crates/server/src/routes/workflow_triggers.rs` -- trigger CRUD routes
- `frontend/src/components/topsi/TopsiWidget.tsx` -- Topsi widget (connection status + split)

# E2E Demo Scripts — Headed Mode Refactor

**Date:** 2026-03-14
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13`
**Status:** COMMITTED — needs test run to verify

---

## Problem

Demo scripts were designed with dual headed/headless support, which caused:
- Required `E2E_HEADED=true` env var to get visible browser
- Each test re-logged-in and re-navigated even in shared-page mode
- `ensureTaskDetail`/`ensureNotificationsOpen` guard functions existed solely
  for headless fallback, adding complexity
- Identical helpers (task card lookup, status change, QA watcher) were
  duplicated across specs

## Changes

### 1. Dedicated demo fixtures (`e2e/demos/fixtures.ts`)
- Always shares single browser context + page — no headed/headless branching
- Separate from `e2e/fixtures.ts` which retains headed/headless toggle for
  regular tests (health-check, rbac, etc.)

### 2. Playwright config (`playwright.config.ts`)
- `demos` project now sets `headless: false` and `slowMo: 250` by default
- No env vars needed: `npx playwright test --project=demos` just works

### 3. Shared helpers extracted to `e2e/helpers.ts`

| Helper | Description |
|--------|-------------|
| `navigateToProjectTasks(page, id)` | goto + wait for "Create new task" |
| `navigateToTaskDetail(page, path)` | goto task detail, skip if already showing |
| `openNotifications(page)` | open bell dropdown, skip if already open |
| `createTaskViaUI(page, title, opts?)` | open dialog, fill, submit, wait for drawer |
| `changeTaskStatus(page, from, to, opts?)` | combobox click + verify toast |
| `addQaWatcher(page, opts?)` | add ORCHA QA agent from task detail |
| `findTaskCard(page, title)` | locate task card on kanban by title |
| `cleanupProject(request, id)` | API delete for afterAll |
| `cleanupTaskByPath(request, path)` | extract task ID from URL path + delete |
| `login(page)` | **updated** — skips re-auth if already on app page |

### 4. Demo specs rewritten
- `manual-qa-trigger.spec.ts` — 160→78 lines
- `bug-report-lifecycle.spec.ts` — 199→112 lines
- `notification-center.spec.ts` — 133→100 lines
- `workflow-crm-pipeline.spec.ts` — 273→222 lines (mostly unique workflow builder logic)

### 5. Other commits in this push
- **fix: BLOB binding for owner_id/user_id in project creation** — `bind_uuid_blob` helpers
- **fix: drawer click-outside ignores portaled overlays** — Radix popovers, dialogs, selects, toasts

## Verification

- [ ] `npx playwright test --project=demos` — all 25 tests pass
- [ ] Browser stays open as single window throughout entire demo suite
- [ ] No login form visible after Step 1 of each demo
- [x] All imports resolve (validated via script)
- [x] All 25 tests parse correctly (`--list`)

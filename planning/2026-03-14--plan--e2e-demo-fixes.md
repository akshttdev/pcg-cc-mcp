# E2E Demo Scripts — Fully Working Feature Showcases

**Date:** 2026-03-14
**Branch:** `e2e/dogfood-demo-replay-2026-03-14`
**Status:** Sprint 1 complete, Sprint 2 planning
**Goal:** Make all 4 demo scripts reliably showcase their features with verified outcomes at each step. **Demos must prove features work** — not just click through UI, but show agent interactions, status transitions, and visual indicators that confirm the system is functioning.

---

## Philosophy

- Demo scripts **showcase features** with clear goals per phase — not exhaustive testing
- Happy path only, but the happy path must **actually work**
- Each critical step explicitly verified — no silent failures
- Self-contained — each demo creates its own prerequisites via API in `beforeAll`
- **Test creation procedure:** Use Playwright MCP to manually walk the feature path first, discover what works, THEN write the test to document the working path
- **Agent-driven transitions are the product** — demos must show the system doing the work, not a human clicking through status dropdowns manually. Status changes should happen because agents acted, not because the test script selected a new status.

---

## Decisions

1. **LLM backend** — PCG Router should be available. Demo 4 uses real extraction. Must assert results are non-empty (fail fast if mock fallback produces nothing).
2. **Self-contained demos** — each demo creates its own prerequisites via API in `beforeAll`. No reliance on specific seed DB state.
3. **Fix combobox** — fix the `onValueChange` pre-selection bug in `WorkflowEditor.tsx` component (real UX fix, not just script workaround).
4. **Agent-driven status transitions** — Demos 1 & 2 currently have the test script manually clicking through status changes. This doesn't prove the agent watcher system works. Sprint 2 will update these demos to use real agent executions so status transitions happen organically (dev agent → In Progress, PR created → In Review, QA watcher → verdict).
5. **Visual indicators via toasts** — When agents trigger status changes, the UI should show toast notifications so the user (and demo viewer) can see what happened. This is the minimum viable "agent comms" surface for now.

---

## Test Creation Procedure

For each demo, follow this sequence:

### Step 1: Walk the feature path with Playwright MCP
Use `mcp__playwright__browser_*` tools to manually drive the browser:
- Navigate to each page
- Click each button, fill each form
- Take snapshots at critical steps to see actual UI state
- Document what works, what breaks, what selectors are correct/wrong

### Step 2: Document findings
- Correct selectors for each UI element
- Actual page routes and navigation behavior
- Any bugs or unexpected behavior
- The exact action sequence that produces the desired outcome

### Step 3: Write/fix the test script
Based on the working path from Step 1:
- Use correct selectors and routes
- Assert each critical step explicitly
- Fail fast with clear messages when prerequisites are missing

### Step 4: Run the automated test
Run the single demo spec to verify it passes end-to-end.

---

## Demo Setup/Cleanup Helpers

### New helpers to add (`e2e/helpers.ts`)

| Helper | Purpose | Used By |
|--------|---------|---------|
| `ensureAgentsSeeded(request)` | `POST /api/agents/seed` — ensures ORCHA QA agent exists | Demos 1, 2 |
| `cleanupDemoWorkflows(request)` | Deletes workflows with `TEST_DATA_PREFIX` name or `e2e_` ID | Demo 4 |
| `cleanupDemoDataSources(request, orgId)` | Deletes data sources with `TEST_DATA_PREFIX` in title | Demo 4 |

### Existing helpers to reuse

| Helper | Purpose |
|--------|---------|
| `createDemoProject(request, name?)` | Creates project in Powerclub Global org, returns ID |
| `cleanupProject(request, projectId)` | Deletes project + tasks |
| `createTaskViaUI(page, title, opts?)` | Task creation through UI modal |
| `changeTaskStatus(page, from, to)` | Status change via combobox |
| `addQaWatcher(page)` | Adds ORCHA QA watcher |
| `findTaskCard(page, title)` | Finds task card on kanban |
| `navigateToProjectTasks(page, projectId)` | Goes to kanban board |
| `t(baseMs)` | Timeout scaler for headed mode |
| `apiLogin(request)` | API authentication |
| `login(page)` | UI authentication |

### Demo `beforeAll`/`afterAll` pattern

```typescript
test.describe("Demo Name", () => {
  let projectId: string;

  test.beforeAll(async ({ request }) => {
    await apiLogin(request);
    await ensureAgentsSeeded(request);  // if agents needed
    projectId = await createDemoProject(request, "Demo Name");
  });

  test("Part 1: ...", async ({ page }) => { /* ... */ });
  test("Part 2: ...", async ({ page }) => { /* ... */ });

  test.afterAll(async ({ request }) => {
    await apiLogin(request);
    await cleanupProject(request, projectId);
  });
});
```

---

## Demo 1: Bug Report Lifecycle

**File:** `e2e/demos/bug-report-lifecycle.spec.ts`

### Sprint 1 (Complete) — Manual Flow
Basic flow working: submit bug report → find on kanban → add watcher → manually click through statuses. 8/8 tests passing. See `planning/2026-03-15--tracker--e2e-demo-sprint1.md`.

### Sprint 2 (Planned) — Agent-Driven Flow
The demo must prove the **agent watcher system actually works**, not just that a human can click status dropdowns. The bug report flow should show:

| Phase | Goal | Demonstrates |
|-------|------|-------------|
| Setup | Seed agents, ensure dev agent config exists | Self-contained prerequisites |
| Part 1 | Submit bug report via Feedback dialog | Bug reporting UX |
| Part 2 | Find bug on kanban → open detail → add QA watcher | Task creation from feedback + watcher attachment |
| Part 3 | Assign dev agent to task → agent checks out branch | **Toast: "Agent started working on branch..."** → status auto-transitions to In Progress |
| Part 4 | Dev agent completes → PR auto-created | **Toast: "PR #N created"** → status auto-transitions to In Review |
| Part 5 | QA watcher triggered → executes review | **Toast: "QA review started by ORCHA QA"** → watcher state changes to "triggered" |
| Part 6 | QA verdict received | **Toast: "QA verdict: PASS/NEEDS_CHANGES"** → watcher state updates to qa_pass/qa_needs_changes |
| Part 7 | Human approval → Done | Only this final step is manual |

### Visual Indicators Needed (Toast Notifications)

These don't exist yet — need to be added to the frontend:

| Trigger | Toast Message | Source |
|---------|--------------|--------|
| Dev agent starts execution | "Agent {name} started working — branch checked out" | WebSocket task update (status → inprogress) |
| PR auto-created | "PR #{number} created for {task title}" | WebSocket task update or SSE event |
| QA watcher triggered | "QA review started by {agent name}" | WebSocket task update (collaborator action → triggered) |
| QA verdict received | "QA verdict: {PASS/NEEDS_CHANGES/FAIL} — {summary}" | WebSocket task update (collaborator action → qa_pass/etc) |

### Backend Flow (Already Implemented)

The watcher trigger system exists in the backend:
- `tasks.rs:694` — on manual InReview, spawns `spawn_watcher_reviews()` if PR exists
- `container.rs:221` — on dev agent completion, auto-sets InReview + triggers watchers
- `qa_review.rs` — creates TaskAttempt with `run_reason = AgentReview`, posts PR comment with verdict
- Watcher states: watching → triggered → qa_pass / qa_needs_changes / qa_fail
- `broadcast_task_event()` already sends WebSocket updates on task changes

### What's Missing

1. **Frontend toast notifications** for agent-driven transitions — the WebSocket updates arrive but produce no visible indicator
2. **Demo script** that uses real agent execution instead of manual status clicks
3. **Possibly:** a lightweight agent execution mode for demos (fast/stub agent that creates a real PR and completes quickly)

### Critical Assertions (Sprint 2)

- [ ] Bug submitted via Feedback dialog, toast confirms
- [ ] Task found on kanban, watcher added
- [ ] Dev agent assigned → **toast shows agent started**
- [ ] Status auto-transitions to In Progress (not manually clicked)
- [ ] PR auto-created → **toast shows PR created**
- [ ] Status auto-transitions to In Review (not manually clicked)
- [ ] QA watcher triggered → **toast shows QA review started**
- [ ] QA verdict received → **toast shows verdict**
- [ ] Watcher collaborator state updated visibly (qa_pass/qa_needs_changes)
- [ ] Human approves → Done (only manual step)

---

## Demo 2: Manual QA Trigger

**File:** `e2e/demos/manual-qa-trigger.spec.ts`

### Sprint 1 (Complete) — Manual Flow
Basic flow working: create task → add watcher → manually click through statuses. 7/7 tests passing. Watcher stays in "Watching" state because no PR exists (expected no-PR path). See `planning/2026-03-15--tracker--e2e-demo-sprint1.md`.

### Sprint 2 (Planned) — Agent-Driven with Real QA Trigger

This demo's purpose is to show the **manual QA trigger path** — a human moves a task to In Review and the QA watcher fires. Unlike Demo 1 (fully agent-driven), this one shows the hybrid human+agent workflow. But it still needs a PR for the watcher to actually trigger.

| Phase | Goal | Demonstrates |
|-------|------|-------------|
| Setup | Create project, seed agents, create task with linked PR/branch | Self-contained prerequisites |
| Part 1 | Create task via UI, link a real or mock PR | Task creation + PR association |
| Part 2 | Add QA watcher | Agent reviewer attachment |
| Part 3 | Human changes status to In Progress | Manual status change (human-driven) |
| Part 4 | Human changes status to In Review | **Toast: "QA review started by ORCHA QA"** — watcher triggered because PR exists |
| Part 5 | Wait for QA watcher to execute and produce verdict | **Toast: "QA verdict: PASS"** — watcher state changes visibly |
| Part 6 | Human approval → Done | Final manual step |

### Key Difference from Demo 1
- Demo 1: fully agent-driven (dev agent creates branch + PR, auto-transitions status)
- Demo 2: human-driven status changes, but watcher still fires automatically on In Review

### What's Needed
1. **A way to associate a PR with the task** — either:
   - Create a real git branch + PR via API in beforeAll (preferred — proves real flow)
   - Or create a mock merge record in the DB so `spawn_watcher_reviews()` finds it
2. **Same toast notifications** as Demo 1 (watcher triggered, verdict received)
3. **Wait for watcher execution** — need to poll or listen for watcher state change from "triggered" to "qa_pass"/"qa_needs_changes"

### Critical Assertions (Sprint 2)

- [ ] Task created, PR linked
- [ ] QA watcher added (agent name + "Watching" visible)
- [ ] Human changes to In Progress — no watcher activity (correct)
- [ ] Human changes to In Review → **toast shows QA watcher triggered**
- [ ] Watcher state changes from "Watching" to "Triggered" visibly
- [ ] QA execution completes → **toast shows verdict**
- [ ] Watcher state shows final verdict (qa_pass/qa_needs_changes)
- [ ] Human approves → Done

---

## Demo 3: Notification Center

**File:** `e2e/demos/notification-center.spec.ts`

### Phase Goals

| Phase | Goal | Demonstrates |
|-------|------|-------------|
| Setup | Create project | Self-contained prerequisites |
| Part 1 | Create task to generate activity notification | Triggering notifications |
| Part 2 | View notifications via bell icon | Notification center UX |
| Part 3 | Mark all notifications as read | Bulk dismiss feature |
| Part 4 | Generate fresh notification, click to navigate | Deep-link from notification |

### Known Issues

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| Fragile CSS selector | `.max-h-80 .cursor-pointer` | Use role-based or text-based selectors |
| Weak navigation assertion | Only checks URL changed | Verify URL contains task/project path |
| No badge verification | Doesn't check blue dot appears/disappears | Assert blue dot before/after mark-all-read |
| No content verification | Doesn't check notification mentions created task | Assert notification text includes task title or action |

### Critical Assertions

- [ ] After task creation: bell icon shows unread indicator (blue dot)
- [ ] Dropdown opens with activity items
- [ ] At least one activity item relates to the created task
- [ ] "Mark all read" clears the unread indicator (blue dot gone)
- [ ] After second task: unread indicator returns
- [ ] Click notification → navigates to relevant page (URL contains task/project)

---

## Demo 4: Workflow CRM Pipeline

**File:** `e2e/demos/workflow-crm-pipeline.spec.ts`

This is the most complex demo — it walks through multiple pages and features.

### Phase Goals

| Phase | Goal | Page | Demonstrates |
|-------|------|------|-------------|
| Setup | Clean up old test data | API | Self-contained prerequisites |
| Part 1 | Build 7-node CRM extraction workflow | Workflows → Builder | Visual workflow editor |
| Part 2 | Create conversation data source | Org Intelligence → Data Sources | Data source management |
| Part 3 | Run workflow against data source | Data Sources page → Run dialog | Workflow execution |
| Part 4 | Review run results | Workflows → Runs tab | Run metrics & status |
| Part 5 | Verify & approve staged CRM data | Workflows → Staging tab | Record review & commit |

### Page Walkthroughs

#### Workflows Page — Builder Tab (Part 1)

**Route:** `/workflows`
**Key elements:**
- "New Workflow" button → opens WorkflowEditor modal
- Editor: Workflow ID field, Name field, Description field
- "Add Node" button → node type picker (Data Source, LLM Extract, Output: *)
- Node configuration: Name input, prompt textarea, input connection combobox
- "Save Workflow" button → closes modal, workflow appears in builder list
- Workflow cards show node count, description, action buttons (run, edit, delete)

**Demo flow:**
1. Click "New Workflow"
2. Fill ID, name, description
3. Add Data Source node
4. Add 3× LLM Extract nodes (contacts, companies, deals) — each connected to Data Source
5. Add 3× Output nodes — each connected to its extract node
6. Click "Save Workflow"
7. Verify workflow in list

#### Org Intelligence — Data Sources (Part 2)

**Route:** `/organizations/:orgId/intelligence/data-sources`
**Context:** Part of Organization Profile page, Intelligence Tab, Data Sources sub-view
**Key elements:**
- "Add Data Source" button → AddDataSourceDialog
- Dialog fields: Title (required), Content textarea, Data Type dropdown
- Source appears in table with: Name, Type, Status, Added date, Actions

**Demo flow:**
1. Navigate to intelligence data sources page
2. Click "Add Data Source"
3. Fill title and conversation transcript content
4. Click save/add button
5. Verify source appears in table

#### Data Sources Page (Part 3)

**Route:** `/organizations/:orgId/data-sources` or `/data-sources`
**Context:** Standalone page with folder tree, list/grid view, preview panel
**Key elements:**
- File/source list (table or grid view)
- Click source → preview panel opens on right
- Preview panel: metadata, content preview, "Run Workflow" button
- "Run Workflow" → RunWorkflowFromSourceDialog
- Dialog: workflow dropdown, run button
- On success: redirects to `/workflows?tab=staging&run=<runId>`

**Demo flow:**
1. Navigate to data sources page
2. Click on the created conversation source
3. Click "Run Workflow" in preview panel
4. Select the CRM workflow from dropdown
5. Click "Run Workflow" button
6. Wait for "Running..." state
7. Wait for redirect to staging tab

#### Workflows Page — Runs Tab (Part 4) — NEW

**Route:** `/workflows` → Runs tab
**Key elements:**
- List of workflow executions
- Per run: status badge (running/completed/failed), workflow name, duration, cost, records staged
- Running status: blue animated badge, auto-refreshes every 10s
- Completed status: green badge with metrics
- "Review Staging" button → navigates to staging tab filtered by run

**Demo flow:**
1. Navigate to Workflows page, click Runs tab
2. Find the CRM extraction run (most recent, or by workflow name)
3. Verify status is "completed" (green)
4. Verify records_staged > 0
5. Verify duration and cost are shown
6. Click "Review Staging" to navigate to staging

#### Workflows Page — Staging Tab (Part 5)

**Route:** `/workflows?tab=staging&run=<runId>`
**Key elements:**
- Records grouped by target type: crm_contact (blue), company (purple), crm_deal (green)
- Per record: confidence score, status badge, extracted data fields
- Filter: all / valid / duplicates / validation_issues
- Record actions: Approve & Commit, Reject, inline edit
- Batch actions: "Approve & Commit All", "Reject Duplicates"
- Status badges: pending_review, approved, rejected, committed, error

**Demo flow:**
1. Verify staging tab loaded with records (NOT "No staged records")
2. Verify records contain expected CRM data from transcript:
   - Contact names: Marcus Webb, Lisa Park, Raj Patel, Sarah Chen
   - Companies: Acme Corp, GlobalTech Industries
   - Deal values: $450,000, $200,000
3. Verify record type categories present (contacts, companies, deals)
4. Approve at least one record (click Approve & Commit)
5. Verify status changes to "committed"

### Known Issues

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| Combobox pre-selection bug | First option pre-selected, clicking is no-op | Fix `onValueChange` in WorkflowEditor.tsx |
| Missing Runs tab phase | Demo skips from run to staging | Add Part 4 for Runs tab walkthrough |
| Route uncertainty | Part 2/3 may use wrong routes | Verify with Playwright MCP |
| LLM dependency | Needs PCG Router for real extraction | Assert non-empty results, fail fast if empty |
| `Promise.any()` assertions | Unhelpful error messages | Replace with sequential checks + clear messages |
| No staging actions | Demo only views records, doesn't approve | Add approve/commit step to demonstrate full flow |

### Critical Assertions

- [ ] Part 1: Workflow saved, name visible in builder list
- [ ] Part 1: Each node connection confirmed ("from: X" text)
- [ ] Part 2: Data source created, title visible in table
- [ ] Part 3: Run starts ("Running..."), completes (redirects to staging)
- [ ] Part 4: Run appears in Runs tab with "completed" status, records > 0
- [ ] Part 5: Staged records contain CRM data (contacts, companies, deals)
- [ ] Part 5: At least one record approved/committed successfully

---

## Frontend Fix: Combobox Pre-Selection Bug

**File:** `frontend/src/components/workflows/WorkflowEditor.tsx`
**Problem:** Connection combobox pre-marks first option as `[active][selected]`. Clicking a pre-selected option doesn't trigger `onValueChange`, so the connection silently fails.
**Fix:** Either clear the combobox's initial selection, or use `onSelect` instead of `onValueChange` to fire even on re-selection of the same value.

---

## Files to Modify

### Sprint 1 (Complete)
| File | Changes | Status |
|------|---------|--------|
| `frontend/src/components/layout/sidebar.tsx` | Added `data-testid` to feedback button | Done |
| `frontend/src/lib/api.ts` | Fixed `listActive()` and `search()` to unwrap `body.data` | Done |
| `e2e/helpers.ts` | Added `ensureAgentsSeeded()`, `openFeedbackDialog()` | Done |
| `e2e/demos/bug-report-lifecycle.spec.ts` | Self-contained setup, correct selectors, stronger assertions | Done |
| `e2e/demos/manual-qa-trigger.spec.ts` | Self-contained setup, watcher state verification | Done |

### Sprint 2 (Planned) — Agent-Driven Demos + Visual Indicators
| File | Changes |
|------|---------|
| `frontend/src/components/tasks/TaskDetails/EnhancedTaskHeader.tsx` or similar | Add toast notifications for agent-driven status transitions |
| `frontend/src/hooks/` or `frontend/src/lib/` | WebSocket listener that detects agent-driven task changes and shows toasts |
| `e2e/helpers.ts` | Add helpers: `assignDevAgent()`, `waitForAgentExecution()`, `waitForWatcherVerdict()`, `linkPrToTask()` |
| `e2e/demos/bug-report-lifecycle.spec.ts` | Rewrite to use real agent execution instead of manual status clicks |
| `e2e/demos/manual-qa-trigger.spec.ts` | Add PR linkage so watcher actually triggers; assert watcher state changes |

### Sprint 3 (Planned) — Remaining Demos
| File | Changes |
|------|---------|
| `e2e/demos/notification-center.spec.ts` | Better selectors, badge & navigation verification |
| `e2e/demos/workflow-crm-pipeline.spec.ts` | Route fixes, add Runs tab, expand staging, fix assertions |
| `frontend/src/components/workflows/WorkflowEditor.tsx` | Fix combobox pre-selection bug |
| `e2e/helpers.ts` | Add `cleanupDemoWorkflows()`, `cleanupDemoDataSources()` |

---

## Implementation Order

### Sprint 1 — Manual Flow Fix (Complete)
1. ~~Add `ensureAgentsSeeded()`, `openFeedbackDialog()` helpers~~ ✓
2. ~~Fix `agentsApi.listActive()` / `search()` crash~~ ✓
3. ~~Demo 1 — MCP walkthrough → fix script → 8/8 passing~~ ✓
4. ~~Demo 2 — MCP walkthrough → fix script → 7/7 passing~~ ✓
5. PR for Sprint 1

### Sprint 2 — Agent-Driven Demos + Visual Indicators
**Detailed dev plan:** `planning/2026-03-15--plan--e2e-demo-sprint2-dev-work.md`

**Phase 0:** Configurable demo repo (sandbox repo + env vars — don't use production codebase)
**Phase 1:** Stub demo agent (script-based, no LLM, creates real commits + PRs in seconds)
**Phase 2:** Toast notifications (backend SSE metadata + frontend toasts for agent transitions)
**Phase 3:** Demo script rewrites (real agent flow + toast assertions)

### Sprint 3 — Remaining Demos
5. Demo 3 (notifications) — MCP walkthrough → fix
6. Demo 4 (workflow CRM) — MCP walkthrough → fix combobox → fix script
7. Full suite — all 4 pass

## Verification

1. **Per-demo Playwright MCP replay** — manually walk through each demo
2. **Automated run** — `npx playwright test --project=demos`
3. **Headed verification** — `E2E_HEADED=true npx playwright test --project=demos`
4. **Agent interaction proof** — Sprint 2 demos must show toast notifications for agent-driven transitions (not just manual clicks)

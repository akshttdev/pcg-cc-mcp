# Sprint 2: Agent-Driven Demos — Development Work

**Date:** 2026-03-15
**Branch:** `e2e/dogfood-demo-replay-2026-03-14` (continuing from Sprint 1)
**Depends on:** Sprint 1 complete (15/15 tests passing)
**Goal:** Demos prove the agent system works — real dev agent commits, real PRs, real QA watcher verdicts, all with visible user-facing indicators.
**Status:** Complete ✅ — 23/23 tests passing (Demos 1–3). Demo 4 deferred (workflow staging issue).

---

## Problem Statement

Sprint 1 demos click through status changes manually. This proves the UI works but doesn't prove:
- Dev agents can check out a branch and commit code
- PRs are auto-created on agent completion
- QA watchers actually trigger and produce verdicts
- The user can SEE any of this happening

---

## Phase 0: Infrastructure — Configurable Demo Repo

### Why
The agent execution system creates git worktrees, makes commits, and pushes PRs to GitHub. We can't use the production ORCHA codebase for E2E demo runs — we need a small, disposable test repo.

### Design
- **E2E config** (env vars or `e2e/demo-config.ts`):
  ```
  E2E_DEMO_REPO_OWNER=<github-org>       # e.g. "powerclub-global"
  E2E_DEMO_REPO_NAME=<repo-name>         # e.g. "e2e-demo-sandbox"
  E2E_DEMO_REPO_DEFAULT_BRANCH=main      # base branch for PRs
  E2E_DEMO_REPO_LOCAL_PATH=<path>        # local clone path for worktrees
  ```
- **Demo sandbox repo** — a minimal GitHub repo with:
  - A `README.md` and basic file structure
  - Enough content for a stub agent to make a meaningful commit
  - Branch protection OFF (so PRs can be created freely)
  - Auto-delete head branches ON (cleanup)

### Decision
The project model's configured repo determines where worktrees are created. So we create the demo project pointing at the sandbox repo — no server-level config change needed.

### Work Items (Complete)
- [x] Sandbox repo configured via `e2e/helpers/demo/config.ts` (owner/name from env vars)
- [x] `createDemoProject()` works with sandbox repo
- [x] Branch/PR creation uses GitHub API directly (no local clone needed)

---

## Phase 1: API-Driven Agent Simulation (No Production Code Changes)

### Why
Real agent executions take minutes and require API keys. But injecting a stub executor into the production `BaseCodingAgent` enum pollutes the core codebase with test concerns. Instead, the E2E test itself acts as the "agent" by calling the same APIs that a real agent's completion would trigger.

### Design: E2E Test as Agent

The E2E test simulates the agent pipeline from the API boundary:

1. **Create a sandbox repo** on GitHub (small, disposable)
2. **Create a project** pointing at that repo
3. **Create a task** with the ORCHA Dev agent assigned
4. **Simulate dev agent work** via API:
   - Clone the sandbox repo locally (or use `gh` CLI)
   - Create a branch, make a commit, push
   - Create a PR via `gh pr create`
   - Create a merge record via API (`POST /api/merges` or direct DB)
   - Update task status to `inreview` (triggers `spawn_watcher_reviews`)
5. **Simulate QA watcher verdict** via API:
   - Create an execution artifact with verdict JSON
   - Update the watcher's collaborator action to `qa_pass`
   - (Or if `spawn_watcher_reviews` is triggered, let it run with a real executor)

### Advantages
- **Zero production code changes** — no stub executor, no match arms, no demo artifacts
- **Tests the real pipeline** — API calls prove the same endpoints work
- **Self-contained** — all simulation logic lives in `e2e/helpers.ts`
- **Realistic** — creates real commits, real PRs, real artifacts

### Work Items (Complete)
- [x] Sandbox repo exists on GitHub
- [x] `simulateDevAgentWork(request, taskId)` — creates branch, commit, PR, links to task, sets status to inreview
- [x] `simulateQaVerdict(request, taskId, agentId, verdict)` — PATCHes collaborator action
- [x] `createPrForTask(request, taskId)` — creates branch + PR, links to task attempt
- [x] `cleanupDemoBranches(request, branches)` / `cleanupDemoPr(request, prNumber)` — cleanup helpers
- [x] Toast notifications verified: status changes, watcher triggers, QA verdicts

### Key Files
| File | Change |
|------|--------|
| `e2e/helpers.ts` | Add API-driven agent simulation helpers |
| `e2e/demo-config.ts` | Sandbox repo config (owner, name, default branch) |

---

## Phase 2: Toast Notifications for Agent-Driven Transitions

### Why
When agents change task state (status, watcher triggered, verdict received), nothing visible happens in the UI. The SSE patch stream delivers the updated `TaskWithAttemptStatus` object, but the frontend just silently updates the kanban card. Users (and demo viewers) have no idea anything happened.

### Design

**Where to hook:** The SSE patch stream delivers JSON patches to the frontend. The patches include the full `TaskWithAttemptStatus` value with `status`, `collaborators` (watcher states), and attempt info. We need a diff layer that detects meaningful changes and fires toasts.

**What to detect:**

| Change | Detection | Toast |
|--------|-----------|-------|
| Status → `inprogress` (by agent) | `op: "replace"`, old status ≠ `inprogress`, new status = `inprogress`, task has `agent_id` | "Agent {name} started working" |
| Status → `inreview` (by agent) | Same pattern, new status = `inreview`, task has `agent_id` | "Task moved to In Review" |
| PR created | Task's attempt gains a merge record (or detect via separate SSE event) | "PR #{number} created" |
| Watcher triggered | Collaborator `last_action` changed from `watching` to `triggered` | "QA review started by {agent_name}" |
| Watcher verdict | Collaborator `last_action` changed to `qa_pass`/`qa_needs_changes`/`qa_fail` | "QA verdict: {PASS/NEEDS_CHANGES/FAIL}" |

**Architecture Options:**

**Option A: Frontend-only (diff in patch consumer)**
- Wrap the existing `useJsonPatchStream` or the page-level patch consumer
- Before applying a patch, compare old vs new task state
- If a meaningful change is detected, fire a toast via sonner/shadcn toast
- Pros: no backend changes
- Cons: needs to track previous state per task, can miss events if page isn't open

**Option B: Backend SSE event with metadata**
- Extend `broadcast_task_event` to include a `metadata` field describing what changed
- e.g. `{ "agent_transition": "status_changed", "from": "todo", "to": "inprogress", "by": "ORCHA Dev" }`
- Frontend reads metadata and fires toast
- Pros: cleaner, authoritative, works even if multiple patches arrive
- Cons: backend changes

**Decision:** Frontend-only diff approach. The WebSocket already sends full `TaskWithAttemptStatus` objects. A hook diffs old vs new state and fires toasts when agent-driven changes are detected. No backend changes needed.

### Implementation (Complete)

**Approach changed:** Frontend-only diff instead of backend metadata. The WebSocket already sends full `TaskWithAttemptStatus` objects with status and collaborators. A new hook (`useTaskChangeNotifications`) diffs old vs new state and fires sonner toasts.

**Backend fix also required:** `spawn_watcher_reviews` runs in `tokio::spawn` — added `broadcast_task_event` after it completes so frontend receives watcher state changes via WebSocket.

### Key Files
| File | Change |
|------|--------|
| `frontend/src/hooks/useTaskChangeNotifications.ts` | New hook: diffs task state, fires sonner toasts for agent transitions |
| `frontend/src/pages/project-tasks.tsx` | Wires up `useTaskChangeNotifications(tasksById)` |

---

## Phase 3: E2E Demo Script Updates

Only after Phases 0–2 are working.

### Demo 1: Bug Report Lifecycle (Agent-Driven)

```
beforeAll: seed agents, configure sandbox repo
Step 1:  Submit bug report via Feedback dialog
Step 2:  Find bug on kanban → open detail → add QA watcher
Step 3:  Assign dev agent (stub) to task → POST /api/task-attempts
Step 4:  Wait for toast: "Agent started working" + status = In Progress
Step 5:  Wait for toast: "PR #N created" + status = In Review
Step 6:  Wait for toast: "QA review started by ORCHA QA"
Step 7:  Wait for toast: "QA verdict: PASS" + watcher state = qa_pass
Step 8:  Human approval → change to Done (only manual step)
afterAll: cleanup project, cleanup PR/branch
```

### Demo 2: Manual QA Trigger (Hybrid Human+Agent)

```
beforeAll: seed agents, create project, create task, link stub PR
Step 1:  Open task detail → add QA watcher
Step 2:  Human changes status to In Progress (manual click — no toast expected)
Step 3:  Human changes status to In Review (manual click)
Step 4:  Wait for toast: "QA review started by ORCHA QA" (watcher triggered because PR exists)
Step 5:  Wait for toast: "QA verdict: PASS" + watcher state = qa_pass
Step 6:  Human approval → Done
afterAll: cleanup project, cleanup PR/branch
```

### New E2E Helpers Needed

| Helper | Purpose |
|--------|---------|
| `assignDevAgent(page, agentName)` | Assign dev agent to task from task detail UI |
| `startAgentExecution(request, taskId, profileId)` | `POST /api/task-attempts` with stub profile |
| `waitForToast(page, pattern, timeout)` | Wait for a toast notification matching a regex |
| `waitForWatcherState(page, state)` | Wait for watcher collaborator to show specific state |
| `linkPrToTask(request, taskId, prUrl)` | Create a merge record linking a PR to a task attempt |
| `cleanupGitBranch(request, owner, repo, branch)` | Delete remote branch after demo |
| `cleanupPr(request, owner, repo, prNumber)` | Close PR after demo |

---

## Implementation Order (All Complete)

1. ~~**Phase 0** — Sandbox repo setup (config in `e2e/helpers/demo/config.ts`)~~ ✅
2. ~~**Phase 1** — API-driven agent simulation helpers (`e2e/helpers/demo/simulation.ts`)~~ ✅
3. ~~**Phase 2** — Toast notifications (`useTaskChangeNotifications` hook + backend broadcast fix)~~ ✅
4. ~~**Phase 3** — Demo script rewrites (Demos 1–3 with toast assertions)~~ ✅

---

## Verification (All Complete)

1. ~~**Phase 1 verify:** API simulation creates real branches, PRs, links to tasks~~ ✅
2. ~~**Phase 2 verify:** Toast notifications appear for status changes, watcher triggers, QA verdicts~~ ✅
3. ~~**Phase 3 verify:** `npx playwright test --project=demos` — 23/23 passing (Demos 1–3)~~ ✅

## Remaining Work

- **Demo 4 (Workflow CRM Pipeline):** Parts 1–2 pass, Part 3 fails — workflow execution produces 0 staged records despite LLM tokens being consumed. Root cause is in workflow staging validation logic, not the test. Deferred to future sprint.

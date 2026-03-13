# Phase 6: Agent Watcher API + UI + Prompt Injection

**Branch:** `feature/qa-watcher-phase6`
**PR:** #26
**Status:** Complete
**Depends on:** Phases 1-5 (PRs #23 + #24, merged to main)

## Context

Phases 1-5 of the QA Agent Watcher refactor are complete. The watcher system works end-to-end: agents auto-register as watchers, get triggered on `InReview`, run with `AgentReview` run reason, and post PR comments with structured verdicts.

**What Phase 6 adds:**
1. API for manually adding/removing agent watchers on a task
2. Frontend UI (AgentWatcherPanel) for managing watchers with status badges
3. QA review prompt injection — agents now receive the structured review prompt directly in their execution prompt
4. CollaborationTimeline enhancement to distinguish agent_watcher actor type
5. E2E tests for the new agent watcher API and UI

## Implementation Steps

### Step 1: Backend API Endpoints — `crates/server/src/routes/tasks.rs`
- `POST /tasks/{task_id}/agent-watchers` — validates agent exists, adds as watcher
- `DELETE /tasks/{task_id}/agent-watchers/{agent_id}` — removes agent watcher
- `GET /tasks/{task_id}/agent-watchers` — lists watchers enriched with agent name/designation
- Types: `AddAgentWatcherRequest`, `AgentWatcherPath`, `AgentWatcherInfo`
- All routes use `load_task_middleware` layer for task loading + access control

### Step 2: Middleware fix — `crates/server/src/middleware/model_loaders.rs`
- Changed `load_task_middleware` from `Path<Uuid>` to `Path<TaskIdPath>` (struct-based extractor)
- Required because the DELETE route has two path params (`{task_id}` and `{agent_id}`); single-value `Path<Uuid>` fails with multiple params in Axum 0.8

### Step 3: QA Review Prompt Injection — `container.rs` + `qa_review.rs`
- Added `prompt_suffix: Option<String>` parameter to `start_attempt_with_reason()`
- When provided, suffix is appended to the task prompt before creating `CodingAgentInitialRequest`
- `qa_review.rs` passes `review_description` as the prompt suffix
- Artifact still stored for audit trail, AND agent receives instructions in its prompt

### Step 4: Frontend API Client — `frontend/src/lib/api.ts`
- `AgentWatcherInfo` interface
- `agentWatchersApi` with `list`, `add`, `remove` methods
- Response unwrapping: `result.data` to match `ApiResponse` wrapper

### Step 5: AgentWatcherPanel Component — `frontend/src/components/tasks/AgentWatcherPanel.tsx`
- Shows current agent watchers with color-coded status badges
- Status badges: watching (blue), triggered (yellow), qa_pass (green), qa_needs_changes (orange), qa_fail (red)
- "Add" button opens popover with searchable agent picker (filters out already-added)
- Remove button (x) on hover for each watcher
- Toast notifications on add/remove failures

### Step 6: Wire into Task Detail Views
- `TaskDetailsPanel.tsx` — between ApprovalPanel and ActivityTimeline
- `EnhancedTaskDetailsPanel.tsx` — before Recent Activity section

### Step 7: CollaborationTimeline Enhancement — `CollaborationTimeline.tsx`
- Distinguishes `agent_watcher` actor type from `agent` and `human`
- Shows separate purple Eye badge for agent watcher count in header

### Step 8: E2E Tests — `e2e/workflow-pipeline.spec.ts`
- **10C-2: Agent Watcher API** — tests manual add/remove/list via API
  - Create task without agent → verify empty watcher list
  - Add QA agent as watcher → verify list returns 1 watcher with "watching" status
  - Add DEV agent → verify list returns 2
  - Remove QA → verify only DEV remains
  - Invalid agent_id returns 400+ error
- **10C-3: Agent Watcher UI** — browser test for AgentWatcherPanel
  - Create task with DEV agent (auto-registers QA watcher)
  - Open task detail panel → verify "Agent Reviewers" heading visible
  - Verify "Watching" badge visible for auto-registered watcher
  - Verify "Add" button present

**E2E impact analysis:** Existing tests in 10C (lines 175-210) and 10D (lines 243-265) test auto-registration via `collaborators` JSON parsing — these are unaffected since we didn't change the auto-registration flow. The `health-check.spec.ts` task detail test (line 174-186) only checks Overview/Artifacts tabs — unaffected.

### Step 9: RBAC Hardening — `access_control.rs` + `model_loaders.rs` + `tasks.rs`

**Problem:** Task routes behind `load_task_middleware` had no project-level authorization — any authenticated user could access/modify any task if they knew the task ID.

**Solution (two layers):**
1. **Middleware (Viewer gate):** `load_task_middleware` now calls `require_viewer()` on the task's parent project. All routes (GET + mutations) require at least Viewer access.
2. **Handler (Editor gate):** All mutation handlers explicitly call `require_editor()`:
   - `update_task`, `delete_task`
   - `approve_task`, `request_changes`, `reject_task`
   - `add_agent_watcher`, `remove_agent_watcher`
3. **Helper methods on `AccessContext`:** Added `require_viewer()`, `require_editor()`, `require_project_admin()` — one-line convenience wrappers around `check_project_access_hierarchical`. Also refactored `load_project_middleware` to use these.

**Admin bypass:** `check_project_access_hierarchical` returns `Owner` for `is_admin` users automatically — admins never need to be added to projects.

**Role hierarchy:**
| Role | can_read | can_write | can_manage_members | can_delete |
|------|----------|-----------|-------------------|------------|
| Owner | yes | yes | yes | yes |
| Admin | yes | yes | yes | no |
| Editor | yes | yes | no | no |
| Viewer | yes | no | no | no |

### Step 10: Follow-up Review Polish — `tasks.rs` + `AgentWatcherPanel.tsx`

1. **`AgentWatcherPath.task_id` doc comment:** Explains `#[allow(dead_code)]` — field is required by route pattern but consumed by `load_task_middleware`, not the handler itself.
2. **Optimistic remove UI:** Watcher row fades to 40% opacity with `pointer-events-none` while DELETE is in-flight, giving immediate visual feedback.
3. **Agent search scaling note:** Comment documenting that client-side filtering is fine for typical agent counts (<50), with note to consider server-side search if the list grows.

## QA Findings & Fixes

| # | Severity | Issue | Resolution |
|---|----------|-------|------------|
| 1 | P0 | `load_task_middleware` `Path<Uuid>` fails with 2 path params | Fixed: struct-based `Path<TaskIdPath>` extractor |
| 2 | P1 | `remove_agent_watcher` handler used tuple Path extraction | Fixed: struct-based `AgentWatcherPath` |
| 3 | P1 | Silent error swallowing in AgentWatcherPanel mutations | Fixed: added sonner toast notifications |
| 4 | P1 | No project-level authorization on task routes | Fixed: Viewer check in middleware + Editor check in mutation handlers (Step 9) |
| 5 | P2 | `AgentWatcherPath.task_id` unexplained `dead_code` allow | Fixed: added doc comment (Step 10) |
| 6 | P2 | No optimistic UI on watcher remove | Fixed: opacity fade + pointer-events-none (Step 10) |
| 7 | P2 | Client-side agent search scaling | Documented: comment noting threshold and future consideration (Step 10) |
| 8 | P1 (pre-existing) | `CollaborationTimeline` missing `chatMessages` dep in useMemo | Not fixed: pre-existing, out of scope |

## Verification

- `flox activate -- cargo check --workspace` — compiles clean
- `cd frontend && npx tsc --noEmit` — no TS errors
- E2E: `npx playwright test e2e/workflow-pipeline.spec.ts` — all tests pass (requires running dev server)

## Cross-PR Conflict Check

- **PR #25** (Topsi Phase 5): Only overlapping file is `TaskDetailsPanel.tsx`. PR #25 adds `AskTopsiButton` in the header toolbar area; PR #26 adds `AgentWatcherPanel` in the sidebar. Different hunks — clean auto-merge in either order. Verified via `git merge-tree`.

# Phase 6: Agent Watcher API + UI + Prompt Injection

**Branch:** `feature/qa-watcher-phase6`
**Status:** Complete
**Depends on:** Phases 1-5 (PRs #23 + #24, merged to main)

## Context

Phases 1-5 of the QA Agent Watcher refactor are complete. The watcher system works end-to-end: agents auto-register as watchers, get triggered on `InReview`, run with `AgentReview` run reason, and post PR comments with structured verdicts.

**What Phase 6 adds:**
1. API for manually adding/removing agent watchers on a task
2. Frontend UI (AgentWatcherPanel) for managing watchers with status badges
3. QA review prompt injection — agents now receive the structured review prompt directly in their execution prompt
4. CollaborationTimeline enhancement to distinguish agent_watcher actor type

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

## QA Findings & Fixes

| # | Severity | Issue | Resolution |
|---|----------|-------|------------|
| 1 | P0 | `load_task_middleware` `Path<Uuid>` fails with 2 path params | Fixed: struct-based `Path<TaskIdPath>` extractor |
| 2 | P1 | `remove_agent_watcher` handler used tuple Path extraction | Fixed: struct-based `AgentWatcherPath` |
| 3 | P1 | Silent error swallowing in AgentWatcherPanel mutations | Fixed: added sonner toast notifications |
| 4 | P1 (pre-existing) | No explicit `check_project_access` on watcher routes | Not fixed: consistent with existing watch/unwatch pattern |
| 5 | P1 (pre-existing) | `CollaborationTimeline` missing `chatMessages` dep in useMemo | Not fixed: pre-existing, out of scope |

## Verification

- `flox activate -- cargo check --workspace` — compiles clean
- `cd frontend && npx tsc --noEmit` — no TS errors

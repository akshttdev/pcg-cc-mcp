# Dogfood Pipeline E2E QA — Browser-Only Testing

**Date:** 2026-03-14 (session started 2026-03-13 ~23:50 UTC)
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13`
**Method:** Playwright MCP (accessibility snapshots, no API calls — UI-only interaction)
**Environment:** Fresh seed DB, backend port 3003, frontend port 3000
**Login:** admin/admin123

---

## Executive Summary

The dogfooding pipeline works end-to-end through the UI: feedback submission creates tasks on the correct board, triggers the Bug Triage Pipeline workflow, and produces completed runs in <400ms. However, the **BLOB UUID mismatch** between the seed database (TEXT UUIDs) and the Rust `Agent` model (expects 16-byte BLOB UUIDs) is a **systemic P0 blocker** that breaks every agent-related feature across the entire application. The **full QA review pipeline (InReview → QA watcher trigger) is untestable** because feedback tasks get no agent assigned, the agent picker is broken, and watcher APIs all 500. Additionally, the **task activity logging is broken** (FK constraint failure), and several UX gaps were identified including missing loading skeletons, non-scrollable edit modals, and no direct status change from task detail panels. **15 bugs total** (2 P0, 5 P1, 8 P2/UX).

---

## Test Matrix

| Area | Status | Notes |
|------|--------|-------|
| Feedback → Task creation (bug report) | PASS | Task lands on Bugs board with `[Bug]` prefix, tags, priority |
| Feedback → Task creation (feature request) | PASS | Task gets `[Feature Request]` prefix |
| Feedback → DataSource creation | PASS | Visible in Data Sources tab, type "report" |
| Bug Triage Pipeline auto-trigger | PASS | Trigger enabled, filters match, runs complete |
| Workflow runs history | PASS | 3 completed runs visible in Runs tab (310ms-376ms) |
| Workflow Builder | PASS | 5 system workflows visible |
| Task creation via dialog | PASS | Title, description, board, priority, tags all work |
| Task detail panel | PASS | Overview, Artifacts, Workflow, Logs, Vibe tabs render |
| Kanban board rendering | PASS | 5 columns (Todo/InProgress/InReview/Done/Cancelled) |
| My Tasks page | PASS | Correct counts (assigned/created/watching), sorting works |
| Settings > General | PASS | All sections render (Appearance, Execution, GitHub, etc.) |
| Settings > Admin nav | PASS | All admin sections accessible |
| Agent picker in task creation | **FAIL** | Perpetual "Loading...", `/api/agents/active` 500 |
| Settings > Agents | **FAIL** | Empty list, `/api/agents/search` 500 |
| Agent Watcher Panel | **FAIL** | "No agent reviewers" (correct text), but 500 errors on API |
| Intelligence Overview | **FAIL** | "Total Sources: 0" despite data sources existing |
| Task activity logging | **FAIL** | DatabaseError on `/{task_id}/activity` |
| Manual status → In Review | PASS (partial) | Status changes via Edit dialog, kanban updates. But no QA watcher trigger (no agents assigned) |
| QA watcher auto-trigger on InReview | **FAIL** | No watchers registered on feedback tasks — `agent_id: None` in feedback.rs |
| Dev agent auto-assignment on feedback tasks | **FAIL** | Feedback endpoint sets `agent_id: None`, task scheduler requires non-null agent |
| Status change from task detail panel | **FAIL** | No direct status change — must open Edit Task dialog |
| My Tasks → Task Detail transition | UX issue | Loading state with no skeleton — blank content for 3-6s |
| Edit Task modal scrolling | UX issue | Long form requires scrolling, Cancel/Update buttons not always visible |

---

## Bug Report

### P0 — Blockers

| # | Bug | Impact | Root Cause | Repro |
|---|-----|--------|------------|-------|
| 1 | `/api/agents` returns 500 | All agent-related features broken | Agent model expects BLOB UUID (16 bytes) but seed DB stores TEXT UUID (36-char string). `sqlx::FromRow` decode fails. | Navigate to Settings > Agents → empty list, console shows 500 |
| 7 | Settings > Agents page completely empty | Can't view, search, or manage any agents | Same as #1 — `/api/agents/search` 500 | Settings → Admin → Agents → no agents shown |

### P1 — Major

| # | Bug | Impact | Root Cause | Repro |
|---|-----|--------|------------|-------|
| 2 | `/api/tasks/{id}/agent-watchers` returns 500 | Agent watcher panel broken for all tasks | Same BLOB/TEXT mismatch in `Agent::find_all()` | Open any task detail → console shows 500 on agent-watchers endpoint |
| 6 | Intelligence Overview shows "Total Sources: 0" | Misleading stats — sources exist but aren't counted | Different query path than Data Sources table (likely BLOB UUID join issue or missing org-scoped aggregation) | Go to Powerclub Global org → Intelligence tab → Overview → "Total Sources: 0" |
| 8 | Agent picker in task creation dialog shows perpetual "Loading..." | Can't assign agents to tasks via UI | `/api/agents/active` returns 500 (BLOB UUID) | Create New Task → click "Assigned Agent" dropdown → "Loading..." forever |
| 9 | Task activity logging fails on creation AND update | No activity trail for any task events | FOREIGN KEY constraint failed (code: 787) on POST to `/{task_id}/activity` | Create or update task → check console → `[API Error] DatabaseError: FOREIGN KEY constraint failed` |
| 10 | Feedback tasks never get dev agent assigned | Bug reports sit idle — no agent starts working | `feedback.rs` sets `agent_id: None`, task scheduler requires `assigned_agent IS NOT NULL` | Submit bug report → task stays in To Do indefinitely, no agent execution |
| 11 | No QA watcher trigger on manual InReview transition | QA review pipeline untestable without agents | No watchers registered because no agents can be assigned (BLOB UUID blocks agent picker) | Change task to InReview via Edit → no watcher activity |

### P2 — Minor / UX

| # | Bug | Impact | Root Cause | Repro |
|---|-----|--------|------------|-------|
| 3 | AgentWatcherPanel shows "No agent reviewers" instead of error state | Users don't know API is failing — silent failure | 500 error caught but no toast shown (despite toast being added in Phase 6 — race condition or wrong error path?) | Open task detail with agent watchers expected |
| 4 | No success toast after bug report feedback submission | User gets no confirmation that feedback was submitted | Bug report type closes dialog silently; Feature Request type shows "Thank You!" modal — inconsistent behavior | Submit bug report via Feedback dialog → dialog just closes |
| 5 | Project task count doesn't auto-refresh after feedback creates task | Stale sidebar/header counts until page refresh | No invalidation of project task count cache when feedback endpoint creates a task | Submit feedback → check project page → count still shows old number |
| 12 | Feature requests processed by "Bug Triage Pipeline" | Confusing naming — pipeline triages all feedback types, not just bugs | Pipeline trigger matches all `data_source_type = "report"` regardless of feedback_type | Submit feature request → workflow runs tab shows "Bug Triage Pipeline" processed it |
| 13 | No direct status change from task detail panel | Must open Edit Task dialog just to change status — friction for common action | Status badge in task header is display-only, context menu has Edit/Duplicate/Delete but no status | Open any task → try to click status badge → nothing happens |
| 14 | Edit Task modal not scrollable with sticky footer | Cancel/Update buttons scroll off-screen on long forms — hard to find CTA | Modal body has no max-height/scroll, footer not sticky | Open Edit Task → scroll down past description → buttons may be off-screen |
| 15 | My Tasks → Task Detail shows blank loading state | 3-6s of "Loading..." text with no skeleton UI — feels broken | No loading skeletons implemented for task detail data fetching | Click task from My Tasks → observe blank content area while data loads |
| 16 | Expanded task detail panel has no obvious back/close navigation | User loses kanban context, unclear how to return | Full-page task detail (`/full` route) replaces the page — no drawer overlay, no breadcrumb back link | Click task → view expands to full page → no clear way back except browser back |

### Observations (Not Bugs)

| # | Finding | Notes |
|---|---------|-------|
| O1 | Kanban shows all tasks regardless of board filter | Board dropdown exists in toolbar but may not filter by default — all 12 tasks show in single view. Need to verify if this is intentional (project-wide kanban) or bug. |
| O2 | 9 leftover E2E test tasks in "Unassigned" | Previous E2E test runs left orphaned tasks. No bulk delete or cleanup mechanism in UI. |
| O3 | Feedback-created tasks not in "Created by Me" | My Tasks shows 10 tasks, excludes 3 feedback-created ones. Server creates them without `created_by = current_user`. Probably correct behavior but could surprise users. |
| O4 | Feature Request feedback hides Severity dropdown | Good UX — only bug reports need severity. But the resulting task still gets "medium" priority default. |
| O5 | `validateDOMNesting` React warning in feedback dialog | Console warning: `<button>` cannot appear as descendant of another interactive element. Minor but indicates nesting issue in feedback dialog component. |

---

## Root Cause Analysis: BLOB UUID Mismatch

**The single biggest issue** affecting this testing session is the BLOB vs TEXT UUID format mismatch. This is a known issue (documented in `remaining-work.md` as item #1 "BLOB UUID → TEXT Migration").

**How it manifests:**
- Rust `Agent` model uses `sqlx::types::Uuid` which expects 16-byte BLOB format
- Seed database (`dev_assets_seed/duck_kanban.db`) stores agent IDs as 36-character TEXT strings (e.g., `"a1b2c3d4-..."`)
- When SQLx tries to decode a TEXT UUID into a BLOB UUID type, it fails with a decode error
- This breaks: `/api/agents`, `/api/agents/active`, `/api/agents/search`, `/api/agents/{id}`, and any endpoint that joins with the `agents` table (e.g., `/api/tasks/{id}/agent-watchers`)

**Affected features:**
- Settings > Agents (empty page)
- Task creation agent picker (perpetual loading)
- Agent Watcher Panel (silent failure)
- Any future agent execution (can't look up agent config)

**Fix options:**
1. **Quick fix:** Update seed DB to use BLOB UUIDs for agents table
2. **Proper fix:** Complete the BLOB → TEXT migration (item #1 in remaining work)
3. **Interim fix:** Change Agent model to accept both formats (try TEXT first, fall back to BLOB)

---

## Workflow Pipeline Verification

### Bug Report Flow (verified via UI)
```
1. Click "Feedback & Support" in sidebar
2. Select "Bug Report", fill title + description
3. Submit → dialog closes (no toast — Bug #4)
4. Task appears on Bugs board with:
   - Title: "[Bug] [QA-E2E] <title>"
   - Tags: qa-e2e, bug
   - Priority: medium (from severity)
   - Status: todo
5. DataSource created (type: "report", visible in Data Sources tab)
6. Bug Triage Pipeline triggered automatically
7. Run completes in ~300-400ms
8. Run visible in Workflows > Runs tab
```

### Feature Request Flow (verified via UI)
```
1. Click "Feedback & Support" in sidebar
2. Select "Feature Request" (severity dropdown hides — good UX)
3. Fill title + description
4. Submit → "Thank You!" confirmation modal appears
5. Task created with "[Feature Request]" prefix
6. Same pipeline triggers (Bug #12 — naming issue)
7. Run completes successfully
```

### Task Creation Flow (verified via UI)
```
1. Click "Create new task" button in project header
2. Fill: title, board (Features), priority, tags
3. Agent picker: broken (Bug #8)
4. Click "Create Task" → navigates to task detail page
5. Task visible on kanban in To Do column
6. Activity logging fails (Bug #9 — console error)
7. Agent Watcher Panel renders but API fails silently (Bug #3)
```

### Manual Status → InReview Flow (verified via UI)
```
1. Open task detail for feedback-created bug report
2. No direct status change on detail panel (Bug #13)
3. Click edit button → Edit Task dialog opens
4. Edit Task dialog is long, requires scrolling (Bug #14)
5. Scroll to Status dropdown → select "In Review"
6. Scroll to "Update Task" button → click
7. Task moves to "In Review" kanban column ✓
8. Activity logging fails again (Bug #9 — FK constraint, code 787)
9. Agent Watcher Panel still shows "No agent reviewers assigned"
10. No QA watcher trigger fires (Bug #11 — no watchers registered)
11. Console: agent-watchers endpoint 500 (BLOB UUID)
```

**Conclusion:** The InReview → QA watcher pipeline is **untestable** through the UI because:
- Feedback tasks have no assigned agent (Bug #10)
- Agent picker is broken (Bug #8 — BLOB UUID)
- Even if an agent were assigned, watchers can't be looked up (Bug #2)
- The entire chain: feedback → agent assignment → dev execution → InReview → QA trigger is blocked

---

## Console Error Summary

| Endpoint | HTTP Status | Count | Cause |
|----------|-------------|-------|-------|
| `/api/agents/active` | 500 | 4 | BLOB UUID decode |
| `/api/agents/search` | 500 | 2 | BLOB UUID decode |
| `/api/tasks/{id}/agent-watchers` | 500 | 4 | BLOB UUID in agent lookup |
| `/api/{task_id}/activity` | 500 | 4 | FOREIGN KEY constraint failed (code: 787) |
| `/api/nora/cinematics/briefs` | 500 | 2 | Nora endpoint not configured |
| WebSocket reconnect | N/A | 1 | Expected in dev (execution events) |

---

## Recommendations

### Immediate (before next QA session)
1. **Fix seed DB agent UUIDs** — convert agents table to BLOB format, OR change Agent model to use String type
2. **Fix task activity endpoint** — investigate the DatabaseError; likely a missing table or column
3. **Add error toast to AgentWatcherPanel** — the Phase 6 toast code exists but doesn't fire on 500

### Short-term
4. **Rename "Bug Triage Pipeline"** to "Feedback Triage Pipeline" (or create separate pipelines per feedback type)
5. **Add success toast for bug report feedback** — currently only feature requests get "Thank You!" modal
6. **Fix Intelligence Overview total sources count** — verify the aggregation query

### Short-term (cont.)
7. **Assign dev agent to feedback-created tasks** — `feedback.rs` should either auto-assign a default dev agent, or the Bug Triage Pipeline should assign one as a workflow step
8. **Add direct status change to task detail panel** — inline status dropdown or quick-action buttons (e.g., "Start", "Submit for Review", "Done")
9. **Add loading skeletons to task detail views** — My Tasks → task detail transition shows blank loading for 3-6s
10. **Fix Edit Task modal scrolling** — add max-height with overflow-y-auto on modal body, sticky footer with Cancel/Update buttons always visible. Create consistent scrollable modal pattern for all data editing forms.

### Medium-term
11. **Complete BLOB → TEXT UUID migration** — this is the root cause of 7+ of the 15 bugs found
12. **Add E2E test cleanup mechanism** — bulk delete or auto-cleanup of `[E2E]` prefixed tasks
13. **Investigate task activity FK constraint** — `FOREIGN KEY constraint failed (code: 787)` — likely references a user/actor that doesn't exist in the referenced table

---

## Environment Notes

- Backend: Rust/Axum on port 3003 (via `flox activate`)
- Frontend: Vite on port 3000 with proxy to backend
- Database: Fresh copy from `dev_assets_seed/duck_kanban.db`
- Migration fix applied: renamed `20260313000000_topsi_user_settings.sql` → `20260331000000_topsi_user_settings.sql` (version conflict with `review_annotations_and_decisions.sql`)
- 5 migrations applied successfully after rename
- Playwright MCP used for all browser interaction (accessibility snapshots, no screenshots)

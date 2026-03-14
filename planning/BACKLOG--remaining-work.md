# Backlog — Remaining Work

**Last updated:** 2026-03-14
**Context:** Consolidated from all completed planning docs. Items prioritized by impact and dependency.

---

## P0 — Blockers / Critical

### 1. Agent Flow Orchestration Engine (Phase 3H)
**Source:** `archive/2026-03-12--plan--agent-task-mcp-wiring.md`
**What:** Agent flows exist as data model only — no background worker to progress phases, enforce gates, or handle delegation. Largest remaining architecture gap.
**Status:** NOT STARTED

### 2. ACP Agents Get No MCP Servers
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F5)
**What:** `harness.rs` passes `mcp_servers: vec![]` to Gemini/Qwen sessions. Non-Claude agents can't use any tools.
**Status:** NOT STARTED

---

## P1 — Major / High Impact

### 3. Task Activity Logging Fails
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #9)
**What:** Task activity endpoint errors on both creation and update. Likely FK constraint issue.

### 4. Feedback Tasks Never Get Dev Agent Assigned
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #10)
**What:** Tasks created from feedback form don't get a dev agent, so no execution happens.

### 5. No QA Watcher Trigger on Manual InReview
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #11)
**What:** Moving a task to InReview manually doesn't trigger QA agent watchers. Only automated transitions work.

### 6. Intelligence Overview Shows 0 Sources
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #6)
**What:** Intelligence Overview page shows "Total Sources: 0" despite data sources existing.

### 7. `send_notification` Node — Real Implementation
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md`
**What:** Currently logs only — no actual email/in-app delivery. Needed for workflow automation to have visible side effects.

### 8. Unify MCP Config Systems
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F9)
**What:** Settings > MCP Servers configures Claude Code CLI's `~/.claude.json`. The executor pipeline reads `default_mcp.json`. Completely separate — confusing for users.

### 9. Workflow Trigger System
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F12)
**What:** Workflows must be manually run. No cron, event, or webhook triggers.

---

## P2 — Medium / UX Polish

### 10. No Direct Status Change from Task Detail
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #13)
**What:** Task detail panel has no status dropdown — must return to kanban to drag.

### 11. Loading Skeletons
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md`, Bug #15
**What:** Pages show blank content for 3-6s during load (task detail, agent list, My Tasks). Need skeleton UI.

### 12. AgentWatcherPanel Error State
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #3)
**What:** Shows "No agent reviewers" instead of error message when API fails.

### 13. DbUuid Phase 3 — Remaining Model Conversions
**Source:** `notes/2026-03-14--reference--dbuuid-phase3-remaining.md`
**What:** ~107 models still use `Uuid`. Phase 1-2 complete — unblocked agent endpoints. Phase 3 removes bridge code and prevents future BLOB/TEXT mismatches.
**Effort:** High volume, low risk per file. Batch by domain.

### 14. Agent "View Profile" Link
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md` (item 7)
**What:** Backend `GET /api/agents/:id/profile` works. No frontend link on agent cards.

### 15. `http_request` Node Guardrails
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md`
**What:** No OAuth/auth token management, no URL allowlisting, no rate limiting. Security concern for production.

### 16. Workflow Scheduler Hardening
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md`
**What:** `spawn_workflow_schedule_loop` needs error recovery, missed-execution handling, better logging.

### 17. Rename "Bug Triage Pipeline"
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #12)
**What:** Feature requests processed by "Bug Triage Pipeline" — should be "Feedback Triage Pipeline".

### 18. No Success Toast for Feedback Submission
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #4)

### 19. Project Task Count Doesn't Auto-Refresh
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #5)
**What:** After feedback creates a task, the sidebar project task count is stale until page refresh.

---

## P3 — Low Priority / Future Sprints

### 20. Schema Improvements for Dogfood Pipeline
**Source:** `archive/2026-03-13--plan--dogfood-pipeline-activation.md`
- Add `qa` status to distinguish QA review from human approval on kanban
- Add `deployed` status to distinguish "code merged" from "manually done"
- Add `task_id` FK on `merges` table for direct task→PR lookup
- Add `task_status_history` table for full audit trail

### 21. Sprint 2-3 Roadmap Items
**Source:** `archive/2026-03-12--plan--three-sprint-roadmap.md`
- Convert hardcoded automations to editable system workflows
- Bulk task operations (select multiple, batch status change)
- Progress indicator for agent execution on task cards
- Undo reject / re-open task workflow

### 22. Known Limitations (Design Choices)
- Conditional node: only simple string matching + `count>N`
- CRM update nodes: match by email/name only (no fuzzy matching)
- ACP MCP loading: path depends on server working directory
- Task FTS search index: auto-sync triggers dropped — needs app-level reindexing

### 23. UI Polish — Future Improvements
- Task card inline editing
- Keyboard shortcuts for common actions
- Board column WIP limits
- Workflow versioning/history
- Mobile user menu shortcut (avatar in navbar)
- View-as keyboard shortcut (Cmd+Shift+V)
- Sidebar section animated transitions
- Settings tab URL persistence

### 24. React Router v7 Migration Warnings
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md`
- Future flag deprecation warnings in console
- `validateDOMNesting` warning (button nested in button in workflow editor)

---

## TODO — Manual Verification

- [ ] Edit Task / Feedback / Project Form dialogs: verify sticky footer
- [ ] ResizableDrawer: drag resize, expand/collapse, click-outside-to-close
- [ ] Fullscreen BreadcrumbNav visible, Esc to close
- [ ] Start dev server → Settings > Agents → agents load (no 500)
- [ ] Create Task → Agent picker → agents listed
- [ ] Task detail → Agent Watcher panel → no 500 errors

---

## Resolved (Removed from Backlog)

| Item | Resolution |
|------|-----------|
| BLOB UUID agent endpoints (QA #1, #2, #7, #8) | DbUuid Phase 1-2 (commit `2cf77c82b`) |
| Edit Task modal not scrollable (QA #14) | FormDialogBody helper (commit `e72d80187`) |
| Task detail no back navigation (QA #16) | ResizableDrawer + BreadcrumbNav (commits `761bdb609`–`fad6c9fd7`) |
| QA watcher refactor (Phase 6) | Complete (PR #23) |
| Topsi-Workflow Bridge | Complete (PR #22) |
| Dogfood Pipeline phases 7-11 | Complete |

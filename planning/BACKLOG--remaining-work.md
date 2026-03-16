# Backlog — Remaining Work

**Last updated:** 2026-03-16
**Context:** Consolidated from all completed planning docs. Items prioritized by impact and dependency.

---

## P0 — Blockers / Critical

### 1. Agent Flow Orchestration Engine (Phase 3H)
**Source:** `archive/2026-03-12--plan--agent-task-mcp-wiring.md`
**What:** Agent flows exist as data model only — no background worker to progress phases, enforce gates, or handle delegation. Largest remaining architecture gap.
**Status:** NOT STARTED

### ~~2. ACP Agents Get No MCP Servers~~ → RESOLVED
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F5)
**Resolution:** Already implemented — `load_platform_mcp_servers()` in `acp/harness.rs` loads from `default_mcp.json` and passes to all ACP sessions (Gemini, Qwen). Discovered during pipeline enhancement sprint (2026-03-16).

---

## P1 — Major / High Impact

### 3. Unify MCP Config Systems
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F9)
**What:** Settings > MCP Servers configures Claude Code CLI's `~/.claude.json`. The executor pipeline reads `default_mcp.json`. Completely separate — confusing for users.

### 4. Workflow Trigger System
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F12)
**What:** Workflows must be manually run. No cron, event, or webhook triggers.

### ~~5. E2E Demo Test Run~~ → RESOLVED
**Source:** `2026-03-14--plan--e2e-demo-refactor.md`
**Resolution:** All suites run and passing — health checks 40/40, demos 28/31 (1 pre-existing flaky locator in bug-report-lifecycle Step 6). Structured test commands added to package.json with env pre-flight check (PR #34).

---

## P1.5 — Notification & Demo Enhancements

### Notification Quick Action Buttons (Future Sprint)
**Source:** 2026-03-15 dogfood session
**What:** Add quick action buttons to notification items so users can take the next workflow step directly from the notification dropdown (e.g., "Approve" staged records, "Review PR", "Mark Done"). Reduces context switching.
**Status:** NOT STARTED — deferred to future sprint

### Demo Script Improvements
**Source:** 2026-03-15 dogfood session
**What:** Add better context/narration to demo scripts — annotations explaining what each step demonstrates, why it matters. Currently the demos work but don't convey the product story.
**Status:** PARTIALLY DONE — workflow CRM demo extended with Parts 5-8 (approve, runs tab, CRM contacts, pipeline). Bug report + manual QA demos still need narration.

---

## P2 — Medium / UX Polish

### 6. AgentWatcherPanel Error State
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #3)
**What:** Shows "No agent reviewers" instead of error message when API fails.

### 7. DbUuid Phase 3 — Remaining Model Conversions
**Source:** `notes/2026-03-14--reference--dbuuid-phase3-remaining.md`
**What:** ~107 models still use `Uuid`. Phase 1-2 complete — unblocked agent endpoints. Phase 3 removes bridge code and prevents future BLOB/TEXT mismatches.
**Effort:** High volume, low risk per file. Batch by domain.

### 8. Agent "View Profile" Link
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md` (item 7)
**What:** Backend `GET /api/agents/:id/profile` works. No frontend link on agent cards.

### 9. `http_request` Node Guardrails
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md`
**What:** No OAuth/auth token management, no URL allowlisting, no rate limiting. Security concern for production.

### ~~10. Rename "Bug Triage Pipeline"~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #12)
**Resolution:** Renamed to "Feedback Triage Pipeline" in `data_source_workflows.rs`, `feedback.rs`, and E2E tests (2026-03-16).

### 11. ~~No Success Toast for Feedback Submission~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #4)
**Resolution:** FeedbackDialog now uses toast (commit `7cd4b22c9`)

### 12. Project Task Count Doesn't Auto-Refresh
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #5)
**What:** After feedback creates a task, the sidebar project task count is stale until page refresh.

---

## P3 — Low Priority / Future Sprints

### 13. Schema Improvements for Dogfood Pipeline
**Source:** `archive/2026-03-13--plan--dogfood-pipeline-activation.md`
- Add `qa` status to distinguish QA review from human approval on kanban
- Add `deployed` status to distinguish "code merged" from "manually done"
- Add `task_id` FK on `merges` table for direct task→PR lookup
- Add `task_status_history` table for full audit trail

### 14. Sprint 2-3 Roadmap Items
**Source:** `archive/2026-03-12--plan--three-sprint-roadmap.md`
- Convert hardcoded automations to editable system workflows
- Bulk task operations (select multiple, batch status change)
- Progress indicator for agent execution on task cards
- Undo reject / re-open task workflow

### 15. Known Limitations (Design Choices)
- Conditional node: only simple string matching + `count>N`
- CRM update nodes: match by email/name only (no fuzzy matching)
- ACP MCP loading: path depends on server working directory
- Task FTS search index: auto-sync triggers dropped — needs app-level reindexing

### 16. UI Polish — Future Improvements
- Task card inline editing
- Keyboard shortcuts for common actions
- Board column WIP limits
- Workflow versioning/history
- Mobile user menu shortcut (avatar in navbar)
- View-as keyboard shortcut (Cmd+Shift+V)
- Sidebar section animated transitions
- Settings tab URL persistence

### 17. React Router v7 Migration Warnings
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
- [ ] Run E2E demos headed: `FRONTEND_PORT=3001 npx playwright test --project=demos`

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
| Task activity logging (Bug #9) | DbUuid migration + ActivityLog calls (commit `bb2e8dfb7`, `ace91ffcb`) |
| Feedback tasks no dev agent (Bug #10) | Topsi assignment in feedback.rs (commit `49363911c`) |
| Manual InReview QA trigger (Bug #11) | Watcher trigger in update_task (commit `ace91ffcb`) |
| Intelligence Overview 0 sources (Bug #6) | find_by_organization query (commit `49363911c`) |
| send_notification node + in-app notifications | Full stack: migration, model, routes, frontend bell (commit `bc07ba53e`) |
| Workflow trigger hardening | Tracing spans + CancellationToken (commit `5165b4091`) |
| Status dropdown in task detail (Backlog #10) | Combobox in task detail header (commit `3b40cf272`) |
| Loading skeletons (Backlog #11) | Skeleton components for My Tasks (commit `7d2e910f1`) |
| BLOB binding for project creation | bind_uuid_blob helpers (commit `e6f34f140`) |
| Drawer click-outside closes on portaled overlays | Radix overlay detection (commit `cb31ba595`) |
| E2E demo script duplication + headed mode | Dedicated fixtures + shared helpers (commit `55d24889a`) |
| PR #27 C1: notifications BLOB/TEXT mismatch | BLOB bytes bind for project_members query (commit `dd3f20388`) |
| PR #27 C2: bind_uuid_blob panic | Returns Result (commit `1b65c73a1`) |
| PR #27 C3: fragile GitHub URL parsing | url::Url parser (commit `dd3f20388`) |
| PR #27 C4: contentFullscreen stuck in localStorage | partialize excludes it (commit `1b65c73a1`) |
| PR #27 C5: status dropdown cache miss | queryClient.invalidateQueries (commit `1b65c73a1`) |
| PR #27 W3: Notification/ActivityLog String→DbUuid | Consistency fix (commit `dd3f20388`) |
| PR #27 W4: feedback agent arbitrary selection | Agent::find_default_assignee (commit `dd3f20388`) |
| PR #27 W8: FeedbackDialog nested JSX | Success → toast notification (commit `7cd4b22c9`) |
| PR #27 W14: DbUuid encoding undocumented | Module docs expanded (commit `dd3f20388`) |
| No success toast for feedback submission (Bug #4) | Toast via sonner (commit `7cd4b22c9`) |
| PR #27 W1: manual InReview doesn't spawn QA review | spawn_watcher_reviews() standalone (commit `f03dab011`) |
| Notification UUIDs instead of display names | Backend LEFT JOIN for actor_name (2026-03-15) |
| Notification icons not visually distinct | Colored icon circles + actor type badges (2026-03-15) |
| Run Workflow button has no loading indicator | Added Loader2 spinner to data-sources.tsx (2026-03-15) |
| Workflow CRM demo incomplete (no commit/verify) | Extended with Parts 5-8: approve, runs, contacts, pipeline (2026-03-15) |
| E2E demo test run (Backlog #5) | Health checks 40/40, demos 28/31, structured test commands (PR #34) |
| Tech debt sprint: .unwrap() elimination | 40 unwraps → proper error handling across 8 files (PR #34) |
| Tech debt sprint: api.ts monolith | 6,669 lines → 21 domain modules (PR #34) |
| Tech debt sprint: org-profile.tsx monolith | 7,086 lines → 33 modular files with lazy-loaded tabs (PR #34) |
| Tech debt sprint: CI pipeline | fmt + clippy + test + lint + types + audit (PR #34) |
| Tech debt sprint: eprintln in prod code | 4 calls → 0, replaced with tracing (PR #34) |
| Modularity Sprint 2: formatters centralization | `formatDate`/`formatCurrency`/`formatCompactNumber`/`parseJsonArray` → `lib/formatters.ts`, 20 duplicates eliminated (PR #37) |
| Modularity Sprint 2: EmptyState adoption | Component adopted in 7 files, 13 inline patterns replaced (PR #37) |
| Modularity Sprint 2: raw fetch() consolidation | 10 consumers migrated to typed API client modules (PR #37) |
| Modularity Sprint 2: nora/tools.rs split | 7,347 lines → 8 modules in `tools/` directory (PR #37) |
| Modularity Sprint 2: workflows.tsx split | 2,418 lines → 9 files in `pages/workflows/` (PR #37) |
| Modularity Sprint 2: WorkflowEditor.tsx split | 2,240 lines → 7 files in `editor/` with barrel re-export (PR #37) |
| Modularity Sprint 2: project-detail.tsx split | 2,293 lines → 9 files in `project-detail/` (PR #37) |
| Modularity Sprint 2: task_server.rs split | 3,457 lines → 9 files in `task_server/` (PR #37) |
| Modularity Sprint 2: brand-guide.tsx split | 1,703 lines → 17 files in `brand-guide/` (PR #37) |
| Modularity Sprint 2: shared query hooks | `useProjectList`, `useOrganizationById`, `useCrmContacts` extracted (PR #37) |
| ACP agents get no MCP servers (Backlog #2) | Already implemented — `load_platform_mcp_servers()` + `default_mcp.json` (2026-03-16) |
| Rename Bug Triage Pipeline (Backlog #10) | → "Feedback Triage Pipeline" in code + E2E tests (2026-03-16) |
| Pipeline enhancement merge + DbUuid fixes | sloperation314 merged, 12 Rust + 4 TS compile fixes (2026-03-16) |
| Pipeline intelligence E2E demo | 5-part 8-stage deal progression demo (2026-03-16) |

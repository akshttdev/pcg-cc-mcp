# Remaining Work — Post-Sprint Summary

**Date:** 2026-03-12 (updated 2026-03-13)
**Branch:** `feature/blob-to-text-scoped` (commit `ff6b76654`)
**Context:** All QA plan items (15/15) complete. All critical bugs fixed. Phases 7-11 (Dogfood Pipeline) code-complete. This doc captures remaining open items across planning docs.

---

## High Priority

### 1. BLOB UUID → TEXT Migration
**Source:** Separate scope (was on `feature/fraze-2026-03-12` but incomplete)
**What:** 26+ tables store UUIDs as 16-byte BLOBs (SQLx default for Rust `Uuid`). Need to convert to TEXT for readability, query compatibility, and tooling.
**Approach:** Code-first (change `Uuid` → `String` in ~140 model files), then data migration SQL.
**Status:** Plan documented in `2026-03-12--task--blob-uuid-to-text-migration.md` (on fraze branch). Not started on current branch.
**Risk:** Touches nearly every file in `crates/db/` and `crates/server/`. Should be a dedicated PR.

### 2. Agent Flow Orchestration Engine (Phase 3H)
**Source:** `2026-03-12--plan--agent-task-mcp-wiring.md`
**What:** Background worker that progresses agent flow phases, enforces gates, handles delegation. Agent flows exist as data model only — no execution engine.
**Status:** NOT STARTED — largest remaining effort item.

### 3. `send_notification` Node — Real Implementation
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md` (originally)
**What:** Currently logs only — no actual email/in-app delivery. Placeholder for future integration.
**Priority:** P1 — needed for workflow automation to have visible side effects.

---

## Medium Priority

### 4. Agent "View Profile" Link
**Source:** `2026-03-12--tracker--sprint1-issues.md` (item 7)
**What:** Agent cards in Settings → Agents should link to profile data. Backend endpoint `GET /api/agents/:id/profile` works, but no frontend UI consumes it.
**Status:** Partially resolved — endpoint verified, UI link not added.

### 5. `http_request` Node — Guardrails
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md`
**What:** No OAuth/auth token management — headers must be static. No URL allowlisting. No rate limiting.
**Priority:** P2 — security concern for production use.

### 6. Workflow Scheduler Hardening
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md`
**What:** Schedule trigger loop (`spawn_workflow_schedule_loop`) needs error recovery, missed-execution handling, and better logging.
**Priority:** P2.

### 7. Loading Skeletons
**Source:** `archive/2026-03-12--review--project-task-agent-automation.md` (U3)
**What:** Pages show blank content for 3-6s during load. Need skeleton UI.
**Priority:** P2 — UX polish.

---

## Low Priority / Future Sprints

### 8. Three-Sprint Roadmap — Remaining Items
**Source:** `archive/2026-03-12--plan--three-sprint-roadmap.md` (archived)
**Sprint 2 open items:**
- Convert hardcoded automations to editable system workflows
- Bulk task operations (select multiple, batch status change)
- Progress indicator for agent execution on task cards

**Sprint 3 open items:**
- Agent flow orchestration (see #2 above)
- Full agent-to-MCP wiring verification (end-to-end test)
- Undo reject / re-open task workflow

### 9. UI/Backend Capability Gaps
**Source:** `2026-03-12--review--ui-backend-capability-gaps.md`
**16 usability gaps + 10 functionality gaps identified.** The critical ones were addressed in this sprint. Remaining are polish-tier items like:
- Task card inline editing
- Keyboard shortcuts for common actions
- Board column WIP limits
- Workflow versioning/history

### 10. Known Limitations
**Source:** `2026-03-12--tracker--sprint1-issues.md`
1. Conditional node: Only simple string matching + `count>N`
2. CRM update nodes: Match by email/name only — no fuzzy matching
3. ACP MCP loading: Path depends on server working directory
4. Task FTS search index: Auto-sync triggers dropped (C4 fix) — needs app-level reindexing

### 11. QA Automation Review — Remaining UX Items
**Source:** `archive/2026-03-12--review--qa-automation-loop.md` (archived, all core items addressed)
**Minor polish leftover:**
- Task detail: show Description/Completion Criteria in default view (not just Enhanced)
- Template table: show Priority + Agent columns
- Workflow triggers panel: expose in Builder tab
- Agent terminal: show capabilities inline + VIBE cost estimate

---

## Completed Since Last Update (2026-03-13)

### Dogfood Pipeline (Phases 7-11) — ALL COMPLETE
**Source:** `2026-03-13--plan--dogfood-pipeline-activation.md`
- Phase 7: Seed agents + runtime config docs ✅
- Phase 8: Auto-approve + QA loop + audit trail ✅
- Phase 9: PR feedback loop (verdict → comment → iteration) ✅
- Phase 10: E2E Playwright tests ✅
- Phase 11: Agent execution dashboard + frontend polish ✅

**Remaining:** Runtime configuration only (API keys, GitHub PAT) — see `notes/2026-03-13--reference--llm-workflow-config.md`

---

## Active Planning Docs

| File | Purpose | Status |
|------|---------|--------|
| `2026-03-13--plan--dogfood-pipeline-activation.md` | Dogfood pipeline phases 7-11 | ✅ Code complete |
| `2026-03-12--plan--agent-task-mcp-wiring.md` | Agent-task integration plan | Phase 3H open |
| `2026-03-12--review--ui-backend-capability-gaps.md` | Usability/functionality gap analysis | Polish items open |
| `2026-03-12--tracker--sprint1-issues.md` | Implementation issues tracker | 5 limitations noted |
| `notes/2026-03-13--reference--llm-workflow-config.md` | Runtime config reference | Active reference |

## Archived Docs (in `planning/archive/`)

| File | Reason |
|------|--------|
| `2026-03-11--analysis--pr11-pr12-diff.md` | Investigation complete |
| `2026-03-11--analysis--pr12-ci-runner.md` | Investigation complete |
| `2026-03-11--analysis--pr12-lost-features.md` | Investigation complete |
| `2026-03-11--analysis--pr13-sound-fix.md` | Investigation complete |
| `2026-03-11--analysis--pr15-merge-conflict.md` | Investigation complete |
| `2026-03-11--review--workflow-pipeline-e2e.md` | All tests documented |
| `2026-03-12--analysis--fraze-sprint-merge-review.md` | Merge complete |
| `2026-03-12--analysis--sloperation311-merge-review.md` | Merge complete |
| `2026-03-12--release--seed-db-migration-update.md` | Release applied |
| `2026-03-12--review--project-task-agent-automation.md` | All bugs fixed, Playwright-verified |
| `2026-03-12--review--qa-automation-loop.md` | All core items addressed (Phases 7-11) |
| `2026-03-12--plan--three-sprint-roadmap.md` | All sprint items either done or captured here |

## Deferred Docs (in `planning/deferred/`)

| File | Reason |
|------|--------|
| `2026-03-11--plan--sqlx-migration-consolidation.md` | Superseded by blob migration approach |

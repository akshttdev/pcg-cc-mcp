# Backlog — Remaining Work

**Last updated:** 2026-03-18 (post PR #46 review — quality sprint)
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

### ~~Notification Quick Action Buttons~~ → RESOLVED
**Source:** 2026-03-15 dogfood session
**Resolution:** Implemented in PR #39 — InboxNotificationItem has source-aware action buttons (View Task, View Run), dismiss button; ActivityNotificationItem has status chips and view buttons; NotificationCenter has improved deep-linking (2026-03-16).

### Demo Script Improvements
**Source:** 2026-03-15 dogfood session
**What:** Add better context/narration to demo scripts — annotations explaining what each step demonstrates, why it matters. Currently the demos work but don't convey the product story.
**Status:** PARTIALLY DONE — workflow CRM demo extended with Parts 5-8 (approve, runs tab, CRM contacts, pipeline). Bug report + manual QA demos still need narration.

---

## P2 — Medium / UX Polish

### ~~6. AgentWatcherPanel Error State~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #3)
**Resolution:** PR #41 added `loadError` state with error UI (AlertTriangle icon + "Failed to load agent reviewers" message + RefreshCw retry button). Distinguishes "no reviewers configured" from "API error".

### 7. DbUuid Migration — 4-Phase Plan
**Source:** `2026-03-17--plan--dbuuid-migration.md`, `notes/2026-03-14--reference--dbuuid-phase3-remaining.md`
**What:** Eliminate all Uuid/Vec<u8>/BLOB boilerplate. 4 phases:
- **Phase A** (highest ROI): `AccessContext.user_id: Uuid` → `DbUuid` — eliminates ~36 conversion sites across ~18 files
- **Phase B**: `Path<Uuid>` → `Path<String>` in route handlers — ~416 sites across ~79 files
- **Phase C**: Migrate `users.id` BLOB → TEXT — eliminates ALL remaining `.as_bytes()` / `Vec<u8>` / `bind_uuid_blob` code
- **Phase D**: Batch convert remaining ~104 models `Uuid` → `DbUuid`
**Status:** Planning complete. 16 files annotated with `// TODO(dbuuid)` in PR #45. Phase A ready to start.

### ~~8. Agent "View Profile" Link~~ → RESOLVED
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md` (item 7)
**Resolution:** PR #41 added agent profile page at `/agents/:agentId/profile` showing agent description, capabilities, status, default model, and autonomy level. AgentWatcherPanel agent names now link to the profile page. Route registered in App.tsx with ProtectedRoute wrapper.

### 8b. `any` Type Cleanup in New CRM/Pipeline Frontend Code
**Source:** PR #45 QA review
**What:** ~20+ `: any` or `as any` casts in `client-overview.tsx`, `projects.tsx`, and other files from the pipeline-progress cherry-pick. Violates frontend TS standards.
**Status:** NOT STARTED — deferred from PR #45 (pre-existing pattern, not a regression)

### 8c. Sovereign Stack Volume Path Deduplication
**Source:** PR #45 QA review
**What:** `resolve_volume_path()` logic is duplicated in `org_cloud.rs` and `data_sources.rs`. Extract to a shared utility in `crates/utils/` or `crates/server/src/`.
**Status:** NOT STARTED — both copies now use env vars, but should be unified

### 9. `http_request` Node Guardrails
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md`
**What:** No OAuth/auth token management, no URL allowlisting, no rate limiting. Security concern for production.

### ~~10. Rename "Bug Triage Pipeline"~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #12)
**Resolution:** Renamed to "Feedback Triage Pipeline" in `data_source_workflows.rs`, `feedback.rs`, and E2E tests (2026-03-16).

### 11. ~~No Success Toast for Feedback Submission~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #4)
**Resolution:** FeedbackDialog now uses toast (commit `7cd4b22c9`)

### ~~12. Project Task Count Doesn't Auto-Refresh~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #5)
**Resolution:** Fixed in PR #38 — sidebar task count refresh on task creation/update (2026-03-16).

---

## P2.5 — Modularity Sprint 6 Candidates

### Modularity Sprint 6 — Hook Mutations + Remaining Debt
**Source:** `archive/2026-03-16--plan--modularity-sprint-5.md` (deferred 2e), PR #46 review
**What:** Remaining raw `useMutation` → `useMutationWithToast`:
- ~~`hooks/useOrgOnboarding.ts` (4 mutations)~~ → Done (PR #46)
- ~~`hooks/useWorkflowTemplates.ts` (1 mutation)~~ → Done (PR #46)
- ~~`hooks/useCrmActivities.ts` (2 mutations)~~ → Done (PR #46)
- `hooks/useAgentFlows.ts` (4 mutations)
- `hooks/useTaskMutations.ts` (3 mutations) — complex: async activity logging side effects
- `pages/oss-library-listener.tsx` (4 mutations)
- `pages/data-sources.tsx:910` `RunWorkflowFromSourceDialog.runMutation` — conditional toast logic
Also: remaining conversation helper adoption (nora/voice, agent_chat, twilio), ~90 inline query keys (down from ~318)
- Over-invalidation in autonomy/bowser/collaboration mutations (broad `autonomyKeys.all` instead of targeted keys)
- `MembersTab` raw `fetch()` → `makeRequest` migration
- Query key string mismatches: `['brandProfile', orgId]` vs factory `['orgBrandProfile', orgId]`, `['workflowTemplates']` vs factory `['workflow-templates']` — need coordinated rename
**Source also:** `archive/2026-03-16--plan--modularity-sprint-4.md` (deferred items)
**Status:** PARTIALLY DONE (PR #46 converted 19 mutations, centralized ~228 keys, ~90 inline keys remain)

### Modularity — Large Frontend File Splits
**Source:** `archive/2026-03-15--plan--modularity-sprint-2.md` (remaining large files section)
**What:** Largest remaining frontend files:
- `virtual-environment.tsx` (1,282 lines)
- `TaskFormDialog.tsx` (1,240 lines) — complex form, high-traffic
- `company-profile.tsx` (1,218 lines) — similar pattern to project-detail split
- ~~`MeetingMode.tsx` (1,207 lines)~~ → Done (PR #46, split to `meeting-mode/`)
- `CrmDealDetailPanel.tsx` (1,202 lines) — grew in PR #36
**Status:** PARTIALLY DONE (PR #46 split 3 files: MeetingMode, NoraAssistant, project-tasks). 7 files >900 lines remain (10 including preserved originals pending deletion).

### Workflow UX — Deferred Polish
**Source:** `archive/2026-03-16--plan--workflow-ux-sprint.md` (deferred items + PR #44 review)
**What:**
- Extract `DataSourceWorkflowRunner` from `data-source-detail.tsx` (~730 lines) — Day 2 deferral
- ~~`CopyWorkflowDialog` NiceModal migration~~ → Already on NiceModal (PR #46 converted its mutation)
- ~~`window.confirm()` → shadcn `AlertDialog`~~ → All `window.confirm()` calls eliminated (PR #46 Day 1)
- Ownership filter toggle (My/Org/All) on workflow cards — badges exist but no filtering
**Status:** PARTIALLY DONE (PR #46 resolved confirm/mutation items)

### Modularity — Deferred Stretch Items
**Source:** `archive/2026-03-15--plan--modularity-sprint-2.md` (Day 5c)
**What:** Hook splits deferred (below priority threshold):
- `useConversationHistory.ts` (540 lines) → extract `flattenEntries`, `executionHelpers`, `patchWithKey`
- `useAutonomy.ts` (478 lines) → extract `useCheckpoints`, `useApprovalGates`
**Status:** NOT STARTED

### Modularity — formatRelativeDate Centralization
**Source:** PR #37 QA review
**What:** 3 independent `formatRelativeDate` implementations remain in EmailInbox, CommunicationsInbox, WorkflowRunsPanel. Different logic in each (today/yesterday vs "Xm ago" format). Could centralize with a configurable formatter.
**Status:** NOT STARTED — not a regression, just incomplete DRY

---

## Active Branch Conflict Notes (2026-03-18)

### `sloperation316-pipeline-progress` — Updated from main (2026-03-18)
PR #45 merged `integration/sloperation316` (a separate integration branch) — NOT this branch. Pipeline-progress has 77 files of unmerged work (CRM pipeline automation, company profiles, brand guides, e2e tests).
**Status**: Merged `origin/main` — no conflicts. Ready to merge to main when pipeline features are approved.

### `sloperation316-vibe-integration` — Updated from main (2026-03-18)
Superset of pipeline-progress (Dockerfile fix + VIBE tokenomics plan). Merged `origin/main` with 17 conflicts resolved:
- `company.rs`: kept main's `hex()` UUID lookup
- `crm_deals.rs`: kept vibe's `call_llm()`, Astra Pass 2, `generate_*_core()` pattern
- `App.tsx`: kept `org_viewer` for `/people` routes (consistent with CRM)
- Frontend: kept vibe's pipeline features, fixed `personsApi.get()` and `tasksApi.getAll()` method names
**Note**: If PR #46 merges to main before these branches, they'll need another merge from main to pick up query key factories, mutation conversions, and file splits.

### `refactor/dev-velocity-sprint` — Zero conflict with PR #46
Planning file only. Focus: DbUuid (Rust), unwrap elimination (Rust), `any` types (different TS files), route authz (Rust).

### Pre-existing DB/Server Issues (found during e2e setup)
1. **Seed DB BLOB→TEXT**: `users` table still has BLOB UUIDs. Runtime fix: `UPDATE users SET id = lower(substr(hex(id),...))`. Seed needs regeneration.
2. **Onboarding warning**: `table projects has no column named slug` — `user_onboarding.rs` references a column missing from seed schema. Non-blocking.
3. **Migration checksum mismatch**: `20260409000000_org_cloud.sql` was modified after being applied to seed. Fresh seed + `sqlx migrate run` required.

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
- ~~Bulk task operations (select multiple, batch status change)~~ → Done (PR #38)
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
- ~~Settings tab URL persistence~~ → Done (PR #39, `?scope=` param)

### 18. UX Audit — Remaining Items (from `2026-03-16--review--ux-design-audit.md`)
**P2**: Topsi chat suggested prompts (#14)
~~Login "session expired" on first visit (#16)~~ → Already fixed (PR #38, `hadPriorSession` localStorage check)
~~Dev banner space reduction (#17)~~ → Fixed (PR #41, inline pill in navbar header)

### 19. UX Audit — Partially Addressed (depth improvements)
**Source:** `archive/2026-03-16--review--ux-design-audit.md`
- Pipeline: stage visual differentiation (gradient backgrounds), agent action tooltip improvements (#2)
- ~~Notifications: tabs (All/Unread/Mentions), "Mark all read" button, full notifications page (#4)~~ → RESOLVED (PR #41)
- KPI trends: trend arrows with percentages (requires API), time period selector, sparklines (#10)
- Project cards: last activity timestamp, assignee avatars (#18)
- Task card data population: seed data needs richer metadata (assignees, due dates, tags) (#3)
- Breadcrumbs: "Jungleverse" still appears as default org name in some views (#15)

### 20. User Account Onboarding (deferred from UX Sprint 2)
**Source:** `archive/2026-03-16--plan--ux-engagement-polish-sprint2.md`
**What:** First-login walkthrough — profile setup, preferences, Topsi intro. Org onboarding (PR #39) covers org-level setup; user account onboarding covers individual user first-run experience.
**Status:** NOT STARTED

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
| UX Sprint 2: Org-scoped onboarding system | New DB tables, API module, React Query hook, org overview integration (PR #39) |
| UX Sprint 2: Notification quick actions | Source-aware action buttons, dismiss, improved deep-linking (PR #39) |
| UX Sprint 2: Dark mode hardcoded color fixes | CSS `--brand-gold` variable, OnboardingCarousel dark variants (PR #39) |
| UX Sprint 2: KPI trend indicators | TrendIndicator + workflow run stats on org overview (PR #39) |
| UX Sprint 2: Project card progress bar | Task completion Progress bar on ProjectCard (PR #39) |
| UX Sprint 2: Workflow last-run status | Status indicator on workflow definition cards (PR #39) |
| UX Sprint 2: Pipeline empty state | Onboarding prompt card in empty pipeline first column (PR #39) |
| UX Sprint 2: Settings "Coming Soon" cleanup | Planned items grouped at bottom with reduced opacity (PR #39) |
| Task.collaborators missing from struct | Field existed in SQL + DB but not in Task struct — SQLx silently discarded (PR #39) |
| Notification quick action buttons (Backlog P1.5) | Implemented: source-aware actions, dismiss, deep-linking (PR #39) |
| Project task count auto-refresh (Backlog #12) | Sidebar task count refresh on create/update (PR #38) |
| UX Sprint 3: Sidebar merged sections (Audit #5) | Management + Global Views → "Views & Management", "More" popover for external links (PR #41) |
| UX Sprint 3: Drawer wider default (Audit #6) | Default width 600→800px in resizable-drawer.tsx (PR #41) |
| UX Sprint 3: Notification tabs + full page (Audit #4) | All/Inbox/Activity tabs in dropdown, `/notifications` page with search/filter (PR #41) |
| UX Sprint 3: Integrations hub redesign (Audit #9) | Recommended/Optional split, progress ring, muted gray for missing optional (PR #41) |
| UX Sprint 3: Task card priority borders (Audit #3) | 3px left-border colored by priority level (PR #41) |
| UX Sprint 3: Hide test tasks toggle (Audit #8) | "Hide Tests" button on kanban filters `[E2E]`/`[Test]` prefix tasks (PR #41) |
| UX Sprint 3: CRM sub-nav dedup (Audit #19) | CRM removed from project sub-nav in ProjectFolder.tsx (PR #41) |
| AgentWatcherPanel error state (Backlog #6) | Error UI with retry button, distinguishes empty vs error (PR #41) |
| Agent profile page (Backlog #8) | `/agents/:agentId/profile` page with capabilities, status, model (PR #41) |
| Route conflict fix (pre-existing) | Merged duplicate routes in org_onboarding.rs (committed to main) |
| Migration version collision fix (pre-existing) | Renamed 20260405000000→20260405000001 to avoid collision (committed to main) |
| Modularity Sprint 5: VIBE billing duplication | `ensure_vibe_balance` + `record_llm_vibe_usage` in `helpers/billing.rs`, 5 blocks eliminated (PR #43) |
| Modularity Sprint 5: topsi raw fetch() | `topsi.ts` API module wrapping 7 endpoints + `topsiKeys` factory (PR #43) |
| Modularity Sprint 5: topiclips raw fetch() | `topiclips.ts` API module wrapping 7 endpoints + `topiclipsKeys` factory (PR #43) |
| Modularity Sprint 5: twilio.rs monolith | 2285 lines → 8-file `twilio/` directory module (PR #43) |
| Modularity Sprint 5: task_attempts.rs monolith | 1909 lines → 6-file `task_attempts/` directory module (PR #43) |
| Modularity Sprint 5: nora/mod.rs decomposition | 1172→718 lines, extracted config.rs, rate_limiter.rs, initialization.rs (PR #43) |
| Modularity Sprint 5: conversation persistence | `helpers/conversations.rs` with `persist_chat_exchange`, adopted in topsi + nora chat (PR #43) |
| Modularity Sprint 5: query key migration (8 files) | ~43 inline keys → factories, new `socialKeys`/`networkKeys` factories (PR #43) |
| Workflow UX: Inline staging review | `RunAndReviewPanel` composite, `RunWorkflowDialog` `onRunComplete` callback (PR #44) |
| Workflow UX: Shared workflow card grid | `WorkflowCardGrid` with ownership badges, used in BuilderTab + WorkflowsView (PR #44) |
| Workflow UX: Copy/promote workflows | `CopyWorkflowDialog` for user ↔ org workflow copying (PR #44) |
| Workflow UX: Node picker search + categories | Search/filter, category headers with counts (PR #44) |
| Workflow UX: Workflow ID auto-generation | ID derived from name with lock/unlock toggle (PR #44) |
| Workflow UX: Graph view zoom + selection | Zoom controls, node selection highlight, connection validation (PR #44) |
| Workflow UX: Prompt templates + output schema | 3 starter templates, schema Select dropdown (PR #44) |
| Workflow UX: Sidebar workflow sub-links | Builder/Runs/Staging sub-links with pending count badge (PR #44) |
| Workflow UX: Real-time updates | AgentWatcher → React Query 10s polling, staging `refetchInterval` (PR #44) |
| Workflow UX: Deleted source display | "(Deleted source)" instead of truncated UUIDs in runs (PR #44) |
| Workflow UX: `any` type cleanup + code quality | `WorkflowRunResult` interface, `cn()`, `useEffect` fixes (PR #44) |
| Modularity Sprint 4: billing helper deferred | Done in Sprint 5 — `helpers/billing.rs` (PR #43) |

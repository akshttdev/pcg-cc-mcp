# Project, Task & Agent Automation — QA Review

**Date:** 2026-03-12
**Branch:** `feature/fraze-2026-03-12`
**Tested by:** Playwright Firefox MCP browser automation
**Status:** Active — bugs found and 6 fixes applied (session 2, 3, & 4)

---

## Executive Summary

Tested the full project → task → agent automation loop across: project creation, kanban task management, My Tasks, Workflows, Settings, Mission Control, and MCP configuration. Found **1 critical routing bug** (fixed), **1 database schema regression** (fixed), **1 task creation bug** (fixed), and several usability gaps (partially fixed). The platform is functional for the core workflow after all fixes.

---

## Critical Findings

### C1. Route Parameter Shadowing — `/api/tasks/created-by-me` (FIXED)

**Severity:** Critical — broke My Tasks page completely
**Root cause:** Axum 0.8 `.nest("/{task_id}", ...)` shadowed sibling static routes (`/created-by-me`, `/assigned-to-me`, `/watched`). The `/{task_id}` parameter matched first, trying to parse "created-by-me" as a UUID.

**Error:** `Cannot parse 'task_id' with value 'created-by-me': UUID parsing failed`

**Fix applied** in `crates/server/src/routes/tasks.rs`:
- Replaced `.nest("/{task_id}", task_id_router)` with explicit `.route("/{task_id}", ...)` routes merged via `.merge(task_id_routes)`
- The `load_task_middleware` layer is scoped only to the task-ID routes
- Compiled clean, server restarted, My Tasks page now loads correctly

**Verification:** After fix, `GET /api/tasks/created-by-me` returns 200. My Tasks page renders all 4 filter tabs (All, Assigned to Me, Created by Me, Watching) with stats bar and sort dropdown.

### C2. Task Templates API — 500 Internal Server Error

**Severity:** High — Settings → General "Task Templates" section broken
**Error:** `GET /api/templates` returns `{"success":false,"message":"DatabaseError: no column found for name: id"}`
**Root cause:** `TEMPLATE_COLS` used SQLx compile-time macro syntax (`"id as 'id!: Uuid'"`) with runtime `query_as()`. SQLite returned columns named `"id!: Uuid"` instead of `"id"`, causing `FromRow` to fail.
**Fix applied** in `crates/db/src/models/task_template.rs`: Changed to plain column names. `FromRow` handles type conversions automatically.
**Verification:** `GET /api/templates` now returns 200 with template data.

### C3. Task `created_by` Stored as Literal "current-user" (FIXED)

**Severity:** High — "Created by Me" tab always showed 0 tasks
**Root cause:** `create_task()` handler passed the frontend's `created_by` field directly to the database without overriding with the authenticated user's ID. Frontend sends `"current-user"` as a placeholder.
**Fix applied** in `crates/server/src/routes/tasks.rs`: Both `create_task()` and `create_task_and_start()` now override `payload.created_by` with `access_context.user_id.to_string()`.
**Verification:** After fix, "Created by Me" tab shows 4 tasks. New tasks store the real user UUID.

### C4. FTS5 Triggers Incompatible with RETURNING Clause (FIXED)

**Severity:** Critical — All task updates fail with "no such column: T.task_id"
**Root cause:** SQLite FTS5 sync triggers (`tasks_fts_insert`, `tasks_fts_update`, `tasks_fts_delete`) fire on `INSERT/UPDATE/DELETE` but SQLite does not allow AFTER triggers that write to virtual tables when the triggering statement uses `RETURNING`. Since sqlx uses `RETURNING` for all write queries, any task modification fails.
**Fix:** Dropped all 3 FTS triggers from the dev DB. Created migration `20260327000000_drop_tasks_fts_triggers.sql` to prevent recurrence on fresh databases.
**Impact:** Task FTS search index will not auto-update. Application-level reindexing needed (future work).
**Verification:** `PUT /api/tasks/:id` returns 200. Task status changes reflected on kanban board.

### C5. Vite Proxy Port Mismatch After Server Restart

**Severity:** High — All API requests return 500 through the proxy
**Root cause:** Vite dev server started without `BACKEND_PORT` env var, falling back to port 58297 (default). Backend runs on 3001.
**Fix:** Restart Vite with `BACKEND_PORT=3001` sourced from `.env`.
**Note:** This is a development environment setup issue, not a code bug.

---

## Pages Tested & Status

| Page | URL | Status | Notes |
|------|-----|--------|-------|
| My Tasks | `/my-tasks` | **WORKS** (after C1 fix) | 4 filter tabs, sort, stats bar. Empty state "All caught up!" |
| Workflows | `/workflows` | **WORKS** | 5 system workflows in Builder tab. Tabs: Builder, Runs, Staging, Automations, More |
| Settings → General | `/settings/general` | **WORKS** | Appearance, Task Execution, Editor, GitHub, Notifications, Templates (fixed — C2), Safety |
| Settings → MCP | `/settings/mcp` | **WORKS** | 4-card grid (ORCHA, Duck Kanban, Context7, Playwright). JSON editor. Agent dropdown. |
| Settings → Developer | `/settings/developer` | **WORKS** | VIBE bypass toggle + Disable Real-time Events toggle. Both functional. |
| Mission Control | `/mission-control` | **WORKS** | Quick-start guidance card shown. 0 Active agents. Timeline/Workflows/Grid tabs. Live Coordination panel. |
| Project Detail | `/projects/:id` | **WORKS** | Brand identity, stats, Integrations Hub, Workstreams, Asset Vault sections |
| Kanban Board | `/projects/:id/tasks` | **WORKS** | 5 columns (To Do, In Progress, In Review, Done, Cancelled). 1 task in To Do. Toolbar: Filter, Priority, Import, Export, Select, Archived, Enhanced Cards, Manage Tags, Board |
| Task Create Modal | (from project header) | **WORKS** | All fields present (see UX findings below) |

---

## Usability Findings

### U1. Task Create Modal — Positioning & Layout Issues

**Severity:** Medium
**Observed:**
- Modal appears left-aligned, not properly centered on viewport
- Title field is clipped at the top of the visible area
- No visible backdrop/overlay dimming — modal blends with background content
- MCPs and Tags textareas are full-height when they typically need only 1-2 lines — wastes vertical space, pushes Completion Criteria and action buttons below the fold

**Expected:**
- Modal centered horizontally and vertically with scroll if content overflows
- Semi-transparent backdrop overlay to focus attention
- MCPs and Tags should be single-line text inputs or compact textareas (max 2 rows)

### U2. Task Create Modal — Agent Combobox Overlay Bug

**Severity:** Medium (from prior session, not retested this session)
**Observed:** The "Assigned Agent" combobox uses a cmdk-based dropdown. When opened and closed, a `bg-black/50 z-[9998]` overlay remains, blocking clicks on other elements including the Create Task button. Required pressing Escape twice to dismiss.

**Expected:** Overlay should dismiss cleanly when the combobox loses focus or an option is selected.

### U3. Page Load Times — Blank Page Perception

**Severity:** Low-Medium
**Observed:** Pages (My Tasks, Workflows, Settings, Project Detail) show a blank content area for 3-6 seconds while loading. Only the sidebar renders initially. No loading skeleton or spinner in the main content area.
**Expected:** Show a loading skeleton or progress indicator in the main content area while data fetches complete. Current behavior can make the app feel broken.

### U4. Settings — i18n Missing Keys

**Severity:** Low (cosmetic)
**Observed:** ~100+ `i18next::translator: missingKey en settings ...` console warnings on every Settings page load. All settings labels render correctly (using raw English keys as fallback), but this generates excessive console noise.
**Recommendation:** Add a `settings` namespace to the i18n translation files, or disable missing key warnings for the `settings` namespace.

### U5. WebSocket Connection Errors

**Severity:** Low (non-blocking)
**Observed:** On every page load:
- `Firefox can't establish a connection to the server at ws://localhost:3000/...` (from `useExecutionEvents.ts:147`)
- `The connection to http://localhost:3000/api/nora/coordination/events/sse was interrupted` (from `useExecutionEvents.ts:174`)
- `The connection to http://localhost:3000/api/events/all was interrupted` (from `event-stream.ts:37`)

**Impact:** No visible user impact — the app functions without WebSocket/SSE connections. But fills console with errors and likely causes unnecessary reconnection attempts.

### U6. Kanban Task Card — Minimal Information

**Severity:** Low
**Observed:** Task cards on the kanban board show only: title and priority badge. No assignee, no agent, no due date, no tags visible on the card.
**Recommendation:** The "Enhanced Cards" toggle exists in the toolbar — verify it adds more detail. Consider showing agent badge by default when an agent is assigned (important for the automation workflow).

---

## What Works Well

- **My Tasks page** (after fix): Clean 4-tab filter design, sort dropdown, stats bar — well structured
- **Workflows Builder**: 5 system workflows with clean card layout. Run/Edit buttons per workflow.
- **Mission Control**: Good quick-start guidance card with clear 3-step instructions and "Go to Projects" CTA
- **MCP Settings**: 4-card grid layout is clean and shows all servers. ORCHA Task Server prominently featured.
- **Task Create Form**: Comprehensive field set — Title, Description (markdown), Priority, Board, Due Date, Assignee, Agent, MCPs, Tags, Completion Criteria, Output Format, Approval toggle, Images, Quickstart. Good placeholder text on Completion Criteria and Output Format.
- **Kanban Board**: Standard 5-column layout with proper status headers and counts. Filter/Sort toolbar is feature-rich.
- **Project Detail**: Rich overview with Brand Identity, Operational Snapshot, Integrations Hub, Workstreams, and Asset Vault sections.
- **Settings Navigation**: Well-organized sidebar with 14 categories including Developer settings for dev-mode tools.

---

## Changes Made During This Review

### Backend Fix — Route Parameter Shadowing (`tasks.rs`)

**File:** `crates/server/src/routes/tasks.rs` (lines 959-979)

**Before:**
```rust
let task_id_router = Router::new()
    .route("/", get(get_task).put(update_task).delete(delete_task))
    .route("/approve", post(approve_task))
    ...
    .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

let inner = Router::new()
    .route("/assigned-to-me", get(get_assigned_to_me))
    .route("/created-by-me", get(get_created_by_me))
    .route("/watched", get(get_watched_tasks))
    .nest("/{task_id}", task_id_router);  // ← shadowed sibling routes
```

**After:**
```rust
let task_id_routes = Router::new()
    .route("/{task_id}", get(get_task).put(update_task).delete(delete_task))
    .route("/{task_id}/approve", post(approve_task))
    ...
    .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

let inner = Router::new()
    .route("/assigned-to-me", get(get_assigned_to_me))
    .route("/created-by-me", get(get_created_by_me))
    .route("/watched", get(get_watched_tasks))
    .merge(task_id_routes);  // ← explicit routes, no shadowing
```

### Session 3 Fixes — Task Creation & WebSocket Backoff

**Fix: `created_by` override** (`crates/server/src/routes/tasks.rs`):
- Both `create_task()` and `create_task_and_start()` now set `payload.created_by = access_context.user_id.to_string()` before calling `Task::create()`

**Fix: `useJsonPatchWsStream` backoff** (`frontend/src/hooks/useJsonPatchWsStream.ts`):
- Added `MAX_RETRY_ATTEMPTS = 3` — stops reconnecting after 3 failures (was infinite)
- Added `isRealtimeDisabled()` check — respects the Developer Settings toggle
- Reduces console errors from kanban board WebSocket from infinite to max 3

**Fix: Task templates** (`crates/db/src/models/task_template.rs`):
- Changed `TEMPLATE_COLS` from macro-style annotations to plain column names

**Fix: Task form modal** (`frontend/src/components/dialogs/tasks/TaskFormDialog.tsx`):
- Widened to 650px, added `max-h-[90vh] overflow-y-auto`
- MCPs and Tags changed from Textarea to Input

### Session 4 Fixes — VIBE Bypass API Route

**Fix: System settings API route mismatch** (`frontend/src/lib/api.ts`):
- `systemSettingsApi` was calling `/api/config/system-settings` but backend route is at `/api/system-settings` (config router merged without `/config` prefix in `routes/mod.rs`)
- Changed both `getAll()` and `update()` paths from `/api/config/system-settings` to `/api/system-settings`
- **Verification:** VIBE bypass toggle in Developer Settings now works end-to-end — toggle on sets `vibe_check_bypass=true` in DB, toggle off sets it back to `false`

---

## Recommended Next Steps

### Priority 1 (Blockers)
1. ~~**Fix task templates schema** (C2)~~ — RESOLVED
2. **Verify route fix doesn't break task CRUD** — Test `GET/PUT/DELETE /api/tasks/:uuid` still works through the middleware

### Priority 2 (UX)
3. **Fix agent combobox overlay** — cmdk dropdown overlay should dismiss on blur
4. **Add loading skeletons** — Replace blank content areas during data fetch with skeleton UI

### Priority 3 (Polish)
5. **Add i18n settings namespace** — Eliminate 100+ console warnings
8. **Graceful WebSocket fallback** — Detect WS failure, suppress reconnect spam, show status indicator
9. **Task card agent badge** — Show agent assignment on kanban cards by default

---

## Test Environment

- **Browser:** Firefox (Playwright MCP automation)
- **Frontend:** Vite dev server on port 3000
- **Backend:** Rust/Axum on port 3001
- **Database:** SQLite at `dev_assets/db.sqlite`
- **Login:** admin/admin123

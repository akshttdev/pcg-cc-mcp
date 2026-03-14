# Dogfood QA Sprint — P1 Bugs + P2 UX Fixes

**Date:** 2026-03-14
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13` (continue existing)
**Status:** Items A, E, G, H implemented + DbUuid migration for activity system
**Triggered by:** E2E QA testing (see `archive/2026-03-13--review--dogfood-e2e-qa.md`)

---

## Context

During E2E QA testing, 16 bugs were found. DbUuid Phase 1-2 fixed 6 (agent endpoints). FormDialogBody + ResizableDrawer fixed 2 more (scrolling + nav). This sprint addresses the remaining P1 bugs + 2 P2 UX items + workflow trigger hardening to complete the dogfood QA cycle.

---

## Scope (8 items)

| # | Item | Type | Priority | Status |
|---|------|------|----------|--------|
| A | Task activity logging (Bug #9) | Bug fix | P1 | **DONE** — verified via Playwright |
| B | Feedback tasks no dev agent (Bug #10) | Bug fix | P1 | Not started |
| C | Manual InReview QA trigger (Bug #11) | Bug fix | P1 | Implemented, not Playwright-tested |
| D | Intelligence Overview 0 sources (Bug #6) | Bug fix | P1 | Not started |
| E | `send_notification` node + in-app notifications | Feature | P1 | **DONE** — migration, model, routes, frontend bell |
| F | Workflow trigger hardening | Hardening | P1 | Not started |
| G | Status dropdown in task detail (Backlog #10) | UX | P2 | **DONE** — verified via Playwright |
| H | Loading skeletons (Backlog #11) | UX | P2 | **DONE** — code in place, loads too fast to visually verify |

**Deferred:** MCP config unification (medium effort, not a bug)

---

## DbUuid Migration (emerged from Item A debugging)

**Root cause of Item A failure:** `uuid::Uuid` encodes as 16-byte BLOB in SQLite, but tables store UUIDs as TEXT. `ActivityLog::create()` was silently inserting 0 rows because BLOB bindings didn't match TEXT columns.

**Fix:** Migrated the entire activity logging code path from `uuid::Uuid` to `DbUuid` (which always encodes as TEXT).

### Files changed:
- `crates/db/src/models/activity.rs` — `ActivityLog` fields now `DbUuid`, `CreateActivityLog.task_id` now `String` (was `Uuid`). All query methods use `query_as::<_, ActivityLog>()` with `.bind()` instead of `query_as!` macro. Zero `uuid::Uuid` imports remaining.
- `crates/server/src/routes/activity.rs` — `TaskIdPath.task_id` now `String` (was `Uuid`). Zero `uuid::Uuid` imports.
- `crates/server/src/routes/notifications.rs` — Project ID queries use `DbUuid` (was `Vec<u8>`). Fixed pre-existing bug where BLOB/TEXT mismatch could cause 0 results.
- `crates/server/src/routes/tasks.rs` — Activity log creation uses `task.id.clone()` (String) directly, no `Uuid::parse_str` needed.
- `crates/local-deployment/src/container.rs` — Activity log creation uses `.to_string()` locally where `task_attempt.task_id` is still `Uuid`.
- `crates/nora/src/editron_tracking.rs` — All 4 public functions (`log_editron_activity`, `record_editron_vibe`, `create_and_link_artifact`, `find_or_create_task`) now accept `&str` inputs. Uuid conversion happens locally only where downstream APIs require it. `find_or_create_task` returns `(String, String)` instead of `(Uuid, Uuid)`.
- `crates/nora/src/tools.rs` — All editron tracking callers updated to pass `&str` references.

### Design principle applied:
> Push the UUID boundary as far as possible without cascading breaking changes. Inputs should be strings; UUID consumers convert locally in the calling scope.

### Broader migration scope:
Audit found **109 model files** in `crates/db/src/models/` still using `uuid::Uuid` in `#[derive(FromRow)]` structs. Only 4 models are correctly using `DbUuid`: `activity.rs`, `agent.rs`, `agent_execution_config.rs`, `notification.rs`. Full migration is a separate initiative.

---

## A. Task Activity Logging — Bug #9 ✅

**Root cause (updated):** Two issues:
1. `create_task()` and `update_task()` never called `ActivityLog::create()` — fixed by adding calls
2. `uuid::Uuid` BLOB encoding prevented inserts from matching TEXT columns — fixed by DbUuid migration

**Verification (Playwright MCP):**
- Created task "QA Test: Activity Logging Verification" → activity log row appeared in `activity_logs` table with TEXT UUIDs
- Changed status To Do → In Progress → notification bell showed "created" and "status_changed" entries
- Kanban board updated correctly (task moved columns, toast appeared)
- DB query confirmed: `SELECT * FROM activity_logs ORDER BY timestamp DESC` shows correct entries

---

## B. Feedback Tasks No Dev Agent — Bug #10

**Root cause:** `feedback.rs:103` hardcodes `agent_id: None`. Workflow trigger fires but pipeline doesn't assign.

**Status:** Not started.

**Files:**
- `crates/server/src/routes/feedback.rs` — CreateTask at line ~91, trigger at line ~156

**Steps:**
1. After task creation (line ~118): look up default dev agent (by agent_tier="dev" or short_name convention)
2. If found, update task with `agent_id = Some(agent.id)`
3. This is the hardcoded fallback — workflow pipeline should also assign (verify pipeline config)
4. Log which agent was assigned or warn if none found

---

## C. Manual InReview QA Trigger — Bug #11

**Root cause:** Only `container.rs:221-233` (automated finalization) triggers watchers. Manual status update in `update_task()` doesn't.

**Status:** Implemented in `tasks.rs` (triggers watchers on manual InReview status change). Not yet Playwright-tested.

**Files:**
- `crates/server/src/routes/tasks.rs` — `update_task()` (line ~688)
- `crates/services/src/services/qa_review.rs` — `trigger_agent_watchers()`
- `crates/db/src/models/task.rs` — watcher queries

---

## D. Intelligence Overview 0 Sources — Bug #6

**Root cause:** `pulse.rs:414-440` calls `PulseSource::find_by_project()` which ignores `organization_id`.

**Status:** Not started.

**Files:**
- `crates/server/src/routes/pulse.rs` — `dashboard_stats` (line ~414)
- `crates/db/src/models/pulse_source.rs` — `find_by_project` (line ~78)

**Steps:**
1. Add `organization_id` param to `PulseSource::find_by_project()` (or create new method)
2. Update SQL: `WHERE project_id = ? AND organization_id = ?`
3. Update `dashboard_stats` handler to extract and pass org context
4. Same for content item count queries if needed

---

## E. `send_notification` Node + In-App Notifications ✅

**Implementation:**
- Migration `20260401000000_notifications.sql` — `notifications` table with `id`, `user_id`, `organization_id`, `title`, `message`, `notification_type`, `source`, `source_id`, `read_at`, `created_at`
- Model `crates/db/src/models/notification.rs` — CRUD, `find_by_user()`, `count_unread()`, `mark_read()`, `mark_all_read()`, `delete()`
- Routes `crates/server/src/routes/notifications.rs` — `GET /notifications` (activity feed), `GET /notifications/inbox`, `GET /notifications/unread-count`, `PUT /notifications/:id/read`, `PUT /notifications/mark-all-read`, `DELETE /notifications/:id`
- Frontend `NotificationCenter.tsx` — Bell icon with unread badge, dropdown with activity-based + in-app notifications, mark all read, 30s polling
- **Click-to-read + deep linking:** Clicking any notification marks it as read and navigates to the relevant detail page:
  - Activity notifications: deep-link to `/projects/:projectId/tasks/:taskId` (extracts project_id from metadata)
  - Inbox notifications: `PUT /api/notifications/:id/read` on click, then navigates by source type (task → `/my-tasks`, workflow → `/workflows`)
  - Both handlers dismiss the notification from the unread list immediately

**Verification (Playwright MCP):**
- Notification bell visible in navbar
- Dropdown shows activity items ("created a task", "status changed")
- "Mark all read" clears items, shows "All caught up"
- [ ] Click activity notification → navigates to task detail page
- [ ] Click inbox notification → marks read via API, navigates to source page

---

## F. Workflow Trigger Hardening

**Status:** Not started.

**Current state:** Schedule loop (`spawn_workflow_schedule_loop` at `data_source_workflows.rs:1259`) and event triggers work but lack error handling.

**Steps:**
1. Error recovery: Wrap individual trigger execution in catch block
2. Missed executions: Compare `last_triggered_at + interval` vs now
3. Observability: Add structured tracing spans
4. Graceful shutdown: Respect cancellation token

---

## G. Status Dropdown in Task Detail — Backlog #10 ✅

**Verification (Playwright MCP):**
- Task detail shows combobox (not static badge) with options: To Do, In Progress, In Review, Done, Cancelled
- Changed "QA Test: Activity Logging Verification" from To Do → In Progress
- Kanban board updated: To Do count dropped 11→10, In Progress count rose 2→3
- Toast notification appeared: "Status changed to In Progress"
- Activity log entry created for status change

---

## H. Loading Skeletons — Backlog #11 ✅

**Implementation:**
- `frontend/src/pages/my-tasks.tsx` — Skeleton cards with header, tabs, and 4 task card rows matching list layout
- Uses existing `Skeleton` component from shadcn/ui

**Verification:** Code confirmed in place. Loads too fast on localhost to visually verify skeleton state in Playwright snapshots (expected — skeletons are for slower network conditions).

---

## Implementation Order (updated)

1. ~~**A** — activity logging~~ ✅
2. **D** — pulse org-scoping (backend, small isolated fix) — NOT STARTED
3. **F** — trigger hardening (backend, isolated) — NOT STARTED
4. **B** — feedback agent assignment (backend) — NOT STARTED
5. ~~**C** — manual InReview trigger~~ — Implemented, needs Playwright test
6. ~~**G** — status dropdown~~ ✅
7. ~~**H** — loading skeletons~~ ✅
8. ~~**E** — notifications~~ ✅

---

## Verification

### Automated
- [x] `flox activate -- cargo check --workspace` — passes (warnings only)
- [ ] `flox activate -- cargo test --workspace`
- [ ] `cd frontend && npx tsc --noEmit`

### Manual QA per item (Playwright MCP)

**A — Activity Logging:**
- [x] Create task → activity log shows "created"
- [ ] Update task title → activity shows "updated" with changed fields
- [x] Change status → activity shows "status_changed" with from/to

**B — Feedback Agent:**
- [ ] Submit bug report → task created with dev agent assigned
- [ ] Verify agent watchers auto-registered

**C — InReview Trigger:**
- [ ] Task with QA watcher + PR → manual InReview → watcher triggered
- [ ] Task with QA watcher, no PR → warning logged, watcher handled per requirements
- [ ] Automated InReview flow still works (no regression)

**D — Intelligence:**
- [ ] Intelligence Overview → Total Sources > 0
- [ ] Switch orgs → count reflects org sources

**E — Notifications:**
- [x] Notification bell appears in navbar with activity items
- [x] Mark all read → shows "All caught up"
- [ ] Workflow with send_notification → notification appears in bell dropdown

**F — Triggers:**
- [ ] Scheduled trigger fires on interval
- [ ] Server restart → trigger catches up, no duplicates
- [ ] Error in workflow → logged, loop continues

**G — Status Dropdown:**
- [x] Task detail → status is dropdown, not badge
- [x] Change status → toast, kanban updates

**H — Skeletons:**
- [x] My Tasks → skeleton code in place (Skeleton components)
- [ ] Agent Settings → skeleton rows during load

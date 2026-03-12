# Feature/Fraze-2026-03-12 → Main Merge Review

> **Status Update (2026-03-12):** All blockers resolved. Build verified. Runtime tested via Playwright. PR #18 open, ready for squash-merge.

> **Branch:** `origin/feature/fraze-2026-03-12` → `main` (`f12d04358`)
> **PR:** [#18](https://github.com/KingBodhi/pcg-cc-mcp/pull/18)
> **Delta:** 24 commits, 68 files changed, +4,982 / -444 lines
> **Trial merge result:** Clean auto-merge (zero git conflicts)

---

## Table of Contents

1. [Feature Inventory](#1-feature-inventory)
2. [Conflict & Risk Analysis](#2-conflict--risk-analysis)
3. [Pre-Merge Action Items](#3-pre-merge-action-items)
4. [Post-Merge Follow-ups](#4-post-merge-follow-ups)
5. [Post-Merge Verification](#5-post-merge-verification)

---

## 1. Feature Inventory

### Task System Enhancements

| Area | Summary |
|------|---------|
| **Completion criteria** | `completion_criteria` and `output_format` TEXT columns on tasks, propagated to all creation call sites (MCP, Nora, agent chat, API) |
| **Task templates** | Enhanced with `priority`, `completion_criteria`, `output_format`, `assigned_agent`, `tags`, `organization_id` |
| **My Tasks overhaul** | Filter tabs (assigned/created/watching), column layout, improved empty states |
| **Task details** | Compact toggle, agent badge on cards, completion criteria display |
| **Find by creator** | `Task::find_by_creator` + `GET /api/tasks/created-by-me` endpoint |

### Workflow System Expansion

| Area | Summary |
|------|---------|
| **7 new action nodes** | `http_request`, `send_notification`, `create_task`, `update_record`, `run_sub_workflow`, `ai_classify`, `ai_extract` |
| **4 system workflows** | Bug Triage Pipeline, Sprint Planning, Client Onboarding, Content Pipeline (seeded via migration) |
| **Schedule triggers** | `WorkflowTrigger` model + background scheduler loop |
| **Triggers panel** | `WorkflowTriggersPanel.tsx` — UI for managing workflow triggers |

### MCP & Agent Enhancements

| Area | Summary |
|------|---------|
| **3 new MCP tools** | `check_dependencies`, `unified_search`, `scaffold_project` |
| **Task server expansion** | `task_server.rs` +606 lines — major MCP tool additions |
| **Agent profiles** | `GET /api/agents/:id/profile` with capability stats |
| **Agent detail dialog** | Enhanced with profile API integration |
| **ACP harness** | MCP wiring for agent execution harness (+92 lines) |

### Frontend & Settings

| Area | Summary |
|------|---------|
| **Developer Settings** | New page with VIBE bypass toggle (dev-only) |
| **Project scaffolding** | Templates in `ProjectFormDialog` (+146 lines) |
| **Mission control** | Layout and interaction improvements |

### Infrastructure

| Area | Summary |
|------|---------|
| **System settings** | `system_settings` table (key-value store), admin API |
| **Config endpoint** | `GET /api/config` for frontend feature flags |
| **Planning conventions** | 9 planning files renamed to `YYYY-MM-DD--<type>--<topic>.md` format |
| **Seed DB** | Updated `dev_assets_seed/` with all pending migrations applied |

---

## 2. Conflict & Risk Analysis

### Git Conflicts: None

Trial merge completed cleanly. Only `task.rs` was touched on both sides (PR #17 added task UUID fix, fraze added completion criteria) — auto-merged with non-overlapping changes.

No overlap with PR #17's auth/twilio changes — fraze doesn't touch `twilio.rs`, `auth.rs`, or `auth_sqlite.rs`.

### Migration Timestamp Collision

**BLOCKER** — Both PR #17 (now on main) and fraze use `20260323000000`:

| Timestamp | Main (PR #17) | Fraze |
|-----------|---------------|-------|
| `20260323000000` | `add_sovereign_storage.sql` | `add_task_completion_criteria.sql` |
| `20260323100000` | `knowledge_source_nullable_project.sql` | — |
| `20260324000000` | — | `enhance_task_templates.sql` |
| `20260325000000` | — | `seed_system_workflows.sql` |
| `20260326000000` | — | `system_settings.sql` |

**Resolution:** Bump all fraze migrations to start after main's latest (`20260323100000`):
- `20260323000000` → `20260324000000_add_task_completion_criteria.sql`
- `20260324000000` → `20260325000000_enhance_task_templates.sql`
- `20260325000000` → `20260326000000_seed_system_workflows.sql`
- `20260326000000` → `20260327000000_system_settings.sql`

### VIBE Bypass Review

**Recommendation: Ship as-is.** Three-layer protection is adequate:

1. **Compile-time:** `cfg!(debug_assertions)` — production release builds always return `false`
2. **Runtime:** DB setting defaults to `'false'`, requires explicit admin toggle
3. **Frontend:** Toggle hidden unless `import.meta.env.MODE === 'development'`

Bypass affects 6 endpoints (agent_chat, marketplace, nora×3, tasks, topsi) — skips token balance checks. Cannot activate in production.

### `send_notification` Action Node

The workflow `send_notification` handler is a stub — logs and returns success. **Decision:** Mark as unimplemented, address post-merge.

### `http_request` Action Node

Allows workflows to make arbitrary HTTP calls with no URL allow-list. **Decision:** Ship as-is, add guardrails post-merge (see [Post-Merge Follow-ups](#4-post-merge-follow-ups)).

---

## 3. Pre-Merge Action Items

### Blockers — ALL RESOLVED

- [x] **Merge main into fraze** — Picked up PR #17 changes (sovereign storage, auth hardening, twilio rewrite, classifier). Auto-merged cleanly.
- [x] **Fix migration timestamp collision** — Renamed 4 fraze migrations to `20260324000000`–`20260327000000`
- [x] **Add missing `meeting_sessions` CREATE TABLE** — Pre-existing bug: table referenced by 4 migrations but never created. Added `20260303000000_create_meeting_sessions.sql`. Updated `20260304000000` to no-op (column already exists).
- [x] **Refresh SQLx offline cache** — `cargo sqlx prepare --workspace` completed
- [x] **Verify build** — `cargo check --workspace` (0 errors, 47 warnings pre-existing) + `tsc --noEmit` (0 errors)

### Non-Blockers — ALL VERIFIED

- [x] Manual review of `task.rs` auto-merge — both UUID hex fix (PR #17, lines 257-260) and completion criteria (fraze) intact
- [x] Confirm seed DB includes all migrations through `20260327000000`

---

## 4. Post-Merge Follow-ups

| Item | Priority | Description |
|------|----------|-------------|
| `http_request` guardrails | Medium | Add URL allow-list or domain restriction to prevent SSRF in workflow actions |
| `send_notification` implementation | Low | Wire up actual notification delivery (email/Slack/webhook) |
| Workflow scheduler hardening | Medium | Add error recovery, dead-letter handling for failed scheduled workflows |
| Task template test coverage | Low | Integration tests for enhanced template fields propagation |

---

## 5. Verification Results

Runtime testing performed on worktree build (backend :3100, frontend :3200) via Playwright MCP.

### Passed

| Test | Result |
|------|--------|
| Login + org dashboard | Loads cleanly, stats cards render (Projects: 1, Contacts: 0, etc.) |
| **My Tasks page** | Filter tabs (All/Assigned/Created/Watching) render with counts |
| **Workflow Builder** | 4 system workflows visible: Bug Triage, Client Onboarding, Content Analysis, Sprint Planning |
| **Developer Settings** | Page loads at `/settings/developer`, VIBE bypass toggle visible with description |
| **API: `GET /tasks/created-by-me`** | Returns `{success: true, data: []}` |
| **API: `GET /system-settings`** | Returns `vibe_check_bypass: false` with timestamp |
| **API: `GET /workflows/definitions`** | Returns all 5 definitions including 4 `is_system: true` workflows |
| Fresh DB migrations | All 100+ migrations apply cleanly from empty DB |
| Rust build | `cargo check --workspace` — 0 errors (47 pre-existing warnings) |
| TypeScript build | `tsc --noEmit` — 0 errors |

### Remaining (manual testing post-merge)

| Test | What to verify |
|------|----------------|
| Task creation with criteria | Create task with `completion_criteria` + `output_format` → fields persist and display |
| Task templates | Create from template → priority, criteria, agent fields populate |
| Workflow action nodes | Test each new node type: `create_task`, `ai_classify`, `ai_extract`, `http_request` |
| Schedule triggers | Create schedule trigger → verify background loop fires |
| Agent profile | `GET /api/agents/:id/profile` → returns stats |
| MCP tools | `check_dependencies`, `unified_search`, `scaffold_project` → valid responses |
| VIBE bypass | With bypass ON → agent chat/task creation works without VIBE balance |

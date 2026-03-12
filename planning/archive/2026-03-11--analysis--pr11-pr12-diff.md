# PR #11 / PR #12 Regression Review

**PR #11**: "feat: configure global MCP server settings in-app" (merged `a1c97f787e`)
**PR #12**: "chore: bump runners" (merged `2cf39a8a38`)
**Diff basis**: `git diff 2cf39a8a38..a1c97f787e` (61 files, +2696 -630 lines)
**Reviewed against**: `feature/fraze-2026-03-11` (current branch HEAD)

---

## Diff Summary

The diff between PR #12 → PR #11 introduced:
- MCP server configuration (backend routes + frontend page)
- ExecutorConfig enhancements (config_path, MCP support, display names)
- Opencode executor
- Sound file format change (MP3 → WAV)
- Execution monitor improvements
- Project/task model changes
- Various frontend UI changes

---

## File-by-File Analysis

### Frontend Files — Present on Both (no changes)

All 12 frontend files that exist on both PR #11 and current branch are **identical**:

| File | Status |
|------|--------|
| `frontend/src/App.tsx` | SAME |
| `frontend/src/components/layout/navbar.tsx` | SAME |
| `frontend/src/components/logo.tsx` | SAME |
| `frontend/src/components/projects/project-list.tsx` | SAME |
| `frontend/src/components/tasks/TaskDetailsHeader.tsx` | SAME |
| `frontend/src/components/tasks/TaskDetailsPanel.tsx` | SAME |
| `frontend/src/components/tasks/TaskDetailsToolbar.tsx` | SAME |
| `frontend/src/components/tasks/TaskFollowUpSection.tsx` | SAME |
| `frontend/src/components/theme-provider.tsx` | SAME |
| `frontend/src/components/ui/file-search-textarea.tsx` | SAME |
| `frontend/src/pages/project-tasks.tsx` | SAME |
| `shared/types.ts` | SAME |

### Backend Files — Relocated to `crates/`

All `backend/` files were moved during the crates restructure. Mapping:

| PR #11 File | Current Location | Status |
|---|---|---|
| `backend/src/executor.rs` | `crates/executors/src/executors/mod.rs` | PRESENT — enhanced |
| `backend/src/routes/config.rs` | `crates/server/src/routes/config.rs` | PRESENT — MCP routes at `/api/config/mcp-config` |
| `backend/src/execution_monitor.rs` | `crates/local-deployment/src/container.rs` + `crates/services/src/services/notification.rs` | PRESENT — refactored (see PR #13 review) |
| `backend/src/executors/gemini.rs` | `crates/executors/src/executors/gemini.rs` | PRESENT |
| `backend/src/executors/opencode.rs` | `crates/executors/src/executors/opencode.rs` | PRESENT — ~500 lines |
| `backend/src/executors/mod.rs` | `crates/executors/src/executors/mod.rs` | PRESENT |
| `backend/src/models/config.rs` | `crates/services/src/services/config/` | PRESENT |
| `backend/src/models/project.rs` | `crates/db/src/models/project.rs` | PRESENT — with org/client scoping |
| `backend/src/models/task.rs` | `crates/db/src/models/task.rs` | PRESENT |
| `backend/src/models/task_attempt.rs` | `crates/db/src/models/task_attempt.rs` | PRESENT |
| `backend/src/routes/projects.rs` | `crates/server/src/routes/projects.rs` | PRESENT |
| `backend/src/routes/task_attempts.rs` | `crates/server/src/routes/task_attempts.rs` | PRESENT |
| `backend/src/routes/tasks.rs` | `crates/server/src/routes/tasks.rs` | PRESENT |
| `backend/src/app_state.rs` | Split into service layer | PRESENT (distributed) |
| `backend/src/main.rs` | `crates/server/src/main.rs` | PRESENT |
| `backend/src/bin/generate_types.rs` | `crates/server/src/bin/generate_types.rs` | PRESENT |

### Frontend Files — Relocated or Renamed

| PR #11 File | Current Location | Status |
|---|---|---|
| `frontend/src/pages/McpServers.tsx` | `frontend/src/pages/settings/McpSettings.tsx` | PRESENT — integrated into settings layout |
| `frontend/src/pages/Settings.tsx` | `frontend/src/pages/settings/GeneralSettings.tsx` + 10+ sub-pages | PRESENT — expanded |
| `frontend/src/components/tasks/TaskFormDialog.tsx` | `frontend/src/components/dialogs/tasks/TaskFormDialog.tsx` | PRESENT |
| `frontend/src/hooks/useTaskDetails.ts` | `frontend/src/stores/useTaskDetailsUiStore.ts` + related hooks | PRESENT — refactored |
| `frontend/src/index.css` | `frontend/src/styles/index.css` | PRESENT |

### Other Files

| PR #11 File | Current Location | Status |
|---|---|---|
| `backend/sounds/*.wav` | `assets/sounds/*.wav` | PRESENT — all 7 WAV files |
| `.github/workflows/*` | Deleted (intentional) | N/A — CI removed for plurigrid deployment |
| `npx-cli/*` | Same location | PRESENT |
| `scripts/mcp_test.js` | Same location | PRESENT |

---

## Feature Verification

### 1. MCP Server Configuration — PRESENT
- Backend: `GET/POST /api/config/mcp-config` in `crates/server/src/routes/config.rs:31`
- Frontend: `frontend/src/pages/settings/McpSettings.tsx` with executor profile selector + JSON editor
- Navigation: Settings layout includes "MCP Servers" tab at `/settings/mcp`

### 2. ExecutorConfig Enhancements — PRESENT
- `supports_mcp()` at `crates/executors/src/executors/mod.rs:122`
- `default_mcp_config_path()` implemented across all executors (claude, gemini, amp, opencode, codex, cursor, duck, qwen)
- `get_mcp_config()` returns executor-specific paths and schemas

### 3. Opencode Executor — PRESENT
- `crates/executors/src/executors/opencode.rs` (~500 lines, full implementation with MCP support)

### 4. Sound Notifications — PRESENT
- WAV format files in `assets/sounds/`
- Cross-platform playback in `crates/services/src/services/notification.rs`
- macOS (afplay), Linux (paplay/aplay), Windows/WSL (powershell SoundPlayer)

### 5. Project/Task Model Changes — PRESENT
- `organization_id`, `client_id`, `folder_id`, `parent_project_id` on Project
- Task model with full status enum, assignee, agent, MCP fields

---

## Verdict

**No regressions found.** All 61 files changed between PR #12 and PR #11 are accounted for on the current branch. Features have been preserved and in many cases enhanced (MCP config expanded to more executors, settings split into dedicated sub-pages, executor trait system more comprehensive).

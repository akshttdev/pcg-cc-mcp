# 10-Day Sprint: Frontend Polish, Features, and Repo Hygiene

**Date**: 2026-03-18
**Branch**: `refactor/frontend-polish-sprint`
**Base**: `main` (post-PR #46 + PR #45)
**Status**: Planning

## Context

Post-quality sprint (PR #46 merged), post-sloperation316 integration (PR #45 merged). The codebase has strong modularity patterns but incomplete adoption. Two feature gaps (Workflow Triggers, Topsi connection status) and significant root directory clutter.

**Current metrics:**
- ~270 `any` types (188 in safe zones — dev-velocity branch already fixed these, cherry-pickable)
- ~90 inline query keys remaining (down from 318 pre-PR #46)
- ~50 files with raw `useMutation` (vs `useMutationWithToast`)
- 7 files >900 lines in frontend
- ~87 raw `fetch()` calls (vs typed API modules)
- ~120 files in repo root (docs, scripts, binaries that belong elsewhere)
- 5 query key string mismatches (factory vs usage)

**Active branches to deconflict:**
- `sloperation316-*` — defer merge to after sprint (per user)
- `refactor/dev-velocity-sprint` — **only active unmerged branch** with real overlap. Has: DbUuid Phase A (Rust), 60 unwrap fixes (Rust), 182 `any` type eliminations (TS), virtual-env split, data-sources split, route authorization. **Cherry-pick frontend work (any types, virtual-env split, data-sources split). Skip Rust work (different sprint scope).**
- All other unmerged branches (`feature/ux-sprint-3`, `modularity/sprint-3`, etc.) are stale — already squash-merged to main via PRs, branches just not deleted.

**Safe zones** (no sloperation overlap): settings pages, hooks, task components, workflows editor, TopsiWidget, nora, meeting-mode, all root-level files

---

## Workstreams

| ID | Workstream | Sprint Target |
|----|-----------|---------------|
| A | Repo Root Cleanup | Move ~100 files, remove tracked binaries |
| B | Cherry-pick dev-velocity frontend work | 182 `any` types → 0, virtual-env + data-sources splits |
| C | Query Key + Mutation Completion | ~90 → ~20 inline keys, ~15 more mutations, fix 5 mismatches |
| D | Large File Splits | TaskFormDialog + CrmDealDetailPanel (or company-profile) |
| E | Workflow Trigger Hardening | Webhook triggers, retry/error handling |
| F | Topsi Connection Status | Connection indicator + voice gating + TopsiWidget split |
| G | Remaining `fetch()` → API Module | ~20 more raw fetch calls migrated |

**Frontend/Backend split: ~75/25**

---

## Day-by-Day Plan

### Day 0 (Start): Branch Setup + Planning File

1. Create branch `refactor/frontend-polish-sprint` from `main`
2. Write this plan to `planning/2026-03-18--plan--frontend-polish-sprint.md`
3. Commit planning file

---

### Day 1: Repo Root Cleanup — Docs + Scripts

**Workstream A** — Non-destructive file reorganization

**Move ~50 markdown docs** from root → `docs/`:
- `APN-*.md`, `APN_*.md` (7 files) → `docs/apn/`
- `DEPLOYMENT*.md`, `DEPLOY-*.md` (5 files) → `docs/deployment/`
- `DOCKER_*.md`, `DOCKER-*.md` (5 files) → `docs/deployment/docker/`
- `SOVEREIGN_*.md`, `STORAGE_PROVIDER_*.md` (4 files) → `docs/sovereign-stack/`
- `ORCHA_*.md` (4 files) → `docs/architecture/`
- `TOPSI_*.md`, `VIRTUAL_ENVIRONMENT_*.md` (5 files) → `docs/core-features/`
- `VOICE_ANALYTICS_*.md` (2 files) → `docs/core-features/`
- `AUTH_QUICK_START.md`, `BACKUP_QUICKSTART.md`, etc. → `docs/getting-started/`
- Session logs → `docs/session-logs/`
- `2026-03-11_sloperation310-merge-plan.md` → `planning/archive/`
- `BOOTSTRAP-INFO.txt`, `CONNECT-NOW.txt`, `PYTHIA-MASTER-INFO*.txt` → `docs/`

**Move ~25 shell scripts** from root → `scripts/`:
- `setup-*.sh`, `start-*.sh`, `stop-*.sh`, `build-*.sh` → `scripts/`
- `connect-to-pythia.sh`, `manage_storage_provider.sh`, etc.
- `check-both.sh`, `check-network-capacity.sh`, etc.

**Move Python scripts** from root → `scripts/`:
- `add_users.py`, `apn_*.py`, `setup_*.py`, `import_sirak_data.py`, etc.

**Move misc** from root:
- `.desktop` files → `deploy/desktop/`
- `.service` files → `deploy/systemd/`

**Keep at root** (standard): `Cargo.toml`, `Cargo.lock`, `Dockerfile*`, `docker-compose*.yml`, `fly.toml`, `Makefile`, `README.md`, `CLAUDE.md`, `LICENSE`, `CODE-OF-CONDUCT.md`, `clippy.toml`, `rustfmt.toml`, `rust-toolchain.toml`, `.env*`, `.gitignore`, `package.json`, `pnpm-*`, `justfile`, `playwright.config.ts`

**Git-tracked binaries** (118MB total — add to `.gitignore`, `git rm --cached`):
- `flox.x86_64-linux.deb` (82MB)
- `test_send_message` (30MB binary)
- `pcg-transfer-20251119.tar.gz` (2.7MB)
- `TopiClips_Day000_Genesis.webp` (3.6MB)
- `app.db` (0 bytes, empty)
- `test_send_message.rs` (1.3KB test source — move to `scripts/` or `tests/`)

**Files**: ~100 moves, `.gitignore` update
**Outcome**: Root directory reduced from ~120 items to ~30 (standard project files only)

**Verification**:
- `git status` shows clean moves (not delete+create)
- `grep -r` for any hardcoded paths to moved files in Makefile, scripts, CI
- `pnpm run dev` still starts correctly
- Update any import paths in `CLAUDE.md`, `README.md`, `INDEX-OF-DOCS.md`

---

### Day 2: Cherry-pick Frontend Work from dev-velocity

**Workstream B + D**

Cherry-pick 3 commits from `refactor/dev-velocity-sprint`:
1. `8b2d05387` — eliminate all 182 `: any` type annotations (13 files)
2. `c9dbfd484` — split virtual-environment.tsx (1,282 lines → 9 files)
3. `6f433b1cf` — split data-sources.tsx (991 lines → 7 files)

**Potential conflicts**: dev-velocity was rebased on main post-PR #46, so conflicts should be minimal. If any, resolve by keeping PR #46's query key/mutation patterns and re-applying the type/split changes.

**Files affected**:
- `any` types: `client-overview.tsx`, `organization-overview.tsx`, `ProjectRow.tsx`, `ProjectsTab.tsx`, `oss-library-listener.tsx`, `pulse.tsx`, `WorkflowDetailPanel.tsx`, `ExecutorConfigForm.tsx`, `PulseWidget.tsx`, `markdown-renderer.tsx`, `OutputNodeConfig.tsx`, `pulse.ts`
- virtual-env split: `pages/virtual-environment.tsx` → `pages/virtual-environment/` (9 files)
- data-sources split: `pages/data-sources.tsx` → `pages/data-sources/` (7 files)

**Outcome**: 182 `any` types eliminated, 2 large file splits done (virtual-env + data-sources)

---

### Day 3: Query Key Mismatch Fixes + Remaining Hook Keys

**Workstream G + C**

**Fix 5 known factory/usage mismatches** (silent cache invalidation bugs):
1. `['brandProfile', orgId]` vs factory `['orgBrandProfile', orgId]`
2. `['workflowTemplates']` vs factory `['workflow-templates']`
3. Any others found during audit of `query-keys.ts` vs `useQuery` calls

**Complete hook query key migration**:
- `hooks/useAgentFlows.ts` (~6 inline keys)
- `hooks/useTaskMutations.ts` (~3 inline keys)
- Remaining scattered inline keys in hooks

**Files**: `lib/query-keys.ts`, ~8 hook files
**Outcome**: Zero query key mismatches, hooks fully using factories

---

### Day 4: Mutation Standardization — Hooks + Components

**Workstream C**

Convert remaining raw `useMutation` → `useMutationWithToast` in safe-zone files:
- `hooks/useAgentFlows.ts` (4 mutations)
- `hooks/useTaskMutations.ts` (3 mutations — complex, may need `onSuccess` callbacks)
- `pages/oss-library-listener.tsx` (4 mutations)
- `pages/data-sources.tsx` RunWorkflowFromSourceDialog (1 mutation — conditional toast)
- `components/workflows/StagingReviewPanel.tsx` (2 mutations)
- Other safe-zone components (~5-8 mutations)

**Skip criteria**: Mutations with optimistic updates or complex conditional logic stay as raw `useMutation` with `// NOTE: complex pattern — not suitable for useMutationWithToast`.

**Also**: Migrate `MembersTab` raw `fetch()` → `makeRequest` (from backlog)

**Files**: ~10-12 files
**Outcome**: Raw `useMutation` count drops by ~15-20

---

### Day 5: Large File Split — `TaskFormDialog.tsx` (1,240 lines)

**Workstream D**

1. Read file, identify logical sub-components (likely: form fields, assignee picker, tag editor, recurrence config, form actions)
2. Create `components/dialogs/task-form/` directory
3. Extract 4-5 sub-components, each 200-350 lines
4. Main `TaskFormDialog.tsx` stays as orchestrator at ~300 lines
5. Barrel export for backward compatibility

**Files**: `components/dialogs/TaskFormDialog.tsx` → `components/dialogs/task-form/`
**Outcome**: 1,240-line file → 4-5 files at ~250-300 lines each

---

### Day 6: Large File Split — `CrmDealDetailPanel.tsx` (1,202 lines)

**Workstream D**

(data-sources split already cherry-picked on Day 2)

1. Read file, identify logical sub-components (likely: deal header, activity timeline, proposal tab, deck tab, action buttons)
2. Create `components/crm/deal-detail/panel/` directory
3. Extract 3-4 sub-components
4. Main panel stays as orchestrator at ~400 lines

**Note**: Check if this file overlaps with sloperation branches. If so, defer to `company-profile.tsx` (1,218 lines) instead — confirmed safe zone.

**Files**: `components/crm/deal-detail/CrmDealDetailPanel.tsx` → split
**Outcome**: 1,202-line file → 3-4 files at ~300 lines each

---

### Day 7: Workflow Trigger Hardening — Webhook Support

**Workstream E** (Backend — 30% of sprint)

**Current state**: Event triggers + schedule triggers work. `fire_triggers_for_data_source()` fires on data source events, `spawn_workflow_schedule_loop()` polls every 5 min. Missing: webhook triggers.

**Scope**:
1. Add `webhook_secret`, `webhook_url` fields to `workflow_triggers` model
2. Add `POST /api/webhooks/:trigger_id` endpoint that validates secret + fires workflow
3. Add retry logic: store `last_error`, `retry_count`, `next_retry_at` on trigger execution failures
4. Add `trigger_executions` table for audit trail (trigger_id, started_at, status, error)
5. Rate limiting: per-trigger cooldown (prevent trigger storms)

**Files**:
- `crates/db/src/models/workflow_trigger.rs` — new fields + methods
- `crates/db/migrations/` — new migration for webhook fields + executions table
- `crates/server/src/routes/workflow_triggers.rs` — webhook endpoint
- `crates/server/src/routes/data_source_workflows.rs` — retry logic in `fire_triggers_for_data_source`

**Frontend** (small):
- Add webhook trigger type option in trigger creation UI
- Show webhook URL + secret in trigger detail

**Outcome**: Full trigger type coverage (event, schedule, webhook) with retry and audit trail

---

### Day 8: Topsi Connection Status + Voice Feature Gating

**Workstream F**

**Current state**: `TopsiWidget.tsx` (963 lines) already has `SUGGESTED_PROMPTS` with 4 prompt pills displayed. What's missing:

1. **Connection status indicator**: Show connected/disconnected/reconnecting state in TopsiWidget header
   - Use existing WebSocket/SSE connection state
   - Small colored dot (green/yellow/red) next to Topsi name
   - Tooltip with connection details

2. **Voice feature gating**: Disable voice-related prompt pills and voice input button when:
   - No microphone permission
   - Voice backend not available
   - Already in a voice session elsewhere

**Also**: TopsiWidget is 963 lines — extract prompt pills into `TopsiPromptPills.tsx`, voice controls into `TopsiVoiceControls.tsx` (~200 lines each)

**Files**: `components/topsi/TopsiWidget.tsx` → split + new components
**Outcome**: Connection indicator visible, voice features properly gated, file under 600 lines

---

### Day 9: Remaining `fetch()` Migration + Query Key Sweep

**Workstream G + C**

**Raw fetch() → typed API modules** (~20 calls in safe-zone files):
- Identify remaining `fetch(` calls outside of API modules
- Create new API modules or extend existing ones as needed
- Wire up with `makeRequest` pattern

**Final query key sweep**:
- Audit remaining ~50 inline keys in pages/components (safe zone only)
- Add any missing factories to `query-keys.ts`
- Target: inline keys under 30

**Files**: ~15 files
**Outcome**: Raw fetch reduced by ~20, inline keys under 30

---

### Day 9.5: QA Infrastructure — Claude Rules + Skills

**New deliverables**: Set up reusable QA patterns for this sprint and all future feature work.

**1. Create `.claude/skills/qa-review/SKILL.md`** — "QA Review" skill
- Runs `/check` skill first (tsc, lint, clippy, fmt)
- Runs E2E tests: `npm run test:e2e` (headless)
- Regression check: `git diff main --stat` to enumerate all changed files, then review each for:
  - Broken imports (moved files, renamed exports)
  - Stale references (hardcoded paths that should have been updated)
  - Missing barrel exports after file splits
- Uses Playwright MCP to smoke-test affected pages in a real browser:
  - Navigate to each page touched by the sprint
  - Verify no console errors, no blank pages, no missing components
  - Spot-check interactive features (dialogs, mutations, form submissions)
- Outputs: pass/fail summary table with links to failing items
- User-invocable: true

**2. Create `.claude/skills/playwright-smoke/SKILL.md`** — "Playwright Smoke Test" skill
- Takes `$ARGUMENTS` as comma-separated page paths (e.g., `/settings,/workflows,/crm`)
- For each path: navigate via Playwright MCP, take snapshot, check for console errors
- If no args: smoke-test a default set of critical pages (login, dashboard, settings, workflows, CRM, tasks)
- Reports: page, status (ok/error), console errors found
- User-invocable: true

**3. Update `.claude/rules/git-workflow.md`** — Add QA section:
```
## QA Before Merging
- Run `/check` before creating PR
- Run `/qa-review` before requesting merge to main
- For frontend changes: run `/playwright-smoke` on affected pages
- For backend changes: run `cargo test --workspace` inside flox
- Reference: existing E2E tests in `e2e/`, manual tests in `scripts/manual-tests/`
```

**4. Update `.claude/rules/frontend-standards.md`** — Add QA reference:
```
## Post-Feature QA
- After completing feature work, run `/review-frontend` on changed files
- Run `/playwright-smoke <affected-pages>` to verify rendering
- If feature involves mutations: manually trigger via Playwright MCP and verify toast + cache invalidation
```

---

### Day 10: Sprint QA + Retrospective

**Comprehensive QA phase** — run before merging final PR.

**1. Static checks:**
- `cd frontend && npx tsc --noEmit` — zero type errors
- `cd frontend && npm run lint` — zero new warnings
- `flox activate -- cargo check --workspace` — zero Rust errors
- `flox activate -- cargo clippy --all --all-targets -- -D warnings` — zero clippy warnings
- Run `/review-frontend` on all changed files
- Run `/review-rust` on all changed Rust files (if any)

**2. E2E test suite:**
- `npm run test:e2e` — run full headless suite, expect zero regressions
- `npm run test:e2e:headed` — visual verification of any flaky tests

**3. Regression check against main:**
- `git diff main --stat` — review full changeset
- For each moved file: verify no broken imports anywhere in codebase (`grep -r` old paths)
- For each split file: verify barrel exports are correct and lazy route imports still work
- For cherry-picked files: verify no merge artifacts or conflict markers

**4. Playwright MCP smoke tests:**
Use Playwright MCP tools (browser_navigate, browser_snapshot, browser_console_messages) to manually verify:
- Login page loads, login with admin/admin123 works
- Dashboard/overview loads with no errors
- Settings pages load (AgentSettings, KeysSettings, WalletSettings — mutation changes)
- Workflows page loads, workflow editor opens
- CRM pipeline view loads
- Data sources page loads (split file)
- Task kanban loads, task form dialog opens (split file)
- Topsi widget opens, connection status indicator visible, prompts displayed
- Trigger a mutation (e.g., create task) → verify toast appears + data refreshes
- If webhook triggers implemented: test with `curl -X POST` to webhook endpoint

**5. Documentation wrap:**
- Update `planning/BACKLOG--remaining-work.md` with sprint results
- Archive sprint planning file to `planning/archive/`
- Update `CLAUDE.md` if any moved file paths affect documented commands
- Write sprint retrospective notes

---

## Sprint Metrics

| Metric | Before | Target | Stretch |
|--------|--------|--------|---------|
| Root dir file count | ~120 | **~30** | ~25 |
| Git-tracked binaries | 118MB | **0** | 0 |
| `: any` types | ~270 | **~80** | 0 (if cherry-pick clean) |
| Inline query keys | ~90 | **~30** | ~20 |
| Raw `useMutation` (convertible) | ~50 | **~30** | ~25 |
| Files >900 lines (FE) | 7 | **3** | 2 |
| Webhook trigger support | No | **Yes** | Yes + UI |
| Topsi connection status | No | **Yes** | Yes |
| Query key mismatches | 5 | **0** | 0 |

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Cherry-pick conflicts (dev-velocity → current main) | Day 2 delay | Manual re-apply if conflicts; same changes, different base |
| Root dir moves break CI/scripts | Build failure | `grep -r` for hardcoded paths before committing |
| Tracked binary removal inflates repo size history | None (git history) | `.gitignore` prevents re-add; `git filter-branch` is optional future cleanup |
| Sloperation branches drift further | Merge conflict later | Sprint avoids all sloperation overlap zones |
| Webhook triggers need auth design | Security gap | Use HMAC secret validation (standard pattern) |
| TopsiWidget split breaks voice state | Runtime error | Lift state to parent, pass as props |

## PR Strategy — 3 PRs Max

All work accumulates on a single branch `refactor/frontend-polish-sprint`. PRs are batched to minimize QA overhead:

**PR 1 (Days 1-4)** — `chore+refactor: repo cleanup, any types, query keys, mutations`
- Repo root reorganization + binary removal
- Cherry-pick `any` type fixes + virtual-env + data-sources splits
- Query key mismatches + hook key migration
- Mutation standardization
- QA: `tsc --noEmit` + `lint` + verify app loads + spot-check mutations

**PR 2 (Days 5-8)** — `feat+refactor: file splits, webhook triggers, Topsi connection`
- TaskFormDialog split + CrmDealDetailPanel split
- Webhook trigger backend + frontend
- Topsi connection status + voice gating + TopsiWidget split
- QA: `tsc --noEmit` + `lint` + `cargo check` + test webhook with curl + verify Topsi indicator

**PR 3 (Days 9-10)** — `refactor+docs: fetch migration, QA infrastructure, sprint wrap`
- Remaining fetch → API module migration
- Final query key sweep
- New QA skill (`qa-review`) + Playwright smoke skill (`playwright-smoke`)
- Updated coding rules (git-workflow, frontend-standards) to reference QA in feature workflow
- Full QA pass: `/check` + E2E tests + regression diff vs main + Playwright MCP smoke tests
- Backlog update + retrospective

## Verification

After each day:
1. `cd frontend && npx tsc --noEmit` — zero type errors
2. `cd frontend && npm run lint` — zero new warnings
3. `flox activate -- cargo check --workspace` — zero Rust errors (Days 7+)
4. `pnpm run dev` — app loads, navigate to affected pages
5. Day 1: verify no broken imports/paths after moves
6. Day 2: verify type-check passes with cherry-picked fixes
7. Day 7: test webhook trigger end-to-end with curl
8. Day 8: verify connection indicator in TopsiWidget
9. Day 10: full E2E suite + Playwright MCP smoke tests on all affected pages

## Key Reference Files

- `frontend/src/lib/query-keys.ts` — 36+ factory objects
- `frontend/src/hooks/useMutationWithToast.ts` — target mutation pattern
- `frontend/src/lib/api/*.ts` — typed API modules (target for fetch migration)
- `crates/db/src/models/workflow_trigger.rs` — trigger model (webhook additions)
- `crates/server/src/routes/workflow_triggers.rs` — trigger CRUD routes
- `crates/server/src/routes/data_source_workflows.rs` — `fire_triggers_for_data_source()`
- `frontend/src/components/topsi/TopsiWidget.tsx` — Topsi widget (connection status + split)
- `refactor/dev-velocity-sprint` branch — cherry-pick source for `any` types + virtual-env + data-sources splits
- `.claude/skills/check/SKILL.md` — existing full project check skill
- `.claude/skills/review-frontend/SKILL.md` — existing frontend review skill
- `.claude/rules/git-workflow.md` — git workflow rules (add QA section)
- `.claude/rules/frontend-standards.md` — frontend standards (add QA section)
- `e2e/` — existing E2E test suites
- `scripts/manual-tests/` — manual test scripts

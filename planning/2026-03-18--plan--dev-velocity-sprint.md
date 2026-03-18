# Dev Velocity Sprint — 10 Days

**Date**: 2026-03-18
**Branch**: `refactor/dev-velocity-sprint`
**Base**: `main` (post-PR #45 merge)
**Status**: In Progress

## Context

Post-sloperation316 merge (PR #45), the codebase has ~251 files of new CRM/sovereign-stack features layered on top of 5 modularity sprints and a tech debt sprint. Current metrics show significant remaining friction:

- **404 `.unwrap()` calls** in non-test Rust code (40 eliminated in PR #34, ~364 remain)
- **188 `: any` type usages** in frontend (masks type errors, slows refactoring)
- **67 files** with inline query keys (cache invalidation is fragile)
- **50 files** with raw `useMutation` (no standardized error/success handling)
- **10 files** >900 lines in frontend (merge conflict magnets)

**Parallel branch**: `refactor/quality-sprint-dialogs-modularity` (PR #46, about to merge) has work on query keys, mutations, MeetingMode/NoraAssistant/project-tasks splits. We work in non-conflicting areas:
- **Skip**: NoraAssistant split, project-tasks split (already done in PR #46)
- **Skip**: Query key work in org-profile/sidebar (already done in PR #46)
- **Non-conflicting**: DbUuid (Rust only), unwrap elimination (Rust only), `any` types (different TS files), route authz (Rust only), sovereign hardening (Rust only), virtual-environment + data-sources splits (untouched by PR #46, though data-sources.tsx has minor PR #46 changes — split after merge)

## Workstreams (Priority Order)

### A. DbUuid Phase A — `AccessContext.user_id` migration (Days 1-2)

**Why highest ROI**: Every authenticated request flows through `AccessContext`. Currently `Uuid` type forces ~36 `.to_string()` / `.as_bytes()` conversions scattered across 18 route files. This is the #1 source of BLOB/TEXT binding bugs (caused 3 issues in PR #45 alone).

**Scope**:
1. Change `AccessContext { user_id: Uuid }` → `AccessContext { user_id: DbUuid }`
2. Update `build_access_context` in `middleware/access_control.rs`
3. Update `orcha_auth.rs` construction site
4. Update model functions that accept `Uuid` user_id to accept `&str`:
   - `Organization::get_user_role`, `find_by_user`, `add_member`, `remove_member`
   - `User::set_wallet_address`, `set_home_project`
   - `BoardShare::check_user_share_access`
5. Remove `.to_string()` at ~55 route handler call sites (use `.as_str()` or deref)
6. Use `bind_uuid_blob(&self.user_id)` for BLOB column bindings in access_control.rs
7. Remove `DbUuid::from(access_context.user_id)` bridge calls (~2 sites)

**Files**:
- Core: `middleware/access_control.rs`, `middleware/orcha_auth.rs`
- Models: `db/src/models/user.rs`, `db/src/models/board_share.rs`
- Routes (~17 files): `organizations/{mod,members,brand,knowledge}.rs`, `tasks.rs`, `projects.rs`, `clients.rs`, `board_shares.rs`, `project_folders.rs`, `wallet.rs`, `notifications.rs`, `invitations.rs`, `org_invitations.rs`, `org_cloud.rs`, `agents.rs`, `topsi/{mod,admin,chat}.rs`, `nora/chat.rs`, `orcha.rs`, `apn_data.rs`, `project_controllers.rs`, `sidebar.rs`, `agent_chat.rs`, `intake/handlers.rs`
- Middleware: `model_loaders.rs`

**Risk**: Medium — validate with `cargo check --workspace` + login flow smoke test
**Ref**: `planning/2026-03-17--plan--dbuuid-migration.md`

### B. Unwrap Elimination — Round 2 (Days 2-3)

**Target**: 404 → ~320 (eliminate ~80 highest-risk unwraps)

**Scope** (top files by unwrap count):
1. `crates/db/src/models/*.rs` — ~50 `serde_json::to_string().unwrap()` in model serialization
2. `crates/nora/src/` — ~15 unwraps in tool execution, voice, crawler
3. `crates/server/src/mcp/task_server/*.rs` — ~12 unwraps in split MCP modules
4. `crates/server/src/sovereign_stack.rs` — ~5 unwraps in new code from PR #45
5. `crates/topsi/src/` — ~8 unwraps in platform data

**Approach**: Extend existing `json_str`/`json_value` helpers from PR #34.

### C. `any` Type Elimination — Systematic Pass (Days 3-5)

**Target**: 188 → ~60 (eliminate ~128)

**Scope**:
1. `client-overview.tsx` (~20 `any` casts)
2. CRM deal/pipeline components (~15)
3. Route handler response types (~10)
4. Remaining scattered (~30)

**Note**: Avoid files touched by PR #46 until it merges.

### D. Fine-Grained Route Authorization (Days 5-6)

**Scope**:
1. `routes/communications.rs` — add org/project membership checks
2. `routes/crm_deals.rs` — add org membership checks to deal action endpoints
3. `routes/org_cloud.rs` — audit existing pattern for completeness

### E. Large File Splits — 2 Remaining Files (Days 6-7)

1. `virtual-environment.tsx` (1,282 lines) → `pages/virtual-environment/`
2. `data-sources.tsx` (991 lines) → `pages/data-sources/` (after PR #46 merges)

### F. Sovereign Stack Hardening (Days 8-9)

1. `CancellationToken` for `SovereignStackService::start()`
2. `tracing::warn!` on `Manifest::load()` parse errors
3. Extract `resolve_volume_path` to shared utility
4. Add missing FK constraints in `org_cloud` migration

### G. DbUuid Phase B — CRM route handlers (Day 9)

`Path<Uuid>` → `Path<String>` in ~30 CRM/deal route handlers.

### H. Planning & Documentation Cleanup (Day 10)

Update backlog, archive completed docs, write retrospective.

## Daily Schedule

| Day | Workstream | Deliverable |
|-----|-----------|-------------|
| 1 | A: DbUuid Phase A | `AccessContext.user_id` → `DbUuid`, model fns updated |
| 2 | A+B: Finish DbUuid + start unwrap elimination | Phase A complete, model unwraps started |
| 3 | B+C: Finish unwraps + start `any` elimination | ~80 unwraps fixed, `any` pass in CRM/pipeline |
| 4 | C: Continue `any` elimination | client-overview, deal components typed |
| 5 | C+D: Finish `any` + start route authz | ~128 `any` removed, communications authz |
| 6 | D+E: Finish authz + virtual-environment split | CRM deal authz, virtual-environment → directory |
| 7 | E+F: data-sources split + sovereign hardening | data-sources → directory, cancellation token |
| 8 | F: Finish sovereign hardening | Shared volume resolver, FK constraints |
| 9 | G: DbUuid Phase B | `Path<Uuid>` → `Path<String>` in CRM handlers |
| 10 | H: Docs + retro | Backlog updated, sprint retrospective |

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| `.unwrap()` (non-test Rust) | 404 | ~320 |
| `: any` (frontend) | 188 | ~60 |
| Files >900 lines | 10 (7 after PR #46) | 5 |
| Unauthz'd route handlers | ~17 | 0 |
| BLOB/TEXT binding bugs | Recurring | Eliminated for AccessContext |

## Verification

After each workstream:
- `flox activate -- cargo check --workspace` (Rust)
- `cd frontend && npx tsc --noEmit` (TypeScript)
- Login flow smoke test (after DbUuid Phase A)

## Decisions

- Base from `main` (post-PR #45 merge). Rebase after PR #46 merges.
- `data-sources.tsx` split deferred until PR #46 lands (minor changes in that PR).
- All Rust work is non-conflicting with PR #46 (frontend-only).

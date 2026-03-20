# Tier 1 Phase 0 Sprint — Foundation & Dogfood Readiness

**Date**: 2026-03-19
**Branch**: `feature/2026-03-19--tier1-phase0`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `main` (commit `ed66d568b`)
**Status**: IN PROGRESS — PRs #53, #54, #55 open for review
**Goal**: Fix regressions, harden CI, build Agent Flow Engine foundation, enable internal dev workflow dogfooding
**Team**: 2-3 devs + Claude Code
**Duration**: Flexible (ship when done)

---

## Context

ORCHA is in late alpha with functional CRM, task management, workflows, and 9 AI executors. The platform has **155K LOC frontend + 44K LOC backend**, 215 migrations, 4 MCP servers, and 19+ workflow node types. However:

1. **Agent Flow Engine doesn't exist** — data models and CRUD routes are complete but no background worker drives autonomous agent progression. This is the **#1 Stage 0 exit blocker**.
2. **CI is broken** — `cargo fmt`, clippy, and tests all fail. `continue-on-error: true` hides failures.
3. **PR #50 introduced regressions** — kanban access control removed, `uuid::Uuid` in new code.
4. **Cost dashboard shows $0** — `TokenUsage.cost_cents` is never populated; `VibeTransaction` has real data but dashboard doesn't read it.
5. **No graceful shutdown** — bare `axum::serve()`, CancellationTokens created but never signaled.
6. **Concurrent webhook double-fire** — cooldown check is non-atomic.

This sprint fixes what's broken, hardens the safety net, then builds the agent engine foundation.

---

## Architectural Patterns (applied during implementation)

These aren't separate items — patterns to apply *while* implementing sprint items:

### A. BackgroundWorker Trait
```rust
// crates/server/src/workers/mod.rs
pub trait BackgroundWorker: Send + 'static {
    fn name(&self) -> &'static str;
    fn interval(&self) -> Duration;
    async fn tick(&self, pool: &SqlitePool) -> Result<(), anyhow::Error>;
}

pub fn spawn_worker<W: BackgroundWorker>(worker: W, pool: SqlitePool, shutdown: CancellationToken) {
    tokio::spawn(async move {
        let mut ticker = interval(worker.interval());
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = worker.tick(&pool).await {
                        tracing::error!("{} error: {}", worker.name(), e);
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("{} shutting down", worker.name());
                    break;
                }
            }
        }
    });
}
```

### B. Centralized Shutdown Registry
```rust
pub struct ShutdownRegistry {
    tokens: Vec<(String, CancellationToken)>,
    root: CancellationToken,
}
impl ShutdownRegistry {
    pub fn register(&mut self, name: &str) -> CancellationToken {
        let token = self.root.child_token();
        self.tokens.push((name.to_string(), token.clone()));
        token
    }
    pub fn shutdown_all(&self) { self.root.cancel(); }
}
```

### C. DomainEvent Broadcast
```rust
// crates/server/src/events/mod.rs
pub enum DomainEvent {
    TaskStatusChanged { task_id: String, old_status: String, new_status: String },
    FlowPhaseAdvanced { flow_id: String, phase: String },
    WorkflowCompleted { workflow_id: String, run_id: String },
    AgentResponseReceived { flow_id: String, response: AgentResponseEnvelope },
}
```
Use `tokio::sync::broadcast` (already used in Nora's `EventBroadcaster`). Pattern: `emit_and_persist()` — broadcast in-memory AND write to DB (follows `ExecutionEngine::emit_flow_event()` at `nora/src/execution/engine.rs:274-309`).

### D. StructuredOutput Trait
```rust
pub trait StructuredOutput {
    fn status(&self) -> ResponseStatus;
    fn artifacts(&self) -> &[ArtifactReference];
    fn needs_clarification(&self) -> Option<&ClarificationRequest>;
}
```

### E. Progressive Validation
Add `validator` to workspace deps. Apply `#[derive(Validate)]` to new structs only: `SubmitFeedbackRequest`, `CreateFlowEvent`, `CreateAgentFlow`.

---

## Sprint Items

### 1. Fix PR #50 Regressions + Access Control Gaps [CRITICAL — Day 1]
**Effort**: 1 day | **PR**: #51

**~~1a. Harden kanban access control~~** — ALREADY DONE (all 20 handlers in `crm_deals.rs` confirmed to have `Extension(access_context)` + org membership checks; retargeted to contacts/pipelines in #1d/#1e)

**1b. Fix `uuid::Uuid` in new code** (`crm_deals.rs`)
- **Verified**: all 6 instances still present (L20 import, L895, L1011, L1191, L1194, L1195 — BLOB-column bindings for `business_reports`)
- L895, L1011: convert `uuid::Uuid::parse_str()` → `DbUuid::parse()` (person/company IDs)
- L1188-1195: 3 instances used for BLOB-column binding (`business_reports.id`/`crm_deal_id` are BLOB) — convert where possible, add comment for Phase C remainder
- L20: remove `use uuid::Uuid` import after conversions
- Scope: route-handler-level only. BLOB-binding deferred to DbUuid Phase C if model signatures need changing.

**1d. Add access control to `crm_contacts.rs`** (12 endpoints, ZERO checks)
- Extract `require_org_membership()` as method on `AccessContext` in `middleware/access_control.rs`
- Add `Extension(access_context)` + membership checks to all handlers

**1e. Add access control to `crm_pipelines.rs`** (13 endpoints, ZERO checks)
- Same approach — use shared `require_org_membership()` from AccessContext

**1f. Split `crm_deals.rs` (2,923 lines)** — do FIRST, before other changes
- Extract stage transitions (~800 lines) → `crm_deal_transitions.rs`
- Extract research/proposal/deck triggers (~400 lines) → `crm_deal_automations.rs`
- Core CRUD stays in `crm_deals.rs` (~1,700 lines)

### 2. Fix CI Pipeline [HIGH — Day 1-2]
**Effort**: 1.5 days | **PR**: #51

**Strategy**: Format only files modified in sprint. Pre-commit hooks (#3) enforce incrementally on future commits.

**2a. `cargo fmt`** — modified files only, not `cargo fmt --all`
**2b. Fix clippy** — `discord-bots` (29), `utils` (23), `cli` (65). Use `#![allow(clippy::uninlined_format_args)]` at crate root for bulk non-critical lints.
**2c. Fix `alpha-protocol-core` test compile errors** — 2 borrow checker issues
**2d. Fix ESLint** — `eslint --fix` on modified files only
**2e. Remove `continue-on-error: true`** from CI lines 48, 65 (clippy + tests). Keep on lines 119, 129 (security audits).
**2f. Add `vite build` step** to CI frontend-check job

**CI fmt strategy**: Modify existing `rustfmt.toml` (already has `reorder_imports`, `group_imports`, `imports_granularity`) to add `ignore` list for vendored crates (`vendor/serenity-voice-model`). Use `#[rustfmt::skip]` for pre-existing violations outside sprint scope. Verify `cargo fmt --check` passes on main first — if it doesn't, determine scope of pre-existing violations before committing to full-codebase check.

### 3. Apply Lint-Staged Config [QUICK WIN — Day 1]
**Effort**: 0.25 days | **PR**: #51

Selectively restore config from stash (NOT cherry-pick — stash uses `git checkout stash@{0} -- <file>`):
- `.githooks/pre-commit`, `frontend/package.json` (lint-staged + simple-import-sort deps), `frontend/.eslintrc.cjs`
- Discard: 654 auto-reformatted source files
- ESLint max-warnings: accept 110→180 bump; re-evaluate at sprint end
- `git config core.hooksPath .githooks` to activate
- **Verified**: `stash@{0}` = "lint-staged + import-sort setup + eslint --fix". Contains `.githooks/pre-commit` (+5), `frontend/package.json` (+7/-), `frontend/.eslintrc.cjs` (+4/-) plus 654 auto-reformatted files to discard
- Run `pnpm install` after restoring `frontend/package.json`

### 4. Cooldown Race Condition Fix (S0-05) [HIGH — Day 2]
**Effort**: 0.5 days | **PR**: #52

**Model fix**: `crates/db/src/models/workflow_trigger.rs` — new atomic method:
```sql
UPDATE workflow_triggers
SET last_triggered_at = datetime('now'), trigger_count = trigger_count + 1
WHERE id = ? AND (last_triggered_at IS NULL
  OR (julianday('now') - julianday(last_triggered_at)) * 86400 >= cooldown_seconds)
RETURNING *
```
- New method: `try_claim_trigger(pool, trigger_id) -> Option<WorkflowTrigger>`
- Note: existing `is_past_cooldown()` (lines 272-293) is an in-memory check — the race is TOCTOU between this check and the spawn

**Caller fix**: `crates/server/src/routes/data_source_workflows.rs:1347`
- Replace `if !trigger.is_past_cooldown() { continue; }` with `try_claim_trigger()` atomic call
- Run `cargo sqlx prepare --workspace` after adding new query

### 5. Graceful Shutdown + Worker Registry (S0-12 + Arch A/B) [HIGH — Day 2-3]
**Effort**: 1.5 days | **PR**: #52

**5a. Signal handler** — `main.rs` line 586
```rust
axum::serve(listener, app_router)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```
Configurable drain timeout: env `SHUTDOWN_DRAIN_TIMEOUT_SECS` (default 30s, higher for LLM streams).

**5b. Shutdown registry** — new `workers/mod.rs`
- Wire existing tokens: `sovereign_stack_shutdown` (L178), `schedule_shutdown` (L348) — created but NEVER signaled
- Register all 16 background operations (14 tokio spawns + 1 APN subprocess + 1 PR Monitor)

**5c. BackgroundWorker trait** (Arch A)
- Migrate `spawn_workflow_schedule_loop` and `spawn_automation_loop` as first adopters
- Agent flow executor (#9) will be third adopter

**5d. APN Node subprocess cleanup** — call `kill_existing_apn_nodes()` in shutdown handler

### 6. Bridge Dual Cost System (S0-02) [HIGH — Day 3]
**Effort**: 1 day | **PR**: #54

Redirect dashboard from dead `token_usage` table to `vibe_transactions` (source of truth).

- Add org-scoped cost aggregation endpoints to `vibe_treasury.rs`:
  - `GET /api/vibe/costs/summary?org_id=&days=`
  - `GET /api/vibe/costs/daily?org_id=&days=`
  - `GET /api/vibe/costs/by-model?org_id=&days=`
  - `GET /api/vibe/costs/by-project?org_id=&days=`
- **Access control**: `require_org_membership()` on all cost endpoints (org-scoped, NOT project-scoped `require_editor()`). [DEPENDENCY: needs #1d from PR #51 to extract this to AccessContext — PR #54 merges after #51]
- Add GROUP BY queries to `vibe_transaction.rs` model
- Create `frontend/src/lib/api/costs.ts` — dedicated cost API module (don't extend vibeApi in workflows.ts)
- Extract `ai-usage.tsx` (518 lines) tab components into separate files during refactor
- Update extracted tab components + `TokenUsageWidget.tsx` to call new cost API
- Run `cargo sqlx prepare --workspace` after adding new queries
- Run `npm run generate-types` if new response structs added

### 7. Structured Response Protocol (S0-03 + Arch D) [FOUNDATION — merged with #9]
**Effort**: Combined with #9 | **PR**: #55

- `crates/db/src/models/agent_response.rs` — `AgentResponseEnvelope`, `AgentResponseStatus` (Done/NeedsClarification/Failed/InProgress), `ClarificationRequest`, `ResponseMetrics`
- `#[derive(TS)]` for frontend type generation
- Run `npm run generate-types` after creating types

### 8. Clarification as First-Class Status (S0-04) [FOUNDATION — Day 4]
**Effort**: 0.5 days | **PR**: #55

- Add `NeedsClarification` as 8th variant to `FlowStatus` in `agent_flow.rs`
- Migration `20260413000001`: add `clarification_request` TEXT column to `agent_flows`
- New endpoint: `POST /agent-flows/{id}/respond-clarification`
- Frontend: render clarification form when flow is in this status
- Run `cargo sqlx prepare --workspace` after migration

### 9. Agent Flow Orchestration Engine — Phase 1 (S0-06 + Arch A/B/C) [KEYSTONE — Day 4-8]
**Effort**: 5 days | **PR**: #55

**New file**: `crates/server/src/agent_flow_executor.rs` — implements `BackgroundWorker`, polls every 15s.

**Config**:
```rust
pub struct AgentFlowExecutorConfig {
    pub enabled: bool,           // ENABLE_AGENT_FLOW_ENGINE (default: false)
    pub poll_interval_secs: u64, // AGENT_FLOW_POLL_INTERVAL (default: 15)
    pub max_concurrent: usize,   // AGENT_FLOW_MAX_CONCURRENT (default: 3)
    pub cooldown_secs: u64,      // AGENT_FLOW_COOLDOWN (default: 60)
}
```

**FSM** — caller-aware guards (strict for agents, permissive for dashboard users):
```
Planning → Executing → Verifying → Completed
Any phase → NeedsClarification → user responds → resume
Any phase → Failed | Paused
user_override → any (bypasses FSM)
```

**Transition table** (ESA matrix):
```
              | Planning  | Executing | Verifying | NeedsClar | AwaitAppr | Paused | Completed | Failed |
phase_done    | →Executing| →Verifying| →Completed| —         | —         | —      | —         | —      |
need_clarify  | →NeedsClar| →NeedsClar| →NeedsClar| ∅         | —         | —      | —         | —      |
respond_clar  | —         | —         | —         | →resume   | —         | —      | —         | —      |
request_appr  | —         | —         | →AwaitAppr| —         | —         | —      | —         | —      |
approve       | —         | —         | —         | —         | →Completed| —      | —         | —      |
reject        | —         | —         | —         | —         | →Executing| —      | —         | —      |
error         | →Failed   | →Failed   | →Failed   | →Failed   | →Failed   | →Failed| —         | —      |
pause/resume  | →Paused   | →Paused   | →Paused   | —         | —         | →prev  | —         | —      |
user_override | →any      | →any      | →any      | →any      | →any      | →any   | →any      | →any   |
```

**Implementation** in `agent_flow.rs`:
```rust
impl FlowStatus {
    pub fn valid_agent_transitions(&self) -> &[FlowStatus] { /* strict mode */ }
    pub fn can_transition_to(&self, target: &FlowStatus) -> bool { /* check */ }
}

pub async fn transition_validated(pool, id, target_status, target_phase) -> Result<Self, AgentFlowError> {
    // Used by executor (strict). Dashboard uses transition_to_phase() directly (permissive).
}
```

**Agent dispatch** — hybrid with configuration:
- **API dispatch** (default): `pcg_router.rs` → `route_completion()` (OpenRouter-compatible). Best for research/analysis.
- **Executor dispatch**: `ContainerService::start_execution()` (container.rs:573) → CLI process in git worktree. Best for coding.
- `flow_config.dispatch_mode: "api" | "executor" | "auto"` (default: "api")
- Both parse output into `AgentResponseEnvelope`

**CRM Deal FSM prototype** — same pattern, embedded in this sprint:
- `CrmDeal::move_to_stage()` (crm_deal.rs:405-537) currently has NO stage ordering validation
- Add `valid_stage_transitions()` to `CrmPipelineStage` — validates within pipeline
- Agent-driven: strict. Dashboard: permissive.

**Phase 1 scope**:
1. Poll `agent_flows` with `status IN ('planning', 'executing', 'verifying')`
2. Dispatch via API or executor based on `flow_config.dispatch_mode`
3. Parse response into `AgentResponseEnvelope`
4. Advance phases via `transition_validated()` (agent) or `transition_to_phase()` (dashboard)
5. Emit events via `emit_and_persist()` (broadcast + DB insert)
6. Handle approval gates
7. **Test harness**: mock dispatcher that returns canned `AgentResponseEnvelope` for verification without requiring live LLM

**Wire into main.rs**:
```rust
let flow_shutdown = shutdown_registry.register("agent_flow_executor");
agent_flow_executor::spawn(pool.clone(), flow_shutdown);
```

**Reuse**: `AgentFlow::find_by_status()`, `transition_to_phase()`, `AgentFlowEvent::emit_phase_started()`, `TaskScheduler` (dead code — adapt `RwLock` concurrency pattern), `spawn_workflow_schedule_loop()` (proven interval pattern).

**Phase 2 note**: Inline stage-trigger agents (Scout, Astra, Cash, Lux) in `crm_deals.rs` coexist with engine in Phase 1. Phase 2 migrates them to engine for retry/timeout/observability. NOT modified this sprint.

### 10. Dogfood Friction Logging (S0-19) [DOGFOOD — Day 6-7]
**Effort**: 1.5 days | **PR**: #53

Extend `crates/server/src/routes/feedback.rs`:
- `friction_area: Option<String>` — enum: workflow_editor, crm_pipeline, task_management, etc.
- `workflow_step: Option<String>`, `workaround: Option<String>`, `feature_request: Option<String>`, `source: Option<String>` ("dogfood" auto-tag)
- Apply `#[derive(Validate)]` with length limits
- Migration `20260413000002`: add friction columns to feedback table (or use JSON metadata — no migration needed)
- Frontend: "Report Friction" in sidebar More popover

---

## PR Strategy

> **Note**: GitHub PR #51 and #52 were burned during workflow setup. Plans #51-#53 are combined into GitHub PR #53. Numbers are in sync from #54 onward.

| GitHub PR | Plan Items | Branch | Status |
|-----------|-----------|--------|--------|
| **#53** | Plan #51 + #52 + #53 (CI, access control, stability, friction) | `pr/51-ci-regressions-access-control` | OPEN |
| **#54** | Plan #54 (cost bridge) | `pr/54-cost-bridge` | OPEN |
| **#55** | Plan #55 (agent engine) | `pr/55-agent-engine` | OPEN |

Merge order: #53 first (foundation), #54 and #55 can follow in either order. #55 merges last.

---

## Execution Schedule

| Day | Dev 1 | Dev 2 | Dev 3 (Claude) |
|-----|-------|-------|----------------|
| 1 | #1 (regressions) | #2a-2b (fmt + clippy) | #3 (lint-staged) |
| 2 | #4 (cooldown race) | #2c-2e (test fixes + CI strict) | #5a-5b (shutdown + registry) |
| 3 | #6 (cost bridge) | #5c (worker trait) | #7 (response protocol) |
| 4 | #7 cont'd | #8 (clarification status) | #9 start (agent engine) |
| 5-7 | #9 (agent engine) | #10 (friction logging) | #9 (agent engine) |
| 8 | QA + verification | E2E tests | Bug fixes |

---

## Verification

### Per-item checks
1. `GET /api/crm/deals/kanban/:id` without org membership → 403
2. Push branch → all CI jobs pass (fmt, clippy, test, frontend-check, types-check)
3. Stage `.tsx` with bad formatting → pre-commit auto-fixes
4. 2 concurrent webhook requests within cooldown → only 1 fires
5. `kill -SIGTERM <pid>` → drain message logged, workers shut down cleanly
6. Run agent task → `ai-usage` dashboard shows non-zero cost
7. Serialize/deserialize `AgentResponseEnvelope` → round-trips correctly
8. Agent returns `NeedsClarification` → flow pauses → UI shows question → user responds → resume
9. Create task → assign agent → engine progresses: planning → executing → verifying → completed
10. Submit friction report → stored with metadata

### End-to-end dogfood test
1. Login → create task → assign to agent
2. Engine picks up → spawns planner → planner returns `Done` → advances to executing
3. Executor works → returns `Done` with artifacts → verifier checks → flow completes
4. Cost dashboard shows $ spent
5. User submits friction report → captured with metadata

### CI commands
```bash
flox activate -- cargo fmt --all -- --check
flox activate -- cargo clippy --all --all-targets -- -D warnings
flox activate -- cargo test --workspace
cd frontend && npm run lint && npx tsc --noEmit
npm run generate-types:check
FRONTEND_PORT=3000 npx playwright test --reporter=list
```

### Build steps not to forget
- `cargo sqlx prepare --workspace` after any new/modified SQL queries (#4, #6, #8)
- `npm run generate-types` after new `#[derive(TS)]` types (#7, #8)
- `npm install` / `pnpm install` after `package.json` changes (#3)

---

## Files to Create/Modify

| File | Action | Item | PR |
|------|--------|------|-----|
| **PR #51 — CI + Regressions + Access Control** |
| `crates/server/src/routes/crm_deals.rs` | Split + access control + DbUuid | #1 | #51 |
| `crates/server/src/routes/crm_deal_transitions.rs` | Create — extracted stage transitions | #1f | #51 |
| `crates/server/src/routes/crm_deal_automations.rs` | Create — extracted automations | #1f | #51 |
| `crates/server/src/routes/crm_contacts.rs` | Add access control to 12 endpoints | #1d | #51 |
| `crates/server/src/routes/crm_pipelines.rs` | Add access control to 13 endpoints | #1e | #51 |
| `crates/server/src/middleware/access_control.rs` | Add `require_org_membership()` to `AccessContext` | #1d | #51 |
| `crates/server/src/helpers.rs` | Create — `parse_db_uuid_param()` | #1b | #51 |
| `.github/workflows/ci.yml` | Remove `continue-on-error`, add `vite build` | #2 | #51 |
| `rustfmt.toml` | Modify — add vendored crate exclusions (file already exists with import settings) | #2 | #51 |
| `.githooks/pre-commit` | Create from stash | #3 | #51 |
| `frontend/package.json` | Add lint-staged + simple-import-sort deps | #3 | #51 |
| `frontend/.eslintrc.cjs` | Add import-sort rules from stash | #3 | #51 |
| `frontend/src/constants/artifact.ts` | Create — deduplicate `VIDEO_EDIT_TYPES` | modularity | #51 |
| **PR #52 — Stability** |
| `crates/db/src/models/workflow_trigger.rs` | Atomic `try_claim_trigger()` | #4 | #52 |
| `crates/server/src/routes/data_source_workflows.rs` | Replace `is_past_cooldown()` with `try_claim_trigger()` at L1347 | #4 | #52 |
| `crates/server/src/workers/mod.rs` | Create — BackgroundWorker trait + ShutdownRegistry | #5 | #52 |
| `crates/server/src/main.rs` | Graceful shutdown + registry wiring | #5 | #52 |
| **PR #53 — Friction Logging** |
| `crates/server/src/routes/feedback.rs` | Add friction fields | #10 | #53 |
| `frontend/src/components/dialogs/feedback/FeedbackDialog.tsx` | Extend with friction fields | #10 | #53 |
| **PR #54 — Cost Bridge** (after #51) |
| `crates/server/src/routes/vibe_treasury.rs` | Add cost aggregation endpoints | #6 | #54 |
| `crates/db/src/models/vibe_transaction.rs` | Add GROUP BY queries | #6 | #54 |
| `frontend/src/lib/api/costs.ts` | Create — dedicated cost API module | #6 | #54 |
| `frontend/src/pages/ai-usage.tsx` | Extract tab components + redirect to cost API | #6 | #54 |
| `frontend/src/constants/query-config.ts` | Create — centralized staleTime/refetchInterval | modularity | #54 |
| **PR #55 — Agent Engine** (after #52, merges last) |
| `crates/db/src/models/agent_response.rs` | Create — envelope types + StructuredOutput | #7 | #55 |
| `crates/db/src/models/agent_flow.rs` | NeedsClarification + FSM validation | #8, #9 | #55 |
| `crates/server/src/routes/agent_flows.rs` | Clarification endpoint | #8 | #55 |
| `crates/server/src/agent_flow_executor.rs` | Create — engine background worker | #9 | #55 |
| `crates/server/src/events/mod.rs` | Create — DomainEvent enum + broadcast | #9 | #55 |
| `crates/db/src/models/crm_deal.rs` | Deal FSM validation + pipeline check | #9 | #55 |
| `crates/db/src/models/crm_pipeline.rs` | `valid_next_stages()` method | #9 | #55 |
| `crates/db/migrations/20260413000001_*.sql` | NeedsClarification column | #8 | #55 |
| `Cargo.toml` (workspace) | Add `validator` to workspace deps | Arch E | #55 |

---

## Decisions Log

All decisions resolved during planning — captured here for reference.

| Decision | Resolution | Context |
|----------|-----------|---------|
| `cargo fmt` in CI | Full-codebase check + `rustfmt.toml` ignore for vendored crates. Verify main passes first. | Incremental format on modified files only |
| `crm_deals.rs` split timing | Split FIRST, before access control / FSM changes | Reduces merge conflict risk |
| Cost endpoint access control | `require_org_membership()` — org-scoped, not project-scoped `require_editor()` | Cost data is per-org, not per-project |
| Vendored crate formatting | Exclude via `rustfmt.toml` `ignore` list | `serenity-voice-model` etc. |
| PR #51 scope | Single PR (no split into #51a/#51b) | CI + access control straightforward enough together |
| Items #7 + #9 merge | Build together — protocol exists to serve the executor | Avoids design-in-a-vacuum risk |
| DomainEvent persistence | Persist events to DB via `emit_and_persist()` pattern | Follows existing Nora `emit_flow_event()` pattern |
| Agent dispatch mode | Hybrid: API + executor, configurable per flow (default: "api") | Support both research and coding tasks |
| ESLint max-warnings | Accept 110→180 bump for now | Re-evaluate at sprint end |
| Drain timeout | Configurable via `SHUTDOWN_DRAIN_TIMEOUT_SECS` (default 30s) | LLM streams can run 60s+ |

---

## PR #53 Regression Analysis (2026-03-19)

Full regression analysis performed against `main`. Findings and fixes below.

### P0 Bugs Found & Fixed

| Bug | File | Fix |
|-----|------|-----|
| `duration_ms` always 0 — `Instant::now().elapsed()` instead of using `start` | `workflow_triggers.rs:388` | Added `start` timer, use `start.elapsed()` |
| `'in_progress'` should be `'inprogress'` (no underscore) | `crm_deal_automations.rs:165,180,448` + 10 more across 5 crates | Replaced all 13+ occurrences, added `canonical_status` normalization in Topsi/Nora |
| `BrandSetupWizard` saves to org API when used on company page | `CompanyBrandGuidePage.tsx` | Added `apiOverride` prop |
| `UpdateCrmDeal` missing `expedited` field | `crm_deal.rs`, `crm.ts`, `OverviewTab.tsx` | Added field, removed `as any` |
| Hex UUID slice panic | `crm_deal_transitions.rs:672` | Added length guard |
| CSS injection via font names | `BrandGuidePage`, `CompanyBrandGuidePage` | Added `sanitizeFont()` |
| Feedback endpoint no auth | `feedback.rs`, `mod.rs` | Moved to `protected_routes` |
| `.expect()` in shutdown handlers | `workers/mod.rs` | Replaced with error logging |
| Schedule loop not using atomic cooldown | `data_source_workflows.rs` | Migrated to `try_claim_trigger()` |
| `collaborators` type conflict on `TaskWithAttemptStatus` | `task.rs`, shared/types.ts, 6 frontend files | Renamed to `parsed_collaborators` |

### Remaining (low — tracked for follow-up)
- `workflow_triggers.rs:172` — manual-fire webhook still uses racy `is_past_cooldown()`
- `workers/mod.rs:53` — `JoinHandle` dropped from `tokio::spawn`
- `main.rs` — no drain timeout on `axum::serve`
- TaskStatus enum constants (`as_str()` method) — replace hardcoded SQL strings
- Potential snake_case migration for task statuses

---

## In-Sprint Modularity Extractions (~4h, embedded in PRs)

| Extraction | Effort | PR |
|-----------|--------|-----|
| `require_org_membership()` → `AccessContext` method | 30m | #51 |
| `parse_db_uuid_param()` helper → `helpers.rs` | 20m | #51 |
| `crm_deals.rs` → transitions + automations modules | 1h | #51 |
| `VIDEO_EDIT_TYPES` → `constants/artifact.ts` | 15m | #51 |
| `BackgroundWorker` trait + `spawn_worker()` | 30m | #52 |
| Date formatting → use `formatDate()` everywhere | 30m | #54 |
| `QUERY_CONFIG` constants → `query-config.ts` | 30m | #54 |
| `ai-usage.tsx` tab component extraction | 45m | #54 |
| `AgentFlowExecutorConfig::from_env()` | 15m | #55 |

Deferred modularity items → `BACKLOG--remaining-work.md`

---

## Out of Scope

See `BACKLOG--remaining-work.md` for full details. Key deferrals:

- **Agent Engine Phase 2**: parallel execution, traceback/null transitions, artifact FSMs, context isolation, HTDAG, Contract Net Protocol
- **Cost & routing**: tiered model routing (S0-08), CAPO tracking (S0-07), prompt caching (S0-10)
- **Infrastructure**: full validation framework (S0-13), webhook retry (S0-14), rate limiting (S0-17), DbUuid Phase C/D
- **Observability**: OpenTelemetry, health checks, structured JSON logs
- **Platform**: A2A Protocol, BDI model, blackboard architecture, workflow marketplace/versioning
- **Modularity**: env var helpers (96+ sites), model error macro (30+ files), FromRow cleanup (83+ structs), large file splits beyond `crm_deals.rs`, frontend component splits, stream hook backoff, modal/button/empty-state components
- **Roadmap features**: F11 call scheduling UI, F3 person invite, workflow triggers (cron + entity-change)

---

## Gap Analysis Summary

Two research phases (8 + 7 parallel agents) audited the codebase before implementation.

### Scope corrections applied
1. **crm_deals.rs access control already complete** — real gaps in `crm_contacts.rs` (12 endpoints) and `crm_pipelines.rs` (13 endpoints). Retargeted.
2. **Cost bridge misdirected** — `TokenUsage::create()` rarely called; all billing via `vibe_transactions`. Redirected dashboard.
3. **Background task count**: 11 direct tokio::spawn + 2 wrapper spawns + 1 APN subprocess = ~14 total (no PR Monitor Service found).
4. **Items #7 + #9 merged** — response protocol and engine too coupled to build separately.

### Confirmed as-planned
- Item #3 (lint-staged stash ready), #4 (race confirmed at webhook handler L172/249), #8 (FlowStatus enum ready), #10 (extensible via JSON metadata)

### Key codebase facts
- 23 workspace crates, nightly-2025-05-18 toolchain
- 443 tests (375 `#[test]` + 68 `#[tokio::test]`)
- `rustfmt.toml` already exists (import settings) — needs vendored crate `ignore` list added
- Latest migration: `20260412000000`
- `workers/`, `events/`, `agent_flow_executor.rs` — all need creating
- `TaskScheduler` (310 lines) — dead code, patterns reusable for engine
- Frontend hooks (`useAgentFlows`, `useAgentFlowEvents`, `useAgentFlowMutations`) already built
- `vibeApi` client already exists — extend for cost bridge
- CancellationTokens created but `root.cancel()` never called
- SSE is polling-based (500ms DB queries per client) — adequate for dogfooding

### Pipeline roadmap validation
Cross-referenced against consolidated roadmap on `research/5-year-product-roadmap` branch:
- Agent Flow Engine = P0 Blocker #3 (largest architecture gap) ✓
- Human gate enforcement ✓ (#9 FSM + #8 NeedsClarification)
- CI pipeline = Critical tech debt ✓ (#2)
- Graceful shutdown = High tech debt ✓ (#5)
- Org-level CRM authz ✓ (#1d, #1e — roadmap listed as "Deferred", we're fixing it)

Inline stage-trigger agents (Scout, Astra, Cash, Lux) coexist with engine in Phase 1 (engine opt-in via `ENABLE_AGENT_FLOW_ENGINE=1`). Phase 2 migrates them for retry/timeout/observability.

---

## Expansion Audit (2026-03-19)

8 parallel codebase audit agents verified all claims. Sequential thinking analyzed dependencies and contradictions.

### Corrections Applied
1. **Item #1a**: Marked as already done — all 20 `crm_deals.rs` handlers confirmed to have access checks
2. **Item #1b**: Verified — all 6 `uuid::Uuid` instances confirmed still present (L20, L895, L1011, L1191, L1194, L1195)
3. **Item #2b**: Fixed crate name `pcg-cli` → `cli` (actual directory name)
4. **Item #4**: Clarified TOCTOU race location (data_source_workflows.rs:1347) vs model method (workflow_trigger.rs:272-293)
5. **Item #6**: Changed access control from `require_editor()` (project-scoped) → `require_org_membership()` (org-scoped)
6. **Item #6**: Added `frontend/src/lib/api/costs.ts` as dedicated cost API module + ai-usage.tsx tab extraction
7. **Item #9**: Fixed `WorkflowLLMService::completion_with_tools()` → `route_completion()` in pcg_router.rs
8. **Item #9**: Fixed `move_deal_stage()` → `CrmDeal::move_to_stage()` (crm_deal.rs:405-537)
9. **Item #10**: Fixed FeedbackDialog path → `components/dialogs/feedback/FeedbackDialog.tsx`
10. **PR renumber**: #53→friction (independent), #54→cost bridge (after #51), #55→agent engine (after #52, last)
11. **Files table**: `rustfmt.toml` changed from "Create" to "Modify" (already exists with import settings)
12. **Gap analysis**: Background operation count corrected to ~14 (no PR Monitor Service exists)
13. **Item #3**: Stash verified — `stash@{0}` contains expected 3 files + 654 auto-reformatted to discard. Added `pnpm install` step.

### Decisions Resolved (post-expansion)
1. ~~uuid::Uuid check~~ → Verified: all 6 still present, fix proceeds as planned
2. ~~PR dependency~~ → Accepted: renumbered PRs so #54 (cost bridge) merges after #51
3. ~~ai-usage.tsx extraction~~ → Extract tab components during cost bridge work (PR #54)
4. ~~Cost API location~~ → Dedicated `frontend/src/lib/api/costs.ts` module
5. ~~Stash verification~~ → Confirmed correct, contents match expectations

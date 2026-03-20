# Tier 1 Phase 0 Sprint — Foundation & Dogfood Readiness

**Date**: 2026-03-19
**Branch**: `feature/2026-03-19--tier1-phase0`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `main` (commit `ed66d568b`)
**Status**: IN PROGRESS
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

**1a. Harden kanban access control** (`crm_deals.rs:256-275`)
- Fix: deny access when `organization_id` is `None` → return 403

**1b. Fix `uuid::Uuid` in new code** (`crm_deals.rs`)
- 6 instances (lines 20, 895, 1011, 1191, 1194, 1195) → convert to `DbUuid::parse()` / `bind_uuid_blob()`
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
**2b. Fix clippy** — `discord-bots` (29), `utils` (23), `pcg-cli` (65). Use `#![allow(clippy::uninlined_format_args)]` at crate root for bulk non-critical lints.
**2c. Fix `alpha-protocol-core` test compile errors** — 2 borrow checker issues
**2d. Fix ESLint** — `eslint --fix` on modified files only
**2e. Remove `continue-on-error: true`** from CI lines 48, 65 (clippy + tests). Keep on lines 119, 129 (security audits).
**2f. Add `vite build` step** to CI frontend-check job

**CI fmt strategy**: Add `rustfmt.toml` with `ignore` list for vendored crates (`serenity-voice-model`). Use `#[rustfmt::skip]` for pre-existing violations outside sprint scope. Verify `cargo fmt --check` passes on main first — if it doesn't, determine scope of pre-existing violations before committing to full-codebase check.

### 3. Apply Lint-Staged Config [QUICK WIN — Day 1]
**Effort**: 0.25 days | **PR**: #51

Selectively restore config from stash (NOT cherry-pick — stash uses `git checkout stash@{0} -- <file>`):
- `.githooks/pre-commit`, `frontend/package.json` (lint-staged + simple-import-sort deps), `frontend/.eslintrc.cjs`
- Discard: 654 auto-reformatted source files
- ESLint max-warnings: accept 110→180 bump; re-evaluate at sprint end
- `git config core.hooksPath .githooks` to activate
- **Note**: verify stash@{0} is correct before applying — stash refs shift

### 4. Cooldown Race Condition Fix (S0-05) [HIGH — Day 2]
**Effort**: 0.5 days | **PR**: #52

File: `crates/db/src/models/workflow_trigger.rs` (lines 272-293)
```sql
UPDATE workflow_triggers
SET last_triggered_at = datetime('now'), trigger_count = trigger_count + 1
WHERE id = ? AND (last_triggered_at IS NULL
  OR (julianday('now') - julianday(last_triggered_at)) * 86400 >= cooldown_seconds)
RETURNING *
```
- New method: `try_claim_trigger(pool, trigger_id) -> Option<WorkflowTrigger>`
- Update callers in `data_source_workflows.rs`
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
**Effort**: 1 day | **PR**: #53

Redirect dashboard from dead `token_usage` table to `vibe_transactions` (source of truth).

- Add org-scoped cost aggregation endpoints to `vibe_treasury.rs`:
  - `GET /api/vibe/costs/summary?org_id=&days=`
  - `GET /api/vibe/costs/daily?org_id=&days=`
  - `GET /api/vibe/costs/by-model?org_id=&days=`
  - `GET /api/vibe/costs/by-project?org_id=&days=`
- **Access control**: `require_editor()` on all cost endpoints
- Add GROUP BY queries to `vibe_transaction.rs` model
  - **DONE**: dual-path JOINs (project direct + agent via task_id→project)
  - **FIXED**: BLOB→TEXT hex conversion in JOINs (vibe_transactions uses BLOB UUIDs, projects/tasks use TEXT)
  - **NOTE**: agent transactions without `task_id` are excluded from org aggregation (by design — only the 0-amount pending record lacks task_id)
- Update `ai-usage.tsx` + `TokenUsageWidget.tsx` to call vibe endpoints
- **TODO before merge**: `cargo sqlx prepare --workspace` + `npm run generate-types`
- **Deferred**: date range filtering on cost endpoints (no `since`/`until` params yet), full `vibe_transactions` BLOB→TEXT migration

### 7. Structured Response Protocol (S0-03 + Arch D) [FOUNDATION — merged with #9]
**Effort**: Combined with #9 | **PR**: #54

- `crates/db/src/models/agent_response.rs` — `AgentResponseEnvelope`, `AgentResponseStatus` (Done/NeedsClarification/Failed/InProgress), `ClarificationRequest`, `ResponseMetrics`
- `#[derive(TS)]` for frontend type generation
- Run `npm run generate-types` after creating types

### 8. Clarification as First-Class Status (S0-04) [FOUNDATION — Day 4]
**Effort**: 0.5 days | **PR**: #54

- Add `NeedsClarification` as 8th variant to `FlowStatus` in `agent_flow.rs`
- Migration `20260413000001`: add `clarification_request` TEXT column to `agent_flows`
- New endpoint: `POST /agent-flows/{id}/respond-clarification`
- Frontend: render clarification form when flow is in this status
- Run `cargo sqlx prepare --workspace` after migration

### 9. Agent Flow Orchestration Engine — Phase 1 (S0-06 + Arch A/B/C) [KEYSTONE — Day 4-8]
**Effort**: 5 days | **PR**: #54

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
- **API dispatch** (default): `pcg_router.rs` → `WorkflowLLMService::completion_with_tools()`. Best for research/analysis.
- **Executor dispatch**: `container.start_execution()` → CLI process in git worktree. Best for coding.
- `flow_config.dispatch_mode: "api" | "executor" | "auto"` (default: "api")
- Both parse output into `AgentResponseEnvelope`

**CRM Deal FSM prototype** — same pattern, embedded in this sprint:
- `move_deal_stage()` currently has NO stage ordering validation
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
**Effort**: 1.5 days | **PR**: #55

Extend `crates/server/src/routes/feedback.rs`:
- `friction_area: Option<String>` — enum: workflow_editor, crm_pipeline, task_management, etc.
- `workflow_step: Option<String>`, `workaround: Option<String>`, `feature_request: Option<String>`, `source: Option<String>` ("dogfood" auto-tag)
- Apply `#[derive(Validate)]` with length limits
- Migration `20260413000002`: add friction columns to feedback table (or use JSON metadata — no migration needed)
- Frontend: "Report Friction" in sidebar More popover

---

## PR Strategy

| PR | Items | Safe Because | Depends On |
|----|-------|-------------|-----------|
| **#51** | #1, #2, #3 | All fixes/infra, no new features | Independent |
| **#52** | #4, #5 | Bug fix + internal refactor | Independent |
| **#53** | #6 | Redirects dashboard to real data | Independent |
| **#54** | #7, #8, #9 | Engine **disabled by default** (`ENABLE_AGENT_FLOW_ENGINE=1`) | After #52 (needs worker trait) |
| **#55** | #10 | Extends existing form with optional fields | Independent |

PRs #51, #52, #53, #55 can merge in any order. PR #54 merges last.

**Critical path**: #5c (worker trait) → #7 (response protocol) → #8 (clarification) → #9 (engine)

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
| `rustfmt.toml` | Create — vendored crate exclusions | #2 | #51 |
| `.githooks/pre-commit` | Create from stash | #3 | #51 |
| `frontend/package.json` | Add lint-staged + simple-import-sort deps | #3 | #51 |
| `frontend/.eslintrc.cjs` | Add import-sort rules from stash | #3 | #51 |
| `frontend/src/constants/artifact.ts` | Create — deduplicate `VIDEO_EDIT_TYPES` | modularity | #51 |
| **PR #52 — Stability** |
| `crates/db/src/models/workflow_trigger.rs` | Atomic `try_claim_trigger()` | #4 | #52 |
| `crates/server/src/routes/data_source_workflows.rs` | Update cooldown callers | #4 | #52 |
| `crates/server/src/workers/mod.rs` | Create — BackgroundWorker trait + ShutdownRegistry | #5 | #52 |
| `crates/server/src/main.rs` | Graceful shutdown + registry wiring | #5 | #52 |
| **PR #53 — Cost Bridge** |
| `crates/server/src/routes/vibe_treasury.rs` | Add cost aggregation endpoints | #6 | #53 |
| `crates/db/src/models/vibe_transaction.rs` | Add GROUP BY queries | #6 | #53 |
| `frontend/src/pages/ai-usage.tsx` | Redirect to vibe endpoints | #6 | #53 |
| `frontend/src/constants/query-config.ts` | Create — centralized staleTime/refetchInterval | modularity | #53 |
| **PR #54 — Agent Engine** |
| `crates/db/src/models/agent_response.rs` | Create — envelope types + StructuredOutput | #7 | #54 |
| `crates/db/src/models/agent_flow.rs` | NeedsClarification + FSM validation | #8, #9 | #54 |
| `crates/server/src/routes/agent_flows.rs` | Clarification endpoint | #8 | #54 |
| `crates/server/src/agent_flow_executor.rs` | Create — engine background worker | #9 | #54 |
| `crates/server/src/events/mod.rs` | Create — DomainEvent enum + broadcast | #9 | #54 |
| `crates/db/src/models/crm_deal.rs` | Deal FSM validation + pipeline check | #9 | #54 |
| `crates/db/src/models/crm_pipeline.rs` | `valid_next_stages()` method | #9 | #54 |
| `crates/db/migrations/20260413000001_*.sql` | NeedsClarification column | #8 | #54 |
| `Cargo.toml` (workspace) | Add `validator` to workspace deps | Arch E | #54 |
| **PR #55 — Friction Logging** |
| `crates/server/src/routes/feedback.rs` | Add friction fields | #10 | #55 |
| `frontend/src/components/feedback/` | Friction form UI | #10 | #55 |

---

## Decisions Log

All decisions resolved during planning — captured here for reference.

| Decision | Resolution | Context |
|----------|-----------|---------|
| `cargo fmt` in CI | Full-codebase check + `rustfmt.toml` ignore for vendored crates. Verify main passes first. | Incremental format on modified files only |
| `crm_deals.rs` split timing | Split FIRST, before access control / FSM changes | Reduces merge conflict risk |
| Cost endpoint access control | `require_editor()` — viewer too permissive for financial data | Consistent with RBAC patterns |
| Vendored crate formatting | Exclude via `rustfmt.toml` `ignore` list | `serenity-voice-model` etc. |
| PR #51 scope | Single PR (no split into #51a/#51b) | CI + access control straightforward enough together |
| Items #7 + #9 merge | Build together — protocol exists to serve the executor | Avoids design-in-a-vacuum risk |
| DomainEvent persistence | Persist events to DB via `emit_and_persist()` pattern | Follows existing Nora `emit_flow_event()` pattern |
| Agent dispatch mode | Hybrid: API + executor, configurable per flow (default: "api") | Support both research and coding tasks |
| ESLint max-warnings | Accept 110→180 bump for now | Re-evaluate at sprint end |
| Drain timeout | Configurable via `SHUTDOWN_DRAIN_TIMEOUT_SECS` (default 30s) | LLM streams can run 60s+ |

---

## In-Sprint Modularity Extractions (~4h, embedded in PRs)

| Extraction | Effort | PR |
|-----------|--------|-----|
| `require_org_membership()` → `AccessContext` method | 30m | #51 |
| `parse_db_uuid_param()` helper → `helpers.rs` | 20m | #51 |
| `crm_deals.rs` → transitions + automations modules | 1h | #51 |
| `VIDEO_EDIT_TYPES` → `constants/artifact.ts` | 15m | #51 |
| `BackgroundWorker` trait + `spawn_worker()` | 30m | #52 |
| Date formatting → use `formatDate()` everywhere | 30m | #53 |
| `QUERY_CONFIG` constants → `query-config.ts` | 30m | #53 |
| `AgentFlowExecutorConfig::from_env()` | 15m | #54 |

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
3. **Background task count 14→16** — added PR Monitor Service + APN Node subprocess.
4. **Items #7 + #9 merged** — response protocol and engine too coupled to build separately.

### Confirmed as-planned
- Item #3 (lint-staged stash ready), #4 (race confirmed at webhook handler L172/249), #8 (FlowStatus enum ready), #10 (extensible via JSON metadata)

### Key codebase facts
- 23 workspace crates, nightly-2025-05-18 toolchain
- 443 tests (375 `#[test]` + 68 `#[tokio::test]`)
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

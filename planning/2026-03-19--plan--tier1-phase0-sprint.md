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
3. **PR #50 introduced regressions** — kanban access control removed, `uuid::Uuid` in new code, raw `fetch()`.
4. **Cost dashboard shows $0** — `TokenUsage.cost_cents` is never populated; `VibeTransaction` has real data but dashboard doesn't read it.
5. **No graceful shutdown** — bare `axum::serve()`, CancellationTokens created but never signaled.
6. **Concurrent webhook double-fire** — cooldown check is non-atomic.

This sprint fixes what's broken, hardens the safety net, then builds the agent engine foundation.

---

## Architectural Improvements (Embedded in Sprint Items)

These aren't separate items — they're patterns to apply *while* implementing sprint items, improving modularity for future growth:

### A. Background Worker Trait
Currently 3 standalone `tokio::spawn` loops with copy-pasted patterns (`spawn_automation_loop`, `spawn_workflow_schedule_loop`, and 10+ ad-hoc spawns in main.rs). When building the Agent Flow Executor (#9), extract a shared `BackgroundWorker` pattern:

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

**ROI**: Every future background worker (webhook retry, CAPO tracking, event bus) gets graceful shutdown, error logging, and naming for free. Existing workers can migrate incrementally.

### B. Centralized Shutdown Registry
Currently CancellationTokens are created ad-hoc and never signaled. When implementing graceful shutdown (#5), create a `ShutdownRegistry`:

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
    pub fn shutdown_all(&self) {
        self.root.cancel(); // cascades to all children
    }
}
```

**ROI**: All background workers drain properly on SIGTERM. Agent flow executor, workflow scheduler, VIBE watchers all benefit.

### C. Event Emission Layer
Currently, when a task status changes, nothing notifies other systems. CRM automations poll hourly. When building the agent flow executor (#9), add a lightweight in-process event channel:

```rust
// crates/server/src/events/mod.rs
pub enum DomainEvent {
    TaskStatusChanged { task_id: String, old_status: String, new_status: String },
    FlowPhaseAdvanced { flow_id: String, phase: String },
    WorkflowCompleted { workflow_id: String, run_id: String },
    AgentResponseReceived { flow_id: String, response: AgentResponseEnvelope },
}
```

Use `tokio::sync::broadcast` channel — multiple listeners, no persistence needed initially. The agent flow executor subscribes to task events; workflow engine subscribes to flow events. Future: persist to JSONL (S0-09) or NATS.

**ROI**: Enables reactive workflows (S0-05 internal event triggers), removes hourly polling dependency for CRM automations, and provides the foundation for the append-only event log (S0-09).

### D. Response Envelope as Interface Contract
The Structured Response Protocol (#7) isn't just a data type — it's the **interface contract** between all system boundaries:
- Agent → Orchestrator (flow engine)
- Workflow node → Workflow engine (node execution results)
- MCP tool → Agent (tool call results)

Design it as a trait:

```rust
pub trait StructuredOutput {
    fn status(&self) -> ResponseStatus;
    fn artifacts(&self) -> &[ArtifactReference];
    fn needs_clarification(&self) -> Option<&ClarificationRequest>;
}
```

**ROI**: Consistent result handling across agent flows, workflow execution, and MCP tools. Future model routing (S0-08) can inspect response quality to route retries to different tiers.

### E. Input Validation Progressive Adoption
No `validator` crate exists today — all validation is inline. Rather than a big-bang adoption, add validation incrementally via a derive macro pattern on the highest-risk structs first:

During this sprint, add `validator` to root `Cargo.toml` workspace dependencies (`[workspace.dependencies]`) and reference via `validator = { workspace = true }` in crates that need it (`server`, `db`). Apply `#[derive(Validate)]` to:
1. `SubmitFeedbackRequest` (friction logging form — new code, clean slate)
2. `CreateFlowEvent` (agent events — new code)
3. `CreateAgentFlow` (flow creation — modified code)

**ROI**: Establishes the pattern for S0-13 (full input validation framework) without blocking sprint items.

---

## Sprint Items (10 items, ROI-ordered)

### 1. Fix PR #50 Regressions + Access Control Gaps [CRITICAL — Day 1]
**Effort**: 1 day | **Parallelizable**: Yes

**1a. Harden kanban access control** (`crates/server/src/routes/crm_deals.rs:256-275`)
- Current: checks `require_org_membership` only when `pipeline.organization_id` is `Some`
- Fix: deny access when `organization_id` is `None` — return 403 "Pipeline has no organization scope"
- This is a security fix, not a feature

**1d. Add access control to `crm_contacts.rs`** (12 endpoints, ZERO access checks)
- All handlers lack `Extension(access_context)` parameter entirely
- Any authenticated user can CRUD all contacts regardless of organization membership
- Fix: Extract `require_org_membership()` to shared module `crates/server/src/helpers/access.rs` (low-effort modularity — reusable across all CRM route files)
- Add `Extension(access_context): Extension<AccessContext>` to all handlers
- Add `require_org_membership()` checks using `organization_id` query param

**1e. Add access control to `crm_pipelines.rs`** (13 endpoints, ZERO access checks)
- Same issue as contacts — handlers accept `organization_id` but never validate membership
- Fix: Use shared `require_org_membership()` from new helpers/access.rs

**1b. Fix `uuid::Uuid` in new code** (`crm_deals.rs`)
- 6 instances of `uuid::Uuid` found (line 20 import + lines 895, 1011, 1191, 1194, 1195).
- Most are for BLOB column binding — these need `DbUuid` but may require model-layer changes since BLOB columns expect `uuid::Uuid`.
- Per rust-standards.md: "NEVER use uuid::Uuid in route handlers" — route handler code should use `DbUuid::parse()`, push BLOB conversion to model/consumer layer.

**DbUuid conversion patterns** (from `crates/db/src/db_uuid.rs`):
- Canonical route handler pattern: `DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid UUID".into()))?.to_uuid()`
- For BLOB column binding: `bind_uuid_blob(&db_uuid)?` → returns `Vec<u8>` (16-byte BLOB)
- For TEXT column binding: `bind_uuid(&db_uuid)` → returns `&str`
- For optional BLOB: `bind_optional_uuid_blob(&opt_db_uuid)?`
- For string→BLOB shortcut: `str_to_uuid_blob(&str_id)?`
- Known BLOB columns: `users.id`, `project_members.user_id/.granted_by`, `organization_members.user_id`, `client_members.user_id`
- All other UUID columns are TEXT — use `bind_uuid()` or `.bind(&db_uuid)` directly.

**Fix approach for crm_deals.rs**:
- Line 895: `uuid::Uuid::parse_str(person_id.as_str())` → use `DbUuid::from_string(person_id.as_str()).to_uuid()` (person_id is already validated)
- Line 1011: `uuid::Uuid::parse_str(company.id.as_str())` → `DbUuid::from_string(company.id.as_str()).to_uuid()`
- Lines 1191-1195: BLOB column binding for `business_reports` table → use `bind_uuid_blob()` helper
- Line 20: remove `use uuid::Uuid;` import after above conversions
- **Scope for this sprint**: Fix route-handler-level usage. BLOB-binding instances deferred to DbUuid Phase C (backlog) only if model layer requires `uuid::Uuid` type signature changes.

**1f. Split `crm_deals.rs` (2,923 lines)** — opportunistic while touching for #1a, #1b, FSM
- Extract stage transition logic (move_deal_stage, advance_deal, mark_deal_won ~800 lines) → `crm_deal_transitions.rs`
- Extract research/proposal/deck auto-triggers (~400 lines) → `crm_deal_automations.rs`
- Core CRUD + list/get handlers stay in `crm_deals.rs` (~1,700 lines)
- `main.rs` split happens naturally via item #5 (workers extract to `workers/mod.rs`)

**1c. ~~Fix raw `fetch()` in call-intake.tsx~~** — RESOLVED
- Exploration found `call-intake.tsx` already uses `makeRequest` (dynamic import), not raw `fetch()`.
- Uses `callIntakeApi` and `reportsApi` domain modules correctly.
- **No fix needed** — remove from sprint scope.

### 2. Fix CI Pipeline [HIGH — Day 1-2]
**Effort**: 1.5 days | **Parallelizable**: Yes (assign to different dev)

**Incremental lint/fmt strategy** — optimize for speed, avoid 200+ file mass commits:
- Run `cargo fmt` and `eslint --fix` only on files modified in this sprint, not full codebase
- Pre-commit hooks (item #3) enforce formatting on all future commits automatically
- Full codebase formatting deferred to a dedicated cleanup PR (low-conflict window)

**2a. `cargo fmt` on modified files only** — format files touched by sprint items, not `cargo fmt --all`
**2b. Fix clippy errors** — `discord-bots` (29), `utils` (23), `pcg-cli` (65). Use `#![allow(clippy::uninlined_format_args)]` at crate root for bulk non-critical lints. Exclude vendored crates (`serenity-voice-model`).
**2c. Fix `alpha-protocol-core` test compile errors** — 2 borrow checker issues in test code
**2d. Fix ESLint on modified files** — `eslint --fix` only on files touched in sprint
**2e. Remove `continue-on-error: true`** from `.github/workflows/ci.yml` lines 48, 65 (clippy + tests). Keep on lines 119, 129 (security audits — advisory only).
**2f. Add `vite build` step** to CI — currently only `tsc --noEmit` catches TS errors; actual build failures go undetected. Add after ESLint step in `frontend-check` job.

**CI fmt strategy (decided)**: CI runs `cargo fmt --check` on full codebase. To avoid mass-formatting:
- Add `rustfmt.toml` with `ignore` list for vendored crates (`serenity-voice-model`, etc.)
- Use targeted `#[rustfmt::skip]` or `eslint-disable` for pre-existing violations outside sprint scope
- Goal: CI passes without touching files outside sprint scope

**`crm_deals.rs` split timing (decided)**: Split FIRST (before making access control / FSM changes). Reduces merge conflict risk and makes subsequent changes to smaller files.

### 3. Apply Lint-Staged Config [QUICK WIN — Day 1]
**Effort**: 0.25 days

**Strategy change**: Extract only config files from stash, discard 657-file import reorder:
- Cherry-pick from stash: `.githooks/pre-commit`, `frontend/package.json` (lint-staged + simple-import-sort deps), `frontend/.eslintrc.cjs` (import-sort rules)
- Discard: mass import reordering across 654 source files (pre-commit hook handles incrementally on future commits)
- ESLint max-warnings: accept 110→180 bump for now; re-evaluate at sprint end based on warning count
- `git config core.hooksPath .githooks` to activate
- Verify: stage a `.tsx` file with bad imports → pre-commit auto-sorts on commit

### 4. Cooldown Race Condition Fix (S0-05) [HIGH — Day 2]
**Effort**: 0.5 days

File: `crates/db/src/models/workflow_trigger.rs` (lines 272-293)
- Replace `is_past_cooldown()` read-then-update with atomic SQL:
  ```sql
  UPDATE workflow_triggers
  SET last_triggered_at = datetime('now'), trigger_count = trigger_count + 1
  WHERE id = ? AND (last_triggered_at IS NULL
    OR (julianday('now') - julianday(last_triggered_at)) * 86400 >= cooldown_seconds)
  RETURNING *
  ```
- New method: `try_claim_trigger(pool, trigger_id) -> Option<WorkflowTrigger>`
- Returns Some if claimed, None if cooldown active
- Update callers in `data_source_workflows.rs`

### 5. Graceful Shutdown + Worker Registry (S0-12 + Arch B) [HIGH — Day 2-3]
**Effort**: 1.5 days

**5a. Signal handler** — `crates/server/src/main.rs` line 586
```rust
axum::serve(listener, app_router)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```
Add `shutdown_signal()` using `tokio::signal` for SIGTERM + SIGINT.
Configurable drain timeout: default 30s, env `SHUTDOWN_DRAIN_TIMEOUT_SECS` for longer-running tasks (LLM streams can run 60s+).
**Note**: `CancellationToken` + Axum may have edge cases (axum issue #3326) — test thoroughly.

**5b. Shutdown registry** — new `crates/server/src/workers/mod.rs`
- `ShutdownRegistry` with root CancellationToken that cascades to children
- Wire existing tokens: `sovereign_stack_shutdown` (line 178), `schedule_shutdown` (line 348) — both created but NEVER signaled
- Register all 14 background tasks (11 in main.rs + 3 in route files):
  - Line 147: file search cache (one-time)
  - Line 161: sovereign storage sync
  - Line 183: sovereign stack scraper (has token, unused)
  - Line 211: pulse NATS consumer
  - Line 226: APN data service
  - Line 249: auto-start APN node (one-time)
  - Line 307: APN peer cleanup (60s)
  - Line 360: VIBE deposit watcher (30s)
  - Line 409: VIBE withdrawal executor (60s)
  - Line 501: meeting stale-session cleanup (120s)
  - Line 345: `spawn_automation_loop` (3600s) — no token
  - Line 349: `spawn_workflow_schedule_loop` (300s) — **only task with working shutdown**
  - Line 355: `spawn_oss_listener` (3600s) — no token
  - Line 575: browser opener (one-time, dev only)

**5c. Background worker trait** (Arch improvement A)
- Extract shared `BackgroundWorker` trait pattern
- Migrate `spawn_workflow_schedule_loop` and `spawn_automation_loop` as first adopters
- Agent flow executor (#9) will be the third adopter

**5d. APN Node subprocess cleanup on shutdown**
- APN Node is spawned as detached child process (main.rs:289) — `Child` handle dropped, no SIGTERM on server exit
- Fix: Call `kill_existing_apn_nodes()` in the shutdown signal handler (same function already used at startup, line 253)
- Keep it simple — BackgroundWorker trait handles tokio tasks, subprocess cleanup goes in signal handler directly
- Total background operations: 16 (14 tokio spawns + 1 APN subprocess + 1 PR Monitor service)

### 6. Bridge Dual Cost System (S0-02) [HIGH — Day 3]
**Effort**: 1 day

**Problem**: Dashboard shows $0 because `ai-usage.tsx` reads from `token_usage` table where `cost_cents` is always NULL. Real cost data lives in `vibe_transactions.calculated_cost_cents`.

**Root cause analysis** (from gap audit):
- `TokenUsage::create()` INSERT doesn't include `cost_cents` column at all
- `CreateTokenUsage` struct doesn't have a `cost_cents` field
- All billing goes through `record_llm_vibe_usage()` → `VibePricingService` → `vibe_transactions` table
- `TokenUsage::create()` is only called from `POST /token-usage` (legacy endpoint, rarely called)
- 6 aggregation queries in `token_usage.rs` SUM cost_cents — all return NULL

**Approach**: Redirect dashboard to `vibe_transactions` (source of truth)
- Add org-scoped cost aggregation endpoints to `vibe_treasury.rs` (extend existing, not new module)
- **Access control**: `require_editor()` on all cost endpoints (viewer too permissive for financial data):
  - `GET /api/vibe/costs/summary?org_id=&days=` — total VIBE + cost_cents + tx count
  - `GET /api/vibe/costs/daily?org_id=&days=` — GROUP BY DATE(created_at)
  - `GET /api/vibe/costs/by-model?org_id=&days=` — GROUP BY model, provider
  - `GET /api/vibe/costs/by-project?org_id=&days=` — GROUP BY source_id WHERE source_type='project'
- Add GROUP BY queries to `vibe_transaction.rs` model (indexes already exist: source, created_at, task)
- Update `frontend/src/pages/ai-usage.tsx` to call vibe cost endpoints (reuse existing `vibeApi` patterns from `vibe.tsx`)
- Update `TokenUsageWidget.tsx` (mission-control) to use vibe data
- Keep `token_usage` routes as-is for backward compatibility
- Verify: dashboard shows actual cost data after running an agent

### 7. Structured Response Protocol (S0-03 + Arch D) [FOUNDATION — merged with #9]
**Effort**: Combined with #9 (agent engine) — built together since protocol exists to serve the executor

**New files**:
- `crates/db/src/models/agent_response.rs` — envelope types + `StructuredOutput` trait
- Types: `AgentResponseEnvelope`, `AgentResponseStatus` (Done/NeedsClarification/Failed/InProgress), `ClarificationRequest`, `ResponseMetrics`
- Serialize/deserialize for storage in `agent_flow_events.event_data`
- TypeScript types via `#[derive(TS)]` for frontend consumption

### 8. Clarification as First-Class Status (S0-04) [FOUNDATION — Day 4]
**Effort**: 0.5 days | **Depends on**: #7

- Add `NeedsClarification` as 8th variant to `FlowStatus` enum in `agent_flow.rs` (existing: Planning, Executing, Verifying, Completed, Failed, Paused, AwaitingApproval)
- Migration: add `clarification_request` TEXT column to `agent_flows`
- New endpoint: `POST /agent-flows/{id}/respond-clarification` — stores answer, resumes flow
- Frontend: render clarification question with input form when flow is in this status

### 9. Agent Flow Orchestration Engine — Phase 1 (S0-06 + Arch A/B/C) [KEYSTONE — Day 4-8]
**Effort**: 5 days

**New file**: `crates/server/src/agent_flow_executor.rs`

Implements `BackgroundWorker` trait from #5c. Polls every 15s.

**Config** (follows existing service config pattern):
```rust
pub struct AgentFlowExecutorConfig {
    pub enabled: bool,           // env: ENABLE_AGENT_FLOW_ENGINE (default: false)
    pub poll_interval_secs: u64, // env: AGENT_FLOW_POLL_INTERVAL (default: 15)
    pub max_concurrent: usize,   // env: AGENT_FLOW_MAX_CONCURRENT (default: 3)
    pub cooldown_secs: u64,      // env: AGENT_FLOW_COOLDOWN (default: 60)
}
```

**Frontend**: No new hooks needed — `useAgentFlows`, `useAgentFlowEvents`, `useAgentFlowsAwaitingApproval` already exist in `frontend/src/hooks/useAgentFlows.ts` with appropriate staleTime (30s/15s/10s). Phase 1 focuses on backend only.

**State machine** (FSM with transition validation + caller-aware guards):

Per research (`fsm-requirement-models.md`): the orchestrator IS a state machine — "deterministic envelope, non-deterministic core." The FSM controls *when* agents run; agents control *what* they produce. Phase 1 implements the foundation; Phase 2 adds traceback/null transitions, parallel regions, and artifact FSMs.

**Key design principle**: These flows are also managed from the dashboard by human users. Agent-driven transitions get strict FSM enforcement. Dashboard/API transitions from authenticated users get **permissive mode** — users can force-transition to any valid status. The FSM guards are for preventing *agent* mistakes, not blocking *human* overrides.

```
Planning → [emit PhaseStarted] → Executing →
[emit PhaseCompleted] → Verifying → [emit FlowCompleted]

Any phase → NeedsClarification → [pause, emit event] → user responds → resume
Any phase → Failed → [emit FlowFailed]
Any phase → Paused → [user action] → resume to previous phase
```

**Transition table** (Event-State Analysis, Phase 1 — simplified ESA matrix):
```
              | Planning  | Executing | Verifying | NeedsClar | AwaitAppr | Paused | Completed | Failed |
--------------|-----------|-----------|-----------|-----------|-----------|--------|-----------|--------|
phase_done    | →Executing| →Verifying| →Completed| —         | —         | —      | —         | —      |
need_clarify  | →NeedsClar| →NeedsClar| →NeedsClar| ∅ (ignore)| —         | —      | —         | —      |
respond_clar  | —         | —         | —         | →resume   | —         | —      | —         | —      |
request_appr  | —         | —         | →AwaitAppr| —         | —         | —      | —         | —      |
approve       | —         | —         | —         | —         | →Completed| —      | —         | —      |
reject        | —         | —         | —         | —         | →Executing| —      | —         | —      |
error         | →Failed   | →Failed   | →Failed   | →Failed   | →Failed   | →Failed| —         | —      |
pause         | →Paused   | →Paused   | →Paused   | —         | —         | ∅      | —         | —      |
resume        | —         | —         | —         | —         | —         | →prev  | —         | —      |
user_override | →any      | →any      | →any      | →any      | →any      | →any   | →any      | →any   |
```

`user_override` = dashboard user with auth, bypasses FSM guards. `→resume` = returns to phase stored in `previous_phase` field. `∅` = event ignored. `—` = invalid (returns error for agents, allowed for user_override).

**Implementation** — add to `agent_flow.rs`:
```rust
impl FlowStatus {
    /// Valid transitions for agent-driven (strict) mode.
    /// Returns None if transition is invalid.
    pub fn valid_agent_transitions(&self) -> &[FlowStatus] {
        match self {
            Self::Planning => &[Self::Executing, Self::NeedsClarification, Self::Failed, Self::Paused],
            Self::Executing => &[Self::Verifying, Self::NeedsClarification, Self::Failed, Self::Paused],
            Self::Verifying => &[Self::Completed, Self::AwaitingApproval, Self::NeedsClarification, Self::Failed, Self::Paused],
            Self::NeedsClarification => &[Self::Planning, Self::Executing, Self::Verifying, Self::Failed],
            Self::AwaitingApproval => &[Self::Completed, Self::Executing, Self::Failed],
            Self::Paused => &[Self::Planning, Self::Executing, Self::Verifying, Self::Failed],
            Self::Completed => &[],  // terminal
            Self::Failed => &[],     // terminal (Phase 2: add retry → Planning)
        }
    }

    pub fn can_transition_to(&self, target: &FlowStatus) -> bool {
        self.valid_agent_transitions().contains(target)
    }
}

/// Validated transition — used by agent flow executor (strict mode).
pub async fn transition_validated(pool: &SqlitePool, id: Uuid, target_status: FlowStatus, target_phase: AgentPhase) -> Result<Self, AgentFlowError> {
    let flow = Self::find_by_id(pool, id).await?.ok_or(AgentFlowError::NotFound)?;
    if !flow.status.can_transition_to(&target_status) {
        return Err(AgentFlowError::InvalidTransition(
            format!("{} → {} not allowed", flow.status, target_status)
        ));
    }
    Self::transition_to_phase(pool, id, target_phase).await
}
```

**Dashboard/API callers** continue using `transition_to_phase()` directly (no validation) — existing behavior preserved. The executor uses `transition_validated()` for strict FSM enforcement.

**Research references applied** (from `roadmap/reference/` on `research/5-year-product-roadmap` branch):
- `fsm-requirement-models.md` — IREB layered FSM model, ESA completeness checking, MetaAgent 3-transition-type FSM
- `rooroo-agent-orchestration.md` — JSON Output Envelope (`{status, message, artifacts}`), cost-tiered model routing, "clarify, don't assume" principle
- `multi-agent-system-design.md` — Hybrid reactive/deliberative architecture, Anthropic orchestrator-workers pattern, context isolation
- `12-ai--workflow-engines.md` — Restate checkpoint-based recovery (Rust-native), event sourcing for execution state, Temporal durable execution patterns

**Phase 1 applies these patterns**:
- **Output Envelope** (RooRoo) → item #7 `AgentResponseEnvelope`
- **"Deterministic envelope, non-deterministic core"** (FSM research) → FSM controls phase transitions, agents have freedom within phases
- **Caller-aware guards** (dashboard constraint) → strict for agents, permissive for humans
- **Hybrid architecture** (MAS research) → BackgroundWorker = reactive loop, agent dispatch = deliberative layer
- **Event sourcing** (workflow engines) → `AgentFlowEvent` table already follows this pattern — current state derivable from event replay

**Phase 2 additions** (deferred — from research backlog #22-27):
- Traceback transitions: `s_i → s_j` (j < i) with accumulated error context (MetaAgent pattern, 85% checkpoint passage)
- Null transitions: self-loops for iterative refinement within a state
- Parallel state regions with `onDone` synchronization (XState statechart pattern)
- Artifact FSMs: `draft → in_review → approved → complete` per output
- ESA matrix auto-generation for workflow definitions
- Guard conditions: `start` requires `assigned_agent IS NOT NULL`, etc.
- Layered validation: deterministic checks before LLM review
- Context isolation: subtask agents get own conversation, parent gets summary only (RooRoo/Anthropic pattern)
- Hierarchical Task DAG (HTDAG): lazy decomposition of non-atomic tasks (Deep Agent pattern)
- Contract Net Protocol: agents bid on tasks based on capability/workload (MAS research)
- Restate-style checkpoint recovery: journal side-effect results, no deterministic replay requirement

**Agent dispatch strategy**: Hybrid with configuration — supports both modes
- **API dispatch** (default): Route through `pcg_router.rs` (HTTP/API calls) — simpler, cost-tracked automatically via VibeTransaction. Best for research, analysis, triage.
- **Executor dispatch**: Spawn via executor service (CLI process) — more powerful, full git worktree isolation. Best for coding, multi-step tasks.
- `flow_config` JSON stores `dispatch_mode: "api" | "executor" | "auto"` — default "api" for Phase 1
- `"auto"` mode: Phase 2 adds automatic routing based on task complexity/FlowType
- Both modes parse agent output into `AgentResponseEnvelope` for uniform handling

**CRM Deal FSM prototype** (added in sprint per gap analysis):
- `crm_deals.rs:move_deal_stage()` currently has NO stage ordering validation — any deal can move to any stage
- Add `valid_stage_transitions()` to `CrmPipelineStage` model — validates stage ordering within pipeline
- Same caller-aware guard pattern: agent-driven transitions strict, dashboard users permissive
- Prototypes the FSM validation pattern alongside agent flow FSM

**Scope note**: Full agent orchestration engine estimated at 2-3 weeks (per architecture-gaps-analysis.md). Phase 1 deliberately scopes to MVP: single-agent progression through phases with API dispatch. Multi-agent delegation, retry/circuit-break, and wide research orchestration deferred to Phase 2.

**Phase 1 scope**:
1. Poll `agent_flows` with `status IN ('planning', 'executing', 'verifying')`
2. Dispatch agent via `pcg_router.rs` (API mode) or executor service (CLI mode) based on `flow_config.dispatch_mode`
3. Parse response into `AgentResponseEnvelope` (from #7)
4. Advance phases using `AgentFlow::transition_to_phase()` (already exists)
5. Emit events via `AgentFlowEvent::emit_phase_started()` (already exists)
6. Handle approval gates (`human_approval_required = true`)
7. Publish `DomainEvent::FlowPhaseAdvanced` on broadcast channel AND persist to `agent_flow_events` table (Arch C)
   - Pattern: `emit_and_persist()` — broadcasts in-memory AND writes to DB in one call
   - Follows existing pattern in `ExecutionEngine::emit_flow_event()` (nora/src/execution/engine.rs:274-309)
   - SSE endpoint (`event_stream.rs`) already polls `agent_flow_events` — persistence = automatic SSE delivery

**Phase 2 migration note**: The inline stage-trigger agents (Scout, Astra, Cash, Lux) in `crm_deals.rs` currently spawn as direct async tasks with no retry/timeout/observability (roadmap P0 Blocker #3). Phase 2 will migrate these to dispatch through the agent flow engine, giving them the retry/timeout/FSM enforcement that this sprint builds. Phase 1: engine and inline agents coexist (engine opt-in via `ENABLE_AGENT_FLOW_ENGINE=1`).

**Reuse**:
- `AgentFlow::find_by_status()` — queries by status (agent_flow.rs)
- `AgentFlow::transition_to_phase()` — phase advancement (agent_flow.rs)
- `AgentFlowEvent::emit_phase_started()` — convenience emitter (agent_flow_event.rs)
- `TaskScheduler` (task_scheduler.rs:1-311) — dead code, never called. Adapt its `RwLock`-based concurrent execution tracking and `max_concurrent_executions` config pattern
- `pcg_router.rs` (622 lines) — model routing for LLM calls
- `spawn_workflow_schedule_loop()` — proven tokio interval pattern

**Wire into main.rs** (after line 352):
```rust
let flow_shutdown = shutdown_registry.register("agent_flow_executor");
agent_flow_executor::spawn(pool.clone(), flow_shutdown);
```

### 10. Dogfood Friction Logging (S0-19) [DOGFOOD — Day 6-7]
**Effort**: 1.5 days

Extend existing feedback system (`crates/server/src/routes/feedback.rs`).

**Backend**: Add fields to `SubmitFeedbackRequest`:
- `friction_area: Option<String>` — enum: workflow_editor, crm_pipeline, task_management, etc.
- `workflow_step: Option<String>` — what user was doing
- `workaround: Option<String>` — how they got around it
- `feature_request: Option<String>` — what they wish existed
- `source: Option<String>` — "dogfood" auto-tag
- Apply `#[derive(Validate)]` (from Arch E) with length limits

**Frontend**: "Report Friction" in sidebar More popover → structured form → stores as feedback with dogfood metadata. Reuse `FeedbackDialog` pattern.

---

## PR Strategy — Incremental Merges to Main

Large feature branches risk merge conflicts and hide broken code. Ship to main incrementally via focused PRs that each leave main in a stable, deployable state. No PR should introduce half-built features visible to users.

### PR #51: CI Hardening + Regressions + Access Control
**Items**: #1 (regressions + access control for contacts/pipelines), #2 (CI fixes + vite build), #3 (lint-staged)
**Why safe for main**: All fixes/infrastructure — no new features. CI starts enforcing quality. Pre-commit hooks prevent formatting drift. Access control fixes are security-critical.
**Gate**: All CI jobs pass after removing `continue-on-error`. All CRM endpoints have access checks.

### PR #52: Stability Fixes
**Items**: #4 (cooldown race), #5 (graceful shutdown + worker trait + registry)
**Why safe for main**: Bug fix (cooldown) + infrastructure (shutdown/workers). No user-facing changes. Worker trait is internal refactor — existing workers migrated, behavior unchanged.
**Gate**: `cargo test --workspace` passes. Manual test: SIGTERM → clean shutdown logged.

### PR #53: Cost System Bridge
**Items**: #6 (cost bridge)
**Why safe for main**: Redirects dashboard from dead `token_usage` table to `vibe_transactions` (source of truth). New aggregation endpoints are additive. Dashboard starts showing real data instead of $0.
**Gate**: Run an agent task → cost dashboard shows non-zero value.

### PR #54: Agent Engine + Response Protocol + Clarification + Deal FSM
**Items**: #7 (response protocol, merged with #9), #8 (clarification status), #9 (agent flow executor), CRM deal FSM prototype
**Why safe for main**: Agent engine **disabled by default** via feature flag (`ENABLE_AGENT_FLOW_ENGINE=1` env var). Response types are additive. `NeedsClarification` status is a new enum variant — no existing flows use it. Deal FSM validation is agent-only (dashboard users bypass). Migration adds nullable columns.
**Gate**: With flag enabled: create flow → engine progresses through phases. Without flag: no behavior change. `npm run generate-types:check` passes.

### PR #55: Dogfood Friction Logging
**Items**: #10 (friction logging)
**Why safe for main**: Extends existing feedback form with optional fields. Existing feedback still works unchanged. New UI button is additive.
**Gate**: Submit standard feedback → works as before. Submit friction report → stored with metadata.

### PR Dependency Chain
```
PR #51 (CI + regressions + access control) → independent
PR #52 (stability) → independent
PR #53 (cost bridge) → independent
PR #54 (agent engine + protocol + clarification + deal FSM) → after #52 (needs worker trait) + clean CI
PR #55 (friction logging) → independent
```

PRs #51, #52, #53, #55 can merge to main in any order.
PR #54 merges last (depends on #52 for worker trait).

---

## Execution Order & Parallelization

| Day | Dev 1 | Dev 2 | Dev 3 (Claude) |
|-----|-------|-------|----------------|
| 1 | #1 (regressions) | #2a-2b (fmt + clippy) | #3 (lint-staged) |
| 2 | #4 (cooldown race) | #2c-2e (test fixes + CI strict) | #5a-5b (shutdown + registry) |
| 3 | #6 (cost bridge) | #5c (worker trait) | #7 (response protocol) |
| 4 | #7 cont'd | #8 (clarification status) | #9 start (agent engine) |
| 5-7 | #9 (agent engine) | #10 (friction logging) | #9 (agent engine) |
| 8 | QA + verification | E2E tests | Bug fixes |

**Critical path**: #5c (worker trait) → #7 (response protocol) → #8 (clarification) → #9 (agent engine)
**Independent**: #1, #2, #3, #4, #6, #10

---

## Verification Plan

### Per-item:
1. **Regressions**: `GET /api/crm/deals/kanban/:id` without org membership → 403
2. **CI**: Push branch → all CI jobs pass (fmt, clippy, test, frontend-check, types-check)
3. **Lint-staged**: Stage `.tsx` with bad formatting → pre-commit auto-fixes
4. **Cooldown**: 2 concurrent webhook requests within cooldown → only 1 fires
5. **Graceful shutdown**: `kill -SIGTERM <pid>` → logs drain message, finishes in-flight requests, all workers log shutdown
6. **Cost bridge**: Run agent task → `ai-usage` dashboard shows non-zero cost
7. **Response protocol**: Serialize/deserialize `AgentResponseEnvelope` → round-trips correctly
8. **Clarification**: Agent returns `NeedsClarification` → flow pauses → UI shows question → user responds → flow resumes
9. **Agent engine**: Create task → assign agent → flow auto-progresses: planning → executing → verifying → completed
10. **Friction logging**: Submit friction report → appears as tagged task in feedback project

### End-to-end dogfood test:
1. Login → create task → assign to agent
2. Agent flow engine picks up, spawns planner
3. Planner returns `Done` → engine advances to executing
4. Executor works → returns `Done` with artifacts
5. Verifier checks → flow completes
6. Cost dashboard shows $ spent
7. User encounters friction → submits friction report → captured with metadata

### CI commands:
```bash
flox activate -- cargo fmt --all -- --check
flox activate -- cargo clippy --all --all-targets -- -D warnings
flox activate -- cargo test --workspace
cd frontend && npm run lint && npx tsc --noEmit
npm run generate-types:check
FRONTEND_PORT=3000 npx playwright test --reporter=list
```

---

## Files to Create/Modify

| File | Action | Sprint Item |
|------|--------|-------------|
| `crates/server/src/routes/crm_deals.rs` | Modify — access control + DbUuid | #1 |
| `crates/server/src/helpers.rs` | Create — `parse_db_uuid_param()`, shared route helpers | #1 |
| `crates/server/src/routes/crm_deal_transitions.rs` | Create — extracted stage transitions from crm_deals.rs | #1 |
| `crates/server/src/routes/crm_deal_automations.rs` | Create — extracted research/proposal/deck triggers | #1 |
| `crates/server/src/routes/crm_contacts.rs` | Modify — add access control to all 12 endpoints | #1 |
| `crates/server/src/routes/crm_pipelines.rs` | Modify — add access control to all 13 endpoints | #1 |
| ~~`frontend/src/pages/call-intake.tsx`~~ | ~~Modify — replace raw fetch~~ | ~~#1~~ (already uses `makeRequest`) |
| `.github/workflows/ci.yml` | Modify — remove continue-on-error | #2 |
| All Rust crates | Modify — `cargo fmt --all` | #2 |
| `crates/alpha-protocol-core/` | Modify — fix compile errors | #2 |
| `.githooks/pre-commit` | Create — lint-staged hook | #3 |
| `crates/db/src/models/workflow_trigger.rs` | Modify — atomic cooldown | #4 |
| `crates/server/src/routes/data_source_workflows.rs` | Modify — update cooldown callers | #4 |
| `crates/server/src/main.rs` | Modify — graceful shutdown + worker registry | #5, #9 |
| `crates/server/src/workers/mod.rs` | Create — BackgroundWorker trait + registry | #5 |
| `crates/server/src/routes/vibe_treasury.rs` | Modify — add cost aggregation endpoints (by-model, by-date, by-project) | #6 |
| `crates/db/src/models/vibe_transaction.rs` | Modify — add GROUP BY aggregation queries | #6 |
| `frontend/src/pages/ai-usage.tsx` | Modify — redirect from token_usage to vibe endpoints | #6 |
| `crates/db/src/models/crm_deal.rs` | Modify — add deal stage FSM validation + pipeline membership check | #9 |
| `crates/db/src/models/crm_pipeline.rs` | Modify — add `valid_next_stages()` method on CrmPipelineStage | #9 |
| `crates/server/src/events/mod.rs` | Create — DomainEvent enum + broadcast channel | #9 |
| `crates/db/src/models/agent_response.rs` | Create — response envelope types | #7 |
| `crates/db/src/models/agent_flow.rs` | Modify — NeedsClarification status | #8 |
| `crates/server/src/routes/agent_flows.rs` | Modify — clarification endpoint | #8 |
| `crates/server/src/agent_flow_executor.rs` | Create — background worker loop | #9 |
| `crates/server/src/routes/feedback.rs` | Modify — friction fields | #10 |
| `frontend/src/components/feedback/` | Create/modify — friction form | #10 |
| `crates/db/migrations/20260413000000_*.sql` | Create — new migrations (NeedsClarification column, friction fields) | #8, #10 |
| `crates/server/src/middleware/access_control.rs` | Modify — add `require_org_membership()` method to `AccessContext` | #1 |
| `frontend/src/constants/artifact.ts` | Create — shared `VIDEO_EDIT_TYPES` constant (dedup 3 files) | #1 |
| `frontend/src/constants/query-config.ts` | Create — centralized query staleTime/refetchInterval constants | #6 |
| `rustfmt.toml` | Create — `ignore` list for vendored crates | #2 |

---

## Modularity Wins (2026-03-19)

Low-effort extractions to apply DURING sprint implementation (not separate tasks).

### In-Sprint Extractions (~4h total, embedded in PRs)

**Backend**:
| Extraction | Effort | File | PR |
|-----------|--------|------|-----|
| `require_org_membership()` → `AccessContext` method | 30m | `middleware/access_control.rs` | #51 |
| `parse_db_uuid_param()` helper | 20m | `crates/server/src/helpers.rs` (new) | #51 |
| `crm_deals.rs` → `crm_deal_transitions.rs` + `crm_deal_automations.rs` | 1h | `routes/` | #51 |
| `BackgroundWorker` trait + `spawn_worker()` | 30m | `workers/mod.rs` | #52 |
| `AgentFlowExecutorConfig::from_env()` | 15m | agent flow executor | #54 |

**Frontend**:
| Extraction | Effort | File | PR |
|-----------|--------|------|-----|
| `VIDEO_EDIT_TYPES` → `constants/artifact.ts` | 15m | 3 files deduped | #51 |
| Date formatting → use `formatDate()` everywhere | 30m | 3-4 files | #53 |
| `QUERY_CONFIG` constants | 30m | `constants/query-config.ts` | #53/#54 |

### Deferred (noted for future sprints)

**Backend**: env var helpers (96+ sites), model error macro (30+ files), inline `FromRow` cleanup (83+ structs), DB error wrapping trait (1,060+ `ApiError::InternalError` conversions), large file splits beyond `crm_deals.rs` (brand.rs 1,863L, tasks.rs 1,572L, intelligence.rs 1,443L, sovereign_storage.rs 2,699L). Note: `main.rs` (588L) background task extraction is covered by sprint item #5.

**Frontend**: stream hook backoff utility (4 hooks), query/mutation hook factory, `useAsyncModalAction()` hook (72 NiceModal dialogs share loading/error pattern), `<LoadingButton>` component (20+ dialog submit buttons), `<EmptyState>` component (standardize empty state UX), localStorage typed wrapper (63 raw `localStorage` calls), validation utils (20+ inline `required` checks), spinner standardization (mixed Loader/Loader2/RefreshCw/custom), large component splits (NoraAssistant 1,027L, StagingReviewPanel 933L, topsi.tsx 937L)

### Large File Inventory (>1,000 lines)

**Backend routes**: `crm_deals.rs` (2,923), `sovereign_storage.rs` (2,699), `brand.rs` (1,863), `data_source_workflows.rs` (1,691), `tasks.rs` (1,572), `intelligence.rs` (1,443), `workflow_staging.rs` (1,401), `meet.rs` (1,372)

**Backend models**: `crm_deal.rs` (1,114), `topiclip.rs` (1,030), `task.rs` (1,010)

**Frontend components**: `NoraAssistant.tsx` (1,027), `useProjectData.ts` (952), `StagingReviewPanel.tsx` (933), `meeting-mode/index.tsx` (887), `editor/index.tsx` (864)

**Frontend pages**: `topsi.tsx` (937), `AgentSettings.tsx` (918), `crm.tsx` (898), `StagingTab.tsx` (856), `business-reports.tsx` (827)

---

## Gap Analysis Findings (2026-03-19, two research phases)

### Phase 1: Codebase Audit (8 parallel agents)

#### Scope corrections
1. **crm_deals.rs access control was already complete** — all 20 handlers protected. Real gaps: `crm_contacts.rs` (12 endpoints, ZERO checks) and `crm_pipelines.rs` (13 endpoints, ZERO checks). Retargeted item #1.
2. **Cost bridge misdirected** — plan said "populate cost_cents in TokenUsage::create()". Reality: `TokenUsage::create()` is rarely called; all billing flows through `vibe_transactions`. Redirected dashboard to read from source of truth instead of patching dead table.
3. **Background task count was 14, actual is 16** — missed PR Monitor Service (stored JoinHandle, never awaited) and APN Node subprocess (detached, no shutdown). Updated item #5.
4. **Items #7 and #9 merged** — response protocol and engine are tightly coupled; building separately creates design-in-a-vacuum risk.

#### New items added
- **1d/1e**: Access control for crm_contacts.rs + crm_pipelines.rs (security-critical)
- **2f**: `vite build` step in CI (frontend build failures undetected)
- **5d**: APN Node subprocess cleanup on shutdown
- **CRM deal FSM prototype**: `move_deal_stage()` has no ordering validation — prototype FSM alongside agent flow FSM

#### Confirmed (no changes needed)
- Item #3 (lint-staged): stash exists as described, ready to apply
- Item #4 (cooldown race): exact check-then-act race confirmed at lines 172/249 in webhook handler
- Item #8 (NeedsClarification): existing FlowStatus enum ready for new variant
- Item #10 (friction logging): current feedback system extensible via JSON metadata, no schema migration needed

### Phase 2: Implementation Readiness Audit (7 parallel agents)

#### Build state
- 23 workspace crates, nightly-2025-05-18 toolchain
- Latest migration: `20260412000000` — next should be `20260413000000`
- `crates/server/src/workers/`, `events/`, `agent_flow_executor.rs` — all need creating (none exist)
- `validator` crate not in any Cargo.toml — needs adding
- Existing lint suppressions only in nora/services WIP code (5 files)

#### Dispatch patterns for agent engine
- **API mode**: `WorkflowLLMService::completion_with_tools(pool, messages, tools, model_hint, max_tokens, temp)` → returns `(LLMResponse, RoutingMetadata)`. Supports tool-calling and structured output. Internally routes via `pcg_router::route_completion()`.
- **Executor mode**: `container.start_execution(task_attempt, executor_action)` → spawns `CodingAgent::spawn()` subprocess in git worktree. 8 executor backends (Claude, Gemini, Qwen, Cursor, CodeX, Duck, AMP, OpenCode).
- Both paths are well-defined; agent flow executor can dispatch via either based on `flow_config.dispatch_mode`.

#### Frontend readiness
- `useAgentFlows`, `useAgentFlowEvents`, `useAgentFlowMutations` hooks all exist in `frontend/src/hooks/useAgentFlows.ts`
- `agentFlowsApi` client has complete CRUD + transition + approval + event endpoints
- Query keys factory pattern in `frontend/src/lib/query-keys.ts` — `agentFlowKeys.*` ready
- `frontend/src/pages/vibe.tsx` already displays `VibeTransaction` history with cost data
- `frontend/src/pages/ai-usage.tsx` reads from `token_usage` (dead data) — needs redirect to vibe

#### Cost bridge simplification
- Frontend already has `vibeApi` client with `getConfig()`, `verifyDeposit()`, transaction list
- Existing vibe routes in `crates/server/src/routes/vibe_treasury.rs` — can extend rather than create new module
- `VibeTransaction::sum_by_source()` aggregation exists; need GROUP BY model/provider/date variants
- `ModelPricing::calculate_cost()` returns `CostEstimate { cost_cents, cost_usd, cost_vibe }`

#### Deal FSM — cross-pipeline bug confirmed
- `move_to_stage()` at `crm_deal.rs:404-537` accepts ANY `stage_id` without checking pipeline membership
- `CrmPipelineStage` has `position: i32` (unique per pipeline), `is_closed`, `is_won`, `stage_type` — all FSM building blocks exist
- `CrmPipelineStage::reorder()` method exists for stage position management
- Stage ordering: `ORDER BY position` in `find_by_pipeline()` — canonical sequence defined

#### DomainEvent — not from scratch
- `tokio::sync::broadcast` already used in `crates/nora/src/execution/events.rs` (EventBroadcaster, capacity 1000) and `crates/nora/src/coordination.rs` (CoordinationManager)
- SSE event streaming for agent flows uses DB polling (500ms) in `event_stream.rs`
- `DeploymentImpl` does not hold a broadcast channel yet — need to add if centralizing
- Pattern: create `DomainEvent` enum, store `broadcast::Sender` in app state, subscribe in workers

#### Lint-staged stash scope
- Stash `stash@{0}` contains **657 files** — mass import reordering via `simple-import-sort` plugin
- Core config: `.githooks/pre-commit`, `lint-staged@^16.4.0`, `eslint-plugin-simple-import-sort@^12.1.1`
- Also increases ESLint max-warnings from 110 → 180
- Must be applied FIRST to avoid merge conflicts with all other frontend changes

### Phase 2 Addendum: Pipeline Roadmap Validation

Cross-referenced against `planning/roadmap/reference/2026-03-18--reference--consolidated-pipeline-roadmap.md` and `2026-03-18--reference--dealflow-pipeline-status.md` from `research/5-year-product-roadmap` branch.

#### Sprint items that directly address roadmap needs
| Roadmap Need | Sprint Item | Roadmap Priority |
|---|---|---|
| Agent Flow Orchestration Engine (P0 Blocker #3) | Item #9 | P0 — "largest remaining architecture gap" |
| Human gate enforcement | Item #9 FSM + Item #8 NeedsClarification | Part of Horizon 1 #1-#2 |
| Agent retry/re-trigger | Item #9 (basic re-dispatch) | Part of Blocker #3 |
| CI pipeline (tech debt CRITICAL #4) | Item #2 | Critical |
| Graceful shutdown (tech debt HIGH) | Item #5 | High |
| Org-level authz on CRM routes | Items #1d, #1e | Listed as "Deferred" in roadmap — we're fixing it |

#### Roadmap features noted but NOT in sprint scope
- **F11**: Call scheduling input UI (45 min, frontend UX, not engine work)
- **F3**: Person invite in `mark_deal_won` (45 min, downstream of engine)
- **Follow Up stage migration** (stage exists, cleanup only)
- **Workflow triggers** (cron + entity-change — Phase 2+)
- **assign_to_agent / http_request nodes** (workflow node types, Phase 2+)

#### Architectural direction note
Roadmap shows agents (Scout, Astra, Cash, Lux) as **direct async spawns on stage advance** in `crm_deals.rs`. Our engine dispatches via background worker. Phase 1: coexist (engine is opt-in via `ENABLE_AGENT_FLOW_ENGINE=1`). Future: engine subsumes inline spawns for retry/timeout/observability benefits. The inline spawns in `crm_deals.rs` are NOT modified in this sprint.

---

## Known Limitations (Phase 1)

- **SSE is polling-based** (`event_stream.rs` queries DB every 500ms per connected client). O(clients) database load. Adequate for internal dogfooding (<10 concurrent users). Phase 2: replace with `tokio::sync::broadcast` channel for O(1) DB queries.
- **No internal pub/sub** for state changes — CRM automations still poll hourly. DomainEvent broadcast (Arch C) is first step but won't replace all polling in Phase 1.
- **SQLite single-writer** limits concurrent agent executions. `max_concurrent` config (default 3) keeps this safe. Phase 2: PostgreSQL if scale demands it.

## Out of Scope (Deferred)

### Agent Engine Phase 2 (from FSM/orchestration research)
- Wide research orchestration — parallel agent execution with `onDone` sync
- Traceback transitions (`s_i → s_j`, j < i) — go back N steps with error context instead of restart (MetaAgent, 85% checkpoint passage)
- Null transitions (self-loops) — iterative refinement within a state without progressing the FSM
- Parallel state regions — XState statechart pattern for concurrent agent work streams
- Artifact FSMs — `draft → in_review → approved → complete` lifecycle per agent output
- ESA matrix tooling — auto-generate Event-State Analysis for workflow definitions, highlight specification gaps
- Context isolation — subtask agents get own conversation context, parent receives summary only (RooRoo/Anthropic pattern)
- Hierarchical Task DAG (HTDAG) — lazy decomposition of non-atomic tasks (Deep Agent pattern)
- Contract Net Protocol — agents bid on tasks based on capability/workload matching
- Restate-style checkpoint recovery — journal side-effect results instead of requiring deterministic replay
- Hierarchical/nested states — orchestrator sees phases, executing agent sees sub-steps
- Guard conditions with DB queries — `start` requires `assigned_agent IS NOT NULL`, `complete` requires all subtasks done
- Layered validation — deterministic checks (schema, completeness, consistency) before expensive LLM review

### Cost & Routing
- Cost-tiered model routing (S0-08) — needs engine working first
- CAPO per-task tracking (S0-07) — needs cost bridge first
- Prompt caching (S0-10) — needs routing first
### Infrastructure & Quality
- Full input validation framework (S0-13) — progressive adoption started here
- Webhook retry worker (S0-14) — fields exist, not urgent
- Rate limiting (S0-17) — internal only
- DbUuid Phase C/D — style debt (84 `Path<Uuid>` remaining)

### Observability & Production Readiness
- OpenTelemetry distributed tracing — `tracing-opentelemetry` crate + OTLP export. Needed for Stage 1 pilot, not Phase 0
- Three-endpoint health checks (live/ready/startup) — Kubernetes-style, needed for production deploy
- JSON structured log output for production — `tracing-subscriber` with `fmt::layer().json()`
- AI-specific tracing attributes per workflow step (model, tokens, cost, duration)

### Platform & Multi-Agent
- A2A (Agent-to-Agent) Protocol — Google's emerging standard for agent discovery/delegation (v0.3 with gRPC). Future direction for agent registry
- BDI (Belief-Desire-Intention) model — formal commitment strategies for agent planning
- Blackboard architecture — shared knowledge base for multi-agent collaboration on same project
- Workflow marketplace/templates — JSON definitions with parameterized credentials, import/export
- Workflow versioning — immutable versioned definitions, running executions pinned to creation-time version
- Auto-generate workflow FSMs from task descriptions (MetaAgent approach)

### Content & Ops
- Vision/mission anchor doc (S0-11) — writing, not code
- Editron/social media workflow migration — needs engine first

---

## Research Findings (Pre-Implementation Audit)

### Codebase Statistics
- **443 total tests**: 375 `#[test]` + 68 `#[tokio::test]` across 94+ files
- **Test infrastructure**: No integration test dirs for server/db — tests colocated with source
- **Git service tests**: 63 tests in `crates/services/tests/` using `tempfile::TempDir` + `git2`

### Dead Code Identified
- **`TaskScheduler`** (`crates/server/src/task_scheduler.rs`, 310 lines): Exported in `lib.rs` but **never instantiated** in main.rs. Generic over `Deployment` trait, uses `RwLock` for concurrent execution tracking. Can adapt its patterns for agent flow executor (#9).

### Background Worker Audit (13+ spawns in main.rs)
| Worker | Interval | Has CancellationToken? |
|--------|----------|----------------------|
| File search cache warmer | One-off | No |
| Sovereign storage sync | 5s | Yes (never signaled) |
| APN peer cleanup | 60s | No |
| Automation loop | 1 hour | No |
| Workflow schedule loop | 5 min | Yes (never signaled) |
| VIBE deposit watcher | 30s | No |
| VIBE withdrawal executor | 60s | No |
| Meeting stale cleanup | 2 min | No |
| APN data service | 30s | Yes (config-based) |

**Confirmed**: CancellationTokens created but `root.cancel()` never called → graceful shutdown (#5) is critical.

### SSE / Real-time Architecture
- **Polling-based SSE** in `event_stream.rs`: queries DB every 500ms per client
- Uses `stream::unfold()` with `AgentFlowEvent::find_since(pool, flow_id, since_timestamp)`
- 15s keep-alive interval
- O(clients) DB load — adequate for internal dogfooding, needs broadcast channel for scale

### Error Handling
- **Well-structured**: Centralized `ApiError` enum in `crates/server/src/error.rs` (323 lines)
- Domain-specific error types with `From` impls for automatic `?` propagation
- Route handlers all return `Result<ResponseJson<ApiResponse<T>>, ApiError>`
- **No changes needed** for this sprint

### Access Control Architecture
- Auth middleware applies `Extension<AccessContext>` to all protected routes
- Authorization is **per-handler** (`require_admin()`, `require_viewer()`, `require_editor()`)
- Hierarchical: Admin bypass → project_members → org membership → client membership
- **PR #50 regression**: New routes added without calling `require_viewer/editor()` — confirms #1 fix

### Configuration Pattern
- Each service defines `Config` struct with `from_env()`
- Agent flow executor should follow same pattern: `AgentFlowExecutorConfig::from_env()`
- Feature flag pattern established: `APN_DATA_SERVICE_ENABLED`, `SOVEREIGN_STORAGE_ENABLED`
- Agent engine uses: `ENABLE_AGENT_FLOW_ENGINE=1` (disabled by default)

### Frontend Agent Flow Hooks (Already Built)
- `useAgentFlows()`: React Query, 30s staleTime
- `useAgentFlowsAwaitingApproval()`: 15s staleTime
- `useAgentFlowEvents()`: 10s staleTime, conditional on flowId
- Mutations: `approve()`, `transitionPhase()`, `complete()`, `requestApproval()`
- Query key factory in `lib/query-keys.ts:450-458`

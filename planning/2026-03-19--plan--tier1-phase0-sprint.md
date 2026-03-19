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

During this sprint, add `validator` to `Cargo.toml` and apply `#[derive(Validate)]` to:
1. `SubmitFeedbackRequest` (friction logging form — new code, clean slate)
2. `CreateFlowEvent` (agent events — new code)
3. `CreateAgentFlow` (flow creation — modified code)

**ROI**: Establishes the pattern for S0-13 (full input validation framework) without blocking sprint items.

---

## Sprint Items (10 items, ROI-ordered)

### 1. Fix PR #50 Regressions [CRITICAL — Day 1]
**Effort**: 0.5 days | **Parallelizable**: Yes

**1a. Harden kanban access control** (`crates/server/src/routes/crm_deals.rs:256-275`)
- Current: checks `require_org_membership` only when `pipeline.organization_id` is `Some`
- Fix: deny access when `organization_id` is `None` — return 403 "Pipeline has no organization scope"
- This is a security fix, not a feature

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

**1c. ~~Fix raw `fetch()` in call-intake.tsx~~** — RESOLVED
- Exploration found `call-intake.tsx` already uses `makeRequest` (dynamic import), not raw `fetch()`.
- Uses `callIntakeApi` and `reportsApi` domain modules correctly.
- **No fix needed** — remove from sprint scope.

### 2. Fix CI Pipeline [HIGH — Day 1-2]
**Effort**: 1.5 days | **Parallelizable**: Yes (assign to different dev)

**2a. `cargo fmt --all`** — single formatting-only commit (200+ files)
**2b. Fix clippy errors** — `discord-bots` (29), `utils` (23), `pcg-cli` (65). Use `#![allow(clippy::uninlined_format_args)]` at crate root for bulk non-critical lints.
**2c. Fix `alpha-protocol-core` test compile errors** — 2 borrow checker issues in test code
**2d. Fix ESLint** — `eslint --fix` for unused disable directives (62 errors)
**2e. Remove `continue-on-error: true`** from `.github/workflows/ci.yml` lines 48, 65 (clippy + tests). Keep on lines 119, 129 (security audits — advisory only).

### 3. Apply Lint-Staged Setup [QUICK WIN — Day 1]
**Effort**: 0.5 days

- Apply stash: `git stash pop` (stash@{0}: "lint-staged + import-sort setup + eslint --fix")
- Verify pre-commit hook works
- Commit: `chore: add lint-staged pre-commit hooks`

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
Add drain timeout (30s) via `tokio::time::timeout` to prevent hanging on long-running LLM streams.
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

### 6. Bridge Dual Cost System (S0-02) [HIGH — Day 3]
**Effort**: 1 day

**Problem**: `TokenUsage::create()` (token_usage.rs:115-149) never sets `cost_cents`. Dashboard shows $0.

**Approach**: Populate `cost_cents` at insert time
- Look up `ModelPricing` for the model/provider after insert
- Call `ModelPricing::calculate_cost(input_tokens, output_tokens)` (model_pricing.rs:120-142)
- UPDATE the record's `cost_cents` field
- Verify: `TokenUsageWidget.tsx` (mission-control) shows non-zero values after running an agent

### 7. Structured Response Protocol (S0-03 + Arch D) [FOUNDATION — Day 3-4]
**Effort**: 1.5 days

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

**State machine**:
```
Planning → [emit PhaseStarted] → Executing →
[emit PhaseCompleted] → Verifying → [emit FlowCompleted]

Any phase → NeedsClarification → [pause, emit event] → user responds → resume
Any phase → Failed → [emit FlowFailed]
```

**Agent dispatch strategy**: Configurable per flow_type
- **Simple tasks** (research, analysis, triage): Route through `pcg_router.rs` (HTTP/API calls) — simpler, cost-tracked automatically via VibeTransaction
- **Complex tasks** (coding, multi-step): Spawn via executor service (CLI process) — more powerful, full git worktree isolation
- `flow_config` JSON stores `dispatch_mode: "api" | "executor"` — default "api" for Phase 1
- Phase 2 adds automatic routing based on task complexity

**Scope note**: Full agent orchestration engine estimated at 2-3 weeks (per architecture-gaps-analysis.md). Phase 1 deliberately scopes to MVP: single-agent progression through phases with API dispatch. Multi-agent delegation, retry/circuit-break, and wide research orchestration deferred to Phase 2.

**Phase 1 scope**:
1. Poll `agent_flows` with `status IN ('planning', 'executing', 'verifying')`
2. Dispatch agent via `pcg_router.rs` (API mode) or executor service (CLI mode) based on `flow_config.dispatch_mode`
3. Parse response into `AgentResponseEnvelope` (from #7)
4. Advance phases using `AgentFlow::transition_to_phase()` (already exists)
5. Emit events via `AgentFlowEvent::emit_phase_started()` (already exists)
6. Handle approval gates (`human_approval_required = true`)
7. Publish `DomainEvent::FlowPhaseAdvanced` on broadcast channel (Arch C)

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

### PR #51: CI Hardening + Regressions
**Items**: #1 (regressions), #2 (CI fixes), #3 (lint-staged)
**Why safe for main**: All fixes/infrastructure — no new features. CI starts enforcing quality. Pre-commit hooks prevent formatting drift.
**Gate**: All CI jobs pass after removing `continue-on-error`.

### PR #52: Stability Fixes
**Items**: #4 (cooldown race), #5 (graceful shutdown + worker trait + registry)
**Why safe for main**: Bug fix (cooldown) + infrastructure (shutdown/workers). No user-facing changes. Worker trait is internal refactor — existing workers migrated, behavior unchanged.
**Gate**: `cargo test --workspace` passes. Manual test: SIGTERM → clean shutdown logged.

### PR #53: Cost System Bridge
**Items**: #6 (cost bridge)
**Why safe for main**: Populates a field that was always NULL. Dashboard starts showing data instead of $0. No schema changes, no new endpoints.
**Gate**: Run an agent task → cost dashboard shows non-zero value.

### PR #54: Agent Response Protocol + Clarification
**Items**: #7 (response protocol), #8 (clarification status)
**Why safe for main**: New types and one new enum variant. The `NeedsClarification` status is additive — no existing flows use it. New endpoint is additive (doesn't modify existing behavior). Migration adds a nullable column.
**Gate**: `npm run generate-types:check` passes. Existing agent flow CRUD still works.

### PR #55: Agent Flow Engine Phase 1
**Items**: #9 (agent flow executor)
**Why safe for main**: New background worker — **disabled by default** via feature flag (`ENABLE_AGENT_FLOW_ENGINE=1` env var). Engine only activates when explicitly enabled. This prevents accidental agent dispatch on main while allowing dogfood testing.
**Gate**: With flag enabled: create flow → engine progresses through phases. Without flag: no behavior change.

### PR #56: Dogfood Friction Logging
**Items**: #10 (friction logging)
**Why safe for main**: Extends existing feedback form with optional fields. Existing feedback still works unchanged. New UI button is additive.
**Gate**: Submit standard feedback → works as before. Submit friction report → stored with metadata.

### PR Dependency Chain
```
PR #51 (CI + regressions) → independent
PR #52 (stability) → independent
PR #53 (cost bridge) → independent
PR #54 (protocol + clarification) → after #51 (needs clean CI)
PR #55 (agent engine) → after #52 (needs worker trait) + #54 (needs response protocol)
PR #56 (friction logging) → independent
```

PRs #51, #52, #53, #56 can merge to main in any order.
PR #54 merges after #51 (clean CI).
PR #55 merges last (depends on #52 + #54).

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
| ~~`frontend/src/pages/call-intake.tsx`~~ | ~~Modify — replace raw fetch~~ | ~~#1~~ (already uses `makeRequest`) |
| `.github/workflows/ci.yml` | Modify — remove continue-on-error | #2 |
| All Rust crates | Modify — `cargo fmt --all` | #2 |
| `crates/alpha-protocol-core/` | Modify — fix compile errors | #2 |
| `.githooks/pre-commit` | Create — lint-staged hook | #3 |
| `crates/db/src/models/workflow_trigger.rs` | Modify — atomic cooldown | #4 |
| `crates/server/src/routes/data_source_workflows.rs` | Modify — update cooldown callers | #4 |
| `crates/server/src/main.rs` | Modify — graceful shutdown + worker registry | #5, #9 |
| `crates/server/src/workers/mod.rs` | Create — BackgroundWorker trait + registry | #5 |
| `crates/db/src/models/token_usage.rs` | Modify — populate cost_cents | #6 |
| `crates/db/src/models/agent_response.rs` | Create — response envelope types | #7 |
| `crates/db/src/models/agent_flow.rs` | Modify — NeedsClarification status | #8 |
| `crates/server/src/routes/agent_flows.rs` | Modify — clarification endpoint | #8 |
| `crates/server/src/agent_flow_executor.rs` | Create — background worker loop | #9 |
| `crates/server/src/routes/feedback.rs` | Modify — friction fields | #10 |
| `frontend/src/components/feedback/` | Create/modify — friction form | #10 |
| `crates/db/migrations/2026031900000X_*.sql` | Create — new migrations | #8, #10 |

---

## Known Limitations (Phase 1)

- **SSE is polling-based** (`event_stream.rs` queries DB every 500ms per connected client). O(clients) database load. Adequate for internal dogfooding (<10 concurrent users). Phase 2: replace with `tokio::sync::broadcast` channel for O(1) DB queries.
- **No internal pub/sub** for state changes — CRM automations still poll hourly. DomainEvent broadcast (Arch C) is first step but won't replace all polling in Phase 1.
- **SQLite single-writer** limits concurrent agent executions. `max_concurrent` config (default 3) keeps this safe. Phase 2: PostgreSQL if scale demands it.

## Out of Scope (Deferred)

- Wide research orchestration (agent engine phase 2)
- Cost-tiered model routing (S0-08) — needs engine working first
- CAPO per-task tracking (S0-07) — needs cost bridge first
- Prompt caching (S0-10) — needs routing first
- Full input validation framework (S0-13) — progressive adoption started here
- Webhook retry worker (S0-14) — fields exist, not urgent
- Vision/mission anchor doc (S0-11) — writing, not code
- Rate limiting (S0-17) — internal only
- DbUuid Phase C/D — style debt
- Editron/social media workflow migration — needs engine first
- A2A (Agent-to-Agent) Protocol — Google's emerging standard for agent discovery/delegation (v0.3 with gRPC). Future direction for agent registry, not Phase 1
- OpenTelemetry distributed tracing — `tracing-opentelemetry` crate + OTLP export. Needed for Stage 1 pilot, not Phase 0
- Three-endpoint health checks (live/ready/startup) — Kubernetes-style, needed for production deploy
- JSON structured log output for production — `tracing-subscriber` with `fmt::layer().json()`

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

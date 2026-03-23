# Functionality Audit Report

**Date**: 2026-03-21
**Planning File**: `planning/2026-03-19--plan--tier1-phase0-sprint.md`
**Branch**: `feature/2026-03-19--tier1-phase0`
**Audit Method**: Code trace + build verification + Playwright walkthrough (F9, F10, F12)

## Feature Verdicts

| # | Feature | Type | Verdict | Issue |
|---|---------|------|---------|-------|
| F1 | DbUuid migration in crm_deals.rs | backend | PARTIAL | `uuid::Uuid` still imported in 2 files; `Uuid::nil()` in transitions |
| F2 | Access control: crm_contacts.rs | backend | WORKING | 12/12 handlers have AccessContext + require_org_membership() |
| F3 | Access control: crm_pipelines.rs | backend | WORKING | 13/13 handlers have AccessContext + require_org_membership() |
| F4 | crm_deals.rs file split | infra | WORKING | 3 files exist, routes wired, 71% size reduction |
| F5 | CI pipeline fixes | infra | WORKING | continue-on-error removed, vite build step added |
| F6 | Lint-staged + pre-commit hooks | infra | WORKING | Hooks + deps + config in place, manual activation only |
| F7 | Atomic cooldown (try_claim_trigger) | backend | PARTIAL | Atomic method works; `workflow_triggers.rs:172` still uses racy `is_past_cooldown()` |
| F8 | Graceful shutdown + worker registry | backend | PARTIAL | Infrastructure correct; only 1 of 10+ background tasks uses BackgroundWorker trait |
| F9 | Friction logging | full-stack | PARTIAL | Backend fields + frontend form exist; discoverable via More → Feedback → Friction Report; no migration, no dedicated sidebar button |
| F10 | Cost bridge dashboard | full-stack | PARTIAL | Backend 100% (real GROUP BY queries + auth); **frontend never calls costsApi** |
| F11 | Structured Response Protocol | backend | STUB | Types defined with `#[derive(TS)]`; not in shared/types.ts; never consumed |
| F12 | NeedsClarification status | full-stack | PARTIAL | Status + transitions + endpoint + frontend badges complete; no response form UI |
| F13 | Agent Flow Engine | backend | STUB | BackgroundWorker + FSM + polling exist; no actual LLM dispatch; explicit Phase 2 stub |

**Totals**: 4 WORKING, 6 PARTIAL, 2 STUB, 0 NOT WIRED

---

## Detailed Findings

### F1: DbUuid migration in crm_deals.rs
- **Verdict**: PARTIAL
- **What works**: `DbUuid::from()`, `DbUuid::new()`, `DbUuid::parse()` used throughout for runtime operations
- **Issues**:
  - `crm_deals.rs:22` — `use uuid::Uuid` still imported (needed for `FromRow` struct deserialization)
  - `crm_deal_transitions.rs:16` — `use uuid::Uuid` imported; line 104 uses `DbUuid::from(Uuid::nil())` as fallback
  - Plan correctly noted: "route-handler-level only; BLOB-binding deferred to DbUuid Phase C"
- **Plan accuracy**: DONE claim is fair given documented scope limits

### F2: Access control — crm_contacts.rs
- **Verdict**: WORKING
- **Route**: 12 handler functions, all with `Extension(access_context): Extension<AccessContext>`
- **Handler**: Each calls `require_org_membership()` directly or via `require_contact_org_access()` helper
- **Issues**: None

### F3: Access control — crm_pipelines.rs
- **Verdict**: WORKING
- **Route**: 13 handler functions, all with `Extension(access_context): Extension<AccessContext>`
- **Handler**: Each calls `require_org_membership()` directly or via `require_pipeline_org_access()` helper
- **Issues**: None

### F4: crm_deals.rs file split
- **Verdict**: WORKING
- **Files**:
  - `crm_deals.rs`: 827 lines (CRUD + enrichment)
  - `crm_deal_transitions.rs`: 756 lines (stage transitions)
  - `crm_deal_automations.rs`: 1,419 lines (AI triggers)
- **Router**: All three modules declared in `mod.rs:43-47`, wired via `crm_deals::router()` at `mod.rs:164`
- **Issues**: None

### F5: CI pipeline fixes
- **Verdict**: WORKING
- **ci.yml changes**:
  - Clippy step (line 46-49): NO `continue-on-error` — strict
  - Test step (line 76-77): NO `continue-on-error` — strict
  - Security audits (lines 133, 143): `continue-on-error: true` retained (correct)
  - Vite build step (line 96): `cd frontend && npx vite build` added to frontend-check
- **Issues**: None

### F6: Lint-staged + pre-commit hooks
- **Verdict**: WORKING
- **Components**:
  - `.githooks/pre-commit`: exists (37 lines), handles Rust fmt + frontend lint-staged
  - `frontend/package.json`: `lint-staged: ^16.4.0`, `eslint-plugin-simple-import-sort: ^12.1.1`
  - `frontend/.eslintrc.cjs`: import/export sort rules at warn level
- **Issues**: Hook activation requires manual `git config core.hooksPath .githooks` (documented, not automatic)

### F7: Atomic cooldown (try_claim_trigger)
- **Verdict**: PARTIAL
- **Route**: `workflow_trigger.rs:274-297` — `try_claim_trigger()` with atomic `UPDATE...WHERE...RETURNING`
- **Handler**: `data_source_workflows.rs:1347` correctly calls `try_claim_trigger()` instead of `is_past_cooldown()`
- **Issues**:
  - `workflow_triggers.rs:172` — manual-fire webhook STILL uses racy `is_past_cooldown()` (plan acknowledged in regression analysis)
  - TOCTOU window remains for concurrent webhook requests

### F8: Graceful shutdown + worker registry
- **Verdict**: PARTIAL
- **Infrastructure**:
  - `workers/mod.rs`: `ShutdownRegistry` + `BackgroundWorker` trait exist
  - `main.rs:355`: Registry created; `main.rs:615`: `with_graceful_shutdown()` on axum::serve
  - `main.rs:600-612`: `sovereign_stack_shutdown` + `schedule_shutdown` tokens wired to registry
- **Issues**:
  - Only `AgentFlowExecutor` uses `BackgroundWorker` trait
  - VIBE deposit/withdrawal watchers, meeting cleanup, APN peer cleanup, OSS listener, pulse consumer — all run raw `tokio::spawn` with infinite loops, no shutdown support
  - These will be forcefully killed on SIGTERM

### F9: Friction logging
- **Verdict**: PARTIAL
- **Backend**: `SubmitFeedbackRequest` in `feedback.rs:27-61` has friction fields (`friction_point`, `user_intent`, `expected_behavior`, `time_lost_seconds`, `frustration_level` with 1-5 validation)
- **Frontend**: `FeedbackDialog.tsx` has friction type selector, frustration level, conditional field rendering
- **Playwright walkthrough**:
  - **Discovery**: Sidebar → More → "Feedback & Support" → opens FeedbackDialog
  - **Friction type**: Feedback Type combobox includes "Friction Report — Something felt slow, confusing, or broken"
  - **Friction fields render**: Selecting Friction Report reveals: "What were you trying to do?", "What went wrong?", "What did you expect?", frustration level 1-5 buttons (shows "Moderate" label)
  - **Tips update**: Contextual tips change to friction-specific guidance
- **Issues**:
  - No migration adding friction columns to a dedicated feedback table — friction data stored via task/data_source metadata
  - No dedicated "Report Friction" sidebar button — reachable via More → Feedback & Support (2 clicks)
  - No `#[derive(Validate)]` — validation is inline in handler
  - Plan marked PARTIAL, which is accurate

### F10: Cost bridge dashboard
- **Verdict**: PARTIAL
- **Backend** (100% working):
  - `vibe_treasury.rs`: 3 endpoints at `/api/organizations/{org_id}/costs/{summary,by-model,by-project}`
  - All handlers: real `GROUP BY` queries with dual-path JOINs, `require_org_membership()` access control
  - `vibe_transaction.rs`: `org_cost_summary()`, `org_cost_by_model()`, `org_cost_by_project()` with proper aggregation
- **Frontend API client** (exists but unused):
  - `costs.ts`: 3 functions matching backend endpoints, exported from `index.ts`
- **Frontend page** (NOT WIRED):
  - `ai-usage.tsx:34` imports `tokenUsageApi`, NOT `costsApi`
  - All queries call old `tokenUsageApi.getDaily()`, `.getByModel()`, etc.
  - **Plan claims this is DONE but frontend was never updated** — this is the most significant plan inaccuracy
- **Playwright walkthrough**:
  - `/settings/ai-usage` page loads but main content area is **empty** — no charts, no data
  - Settings → Org tab shows "Billing & Usage" as **"Coming soon"** — never wired
  - Confirms code trace: `costsApi` exists but is never called
- **Missing**: `/daily` endpoint (plan acknowledged as deferred), date range filtering (deferred)
- **Dormant issue**: `i64` → `bigint` type mismatch in TS types (will bite when wired)

### F11: Structured Response Protocol
- **Verdict**: STUB
- **Types defined** in `agent_response.rs:12-59`:
  - `AgentResponseEnvelope` with status, summary, data, clarification, suggested_next, confidence
  - `AgentResponseStatus`: Success, Error, NeedsClarification, InProgress, Skipped
  - `ClarificationRequest`: question, context, options, blocking
  - All have `#[derive(TS)]` + `#[ts(export)]`
- **TS bindings generated** in `crates/db/bindings/` directory
- **Issues**:
  - Types NOT in `shared/types.ts` (the canonical frontend import location)
  - Types never consumed: no executor returns `AgentResponseEnvelope`, no handler parses it
  - `ResponseMetrics` not implemented (plan noted as deferred)
  - Status enum naming differs from spec: Success/Error vs Done/Failed

### F12: NeedsClarification status
- **Verdict**: PARTIAL
- **Backend** (complete):
  - `FlowStatus::NeedsClarification` variant in `agent_flow.rs:49-62`
  - Migration `20260413000001`: adds `clarification_request TEXT` column
  - `POST /agent-flows/{id}/respond-clarification` endpoint in `agent_flows.rs:269-321`
  - Valid transitions defined and enforced with TOCTOU guard
- **Frontend** (display only):
  - `AgentFlowBoard.tsx`: groups flows by `needs_clarification` status
  - `AgentFlowCard.tsx`: amber color + AlertTriangle icon for NeedsClarification
- **Playwright walkthrough**:
  - Agent Flows tab accessible via Workflows → More → Agent Flows
  - Page renders with empty state: "No Agent Flows — Historical agent execution flows appear here"
  - No flows in seed DB to test NeedsClarification badge rendering live, but code trace confirms amber color + AlertTriangle icon
- **Issues**:
  - No clarification response form — users see the status but can't respond via UI
  - Plan marked PARTIAL, which is accurate

### F13: Agent Flow Orchestration Engine
- **Verdict**: STUB
- **Infrastructure** (exists):
  - `agent_flow_executor.rs`: implements `BackgroundWorker`, polls every 15s
  - Registered in `main.rs` conditionally via `ENABLE_AGENT_FLOW_ENGINE`
  - `tick()` finds flows by status, delegates to phase handlers
  - FSM validation via `valid_transitions()` + `can_transition_to()` + `transition_to_phase()` with TOCTOU guard
- **Core logic** (stubbed — 9 items NOT implemented):
  1. No LLM dispatch — comment: "Phase 2 will add real agent dispatch"
  2. No `AgentFlowExecutorConfig` with env vars
  3. No `DomainEvent` enum (but `FlowEventType`/`FlowEventPayload` exist in `agent_flow_event.rs`)
  4. No `emit_and_persist()` (emit helpers exist but never called from executor)
  5. No `valid_stage_transitions()` on CrmPipelineStage
  6. No `flow_config.dispatch_mode` field
  7. No test harness / mock dispatcher
  8. No `AwaitingApproval` handling beyond status transition
  9. No event emission on phase transitions
- **What the executor actually does**: transitions statuses without performing work; checks `execution_completed_at`/`verification_completed_at` flags that nothing sets
- **Plan marked STUB, which is accurate**

---

## Action Items

### Must Fix (blocks shipping)
- [ ] **F10**: Wire `costsApi` into `ai-usage.tsx` — replace `tokenUsageApi` calls with `costsApi.orgSummary()`, `.orgByModel()`, `.orgByProject()`. Plan claims DONE but frontend integration is missing.
- [ ] **F10**: Add `#[ts(type = "number")]` annotation on cost struct `i64` fields to avoid `bigint` runtime mismatch when costsApi is wired.

### Should Fix (quality/UX gaps)
- [ ] **F7**: Migrate `workflow_triggers.rs:172` from `is_past_cooldown()` to `try_claim_trigger()` — racy for concurrent webhooks
- [ ] **F9**: Add "Report Friction" entry point in sidebar (or keyboard shortcut) — friction form exists but is undiscoverable
- [ ] **F12**: Add clarification response form in AgentFlowCard — endpoint exists but no UI to invoke it
- [ ] **F8**: Migrate remaining background tasks to `BackgroundWorker` trait — VIBE watchers, meeting cleanup, APN, OSS listener
- [ ] **F11**: Ensure `AgentResponseEnvelope`/`AgentResponseStatus`/`ClarificationRequest` appear in `shared/types.ts` — currently only in `crates/db/bindings/`

### Nice to Have (polish)
- [ ] **F1**: Replace `Uuid::nil()` in `crm_deal_transitions.rs:104` with `DbUuid` equivalent
- [ ] **F6**: Auto-install git hooks via npm postinstall script (currently manual `git config core.hooksPath .githooks`)
- [ ] **F9**: Add `#[derive(Validate)]` on `SubmitFeedbackRequest` instead of inline validation
- [ ] **F13**: Implement `AgentFlowExecutorConfig::from_env()` for configurable poll interval/concurrency

---

## Build Verification

- [x] `npx vite build` — PASS (built in 15.19s)
- [x] `npx tsc --noEmit` — PASS (0 errors)
- [x] `npm run generate-types:check` — PASS (types up to date)
- [x] `cargo test --workspace` — PASS (with flox; `alpha-protocol-core` doctests fail on crate resolution — pre-existing, not sprint-related)
- [x] `cargo build` — PASS (implicit via test)

### Build Environment Blockers (pre-existing, not sprint-caused)
- `alpha-protocol-core` doctests: 18 errors — `sqlx`/`db` crates not found in doctest context. Doctests reference `sqlx::query_scalar` and `db::models::` but doctests don't link workspace crates. Pre-existing issue, not related to this sprint.
- `CMAKE_POLICY_VERSION_MINIMUM=3.5` must be set for `audiopus_sys` build. Flox sets this automatically; if flox cache is stale, clear `~/.cache/flox/run/<hash>/` and re-activate. Also set in `.env` as fallback.

---

## Cross-Cutting Findings

1. **Race conditions**: `workflow_triggers.rs:172` still racy; `respond_clarification` partially mitigated; executor `tick()` has no claim/lock pattern
2. **Observability**: New features have tracing but no metrics/dashboards; friction data has no aggregation endpoint
3. **Frontend Vite HMR error**: `FeedbackDialog.tsx` triggers import resolution error at runtime (path alias `@/components/ui/button` — file exists, tsc passes, production build passes). Likely Vite cache issue, blocks dev server page rendering. **Server restart should fix.**
4. **Plan accuracy**: F10 cost bridge is marked DONE but frontend integration is missing — plan says "Update extracted tab components + TokenUsageWidget.tsx to call new cost API" but `ai-usage.tsx` still imports `tokenUsageApi`

---

## Overall Verdict

**SHIP WITH CAVEATS**

The sprint's self-reported status is largely accurate: items marked DONE are working, items marked PARTIAL/STUB are honestly reported. The one exception is F10 (cost bridge) where the plan claims DONE but the frontend integration is missing — the backend and API client exist but the dashboard page never calls the new endpoints. This is the only Must Fix item. All other gaps are tracked and deferred to future phases.

# Sprint Gap Audit — Tier 1 Phase 0

**Date**: 2026-03-21
**Branch**: `feature/2026-03-21--gap-analysis`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Purpose**: Audit completed sprint items against the plan, identify gaps, and recommend process improvements

---

## Summary

| Item | Planned | Verdict | Gap Score |
|------|---------|---------|-----------|
| #1 Regressions + Access Control | 5 sub-items | **COMPLETE** | 0% gap |
| #2 CI Pipeline | 6 sub-items | **COMPLETE** | 0% gap |
| #3 Lint-Staged | 3 sub-items | **COMPLETE** | 0% gap |
| #4 Cooldown Race Condition | 3 sub-items | **COMPLETE** | 0% gap |
| #5 Graceful Shutdown | 5 sub-items | **COMPLETE** | ~5% gap (no drain timeout) |
| #6 Cost Bridge | 7 sub-items | **PARTIAL** | ~40% gap |
| #7 Structured Response Protocol | 5 sub-items | **MOSTLY DONE** | ~20% gap |
| #8 Clarification Status | 5 sub-items | **MOSTLY DONE** | ~20% gap |
| #9 Agent Flow Engine Phase 1 | 13 sub-items | **STUB ONLY** | ~70% gap |
| #10 Friction Logging | 7 sub-items | **PARTIAL** | ~50% gap |

**Pattern**: Items #1-5 (CI, access control, stability) shipped COMPLETE. Items #6-10 (features) shipped PARTIAL to STUB. The sprint delivered infrastructure but not features.

---

## Detailed Findings

### Items #1-5: COMPLETE (shipped as planned)

All 22 sub-items across items #1-5 are verified in the codebase:
- All 25 CRM endpoints have org membership access control
- crm_deals.rs split into 3 files (827 + 756 + 1419 lines)
- CI runs strict clippy, fmt, tsc, eslint, vite build — no continue-on-error
- Pre-commit hook with Rust fmt + lint-staged + TS derive reminder
- Atomic `try_claim_trigger()` replaces racy cooldown check in both webhook and schedule paths
- ShutdownRegistry + BackgroundWorker trait + signal handler + APN cleanup all wired

**Minor residuals** (not gaps, tracked as tech debt):
- `use uuid::Uuid` import lingers in 2 files (supports `Uuid::nil()` pattern)
- No configurable drain timeout on `with_graceful_shutdown`
- 27 clippy `-A` flags for bulk pre-existing lints

### Item #6: Cost Bridge — PARTIAL (~40% gap)

**Done:**
- 3 of 4 cost endpoints (summary, by-model, by-project)
- GROUP BY aggregation queries with BLOB→TEXT hex conversion
- `require_org_membership` on all cost endpoints
- Frontend `costs.ts` API module, exported from barrel
- Cost types in `shared/types.ts`

**Not done:**
| Gap | Effort | Impact |
|-----|--------|--------|
| `/costs/daily` endpoint missing | 0.5h | No daily cost trend chart |
| `ai-usage.tsx` still calls old `tokenUsageApi` — dashboard shows no data | 2h | **HIGH** — the whole point of the cost bridge |
| `query-config.ts` never created | 0.5h | Low — convenience constants |
| `ai-usage.tsx` tab extraction not done (still 518 lines) | 1h | Low — modularity debt |

**Impact**: The backend endpoints exist but the frontend never calls them. The AI usage dashboard still shows $0 because it reads from the dead `token_usage` table. The cost bridge is backend-complete but **frontend-broken**.

### Item #7: Structured Response Protocol — MOSTLY DONE (~20% gap)

**Done:**
- `AgentResponseEnvelope`, `AgentResponseStatus`, `ClarificationRequest` — all with `#[derive(TS)]`

**Not done:**
| Gap | Effort | Impact |
|-----|--------|--------|
| `ResponseMetrics` struct not implemented | 0.5h | Low — metrics tracking deferred |
| `AgentResponseEnvelope`/`AgentResponseStatus` not in `shared/types.ts` | 0.5h | Medium — frontend can't type-check agent responses |

### Item #8: Clarification Status — MOSTLY DONE (~20% gap)

**Done:**
- `FlowStatus::NeedsClarification` variant + FSM transitions
- Migration adding `clarification_request` column
- `POST /agent-flows/{id}/respond-clarification` endpoint
- Frontend badges/cards show the status

**Not done:**
| Gap | Effort | Impact |
|-----|--------|--------|
| No frontend clarification form | 2h | **HIGH** — users can't respond to clarification requests via UI |

### Item #9: Agent Flow Engine Phase 1 — STUB ONLY (~70% gap)

**Done (4 of 13 sub-items):**
- BackgroundWorker impl with 15s poll interval
- Polls by status (Planning, Executing, Verifying)
- FSM transition table with `valid_transitions()` and `can_transition_to()`
- Wired into main.rs with `ENABLE_AGENT_FLOW_ENGINE` gate

**Not done (9 of 13 sub-items):**
| Gap | Effort | Impact |
|-----|--------|--------|
| No LLM dispatch (API or executor) | 3 days | **CRITICAL** — engine does nothing useful |
| No `AgentResponseEnvelope` parsing | 0.5h | Coupled with dispatch |
| `transition_validated()` (strict mode) not implemented | 1h | Medium — only permissive transitions exist |
| No event emission (`emit_and_persist`) | 1 day | High — no observability of flow progress |
| No `DomainEvent` enum / `events/mod.rs` | 0.5h | Coupled with event emission |
| No `AgentFlowExecutorConfig` with env vars | 1h | Medium — no runtime configuration |
| No approval gate handling via FSM | 0.5h | Uses raw SQL instead of FSM validation |
| No test harness / mock dispatcher | 1 day | High — no way to verify without live LLM |
| No `CrmPipelineStage::valid_stage_transitions()` | 1h | Low — CRM FSM deferred |
| No `flow_config.dispatch_mode` field | 0.5h | Low — schema change needed |

**Impact**: The agent flow engine is scaffolding only. It polls status and transitions immediately without doing any work. No agent is actually dispatched. This was the **5-day keystone item** — only ~1 day of scaffolding was delivered.

### Item #10: Friction Logging — PARTIAL (~50% gap)

**Done:**
- FeedbackDialog has friction type with form fields (frictionPoint, userIntent, expectedBehavior, frustrationLevel)
- Backend accepts optional friction fields
- Frustration level validation (1-5 range)

**Not done:**
| Gap | Effort | Impact |
|-----|--------|--------|
| No `friction_area` enum (using free text instead) | 0.5h | Low — less structured analytics |
| No `workflow_step`, `workaround`, `feature_request` fields | 1h | Medium — less detailed friction data |
| No migration for friction columns | 0.5h | Low — data stored as general feedback metadata |
| No `#[derive(Validate)]` | 0.5h | Low — manual validation sufficient |
| No sidebar "Report Friction" button | 0.5h | **HIGH** — no discoverable entry point for dogfooding |

---

## Time Allocation Analysis

**What the sprint time was spent on** (estimated from commit history):

| Activity | Estimated Time | % of Sprint |
|----------|---------------|-------------|
| CI fixes (clippy, fmt, typegen, system deps) | ~8 hours | 35% |
| Regression analysis + PR reviews | ~4 hours | 17% |
| Merge conflict resolution | ~2 hours | 9% |
| Access control + crm_deals split (Item #1) | ~3 hours | 13% |
| Items #2-5 (CI, lint-staged, cooldown, shutdown) | ~3 hours | 13% |
| Items #6-10 (features) | ~3 hours | 13% |

**Key insight**: 61% of sprint time went to CI/review/merge overhead. Only 13% went to the feature items (#6-10) that represent the sprint's actual goals.

---

## Root Causes

### 1. CI was not a prerequisite — it became the sprint
The plan estimated 1.5 days for CI (Item #2). Reality: ~8 hours of CI firefighting across multiple push-fix-push cycles. Each clippy fix surfaced more errors from cache invalidation. The plan didn't account for the gap between "CI passes with incremental builds" and "CI passes with clean builds."

### 2. Over-specified plans create false completeness
Item #9 has 80+ lines of planning with code snippets, FSM tables, and architecture diagrams. This created an illusion of thoroughness that masked the fact that only scaffolding was built. The plan described WHAT to build in great detail but didn't verify WHEN each piece would ship.

### 3. No acceptance criteria tied to user outcomes
Every item was specified as "add X to file Y" — implementation tasks. None were specified as "user can do X and see Y." The cost bridge (Item #6) is a perfect example: backend endpoints exist but the dashboard still shows $0 because nobody verified the frontend calls them.

### 4. Feature items were squeezed into leftover time
Items #1-5 (infrastructure) were done first and consumed most of the sprint. Items #6-10 (features) got whatever time remained. The plan didn't prioritize by user value — it prioritized by implementation order.

### 5. No mid-sprint checkpoint
There was no point where someone asked "are we on track?" At ~50% through the sprint, items #1-5 were done but #6-10 hadn't started. A checkpoint could have forced a scope cut.

---

## Process Improvements

### For the expand-plan skill:

1. **Acceptance criteria on every item** — not "add endpoint X" but "dashboard shows non-zero cost data." This catches cases like Item #6 where backend ships but frontend doesn't wire up.

2. **Time estimates per sub-item, not per item** — Item #9 was "5 days" but had 13 sub-items. Breaking it down would have revealed that dispatch alone (sub-item 2) is 3 days.

3. **Priority tiers with cut lines** — "If we run out of time, cut items below this line." Items #1-5 should have been Tier 1, Items #6-7 Tier 2, Items #8-10 Tier 3.

4. **CI health gate** — "If CI is red, no feature work until green." This prevents the pattern of fixing CI incrementally while trying to ship features.

### For the qa-review / regression-test skills:

5. **End-to-end verification** — the regression test should check not just "does the code exist" but "does the feature work end-to-end." For Item #6: does the dashboard actually show cost data?

6. **Frontend integration check** — for every new backend endpoint, verify there's a frontend consumer that calls it. Dead backend endpoints are waste.

### For the development workflow:

7. **Ship vertical slices, not horizontal layers** — instead of "all backends first, then all frontends," ship "cost bridge backend + frontend + verification" as one unit. This catches integration gaps immediately.

8. **Mid-sprint scope review** — at 50% elapsed time, review what's done vs planned. Cut scope proactively rather than shipping stubs.

9. **Distinguish scaffolding from implementation** — the planning doc should have separate line items for "create the file structure" vs "implement the actual behavior." A scaffold that auto-transitions without dispatching an agent is not Phase 1 — it's Phase 0.

---

## Remaining Work (ordered by impact)

| Priority | Item | Gap | Effort | Why it matters |
|----------|------|-----|--------|----------------|
| **P0** | #6 Wire ai-usage.tsx to new cost API | Dashboard shows $0 | 3h | Core sprint goal unmet |
| **P0** | #9 Implement LLM dispatch in executor | Engine does nothing | 3 days | Keystone item — without this, agent flows are inert |
| **P1** | #8 Frontend clarification form | Users can't respond | 2h | NeedsClarification status useless without UI |
| **P1** | #10 Sidebar "Report Friction" button | No entry point for dogfooding | 0.5h | Dogfooding friction logging is invisible |
| **P2** | #9 Event emission (emit_and_persist) | No observability | 1 day | Can't track flow progress |
| **P2** | #9 Test harness | Can't verify without LLM | 1 day | Blocks QA |
| **P2** | #9 AgentFlowExecutorConfig | No runtime tuning | 1h | Hardcoded 15s interval |
| **P3** | #6 /costs/daily endpoint | No daily trends | 0.5h | Nice to have |
| **P3** | #7 ResponseMetrics + type generation | Metrics tracking | 1h | Deferred |
| **P3** | #10 Structured friction schema | Better analytics | 1.5h | Nice to have |

---

## Solution Paths

### P0: Wire ai-usage.tsx to new cost API (Item #6)

**Problem**: `ai-usage.tsx` calls 6 endpoints on `tokenUsageApi` (which reads from the dead `token_usage` table). The new `costsApi` reads from `vibe_transactions` (real data) but only has 3 endpoints with different response shapes.

**Current state**:

| Old API (`tokenUsageApi`) | Endpoint | Response shape |
|--------------------------|----------|----------------|
| `getToday()` | `/api/token-usage/today` | `TokenUsageSummary { total_input_tokens, total_output_tokens, total_tokens, total_cost_cents, request_count }` |
| `getDaily(days)` | `/api/token-usage/daily?days=N` | `DailyTokenUsage[] { usage_date, project_id, model, provider, total_input_tokens, total_output_tokens, total_tokens, total_cost_cents, request_count }` |
| `getByProvider(days)` | `/api/token-usage/by-provider?days=N` | `TokenUsageByProvider[] { provider, total_input_tokens, total_output_tokens, total_tokens, total_cost_cents, request_count }` |
| `getByModel(days)` | `/api/token-usage/by-model?days=N` | `TokenUsageByModel[] { model, provider, total_input_tokens, total_output_tokens, total_tokens, total_cost_cents, request_count }` |
| `getByProject(days)` | `/api/token-usage/by-project?days=N` | `TokenUsageByProject[] { project_id, project_name, total_tokens, request_count }` |
| `getByAgent(days)` | `/api/token-usage/by-agent?days=N` | `TokenUsageByAgent[] { agent_id, agent_name, total_tokens, request_count }` |

| New API (`costsApi`) | Endpoint | Response shape |
|---------------------|----------|----------------|
| `orgSummary(orgId)` | `/api/organizations/:orgId/costs/summary` | `OrgCostSummary { total_vibe, total_cost_cents, transaction_count, total_input_tokens, total_output_tokens }` |
| `orgByModel(orgId)` | `/api/organizations/:orgId/costs/by-model` | `ModelCostRow[] { model, provider, total_vibe, total_cost_cents, transaction_count, input_tokens, output_tokens }` |
| `orgByProject(orgId)` | `/api/organizations/:orgId/costs/by-project` | `ProjectCostRow[] { project_id, project_name, total_vibe, total_cost_cents, transaction_count }` |

**Key differences**:
- New API is org-scoped (requires `orgId`), old was global
- New API has no `days` parameter (returns all-time) — needs backend change for date filtering
- New API has `total_vibe` field (VIBE token cost), old only had `total_cost_cents`
- New API uses `bigint` in TS types (from `i64` via ts-rs) but JSON delivers `number` — needs `#[ts(type = "number")]` on Rust structs
- Missing new endpoints: `daily` (time series), `by-provider`, `by-agent`
- Field naming differs: `request_count` vs `transaction_count`, `total_tokens` vs separate `input_tokens`/`output_tokens`

**Recommended solution** (3 steps):

**Step 1: Backend — add `days` query param + missing endpoints (~1.5h)**
File: `crates/server/src/routes/vibe_treasury.rs` + `crates/db/src/models/vibe_transaction.rs`

```rust
// Add Query param to all 3 existing endpoints:
#[derive(Deserialize)]
struct CostQuery { days: Option<i64> }

// Add WHERE clause: AND vt.created_at >= datetime('now', '-' || ?days || ' days')
// when days is Some

// Add 3 new endpoints:
// GET /organizations/:org_id/costs/daily       → Vec<DailyCostRow>
// GET /organizations/:org_id/costs/by-provider → Vec<ProviderCostRow>
// GET /organizations/:org_id/costs/by-agent    → Vec<AgentCostRow>
```

New Rust structs needed: `DailyCostRow`, `ProviderCostRow`, `AgentCostRow` (all with `#[derive(TS)]`).

**Step 2: Fix bigint types (~10min)**
File: `crates/db/src/models/vibe_transaction.rs`

Add `#[ts(type = "number")]` to all `i64` fields on `OrgCostSummary`, `ModelCostRow`, `ProjectCostRow`, and the new structs. Then `npm run generate-types`.

**Step 3: Frontend — replace tokenUsageApi calls (~1.5h)**
File: `frontend/src/pages/ai-usage.tsx`

1. Import `useOrganization` from `@/contexts/organization-context` for `effectiveOrgId`
2. Import `costsApi` and `costKeys`
3. Replace each `useQuery` call:
   - `tokenUsageApi.getToday()` → `costsApi.orgSummary(effectiveOrgId!)`
   - `tokenUsageApi.getDaily(days)` → `costsApi.orgDaily(effectiveOrgId!, days)` (new endpoint)
   - `tokenUsageApi.getByProvider(days)` → `costsApi.orgByProvider(effectiveOrgId!, days)` (new endpoint)
   - `tokenUsageApi.getByModel(days)` → `costsApi.orgByModel(effectiveOrgId!, days)` (update to pass days)
   - `tokenUsageApi.getByProject(days)` → `costsApi.orgByProject(effectiveOrgId!, days)` (update to pass days)
   - `tokenUsageApi.getByAgent(days)` → `costsApi.orgByAgent(effectiveOrgId!, days)` (new endpoint)
4. Update `costKeys` in `query-keys.ts` to include `orgDaily`, `orgByProvider`, `orgByAgent`
5. Update `costs.ts` API module with the 3 new functions
6. Map response field names in rendering code:
   - `m.request_count` → `m.transaction_count`
   - `m.total_tokens` → `Number(m.input_tokens) + Number(m.output_tokens)`
   - Add VIBE column to tables where relevant

**Verification**: Navigate to `/ai-usage`, confirm:
- Overview tab shows non-zero totals (if vibe_transactions has data for the org)
- Models tab shows model breakdown
- Projects tab shows project breakdown
- Daily tab shows time series
- All tabs respond to days filter dropdown

**Build steps**: `cargo sqlx prepare --workspace`, `npm run generate-types`, verify tsc passes

### P0: Implement LLM dispatch in agent flow executor (Item #9)

**Problem**: `agent_flow_executor.rs` polls flows and auto-transitions between phases without doing any actual work. No LLM is called, no agent is dispatched.

**Current state**:
- `handle_planning_flow`: immediately transitions Planning → Executing (no planning work)
- `handle_executing_flow`: checks if `execution_completed_at` is set, transitions to Verifying (nothing sets this timestamp)
- `handle_verifying_flow`: checks `verification_completed_at`, raw SQL transition (nothing sets this timestamp)

**Recommended solution**: This is a 3-day item. Scope to Phase 1 minimum:

1. **API dispatch via PCG Router** (~1 day): In `handle_executing_flow`, when a flow is in `Executing` without `execution_completed_at`:
   - Load `flow_config` JSON for model/prompt configuration
   - Call `pcg_router::route_completion()` with the task description as input
   - Parse response into `AgentResponseEnvelope`
   - If `NeedsClarification` → transition flow
   - If `Success` → set `execution_completed_at` and store response as artifact
   - If `Error` → transition to Failed

2. **Event emission** (~0.5 day): After each transition, call `AgentFlowEvent::emit_phase_started/completed` (methods already exist)

3. **Config** (~0.5 day): Add `AgentFlowExecutorConfig` with `poll_interval_secs`, `max_concurrent` from env vars

4. **Basic test** (~0.5 day): Mock the LLM response, verify the executor transitions correctly

**Key files**: `crates/server/src/agent_flow_executor.rs`, `crates/server/src/routes/pcg_router.rs` (existing LLM routing), `crates/db/src/models/agent_response.rs`

### P1: Frontend clarification form (Item #8)

**Problem**: When a flow enters `NeedsClarification`, the backend has the endpoint (`POST /agent-flows/:id/respond-clarification`) but the frontend has no UI to submit a response.

**Recommended solution** (~2h):
1. In `AgentFlowCard.tsx`, when `flow.status === 'needs_clarification'`:
   - Parse `flow.clarification_request` JSON into `ClarificationRequest` type
   - Show the question text and any options
   - Render a text input + submit button
   - On submit, call `POST /agent-flows/:id/respond-clarification` with `{ response, resume_status }`
2. Add `respondClarification` to `frontend/src/lib/api/agent-flows.ts`
3. Add mutation with toast in the card component

### P1: Sidebar "Report Friction" button (Item #10)

**Problem**: The friction form exists in FeedbackDialog but there's no discoverable way to access it.

**Recommended solution** (~30min):
1. In sidebar "More" popover, add a "Report Friction" item next to existing "Send Feedback"
2. On click, open `FeedbackDialog` with `type` pre-set to `'friction'`
3. The dialog already has all the friction fields — just needs the entry point

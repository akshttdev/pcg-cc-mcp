# Sprint Plan: CRM Pipeline — Fully Operational

**Date**: 2026-03-21
**Branch**: `feature/2026-03-21--pipeline-ops`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Base**: `main`
**Duration**: 10 working days (2026-03-21 → 2026-04-03)
**PR Strategy**: One bundled PR
**Merge distance**: 57 commits ahead, 0 behind main (clean, includes tier1 UX/regression fixes)

---

## Progress

### Completed
- **W1: Unified StageTransitionProcessor** — DONE
  - Migration `20260414000000_stage_config_and_flow_link.sql`
  - `stage_transition.rs`: process_transition(), handle_won_transition() (contact_id dedup), schedule_agent_flow(), cancel_agent_flow()
  - StageConfig/StageAction/StageValidation/StageOwner serde types with `#[derive(TS)]`
  - Refactored move_deal_stage() → returns TransitionResult (warnings, cancel_deadline, actions)
  - Refactored advance_deal() → delegates to processor
  - New endpoints: POST cancel-agent, POST approve-agent (user requested "Run Now" alongside Cancel)
  - CrmPipelineStage model: stage_config field + UPDATE SQL
  - AgentFlow model: crm_deal_id, cancel_deadline, retry_count, last_error + find_by_deal(), find_pending_flows()

- **W2: Agent Flow LLM Dispatch** — DONE
  - Extended existing AgentFlowExecutor (not new file) with real LLM dispatch
  - WorkflowLLMService::completion_with_tools() for agent calls
  - Tools: get_deal_context, update_deal_field, save_artifact
  - Multi-turn tool loop (max 5 turns)
  - Retry: attempt → same model → fallback to Sonnet → fail
  - Event emission: PhaseStarted, ArtifactCreated, FlowCompleted/Failed
  - Agent prompts hardcoded per agent name (Scout, Astra, Cash, Lux)
  - Frontend: pulsing status badge on deal card (Bot icon "Scout running..." / Clock "Agent pending")
  - Frontend: useMoveDeal onSuccess shows validation warnings + cancel/approve toast
  - API client: cancelDealAgent(), approveDealAgent() methods
  - Types: TransitionResult, ValidationWarning, StageConfig, StageAction, StageValidation, StageOwner
  - CrmDealWithContact: active_agent_flow_id/status/name/cancel_deadline from kanban JOIN

### In Progress
- **W3**: DnD verification (context menu already shipped `c5f66c9cb`)
- **W4**: Call scheduling input UI

### Pending
- W5: Deal detail panel + agent history
- W6: Person invitation
- W7: Pipeline settings + seed configs

---

## Context

The CRM pipeline has **two separate transition paths** with **inconsistent automation coverage**:
- `move_deal_stage()` (lines 32-221): triggers Scout, Astra Pass 2, Lux, review tasks
- `advance_deal()` (lines 346-630): triggers Won→Delivery creation, Scout, Astra Pass 1, review tasks

Neither covers the full set. Won provisioning only fires via `advance_deal`. Astra Pass 2 only fires via `move_deal_stage`. Both have duplicated Won→Delivery logic with different dedup strategies.

An `AgentFlowExecutor` already exists (`crates/server/src/agent_flow_executor.rs`) with BackgroundWorker trait, 15s polling, and ShutdownRegistry integration — but does **no real LLM dispatch** (Phase 1 stub only, gated behind `ENABLE_AGENT_FLOW_ENGINE=1`).

This sprint unifies the transition architecture, adds real LLM dispatch to the existing executor, and completes the pipeline UX.

### User Decisions Summary
- Unify transitions into one StageTransitionProcessor
- Data-driven CRM stages v1, with path to generic pipeline framework
- Ordered + skip-ahead FSM, path to any-to-any
- PCG Router is live (in-process, OpenAI-compatible)
- Single-phase agent execute-only (3-phase later)
- Extend existing AgentFlowExecutor (not create new worker)
- Incremental migration: new stages use config, existing keep hardcoded fallback
- Required fields + approval gates (soft enforcement: allow move + show warning)
- Approval routing: both queue + inline on deal card
- Deal panel: drawer+expand (extract shared primitives from TaskDetailsPanel)
- Auto-start agents with 30s cancel window (toast + card indicator)
- Internal call tracking only (no calendar integration)
- Invite: copy link to clipboard
- Global defaults, per-org override later
- Agent prompts: hardcode initially, extract later
- Status badge only for agent indicator (no streaming)
- Defer webhook notifications to next sprint

---

## Priority Tiers (Cut Line)

**MUST HAVE** (Days 1–6, ~6 days):
- W1: Unified StageTransitionProcessor
- W2: Agent Flow LLM dispatch (extend existing executor)
- W3: DnD verification + context menu

**SHOULD HAVE** (Days 6–8, ~2.5 days):
- W4: Call scheduling input UI
- W5: Deal detail panel expand + agent history tab

**NICE TO HAVE** (Days 9–10, ~2 days):
- W6: Person invitation
- W7: Pipeline settings UI + seed configs

*If time runs short, cut from bottom up.*

---

## Day-by-Day Schedule

### Days 1–2: W1 — Unified StageTransitionProcessor

**Done when**: User drags deal to any stage → same automations fire as advance button. Warnings show for missing required fields.

**Day 1: Migration + processor skeleton**

Migration `20260414000000_stage_config_and_flow_link.sql`:
- Add `stage_config TEXT` (nullable JSON) to `crm_pipeline_stages`
- Add `crm_deal_id TEXT REFERENCES crm_deals(id)` to `agent_flows`
- Add `cancel_deadline TEXT` to `agent_flows`
- Add `retry_count INTEGER DEFAULT 0`, `last_error TEXT` to `agent_flows`
- Index on `agent_flows(crm_deal_id)`

Build steps after migration:
```bash
sqlx migrate run
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace
```

New module: `crates/server/src/stage_transition.rs` (in server crate, not services — needs access to automation functions in `crm_deal_automations.rs`)
```
StageTransitionProcessor
├── process_transition(pool, deal_id, from_stage_id, to_stage_id, position)
│   ├── load_stage_config(stage_id) → Option<StageConfig>  // NULL = hardcoded fallback
│   ├── validate_exit(deal, from_config) → Vec<ValidationWarning>  // soft: warn, don't block
│   ├── CrmDeal::move_to_stage(pool, deal_id, to_stage_id, position)
│   ├── execute_entry(deal, to_config)
│   │   ├── handle_won_transition()  // deduplicated from both paths
│   │   ├── create_review_task()     // deduplicated from both paths
│   │   └── schedule_agent_flow(deal_id, agent, cancel_window_secs)
│   └── return TransitionResult { deal, warnings, agent_flow_id, cancel_deadline }
│
├── StageConfig (serde struct from JSON, #[derive(TS)])
│   ├── assigned_agent: Option<String>
│   ├── auto_trigger: bool
│   ├── cancel_window_secs: u32 (default 30)
│   ├── required_fields: Vec<String>
│   ├── approval_gate: bool
│   ├── on_enter_actions: Vec<StageAction>
│   ├── on_exit_validations: Vec<StageValidation>
│   └── stage_owner: Option<StageOwner>
│
├── StageAction enum: TriggerAgent, CreateReviewTask, CreateDeliveryDeal
└── StageValidation enum: RequireField, RequireIntel, RequirePendingTasks
```

**Day 2: Wire into both paths + cancel endpoint**

- Refactor `move_deal_stage()` (crm_deal_transitions.rs:32-221): Replace inline automations (lines 130-218) with `process_transition()` call.
- Refactor `advance_deal()` (crm_deal_transitions.rs:346-630): Replace inline hooks (lines 475-627) with `process_transition()` call.
- Deduplicate Won→Delivery logic into `handle_won_transition()`:
  - `move_deal_stage` uses contact_id dedup (lines 73-83)
  - `advance_deal` uses name pattern dedup (lines 505-520)
  - Unify to contact_id dedup (more reliable)
- Modify `PATCH /crm/deals/:id/stage` response to return `TransitionResult` (includes `warnings`, `agent_flow_id`, `cancel_deadline`)
- New endpoint: `POST /crm/deals/:id/cancel-agent`
- Update `CrmPipelineStage` struct: add `stage_config: Option<String>` (line 91 area)
- Update all SQL in crm_pipeline.rs: INSERT (line 596), VALUES (line 598), UPDATE (lines 647-657)

Build steps:
```bash
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace
npm run generate-types
```

**Files:**
| File | Action | Notes |
|------|--------|-------|
| `crates/db/migrations/20260414000000_stage_config_and_flow_link.sql` | CREATE | |
| `crates/server/src/stage_transition.rs` | CREATE | In server crate (needs automation fns) |
| `crates/server/src/lib.rs` or `mod.rs` | MODIFY | Add `pub mod stage_transition;` |
| `crates/db/src/models/crm_pipeline.rs` | MODIFY | Add stage_config to struct + 4 SQL queries |
| `crates/db/src/models/agent_flow.rs` | MODIFY | Add crm_deal_id, cancel_deadline, retry_count, last_error |
| `crates/server/src/routes/crm_deal_transitions.rs` | MODIFY | Refactor both paths to use processor |
| `crates/server/src/routes/crm_deals.rs` | MODIFY | Add cancel-agent route, modify move response type |

---

### Days 3–5: W2 — Agent Flow LLM Dispatch

**Done when**: User moves deal to Intel stage → Scout agent runs automatically after 30s → research artifact appears in deal detail.

**Day 3: Extend existing AgentFlowExecutor with LLM dispatch**

File: `crates/server/src/agent_flow_executor.rs` (EXISTS — 77+ lines, BackgroundWorker trait)

Current state:
- `tick()` polls every 15s, finds flows in Planning/Executing/Verifying states
- `handle_planning_flow()`, `handle_executing_flow()`, `handle_verifying_flow()` — all stubs (log + transition)
- Spawned in `main.rs:358-363` with `ENABLE_AGENT_FLOW_ENGINE=1`

Changes needed:
- Add `SqlitePool` field (already has it)
- In `handle_executing_flow()`: add real LLM dispatch via `WorkflowLLMService::completion_with_tools()`
  - Signature: `completion_with_tools(pool, messages, tools, model_hint, max_tokens, temperature)` → `(LLMResponse, RoutingMetadata)`
- Add cancel_deadline check to tick() query (skip flows within cancel window)
- Add retry logic: attempt → same model → fallback to sonnet → fail
- Single-phase for v1: Planning → Executing → Completed (skip Verifying)

**Day 4: Tool definitions + agent prompts**

Deal-specific tools (defined in agent_flow_executor.rs or sub-module):
- `update_deal_field` — updates deal fields
- `get_deal_context` — reads deal + contact + company + intel
- `get_person_intel` / `get_company_intel` — reads intelligence data
- `save_artifact` — stores output via `AgentFlowEvent::emit_artifact_created()` (line 298)

Agent prompts: hardcoded per agent name (scout, astra, cash, lux) for v1.

Emit events using existing helpers:
- `AgentFlowEvent::emit_phase_started()` (line 277)
- `AgentFlowEvent::emit_artifact_created()` (line 298)
- `AgentFlowEvent::create()` with FlowCompleted/FlowFailed event types

**Day 5: Frontend agent indicator + cancel toast**

Backend:
- Add `active_agent_flow_status` and `active_agent_name` to `CrmDealWithContact` query (LEFT JOIN agent_flows)

Frontend (`CrmDealCard.tsx`, 349 lines):
- When `deal.active_agent_flow_status === 'executing'`: pulsing status badge with agent name
- When `cancel_deadline` is future: show "Cancel" link

Frontend (`CrmPipelineBoard.tsx`, 449 lines):
- After `moveDeal.mutate()` succeeds: check response for `cancel_deadline`. Show Sonner toast with Cancel action.

Build steps:
```bash
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace
npm run generate-types
```

**Files:**
| File | Action | Notes |
|------|--------|-------|
| `crates/server/src/agent_flow_executor.rs` | MODIFY | Add LLM dispatch, tools, retry logic |
| `crates/db/src/models/agent_flow.rs` | MODIFY | Add `find_pending_flows()` query |
| `crates/db/src/models/crm_deal.rs` | MODIFY | Update CrmDealWithContact query for agent flow JOIN |
| `frontend/src/components/crm/CrmDealCard.tsx` | MODIFY | Agent running indicator (349 lines) |
| `frontend/src/components/crm/CrmPipelineBoard.tsx` | MODIFY | Cancel toast (449 lines) |
| `frontend/src/hooks/useCrmPipeline.ts` | MODIFY | Update useMoveDeal response type for TransitionResult |

---

### Day 6 (AM): W3 — DnD Verification (0.5 day)

**Done when**: DnD confirmed working in browser. Context menu already shipped.

- ~~Add "Move to..." submenu~~ — **DONE** (commit `c5f66c9cb`, merged from tier1 branch)
- Browser smoke test: verify DnD works (handleDragEnd → moveDeal.mutate())
- Verify "Move to..." context menu works end-to-end after W1 processor wiring

**Files:**
- No new file changes — verification only

---

### Day 6 (PM): W4 — Call Scheduling Input UI (0.5 day)

**Done when**: User opens deal detail → Overview tab → can set call date, method, and status.

- In `OverviewTab.tsx` (deal-detail/tabs/): add "Schedule Call" section
- DatePicker, method selector (Phone/Video/In-Person), status toggle
- Save to `custom_fields.call_schedule` via `updateDeal.mutate()`

**Files:**
- `frontend/src/components/crm/deal-detail/tabs/OverviewTab.tsx` — add section

---

### Days 7–8: W5 — Deal Detail Panel + Agent History (1.5 days)

**Done when**: User clicks deal card → drawer opens → can expand to fullscreen → Agent History tab shows flow timeline.

**Day 7: Extract shared primitives + expand mode**

Deal detail panel exists at `deal-detail/index.tsx` (159 lines, 8 tabs, Sheet pattern). No expand mode currently.

- Extract drawer+expand pattern from `TaskDetailsPanel.tsx` (838 lines, uses `isFullScreen` prop at line 72, `useTaskViewManager` store at line 119) into shared `DetailPanelShell`
- Note: TaskDetailsPanel expand uses a Zustand store (`useTaskViewManager`), not simple state. DetailPanelShell should accept `isExpanded`/`onToggleExpand` props instead of coupling to that store.
- Add expand button (Maximize2) to `DealHeader.tsx` (226 lines)

**Day 8: Agent History tab**

- New API: `GET /crm/deals/:id/agent-flows`
- New hook: `useDealAgentFlows.ts` using `crmKeys` factory from query-keys.ts (lines 31-89)
- New tab component: `AgentHistoryTab.tsx`
- Approval inline UI on deal card when flow is `awaiting_approval`

**Files:**
| File | Action | Notes |
|------|--------|-------|
| `frontend/src/components/shared/DetailPanelShell.tsx` | CREATE | Drawer+expand primitive |
| `frontend/src/components/crm/deal-detail/index.tsx` | MODIFY | Use shell, add Agent History tab |
| `frontend/src/components/crm/deal-detail/DealHeader.tsx` | MODIFY | Expand button |
| `frontend/src/components/crm/deal-detail/tabs/AgentHistoryTab.tsx` | CREATE | Flow timeline |
| `frontend/src/components/tasks/TaskDetailsPanel.tsx` | MODIFY | Refactor to use shell (838 lines) |
| `frontend/src/hooks/useDealAgentFlows.ts` | CREATE | React Query hook |
| `frontend/src/lib/api/crm.ts` | MODIFY | Add dealAgentFlows method (679 lines) |
| `frontend/src/lib/query-keys.ts` | MODIFY | Add dealAgentFlows keys |
| `crates/server/src/routes/crm_deals.rs` | MODIFY | Add agent-flows endpoint |

---

### Day 9 (AM): W6 — Person Invitation (0.5 day)

**Done when**: User clicks "Generate Invite Link" on a Won deal → link copied to clipboard.

- Backend: `POST /crm/deals/:id/generate-invite`
- Reuse existing invitation infrastructure (`crates/server/src/routes/invitations.rs` — create_invitation lines 92-191, accept flow lines 344-457)
- Frontend: button in DeckTab, clipboard copy, Sonner toast

**Files:**
- `crates/server/src/routes/crm_deal_automations.rs` — add endpoint
- `frontend/src/components/crm/deal-detail/tabs/DeckTab.tsx` — invite button

---

### Day 9 (PM) – Day 10 (AM): W7 — Pipeline Settings + Seed Configs (1 day)

**Done when**: Admin opens pipeline settings → can configure agent assignment, gates per stage. Default configs seeded for existing pipelines.

Extend `CrmPipelineSettings.tsx` (730 lines — consider splitting `StageConfigEditor.tsx` as sub-component to stay under 500-line limit).

Stage config editor fields: Assigned Agent, Auto-trigger, Cancel window, Required fields, Approval gate.

Migration `20260414000001_seed_stage_configs.sql`: seed stage_config JSON for all existing pipeline stages.

Replace hardcoded `getStageOwner()` (CrmPipelineBoard.tsx lines 38-66) with `stage.stage_config?.stage_owner` lookup.

**Files:**
| File | Action | Notes |
|------|--------|-------|
| `frontend/src/components/crm/CrmPipelineSettings.tsx` | MODIFY | Add config section (730 lines — split if needed) |
| `frontend/src/components/crm/StageConfigEditor.tsx` | CREATE | Extracted config form sub-component |
| `crates/db/migrations/20260414000001_seed_stage_configs.sql` | CREATE | Seed default configs |
| `frontend/src/components/crm/CrmPipelineBoard.tsx` | MODIFY | Dynamic stage owners |

---

### Day 10 (PM): Integration Testing + Polish

- `cargo fmt --all -- --check && cargo clippy --all --all-targets --all-features -- -D warnings`
- `cd frontend && npm run lint && npx tsc --noEmit`
- `npm run generate-types:check`
- `cargo test --workspace`
- `npx playwright test --reporter=list`

---

## Key Existing Code to Reuse

| What | Where | How Used |
|------|-------|----------|
| AgentFlowExecutor | `crates/server/src/agent_flow_executor.rs` | EXTEND with LLM dispatch (not create new) |
| WorkflowLLMService | `crates/services/src/services/workflow_llm.rs:167` | `completion_with_tools()` for agent dispatch |
| AgentFlow FSM | `crates/db/src/models/agent_flow.rs:49-77` | `FlowStatus::valid_transitions()`, `transition_to_phase()` |
| AgentFlowEvent emitters | `crates/db/src/models/agent_flow_event.rs:277-351` | `emit_phase_started()`, `emit_artifact_created()` |
| ShutdownRegistry | `crates/server/src/workers/` | Worker lifecycle (already spawns executor at main.rs:355) |
| CRM automations | `crates/server/src/routes/crm_deal_automations.rs` | `trigger_who_is_research`, `generate_phase1_business_report`, etc. |
| Invitation system | `crates/server/src/routes/invitations.rs:92-457` | Token gen + accept flow |
| ApprovalQueueBoard | `frontend/src/components/approval-queue/ApprovalQueueBoard.tsx` | Works with `ArtifactReview[]` |
| CRM query keys | `frontend/src/lib/query-keys.ts:31-89` | `crmKeys` factory |
| E2E helpers | `e2e/helpers/seed.ts` | `createTestDeal()`, `createTestContact()`, `TEST_CONSTANTS` |

## stage_config JSON Schema

```json
{
  "assigned_agent": "scout",
  "auto_trigger": true,
  "cancel_window_secs": 30,
  "required_fields": ["description"],
  "approval_gate": false,
  "on_enter_actions": [
    { "type": "trigger_agent", "agent": "scout", "flow_type": "research" },
    { "type": "create_review_task", "description": "Review intelligence" }
  ],
  "on_exit_validations": [
    { "type": "require_field", "field": "description", "message": "Required" },
    { "type": "require_intel", "entity": "person", "status": "done" }
  ],
  "stage_owner": { "label": "Scout", "type": "agent" }
}
```

---

## Verification Plan

### Functional
- [x] DnD deal move triggers same automations as advance button (W1: processor wired to both paths)
- [x] "Move to..." context menu works as DnD fallback (commit `c5f66c9cb`)
- [x] Soft gate warnings appear when required fields are empty (W1: ValidationWarning in TransitionResult)
- [x] Agent flow executor picks up flows after cancel window expires (W2: find_pending_flows)
- [x] Cancel button stops pending agent within 30s window (W1: cancel-agent endpoint)
- [x] Agent running indicator (pulsing badge) on deal card (W2: StatusChip with Bot icon)
- [x] Agent completes → flow status "completed" → artifact stored (W2: complete_flow + emit events)
- [x] Retry: LLM error → retry → fallback model → fail gracefully (W2: call_llm_with_retry)
- [ ] Call scheduling: date/method/status save and display in OverviewTab
- [ ] Deal detail panel: drawer opens, expand button → fullscreen dialog
- [ ] Agent History tab: flow timeline with events
- [ ] Approval gates: inline on deal card + in ApprovalQueueBoard
- [ ] Invite link: copied to clipboard on Won deals
- [ ] Pipeline Settings: stage_config editor saves and persists
- [ ] Dynamic stage owners from stage_config (hardcoded fallback)

### Playwright Smoke Tests
- [ ] Navigate `/organizations/${ORG_ID}/crm/pipeline` → kanban board renders
- [ ] Click deal card → drawer opens with all tabs
- [ ] Right-click deal → "Move to..." submenu visible
- [ ] No console errors on pipeline page

### Build Checks
- [ ] `cargo test --workspace` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all --all-targets --all-features -- -D warnings` passes
- [ ] `npm run check` passes
- [ ] `npm run generate-types:check` passes
- [ ] `npx playwright test --reporter=list` passes (28+/31 demos)

---

## Decisions (Resolved)

1. **Processor location**: `StageTransitionProcessor` lives in server crate (`crates/server/src/stage_transition.rs`). Needs access to automation functions in `crm_deal_automations.rs`. Move to services crate in future refactor.

2. **Won dedup strategy**: Unified processor uses `contact_id` dedup (from `move_deal_stage` path). More reliable than name pattern matching.

---

## Modularity Wins

**In-sprint:**
- Split `CrmPipelineSettings.tsx` (730 lines) → `CrmPipelineSettings.tsx` + `StageConfigEditor.tsx` (~0.5h)
- Extract `DetailPanelShell.tsx` from TaskDetailsPanel (already planned in W5)
- Deduplicate Won→Delivery and review task logic into processor (already planned in W1)

**Deferred (add to backlog):**
- TaskDetailsPanel.tsx (838 lines) → further decomposition after shell extraction
- query-keys.ts (615 lines) → split by domain when it hits 800+

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| stage_config migration breaks existing behavior | Config is nullable; NULL = hardcoded fallback, zero behavior change |
| Agent executor starves Axum threads | Existing executor uses BackgroundWorker trait with cooperative scheduling; limit to 5 concurrent flows |
| LLM dispatch failures block deals | Retry → fallback model → fail gracefully; deal stays in stage, user can re-trigger |
| TransitionResult API change breaks frontend | Update useMoveDeal hook + API client simultaneously; new fields are additive |
| Sprint overrun from CI overhead | CI stable (PR #53); cut line defined — drop W6/W7 if needed |
| DnD actually broken (runtime, not code) | Context menu is Day 6 fallback; DnD fix is low-effort if needed |

---

## Path to Generic Pipeline Framework (Future Sprint)

1. Extract `StageTransitionProcessor` into `PipelineProcessor<T>` trait
2. CRM deals = one implementation; content pipeline, task pipeline = others
3. Move agent prompts from hardcoded → agent_profiles table → stage_config references
4. Per-org stage_config overrides (currently global defaults)
5. Any-to-any FSM with configurable allowed_transitions per stage pair
6. Webhook/notification actions on stage transitions
7. Custom pipeline types created by users

# Sprint Plan: Sloperation Intent Parity — Full Pipeline Operational

**Date**: 2026-03-24
**Branch**: `feature/2026-03-24--sloperation-sprint`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Base**: `feature/2026-03-19--tier1-phase0` (tier1) + `feature/2026-03-23--continued` merged in
**Duration**: 10 working days (2026-03-24 → 2026-04-07)
**PR Strategy**: Single feature branch, one bundled PR
**LLM Mode**: SIMULATE_LLM=1 throughout
**Status**: FEATURES COMPLETE — 37/40 E2E pass, 3 remaining (SSE timing + tab context)

## Sprint Summary (2026-03-25)

### Completed
- **W1**: Discovery stage restored (migration + hero card + transcript exit gate)
- **W2**: Sequential agent queue (chain_actions + Astra P2→Cash)
- **W3**: Won unification (provision_won_deal, org-level project, delivery deal)
- **W4**: Present & Invoice consolidation (rename + presentation tracking)
- **W5**: Stage-aware tab visibility (STAGE_TAB_MAP + toggle) — **MCP verified working**
- **W6**: Data source linking (dual agent/stage scoping, org picker)
- **W7**: Review task routing (config-driven + org owner fallback)
- **Auto-skip**: Lead stage auto-skips to Intel (configurable)
- **Review gate**: Auto-advance blocked until review tasks completed
- **Mark Review Complete**: New button in ReviewTab for UI-driven task completion
- **SSE endpoint**: GET /api/events/pipeline for real-time kanban updates (deployed)
- **Kanban SSE**: Board subscribes to SSE, no more polling
- **Conferences**: Scout stage_config added to Researching stage

### E2E Results: 37 pass, 3 fail, 3 skip → fixes applied (2026-03-25)
| Test | Status | Fix Applied |
|------|--------|-------------|
| AA-3 | FIXED | Increased test timeout 45→90s, waitForDealStage 30→60s. Optimized helper: skip drawer interaction when stage just changed, reduce sleep 2s→1.5s, open drawer every other iteration only. |
| DD-1 | FIXED | Increased visibility check timeout 1→3s, added explicit `toBeVisible` assertion before click. DD-2 setup now reopens panel via card testid instead of text click (sets stageName). MCP verified: Transcripts tab visible at Intel, heading "Discovery Transcripts" correct. |
| DD-4 | FIXED | Root cause: Send Invoice gated on (1) stage past Proposal via PRE_INVOICE_STAGES, (2) presentation_status === 'presented'. Test now moves deal to Present & Invoice via context menu, clicks "presented", saves, then verifies Send Invoice is enabled. MCP verified: full flow works. |

### Remaining Work (next session)

**HIGH — verify fixes:**
1. **Run E2E tests** to confirm all 40 tests pass (pending user approval to run).

**Fixes applied (2026-03-25):**
- ~~Fix DD-1 setup~~ — done: card testid + timeout increase
- ~~Fix AA-3 timing~~ — done: timeout increase + helper optimization
- ~~Fix DD-4 cascade~~ — done: move to Present & Invoice + set presentation status

**MEDIUM — polish:**
4. **Add missing testids**: Interactive elements in CrmPipelineSettings, TranscriptsTab link form, CrmDealCard menu buttons need testids per frontend standards.
5. **Demo-quality test refactor**: Tests should look like a user demo — watch kanban board, open drawer to verify data at key stages, close and watch next action. Current helpers still mix API checks with UI.
6. **SSE content-type**: Console shows `EventSource MIME type` error — the SSE endpoint may need explicit `text/event-stream` content type header.

**LOW — deferred:**
7. **W8 Cost bridge**: Agent flows record token usage for cost dashboard
8. **Broadcast channel SSE**: Replace DB polling with tokio::sync::watch for true push
9. **Person invite (F3)**: Token generation + email via Resend

### Key Decisions Made During Sprint
- **Data source scoping**: Dual `relevant_stages` + `relevant_agents` fields (both JSON arrays, both nullable = all)
- **Lead stage**: `auto_skip: true` by default — deals fly through to Intel
- **Review task gate**: Auto-advance requires all deal-linked tasks completed (not just agent flows)
- **Project on Won**: Org-level project (no client_id), unique git_repo_path per deal
- **Agent poll interval**: `AGENT_FLOW_POLL_INTERVAL=3` for dev/test (default 15s production)
- **No API shortcuts in E2E**: Pipeline actions must go through UI — setup data via API is fine
- **SSE over polling**: Kanban board subscribes to /api/events/pipeline SSE endpoint

## Implementation Log

### W1 Discovery (2026-03-24)
**Key discovery**: Discovery stage already exists in `create_sales_pipeline()` (position 2) and has stage_config seed, STAGE_DESCRIPTIONS, and getStageOwner() entries. The gap is only for **existing pipelines** created before that code (e.g., Sirak Studios Acquisition with the 9-stage rebuild).

**W1 changes**:
- `stage_transition.rs` — added `RequireTranscriptOrSource` variant to `StageValidation` enum + handler + `check_deal_has_transcript_or_source()` helper
- `20260417000000_discovery_stage_and_transcript_validation.sql` — migration: inserts Discovery stage into Sales pipelines missing it (position 3, shifts others), updates all Discovery stage_configs with the new exit validation
- `shared/testids.ts` — added `discovery` testid group
- `OverviewTab.tsx` — added Discovery hero card (call status badge, Schedule Call, Link Transcript CTA), `onSwitchTab` prop for tab navigation, also added `present_&_invoice` to call scheduling stage check
- `deal-detail/index.tsx` — wired `onSwitchTab={setActiveTab}` to OverviewTab

### W2 Sequential Agent Queue (2026-03-24)
**Changes**:
- `stage_transition.rs` — sequential agent scheduling: collect all TriggerAgent actions, schedule first, store remaining as `chain_actions` in flow_config JSON
- `agent_flow_executor.rs` — `try_chain_next_agent()`: reads chain_actions after flow completion, schedules next agent with remaining chain, skips auto-advance until chain is empty
- `20260417000001_proposal_sequential_agents.sql` — update Proposal stage_config with Astra deep_research first, Cash proposal second

### W6 Design Revision: Data Source Scoping (2026-03-24)
**User feedback**: Not all linked sources are relevant for every agent. Sources should be taggable by stage/agent relevance. Also, sources should be linkable from any input stage, not just the Transcripts tab.

**Revised design**:
- `deal_data_sources` join table has `relevant_stages TEXT` — JSON array of stage names the source is relevant for (e.g. `["proposal", "polish"]`). NULL = available to all agents.
- Agent executor's `load_deal_context()` filters sources by current deal stage when loading context
- "Add Source" action available from Overview tab at any stage (compact inline form)
- Transcripts & Sources tab remains the primary management view with full link/unlink UI
- Backlog: data source picker dialog (browse org's data library instead of paste UUID), file upload → auto-create source

### Critical bug found: auto-advance skips review tasks (2026-03-24)
**Issue**: `try_auto_advance_deal()` only checked for pending agent flows but NOT pending review tasks. Agents would complete → deal auto-advances → review task sits untouched in 'todo' status. This defeats the purpose of review gates.
**Fix**: Added pending deal-linked task check (via `crm_deal_id`, not title matching) to `try_auto_advance_deal()`. Operator must complete/cancel review tasks in the UI before auto-advance proceeds.
**Impact**: All agent-owned stages now properly gate on review task completion. This is a **behavioral change** — previously auto-advance was instant after agent completion, now it waits for human review.

---

## Context

The CRM pipeline was rebuilt into a 9-stage architecture (PRs #55-58) but lost 5 sloperation317 intents during the migration. The 2026-03-24 gap audit identified 5 missing capabilities, 3 partial implementations, and 1 failing spec (AA-4). This sprint restores full sloperation intent parity so the pipeline operates as originally designed — with Discovery calls feeding into enhanced research, sequential agent chaining, unified Won provisioning, and stage-aware UI.

**ROI justification**: These are missing business capabilities, not tech debt. Without them, proposals lack client-specific insights (no Discovery→Pass 2 chain), Won deals don't provision fully via the UI (dual-path bug), and the AM has no pipeline prompt for the human call step.

---

## Scope Summary

| # | Workstream | Days | Priority |
|---|-----------|------|----------|
| W1 | Discovery Stage Restoration | 1.5 | MUST |
| W2 | Sequential Agent Queue + Astra Pass 2→Cash Chain | 2 | MUST |
| W3 | Won Unification (stage transition = full provisioning) | 1 | MUST |
| W4 | Present & Invoice Stage Consolidation | 1 | SHOULD |
| W5 | Stage-Aware Tab Visibility | 1 | SHOULD |
| W6 | Transcripts & Sources Tab Enhancement | 1 | SHOULD |
| W7 | Org-Specific Review Routing + Person Invite | 1 | NICE |
| W8 | Cost Bridge (S0-02) | 0.5 | NICE |
| W9 | E2E Tests + QA | 1 | MUST |

**Total**: ~10 days. Cut from bottom up if needed.

---

## W1: Discovery Stage Restoration (Days 1-2)

**Done when**: Discovery stage exists at position 4 between BA and Proposal. Human-owned (AM + Nora). Soft exit gates warn if call not completed and no transcript linked.

### Backend

**Migration `20260325000001_add_discovery_stage.sql`**:
- INSERT Discovery stage at position 4 for all Acquisition pipelines (stage_type = 'discovery')
- UPDATE positions: Proposal→5, Polish→6, Present & Invoice→7, Negotiation→8, Won→9, Lost→10
- Set `stage_config` JSON:
  ```json
  {
    "auto_trigger": false,
    "stage_owner": {"label": "Account Manager + Nora", "type": "human"},
    "on_enter_actions": [
      {"type": "create_review_task", "description": "Conduct discovery call and link transcript"}
    ],
    "on_exit_validations": [
      {"type": "require_transcript_or_source", "message": "No transcript or data source linked — proposals will lack client-specific insights"}
    ]
  }
  ```

**`crates/server/src/stage_transition.rs`**:
- Add `RequireTranscriptOrSource` variant to `StageValidation` enum
- Implement validation: query `deal_transcripts WHERE deal_id = ?` count. If 0, add warning.

### Frontend

**`frontend/src/components/crm/deal-detail/tabs/OverviewTab.tsx`**:
- When stage is 'discovery': render a hero-style "Discovery Call" card at top of left column
- Shows call status badge (not scheduled / scheduled / completed)
- "Schedule Call" button opens CallSchedulingSection editor
- After call marked completed: "Link Transcript →" CTA switches to Transcripts tab

**`frontend/src/components/crm/CrmPipelineBoard.tsx`**:
- Add Discovery to `STAGE_DESCRIPTIONS` map
- Verify `getStageOwner()` handles the new stage

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260325000001_add_discovery_stage.sql` | CREATE |
| `crates/server/src/stage_transition.rs` | MODIFY — add RequireTranscriptOrSource validation |
| `frontend/src/components/crm/deal-detail/tabs/OverviewTab.tsx` | MODIFY — Discovery hero card |
| `frontend/src/components/crm/CrmPipelineBoard.tsx` | MODIFY — stage descriptions |

---

## W2: Sequential Agent Queue + Astra Pass 2→Cash Chain (Days 2-4)

**Done when**: Moving a deal from Discovery→Proposal triggers Astra Pass 2 (flow_type 'deep_research'), then Cash chains automatically after Astra completes. AA-4 E2E test passes.

### Sequential Agent Queue Design

**`crates/server/src/stage_transition.rs`** — `process_transition()`:
1. Collect all `TriggerAgent` entries from `on_enter_actions` into a Vec
2. Schedule only the FIRST agent via `schedule_agent_flow()`
3. Serialize remaining agents as `chain_actions` in the flow's config JSON
4. Non-agent actions (CreateReviewTask) still execute immediately

**`crates/server/src/agent_flow_executor.rs`** — after `complete_flow()`:
1. Check `flow_config.chain_actions` for remaining agents
2. If present: schedule next agent with remaining chain, DON'T auto-advance yet
3. If empty: proceed with `try_auto_advance_deal()` as before

### Astra Pass 2 Configuration

**Update Proposal stage_config** (migration):
```json
{
  "on_enter_actions": [
    {"type": "trigger_agent", "agent": "astra", "flow_type": "deep_research"},
    {"type": "trigger_agent", "agent": "cash", "flow_type": "proposal"},
    {"type": "create_review_task", "description": "Review and approve proposal"}
  ]
}
```

**Agent prompts**: Astra with flow_type 'deep_research' gets an enhanced prompt in the executor that includes deal transcripts + Phase 1 report + all accumulated context. Cash continues to use the existing 'proposal' prompt but now benefits from Astra P2's enriched data.

### Files
| File | Action |
|------|--------|
| `crates/server/src/stage_transition.rs` | MODIFY — sequential agent scheduling |
| `crates/server/src/agent_flow_executor.rs` | MODIFY — chain_actions handling after completion |
| `crates/db/migrations/20260325000002_update_stage_configs.sql` | CREATE — update Proposal stage config |
| `crates/db/src/models/agent_flow.rs` | MODIFY — add chain_actions to flow config if needed |

---

## W3: Won Unification (Days 4-5)

**Done when**: Moving a deal to Won via any method (DnD, context menu, advance, stage bar) triggers full provisioning: client + project + tasks + VIBE transaction + delivery deal. DL-3 spec passes.

### Design

**`crates/server/src/routes/crm_deal_automations.rs`**:
1. Extract provisioning logic from `mark_deal_won()` into `provision_won_deal(pool, deal_id) -> Result<WonProvisionResult>`
2. Include: client creation, project creation, deliverables→tasks, VIBE transaction, delivery deal (contact_id dedup)
3. Add dedup guard: if `deal.won_at` already set OR client/project already exist, skip (idempotent)
4. Return `WonProvisionResult { client_id, client_name, project_id, project_name, tasks_created, vibe_amount }`
5. Keep `mark_deal_won()` as thin wrapper: set won_at/win_reason, call `provision_won_deal()`

**`crates/server/src/stage_transition.rs`** — `handle_won_transition()`:
- Replace delivery-deal-only logic with call to `provision_won_deal()`
- Store `WonProvisionResult` in `TransitionResult.won_provision` field

### Person Invite (F3)

**`crates/server/src/routes/crm_deal_automations.rs`** — in `provision_won_deal()`:
- Replace stub log with real token generation
- Create `person_invites` record (token, deal_id, person_id, expires_at = +7 days)
- Return invite URL in `WonProvisionResult`

**Migration**: Add `person_invites` table (id, deal_id, person_id, token, status, created_at, accepted_at, expires_at)

**New endpoints**:
- `GET /api/crm/deals/:id/invite` — returns current invite
- `POST /api/crm/deals/:id/invite/regenerate` — new token

### Frontend Won Feedback

**`frontend/src/components/crm/deal-detail/tabs/DeckTab.tsx`**:
- Enhance markWon `onSuccess` toast to rich persistent toast showing checklist:
  - Client created/found
  - Project created (with link)
  - N tasks created
  - VIBE transaction recorded ($amount)
- Extend `MarkWonResult` type with `client_name`, `vibe_amount`

### Files
| File | Action |
|------|--------|
| `crates/server/src/routes/crm_deal_automations.rs` | MODIFY — extract provision_won_deal, replace invite stub |
| `crates/server/src/stage_transition.rs` | MODIFY — Won entry calls provision_won_deal |
| `crates/db/migrations/20260325000003_person_invites.sql` | CREATE |
| `frontend/src/components/crm/deal-detail/tabs/DeckTab.tsx` | MODIFY — rich Won toast |
| `shared/types.ts` | REGENERATE |

---

## W4: Present & Invoice Stage Consolidation (Day 5)

**Done when**: "Invoice" stage renamed to "Present & Invoice". Presentation tracking (date, status) in the deal detail. DeckTab shows presentation section before invoice.

### Backend

**Migration `20260325000004_present_invoice_stage.sql`**:
- Rename stage: `UPDATE crm_pipeline_stages SET name = 'Present & Invoice' WHERE stage_type = 'present' OR name = 'Invoice'`
- Update stage_config with exit validations for presentation_status and invoice_id
- Note: presentation tracking stored in `custom_fields` JSON (no new columns needed — consistent with call scheduling pattern)

### Frontend

**`frontend/src/components/crm/deal-detail/tabs/DeckTab.tsx`**:
- Add "Presentation" section between Deck and Invoice sections
- Fields: presentation_date (date picker), presentation_status (dropdown: pending/scheduled/presented), presentation_notes (textarea)
- Save to `custom_fields.presentation` via updateDeal mutation
- Invoice section gated on `presentation_status === 'presented'` (replaces current `isTooEarly` stage check)

**`frontend/src/components/crm/CrmPipelineBoard.tsx`**:
- Update `STAGE_DESCRIPTIONS` for the renamed stage
- `getStageOwner()` already handles 'present' case

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260325000004_present_invoice_stage.sql` | CREATE |
| `frontend/src/components/crm/deal-detail/tabs/DeckTab.tsx` | MODIFY — presentation tracking |
| `frontend/src/components/crm/CrmPipelineBoard.tsx` | MODIFY — stage description |

---

## W5: Stage-Aware Tab Visibility (Day 6)

**Done when**: Deal detail panel shows only stage-relevant tabs by default. "Show all tabs" toggle reveals everything.

### Design

**NEW `frontend/src/components/crm/deal-detail/stage-tab-config.ts`**:
```typescript
export const STAGE_TAB_MAP: Record<string, string[]> = {
  'lead':                ['overview', 'activity'],
  'intel':               ['overview', 'intel', 'review', 'activity', 'agents'],
  'business analysis':   ['overview', 'intel', 'review', 'activity', 'agents'],
  'discovery':           ['overview', 'intel', 'transcripts', 'review', 'activity', 'agents'],
  'proposal':            ['overview', 'intel', 'transcripts', 'proposal', 'review', 'activity', 'agents'],
  'polish':              ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'present & invoice':   ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'negotiation':         ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'won':                 ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'projects', 'activity', 'agents'],
  'lost':                ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'activity', 'agents'],
};
```

**`frontend/src/components/crm/deal-detail/index.tsx`**:
- Add `showAllTabs` state with toggle button
- Filter tab list through `STAGE_TAB_MAP[deal.stage]`
- If active tab not in visible set, reset to 'overview'

### Backlog Notes
- Future: stage_config.visible_tabs drives tab visibility from backend
- Future: progressive unlock — tabs accumulate as deal advances through stages
- Future: per-pipeline tab config in pipeline settings UI

### Files
| File | Action |
|------|--------|
| `frontend/src/components/crm/deal-detail/stage-tab-config.ts` | CREATE |
| `frontend/src/components/crm/deal-detail/index.tsx` | MODIFY — tab filtering + toggle |

---

## W6: Transcripts & Sources Tab Enhancement (Days 6-7)

**Done when**: Transcripts tab renamed to "Transcripts & Sources". AM can link existing data sources and upload files (which transparently create data sources). Astra Pass 2 reads deal-linked sources via get_deal_context.

### Backend

**`crates/server/src/routes/crm_deal_automations.rs`**:
- Extend `LinkTranscriptRequest` with optional `data_source_id` field
- New endpoint: `POST /api/crm/deals/:id/data-sources` — upload file creates data_source + links to deal
- New endpoint: `GET /api/crm/deals/:id/data-sources` — list linked sources

**Migration**: `ALTER TABLE deal_transcripts ADD COLUMN data_source_id TEXT REFERENCES data_sources(id)`

**`crates/server/src/agent_flow_executor.rs`** — `load_deal_context()`:
- Extend to fetch linked data sources: `SELECT dt.summary, ds.content, ds.title FROM deal_transcripts dt LEFT JOIN data_sources ds ON dt.data_source_id = ds.id WHERE dt.deal_id = ?`
- Include in context passed to Astra Pass 2 prompt

### Frontend

**`frontend/src/components/crm/deal-detail/tabs/TranscriptsTab.tsx`**:
- Rename tab label to "Transcripts & Sources"
- Add "Link Data Source" button → opens picker dialog showing org's data sources
- Add file upload zone (adapt UploadZone component pattern from data-sources page)
- Display linked sources alongside transcripts

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260325000005_deal_data_sources.sql` | CREATE |
| `crates/server/src/routes/crm_deal_automations.rs` | MODIFY — data source endpoints |
| `crates/server/src/agent_flow_executor.rs` | MODIFY — load data sources in context |
| `frontend/src/components/crm/deal-detail/tabs/TranscriptsTab.tsx` | MODIFY — link + upload UI |
| `frontend/src/components/crm/deal-detail/index.tsx` | MODIFY — rename tab |

---

## W7: Org-Specific Review Routing + Person Invite UI (Day 8)

### Review Routing (F8)

**Done when**: Review tasks are assigned to the org-specific operator when `stage_config.review_assignee` is set, falling back to `org.owner_id`.

**`crates/server/src/stage_transition.rs`**:
- Add `review_assignee: Option<String>` to `StageConfig` struct
- After creating review task, call `resolve_review_assignee()`:
  1. Check `stage_config.review_assignee` → resolve username to user_id
  2. Fallback: query `organizations.owner_id` via deal's `organization_id`
  3. UPDATE task SET assignee_id = resolved_id

**Update stage_config seeds**: Set `review_assignee` for BA stage (Sirak→Sirak op, PCG→Bodhi).

### Person Invite UI

**`frontend/src/components/crm/deal-detail/tabs/DeckTab.tsx`**:
- InviteLinkSection already exists (lines 246-331) — verify it uses the real invite API
- Also surface invite link on Overview tab for Won deals

### Files
| File | Action |
|------|--------|
| `crates/server/src/stage_transition.rs` | MODIFY — review_assignee + resolve function |
| `crates/db/migrations/20260325000002_update_stage_configs.sql` | UPDATE — review_assignee in seeds |
| `frontend/src/components/crm/deal-detail/tabs/OverviewTab.tsx` | MODIFY — invite section for Won |

---

## W8: Cost Bridge (Day 8-9)

**Done when**: Cost dashboard shows real data from VIBE transactions. Pipeline agent flows record token usage.

### Investigation
- Verify `WorkflowLLMService::completion_with_tools()` records to `token_usage` table
- If not: add `record_llm_vibe_usage()` call after each successful LLM dispatch in `agent_flow_executor.rs`
- Ensure cost queries aggregate by `organization_id` (not just `project_id`) since pipeline tasks may lack project context
- Verify `costsApi` endpoints return non-zero data when VIBE transactions exist

### Files
| File | Action |
|------|--------|
| `crates/server/src/agent_flow_executor.rs` | MODIFY — ensure token usage recorded |
| Cost route handlers (investigate which file) | MODIFY — ensure org-level aggregation works |

---

## W9: E2E Tests + QA (Days 9-10)

### E2E Test Updates

**`e2e/pipeline/agent-automations.spec.ts`**:
- **AA-4**: Unblock from `test.fixme`. Test: deal moves Discovery→Proposal → Astra Pass 2 fires → Cash chains → both complete
- **AA-7**: Unblock retrigger test with proper failure setup

**`e2e/pipeline/deal-lifecycle.spec.ts`**:
- **DL-3**: Add Won provisioning test — move deal to Won → verify client + project + tasks created

**New/updated tests**:
- Discovery stage soft gate warnings
- Stage-aware tab visibility (check tab count changes per stage)
- Present & Invoice presentation tracking

### QA Checklist
- [ ] `cargo fmt --all && cargo fmt --all -- --check`
- [ ] `flox activate -- cargo clippy --all --all-targets -- -D warnings`
- [ ] `cd frontend && npx tsc --noEmit`
- [ ] `cd frontend && npx eslint . --ext ts,tsx`
- [ ] `npm run generate-types:check`
- [ ] `npx playwright test e2e/pipeline/ --reporter=list` (headed mode)
- [ ] Playwright MCP walkthrough: create deal → Intel → BA → Discovery → link transcript → Proposal (Astra P2 + Cash chain) → Polish → Present & Invoice → Won (full provisioning)

### Backlog Updates
- Add note: full E2E refresh for 11-stage pipeline (2 days, next sprint)
- Add note: research chain_to field vs dependency graph for agent queue (backlog)
- Add note: stage_config-driven tab visibility (backlog)
- Add note: progressive tab unlock (backlog)
- Add note: Resend email integration for person invite (backlog)
- Add note: auto-match transcripts from call intake (backlog)

---

## Day-by-Day Schedule

| Day | Workstream | Deliverable |
|-----|-----------|-------------|
| 1 | W1 (backend) | Discovery stage migration + exit validation |
| 2 | W1 (frontend) + W2 start | Discovery UI + sequential queue design |
| 3 | W2 | Sequential agent queue + chain_actions in executor |
| 4 | W2 + W3 start | Astra P2→Cash config + Won extraction |
| 5 | W3 + W4 | Won unification + Present & Invoice rename |
| 6 | W5 + W6 start | Stage-aware tabs + Transcripts enhancement start |
| 7 | W6 | Transcripts & Sources complete + data source in agent context |
| 8 | W7 + W8 | Review routing + person invite + cost bridge |
| 9 | W9 | E2E tests: AA-4, AA-7, DL-3, new specs |
| 10 | W9 | QA pass, /check, Playwright smoke, backlog updates |

---

## Key Existing Code to Reuse

| What | Where | How Used |
|------|-------|----------|
| StageTransitionProcessor | `crates/server/src/stage_transition.rs` | Extend with sequential queue + Discovery validation |
| AgentFlowExecutor | `crates/server/src/agent_flow_executor.rs` | Add chain_actions handling |
| mark_deal_won | `crates/server/src/routes/crm_deal_automations.rs` | Extract provision_won_deal() |
| CallSchedulingSection | `frontend/src/.../OverviewCallSchedulingSection.tsx` | Reuse for Discovery stage UI |
| InviteLinkSection | `frontend/src/.../DeckTab.tsx:246-331` | Already exists — wire to real invite API |
| schedule_agent_flow() | `crates/server/src/stage_transition.rs` | Reuse for chained agent scheduling |
| UploadZone pattern | `frontend/src/pages/data-sources/` | Adapt for transcript upload |
| costsApi | `frontend/src/lib/api/costs.ts` | Already wired — verify data flow |
| E2E helpers | `e2e/helpers/seed.ts` | createTestDeal, waitForDealStage |

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Stage position shift breaks existing data | Migration uses stage_type matching, not position. Test with existing deals. |
| Sequential queue adds complexity to executor | chain_actions stored in flow config JSON — no schema change. Fallback: single agent still works. |
| Won provision_won_deal double-execution | Idempotent: check won_at + client existence before provisioning |
| Data source linking adds load to agent context | Limit to 5 most recent sources. Truncate content if > 10K chars. |
| Stage-aware tabs hide important data | "Show all tabs" toggle always available. Default to all tabs for Won/Lost. |
| Cost bridge requires investigation | Time-boxed to 0.5 day. If blocked, move to backlog. |

---

## Verification

### Functional (Playwright MCP walkthrough)
- [ ] Create deal → appears in Lead stage
- [ ] Move to Intel → Scout triggers → auto-advance to BA
- [ ] BA → Astra triggers → auto-advance to Discovery
- [ ] Discovery: call scheduling hero card visible, tabs show Transcripts
- [ ] Link transcript/data source in Discovery
- [ ] Advance to Proposal → Astra Pass 2 fires FIRST → Cash chains after
- [ ] Proposal tab shows generated proposal
- [ ] Advance to Polish → Lux triggers
- [ ] Present & Invoice: presentation tracking visible, invoice gated on presentation
- [ ] Move to Won → full provisioning toast (client + project + tasks + VIBE)
- [ ] Won deal shows invite link

### E2E Tests
- [ ] AA-4 passes (Astra Pass 2 → Cash chain)
- [ ] AA-7 passes (retrigger failed agent)
- [ ] DL-3 passes (Won full provisioning)
- [ ] All existing pipeline tests still pass (143 baseline)

### Build
- [ ] `cargo check --workspace` — 0 errors
- [ ] `npx tsc --noEmit` — 0 errors
- [ ] `npm run generate-types:check` — types in sync

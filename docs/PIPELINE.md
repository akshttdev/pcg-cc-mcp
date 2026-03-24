# CRM Pipeline — Architecture & Operations Guide

> Canonical reference for how the CRM pipeline works, its agent automation system, and the full deal lifecycle from Lead to Delivery.

**Origin**: Rebuilt from the sloperation317 dealflow pipeline plan. See `planning/2026-03-18--reference--pipeline-status.md` for the original intended flow and feature items (F1–F13). See `planning/2026-03-18--plan--sloperation317-integration.md` for the integration strategy.

## Executive Summary

The CRM pipeline is a **9-stage deal progression system** with **AI agent automation** at 4 key stages. When a deal moves into an agent-owned stage, the system automatically schedules an AI agent (Scout, Astra, Cash, or Lux), gives the user a 30-second cancel window, then executes the agent via LLM with deal-specific tools. On completion, the deal auto-advances to the next stage, chaining agents through the pipeline without manual intervention.

**Current state**: Fully operational with simulated LLM responses (`SIMULATE_LLM=1`). Real LLM execution requires funded Anthropic API credits. 5 sloperation intent gaps remain (see "Sloperation Intent Gaps" section below).

---

## Pipeline Stages

### Acquisition Pipeline (Sales)

| # | Stage | Owner | Agent | Auto-Trigger | What Happens on Entry |
|---|-------|-------|-------|-------------|----------------------|
| 1 | **Lead** | Sales Rep | — | No | Manual entry point. No automation. |
| 2 | **Intel** | Scout (AI) | scout | Yes (30s) | Scout researches contact + company. Creates review task. Exit requires person + company intel done. |
| 3 | **Business Analysis** | Astra (AI) | astra | Yes (30s) | Astra analyzes pain points, opportunities, recommended services. Creates review task. |
| 4 | **Proposal** | Cash (AI) | cash | Yes (30s) | Cash generates proposal (executive summary, scope, pricing, timeline). Creates review task. |
| 5 | **Polish** | Lux (AI) | lux | Yes (30s) | Lux creates presentation deck. Exit requires proposal_text. Creates review task. |
| 6 | **Invoice** | Account Manager | — | No | Human sends invoice. Review task: confirm payment. |
| 7 | **Negotiation** | Nora + AM | — | No | Human follows up. Review task: update status. |
| 8 | **Won** | Team | — | No | Auto-creates delivery deal (deduped by contact). Sets won_at. |
| 9 | **Lost** | Account Manager | — | No | Terminal. Sets lost_at. |

### Delivery Pipeline

9 stages (Onboarding → Brand Guide → Online Presence → Social Stack → Monthly Retainer → Active Retainer → Completed). All human-driven, no agent automation.

---

## Agent Flow Lifecycle

```
Deal moves to agent stage
        │
        ▼
┌─────────────────────┐
│  Schedule Agent Flow │  status: planning
│  cancel_deadline:    │  cancel_window: 30s
│  now + 30s           │
└─────────┬───────────┘
          │
    ┌─────┴──────┐
    │            │
 Cancel      Run Now / Wait 30s
    │            │
    ▼            ▼
 Failed    ┌─────────────┐
           │  Executing   │  LLM dispatch + tools
           │  (15s poll)  │  max 5 turns
           └──────┬──────┘
                  │
           ┌──────┴──────┐
           │             │
        Success       Failure
           │             │
           ▼             ▼
      Completed       Retry (3x)
           │          → fallback model
           │          → Failed
           ▼
     Auto-advance?
     (if agent-owned stage)
           │
           ▼
     Next stage entry
     → may trigger next agent
```

### Agent Tools

| Tool | What It Does |
|------|-------------|
| `get_deal_context` | Reads deal name, description, stage, amount, currency, proposal_text |
| `update_deal_field` | Updates description, proposal_text, deck_url, or custom_fields |
| `save_artifact` | Stores output as AgentFlowEvent (artifact_created) |

### Agent Chain (Auto-Advance)

When an agent completes and the current stage is agent-owned:
1. Check if all flows for this deal are done
2. Auto-move deal to next stage by position
3. Trigger entry actions on new stage (may schedule next agent)

**Example chain**: Intel (Scout) → BA (Astra) → Proposal (Cash) → Polish (Lux) → Invoice (human stops chain)

---

## Environment Setup

### Required for Agent Execution

```bash
# In .env
ENABLE_AGENT_FLOW_ENGINE=1    # Start background worker (polls every 15s)
SIMULATE_LLM=1                # Use simulated responses (no API credits needed)
# OR
ANTHROPIC_API_KEY=sk-ant-...  # Real LLM execution (requires funded account)
```

### Required for E2E Tests

```bash
FRONTEND_PORT=3010
BACKEND_PORT=3012
VITE_SKIP_ONBOARDING=1
GITHUB_TOKEN=...              # For demo tests with GitHub sandbox
```

### Seed Data

```bash
# Fresh database from seed
cp dev_assets_seed/test-seed.sqlite dev_assets/db.sqlite

# Seed agents (10 total: Nora, Maci, Editron, Genesis, Astra, Scout, Auri, Topsi, Cash, Lux)
curl -X POST http://localhost:3012/api/agents/seed
```

**Note**: Auto-created pipelines (Powerclub Global org) may not have `stage_config` set. The seed migration (`20260414000001`) only updates stages with matching `stage_type`. If stages lack `stage_type`, set `stage_config` manually or use a pipeline created via the settings UI.

---

## API Endpoints

### Deal Stage Movement

| Method | Path | Description |
|--------|------|-------------|
| PATCH | `/api/crm/deals/:id/stage` | Move deal to any stage. Returns TransitionResult with agent_flow_id, warnings, actions_taken. |
| POST | `/api/crm/deals/:id/advance` | Move to next stage (validates exit gates + review tasks). |
| GET | `/api/crm/deals/:id/advance-requirements` | Pre-flight: pending tasks, intel status, next stage. |

### Agent Flow Control

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/crm/deals/:id/cancel-agent` | Cancel pending flow within cancel window. |
| POST | `/api/crm/deals/:id/approve-agent` | Clear cancel deadline — agent executes on next poll. |
| POST | `/api/crm/deals/:id/retrigger-agent` | Re-trigger agent for failed/cancelled flows. |
| GET | `/api/crm/deals/:id/agent-flows` | List flows + events for a deal. |

### TransitionResult Response

```json
{
  "deal": { "id": "...", "stage": "Business Analysis", "probability": 35 },
  "warnings": [{ "field": "description", "message": "Operator context recommended" }],
  "agent_flow_id": "abc-123",
  "cancel_deadline": "2026-03-24 12:00:30.000",
  "actions_taken": ["Scheduled astra agent", "Created review task"]
}
```

---

## Stage Configuration (stage_config JSON)

Each pipeline stage can have a JSON `stage_config` that drives automation:

```json
{
  "assigned_agent": "scout",
  "auto_trigger": true,
  "cancel_window_secs": 30,
  "required_fields": ["description"],
  "approval_gate": false,
  "on_enter_actions": [
    { "type": "trigger_agent", "agent": "scout", "flow_type": "research" },
    { "type": "create_review_task", "description": "Review Phase I intelligence" }
  ],
  "on_exit_validations": [
    { "type": "require_field", "field": "description", "message": "Required" },
    { "type": "require_intel", "entity": "person", "status": "done" }
  ],
  "stage_owner": { "label": "Scout", "type": "agent" }
}
```

Stages without `stage_config` (NULL) fall back to hardcoded behavior in `run_hardcoded_entry_actions()`.

---

## What's Working (Verified 2026-03-24)

- Deal CRUD + stage movement (DnD, context menu, progression bar, advance)
- All 4 agents trigger on stage entry with cancel window + Run Now
- Agent execution (simulated) with real tool calls updating deal data
- Auto-advance chains agents through pipeline
- Review task creation per stage
- Won → Delivery deal auto-creation (contact_id dedup)
- Agent History tab shows completed flows with status, duration, events
- Pipeline Settings with Stages/Automations/Pipeline tabs
- Dynamic stage owner badges from stage_config
- Deal detail: 9 tabs, Sheet↔Dialog expand, 2-column Overview
- Call scheduling, invite links (Won deals), invoice sending
- Toast feedback on every mutation
- Kanban instant refresh after stage moves
- Dark mode fully themed
- Zero console errors across all interactions

## Sloperation Intent Gaps — RESOLVED (2026-03-24)

All 5 missing intents and 3 partial implementations resolved in the sloperation sprint (`feature/2026-03-24--sloperation-sprint`). See `planning/2026-03-24--plan--sloperation-intent-sprint.md`.

| Gap | Status | Resolution |
|-----|--------|-----------|
| **Discovery stage** | ✅ Restored | Position 3 between BA and Proposal. Hero card, call scheduling, transcript exit gate. |
| **Astra Pass 2** | ✅ Implemented | Sequential agent queue. Proposal entry triggers Astra deep_research first, chains to Cash. |
| **Astra→Cash chaining** | ✅ Implemented | chain_actions in flow_config. Executor chains next agent before auto-advance. |
| **Present stage** | ✅ Consolidated | Invoice renamed to "Present & Invoice". Presentation tracking in DeckTab gates invoice. |
| **Transcript→Proposal** | ✅ Implemented | deal_data_sources join table with dual agent/stage scoping. Agent context includes linked sources. |
| **F8: Review routing** | ✅ Implemented | stage_config.review_assignee with org owner fallback. |
| **Won dual-path** | ✅ Fixed | provision_won_deal() extracted. All Won arrivals trigger full provisioning. |
| **F3: Person invite** | ⚠️ Stub | Token + email deferred. Backlog: Resend integration. |

### Current Stage Mapping (Post-Sprint)

```
SLOPERATION (8 stages)          CURRENT (11 stages)         STATUS
═════════════════════           ═══════════════════         ══════
                                Lead (manual)               ADDED
Intel (Scout)                   Intel (Scout)               MATCH
Business Analysis (Astra)       Business Analysis (Astra)   MATCH
Discovery (Human call)          Discovery (AM + Nora)       ✅ RESTORED
  → Astra Pass 2 transition    → Sequential queue on entry  ✅ IMPLEMENTED
Proposal (Cash, from Pass 2)    Proposal (Astra P2→Cash)    ✅ CHAINED
Polish (Lux)                    Polish (Lux)                MATCH
Present (deck on live call)     Present & Invoice (AM)      ✅ CONSOLIDATED
Follow Up (human)               Negotiation (Nora + AM)     RENAMED
Won (full provisioning)         Won (full provisioning)     ✅ UNIFIED
                                Lost (terminal)             MATCH
```

## Other Known Gaps

| Gap | Impact | Effort |
|-----|--------|--------|
| Auto-match transcripts from call intake | Manual linking required | 1-2 days |
| AR invoice dashboard | No revenue tracking view | 2-3 days |
| Real LLM execution | Requires funded API credits | 0 (just add credits) |
| Per-org stage_config overrides | Global defaults only | 1 day |
| Cost bridge (W8) | Agent flows don't record token usage | 0.5 day |
| Data source picker dialog | Currently paste UUID only | 1 day |

---

## Production Deployment

### Required Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `ENABLE_AGENT_FLOW_ENGINE` | `0` (off) | Set to `1` to enable the background agent flow worker |
| `SIMULATE_LLM` | `0` (off) | **Development/testing only.** Set to `1` for simulated agent responses. Never enable in production — agents must produce real work. |
| `AGENT_FLOW_POLL_INTERVAL` | `15` | Seconds between engine ticks. Lower = faster agent response, higher = less DB load. |
| `ANTHROPIC_API_KEY` | required if `SIMULATE_LLM=0` | API key for Claude LLM calls |
| `BACKEND_PORT` | `3002` | Server port |
| `DATABASE_URL` | `sqlite:dev_assets/db.sqlite` | SQLite database path |

### Deployment Checklist

1. **Database**: Run all migrations. Migration `20260416000000` normalizes BLOB UUIDs to TEXT — run once, idempotent.
2. **Stage configs**: Verify `crm_pipeline_stages.stage_config` is populated for all agent stages. If blank, re-run `20260414000001_seed_stage_configs.sql`.
3. **Agent engine**: Set `ENABLE_AGENT_FLOW_ENGINE=1`. Without this, agent flows stay in "planning" indefinitely.
4. **LLM mode**: Do NOT set `SIMULATE_LLM=1` in production. Real LLM calls require a funded `ANTHROPIC_API_KEY`. Use `SIMULATE_LLM=1` only in dev/staging to validate the pipeline without API costs.
5. **Cancel window**: Default 30s per stage config. Deals entering agent stages show a toast with Cancel/Run Now. "Run Now" clears the cancel_deadline so the engine picks it up on the next tick.
6. **Auto-advance**: After each agent completes, the deal auto-moves to the next stage and triggers the next agent. The full chain (Intel→BA→Proposal→Polish) runs without user intervention.
7. **Error handling**: All pipeline DB writes log at `tracing::error!` level on failure. Monitor logs for `[StageTransition]` and `[AgentFlowEngine]` prefixes.
8. **Retrigger**: If an agent fails, users can retry via the Agent History tab "Retry Agent" button, or `POST /crm/deals/:id/retrigger-agent`.

### Monitoring

Watch for these log patterns:
- `[AgentFlowEngine] Found N pending flows` — engine is processing
- `[AgentFlowEngine] Auto-advancing deal` — deal moving between stages
- `[AgentFlowEngine] Flow completed successfully` — agent finished
- `[StageTransition] Failed to` — stage transition error (investigate)
- `[mark_deal_won] Failed to` — won chain error (client/project creation)

## Key Source Files

| File | Purpose |
|------|---------|
| `crates/server/src/stage_transition.rs` | Unified StageTransitionProcessor — all stage moves go through here |
| `crates/server/src/agent_flow_executor.rs` | Background worker — polls, executes, auto-advances |
| `crates/server/src/routes/crm_deal_transitions.rs` | move_deal_stage, advance_deal, cancel/approve/retrigger |
| `crates/db/src/models/agent_flow.rs` | AgentFlow model, FlowStatus FSM, find_pending_flows |
| `crates/db/migrations/20260414000001_seed_stage_configs.sql` | Default stage configs for all pipeline stages |
| `frontend/src/components/crm/CrmPipelineBoard.tsx` | Kanban board with DnD + agent toasts |
| `frontend/src/components/crm/CrmDealCard.tsx` | Deal card with status badges + agent indicators |
| `frontend/src/components/crm/deal-detail/` | Deal detail panel (9 tabs, expand mode) |
| `frontend/src/hooks/useCrmPipeline.ts` | useMoveDeal hook with optimistic update + cache invalidation |
| `e2e/pipeline/` | E2E tests: lifecycle, automations, detail, config, review gates |

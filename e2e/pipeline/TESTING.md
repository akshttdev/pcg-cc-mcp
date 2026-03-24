# Pipeline E2E — Setup & Rules

See also: `e2e/TESTING.md` for shared conventions.

## Backend Environment

Pipeline tests require the agent flow engine with LLM simulation:

```bash
ENABLE_AGENT_FLOW_ENGINE=1  # Enables background agent flow polling (15s interval)
SIMULATE_LLM=1              # Returns realistic simulated results without LLM API calls
BACKEND_PORT=3002
DATABASE_URL="sqlite:dev_assets/db.sqlite"

# Full start command
ENABLE_AGENT_FLOW_ENGINE=1 SIMULATE_LLM=1 BACKEND_PORT=3002 DATABASE_URL="sqlite:dev_assets/db.sqlite" ./target/debug/server
```

Without these, agent flows stay in "planning" forever and agent-stage tests will fail/timeout.

## Stage Ownership Model

The pipeline has two types of stages:

### Human stages — user moves the deal manually
- **Lead**: Sales rep qualifies the lead
- **Invoice**: Account manager sends invoice
- **Negotiation**: Nora + AM handle negotiation
- **Won**: Team closes the deal
- **Lost**: Account manager records loss

### Agent stages — agent runs and auto-advances on completion
- **Intel** → Scout agent (research)
- **Business Analysis** → Astra agent (analysis)
- **Proposal** → Cash agent (proposal generation)
- **Polish** → Lux agent (deck/presentation)

**Tests should NOT manually move deals past agent stages.** Instead:
1. Move deal INTO the agent stage (e.g., Lead → Intel via context menu)
2. Wait for the agent toast ("Scheduled scout agent") — verify Cancel/Run Now buttons
3. Wait for the agent to complete (poll the card for status change or wait for auto-advance)
4. Verify the deal moved to the next stage automatically

### Auto-advance flow
When all agent flows for a deal complete:
1. Engine checks if current stage is agent-owned (has `agent` in stage_config)
2. If no pending flows remain, moves deal to next stage by position
3. Runs `process_transition` on the new stage (which may trigger the next agent)

This means: moving a deal to Intel triggers Scout → Scout completes → deal auto-moves to BA → Astra triggers → Astra completes → deal auto-moves to Proposal → etc.

## Test Files

| File | Coverage |
|------|----------|
| `pipeline-flow.spec.ts` | Full deal lifecycle: Lead → Intel → BA → Proposal → Won |
| `pipeline-config.spec.ts` | Pipeline settings dialog, stage config, automation tabs |
| `deal-lifecycle.spec.ts` | DL-1..4: create, move, won chain, lost tracking |
| `deal-detail.spec.ts` | DD-1..4: panel tabs, call scheduling, invoice, invite link |
| `agent-automations.spec.ts` | AA-1..5: agent triggering, cancel window, chaining |
| `review-gates.spec.ts` | RG-1..2: review tasks, advance validation, intel gating |

## Testid Helpers

Pipeline testids are in `e2e/pipeline/testids.ts` (re-exports from `shared/testids.ts`):

```typescript
import { pipeline, dealCard, dealDetail, callScheduling, deck, pipelineSettings, tabs } from "./testids";
```

Key testids:
- `pipeline.stageColumn("intel")` → `stage-column-intel`
- `pipeline.settings` → `pipeline-settings`
- `dealCard.menu(deal.id)` → `deal-menu-{id}`
- `callScheduling.date("discovery")` → `call-discovery-date`
- `deck.sendInvoice` → `deck-send-invoice`
- `deck.generateInvite` → `deck-generate-invite`
- `pipelineSettings.stageEdit("intel")` → `stage-edit-intel`

## Pipeline Helpers

`e2e/pipeline/helpers.ts` provides:
- `ORG_ID` — test organization UUID
- `PIPELINE_URL` — direct URL to the acquisition pipeline
- `moveDealViaContextMenu(page, dealText, stageName)` — opens card menu → Move to → stageName

## Review Gate Behavior

- **Intel exit**: requires description + person_intel + company_intel (soft gate — returns structured warnings)
- **All stages**: pending review tasks block advance (hard gate — returns 400)
- Tests in `review-gates.spec.ts` verify both soft and hard gating

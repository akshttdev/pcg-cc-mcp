# Phase 1: Pipeline Workflow Enhancement — Stages, Intelligence & Conversation Persistence

> **Status:** In Progress
> **Branch:** `feature/pipeline-workflow-enhancement`
> **Created:** 2026-03-15

## Context

Pipeline deal cards and detail panels are functional but sparse. The current "Clients" pipeline stages (Lead → Qualified → Proposal → Negotiation → Won/Lost) don't reflect the actual sales workflow with human review gates. Discord bot conversations (Nora/Topsi) have persistence infrastructure (`AgentConversation` + `AgentConversationMessage` models) but the Discord-specific chat handlers may not be wiring into it consistently. This plan covers: redefining the pipeline stages with human gates, enriching the deal UI, wiring auto-triggered intelligence, and ensuring conversation persistence.

**Key existing infrastructure to reuse:**
- `CrmDealWithContact` already has: `intelligence_status`, `intelligence_summary`, `intelligence_confidence`, `research_pass_count`, `report_id`, `report_status`, `report_review_status`, `review_task_id`, `review_task_status`, `review_task_assignee`
- `move_deal_stage()` already has hooks for auto-creating delivery deals on sales wins and creating review tasks at "Analysis Done" stages
- `get_deal_rich()` endpoint already exists with person intel, business report, company intel, tasks, and knowledge sources
- `AgentConversation::get_or_create()` + `AgentConversationMessage::add_*()` persistence model exists
- Person research flow: `POST /api/persons/:id/research` → Nora delegates to Scout/Astra → results stored in `persons.intelligence_*` fields
- `api.ts` has been split into `frontend/src/lib/api/` — new endpoints go in `frontend/src/lib/api/crm.ts`

---

## The Pipeline Workflow (8 Stages)

Each stage has automated work + a human review gate before advancing.

| # | Stage | Color | Prob | Auto-trigger | Human Gate |
|---|-------|-------|------|-------------|------------|
| 1 | Lead | #6B7280 | 5% | "Who Is" person + company research | Verify research, confirm lead |
| 2 | Business Analysis | #3B82F6 | 20% | MBA-level business analysis report | Verify report, schedule discovery |
| 3 | Discovery | #8B5CF6 | 40% | Post-call research integration | Confirm knowledge is proposal-ready |
| 4 | Build Proposal | #F59E0B | 55% | Generate tiered proposal from intel | Verify services/budgets |
| 5 | Polish | #EC4899 | 70% | Create report card + presentation deck | QA graphical/data quality |
| 6 | Proposal Meeting | #EF4444 | 85% | (none — schedule and pitch) | (terminal active stage) |
| 7 | Closed Won | #22C55E | 100% | Auto-create delivery deal | — |
| 8 | Closed Lost | #9CA3AF | 0% | — | — |

---

## Implementation Steps

### Step 1: Update Pipeline Stages (S effort)

**File:** `crates/db/src/models/crm_pipeline.rs` (~line 453-460)

Change `create_clients_pipeline()` stages from the current 6 to 8:

```rust
let stages = vec![
    ("Lead",              "#6B7280", 0, false, false, 5),
    ("Business Analysis", "#3B82F6", 1, false, false, 20),
    ("Discovery",         "#8B5CF6", 2, false, false, 40),
    ("Build Proposal",    "#F59E0B", 3, false, false, 55),
    ("Polish",            "#EC4899", 4, false, false, 70),
    ("Proposal Meeting",  "#EF4444", 5, false, false, 85),
    ("Closed Won",        "#22C55E", 6, true,  true,  100),
    ("Closed Lost",       "#9CA3AF", 7, true,  false, 0),
];
```

**Migration:** Create `crates/db/migrations/20260316000000_update_clients_pipeline_stages.sql`:
- Find all `crm_pipeline_stages` belonging to Clients-type pipelines
- Delete old stages, insert new 8
- Map existing deals: Lead→Lead, Qualified→Lead, Proposal→Build Proposal, Negotiation→Proposal Meeting, Won→Closed Won, Lost→Closed Lost

### Step 2: Conversation Persistence — Nora & Topsi Discord (M effort)

**Existing models to reuse:**
- `AgentConversation::get_or_create(pool, agent_id, session_id, project_id)` — `crates/db/src/models/agent_conversation.rs:257`
- `AgentConversationMessage::add_user_message(pool, conversation_id, content)` — line 450
- `AgentConversationMessage::add_assistant_message(pool, conversation_id, content, model, provider, input_tokens, output_tokens, latency_ms)` — line 476
- Pattern already used in `crates/server/src/routes/agent_chat.rs` (lines 159-480)

**File:** `crates/server/src/routes/nora.rs` — `chat_with_nora()` handler
- After Nora processes a request, call `AgentConversation::get_or_create()` with the nora agent ID + session_id
- Save user message and assistant response via the existing `add_*_message()` methods
- Load persisted history as fallback when in-memory is empty (agent restart recovery)

**File:** `crates/server/src/routes/topsi.rs` — `chat_with_topsi()` handler
- Same persistence pattern as Nora
- Per-user isolation already handled by session_id + user_id scoping

### Step 3: Auto-Trigger "Who Is" Research on Stage Transitions (M effort)

**File:** `crates/server/src/routes/crm_deals.rs`

Extend `move_deal_stage()` (~line 348) — it already has hooks for sales→delivery and "Analysis Done" task creation. Add stage-aware triggers:

```
Stage contains "lead"              → trigger person + company "Who Is" research
Stage contains "business analysis" → create review task for human gate #2
Stage contains "discovery"         → create review task for human gate #3
Stage contains "build proposal"    → create review task for human gate #4
Stage contains "polish"            → create review task for human gate #5
```

**Research trigger** reuses existing flow:
1. Look up `crm_contact` → `person_id` for the deal
2. If person's `intelligence_status` is NULL or "idle", call the existing person research endpoint logic (`POST /api/persons/:id/research` internally)
3. If contact has `company_name`, check/create Company record → trigger company research if idle

**File:** `crates/server/src/routes/intelligence.rs`

After `write_intelligence_results()` completes for a person:
1. Look up CRM deals linked to this person (via `CrmDeal::find_by_contact()` — already exists at line 277)
2. If deal is in "Lead" stage and research is complete → mark `intelligence_status` as "complete" (human gate reviews before advancing)
3. If person has `company_name` → check/create Company → trigger company research if idle

### Step 4: Review → Advance Flow (S effort)

**New endpoint:** `POST /api/crm/deals/:id/advance`

**File:** `crates/server/src/routes/crm_deals.rs`

```
1. Validate current review task is approved (review_task_status == "done")
2. Find current stage → look up next stage by position + 1
3. Call existing CrmDeal::move_to_stage()
4. Stage-entry hooks fire automatically (Step 3)
```

**Frontend:** Add `advanceDeal(dealId)` to `frontend/src/lib/api/crm.ts`

### Step 5: Enrich Deal Cards (S effort)

**File:** `frontend/src/components/crm/CrmDealCard.tsx`

The card already shows intel badges, review status, contact info, and task progress. Add:

1. **Stage-aware indicators** (extend existing intel badge row):
   - Lead + no intel → amber "Research Needed" badge
   - Lead + intel complete → green "Ready for Review" badge
   - Any stage + pending review task → pulsing "Needs Review" badge
2. **Company intel status** — small icon showing company research state (requires adding `company_intelligence_status` to `CrmDealWithContact`)

**Backend:** Add `company_intelligence_status: Option<String>` to `CrmDealWithContact` in `crates/db/src/models/crm_deal.rs`, populated via LEFT JOIN on companies table in the enrichment query (~line 607+).

**Frontend types:** Update `frontend/src/types/crm.ts` — add `company_intelligence_status` to `CrmDealWithContact` interface.

### Step 6: Improve Deal Detail Panel (M effort)

**File:** `frontend/src/components/crm/CrmDealDetailPanel.tsx`

#### 6a. Pipeline Stage Stepper
Below the summary stats grid (~line 165):
- Horizontal progress indicator showing all pipeline stages
- Current stage highlighted, completed stages green, upcoming muted
- Each stage shows human gate status (pending/approved)

#### 6b. Enhance Intel Tab (existing tab, ~line 212)
- Add company intelligence section alongside person intel
- Inline "Trigger Research" button (not just link to person page)
- Person social profiles summary

#### 6c. Add Review Tab (new tab)
- Current review task with status and assignee
- "Approve & Advance" button → calls `POST /api/crm/deals/:id/advance`
- "Request More Research" button → triggers another research pass
- Person + Company wiki summaries side by side
- Stage-specific review checklist

#### 6d. Rich Data Loading
- `get_deal_rich()` endpoint already exists at `GET /api/crm/deals/:id/rich` (~line 115 of crm_deals.rs)
- `getDealRich(dealId)` already exists in `frontend/src/lib/api/crm.ts`
- Create hook `frontend/src/hooks/useDealRich.ts` wrapping the existing endpoint with React Query

---

## Implementation Order

1. **Step 1** — Pipeline stages (backend model + migration)
2. **Step 2** — Conversation persistence (Nora/Topsi)
3. **Step 3** — Auto-trigger Who Is research (backend hooks in move_deal_stage)
4. **Step 4** — Review→advance endpoint (backend)
5. **Step 5** — Deal card enrichment (frontend)
6. **Step 6** — Detail panel improvements (frontend)

---

## Critical Files

| File | Changes |
|------|---------|
| `crates/db/src/models/crm_pipeline.rs` | Update `create_clients_pipeline()` to 8 stages |
| `crates/db/migrations/20260316000000_update_clients_pipeline_stages.sql` | Migrate existing pipeline stages + remap deals |
| `crates/server/src/routes/nora.rs` | Wire `AgentConversation` persistence in chat handler |
| `crates/server/src/routes/topsi.rs` | Wire `AgentConversation` persistence in chat handler |
| `crates/server/src/routes/crm_deals.rs` | Stage-aware hooks in `move_deal_stage()`, new `advance` endpoint |
| `crates/server/src/routes/intelligence.rs` | Auto-link research results to CRM deals, company research trigger |
| `crates/db/src/models/crm_deal.rs` | Add `company_intelligence_status` to `CrmDealWithContact` |
| `frontend/src/components/crm/CrmDealCard.tsx` | Stage-aware badges, company intel indicator |
| `frontend/src/components/crm/CrmDealDetailPanel.tsx` | Stage stepper, review tab, enhanced intel tab |
| `frontend/src/hooks/useDealRich.ts` | New React Query hook wrapping existing `getDealRich` |
| `frontend/src/lib/api/crm.ts` | Add `advanceDeal()` endpoint |
| `frontend/src/types/crm.ts` | Add `company_intelligence_status` field |

---

## Verification

1. Delete `dev_assets/db.sqlite` and restart backend — verify new 8-stage Clients pipeline auto-creates
2. Create a deal in Lead stage — verify "Who Is" research auto-triggers for linked contact
3. Send a message to Nora in Discord — verify conversation persisted in `agent_conversation_messages` table
4. Deal cards show stage-aware indicators (Research Needed / Ready for Review / Needs Review)
5. Open deal detail panel — verify stage stepper renders all 8 stages with correct highlighting
6. Approve research in Review tab → verify deal advances to Business Analysis stage
7. `cargo clippy` and `npx tsc --noEmit` pass after all changes

# Consolidated Pipeline Roadmap — ORCHA Sovereign Stack

**Date:** 2026-03-18
**Status:** ACTIVE — consolidation of all pipeline planning from 2026-03-12 through 2026-03-18
**Audience:** Bodhi (lead/arch), Fraze (strategy/product), Madhav (dev)

---

## Source Documents

This document consolidates the following planning artifacts:

| Document | Date | Focus |
|----------|------|-------|
| `2026-03-15--plan--long-term-scope-ecosystem-alignment.md` | 03-15 | Full sovereign stack ecosystem alignment, 4-horizon roadmap |
| `2026-03-15--plan--pipeline-workflow-enhancement.md` | 03-15 | 9-stage CRM pipeline, intelligence auto-triggers, conversation persistence |
| `2026-03-14--plan--dogfood-qa-p1-fixes.md` | 03-14 | QA sprint — 8 P1/P2 bug fixes, 14 E2E fixes, 7 PR review rounds |
| `2026-03-14--plan--e2e-demo-refactor.md` | 03-14 | E2E test refactoring for headed mode demos |
| `2026-03-15--plan--sidebar-collapse-modularize.md` | 03-15 | Sidebar collapse fix + modularization (PR #30) |
| `2026-03-12--plan--agent-task-mcp-wiring.md` | 03-12 | Agent-Task MCP integration, phases 1-3 (3H not started) |
| `2026-03-12--plan--role-based-ui.md` | 03-12 | Role-based UI, view-as context switcher |
| `BACKLOG--remaining-work.md` | 03-14 | Consolidated backlog with resolved/open items |
| `notes/2026-03-15--analysis--tech-debt-review.md` | 03-15 | 24-finding tech debt review, 4 critical |
| `archive/2026-03-12--plan--three-sprint-roadmap.md` | 03-12 | 3-sprint dashboard improvement roadmap |

---

## Pipeline Architecture — Intended Use Case

The ORCHA pipeline is a **many-to-many human-agent collaboration engine** for agency operations. It is NOT a single linear pipeline — it's a system of interconnected pipelines serving the agency workflow model.

### Master Pipeline Diagram — Agency Workflow

```
                        ORCHA AGENCY PIPELINE — ACTUAL ARCHITECTURE
 ========================================================================================

                          ┌─────────────────────────────────────┐
                          │        AGENTIC INTAKE LAYER          │
                          │  (Implemented — `intake/pipeline.rs`) │
                          │                                       │
                          │  INGEST ───────────────────────────── │
                          │  │ Email webhook (SendGrid/Zoho)      │
                          │  │ Manual transcript upload            │
                          │  │ Call log ingestion (Twilio)         │
                          │  │ SMS intake (Twilio)                 │
                          │  │                                     │
                          │  ▼                                     │
                          │  EXTRACT (Claude LLM) ──────────────  │
                          │  │ Participants, pain points, topics   │
                          │  │ Action items, sentiment, opps       │
                          │  │ Individuals, businesses identified  │
                          │  │                                     │
                          │  ▼                                     │
                          │  ASSOCIATE (Fuzzy Match → CRM) ─────  │
                          │  │ Email match → persons table         │
                          │  │ Name+company fuzzy match            │
                          │  │ Create new person if unknown        │
                          │  │ Link person → org, company          │
                          │  │ Auto-create proposal (draft)        │
                          │  │ Auto-create CRM deal (Lead stage)   │
                          │  │                                     │
                          │  ▼                                     │
                          │  RESEARCH (Background — 3 passes) ──  │
                          │  │ Company research (Claude per biz)   │
                          │  │ Person intelligence (scout agent)   │
                          │  │ Advance deal → Research stage       │
                          │  │                                     │
                          │  ▼                                     │
                          │  REPORT & TASKS ────────────────────  │
                          │  │ Register in knowledge graph         │
                          │  │ Create tasks from action items      │
                          │  │   (trusted sender → Nora/Bodhi/     │
                          │  │    Josh assignment routing)          │
                          │  │ Generate business_audit report      │
                          │  │ Ingest contextual individuals       │
                          │  │   into org knowledge graph          │
                          │                                       │
                          │  Org routing: @sirakstudios.com →      │
                          │    Sirak Studios org + Sirak user      │
                          │  All others → PCG org                  │
                          └──────────┬──────────────────────────────┘
                                     │
         ┌───────────────────────────┼───────────────────────────┐
         ▼                           ▼                           ▼
   ┌───────────┐            ┌──────────────┐           ┌──────────────┐
   │  Social   │            │ Data Source  │           │  Webhook /   │
   │  Inbox    │            │ Connectors   │           │  Manual      │
   │ (9 plat-  │            │ (Airtable,   │           │  Entry       │
   │  forms)   │            │  CSV, API)   │           │              │
   └─────┬─────┘            └──────┬───────┘           └──────┬───────┘
         │                         │                          │
         └─────────────────────────┴──────────────────────────┘
                                   │
                                   ▼
   ┌──────────────────────────────────────────────────────────────┐
   │         CLIENTS CRM PIPELINE (9-Stage — Implemented)         │
   │         Commit: 1fe2f2c — with intelligence auto-triggers    │
   │                                                              │
   │  ┌─────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐  │
   │  │  LEAD   │──▶│ BUSINESS │──▶│DISCOVERY │──▶│  BUILD   │  │
   │  │  (5%)   │   │ ANALYSIS │   │  (40%)   │   │ PROPOSAL │  │
   │  │         │   │  (20%)   │   │          │   │  (55%)   │  │
   │  │ Auto:   │   │          │   │          │   │          │  │
   │  │ "Who Is"│   │ Human    │   │ Human    │   │ Human    │  │
   │  │ person +│   │ Gate:    │   │ Gate:    │   │ Gate:    │  │
   │  │ company │   │ Review   │   │ Review   │   │ Confirm  │  │
   │  │ research│   │ research │   │ analysis │   │ discovery│  │
   │  └─────────┘   └──────────┘   └──────────┘   └────┬─────┘  │
   │                                                     │        │
   │  ┌──────────┐  ┌──────────┐  ┌──────────┐    ┌───▼──────┐  │
   │  │ CLOSED   │  │ CLOSED   │  │ FOLLOW   │    │  POLISH  │  │
   │  │  WON     │  │  LOST    │  │   UP     │    │  (70%)   │  │
   │  │ (100%)   │  │  (0%)    │  │  (90%)   │    │          │  │
   │  │          │  │          │  │          │    │ Human    │  │
   │  │ Auto:    │  │          │  │ Agent:   │    │ Gate:    │  │
   │  │ Create   │  │          │  │ Sched    │    │ Verify   │  │
   │  │ Delivery │  │          │  │ follow-  │    │ services │  │
   │  │ pipeline │  │          │  │ ups,     │    │ + budget │  │
   │  │ deal     │  │          │  │ check-ins│    │          │  │
   │  └──────────┘  └──────────┘  └─────┬────┘    └──────────┘  │
   │                                     │                        │
   │                    ┌──────────┐     │                        │
   │                    │ PROPOSAL │     │                        │
   │                    │ MEETING  │─────┘                        │
   │                    │  (85%)   │                              │
   │                    │          │                              │
   │                    │ Human    │                              │
   │                    │ Gate:    │                              │
   │                    │ Present  │                              │
   │                    │ & close  │                              │
   │                    └──────────┘                              │
   │                                                              │
   │  STAGE TRANSITIONS (implemented in crm_deals.rs):            │
   │  • LEAD entry → trigger_who_is_research() [person+company]   │
   │  • BUSINESS ANALYSIS → create_review_task_if_needed()        │
   │  • DISCOVERY → create_review_task_if_needed()                │
   │  • BUILD PROPOSAL → create_review_task_if_needed()           │
   │  • POLISH → create_review_task_if_needed()                   │
   │  • PROPOSAL MEETING → present to prospect                    │
   │  • FOLLOW UP (NEW) → agent-scheduled follow-ups, check-ins, │
   │    decision chasing after presentation. Auto-trigger cadence │
   │  • CLOSED WON (Sales) → auto-create Delivery pipeline deal   │
   │  • POST /api/crm/deals/:id/advance — review-gated endpoint  │
   │                                                              │
   │  DEAL ENRICHMENT (implemented in useDealRich hook):          │
   │  • Stage-aware badges (Research Needed/Ready/Needs Review)   │
   │  • Company intelligence status + summary                     │
   │  • Pipeline stage stepper in detail panel                    │
   │  • Review tab: approve & advance, request research           │
   │  • Intel tab: company intelligence + inline research trigger │
   └──────────────────────────────────────────────────────────────┘
                              │
                     ┌────────┴────────┐
                     ▼                 ▼
   ┌────────────────────────┐  ┌────────────────────────────────┐
   │  WORKFLOW AUTOMATION   │  │  AGENT ORCHESTRATION ENGINE    │
   │  (Node-Based Builder)  │  │  (Phase 3H — NOT STARTED)     │
   │                        │  │                                │
   │  11 node types:        │  │  Intended:                     │
   │  • llm_extract         │  │  ┌─────────────────────────┐   │
   │  • llm_analyze         │  │  │  Phase 1: Research      │   │
   │  • llm_summarize       │  │  │  Agent: Research Bot     │   │
   │  • transform           │  │  │  Gate: Human Review ─────┼─▶ Task Board
   │  • filter              │  │  ├─────────────────────────┤   │
   │  • merge               │  │  │  Phase 2: Sales         │   │
   │  • conditional         │  │  │  Agent: Sales Bot        │   │
   │  • data_source         │  │  │  Human: Account Exec ────┼─▶ Task Board
   │  • output_crm_*        │  │  ├─────────────────────────┤   │
   │  • output_tasks        │  │  │  Phase 3: Graphics      │   │
   │  • send_notification   │  │  │  Agent: Maci/Cinematics  │   │
   │  ─────────────────     │  │  │  Human: Art Director ────┼─▶ Task Board
   │  Missing:              │  │  ├─────────────────────────┤   │
   │  • assign_to_agent     │  │  │  Phase 4: Content       │   │
   │  • http_request        │  │  │  Agent: Content Bot      │   │
   │  • create_project      │  │  │  Human: Editor ──────────┼─▶ Task Board
   │  • update_crm_*        │  │  ├─────────────────────────┤   │
   │  • trigger_schedule    │  │  │  Phase 5: Distribution  │   │
   │  • trigger_entity      │  │  │  Agent: Social Bot       │   │
   │                        │  │  │  Human: Marketing ───────┼─▶ Task Board
   └────────────────────────┘  │  └─────────────────────────┘   │
                               │                                │
                               │  Background worker progresses   │
                               │  phases, enforces gates,        │
                               │  handles delegation & rollback  │
                               └────────────────────────────────┘
                              │
                              ▼
   ┌──────────────────────────────────────────────────────────────┐
   │                 CONTENT PRODUCTION PIPELINE                   │
   │                                                              │
   │  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌───────────┐  │
   │  │ Content  │  │ Platform  │  │ Review   │  │ Schedule  │  │
   │  │ Blocks   │─▶│ Adapt     │─▶│ Queue    │─▶│ & Publish │  │
   │  │ (hook,   │  │ (9 plat-  │  │ (human   │  │ (auto-    │  │
   │  │  body,   │  │  forms,   │  │  approve │  │  post to  │  │
   │  │  CTA,    │  │  char     │  │  or edit)│  │  social)  │  │
   │  │  media)  │  │  limits)  │  │          │  │           │  │
   │  └──────────┘  └───────────┘  └──────────┘  └───────────┘  │
   │                                                              │
   │  Supported: LinkedIn, Twitter, Instagram, Facebook, TikTok,  │
   │  Bluesky, YouTube, Pinterest, Threads                        │
   └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
   ┌──────────────────────────────────────────────────────────────┐
   │                  EXECUTION & MONITORING                       │
   │                                                              │
   │  ┌──────────────┐  ┌─────────────┐  ┌────────────────────┐  │
   │  │ Mission      │  │ Agent Flow  │  │ Autonomy &         │  │
   │  │ Control      │  │ Events      │  │ Approval Gates     │  │
   │  │ (real-time   │  │ (stage      │  │ (manual/supervised │  │
   │  │  dashboard)  │  │  tracking)  │  │  /autonomous)      │  │
   │  └──────────────┘  └─────────────┘  └────────────────────┘  │
   │                                                              │
   │  4 MCP Servers: Task, Nora, Pulse, Topsi                     │
   │  8 Executor Backends: Claude, Gemini, Qwen, Cursor,          │
   │                       CodeX, Duck, AMP, OpenCode              │
   └──────────────────────────────────────────────────────────────┘
```

---

## Current Iteration — What's Built vs What's Planned

### Component Status Matrix

```
  COMPONENT                    BUILT ████████░░  PLANNED ░░░░░░░░░░

  Agentic Intake Pipeline       ████████████  FULL — 6-stage: ingest→extract→associate→
                                              research→report. Auto CRM deal, auto tasks,
                                              company research, knowledge graph registration
  CRM 9-Stage Pipeline          ████████░░░░  BUILT (local) — 9 stages, auto-triggers,
                                              review tasks, advance API. CONFLICTS with
                                              remote main (10 files). Needs merge resolution
  Workflow Node Builder          ████████░░░░  11 node types, staging→commit, manual run only
  Agent MCP Integration          ████████░░░░  26 tools + 6 resources, executors wired
  Content Production             ██████░░░░░░  Blocks, calendar, scheduling — no agent generation
  Social Distribution            ████████░░░░  9 platforms, unified inbox, post composer
  Notification System            ████████░░░░  Bell, per-item read, deep-link, toast
  Conversation Persistence       ████████░░░░  BUILT (local) — AgentConversation wired into
                                              Nora + Topsi chat handlers. CONFLICTS with remote
  Agent Flow Orchestration       ░░░░░░░░░░░░  NOT STARTED — data model only, no worker
  Human Gate → Task Gen          ████░░░░░░░░  PARTIAL — review tasks created at gate stages
                                              via create_review_task_if_needed(), but no
                                              orchestration engine to enforce gates
  Workflow Triggers              ░░░░░░░░░░░░  NOT STARTED — manual run only
  Role-Based UI (RBAC)           ██░░░░░░░░░░  Planning only, 10-role hierarchy designed
  Graph DB / Knowledge           ██░░░░░░░░░░  Entity + topology on SQLite, no graph DB.
                                              Intake pipeline registers to KG already
  VIBELAND Backend               ░░░░░░░░░░░░  Frontend 3D exists, no server-side world
  APN Intranet Addressing        ░░░░░░░░░░░░  Mesh primitives exist, no conventions
  VIBE Privacy Chain             ░░░░░░░░░░░░  Aptos bridge only
  Pythia Federated Learning      ░░░░░░░░░░░░  Client stub only
```

### CRM Pipeline — Current State (commit `1fe2f2c`)

The 9-stage Clients pipeline (8 implemented + Follow Up planned) with intelligence auto-triggers was implemented in a single commit that is currently on local only (conflicts with remote `main`). The Follow Up stage between Proposal Meeting and Closed needs to be added to the migration.

```
  IMPLEMENTED (local commit 1fe2f2c)           STILL NEEDED
  ──────────────────────────────────           ────────────────

  ✅ 8 of 9 stages migrated (Lead → Closed)    ⬜ Add Follow Up stage to migration
                                               ⬜ Conversation persistence
  ✅ Auto "Who Is" research on Lead entry        NOT merged (nora.rs/topsi.rs
  ✅ Review tasks at human gate stages           deleted on remote in #47)
  ✅ POST /deals/:id/advance endpoint          ⬜ Full Nora orchestration of
  ✅ Stage-aware badges on deal cards             research (currently queues
  ✅ Pipeline stage stepper in detail             status only, admin triggers
  ✅ Review tab (approve/advance/research)        manually)
  ✅ Intel tab with company intelligence        ⬜ Resolve merge conflict with
  ✅ useDealRich React Query hook                 upstream refactoring (#47)
  ✅ getDealRich + advanceDeal API methods      ⬜ Agent Flow Engine to
  ✅ Nora/Topsi conversation persistence          automate the human gates
  ✅ company_intelligence_status field            (Phase 3H — NOT STARTED)

  ALL 12 FILES IMPLEMENTED (971 insertions):
  ├── crates/db/migrations/20260402000000_update_clients_pipeline_stages.sql  ✅
  ├── crates/db/src/models/crm_pipeline.rs   — 9-stage definition (Follow Up pending) ✅
  ├── crates/db/src/models/crm_deal.rs       — company_intelligence_status     ✅
  ├── crates/server/src/routes/crm_deals.rs  — advance endpoint + stage hooks  ✅
  ├── crates/server/src/routes/intelligence.rs — link research to CRM deals    ✅
  ├── crates/server/src/routes/nora.rs       — conversation persistence        ✅ (CONFLICTS)
  ├── crates/server/src/routes/topsi.rs      — conversation persistence        ✅ (CONFLICTS)
  ├── frontend/src/components/crm/CrmDealCard.tsx       — stage badges         ✅ (CONFLICTS)
  ├── frontend/src/components/crm/CrmDealDetailPanel.tsx — stepper + review    ✅ (CONFLICTS)
  ├── frontend/src/hooks/useDealRich.ts      — new React Query hook            ✅ (CONFLICTS)
  ├── frontend/src/lib/api.ts                — advanceDeal + getDealRich       ✅ (CONFLICTS)
  └── frontend/src/types/crm.ts              — company_intelligence_status     ✅ (CONFLICTS)
```

### Agentic Intake Pipeline — Current State (implemented in `crates/server/src/routes/intake/`)

This is the **primary entry point** for new leads into the system. It's a 6-stage agentic pipeline that runs automatically on email/call ingestion:

```
  ┌────────────────────────────────────────────────────────────────┐
  │  AGENTIC INTAKE — FULLY IMPLEMENTED                            │
  │  Location: crates/server/src/routes/intake/pipeline.rs         │
  │                                                                │
  │  Stage 1: INGEST                                               │
  │  ├── POST /intake/call-intake/email    (webhook from SendGrid) │
  │  ├── POST /intake/call-intake/upload   (manual transcript)     │
  │  └── Org routing: @sirakstudios.com → Sirak org, else → PCG   │
  │                                                                │
  │  Stage 2: EXTRACT (Claude LLM)                                 │
  │  ├── Participants (name, email, company, role, is_prospect)    │
  │  ├── Individuals (contextual mentions, not prospects)          │
  │  ├── Businesses (name, website, industry, description, size)   │
  │  ├── Topics, pain points, action items, sentiment              │
  │  └── Opportunity signals                                       │
  │                                                                │
  │  Stage 3: ASSOCIATE (CRM Person Matching)                      │
  │  ├── Email match → persons table                               │
  │  ├── Name + company fuzzy match (4-level: exact → first+last   │
  │  │   → single+company → multi+company)                         │
  │  ├── Create new person if no match (type='lead')               │
  │  ├── Link person → organization + company record               │
  │  ├── Auto-create draft proposal (appears in pipeline Kanban)   │
  │  └── Auto-create CRM deal in Acquisition pipeline (Lead stage) │
  │                                                                │
  │  Stage 4: KNOWLEDGE GRAPH                                      │
  │  ├── Register call in project knowledge sources                │
  │  ├── Create tasks from action items (trusted senders only)     │
  │  │   └── Routing: Nora tasks, Bodhi tasks, Josh tasks,        │
  │  │       or unassigned based on owner name extraction          │
  │  └── Ingest contextual individuals into org knowledge graph    │
  │                                                                │
  │  Stage 5: COMPANY RESEARCH (background, up to 3 passes each)  │
  │  ├── Claude researches each extracted business                 │
  │  ├── Ensures company records exist (skip internal companies)   │
  │  ├── Queues person intelligence (scout agent)                  │
  │  └── Advances CRM deal → Research stage                        │
  │                                                                │
  │  Stage 6: REPORT GENERATION                                    │
  │  ├── Business audit report with all research context           │
  │  ├── Executive summary, opportunities, recommended services    │
  │  ├── Market analysis, competitor analysis, digital presence     │
  │  └── Human review endpoints: approve / request revision        │
  └────────────────────────────────────────────────────────────────┘
```

### Agent Orchestration — Current vs Intended

```
  CURRENT STATE                            INTENDED STATE (Phase 3H)
  ─────────────                            ────────────────────────

  Agent Flows:                             Agent Flows:
  ┌────────────────────┐                   ┌────────────────────────────────┐
  │ ● Data model exists│                   │ ● Background orchestration     │
  │   (agent_flows,    │                   │   worker progresses phases     │
  │    flow_events)    │                   │                                │
  │                    │    ──────▶        │ ● Human gates auto-generate    │
  │ ● No worker to     │                   │   tasks on project boards      │
  │   progress phases  │                   │   assigned to responsible      │
  │                    │                   │   humans                       │
  │ ● No human gates   │                   │                                │
  │                    │                   │ ● 5-phase process, 5 gates     │
  │ ● No delegation    │                   │   6-10 agents + humans per     │
  │                    │                   │   workflow                     │
  │ ● Agents execute   │                   │                                │
  │   tasks in         │                   │ ● Phase-aware agent routing    │
  │   isolation        │                   │   (research → sales →          │
  │                    │                   │    graphics → specialized)     │
  │ ● Autonomy API     │                   │                                │
  │   exists but       │                   │ ● Autonomy enforcement at      │
  │   never enforced   │                   │   each gate (can-proceed       │
  └────────────────────┘                   │   validation)                  │
                                           │                                │
                                           │ ● Rollback on failure          │
                                           │ ● Real-time progress in        │
                                           │   Mission Control              │
                                           └────────────────────────────────┘
```

### Workflow Builder — Current vs Intended

```
  CURRENT (11 Node Types)                  INTENDED (+6 Node Types)
  ───────────────────────                  ─────────────────────────

  DATA IN:                                 DATA IN:
  ┌────────────┐                           ┌────────────┐
  │ data_source│                           │ data_source│
  └─────┬──────┘                           │ trigger_   │  ◀── NEW: cron, event
        │                                  │   schedule │
        ▼                                  │ trigger_   │
  PROCESSING:                              │   entity   │  ◀── NEW: state change
  ┌────────────┐                           └─────┬──────┘
  │ llm_extract│                                 │
  │ llm_analyze│                                 ▼
  │ llm_summar.│                           PROCESSING:
  │ transform  │                           ┌────────────┐
  │ filter     │                           │ (all 6     │
  │ merge      │                           │  current + │
  │ conditional│                           │  http_req) │  ◀── NEW: external API
  └─────┬──────┘                           └─────┬──────┘
        │                                        │
        ▼                                        ▼
  OUTPUT:                                  OUTPUT + ACTIONS:
  ┌────────────────┐                       ┌────────────────┐
  │ output_crm_    │                       │ (all current + │
  │   contacts     │                       │  update_crm_*  │  ◀── NEW: update existing
  │ output_crm_    │                       │  assign_agent  │  ◀── NEW: agent delegation
  │   companies    │                       │  create_project│  ◀── NEW: project creation
  │ output_crm_    │                       │  )             │
  │   deals        │                       └────────────────┘
  │ output_tasks   │
  │ send_notif.    │                       KEY MISSING PIECE:
  └────────────────┘                       Workflow triggers — currently
                                           manual run only. Need cron +
  Staging → human review → commit          entity-change triggers for
  (working, validated via E2E)             true automation.
```

---

## Sovereign Stack Roadmap — 4 Horizons

### Horizon 1: Agency MVP (Now → 6 weeks)

**Goal:** PCG + Sirak Studios running deal flow with human+agent collaboration.

```
  PRIORITY ORDER (critical path highlighted)

  ┌─────────────────────────────────────────────────────────┐
  │  ★ #1  PHASE 3H: Agent Flow Orchestration Engine        │  XL effort
  │        Background worker, phase progression,             │  BLOCKS #2-4
  │        gate enforcement, delegation                      │
  ├─────────────────────────────────────────────────────────┤
  │  ★ #2  Human Gate → Task Generation                     │  L effort
  │        Auto-assign to responsible humans                 │  NEEDS #1
  ├─────────────────────────────────────────────────────────┤
  │    #3  CRM Pipeline Hardening                            │  M effort
  │        9-stage + Follow Up, conversation persistence     │  PARTIAL DONE
  ├─────────────────────────────────────────────────────────┤
  │    #4  Multi-Agent Workflow Templates                    │  L effort
  │        research → sales → graphics → content             │  NEEDS #1
  ├─────────────────────────────────────────────────────────┤
  │    #5  Reliability Fixes                                 │  M effort
  │        Deadlock, top-30 unwraps, CI pipeline             │  INDEPENDENT
  ├─────────────────────────────────────────────────────────┤
  │    #6  Sidebar Modularization                            │  M effort
  │        PR #30 — merge conflict prevention                │  READY
  ├─────────────────────────────────────────────────────────┤
  │    #7  Social Tab Hardening                              │  L effort
  │        All 9 platforms operational end-to-end             │  INDEPENDENT
  ├─────────────────────────────────────────────────────────┤
  │    #8  Email Integration                                 │  M effort
  │        Send/receive from dashboard                       │  INDEPENDENT
  └─────────────────────────────────────────────────────────┘
```

### Horizon 2: Platform Hardening + Knowledge (6 weeks → 3 months)

| # | Work Item | Effort | Dependency |
|---|-----------|--------|------------|
| 9 | Graph DB architecture + initial migration | XL | Arch decision needed |
| 10 | Topos knowledge graph: entity resolution, artifact library | XL | Needs graph DB |
| 11 | Org-level Topsi model (rename, configure, shared context, VIBE billing) | L | Agent refactor |
| 12 | Migrate away from per-user Orchas | M | Needs #11 |
| 13 | Legacy web bridge framework (data hygiene patterns) | L | Independent |
| 14 | Messaging integration framework (Matrix bridges) | XL | Independent |
| 15 | Rate limiting, input validation, auth session hardening | M | Independent |
| 16 | PostgreSQL migration (or graph DB replaces need) | L-XL | Depends on #9 |
| 17 | Frontend unit test infrastructure | L | Independent |
| 18 | Monolith file splits (api.ts 6.6K, org-profile 7K) | L | Independent |

### Horizon 3: VIBELAND + Ecosystem (3-6 months)

| # | Work Item | Effort | Dependency |
|---|-----------|--------|------------|
| 19 | VIBELAND MMO: multi-user presence, org-hosted spaces | XL | WebSocket/WebRTC |
| 20 | Org orchestration agents in VIBELAND (visual Topsi) | L | Needs #11, #19 |
| 21 | Gamification system (achievements, progression) | L | Needs #19 |
| 22 | Item/artifact trading in VIBELAND | M | Needs VIBE integration |
| 23 | Pythia persistence layer (production-ready) | M | Pythia project |
| 24 | Pythia <-> ORCHA integration (task routing via Pythia) | L | Needs #23 |
| 25 | APN addressing scheme research + RFC | L | Domain expertise |
| 26 | URI abstraction layer in ORCHA (prep for APN addressing) | M | Needs #25 |
| 27 | Client meeting hosting (Google Meet-like) | L | WebRTC |
| 28 | VIBE token chain-agnostic interface | M | Independent |

### Horizon 4: Sovereign Network (6-12 months)

| # | Work Item | Effort | Dependency |
|---|-----------|--------|------------|
| 29 | APN internal addressing (.apn or equivalent) | XL | Needs #25-26 |
| 30 | VIBE privacy chain migration | XL | Research + design |
| 31 | Pythia federated learning | XL | Pythia project |
| 32 | Pythia as default LLM option | L | Needs #31 |
| 33 | Spectrum Galactic project tracking | S | Awareness only |
| 34 | Vibertas OS integration (ORCHA as native app) | XL | OS project |

---

## Tech Debt — Critical Items Blocking Production

From the 24-finding tech debt review (`notes/2026-03-15--analysis--tech-debt-review.md`):

```
  SEVERITY    COUNT    TOP ITEMS
  ─────────   ─────    ──────────────────────────────────────────
  CRITICAL      4      • 100+ .unwrap() calls (server crash risk)
                       • SQLite not suitable for production scale
                       • Global mutex deadlock risk (worktree manager)
                       • No CI pipeline for core application

  HIGH          9      • No rate limiting on most endpoints
                       • No input validation framework
                       • Missing database indexes
                       • N+1 query patterns
                       • Monolithic files (api.ts: 6.6K, org-profile: 7K)
                       • Large custom hooks (5 hooks > 300 lines)
                       • Missing graceful shutdown
                       • No unit tests for frontend
                       • Missing structured logging

  MEDIUM       11      (auth hardening, code splitting, bundle size, etc.)
```

**Estimated effort for production-minimum (Phase 0-2):** 4-6 weeks
**Estimated effort for full remediation:** 3-4 months

---

## QA Sprint Summary (2026-03-14)

All items from the dogfood QA sprint have been resolved:

| Item | Type | Status |
|------|------|--------|
| Task activity logging (Bug #9) | Bug fix | DONE — DbUuid migration fixed BLOB/TEXT mismatch |
| Feedback tasks no dev agent (Bug #10) | Bug fix | DONE — Topsi system-tier assignment |
| Manual InReview QA trigger (Bug #11) | Bug fix | DONE — `spawn_watcher_reviews()` standalone entry |
| Intelligence Overview 0 sources (Bug #6) | Bug fix | DONE — `find_by_organization` query |
| `send_notification` node + in-app notifications | Feature | DONE — full stack: migration, model, routes, frontend bell |
| Workflow trigger hardening | Hardening | DONE — tracing spans + CancellationToken |
| Status dropdown in task detail | UX | DONE — Combobox in task detail header |
| Loading skeletons | UX | DONE — Skeleton components for My Tasks |
| E2E test failures (14 → 0) | Test | DONE — 7 PR review rounds |
| E2E demo refactor (headed mode) | Test | DONE — shared helpers, dedicated fixtures |
| Sidebar collapse + modularize | UX | DONE — PR #30, `useExpandable` store |

**Remaining deferred items:** See `BACKLOG--remaining-work.md` for 17 non-blocking items (P2-P3).

---

## Agent-Task MCP Integration Status

From `2026-03-12--plan--agent-task-mcp-wiring.md`:

```
  Phase 1: Foundation                              ████████████  COMPLETE
  ├── A. Task completion criteria fields           ✅ Done
  ├── B. MCP resources (orcha:// browsing)         ✅ Done
  └── C. Check dependencies tool                   ✅ Done

  Phase 2: Intelligence                            ████████████  COMPLETE
  ├── D. Unified cross-entity search               ✅ Done
  └── E. Scaffold project tool (4 templates)       ✅ Done

  Phase 3: Wiring                                  ████████░░░░  75% COMPLETE
  ├── F. Connect built-in MCP to executors         ✅ Done
  ├── G. Wire ACP agents to MCP                    ✅ Done
  ├── H. Agent Flow Orchestration Engine           ⬜ NOT STARTED (largest item)
  └── I. Autonomy level enforcement                ✅ Done

  MCP tool count: 26 tools + 6 resources
  Executor backends: 8 (Claude, Gemini, Qwen, Cursor, CodeX, Duck, AMP, OpenCode)
```

---

## Anti-Debt Principles

These principles prevent the patterns that caused current tech debt:

1. **Graph-first data modeling** — New entity relationships go into graph DB design, not more SQLite foreign keys
2. **Org-scoped, not user-scoped** — All new agent features scope to organization, not individual
3. **Chain-agnostic economics** — VIBE interfaces must abstract the underlying chain
4. **URI-agnostic routing** — Don't hardcode HTTP URLs; prepare for APN addressing
5. **Privacy-by-default** — Topsi learns maximally but never leaks between users/orgs
6. **Human gates are first-class** — Workflow engine treats human tasks as equal to agent tasks
7. **Legacy web is a bridge, not home** — External integrations are bridges with data hygiene, not core dependencies
8. **Many-to-many always** — Never design for 1-to-1 agent-user interaction; always assume team collaboration

---

## Open Questions Requiring Team Input

1. **Graph DB selection** — Neo4j (mature, Java), SurrealDB (Rust-native, multi-model), DGraph (Go, GraphQL-native)?
2. **APN addressing** — Need domain expert to research Aksak and propose conventions
3. **Privacy chain design** — APN facet vs Vibertas facet? Custom chain vs fork?
4. **VIBELAND MMO architecture** — Authoritative server vs P2P state sync?
5. **Meeting hosting** — Build on WebRTC directly or integrate Jitsi/similar?
6. **Federated learning path** — What data flows between Topsi -> Pythia without violating sovereignty?
7. **Merge conflict resolution** — Local commit `1fe2f2c` (9-stage pipeline) conflicts with upstream refactoring (#47) across 10 files

---

## Codebase Metrics (as of 2026-03-18)

| Metric | Count |
|--------|-------|
| Rust crates | 23 |
| Database models | 138 |
| API routes | 116 |
| DB migrations | 202 |
| Frontend pages | 60+ |
| React components | 311+ |
| Custom hooks | 60+ |
| Zustand stores | 15 |
| MCP servers | 4 (Task, Nora, Pulse, Topsi) |
| Executor backends | 8 |
| Social platforms | 9 |
| Discord bots | 2 (Serenity + Twilight) |
| Docker services | 6 |
# April 21–30 Sprint Plan
**Goal:** Enter May 1st with a stable, well-documented foundation ready for a strong Month 2 push.

---

## Team

```
Madhav   ████████████████████████████████  Full capacity
         Senior engineer, deepest codebase knowledge
         Agent Flow Engine + Pipeline completion + Integrations onboarding

Sam      ████████████████████████████████  Full capacity
         Video Studio branch (feature/video-studio)
         P1 Provider Router → P2 Pulse Intelligence → P3 Script Engine

Akshat   ████████████████████████████████  Full capacity
         PCG Component Library (separate repo — plan TBD)

Fraze    ████░░░░░░░░░░░░░░░░░░░░░░░░░░░  Review + Planning ONLY
         NO new code. Review current state, minor cleanup of own work,
         produce May planning artifacts.

Integrations Dev  ░░░░░░░░░░░░░░░░░████  Arriving ~Apr 28
         Day 1 task pre-prepared by Madhav
```

**Hard constraint:** `dashboard.powerclubglobal.com` stays live at all times. No breaking changes to prod without a deploy plan.

---

## Madhav — Priority Order

```
┌─────────────────────────────────────────────────────────────────┐
│ PRIORITY 1 — Dealflow Pipeline Gaps (F1–F13)                   │
│                                                                 │
│ These are ~8 hours of focused work that complete the pipeline. │
│ Do this first — it's the most concrete, bounded, high-value    │
│ work on the board. Unblocks the team from manual deal ops.     │
│                                                                 │
│ BATCH 1 (parallel):                                            │
│   F1   "Create Deal" button on processed intake items          │
│   F10  Operator context textarea at Intel stage                │
│   F4   Proposal edit UI (textarea + save)                      │
│   F6   Expedite toggle                                          │
│   F8   Org-specific BA operator routing                        │
│                                                                 │
│ BATCH 2 (sequential):                                          │
│   F11  Discovery call scheduling UI (2 calls)                  │
│   F12  Astra Pass 2 auto-trigger on Discovery → Proposal       │
│   F7   Intel completeness gate (blocks if context missing)     │
│   F13  Cash sets deal.amount from Investment section           │
│                                                                 │
│ BATCH 3 (parallel):                                            │
│   F2   Deliverables auto-create on proposal approve            │
│   F3   Won provisioning — person-level invite (NOT org)        │
│   F5   Internal deck review link (team only, not client)       │
│                                                                 │
│ BATCH 4:                                                       │
│   F9   E2E Playwright validation (full pipeline smoke test)    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ PRIORITY 2 — Agent Flow Orchestration Engine Phase 1 + 2       │
│                                                                 │
│ This is the autonomy blocker. Every stage transition currently │
│ requires a human to manually complete review tasks. Phase 1    │
│ adds auto_complete_review to StageConfig — once wired, agents  │
│ can progress deals without human gates.                        │
│                                                                 │
│ Phase 1:                                                       │
│   W1.1  Add auto_complete_review to StageConfig JSON          │
│   W1.2  Auto-complete review tasks on flow success            │
│          (in complete_flow() in agent_flow_executor.rs)        │
│   W1.3  Enable engine by default in dev builds                 │
│   W1.4  SSE endpoint for flow progress                         │
│          GET /api/events/agent-flows/:id/stream                │
│                                                                 │
│ Phase 2:                                                       │
│   W2.1  AgentResponse struct (status, artifacts, next_action) │
│   W2.2  submit_response tool (agents call when work complete) │
│   W2.3  Structured response handler + artifact parsing        │
│   W2.4  NeedsClarification transition                         │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ PRIORITY 3 — Integrations Dev Onboarding Kit                   │
│                                                                 │
│ Must be ready before they arrive (~Apr 28). Madhav prepares:  │
│                                                                 │
│   • integration_connections table + model                      │
│     (unified OAuth credential store — shared infra)           │
│   • Token Refresh Worker skeleton (15-min interval)            │
│   • Annotated file map: social_accounts.rs, social_posts.rs,  │
│     quickbooks.rs, email_accounts.rs, airtable.rs (reference) │
│   • Day 1 task brief: LinkedIn OAuth connect + first publish   │
│   • Offer letter + NDA on Desktop for Bodhi to send           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Sam — Priority Order

```
Full reference plan: /home/pythia/pcg-cc-mcp/planning/2026-04-21--plan--video-studio-sam.md

┌─────────────────────────────────────────────────────────────────┐
│ PRIORITY 1 — Provider Router (P1)                              │
│                                                                 │
│ BLOCKS EVERYTHING. All video generation routes through here.  │
│                                                                 │
│   crates/video_gen/src/providers/                             │
│     mod.rs      VideoGenerationProvider trait + get_provider()│
│     hedra.rs    Hedra Character-3 (PRIMARY)                   │
│     heygen.rs   existing → move here as fallback              │
│     router.rs   HEDRA_API_KEY → Hedra, else HeyGen            │
│                                                                 │
│   DB migration:                                                │
│     avatar_profiles: ADD hedra_character_id, preferred_provider│
│     video_jobs: ADD provider column                            │
│                                                                 │
│   Update video_gen.rs produce_job():                          │
│     replace heygen:: calls → get_provider()                   │
│                                                                 │
│   Env: HEDRA_API_KEY (Bodhi to provision)                      │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ PRIORITY 2 — Pulse Intelligence Engine (P2)                    │
│                                                                 │
│   DB migration: video_topics + video_pulse_items tables        │
│   DB models: VideoTopic, VideoPulseItem CRUD                  │
│                                                                 │
│   crates/video_gen/src/intelligence.rs                        │
│     gather_intelligence(org_id, topics[])                     │
│     → Perplexity + Tavily parallel                            │
│     → Claude Haiku: dedup + score + extract source_url        │
│     → upsert video_pulse_items                                │
│     → register in project_knowledge_sources (KG)             │
│                                                                 │
│   Routes:                                                      │
│     GET/POST  /video-studio/topics                            │
│     GET       /video-studio/pulse                             │
│     POST      /video-studio/pulse/gather                      │
│                                                                 │
│   Cron: wire gather_intelligence into daily 06:00 UTC         │
│                                                                 │
│   Seed topics: AI & Inference, Blockchain, Quantum,           │
│   Aviation & Cargo (TIACA), Sovereign Technology              │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ PRIORITY 3 — Script Engine (P3)                                │
│                                                                 │
│   DB migration: video_scripts table (all JSON columns)        │
│   DB model: VideoScript CRUD + update_section()               │
│                                                                 │
│   crates/video_gen/src/script_gen.rs                          │
│     generate_script(pulse_item | prompt, brand_profile)       │
│     → Claude Sonnet structured output                         │
│     → VideoScript { cold_open, headline, body, brand_tiein,   │
│         cta_signoff, lower_thirds, ticker_items[],            │
│         broll_cues[], overlay_cues[] }                        │
│                                                                 │
│   Routes:                                                      │
│     POST /video-studio/scripts/generate                       │
│     GET  /video-studio/scripts                                │
│     GET  /video-studio/scripts/:id                            │
│     PATCH /video-studio/scripts/:id                           │
│     POST /video-studio/scripts/:id/approve                    │
│                                                                 │
│   Frontend: add Pulse Feed tab + Script Editor tab to         │
│     video-studio.tsx (operator selects story → edits → approves)
└─────────────────────────────────────────────────────────────────┘

  NOTE: P4 (B-Roll), P5 (Overlay Engine), P6 (Assembly),
  P7 (Full Editor), P8 (Distribution), P9 (TIACA Automation)
  → These are May priorities. Build P1→P3 solidly in April.
```

---

## Fraze — Review + Planning Only

```
NO new features. NO new routes. NO migrations.
Minor bug fixes on his own previous work only.

┌─────────────────────────────────────────────────────────────────┐
│ TASK 1 — State of the Union Review                             │
│                                                                 │
│ Walk through every system he has touched and document:         │
│   • What's working correctly in prod                           │
│   • What has known bugs or edge cases                          │
│   • What was left half-done                                    │
│   • What will need attention in May                            │
│                                                                 │
│ Deliver: written review doc (markdown, in /planning/)          │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ TASK 2 — Minor Cleanup (his own work only)                     │
│                                                                 │
│ Fix any small, clear, isolated bugs surfaced in Task 1.        │
│ No new features. No architectural changes.                     │
│ If a fix requires touching more than 2 files → defer to May.  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ TASK 3 — May Planning Artifacts                                │
│                                                                 │
│ Produce detailed planning docs for his May workstream.         │
│ Fraze to confirm with Bodhi what his May focus area is,       │
│ then write a priority-ordered plan in the same format as       │
│ the video-studio-sam.md plan.                                  │
│                                                                 │
│ Deliver: /planning/2026-05-01--plan--fraze-may.md             │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ TASK 4 — BGF Deal Triage (data only, no code)                 │
│                                                                 │
│ Stephanie Bruno deal (cbcfec1d) is Won but nothing delivered. │
│ Fraze can triage without writing code:                         │
│                                                                 │
│   • Find Stephanie's email/phone from intake items or records │
│   • Run SQL updates directly on dev DB to fix:                │
│       deal.project_id NULL → link to project                  │
│   • Document exactly what deliverables should exist          │
│     (from proposal text) so May dev can create them cleanly   │
│                                                                 │
│ Deliver: /planning/bgf-delivery-triage.md                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Integrations Dev — Day 1 Kit (prepared by Madhav)

```
┌─────────────────────────────────────────────────────────────────┐
│ Pre-built by Madhav before arrival:                            │
│   integration_connections table + model                        │
│   Token Refresh Worker skeleton                                │
│   Annotated reference: airtable.rs (most complete example)    │
│                                                                 │
│ Day 1 Task:                                                    │
│   LinkedIn OAuth connect flow                                  │
│   GET  /integrations/connect/linkedin                         │
│   GET  /integrations/callback/linkedin                        │
│   POST /social/posts/:id/publish (LinkedIn)                   │
│                                                                 │
│ Reference docs to read first:                                  │
│   /planning/2026-04-21--plan--integrations-dev-role.md        │
│   airtable.rs (most complete integration in codebase)         │
│   social_accounts.rs, social_posts.rs (what already exists)   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Dependency Map

```
MADHAV                    SAM                     FRAZE

F1-F13 pipeline           P1 Provider Router      Review + triage
(no dependencies)         (needs HEDRA_API_KEY)   (no dependencies)
       │                         │
       ▼                         ▼
Agent Flow Engine P1      P2 Pulse Intelligence
(needs F1-F13 done        (needs P1 complete)
 for real test data)             │
       │                         ▼
       ▼                  P3 Script Engine
Agent Flow Engine P2      (needs P2 complete)
       │
       ▼
Integrations onboarding
kit (before Apr 28)
```

---

## What We Enter May With

```
  ✓ Dealflow pipeline COMPLETE (F1-F13)
    Deals can move from intake to Won with minimal operator clicks

  ✓ Agent Flow Engine Phase 1+2
    Stages progress autonomously — no manual review gate blocking

  ✓ Video Studio P1-P3 live on feature branch
    Hedra routing ready, Pulse collecting, Scripts generating
    Ready for B-Roll + Assembly work in May

  ✓ Integrations dev onboarded + LinkedIn publish live

  ✓ Fraze's state-of-union review → clean May backlog
    No hidden debt, known issues documented, May plan written

  ✓ Akshat's component library (plan TBD — separate repo)

  ✓ dashboard.powerclubglobal.com stable throughout
```

---

## Reference Docs

- Video Studio plan:    `/planning/2026-04-21--plan--video-studio-sam.md`
- Integrations dev:     `/planning/2026-04-21--plan--integrations-dev-role.md`
- Agent Flow Engine:    `/planning/2026-04-13--plan--agent-flow-orchestration-engine.md`
- Voice platform:       `/planning/2026-04-21--plan--voice-platform-completion.md`
- Pipeline gaps:        `memory/dealflow-pipeline-plan.md`
- BGF deal:             `memory/bgf-pipeline-plan.md`

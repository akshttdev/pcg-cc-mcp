# Dealflow Pipeline — Complete Intended Flow & Blocker Status
**Generated:** 2026-03-18 | **Based on:** sloperation316 integration (PR #45) + live code audit
**Overall Completion:** ~85% (up from 72% at last review)

---

## Legend

```
✅  Implemented & wired end-to-end
🔧  Partial — exists but incomplete or stubbed
❌  Not started
🚫  P0 Blocker — architectural gap
```

---

## Complete Pipeline Flow with Status

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                                                                 │
│   INTAKE                                                                        │
│   ──────                                                                        │
│   Phone / Email / Manual                                                        │
│        │                                                                        │
│        ▼                                                                        │
│   Person + Company + CrmContact created                          ✅             │
│        │                                                                        │
│   ┌────┴──────────────────────────────────────────────────────┐                 │
│   │  [Create Deal] button on processed intake item            │  ✅  F1        │
│   │   → creates crm_deal in "intel" stage                     │                │
│   └────────────────────────────────────────────────────────────┘                │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 0: INTEL                                                    ~5% weight   │
│                                                                                 │
│  ┌─ STEP 1: Operator adds context ─────────────────────────────────────────┐   │
│  │                                                                         │   │
│  │  "Tell us about this lead" textarea UI            ✅  F10               │   │
│  │   • Who is this person?                                                 │   │
│  │   • What does their company do?                                         │   │
│  │   • What's the opportunity / budget signals?                            │   │
│  │   Saves to: deal.description                                            │   │
│  │   Required before advancing (gate enforced)                             │   │
│  │                                                                         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                        │                                        │
│                                        ▼                                        │
│  ┌─ STEP 2: Scout auto-research (on deal creation) ────────────────────────┐   │
│  │                                                                         │   │
│  │  READS: operator context + person.name/email + company.name  ✅         │   │
│  │  WRITES: persons.intelligence_summary                                   │   │
│  │          companies.intelligence_summary                                 │   │
│  │          knowledge_sources entries                                      │   │
│  │                                                                         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                        │                                        │
│                                        ▼                                        │
│  ┌─ GATE ──────────────────────────────────────────────────────────────────┐   │
│  │  ◆ deal.description not empty            ✅  F7                         │   │
│  │  ◆ person.intelligence_status = 'done'   ✅  F7                         │   │
│  │  ◆ company.intelligence_status = 'done'  ✅  F7                         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 1: BUSINESS ANALYSIS                                       ~20% weight   │
│                                                                                 │
│  AUTO: Astra generates Phase 1 business report                   ✅             │
│   READS: person_intel + company_intel + operator context notes                  │
│   WRITES: business_reports                                                      │
│     • executive_summary                                                         │
│     • pain_points                                                               │
│     • opportunities                                                             │
│     • recommended_services                                                      │
│     • competitive_landscape                                                     │
│                                                                                 │
│  HUMAN: Org-specific operator reviews report                     ✅  F8         │
│   Sirak → Sirak operator | PCG → Bodhi                                          │
│   Review task auto-assigned to org operator                                     │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 2: DISCOVERY                                               ~40% weight   │
│                                                                                 │
│  HUMAN: Account Manager conducts discovery call                                 │
│                                                                                 │
│  ┌─ Call Scheduling UI ────────────────────────────────────────────────────┐   │
│  │                                                                         │   │
│  │  Discovery Call          [date] [method] [status toggle]   🔧  F11     │   │
│  │  Presentation Call       [date] [method] [status toggle]   🔧  F11     │   │
│  │                                                                         │   │
│  │  CURRENT STATE:                                                         │   │
│  │  • Display is wired — shows dates from custom_fields ✅                 │   │
│  │  • NO input UI — dates cannot be set from the UI ❌                    │   │
│  │  • No date picker, no method selector, no status toggle                │   │
│  │  • custom_fields JSON must be edited via API or raw DB                 │   │
│  │                                                                         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  After call: transcript linked to deal (deal_transcripts)        ✅             │
│  Operator can add additional notes via deal.description          ✅             │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                     ┌──────────────────┘
                     │  TRANSITION: Discovery → Proposal
                     ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ★ SECOND RESEARCH CYCLE                                                        │
│                                                                                 │
│  Astra Deep Research Pass 2 — auto-triggers on advance to Proposal  ✅  F12    │
│   READS: Phase 1 report + discovery transcript + all operator notes             │
│           + person intel + company intel + call_intake_items                    │
│   WRITES: Updates business_reports (report.research_depth += 1)                │
│   THEN: Automatically chains to Cash proposal generation           ✅           │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 3: PROPOSAL                                                ~55% weight   │
│                                                                                 │
│  AUTO: Cash generates proposal (chained from Astra Pass 2)       ✅             │
│   READS: Enhanced business report (Pass 2) + person/company intel               │
│           + discovery transcript + operator context                             │
│   WRITES:                                                                       │
│     • deal.proposal_text (full markdown)                          ✅             │
│     • deal.proposal_status = 'draft'                              ✅             │
│     • deal.amount ← parsed from Investment section               ✅  F13        │
│     • ```json deliverables block at end of proposal              ✅             │
│                                                                                 │
│  HUMAN: AM reviews in ProposalTab                                               │
│   • Edit proposal text (textarea + Save)                         ✅  F4         │
│   • Verify pricing                                                              │
│   • Click "Approve Proposal"                                                    │
│     → Deliverables auto-created from JSON block                  ✅  F2         │
│                                                                                 │
│  UI: Presentation call reminder visible                          🔧             │
│   • Shows scheduled date if set — but date cannot be set yet (F11 gap)         │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 4: POLISH                                                  ~70% weight   │
│                                                                                 │
│  AUTO: Lux generates deck on stage entry                         ✅             │
│   READS: deal.proposal_text + org brand profile                                 │
│   WRITES: deck_script in custom_fields + deck_url                               │
│                                                                                 │
│  HUMAN: AM reviews deck in DeckTab                                              │
│   • [Share for Review] → copies internal review link to clipboard ✅  F5        │
│     (team-only — NOT client-facing, enforced by design)                         │
│   • Team provides feedback via review system                     ✅             │
│   • Mark review task done when deck is presentation-ready                       │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 5: PRESENT                                                 ~85% weight   │
│                                                                                 │
│  HUMAN: AM presents deck to client on LIVE CALL                                 │
│   • Uses scheduled Presentation Call (from Discovery stage)                     │
│   • Deck via screen share / in-person                                           │
│   • NO link ever sent to client directly                                        │
│                                                                                 │
│  AFTER PRESENTATION:                                                            │
│   • Send invoice if client is ready                                             │
│   • Log presentation outcome in activity                                        │
│                                                                                 │
│  NOTE: No system blockers here — pure human stage                               │
│  DEPENDENCY: Presentation date must be set (blocked by F11 gap)  🔧            │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 6: FOLLOW UP                                               ~90% weight   │
│                                                                                 │
│  HUMAN: AM follows up on proposal / invoice                                     │
│   • Won → Mark Won                                                              │
│   • Lost → Move to Lost stage                                                   │
│   • Still in discussion → stays here                                            │
│                                                                                 │
│  No system blockers — pure human stage                           ✅             │
│                                                                                 │
└───────────────────────────────────────┬─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  STAGE 7: WON                                                     100%          │
│                                                                                 │
│  AUTO: mark_deal_won handler                                                    │
│   ✅  Create / find CLIENT record                                               │
│   ✅  Create PROJECT ("{company} Project")                                      │
│   ✅  Link deliverables to project                                              │
│   ✅  Create TASKS from deliverables                                            │
│   ✅  Log VIBE revenue transaction                                              │
│   🔧  Send person-level platform invitation   ← STUB ONLY (F3)                 │
│         Currently: logs "invitation pending — wire invite system later"         │
│         Missing: actual invite token generation + email send                    │
│   ✅  Auto-create Delivery pipeline deal                                        │
│   ✅  Log deal_won activity                                                     │
│                                                                                 │
│  INVITATION MODEL (intended, not implemented):                                  │
│  ┌───────────────────────────────────────────────────────────────────────┐     │
│  │  Person-level invite — NOT org creation                               │     │
│  │   a. Generate invite token for person                                 │     │
│  │   b. Pre-seed with: full_name, email, phone, company, job_title      │     │
│  │   c. Grants: VIBELAND + scheduled calls + their person profile        │     │
│  │   d. Does NOT grant: Dashboard, CRM, other client data               │     │
│  │   e. Org creation = admin-only (hardware + APN join)                 │     │
│  └───────────────────────────────────────────────────────────────────────┘     │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Agent Orchestration Map

```
  STAGE ENTRY              AGENT         TRIGGER                    STATUS
  ═══════════              ═════         ═══════                    ══════

  → Intel (on deal         SCOUT         Auto on deal creation      ✅
    creation)              Pass 1        reads operator context
                                         + person + company names

  → Biz Analysis           ASTRA         Auto on stage entry        ✅
    (on entry)             Pass 1        reads Scout output
                                         + operator context notes

  → Discovery              HUMAN ONLY    AM conducts call           —
                                         schedules two calls         🔧 (no input UI)

  Discovery → Proposal     ASTRA         Auto on stage transition   ✅
  (transition hook)        Pass 2        reads: Pass 1 report
                                         + transcript + operator
                                         notes + intel + intake
                                         items

  → Proposal               CASH          Chained from Astra Pass 2  ✅
  (after Pass 2)                         reads: enhanced report
                                         + all accumulated context
                                         sets: proposal_text,
                                               deal.amount,
                                               deliverables JSON

  → Polish                 LUX           Auto on stage entry        ✅
  (on entry)                             reads: proposal_text
                                         + org brand profile
                                         sets: deck_script, deck_url

  → Won                    SYSTEM        manual "Mark Won" click     🔧
  (mark_deal_won)                        provisions: client, project,
                                         deliverables → tasks,
                                         VIBE txn
                                         person invite: LOG ONLY ❌
```

---

## Feature Item Status

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║  #    ITEM                              STATUS          NOTES                  ║
╠══════════════════════════════════════════════════════════════════════════════════╣
║                                                                                 ║
║  INTAKE                                                                         ║
║  F1   "Create Deal" button              ✅ DONE         call-intake.tsx:337    ║
║       on processed intake items                                                 ║
║                                                                                 ║
║  INTEL STAGE                                                                    ║
║  F10  Operator context input UI         ✅ DONE         OverviewTab.tsx:150    ║
║       textarea → deal.description                       save mutation wired    ║
║       "Required before advancing" hint                  amber border if empty  ║
║                                                                                 ║
║  F7   Intel completeness gate           ✅ DONE         crm_deals.rs:1226     ║
║       blocks advance if:                                                        ║
║       - no operator context                                                     ║
║       - person intel not done                                                   ║
║       - company intel not done                                                  ║
║                                                                                 ║
║  DISCOVERY STAGE                                                                ║
║  F11  Call scheduling UI                🔧 PARTIAL      OverviewTab.tsx:203   ║
║       Discovery + Presentation call     DISPLAY ONLY    dates shown from       ║
║       date / method / status                            custom_fields JSON     ║
║                                         BLOCKER:        no date picker,        ║
║                                                         no method selector,   ║
║                                                         no status toggle       ║
║                                                                                 ║
║  DISCOVERY → PROPOSAL                                                           ║
║  F12  Astra Deep Research Pass 2        ✅ DONE         crm_deals.rs:1645     ║
║       auto on Discovery→Proposal                        chains to Cash after  ║
║       reads all accumulated context                                             ║
║                                                                                 ║
║  PROPOSAL STAGE                                                                 ║
║  F4   Proposal edit UI                  ✅ DONE         ProposalTab.tsx:25    ║
║       textarea + save + cancel                                                  ║
║                                                                                 ║
║  F13  Cash sets deal.amount             ✅ DONE         crm_deals.rs:1838     ║
║       parses ```json estimated_value                                            ║
║       fields, SUM → deal.amount                                                 ║
║                                                                                 ║
║  F2   Deliverables on approve           ✅ DONE         crm_deals.rs:1896     ║
║       parse JSON block → INSERT                                                 ║
║       INTO deliverables                                                         ║
║                                                                                 ║
║  POLISH STAGE                                                                   ║
║  F5   Internal deck review link         ✅ DONE         DeckTab.tsx:114       ║
║       "Share for Review" copies link                    team-only,             ║
║       to clipboard                                      NOT client-facing      ║
║                                                                                 ║
║  OTHER                                                                          ║
║  F6   Expedite toggle                   ✅ DONE         OverviewTab.tsx:138   ║
║                                                                                 ║
║  F8   Org-specific BA operator          ✅ DONE         crm_deals.rs:1022     ║
║       auto-assigns review task to                                               ║
║       org-specific operator                                                     ║
║                                                                                 ║
║  WON STAGE                                                                      ║
║  F3   Won provisioning                  🔧 PARTIAL      crm_deals.rs:2077     ║
║       ✅ Client record                                                          ║
║       ✅ Project creation                                                       ║
║       ✅ Link deliverables → project                                            ║
║       ✅ Tasks from deliverables                                                ║
║       ✅ VIBE revenue transaction                                               ║
║       ❌ Person invite — LOG ONLY       BLOCKER         crm_deals.rs:2225     ║
║          "wire invite system later"                     no token, no email     ║
║                                                                                 ║
║  E2E                                                                            ║
║  F9   Live E2E validation               🔧 PARTIAL      28/31 demos passing   ║
║       pipeline-specific E2E                             1 pre-existing flaky  ║
║                                                         no F11 coverage        ║
║                                                                                 ║
╠══════════════════════════════════════════════════════════════════════════════════╣
║  TOTALS: 10 done ✅  |  3 partial 🔧  |  0 not started ❌                      ║
╚══════════════════════════════════════════════════════════════════════════════════╝
```

---

## Active Blockers

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  BLOCKER 1 — F11: Call Scheduling Has No Input UI           Priority: HIGH      │
│                                                                                 │
│  What's broken:                                                                 │
│    OverviewTab.tsx renders discovery_call_date and presentation_call_date       │
│    from custom_fields, but there are no controls to SET them. Operators         │
│    cannot schedule calls from the UI at all.                                   │
│                                                                                 │
│  Impact:                                                                        │
│    • Present stage (Stage 5) has no scheduled date to reference               │
│    • Proposal tab "presentation call reminder" shows nothing useful            │
│    • Discovery → Proposal advance has no gate for call completion              │
│                                                                                 │
│  What's needed:                                                                 │
│    • Date picker input for each call                                            │
│    • Method selector (Phone/Zoom/Discord/In-person)                            │
│    • Done/Pending status toggle                                                 │
│    • Save mutation to PATCH deal custom_fields                                  │
│    • ~45 min effort                                                             │
│                                                                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│  BLOCKER 2 — F3: Person Invitation Not Implemented          Priority: HIGH      │
│                                                                                 │
│  What's broken:                                                                 │
│    mark_deal_won() logs the intended invite but does nothing:                  │
│    "Won deal X: person invitation pending — wire invite system later"          │
│                                                                                 │
│  Impact:                                                                        │
│    • Won stage cannot complete the client onboarding flow                      │
│    • Clients never receive VIBELAND / call access                              │
│                                                                                 │
│  What's needed:                                                                 │
│    • Reuse existing invite token system (organizations.invite_token)           │
│    • Target: person, not org                                                   │
│    • Pre-seed with person.full_name, email, phone, company_name               │
│    • Invite grants: VIBELAND + scheduled calls + person profile                │
│    • Does NOT grant: dashboard, CRM, other client data                         │
│    • Send via email (Resend/Twilio SMS as fallback)                            │
│    • ~45 min effort                                                             │
│                                                                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│  BLOCKER 3 — Agent Flow Orchestration Engine               Priority: P0        │
│                                                                                 │
│  What's broken:                                                                 │
│    Agent flows (Scout/Astra/Cash/Lux) are wired as direct async spawns        │
│    on stage advance. There is no persistent orchestration layer that:          │
│    • retries failed agent runs                                                  │
│    • enforces gates between agent steps                                         │
│    • handles timeouts / partial completion                                      │
│    • provides observable execution history per deal                            │
│                                                                                 │
│  Impact:                                                                        │
│    • If Scout/Astra/Cash fail silently, deal is stuck with no recovery         │
│    • No UI feedback on agent progress within a stage                           │
│    • Cannot replay or re-trigger individual agent steps                        │
│                                                                                 │
│  What's needed:                                                                 │
│    • Background worker polling for pending agent tasks                         │
│    • Per-deal execution log (stage → agent → status → output)                  │
│    • Retry / re-trigger endpoints                                               │
│    • This is the largest remaining architecture gap                            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Deferred (Non-Blocking)

```
  ITEM                                   SOURCE              STATUS
  ────                                   ──────              ──────
  ~20+ :any casts in new CRM TS code    PR #45 QA           Not started
  Org-level authz on CRM routes          PR #45 QA           Deferred (auth exists)
  Company::find_by_id hex() workaround   DbUuid Phase C dep  Blocked on Phase C
  DbUuid migration — Phase A-D           Backlog P2          Planning complete
  Sovereign stack graceful shutdown      PR #45 QA           Acceptable for now
  Resolve_volume_path deduplication      PR #45 QA           Not started
  http_request node guardrails           Backlog P2          Not started
```

---

## Definition of Done — Checklist

```
  #   CHECK                                                        STATUS
  ─── ─────────────────────────────────────────────────────────    ──────

  1.  Operator adds context notes at Intel stage                   ✅
  2.  Scout researches with operator context → refined intel       ✅
  3.  Intel gate blocks if context / research incomplete           ✅
  4.  BA report generated by Astra, reviewed by org operator      ✅
  5.  Discovery stage shows call scheduling UI (2 calls)          🔧  input missing
  6.  Second Astra pass runs on Discovery → Proposal transition   ✅
  7.  Cash generates proposal from enhanced research              ✅
  8.  Cash SETS deal.amount from pricing analysis                 ✅
  9.  AM can edit proposal text before approving                  ✅
  10. Approve creates deliverable records                         ✅
  11. Lux generates deck, team reviews internally                 ✅
  12. Presentation call conducted live (no link to client)        ✅  (by design)
  13. Invoice sent after presentation                             ✅  (manual)
  14. Won provisions: project, client, deliverables → tasks,
      VIBE txn, person invite (NOT org)                           🔧  invite stub
  15. Full E2E test passes with real LLM output                   🔧  28/31 passing

  ─── ─────────────────────────────────────────────────────────    ──────
  COMPLETE: 11/15   PARTIAL: 3/15   BLOCKED: 1/15 (orchestration)
```

---

## Recommended Next Steps

```
  PRIORITY  ITEM                  EFFORT   DEPENDENCY
  ────────  ────                  ──────   ──────────
  1         F11: Call scheduling  45 min   None — self-contained UI change
            input UI (date
            picker + method +
            status toggle)

  2         F3: Person invite     45 min   Existing invite token system
            wire-up in                     (organizations.invite_token)
            mark_deal_won

  3         Agent orchestration   Large    Architectural — design first
            engine                         before implementing

  4         F9: E2E for F11+F3    30 min   After 1 + 2 above
```

---

## Status Updates (Post-Sloperation)

This reference document captures the **original intended flow** from the sloperation317 branch. The pipeline was subsequently rebuilt into a 9-stage architecture. Below tracks resolution status.

### Resolved Since This Document Was Written

| Item | Resolution | Where |
|------|-----------|-------|
| **Blocker 3: Agent Orchestration Engine** | Fully built — background worker, cancel/Run Now, retry, agent history UI | `crates/server/src/agent_flow_executor.rs` |
| **F11: Call scheduling input UI** | Full date/method/status editing with save mutation | `frontend/src/components/crm/deal-detail/tabs/OverviewCallSchedulingSection.tsx` |
| **Agent auto-advance chain** | Intel(Scout)→BA(Astra)→Proposal(Cash)→Polish(Lux) verified working with SIMULATE_LLM | `crates/server/src/agent_flow_executor.rs:try_auto_advance_deal()` |
| **Silent failure audit** | 39 `let _ =` patterns converted to proper error handling | `crates/server/src/routes/crm_deal_automations.rs` (tier1 commit cf9f8f95f) |

### Still Outstanding (Audited 2026-03-24)

| Item | Status | Notes |
|------|--------|-------|
| **Discovery stage** | ❌ Not ported | Stage removed in 9-stage rebuild. No analogue exists. See `docs/PIPELINE.md` gap analysis. |
| **Astra Pass 2 (F12)** | ❌ Not ported | Depends on Discovery stage. AA-4 test marked `test.fixme`. |
| **Astra→Cash chaining** | ❌ Not ported | Cash triggers directly on Proposal entry, no Pass 2 enrichment. |
| **Present stage** | ❌ Replaced by Invoice | Different intent: Present = deck on live call, Invoice = payment confirmation. |
| **F8: Org-specific review routing** | ⚠️ Partial | Tasks created but not assigned to org operator. |
| **Won dual-path** | ⚠️ Bug | Stage transition only creates delivery deal; mark_deal_won does full provisioning. |
| **F3: Person invite** | ⚠️ Stub | Log-only in mark_deal_won. |

### Files That Built on This Plan

The following files were created after this reference and implement or extend its intents:

| File | Relationship |
|------|-------------|
| `docs/PIPELINE.md` | Canonical architecture doc for the rebuilt pipeline. Contains gap analysis mapping original intents. |
| `crates/server/src/stage_transition.rs` | Config-driven StageTransitionProcessor replacing hardcoded stage logic |
| `crates/server/src/agent_flow_executor.rs` | Resolves Blocker 3 (Agent Orchestration Engine) |
| `crates/db/migrations/20260414000001_seed_stage_configs.sql` | stage_config JSON seeds for all pipeline stages |
| `e2e/pipeline/agent-automations.spec.ts` | E2E tests for agent automation chain (AA-1 through AA-7) |
| `planning/BACKLOG--remaining-work.md` | Pipeline specs section tracks remaining test coverage |
| `frontend/src/components/crm/deal-detail/tabs/OverviewCallSchedulingSection.tsx` | Resolves F11 (call scheduling input UI) |
| `crates/server/src/routes/crm_deal_automations.rs` | Contains mark_deal_won, approve_proposal, generate_proposal — all F-items |

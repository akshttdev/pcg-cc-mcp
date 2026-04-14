# Backlog — Remaining Work

**Last updated:** 2026-04-13 (demo-sprint merged; PR #62 + PR #63 regression fixes applied)
**Context:** Consolidated from all completed planning docs + 27 research reports + 45-item research-derived backlog. **Prioritized by ROI = (revenue impact × probability) / effort**, not legacy ordering.
**Phase 0 Sprint Plan:** See [`2026-03-19--analysis--phase0-sprint-candidates.md`](2026-03-19--analysis--phase0-sprint-candidates.md) for full scoring and sprint schedule.

---

## P0 — Persons→Contacts Unification (HIGH PRIORITY)

**Audit**: [`2026-03-27--audit--persons-contacts-unification.md`](2026-03-27--audit--persons-contacts-unification.md)
**Decision**: Keep `crm_contacts` as canonical. Merge `persons` intelligence fields into contacts. Retire `persons` table.
**Why high priority**: Scout agent research is invisible in the Intel tab because intelligence lives on `persons` but the CRM pipeline operates through `crm_contacts`. Every new deal created via the CRM UI has no person record, so agent research has nowhere to land.

### PC-1: Add intelligence fields to crm_contacts — ✅ DONE (Phase 4, commit 46c958a13)
- Migration `20260418000000_contacts_intelligence.sql` adds all fields + backfill from persons
- Experience audit: 1 BLOCKER (review task links), 2 pain points, 4 friction items

### PC-1a: Review task links — ✅ FIXED
- Guarded empty `project_id` in my-tasks.tsx, global-tasks.tsx (falls back to `/my-tasks`)

### PC-1b: Strip raw JSON from intelligence summaries — ✅ FIXED
- Root cause fix in `agent_flow_executor.rs` — simulated Scout no longer appends raw context

### PC-1c: Surface intelligence on contact detail panel — ✅ FIXED
- Added Intelligence section to ContactDetailModal with status, confidence, summary, agent

### PC-2: Add research endpoint for contacts — ✅ DONE (Phase 3 W2, commit 8bf10a2cb + refactor 103c250cd)
- `POST /api/crm/contacts/:id/research` — contact-first, no person bridge
- `run_contact_research_direct()` writes to crm_contacts directly
- Company research auto-creates company + uses Scout agent (not Astra)

### PC-3: Move research passes to contacts — ✅ DONE (contacts-unification PR #63, migration 20260418000001)
- `contact_research_passes` table created; data migrated from `person_research_passes`; old table dropped

### PC-4: Move social profiles to contacts — ✅ DONE (contacts-unification PR #63, migration 20260418000001)
- `contact_social_profiles` table created; data migrated from `person_social_profiles`; old table dropped

### PC-5: Update intake pipeline to create contacts — ✅ DONE (Phase 3 W1, commit 800de2fe0)
- Intake pipeline creates `crm_contacts` directly
- Intelligence written to contact record

### PC-6: Retire persons table — ✅ DONE (contacts-unification PR #63)
- `/api/persons` endpoints removed; intelligence routes target contacts
- `persons` table retired; person models replaced with contact equivalents
- user_profiles table created; person_type/financial_role migrated to custom_fields
- **Pre-merge checklist**: backup DB before 20260416000000 (irreversible BLOB→TEXT); audit `persons.crm_contact_id IS NULL` rows before 20260418000001 drops tables

---

## Sloperation317 Feature Parity — Acceptance Specs

**Purpose:** Track remaining work to reach full feature parity with the sloperation317 dealflow plan.
Specs are written as user stories with Gherkin acceptance criteria so feature equivalence can be verified
regardless of architectural changes. Each spec is tagged: PASSING, PARTIAL, or FAILING.

**Source plans:**
- `planning/2026-03-18--plan--sloperation317-integration.md`
- `planning/2026-03-18--reference--pipeline-status.md`
- `planning/2026-03-21--plan--pipeline-ops-sprint.md`
- `planning/reviews/2026-03-23--review--functionality-audit-pipeline-ops.md`

### Deal Lifecycle

#### DL-1: Create and track a deal through the pipeline — PASSING
```gherkin
Feature: Deal creation and pipeline tracking
  As a sales operator
  I want to create deals and track them through pipeline stages
  So that I can manage my sales funnel

  Scenario: Create a deal via the pipeline board
    Given I am on the CRM Pipeline page for an organization
    When I click "Add Deal" and fill in name, amount, and description
    Then the deal appears in the first stage column of the kanban board
    And the deal card shows the contact name, amount, and stage color

  Scenario: View deal details
    Given a deal exists in the pipeline
    When I click the deal card
    Then a detail panel opens with tabs: Overview, Intel, Transcripts, Proposal, Deck
    And I can expand the panel to full dialog mode
```

#### DL-2: Move deals between stages — PASSING
```gherkin
Feature: Deal stage transitions
  As a sales operator
  I want to move deals between pipeline stages
  So that deals progress through the sales funnel

  Scenario: Move via context menu
    Given a deal exists in the "Lead" stage
    When I right-click the deal card and select "Move to..." → "Intel"
    Then the deal moves to the Intel column
    And a toast confirms the transition

  Scenario: Move via deal detail stage bar
    Given I have the deal detail panel open
    When I click a stage in the progress bar
    Then the deal moves to that stage
    And the kanban board updates

  Scenario: Advance via API
    Given a deal exists in a non-terminal stage
    When I call POST /crm/deals/:id/advance
    Then the deal moves to the next stage by position
    And stage entry actions fire (agent triggers, review tasks)
```

#### DL-3: Won deal automation — PARTIAL
```gherkin
Feature: Won deal creates delivery pipeline entry
  As a sales operator
  I want won deals to automatically create delivery work
  So that client onboarding begins immediately

  Scenario: Deal reaches Won stage
    Given a deal exists in the Negotiation stage
    When the deal is moved to Won
    Then a delivery pipeline deal is auto-created         # PASSING
    And the delivery deal has the same contact and amount  # PASSING
    And a Client record is created or found by company     # FAILING — not implemented
    And a Project is created from the deal                 # FAILING — not implemented
    And tasks are created from proposal deliverables        # FAILING — not implemented
    And a VIBE transaction is recorded for the deal value   # FAILING — not implemented

  Scenario: Deduplication
    Given a contact already has a delivery deal
    When another deal for the same contact reaches Won
    Then no duplicate delivery deal is created              # PASSING
```

#### DL-4: Lost deal tracking — PASSING
```gherkin
Feature: Lost deal tracking
  As a sales operator
  I want to mark deals as lost with a reason
  So that I can analyze why deals fail

  Scenario: Move deal to Lost
    Given a deal exists in any active stage
    When I move the deal to Lost
    Then the deal shows in the Lost column
    And lost_at timestamp is recorded
```

### Agent Automations

#### AA-1: Scout research on Intel stage entry — PASSING
```gherkin
Feature: Scout agent triggers on Intel stage
  As a sales operator
  I want Scout to automatically research a prospect when a deal enters Intel
  So that I have intelligence before engaging

  Scenario: Auto-trigger Scout
    Given a deal with a linked contact
    When the deal enters the Intel stage
    Then an agent flow is created for Scout with flow_type "research"
    And a 30-second cancel window is active
    And the deal card shows "Scout running..." badge
    And a review task is created: "Review Phase I intelligence"

  Scenario: Cancel Scout
    Given Scout is running on a deal within the cancel window
    When I click "Cancel Agent"
    Then the agent flow is cancelled
    And the badge disappears

  Scenario: Run Now bypasses cancel window
    Given Scout is running on a deal within the cancel window
    When I click "Run Now"
    Then the cancel deadline is cleared
    And the agent executes on the next poll cycle (within 15s)
```

#### AA-2: Astra business analysis — PASSING
```gherkin
Feature: Astra agent triggers on Business Analysis stage
  As a sales operator
  I want Astra to analyze the business when a deal enters Business Analysis
  So that I understand pain points and opportunities

  Scenario: Auto-trigger Astra
    When the deal enters the Business Analysis stage
    Then an agent flow is created for Astra with flow_type "business_analysis"
    And a review task is created: "Review business report (Astra)"
```

#### AA-3: Cash proposal generation — PASSING
```gherkin
Feature: Cash agent generates proposals on Proposal stage
  As a sales operator
  I want Cash to generate a proposal when a deal enters the Proposal stage
  So that I have a professional proposal to present

  Scenario: Auto-trigger Cash
    When the deal enters the Proposal stage
    Then an agent flow is created for Cash with flow_type "proposal"
    And a review task is created: "Review and approve proposal"

  Scenario: Manual proposal generation
    Given I am on the deal detail Proposal tab
    When I click "Generate Proposal"
    Then Cash generates a proposal
    And the proposal text appears in the Proposal tab

  Scenario: Approve proposal
    Given a proposal exists with status "draft"
    When I click "Approve Proposal"
    Then proposal_status changes to "approved"
```

#### AA-4: Astra Pass 2 deep research chain — FAILING
```gherkin
Feature: Astra Pass 2 chains into Cash proposal generation
  As a sales operator
  I want enhanced research before proposal generation
  So that proposals are informed by deep business analysis

  Scenario: Proposal stage triggers Astra Pass 2 then Cash
    When the deal enters the Proposal stage AND has no proposal_text
    Then Astra Pass 2 runs first (enhanced business analysis)    # FAILING — direct Cash trigger only
    And after Astra completes, Cash is chained automatically      # FAILING — no chaining
    And Cash uses Astra's enhanced report to write the proposal   # FAILING — Cash runs independently
```

#### AA-5: Lux deck generation — PASSING
```gherkin
Feature: Lux agent generates decks on Polish stage
  As a sales operator
  I want Lux to generate a presentation deck when a deal enters Polish
  So that I have a professional deck for client meetings

  Scenario: Auto-trigger Lux
    Given a deal has an approved proposal
    When the deal enters the Polish stage
    Then an agent flow is created for Lux with flow_type "deck"
    And a review task is created: "Review and approve deck"
    And proposal_text is required before entry (soft gate)
```

#### AA-6: Agent auto-advance chain — PASSING
```gherkin
Feature: Agents chain through pipeline via auto-advance
  As a sales operator
  I want agents to automatically advance deals through agent-owned stages
  So that the pipeline progresses without manual intervention

  Scenario: Agent completes and auto-advances
    Given a deal is in the Intel stage with Scout executing
    When Scout completes successfully
    Then the deal auto-advances to Business Analysis
    And Astra is automatically triggered on the new stage
    And the chain continues: BA → Proposal → Polish

  Scenario: Auto-advance stops at human-owned stages
    Given a deal is in Polish with Lux executing
    When Lux completes successfully
    Then the deal auto-advances to Invoice
    And no agent is triggered (Invoice is human-owned)
    And the deal waits for manual progression
```

#### AA-7: Retrigger failed/cancelled agent — PASSING
```gherkin
Feature: Retrigger agent for failed or cancelled flows
  As a sales operator
  I want to retry an agent that failed or was cancelled
  So that I can recover from errors without moving the deal

  Scenario: Retrigger from Agent History tab
    Given a deal has a failed or cancelled agent flow
    When I click "Retry Agent" in the Agent History tab
    Then a new agent flow is created for the same agent
    And the new flow has a fresh cancel window

  Scenario: Retrigger via API
    When I POST to /crm/deals/:id/retrigger-agent
    Then a new flow is scheduled with the stage's default agent
```

### Agent Observability

#### DD-5: Agent History tab — PASSING
```gherkin
Feature: View agent execution history on deals
  As a sales operator
  I want to see what agents have done on a deal
  So that I can review their work and track progress

  Scenario: View completed agent flow
    Given an agent has completed on a deal
    When I open the deal detail and click Agent History tab
    Then I see the agent name, status badge (Completed/Failed/etc.)
    And the flow duration and time since completion
    And I can expand to see individual events (phase_started, artifact_created)

  Scenario: Empty state
    Given no agents have run on a deal
    When I click the Agent History tab
    Then I see "No agent activity yet" with an explanation
```

### Deal Card & Board UX

#### DL-5: Deal card agent status badges — PASSING
```gherkin
Feature: Deal cards show agent execution status
  As a sales operator
  I want to see agent status on deal cards at a glance
  So that I know which deals have active or failed agents

  Scenario: Agent running badge
    Given an agent is executing on a deal
    Then the deal card shows a pulsing "Scout running..." badge

  Scenario: Agent pending badge
    Given an agent is scheduled but within the cancel window
    Then the deal card shows "Scout pending" badge

  Scenario: Agent failed badge
    Given an agent flow has failed
    Then the deal card shows "Agent failed" badge
```

#### DL-6: Toast feedback on stage transitions — PASSING
```gherkin
Feature: Toast notifications confirm stage transitions
  As a sales operator
  I want clear feedback when deals move between stages
  So that I know my actions succeeded

  Scenario: Stage move confirmation
    When I move a deal to a new stage
    Then a toast appears: "Moved to {stage name}"

  Scenario: Agent scheduling confirmation
    When a deal enters an agent-owned stage
    Then a toast appears: "Agent starting soon... Scheduled {agent} agent"
    And the toast has Cancel and Run Now action buttons
```

#### PC-4: Stage owner badges on pipeline board — PASSING
```gherkin
Feature: Stage columns show ownership badges
  As a sales operator
  I want to see who owns each stage at a glance
  So that I know whether a stage is automated or human-driven

  Scenario: Agent-owned stage badge
    Given the Intel stage has stage_owner = { label: "Scout", type: "agent" }
    Then the Intel column header shows "Scout" with an AI agent icon

  Scenario: Human-owned stage badge
    Given the Invoice stage has stage_owner = { label: "Account Manager", type: "human" }
    Then the Invoice column header shows "Account Manager" with a human icon

  Scenario: Dynamic from stage_config
    When a pipeline admin changes stage_owner via Pipeline Settings
    Then the column header badge updates to reflect the change
```

### Review Gates

#### RG-1: Review tasks block stage advancement — PASSING
```gherkin
Feature: Review tasks gate deal advancement
  As a sales manager
  I want review tasks to be completed before deals advance
  So that quality is maintained at each stage

  Scenario: Pending tasks generate warnings
    Given a deal has an incomplete review task
    When I try to advance the deal
    Then the advance succeeds (soft enforcement)
    And a warning is returned: "N pending task(s) should be completed"

  Scenario: Review task created on stage entry
    Given a stage has on_enter_actions with CreateReviewTask
    When a deal enters that stage
    Then a task is created with the configured description
    And the task is linked to the deal via crm_deal_id
```

#### RG-2: Required field validation on stage exit — PASSING
```gherkin
Feature: Required fields validated on stage exit
  As a pipeline administrator
  I want to enforce that certain fields are filled before deals leave a stage
  So that downstream stages have the data they need

  Scenario: Exit validation warns on missing fields
    Given the Intel stage requires "description" before exit
    And a deal in Intel has no description
    When the deal is moved to the next stage
    Then a warning is returned: "Operator context (description) is required"
    And the move still succeeds (soft gate)

  Scenario: Intel stage validates intelligence status
    Given the Intel stage requires person intel = "done"
    And the linked person's intelligence_status is "running"
    When the deal is moved to the next stage
    Then a warning is returned: "person intelligence is not 'done' yet"
```

### Deal Detail Features

#### DD-1: Call transcript linking — PARTIAL
```gherkin
Feature: Link call transcripts to deals
  As a sales operator
  I want to link call transcripts to deals
  So that conversation context is preserved

  Scenario: View transcripts tab
    Given a deal has linked transcripts
    When I click the Transcripts tab
    Then I see a list of linked transcripts with summaries   # PASSING

  Scenario: Link a transcript via API
    When I POST to /crm/deals/:id/transcripts
    Then the transcript is linked to the deal               # PASSING

  Scenario: Auto-match transcripts from call intake
    Given a call intake item mentions a known contact
    When the call is processed
    Then the transcript is auto-linked to the contact's deal # FAILING — no auto-matching
```

#### DD-2: Call scheduling — PASSING
```gherkin
Feature: Schedule calls with deal contacts
  As a sales operator
  I want to schedule calls from the deal detail
  So that I can track meeting plans

  Scenario: Schedule a call
    Given I am on the deal detail Overview tab
    When I set a date, method (Phone/Video/In-Person), and status
    And I click Save
    Then the call schedule is saved to the deal's custom_fields
```

#### DD-3: Person invitation — PASSING
```gherkin
Feature: Generate invitation link for won deals
  As a sales operator
  I want to invite clients to the platform after winning a deal
  So that they can access their project

  Scenario: Generate invite link
    Given a deal is in the Won stage
    When I click "Generate Invite Link" in the Deck tab
    Then a token-based invite URL is generated
    And I can copy it to clipboard
```

#### DD-4: Invoice generation — PARTIAL
```gherkin
Feature: Generate and track invoices
  As a sales operator
  I want to generate invoices from deals
  So that I can track revenue

  Scenario: Send invoice from deal detail
    Given a deal has an approved proposal with amount
    When I click "Send Invoice" in the Deck tab
    Then an invoice is generated                    # PASSING — API exists
    And the invoice_id is stored on the deal        # PASSING
    And the invoice appears in AR tracking           # FAILING — no AR dashboard
```

### Pipeline Configuration

#### PC-1: Pipeline settings accessible from board — PASSING
```gherkin
Feature: Pipeline settings accessible from the kanban board
  As a pipeline administrator
  I want to configure pipeline stages from the board
  So that I can manage the pipeline without leaving the context

  Scenario: Open pipeline settings
    Given I am on the CRM Pipeline page
    When I click the gear icon next to "Add Deal"
    Then the Pipeline Settings dialog opens
    And I can switch between pipelines via dropdown
    And I see three tabs: Stages, Automations, Pipeline

  Scenario: View automation overview
    When I click the Automations tab
    Then I see all stages with their agents, triggers, actions, and validations
    In a single scrollable view
```

#### PC-2: Stage config controls pipeline behavior — PASSING
```gherkin
Feature: Stage configuration drives pipeline behavior
  As a pipeline administrator
  I want UI changes to stage config to affect pipeline behavior
  So that I can customize the pipeline without SQL

  Scenario: Assign agent via UI
    Given I edit a stage and set Assigned Agent to "Scout" with auto-trigger on
    When I save the stage
    Then deals entering that stage will trigger a Scout agent flow

  Scenario: Set required fields via UI
    Given I edit a stage and check "Description" in Required Fields
    When I save the stage
    Then deals leaving that stage will get a warning if description is empty

  Scenario: Enable approval gate via UI
    Given I edit a stage and check "Require approval"
    When I save the stage
    Then deals entering that stage will get a review task created

  Scenario: UI matches active rules
    Given a stage has on_enter_actions and on_exit_validations from migration
    When I open the Edit Stage dialog
    Then the Automation tab checkboxes reflect the active rules
    And the Active Rules tab shows the same information read-only
```

#### PC-3: All pipelines have correct stage configs — PASSING
```gherkin
Feature: Stage configs match the dealflow plan across all pipelines
  As a pipeline administrator
  I want all pipelines to have correct agent assignments and automations
  So that the system operates as designed

  Scenario Outline: Stage config matches plan
    Given the "<pipeline>" pipeline exists
    Then the "<stage>" stage has assigned_agent "<agent>"
    And auto_trigger is <trigger>
    And stage_owner label is "<owner>"

    Examples:
      | pipeline                  | stage              | agent | trigger | owner                  |
      | Sirak Studios Acquisition | Lead               |       | false   | Sales Rep              |
      | Sirak Studios Acquisition | Intel              | scout | true    | Scout                  |
      | Sirak Studios Acquisition | Business Analysis  | astra | true    | Astra                  |
      | Sirak Studios Acquisition | Proposal           | cash  | true    | Cash                   |
      | Sirak Studios Acquisition | Polish             | lux   | true    | Lux                    |
      | Sirak Studios Acquisition | Invoice            |       | false   | Account Manager        |
      | Sirak Studios Acquisition | Negotiation        |       | false   | Nora + AM              |
      | Sirak Studios Acquisition | Won                |       | false   | Team                   |
      | Sirak Studios Acquisition | Lost               |       | false   | Account Manager        |
      | Clients                   | Lead               |       | false   | Sales Rep              |
      | Clients                   | Business Analysis  | astra | true    | Astra                  |
      | Clients                   | Discovery          |       | false   | Account Manager + Nora |
      | Clients                   | Build Proposal     | cash  | true    | Cash                   |
      | Clients                   | Polish             | lux   | true    | Lux                    |
      | Clients                   | Proposal Meeting   |       | false   | Account Manager        |
      | Clients                   | Closed Won         |       | false   | Team                   |
      | Clients                   | Closed Lost        |       | false   | Account Manager        |
      | Client Delivery           | (all 7 stages)     |       | false   | PM                     |
      | Conferences               | Researching        | scout | true    | Scout                  |
```

### Spec Coverage Summary (2026-03-24, updated post-sloperation sprint)

**PASSING**: DL-1, DL-2, DL-3 (Won unified), DL-4, AA-1 (+ Run Now), AA-2, AA-3, AA-4 (Pass 2 chain), AA-5, AA-6 (auto-advance chain), AA-7 (retrigger), RG-1, RG-2, DD-2, DD-3, DD-5 (agent history), DL-5 (card badges), DL-6 (toasts), PC-1, PC-2, PC-3, PC-4 (stage owners)
**PARTIAL**: DD-1, DD-4
**FAILING**: (none)

### Remaining Gaps (PARTIAL specs)

| Spec | Gap | Effort | Priority |
|------|-----|--------|----------|
| **DD-1** | Auto-match transcripts from call intake | 1-2 days | LOW — manual linking works |
| **DD-4** | AR invoice dashboard | 2-3 days | MEDIUM — invoice generation works, tracking doesn't |

### Sloperation Intent Gaps (Audited 2026-03-24)

**Sloperation sprint (2026-03-24)** resolved all 5 missing intents and 3 partial items. See `planning/2026-03-24--plan--sloperation-intent-sprint.md`.

| Gap | Status | Resolution |
|-----|--------|-----------|
| **Discovery stage** | ✅ RESOLVED | Restored at position 3. Hero card in OverviewTab. Soft exit gate for transcript/source. |
| **Astra Pass 2 (F12)** | ✅ RESOLVED | Sequential agent queue: Astra deep_research fires first on Proposal entry, chains to Cash. |
| **Astra→Cash chaining** | ✅ RESOLVED | chain_actions in flow_config. Executor chains next agent before auto-advance. |
| **Present stage** | ✅ RESOLVED | Invoice renamed to "Present & Invoice". Presentation tracking in DeckTab. Invoice gated on presentation. |
| **Transcript→Proposal pipeline** | ✅ RESOLVED | deal_data_sources join table with dual agent/stage scoping. Agent context loads linked sources. |
| **F8: Org-specific review routing** | ✅ RESOLVED | stage_config.review_assignee with org owner fallback via resolve_review_assignee(). |
| **Won dual-path** | ✅ RESOLVED | provision_won_deal() extracted. Stage transition to Won triggers full provisioning. |
| **F3: Person invite** | ⚠️ STUB | Still logs only. Token generation + copy link deferred. Backlog: Resend email integration. |
| **Lead stage Scout label** | ⚠️ OPEN | Lead shows "Scout" badge but no agent fires there | Either add light Scout trigger or remove Scout label |

### Remaining Items from Sloperation Sprint

| Item | Effort | Priority |
|------|--------|----------|
| **SSE events for pipeline refresh** — replace 15s polling with SSE events from agent executor. Emit on flow completion, auto-advance, chaining. Frontend subscribes via useEventSourceManager. | 1 day | HIGH |
| **W8: Cost bridge** — agent flows record token usage, cost dashboard shows real VIBE data | 0.5 day | MEDIUM |
| **Data source picker dialog** — browse org's data library instead of paste UUID | 1 day | MEDIUM |
| **Data source file upload** — upload creates data_source + links to deal | 0.5 day | LOW |
| **Full E2E refresh** — update all 5 pipeline specs for 10-stage pipeline | 2 days | MEDIUM |
| **chain_to vs dependency graph** — research alternative agent chaining approaches | research | LOW |
| **stage_config.visible_tabs** — drive tab visibility from backend config | 1 day | LOW |
| **Progressive tab unlock** — tabs accumulate as deal advances, never hide | 0.5 day | LOW |
| **Resend email integration** — person invite via email on Won | 1 day | MEDIUM |
| **Auto-match transcripts** from call intake (DD-1) | 1-2 days | LOW |
| **crm_deal_automations.rs split** — 1400+ lines, split into won_provisioning + research_triggers + invoice_transcripts | 0.5 day | LOW |

### Architectural Divergences (intentional, documented)

These are places where the current implementation deliberately differs from the 317 plan:

| 317 Plan | Current | Rationale | Still Valid? |
|----------|---------|-----------|-------------|
| Deals enter at Intel (no Lead stage) | Lead with auto_skip: true (sloperation sprint) | Deals auto-skip Lead to Intel. Orgs can disable auto_skip for manual qualification. | ✅ |
| Discovery stage between BA and Proposal | ✅ Restored (sloperation sprint) | Discovery stage at position 3 with hero card + exit gates | ✅ |
| Astra Pass 2 on Discovery→Proposal | ✅ Restored (sloperation sprint) | Sequential agent queue with chain_actions | ✅ |
| Present stage (deck on live call) | ✅ Consolidated into "Present & Invoice" | Invoice renamed, presentation tracking added | ✅ |
| Hardcoded stage automations in route handlers | Data-driven `stage_config` JSON with `StageTransitionProcessor` | Enables UI-based configuration; hardcoded logic preserved as fallback | ✅ |
| `call_llm()` direct HTTP in route handlers | `WorkflowLLMService` + `AgentFlowExecutor` with tool-calling loop | Richer agent capabilities (get_deal_context, update_deal_field, save_artifact) | ✅ |
| No cancel window | 30s cancel window with frontend indicator | User control over agent execution | ✅ |
| Blocking transitions (pending tasks = hard block) | Soft enforcement (warnings, allow move) | Prevents deadlocks; operators can override gates when needed | ✅ |

---

## Phase 0 — ROI-Ordered Sprint Backlog (Q2 2026)

> These items are ordered by ROI score. See the analysis doc for scoring methodology.
> **Dogfooding is the strategy**: every item below serves dual purpose — build the platform AND discover the product through daily real-work usage.

### S0-01. CI Strictness Fix [ROI: 50.0] ✅ DONE (PR #53)
**Source:** [`roadmap/research/04-infra--cicd-gaps.md`](roadmap/research/04-infra--cicd-gaps.md)
**What:** Strict `-D warnings` clippy, all 23 pre-existing clippy errors fixed, `cargo fmt` clean, type generation passes, system deps added (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`).
**Follow-up:** ESLint `--max-warnings` at 860 (was 110) due to `simple-import-sort` on untouched files. Run `eslint --fix` across codebase to reduce. TaskStatus enum constants (replace hardcoded strings with `TaskStatus::as_str()`) — see sprint plan for details.

### S0-01b. Add E2E Tests to CI [ROI: 40.0]
**Source:** PR #53 review findings
**What:** Add Playwright E2E test step to CI workflow. Run on `main` only (not feature branches) to avoid wasteful CI runs. Use test seed DB, headless Chromium, and the existing `e2e/` suite. Merge to main first, then feature branches pick it up on rebase/merge.
**Effort:** 0.5 days | **Sprint:** 0.1 | **Status:** NOT STARTED
**Implementation:** Add job to `ci.yml` gated on `github.ref == 'refs/heads/main'`. Use `scripts/create-test-seed.sh` for DB, `npx playwright test --reporter=list`. Exclude `e2e/quarantine/` and `e2e/demos/`.

### S0-01c. Reduce ESLint max-warnings [ROI: 30.0]
**Source:** PR #51 CI fixes
**What:** Run `eslint --fix` across entire frontend to auto-fix `simple-import-sort` warnings, then reduce `--max-warnings` from 860 back to ~120. Do on main to avoid polluting feature branch diffs.
**Effort:** 0.25 days | **Sprint:** 0.1 | **Status:** NOT STARTED

### S0-01d. Fix 467 Pre-existing Clippy Warnings [ROI: 25.0]
**Source:** PR #51 CI fixes
**What:** 13 clippy lint categories currently allowed in CI (`-A` flags). Run `cargo clippy --fix` per-crate to resolve 467 warnings (326 `uninlined_format_args`, 68 `collapsible_if`, etc.), then remove `-A` flags. Do on main.
**Effort:** 1 day | **Sprint:** 0.1 | **Status:** NOT STARTED

### S0-02. Bridge Dual Cost System [ROI: 18.0]
**Source:** [`roadmap/architecture-gaps-analysis.md` §GAP-S0-03](roadmap/architecture-gaps-analysis.md)
**What:** `ai-usage.tsx` reads `TokenUsage.cost_cents` (never populated) instead of `VibeTransaction` (has real cost data). Bridge the gap so the dashboard shows actual costs.
**Effort:** 1 day | **Sprint:** 0.1 | **Status:** NOT STARTED

### S0-03. Structured Response Protocol [ROI: 13.5] ✅ DONE (pipeline-ops sprint)
**What:** Standard JSON response envelope for agent-to-orchestrator communication. Implemented via AgentFlowExecutor tool-call loop with structured artifact storage.

### S0-04. Clarification as First-Class Status [ROI: 12.0] ✅ DONE (pipeline-ops sprint)
**What:** `NeedsClarification` flow status + `clarification_request` column on agent_flows. Migration `20260413000001_agent_flow_clarification.sql`.

### S0-05. Cooldown Race Condition Fix [ROI: 10.0] ✅ DONE (tier1-phase0 sprint, commit `33a8a2031`)
**What:** Atomic `try_claim_trigger()` replaces racy `is_past_cooldown()`.

### S0-06. Agent Flow Orchestration Engine [ROI: 9.0] ✅ DONE (pipeline-ops sprint)
**What:** Real LLM dispatch via WorkflowLLMService, 3 tools (get_deal_context, update_deal_field, save_artifact), retry with model fallback, BackgroundWorker trait, cancel window support.

### S0-07. CAPO Per-Task Tracking [ROI: 8.0]
**Source:** [`roadmap/research/20-ai--unit-economics-agent-metrics.md`](roadmap/research/20-ai--unit-economics-agent-metrics.md), research-derived-backlog #40
**What:** Instrument agent invocations: tokens, model, wall-clock time. Calculate CAPO per task type. Stage 0 exit criterion.
**Effort:** 2 days | **Sprint:** 0.3 | **Depends on:** S0-02 | **Status:** NOT STARTED

### S0-08. Cost-Tiered Model Routing [ROI: 7.5]
**Source:** [`roadmap/research/10-ai--agent-orchestration.md`](roadmap/research/10-ai--agent-orchestration.md), research-derived-backlog #2
**What:** Route agent tasks to cheapest capable model. Haiku for routing, Sonnet for analysis, Opus for complex work. `pcg_router.rs` (622 lines) exists as foundation.
**Effort:** 3 days | **Sprint:** 0.3 | **Depends on:** S0-06 | **Status:** NOT STARTED

### S0-09. Append-Only Event Logging [ROI: 6.0]
**Source:** [`roadmap/research/10-ai--agent-orchestration.md`](roadmap/research/10-ai--agent-orchestration.md), research-derived-backlog #6
**What:** JSONL event log for agent orchestration events. Enables debugging, replay, audit trails.
**Effort:** 1.5 days | **Sprint:** 0.2 | **Depends on:** S0-06 | **Status:** NOT STARTED

### S0-10. Prompt Caching Strategy [ROI: 6.0]
**Source:** [`roadmap/research/20-ai--unit-economics-agent-metrics.md`](roadmap/research/20-ai--unit-economics-agent-metrics.md), research-derived-backlog #42
**What:** Anthropic prompt caching for agent system prompts + project context. 90% savings on cache reads.
**Effort:** 2 days | **Sprint:** 0.3 | **Depends on:** S0-08 | **Status:** NOT STARTED

### S0-11. Vision/Mission Anchor Document [ROI: 5.0]
**Source:** [`roadmap/research/12-ai--sudolang-aidd-framework.md`](roadmap/research/12-ai--sudolang-aidd-framework.md), research-derived-backlog #14
**What:** Formal project vision document for agent alignment. Prevents drift toward speculative features.
**Effort:** 1 day | **Sprint:** 0.1 | **Status:** NOT STARTED

### S0-12. Graceful Shutdown + Worker Registry [ROI: 4.5] ✅ DONE (tier1-phase0 sprint)
**What:** ShutdownRegistry + BackgroundWorker trait. CancellationToken-based graceful shutdown. 4 background tasks migrated.
**Effort:** 1.5 days | **Sprint:** 0.1 | **Status:** NOT STARTED

### S0-13. Input Validation Framework [ROI: 4.0]
**Source:** [`roadmap/research/08-legal--compliance-security.md`](roadmap/research/08-legal--compliance-security.md)
**What:** `validator` crate with `#[derive(Validate)]` on high-risk endpoints. OWASP baseline.
**Effort:** 1.5 days | **Sprint:** 0.2 | **Status:** NOT STARTED

### S0-13b. Fix `any` Type Warnings Across Frontend [ROI: 3.5] ✅ DONE (componentization sprint)
**What:** Replaced `any` with proper types in 50 files. Zero `no-explicit-any` warnings remaining.
**Approach:** SSE hooks → typed payloads, rjsf → typed extensions, API responses → specific interfaces, catch blocks → `unknown`.

### S0-13c. ESLint `--max-warnings` Reduction [ROI: 3.0]
**Source:** Frontend componentization sprint — current: 684 warnings (down from 807, mostly `simple-import-sort`)
**What:** Run `eslint --fix` across remaining untouched files to auto-fix import sorting. Then reduce `--max-warnings` to ~100.
**Effort:** 0.5 days | **Sprint:** 0.1 | **Status:** PARTIALLY DONE (componentization sprint reduced 807→684)

### S0-13d. Extract Remaining Large Pages [ROI: 2.5]
**Source:** Frontend componentization sprint — 3 pages still over 700 lines
**What:** Extract sub-components from:
- `pages/organization-profile/index.tsx` (766 lines) → OrgHeader, TabRouter, BrandSection
- `pages/workflows/index.tsx` (661 lines) → WorkflowGrid, RunsPanel, StagingPanel
- `pages/project-tasks/index.tsx` (660 lines) → TaskBoard, TaskFilters, TaskDetail
**Pattern:** Directory-per-parent, parent under 400 lines, index.tsx re-exports.
**Effort:** 2-3 days | **Sprint:** 0.2 | **Status:** NOT STARTED

### S0-14. Webhook Retry Worker [ROI: 3.5]
**Source:** BACKLOG P2 "Webhook Retry Logic is Dead Code"
**What:** `max_retries`/`next_retry_at` fields exist but are never read. Add retry check to schedule loop.
**Effort:** 1 day | **Sprint:** 0.2 | **Status:** NOT STARTED

### S0-19. Dogfood Friction Logging [ROI: 3.0]
**Source:** [`roadmap/5-year-product-roadmap.md` §Stage 0](roadmap/5-year-product-roadmap.md) — client discovery through dogfooding
**What:** Structured friction capture: `{workflow_step, friction_type, severity, workaround, feature_request}`. Extends existing FeedbackDialog with dogfood-specific metadata. Primary client discovery instrument.
**Effort:** 1.5 days | **Sprint:** 0.4 | **Status:** NOT STARTED

### S0-20. Editron + Social Media Workflow Templates [ROI: 2.8]
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 4](roadmap/5-year-product-roadmap.md)
**What:** Pre-built workflow templates for Editron post-production and social media content pipeline. Foundation for Stage 1 pilot onboarding.
**Effort:** 3 days | **Sprint:** 0.4 | **Depends on:** S0-06 | **Status:** NOT STARTED

### S0-15. Editron Workflow Migration [ROI: 3.3]
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 2](roadmap/5-year-product-roadmap.md)
**What:** Map Editron post-production jobs into ORCHA workflows. Test with 3 real client deliverables. Stage 0 exit criterion.
**Effort:** 5 days | **Sprint:** 0.4 | **Depends on:** S0-06, S0-20 | **Status:** NOT STARTED

### S0-16. Social Media Pipeline Migration [ROI: 2.7]
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 4](roadmap/5-year-product-roadmap.md)
**What:** Content pipeline (compose → review → schedule → publish) through ORCHA. Stage 0 exit criterion.
**Effort:** 5 days | **Sprint:** 0.5 | **Depends on:** S0-06, S0-20 | **Status:** NOT STARTED

### S0-17. Rate Limiting (tower_governor) [ROI: 2.5]
**Source:** [`roadmap/research/25-team--scaling-api-design.md`](roadmap/research/25-team--scaling-api-design.md)
**What:** Global and per-tenant rate limiting. Currently only Nora has limits.
**Effort:** 2 days | **Sprint:** 0.3 | **Status:** NOT STARTED

### S0-18. Dogfood Metrics Dashboard [ROI: 2.4]
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 7](roadmap/5-year-product-roadmap.md)
**What:** CAPO per task type, agent success rate, workflow completion rate, time savings vs. manual.
**Effort:** 3 days | **Sprint:** 0.5 | **Depends on:** S0-02, S0-07 | **Status:** NOT STARTED

---

## Previously P0 — Now Integrated Above

### ~~1. Agent Flow Orchestration Engine~~ → Renumbered S0-06
See Phase 0 sprint backlog above.

### ~~2. ACP Agents Get No MCP Servers~~ → RESOLVED
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F5)
**Resolution:** Already implemented — `load_platform_mcp_servers()` in `acp/harness.rs` loads from `default_mcp.json` and passes to all ACP sessions (Gemini, Qwen). Discovered during pipeline enhancement sprint (2026-03-16).

---

## P0.5 — Sprint Gap Fixes (from 2026-03-21 gap audit)

### GAP-1. Cost bridge frontend integration [P0] ✅ DONE
**Source:** `planning/2026-03-21--analysis--sprint-gap-audit.md`
**What:** `ai-usage.tsx` now calls `costsApi` (6 endpoints with `?days` filter). All `tokenUsageApi` references removed. Types use `number` not `bigint`.
**Status:** DONE (branch `feature/2026-03-21--gap-analysis`)

### GAP-1a. TokenUsageWidget migration [P1] ✅ DONE
**Status:** DONE — migrated to costsApi.orgSummary/orgByProject with org context

### GAP-1b. Cost formatting helpers [P2] ✅ DONE
**Status:** DONE — extracted to `frontend/src/lib/format.ts`

### GAP-1c. Cost page component extraction [P2] ✅ DONE
**Status:** DONE — split into `ai-usage/` directory (AIUsagePage 450 lines + CostSummaryCards + CostTrendChart)

### GAP-2. Agent flow engine LLM dispatch [P0]
**Source:** `planning/2026-03-21--analysis--sprint-gap-audit.md`
**What:** `agent_flow_executor.rs` polls and auto-transitions without doing work. No LLM is called. 5-day keystone item shipped as stub.
**Scope:** (1) API dispatch via PCG Router in `handle_executing_flow`, (2) parse response into `AgentResponseEnvelope`, (3) event emission on transitions, (4) `AgentFlowExecutorConfig` env vars, (5) basic test harness.
**Effort:** 3 days | **Status:** NOT STARTED
**Rationale:** Without dispatch, agent flows are inert — the entire orchestration system is non-functional.

### GAP-3. Frontend clarification form [P1] ✅ DONE
**Status:** DONE — ClarificationResponseForm in AgentFlowCard with API function + mutation hook

### GAP-4. Sidebar "Report Friction" button [P1] ✅ DONE
**Status:** DONE — Added to sidebar More popover, opens FeedbackDialog with defaultType='friction'


### GAP-5. Granular cost scoping — project + task level [P2]
**Source:** `planning/2026-03-21--analysis--sprint-gap-audit.md` (multi-scope cost section)
**What:** Cost aggregation currently org-level only. Data already has project_id, task_id, agent references — just needs query endpoints.
**Scope:** Phase 2 of cost scoping — add `project_cost_summary`, `task_cost_summary` queries + endpoints. Enables project budget tracking and task cost display.
**Effort:** 3h | **Status:** NOT STARTED
**Rationale:** Project owners need to see per-project costs for budget management. Task-level cost in the detail panel gives visibility into expensive operations.

### GAP-6. Granular cost scoping — agent + flow level [P3]
**Source:** `planning/2026-03-21--analysis--sprint-gap-audit.md` (multi-scope cost section)
**What:** Add agent-scoped and flow-scoped cost aggregation.
**Scope:** Phase 3 — add `agent_cost_summary`, `flow_cost_summary` queries + endpoints. JOIN through agent_wallets table for agent scope, through agent_flows.task_id for flow scope.
**Effort:** 2h | **Status:** NOT STARTED
**Depends on:** GAP-2 (flows need to be functional for flow costs to be meaningful)
**Rationale:** Agent profiles need cost visibility for wallet management. Flow costs track orchestration expenses.

### GAP-7. Cost query performance — TEXT column backfill [P3]
**Source:** `planning/2026-03-21--analysis--sprint-gap-audit.md` (multi-scope cost section)
**What:** Current cost JOINs use `lower(substr(hex(BLOB)))` for UUID matching — slow for large datasets. Backfill a TEXT `project_id_text` column and index.
**Effort:** 1h | **Status:** NOT STARTED
**Rationale:** Only needed when query latency becomes noticeable. Not a concern at current scale.

---

## P1 — Demo Sprint Follow-up (from PR #62 regression audit, 2026-03-31)

### DS-1. Silent error drops in pipeline backend [MEDIUM]
**What:** `let _ =` ignores failures on task/review-task creation in `stage_transition.rs` and `crm_deal_automations.rs`. Failed creates are invisible in logs.
**Fix:** Replace `let _ =` with `if let Err(e) = ... { tracing::error!(...) }` pattern.
**Effort:** 1h | **Status:** NOT STARTED

### DS-2. Empty-string defaults masking missing flow config [MEDIUM]
**What:** `flow_config.get("deal_id").and_then(...).unwrap_or("")` in `agent_flow_executor.rs` silently continues with an empty deal_id when config is malformed.
**Fix:** Return an error for missing required config fields; reserve `unwrap_or("")` for optional strings.
**Effort:** 1h | **Status:** NOT STARTED

### DS-3. stage_config JSON has no schema validation [MEDIUM]
**What:** `stage_config` column is a freeform JSON string. If a migration seeds invalid JSON or an agent writes unexpected keys, stage automation breaks silently at runtime.
**Fix:** Define a `StageConfig` Rust struct with `#[derive(Deserialize)]`, validate on read, log parse errors with the raw value.
**Effort:** 2h | **Status:** NOT STARTED

### DS-4. Stage-tab visibility falls back to ALL_TABS for unknown stages [MEDIUM]
**What:** `getVisibleTabs()` in `stage-tab-config.ts` shows all tabs for any stage name not in `STAGE_TAB_MAP`. Unknown stages (e.g., custom pipelines) show every tab regardless of relevance.
**Fix:** Read `stage_config.visible_tabs` from the backend and use that as the source of truth; `STAGE_TAB_MAP` becomes a fallback for stages without explicit config.
**Effort:** 3h | **Status:** NOT STARTED (noted as Backlog in the file already)

### DS-5. Migration pre-deployment checklist (before merging PR #62→main) [HIGH]
**What:** Several migrations in demo-sprint are irreversible or destructive:
- `20260416000000_normalize_blob_uuids_to_text`: converts 13 tables' BLOB IDs to TEXT — **take full DB backup first**
- `20260418000001_child_tables_to_contacts`: drops `person_research_passes` and `person_social_profiles` — **audit `persons.crm_contact_id IS NULL` rows first**
- `20260417000000_discovery_stage`: position reshift assumes `max(position) < 100` — **verify in prod data**
- Five stage-config UPDATE migrations overwrite JSON without backup — **verify stage automation still parses correctly after deploy**
**Status:** NOT STARTED — block merge until completed

---

## P0.5 — CI Quality (do on main, feature branches pick up on merge)

### CI-1. Add E2E Tests to CI
**Source:** PR #53 review
**What:** Add Playwright E2E test job to `ci.yml`, gated on `github.ref == 'refs/heads/main'` to avoid wasteful runs on feature branches. Use test seed DB, headless Chromium, existing `e2e/` suite. Exclude `quarantine/` and `demos/`.
**Effort:** 0.5 days | **Status:** NOT STARTED

### CI-2. Reduce ESLint max-warnings (860 → ~120)
**Source:** PR #51 CI fixes
**What:** `simple-import-sort` added but not auto-fixed on existing files, so `--max-warnings` was bumped from 110 to 860. Run `eslint --fix` across entire frontend, then lower threshold back.
**Effort:** 0.25 days | **Status:** NOT STARTED

### CI-3. Fix 467 Pre-existing Clippy Warnings
**Source:** PR #51 CI fixes
**What:** 13 clippy lint categories suppressed via `-A` flags in CI (326 `uninlined_format_args`, 68 `collapsible_if`, etc.). Run `cargo clippy --fix` per-crate, then remove `-A` flags.
**Effort:** 1 day | **Status:** NOT STARTED

---

## Post-Phase 0 — Stage 1 Preparation (Q3 2026)

> Items below are deprioritized from Phase 0 sprints. They become relevant when Stage 0 exit criteria are met and pilot onboarding begins.

---

## P1 — Major / High Impact

### 3. Unify MCP Config Systems
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F9)
**What:** Settings > MCP Servers configures Claude Code CLI's `~/.claude.json`. The executor pipeline reads `default_mcp.json`. Completely separate — confusing for users.

### ~~4. Workflow Trigger System~~ → RESOLVED
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F12)
**Resolution:** Frontend polish sprint (2026-03-18) added webhook triggers with HMAC-SHA256 validation, execution audit trail (`trigger_executions` table), per-trigger cooldown, retry logic, and full frontend UI. Event triggers and schedule triggers already existed. All three trigger types now operational.
**Known issues (from PR #49 review):** ~~Webhook route behind `require_auth` (P1)~~ FIXED, retry logic dead code (P2), cooldown race condition (P2). See P2 section below.

### ~~5. E2E Demo Test Run~~ → RESOLVED
**Source:** `2026-03-14--plan--e2e-demo-refactor.md`
**Resolution:** All suites run and passing — health checks 40/40, demos 28/31 (1 pre-existing flaky locator in bug-report-lifecycle Step 6). Structured test commands added to package.json with env pre-flight check (PR #34).

---

## P1.5 — Notification & Demo Enhancements

### ~~Notification Quick Action Buttons~~ → RESOLVED
**Source:** 2026-03-15 dogfood session
**Resolution:** Implemented in PR #39 — InboxNotificationItem has source-aware action buttons (View Task, View Run), dismiss button; ActivityNotificationItem has status chips and view buttons; NotificationCenter has improved deep-linking (2026-03-16).

### Demo Script Improvements
**Source:** 2026-03-15 dogfood session
**What:** Add better context/narration to demo scripts — annotations explaining what each step demonstrates, why it matters. Currently the demos work but don't convey the product story.
**Status:** PARTIALLY DONE — workflow CRM demo extended with Parts 5-8 (approve, runs tab, CRM contacts, pipeline). Bug report + manual QA demos still need narration.

---

## P2 — Medium / UX Polish

### ~~6. AgentWatcherPanel Error State~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #3)
**Resolution:** PR #41 added `loadError` state with error UI (AlertTriangle icon + "Failed to load agent reviewers" message + RefreshCw retry button). Distinguishes "no reviewers configured" from "API error".

### 7. DbUuid Migration — 4-Phase Plan
**Source:** `2026-03-17--plan--dbuuid-migration.md`, `notes/2026-03-14--reference--dbuuid-phase3-remaining.md`
**What:** Eliminate all Uuid/Vec<u8>/BLOB boilerplate. 4 phases:
- ~~**Phase A** (highest ROI): `AccessContext.user_id: Uuid` → `DbUuid`~~ → **DONE** (PR #47, 20 files)
- ~~**Phase B**: `Path<Uuid>` → `Path<String>` in CRM route handlers~~ → **DONE** (PR #47, 71 handlers across 7 files)
- ~~**Phase B remainder**: `Path<Uuid>` → `Path<String>` in non-CRM routes~~ → **DONE** (PR #48, 231 conversions across 34 files, 311→80 remaining)
- ~~**Phase B2**: `Uuid::parse_str()` → `DbUuid::parse()`~~ → **DONE** (PR #48, ~200 replacements across 33 route files)
- **Phase C**: Migrate `users.id` BLOB → TEXT — eliminates ALL remaining `.as_bytes()` / `Vec<u8>` / `bind_uuid_blob` code
- **Phase D**: Batch convert remaining ~104 models `Uuid` → `DbUuid`
**Status:** Phases A+B+B2 complete. 80 `Path<Uuid>` remain where model layer requires `Uuid`. Phase C/D remain.

### ~~8. Agent "View Profile" Link~~ → RESOLVED
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md` (item 7)
**Resolution:** PR #41 added agent profile page at `/agents/:agentId/profile` showing agent description, capabilities, status, default model, and autonomy level. AgentWatcherPanel agent names now link to the profile page. Route registered in App.tsx with ProtectedRoute wrapper.

### ~~8b. `any` Type Cleanup in New CRM/Pipeline Frontend Code~~ → RESOLVED
**Source:** PR #45 QA review
**Resolution:** PR #47 eliminated all 182 `: any` in targeted files. 107 remain in PR #46 org-profile files (not in scope). See `2026-03-18--plan--dev-velocity-sprint.md`.

### ~~8c. Sovereign Stack Volume Path Deduplication~~ → RESOLVED
**Source:** PR #45 QA review
**Resolution:** PR #47 extracted `resolve_volume_path()` to `crates/utils/src/volume.rs`. Both `org_cloud.rs` and `org_cloud_indexer.rs` now use the shared utility.

### 9. `http_request` Node Guardrails
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md`
**What:** No OAuth/auth token management, no URL allowlisting, no rate limiting. Security concern for production.

### ~~10. Rename "Bug Triage Pipeline"~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #12)
**Resolution:** Renamed to "Feedback Triage Pipeline" in `data_source_workflows.rs`, `feedback.rs`, and E2E tests (2026-03-16).

### 11. ~~No Success Toast for Feedback Submission~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #4)
**Resolution:** FeedbackDialog now uses toast (commit `7cd4b22c9`)

### ~~12. Project Task Count Doesn't Auto-Refresh~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #5)
**Resolution:** Fixed in PR #38 — sidebar task count refresh on task creation/update (2026-03-16).

---

## P2.5 — Code Quality Infrastructure

### Lint-Staged + Import Sort + Prettier Pre-Commit
**Source:** PR #49 wrap-up (2026-03-18)
**What:** Full setup for auto-formatting on every commit:
- `lint-staged` + `eslint --fix` + `prettier --write` on staged `.ts`/`.tsx` files
- `eslint-plugin-simple-import-sort` added (auto-sorts imports on `--fix`)
- `.githooks/pre-commit` hook with `git config core.hooksPath .githooks`
- `eslint --fix` already run across entire codebase (658 files, import reordering)
- `--max-warnings` threshold needs updating after import sort warnings cleared
**Stash:** `git stash list` → "lint-staged + import-sort setup + eslint --fix" on `refactor/frontend-polish-sprint`. Apply with `git stash pop` on the target branch.
**Status:** STASHED — ready to apply on a new branch

---

## P2.5 — Modularity Sprint 6 Candidates

### Modularity Sprint 6 — Hook Mutations + Remaining Debt
**Source:** `archive/2026-03-16--plan--modularity-sprint-5.md` (deferred 2e), PR #46 review
**What:** Remaining raw `useMutation` → `useMutationWithToast`:
- ~~`hooks/useOrgOnboarding.ts` (4 mutations)~~ → Done (PR #46)
- ~~`hooks/useWorkflowTemplates.ts` (1 mutation)~~ → Done (PR #46)
- ~~`hooks/useCrmActivities.ts` (2 mutations)~~ → Done (PR #46)
- `hooks/useAgentFlows.ts` (4 mutations)
- `hooks/useTaskMutations.ts` (3 mutations) — complex: async activity logging side effects
- `pages/oss-library-listener.tsx` (4 mutations)
- `pages/data-sources/RunWorkflowFromSourceDialog.tsx` `runMutation` — conditional toast logic (split from data-sources.tsx in PR #47)
Also: remaining conversation helper adoption (nora/voice, agent_chat, twilio), ~15 inline query keys (down from ~318 via PRs #46, #48)
- Over-invalidation in autonomy/bowser/collaboration mutations (broad `autonomyKeys.all` instead of targeted keys)
- `MembersTab` raw `fetch()` → `makeRequest` migration
- ~~Query key string mismatches: `['brandProfile', orgId]` vs `['orgBrandProfile', orgId]`, `['workflowTemplates']` vs `['workflow-templates']`~~ → **FIXED** (PR #48)
**Source also:** `archive/2026-03-16--plan--modularity-sprint-4.md` (deferred items)
**Status:** PARTIALLY DONE (PR #46 converted 19 mutations, PR #48 migrated 74 inline keys + fixed 2 key mismatches. ~15 inline keys remain.)

### Modularity — Large Frontend File Splits
**Source:** `archive/2026-03-15--plan--modularity-sprint-2.md` (remaining large files section)
**What:** Largest remaining frontend files:
- ~~`virtual-environment.tsx` (1,282 lines)~~ → Done (PR #47, split to `virtual-environment/`)
- `TaskFormDialog.tsx` (1,240 lines) — complex form, high-traffic
- `company-profile.tsx` (1,218 lines) — similar pattern to project-detail split
- ~~`MeetingMode.tsx` (1,207 lines)~~ → **DELETED** (PR #48, replaced by `meeting-mode/` directory)
- `CrmDealDetailPanel.tsx` (1,202 lines) — grew in PR #36
- ~~`data-sources.tsx` (991 lines)~~ → Done (PR #47, split to `data-sources/`)
- ~~`project-tasks.tsx` (998 lines)~~ → **DELETED** (PR #48, replaced by `project-tasks/` directory)
**Status:** PARTIALLY DONE. 6 files >900 lines remain. PR #48 deleted 2 dead monolith originals (-2,205 lines).

### Component Hook Extraction — Research Similar Patterns
**Source:** Frontend polish sprint (2026-03-18), task card refactor
**What:** Extracting shared hooks from task cards (`useResolvedAssignee`, `useResolvedAgent`, `useScrollIntoView`) + shared sub-components (`PriorityBadge`, `DueDateBadge`, `CollaboratorAvatars`) reduced TaskCard 396→259 lines and EnhancedTaskCard 541→428 lines while eliminating duplication. Research similar opportunities across the codebase:
- **CRM cards** (`CrmDealCard`, `CrmContactCard`) — likely duplicate assignee resolution, priority badges
- **Project cards** (`ProjectCard`) — may have inline member avatar logic that parallels `CollaboratorAvatars`
- **Detail panels** — `CrmDealDetailPanel`, `TaskDetailsPanel` likely duplicate the assignee IIFE pattern
- **Scroll-into-view** — search for `scrollIntoView` calls across components, consolidate to `useScrollIntoView`
- **Agent name resolution** — any component showing agent names should use `useResolvedAgent` instead of inline lookup
**Approach:** Audit with `grep -r "usersMap?.get\|scrollIntoView\|agentsMap" frontend/src/` to find candidates. Prioritize files >400 lines with inline data resolution patterns.
**Status:** NOT STARTED — research item for next modularity sprint

### Component Hook Extraction — Research Similar Patterns
**Source:** Frontend polish sprint (2026-03-18), task card refactor
**What:** Extracting shared hooks from task cards (`useResolvedAssignee`, `useResolvedAgent`, `useScrollIntoView`) + shared sub-components (`PriorityBadge`, `DueDateBadge`, `CollaboratorAvatars`) reduced TaskCard 396→259 lines and EnhancedTaskCard 541→428 lines while eliminating duplication. Research similar opportunities across the codebase:
- **CRM cards** (`CrmDealCard`, `CrmContactCard`) — likely duplicate assignee resolution, priority badges
- **Project cards** (`ProjectCard`) — may have inline member avatar logic that parallels `CollaboratorAvatars`
- **Detail panels** — `CrmDealDetailPanel`, `TaskDetailsPanel` likely duplicate the assignee IIFE pattern
- **Scroll-into-view** — search for `scrollIntoView` calls across components, consolidate to `useScrollIntoView`
- **Agent name resolution** — any component showing agent names should use `useResolvedAgent` instead of inline lookup
**Approach:** Audit with `grep -r "usersMap?.get\|scrollIntoView\|agentsMap" frontend/src/` to find candidates. Prioritize files >400 lines with inline data resolution patterns.
**Status:** NOT STARTED — research item for next modularity sprint

### Workflow UX — Deferred Polish
**Source:** `archive/2026-03-16--plan--workflow-ux-sprint.md` (deferred items + PR #44 review)
**What:**
- Extract `DataSourceWorkflowRunner` from `data-source-detail.tsx` (~730 lines) — Day 2 deferral
- ~~`CopyWorkflowDialog` NiceModal migration~~ → Already on NiceModal (PR #46 converted its mutation)
- ~~`window.confirm()` → shadcn `AlertDialog`~~ → All `window.confirm()` calls eliminated (PR #46 Day 1)
- Ownership filter toggle (My/Org/All) on workflow cards — badges exist but no filtering
**Status:** PARTIALLY DONE (PR #46 resolved confirm/mutation items)

### Modularity — Deferred Stretch Items
**Source:** `archive/2026-03-15--plan--modularity-sprint-2.md` (Day 5c)
**What:** Hook splits deferred (below priority threshold):
- `useConversationHistory.ts` (540 lines) → extract `flattenEntries`, `executionHelpers`, `patchWithKey`
- `useAutonomy.ts` (478 lines) → extract `useCheckpoints`, `useApprovalGates`
**Status:** NOT STARTED

### Modularity — formatRelativeDate Centralization
**Source:** PR #37 QA review
**What:** 3 independent `formatRelativeDate` implementations remain in EmailInbox, CommunicationsInbox, WorkflowRunsPanel. Different logic in each (today/yesterday vs "Xm ago" format). Could centralize with a configurable formatter.
**Status:** NOT STARTED — not a regression, just incomplete DRY

---

## P1 — PR #49 Review: Critical Deferred Issues

### ~~Webhook Route Behind `require_auth` Middleware~~ → RESOLVED
**Source:** PR #49 backend review (2026-03-18)
**Resolution:** Split `workflow_triggers::router()` into authenticated `router()` (CRUD) + public `public_router()` (webhook handler). Webhook endpoint now registered in `base_routes` (before `require_auth` layer). HMAC-SHA256 is the sole auth mechanism for webhooks.

---

## P2 — PR #49 Review: Medium Priority Deferred Issues

### Webhook Retry Logic is Dead Code
**Source:** PR #49 backend review (2026-03-18)
**File:** `crates/db/src/models/workflow_trigger.rs`
**What:** `max_retries`, `retry_count`, and `next_retry_at` fields are stored in the DB but never read or acted upon. No background worker checks `next_retry_at` to re-execute failed triggers.
**Recommendation:** Either (a) implement a retry worker in `spawn_workflow_schedule_loop` that checks for triggers past `next_retry_at` with `retry_count < max_retries`, or (b) remove the fields if retry isn't needed yet.
**Status:** NOT STARTED

### ~~Webhook Cooldown Race Condition~~
**Status:** ✅ RESOLVED (2026-03-21) — Both `data_source_workflows.rs` and `workflow_triggers.rs` now use atomic `try_claim_trigger()` with `UPDATE...WHERE...RETURNING`. Committed in sprints 4 (tier1-phase0) and the original PR #53.

### `webhook_url` Stored as Relative Path
**Source:** PR #49 backend review (2026-03-18)
**File:** `crates/db/src/models/workflow_trigger.rs`
**What:** `webhook_url` field stores a relative path (like `/webhooks/abc123`) rather than a full URL. Without the server hostname, the stored URL is not useful for display or documentation.
**Recommendation:** Either store the full URL (constructed from `HOST`/`BACKEND_PORT` at creation time) or construct it dynamically in the API response.
**Status:** NOT STARTED — low impact

### `useResolvedAgent` O(n) Scan Per Card
**Source:** PR #49 frontend review (2026-03-18)
**File:** `frontend/src/components/tasks/task-card-parts/useResolvedAgent.ts`
**What:** When `agentsMap` doesn't have a direct hit, the hook does `Array.from(agentsMap.values()).find()` — O(n) per card per render. With many agents and cards, this adds up.
**Recommendation:** Build a secondary `Map<string, Agent>` keyed by `short_name` at the page level (alongside `agentsMap`), pass it as an optional prop. Falls back to O(n) scan only if secondary map not provided.
**Status:** NOT STARTED — low impact unless agent count grows

### ~~WorkflowTriggersPanel Toggle Toast Not Descriptive~~ → RESOLVED
**Resolution:** Changed `successMessage` to `(result) => \`Trigger ${result.enabled ? 'enabled' : 'disabled'}\``.

### ~~`useBranchLoader` Swallows Errors Silently~~ → RESOLVED
**Resolution:** Added `console.error` to empty catch block.

### ~~`useTopsiVoice` Exports Unused Functions~~ → RESOLVED
**Resolution:** Removed `startRecording`/`stopRecording` from `TopsiVoiceActions` interface and return object. Functions remain internal for use by `handlePushToTalkStart`.

### ~~`useMoveDeal` Missing Error Toast~~ → RESOLVED
**Resolution:** Added `toast.error('Failed to move deal')` + `console.error` in `onError` handler. Kept raw `useMutation` for optimistic update pattern.

### `workflowKeys` Mixed Naming Convention
**Source:** PR #49 frontend review (2026-03-18)
**File:** `frontend/src/lib/query-keys.ts`
**What:** `workflowKeys` uses mixed casing — some keys use camelCase, others use kebab-case strings internally. Inconsistent but not buggy (keys match between factory and usage).
**Recommendation:** Standardize to camelCase in a future query key sweep.
**Status:** NOT STARTED — cosmetic

### NetworkSettings Wasted API Call
**Source:** PR #49 frontend review (2026-03-18)
**File:** `frontend/src/pages/settings/NetworkSettings.tsx`
**What:** Fetches `/api/mesh/stats` on mount even when the mesh stats panel isn't visible.
**Recommendation:** Gate the query with `enabled: isPanelExpanded` or lazy-load the stats section.
**Status:** NOT STARTED — low impact

---

## P2.7 — E2E Test Infrastructure Issues

### E2E: Removed Assertions Without Replacement (PR #49)
**Source:** PR #49 e2e review (2026-03-18)
**Files:** `e2e/demos/bug-report-lifecycle.spec.ts`, `e2e/demos/manual-qa-trigger.spec.ts`
**What:** PR removed `waitForToast` assertions for task state transitions and `page.reload()` before watcher badge assertions. These were removed to fix test failures, but no equivalent assertions replaced them — the tests now skip verification of those behaviors.
**Recommendation:** Re-add assertions using a more resilient pattern (e.g., `page.waitForSelector` for toast elements, or poll-based checks for badge appearance without hard reload).
**Status:** NOT STARTED — reduces test coverage

### ~~E2E: `DEMO_PR_NUMBER` Used Without Null Guard~~ → RESOLVED
**Resolution:** Added `test.skip(!DEMO_PR_NUMBER, "Skipping — Step 4 did not create a PR")` guard at top of Step 6.

### E2E: Conditional Visual Checks Are No-Op
**Source:** PR #49 e2e review (2026-03-18)
**Files:** Multiple demo specs
**What:** Several visual checks are wrapped in `if (element) { expect... }` — if the element isn't found, the assertion silently passes. These are no-op assertions that give false confidence.
**Recommendation:** Either make the assertions unconditional (fail if element missing) or use `test.fixme()` to mark as known incomplete.
**Status:** NOT STARTED — reduces test value

### E2E: Testid Coverage Gaps in CRM Components
**Source:** Testid audit (2026-03-24)
**What:** `shared/testids.ts` is now the single source of truth for `data-testid` attributes, used by both frontend components and E2E tests. 22 of 53 interactive CRM elements have testids. The 31 missing are:
- **PipelineSettingsStageDialog**: all tab triggers, Save Stage button, checkbox controls (7 elements)
- **StageConfigEditor**: agent select, auto-trigger checkbox, cancel window input, 5 required-field checkboxes, approval gate (8 elements)
- **DeckTab**: Regenerate, Share for Review, Cancel buttons, Copy/Regenerate invite (7 elements)
- **CrmPipelineBoard**: empty state Add Deal, per-stage "+ Add deal" (3 elements — helpers exist: `tid.addDealEmpty`, `tid.addDealStage`)
- **CrmPipelineSettings**: pipeline dropdown, Add Stage, New Pipeline buttons (3 elements)
- **PipelineSettingsStagesTab**: Move up/down, Delete stage buttons (2 elements)
- **DealHeader**: pipeline stage stepper clicks (1 element)
**Recommendation:** Add testids as needed when writing tests that target these elements. Helpers exist in `shared/testids.ts` — add new ones there first, then import in both component and spec.
**Status:** PARTIAL — primary interaction paths covered (PR #59), secondary/config controls pending. Close button and resize handle testids added.

### E2E: Fragile Title-Based Button Selectors
**Source:** PR #49 e2e review (2026-03-18)
**Files:** Workflow demo specs
**What:** Button selectors use `[title="..."]` which breaks if button text/title changes. Also `addExtractNode` matches placeholder strings instead of `data-testid`.
**Recommendation:** Add `data-testid` attributes to workflow editor buttons and use those in selectors.
**Status:** NOT STARTED — fragile but functional

### E2E: `timing.ts` Pace Detection Doesn't Detect `demos` Project
**Source:** PR #49 e2e review (2026-03-18)
**File:** `e2e/helpers/timing.ts`
**What:** Pace detection logic checks for `--project=` arg but may not correctly identify the `demos` project, causing demo tests to run at default (fast) pace instead of demo pace.
**Recommendation:** Verify pace detection logic handles `--project=demos` correctly. Add a test or log output.
**Status:** NOT STARTED

### E2E: GITHUB_TOKEN Required for Agent Simulation Tests
**Source:** Frontend polish sprint QA (2026-03-18)
**Tests affected:** `bug-report-lifecycle.spec.ts` Step 4, `manual-qa-trigger.spec.ts` Step 3
**What:** `simulateDevAgentWork()` and `createPrForTask()` in `e2e/helpers/demo/simulation.ts` call `ghApi()` which requires `GITHUB_TOKEN` env var. Tests fail immediately with "GITHUB_TOKEN env var required for demo simulation".
**Recommendation:** Either: (a) set `GITHUB_TOKEN` in `.env.test` for CI, (b) add mock mode to `simulation.ts` that fakes GitHub API responses for local testing, or (c) skip these steps gracefully with `test.skip(!process.env.GITHUB_TOKEN, "GITHUB_TOKEN required")`.
**Status:** NOT STARTED

### E2E: Pipeline Intelligence Expects 8 Stages, Seed Has 7
**Source:** Frontend polish sprint QA (2026-03-18)
**Tests affected:** `pipeline-intelligence-workflow.spec.ts` Part 1
**What:** Test asserts `stageList.length >= 8` but seed DB only provides 7 pipeline stages.
**Recommendation:** Either: (a) update seed DB to include all 8 expected stages, or (b) update test to match actual seed data (7 stages), or (c) add a test setup step that creates the missing stage via API.
**Status:** NOT STARTED

### E2E: Workflow CRM/Spanish Pipeline Demos Need LLM Backend
**Source:** Frontend polish sprint QA (2026-03-18)
**Tests affected:** `workflow-crm-pipeline.spec.ts` Part 3+, `workflow-spanish-pipeline.spec.ts` Part 3+
**What:** Workflow execution requires PCG Router (LLM backend) to produce staged records. Parts 1-2 (build workflow + create data source) pass, but Part 3+ (run workflow, verify staged output) times out without an active LLM backend.
**Recommendation:** These tests are integration-level and require full stack. Tag with `@requires-llm` annotation and skip in CI unless LLM backend is available. Add `test.skip(!process.env.LLM_BACKEND_URL, "LLM backend required")` guard.
**Status:** NOT STARTED

---

## P3 — Developer Experience & Testing Infrastructure

### Error Messages Should Include Actionable Steps (Role-Scoped)
**Source:** Frontend polish sprint QA (2026-03-18) — user feedback
**What:** When operations fail (e.g., workflow run, API calls), error messages should:
1. Include actionable steps the user can take to fix the issue
2. Scope detail level by user role: platform-level users see system details (ports, services, config), end users see only user-facing guidance without platform internals
3. Never leak infrastructure details (server ports, internal service names, file paths) to non-platform users
**Examples:**
- Workflow run failure → Platform user: "Workflow execution failed: LLM backend not reachable at PCG Router. Check service status or configure alternative model." End user: "Workflow could not complete. Please try again or contact your administrator."
- Missing config → Platform user: "GITHUB_TOKEN not configured. Set in .env or environment." End user: "Integration not configured. Contact your administrator."
**Recommendation:** Create an `AppError` utility that accepts error detail + user role, returns appropriate message. Add `isSystemError` flag for errors that should only show technical details to admins.
**Status:** NOT STARTED

### E2E: Missing Env Vars Should Warn, Not Silently Fail
**Source:** Frontend polish sprint QA (2026-03-18) — user feedback
**What:** When essential env vars (GITHUB_TOKEN, LLM_BACKEND_URL, etc.) are missing, tests throw cryptic errors deep in execution instead of warning upfront. The `simulation.ts` helper throws at line 15 but only after 3 prior steps pass, wasting test time.
**Recommendation:** Add a pre-flight env check to `e2e/helpers/index.ts` or `playwright.config.ts` globalSetup that logs warnings for optional env vars and fails fast for required ones. Pattern: `console.warn("⚠️ GITHUB_TOKEN not set — agent simulation tests will be skipped")`.
**Status:** NOT STARTED

### PR #61: Migration CAST fragility in contacts_intelligence backfill
**Source:** QA review PR #61 (2026-03-27)
**File:** `crates/db/migrations/20260418000000_contacts_intelligence.sql:30-53`
**What:** Backfill UPDATE uses `CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)` 8 times. If both columns are TEXT, CASTs are unnecessary. If one is BLOB, this may silently fail to match. A single CTE or JOIN would be more efficient.
**Recommendation:** Simplify to `WHERE p.crm_contact_id = crm_contacts.id` (both are TEXT post BLOB→TEXT migration). Replace 8 correlated subqueries with a single CTE-based UPDATE.
**Status:** NOT STARTED — affects contacts unification phase, not blocking current sprint

### PR #61: ReviewTab missing error UI for task fetch
**Source:** QA review PR #61 (2026-03-27)
**File:** `frontend/src/components/crm/deal-detail/tabs/ReviewTab.tsx:162-169`
**What:** `useQuery` for deal tasks has no `isError` state rendered. API failures silently default to empty array. User sees no feedback.
**Recommendation:** Add error boundary or `isError` check with retry button, matching pattern from AgentHistoryTab.
**Status:** NOT STARTED — low priority, follows existing tab pattern

### PR #61: Agent History "View Task" link is generic
**Source:** QA review PR #61 (2026-03-27)
**File:** `frontend/src/components/crm/deal-detail/tabs/AgentHistoryTab.tsx:186-191`
**What:** "View Task" navigates to `/my-tasks` instead of deep-linking to the specific task. `flow.task_id` is available but `project_id` is not in `AgentFlowSummary`.
**Recommendation:** Add `project_id` to the agent flows API response, then link to `/projects/{projectId}/tasks/{taskId}`.
**Status:** NOT STARTED — UX improvement

### PR #61: E2E timeout increases are symptomatic
**Source:** QA review PR #61 (2026-03-27)
**Files:** `e2e/pipeline/agent-automations.spec.ts:187,202`
**What:** Test timeouts doubled (45→90s, 30→60s) to reduce flakiness. Increases test suite runtime.
**Recommendation:** Replace polling with SSE event wait or targeted condition checks.
**Status:** NOT STARTED — test infrastructure improvement

### ~~13. Onboarding Dialog Bypass for Test Environments~~ → RESOLVED
**Source:** PR #47 smoke testing (2026-03-18)
**Resolution:** PR #48 unified 4 sequential modals into single WelcomeWizard, added `VITE_SKIP_ONBOARDING=1` env var bypass, and SetupProgress sidebar indicator for return-to-setup.

### ~~14. SSE Connections Should Not Fire Before Authentication~~ → RESOLVED
**Source:** PR #47 smoke testing (2026-03-18)
**Resolution:** PR #48 added `enabled` option to `useAgentDirectory` hook (default true). Connections now skippable for pre-auth contexts.

### 15. Seed Database Refresh for Smoke Testing
**Source:** PR #47 smoke testing (2026-03-18)
**What:** Seed DB (`dev_assets_seed/duck_kanban.db`) lacks CRM deals, communications, and org cloud data. Smoke testing these features requires creating data manually via the UI or API, which is slow and fragile.
**Rationale:** Every QA pass on CRM/comms features starts from zero. A richer seed with sample pipelines, deals, contacts, and comms would make smoke testing 10x faster.
**Proposal:** Add seed data generation script or extend existing seed with: 1 pipeline per org with 3-5 deals across stages, 5-10 CRM contacts, sample call/SMS records. Run as part of `flox activate` or `npm run seed:dev`.
**Status:** NOT STARTED

## P3.5 — CRM UX Polish

### 16. CRM Deal Card — Explain "Research Needed" Badge
**Source:** PR #47 smoke testing (2026-03-18)
**What:** New deals immediately show an orange "Research needed" badge and "5%" confidence score with no explanation of what triggers research or what the percentage represents.
**Rationale:** Users creating simple deals to track prospects don't expect AI research status. The badge implies action is needed but provides no path to take that action. Confidence score has no tooltip or explanation.
**Proposal:** Add tooltip on hover explaining the badge ("AI research has not been run for this deal. Click to start."). Show confidence explanation ("Based on available data about this prospect"). Consider making research opt-in rather than defaulting to "needed".
**Status:** NOT STARTED

### 17. Pipeline Aggregate Should Show Currency
**Source:** PR #47 smoke testing (2026-03-18)
**What:** Pipeline header shows "$50,000" but the deal form supports multiple currencies (USD, EUR, etc.). If deals have mixed currencies, the aggregate sum would be misleading.
**Rationale:** Financial data accuracy is critical for CRM. Mixing currencies in a sum is a data integrity issue.
**Proposal:** Either: (a) normalize all values to pipeline's base currency with conversion, (b) show aggregate per currency, or (c) only show aggregate when all deals share the same currency — otherwise show "Mixed currencies".
**Status:** NOT STARTED

### 18. Pipeline Deal Cards — Show Invalid/Blocked State
**Source:** E2E testing session (2026-03-24)
**What:** When a deal has errors that prevent it from continuing through the pipeline (e.g., missing required fields, failed agent flows, validation errors), there is no visual indication on the kanban card. The deal looks normal, and the error only surfaces when attempting to move it — as a toast that disappears.
**Rationale:** Users need to see at a glance which deals need attention. Silent failures mean deals get stuck without anyone noticing.
**Proposal:** Add a visual state to deal cards for "blocked" or "needs attention":
- Red/orange border or badge when the deal has blocking validation errors
- "Agent failed" badge when the last agent flow errored
- Tooltip showing what's blocking the deal from advancing
- Consider preventing drag/move for deals that will definitely fail validation (show reason on hover)
**Status:** PARTIAL (PR #59) — "Agent failed" and "Agent cancelled" badges now show on deal cards. Failed flows included in kanban query. Tooltip and drag-prevention still TODO.

### 19. Agent Flow Interaction — Run Now / Cancel / View Results
**Source:** E2E testing session (2026-03-24)
**What:** When a deal moves to an agent stage (Intel, BA, Proposal, Polish), a toast shows "Scheduled scout agent" with Cancel/Run Now buttons, but:
- Tests don't verify what happens when you click Run Now or Cancel
- No E2E coverage of the Agent History tab after an agent completes
- Agent failures (LLM errors, timeouts) have no UI feedback on the deal card
**Rationale:** The agent notification is the user's only interaction point with the agent system. If they click Run Now, they should see the agent progress. If the agent fails, they should know.
**Proposal:**
1. E2E test: click Run Now → verify agent_flow status transitions → verify Agent History tab shows flow
2. E2E test: click Cancel → verify agent_flow is cancelled → verify no agent work runs
3. Agent failure: deal card shows "Agent failed" badge, Agent History tab shows error details with retry button
**Status:** PARTIAL (PR #59) — Run Now tested in AA-1, Agent History tab tested in AA-3, Retry button implemented + retrigger API wired. Cancel test and re-trigger E2E still TODO.

---

## Active Branch Conflict Notes (2026-03-18)

### `sloperation317` — 11 commits behind main (PRs #46, #47, #48)
Dealflow pipeline v2, company profiles, brand guides, Dockerfile fixes, VIBE tokenomics plan.
**Conflicts expected with:** DbUuid::parse changes (PR #48), query key factories (PR #48), file splits (PR #46/47), onboarding wizard (PR #48), Path<Uuid>→Path<String> conversions (PR #48).
**Status**: Next sprint target — merge to main with quality pass applying new standards.

### Stale branches (can be cleaned up):
- `sloperation316-pipeline-progress` — superseded by sloperation317
- `sloperation316-vibe-integration` — superseded by sloperation317
- `refactor/dev-velocity-sprint` — merged as PR #47
- `refactor/ux-polish-quality-sprint-2` — merged as PR #48
- `refactor/frontend-polish-sprint` — in review, may have active work

### Pre-existing DB/Server Issues (found during e2e setup)
1. **Seed DB BLOB→TEXT**: `users` table still has BLOB UUIDs. Runtime fix: `UPDATE users SET id = lower(substr(hex(id),...))`. Seed needs regeneration.
2. **Onboarding warning**: `table projects has no column named slug` — `user_onboarding.rs` references a column missing from seed schema. Non-blocking.
3. **Migration checksum mismatch**: `20260409000000_org_cloud.sql` was modified after being applied to seed. Fresh seed + `sqlx migrate run` required.

---

## P3 — Low Priority / Future Sprints

### 13. Schema Improvements for Dogfood Pipeline
**Source:** `archive/2026-03-13--plan--dogfood-pipeline-activation.md`
- Add `qa` status to distinguish QA review from human approval on kanban
- Add `deployed` status to distinguish "code merged" from "manually done"
- Add `task_id` FK on `merges` table for direct task→PR lookup
- Add `task_status_history` table for full audit trail

### 14. Sprint 2-3 Roadmap Items
**Source:** `archive/2026-03-12--plan--three-sprint-roadmap.md`
- Convert hardcoded automations to editable system workflows
- ~~Bulk task operations (select multiple, batch status change)~~ → Done (PR #38)
- Progress indicator for agent execution on task cards
- Undo reject / re-open task workflow

### 15. Known Limitations (Design Choices)
- Conditional node: only simple string matching + `count>N`
- CRM update nodes: match by email/name only (no fuzzy matching)
- ACP MCP loading: path depends on server working directory
- Task FTS search index: auto-sync triggers dropped — needs app-level reindexing

### 16. UI Polish — Future Improvements
- Task card inline editing
- Keyboard shortcuts for common actions
- Board column WIP limits
- Workflow versioning/history
- Mobile user menu shortcut (avatar in navbar)
- View-as keyboard shortcut (Cmd+Shift+V)
- Sidebar section animated transitions
- ~~Settings tab URL persistence~~ → Done (PR #39, `?scope=` param)

### 18. UX Audit — Remaining Items (from `2026-03-16--review--ux-design-audit.md`)
**P2**: Topsi chat suggested prompts (#14)
~~Login "session expired" on first visit (#16)~~ → Already fixed (PR #38, `hadPriorSession` localStorage check)
~~Dev banner space reduction (#17)~~ → Fixed (PR #41, inline pill in navbar header)

### 19. UX Audit — Partially Addressed (depth improvements)
**Source:** `archive/2026-03-16--review--ux-design-audit.md`
- Pipeline: stage visual differentiation (gradient backgrounds), agent action tooltip improvements (#2)
- ~~Notifications: tabs (All/Unread/Mentions), "Mark all read" button, full notifications page (#4)~~ → RESOLVED (PR #41)
- KPI trends: trend arrows with percentages (requires API), time period selector, sparklines (#10)
- Project cards: last activity timestamp, assignee avatars (#18)
- Task card data population: seed data needs richer metadata (assignees, due dates, tags) (#3)
- Breadcrumbs: "Jungleverse" still appears as default org name in some views (#15)

### 20. User Account Onboarding (deferred from UX Sprint 2)
**Source:** `archive/2026-03-16--plan--ux-engagement-polish-sprint2.md`
**What:** First-login walkthrough — profile setup, preferences, Topsi intro. Org onboarding (PR #39) covers org-level setup; user account onboarding covers individual user first-run experience.
**Status:** NOT STARTED

### 17. React Router v7 Migration Warnings
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md`
- Future flag deprecation warnings in console
- `validateDOMNesting` warning (button nested in button in workflow editor)

---

## TODO — Manual Verification

- [ ] Edit Task / Feedback / Project Form dialogs: verify sticky footer
- [ ] ResizableDrawer: drag resize, expand/collapse, click-outside-to-close
- [ ] Fullscreen BreadcrumbNav visible, Esc to close
- [ ] Start dev server → Settings > Agents → agents load (no 500)
- [ ] Create Task → Agent picker → agents listed
- [ ] Task detail → Agent Watcher panel → no 500 errors
- [ ] Run E2E demos headed: `FRONTEND_PORT=3001 npx playwright test --project=demos`

---

## Resolved (Removed from Backlog)

| Item | Resolution |
|------|-----------|
| BLOB UUID agent endpoints (QA #1, #2, #7, #8) | DbUuid Phase 1-2 (commit `2cf77c82b`) |
| Edit Task modal not scrollable (QA #14) | FormDialogBody helper (commit `e72d80187`) |
| Task detail no back navigation (QA #16) | ResizableDrawer + BreadcrumbNav (commits `761bdb609`–`fad6c9fd7`) |
| QA watcher refactor (Phase 6) | Complete (PR #23) |
| Topsi-Workflow Bridge | Complete (PR #22) |
| Dogfood Pipeline phases 7-11 | Complete |
| Task activity logging (Bug #9) | DbUuid migration + ActivityLog calls (commit `bb2e8dfb7`, `ace91ffcb`) |
| Feedback tasks no dev agent (Bug #10) | Topsi assignment in feedback.rs (commit `49363911c`) |
| Manual InReview QA trigger (Bug #11) | Watcher trigger in update_task (commit `ace91ffcb`) |
| Intelligence Overview 0 sources (Bug #6) | find_by_organization query (commit `49363911c`) |
| send_notification node + in-app notifications | Full stack: migration, model, routes, frontend bell (commit `bc07ba53e`) |
| Workflow trigger hardening | Tracing spans + CancellationToken (commit `5165b4091`) |
| Status dropdown in task detail (Backlog #10) | Combobox in task detail header (commit `3b40cf272`) |
| Loading skeletons (Backlog #11) | Skeleton components for My Tasks (commit `7d2e910f1`) |
| BLOB binding for project creation | bind_uuid_blob helpers (commit `e6f34f140`) |
| Drawer click-outside closes on portaled overlays | Radix overlay detection (commit `cb31ba595`) |
| E2E demo script duplication + headed mode | Dedicated fixtures + shared helpers (commit `55d24889a`) |
| PR #27 C1: notifications BLOB/TEXT mismatch | BLOB bytes bind for project_members query (commit `dd3f20388`) |
| PR #27 C2: bind_uuid_blob panic | Returns Result (commit `1b65c73a1`) |
| PR #27 C3: fragile GitHub URL parsing | url::Url parser (commit `dd3f20388`) |
| PR #27 C4: contentFullscreen stuck in localStorage | partialize excludes it (commit `1b65c73a1`) |
| PR #27 C5: status dropdown cache miss | queryClient.invalidateQueries (commit `1b65c73a1`) |
| PR #27 W3: Notification/ActivityLog String→DbUuid | Consistency fix (commit `dd3f20388`) |
| PR #27 W4: feedback agent arbitrary selection | Agent::find_default_assignee (commit `dd3f20388`) |
| PR #27 W8: FeedbackDialog nested JSX | Success → toast notification (commit `7cd4b22c9`) |
| PR #27 W14: DbUuid encoding undocumented | Module docs expanded (commit `dd3f20388`) |
| No success toast for feedback submission (Bug #4) | Toast via sonner (commit `7cd4b22c9`) |
| PR #27 W1: manual InReview doesn't spawn QA review | spawn_watcher_reviews() standalone (commit `f03dab011`) |
| Notification UUIDs instead of display names | Backend LEFT JOIN for actor_name (2026-03-15) |
| Notification icons not visually distinct | Colored icon circles + actor type badges (2026-03-15) |
| Run Workflow button has no loading indicator | Added Loader2 spinner to data-sources.tsx (2026-03-15) |
| Workflow CRM demo incomplete (no commit/verify) | Extended with Parts 5-8: approve, runs, contacts, pipeline (2026-03-15) |
| E2E demo test run (Backlog #5) | Health checks 40/40, demos 28/31, structured test commands (PR #34) |
| Tech debt sprint: .unwrap() elimination | 40 unwraps → proper error handling across 8 files (PR #34) |
| Tech debt sprint: api.ts monolith | 6,669 lines → 21 domain modules (PR #34) |
| Tech debt sprint: org-profile.tsx monolith | 7,086 lines → 33 modular files with lazy-loaded tabs (PR #34) |
| Tech debt sprint: CI pipeline | fmt + clippy + test + lint + types + audit (PR #34) |
| Tech debt sprint: eprintln in prod code | 4 calls → 0, replaced with tracing (PR #34) |
| Modularity Sprint 2: formatters centralization | `formatDate`/`formatCurrency`/`formatCompactNumber`/`parseJsonArray` → `lib/formatters.ts`, 20 duplicates eliminated (PR #37) |
| Modularity Sprint 2: EmptyState adoption | Component adopted in 7 files, 13 inline patterns replaced (PR #37) |
| Modularity Sprint 2: raw fetch() consolidation | 10 consumers migrated to typed API client modules (PR #37) |
| Modularity Sprint 2: nora/tools.rs split | 7,347 lines → 8 modules in `tools/` directory (PR #37) |
| Modularity Sprint 2: workflows.tsx split | 2,418 lines → 9 files in `pages/workflows/` (PR #37) |
| Modularity Sprint 2: WorkflowEditor.tsx split | 2,240 lines → 7 files in `editor/` with barrel re-export (PR #37) |
| Modularity Sprint 2: project-detail.tsx split | 2,293 lines → 9 files in `project-detail/` (PR #37) |
| Modularity Sprint 2: task_server.rs split | 3,457 lines → 9 files in `task_server/` (PR #37) |
| Modularity Sprint 2: brand-guide.tsx split | 1,703 lines → 17 files in `brand-guide/` (PR #37) |
| Modularity Sprint 2: shared query hooks | `useProjectList`, `useOrganizationById`, `useCrmContacts` extracted (PR #37) |
| ACP agents get no MCP servers (Backlog #2) | Already implemented — `load_platform_mcp_servers()` + `default_mcp.json` (2026-03-16) |
| Rename Bug Triage Pipeline (Backlog #10) | → "Feedback Triage Pipeline" in code + E2E tests (2026-03-16) |
| Pipeline enhancement merge + DbUuid fixes | sloperation314 merged, 12 Rust + 4 TS compile fixes (2026-03-16) |
| Pipeline intelligence E2E demo | 5-part 8-stage deal progression demo (2026-03-16) |
| UX Sprint 2: Org-scoped onboarding system | New DB tables, API module, React Query hook, org overview integration (PR #39) |
| UX Sprint 2: Notification quick actions | Source-aware action buttons, dismiss, improved deep-linking (PR #39) |
| UX Sprint 2: Dark mode hardcoded color fixes | CSS `--brand-gold` variable, OnboardingCarousel dark variants (PR #39) |
| UX Sprint 2: KPI trend indicators | TrendIndicator + workflow run stats on org overview (PR #39) |
| UX Sprint 2: Project card progress bar | Task completion Progress bar on ProjectCard (PR #39) |
| UX Sprint 2: Workflow last-run status | Status indicator on workflow definition cards (PR #39) |
| UX Sprint 2: Pipeline empty state | Onboarding prompt card in empty pipeline first column (PR #39) |
| UX Sprint 2: Settings "Coming Soon" cleanup | Planned items grouped at bottom with reduced opacity (PR #39) |
| Task.collaborators missing from struct | Field existed in SQL + DB but not in Task struct — SQLx silently discarded (PR #39) |
| Notification quick action buttons (Backlog P1.5) | Implemented: source-aware actions, dismiss, deep-linking (PR #39) |
| Project task count auto-refresh (Backlog #12) | Sidebar task count refresh on create/update (PR #38) |
| UX Sprint 3: Sidebar merged sections (Audit #5) | Management + Global Views → "Views & Management", "More" popover for external links (PR #41) |
| UX Sprint 3: Drawer wider default (Audit #6) | Default width 600→800px in resizable-drawer.tsx (PR #41) |
| UX Sprint 3: Notification tabs + full page (Audit #4) | All/Inbox/Activity tabs in dropdown, `/notifications` page with search/filter (PR #41) |
| UX Sprint 3: Integrations hub redesign (Audit #9) | Recommended/Optional split, progress ring, muted gray for missing optional (PR #41) |
| UX Sprint 3: Task card priority borders (Audit #3) | 3px left-border colored by priority level (PR #41) |
| UX Sprint 3: Hide test tasks toggle (Audit #8) | "Hide Tests" button on kanban filters `[E2E]`/`[Test]` prefix tasks (PR #41) |
| UX Sprint 3: CRM sub-nav dedup (Audit #19) | CRM removed from project sub-nav in ProjectFolder.tsx (PR #41) |
| AgentWatcherPanel error state (Backlog #6) | Error UI with retry button, distinguishes empty vs error (PR #41) |
| Agent profile page (Backlog #8) | `/agents/:agentId/profile` page with capabilities, status, model (PR #41) |
| Route conflict fix (pre-existing) | Merged duplicate routes in org_onboarding.rs (committed to main) |
| Migration version collision fix (pre-existing) | Renamed 20260405000000→20260405000001 to avoid collision (committed to main) |
| Modularity Sprint 5: VIBE billing duplication | `ensure_vibe_balance` + `record_llm_vibe_usage` in `helpers/billing.rs`, 5 blocks eliminated (PR #43) |
| Modularity Sprint 5: topsi raw fetch() | `topsi.ts` API module wrapping 7 endpoints + `topsiKeys` factory (PR #43) |
| Modularity Sprint 5: topiclips raw fetch() | `topiclips.ts` API module wrapping 7 endpoints + `topiclipsKeys` factory (PR #43) |
| Modularity Sprint 5: twilio.rs monolith | 2285 lines → 8-file `twilio/` directory module (PR #43) |
| Modularity Sprint 5: task_attempts.rs monolith | 1909 lines → 6-file `task_attempts/` directory module (PR #43) |
| Modularity Sprint 5: nora/mod.rs decomposition | 1172→718 lines, extracted config.rs, rate_limiter.rs, initialization.rs (PR #43) |
| Modularity Sprint 5: conversation persistence | `helpers/conversations.rs` with `persist_chat_exchange`, adopted in topsi + nora chat (PR #43) |
| Modularity Sprint 5: query key migration (8 files) | ~43 inline keys → factories, new `socialKeys`/`networkKeys` factories (PR #43) |
| Workflow UX: Inline staging review | `RunAndReviewPanel` composite, `RunWorkflowDialog` `onRunComplete` callback (PR #44) |
| Workflow UX: Shared workflow card grid | `WorkflowCardGrid` with ownership badges, used in BuilderTab + WorkflowsView (PR #44) |
| Workflow UX: Copy/promote workflows | `CopyWorkflowDialog` for user ↔ org workflow copying (PR #44) |
| Workflow UX: Node picker search + categories | Search/filter, category headers with counts (PR #44) |
| Workflow UX: Workflow ID auto-generation | ID derived from name with lock/unlock toggle (PR #44) |
| Workflow UX: Graph view zoom + selection | Zoom controls, node selection highlight, connection validation (PR #44) |
| Workflow UX: Prompt templates + output schema | 3 starter templates, schema Select dropdown (PR #44) |
| Workflow UX: Sidebar workflow sub-links | Builder/Runs/Staging sub-links with pending count badge (PR #44) |
| Workflow UX: Real-time updates | AgentWatcher → React Query 10s polling, staging `refetchInterval` (PR #44) |
| Workflow UX: Deleted source display | "(Deleted source)" instead of truncated UUIDs in runs (PR #44) |
| Workflow UX: `any` type cleanup + code quality | `WorkflowRunResult` interface, `cn()`, `useEffect` fixes (PR #44) |
| Modularity Sprint 4: billing helper deferred | Done in Sprint 5 — `helpers/billing.rs` (PR #43) |
| Onboarding: 4 sequential blocking modals | Unified WelcomeWizard + VITE_SKIP_ONBOARDING env var (PR #48) |
| SSE pre-auth console errors | useAgentDirectory `enabled` option (PR #48) |
| `: any` types (107 in org-profile/workflow) | 78 eliminated, 29 remaining RJSF/dynamic (PR #48) |
| Path<Uuid> non-CRM routes (311) | 231 converted to Path<String>, 80 remaining (PR #48) |
| Uuid::parse_str in route handlers (~200) | Replaced with DbUuid::parse across 33 files (PR #48) |
| Inline query keys (89) | 74 migrated to factories, 15 remaining (PR #48) |
| Dead monolith files (project-tasks, MeetingMode) | Deleted -2,205 lines (PR #48) |
| Query key mismatches (brandProfile, workflowTemplates) | Fixed cache invalidation bugs (PR #48) |
| Orphaned project-level CRM routes (7) | Commented out in App.tsx (PR #48) |
| Rust warnings (9 unused imports/vars in server+db) | Cleaned up (PR #48) |
| PR #62 C1: pipeline_events SSE auth bypass | Added AccessContext + require_org_membership (PR #62) |
| PR #62 C2: stage_transition "complete" typo | Fixed → "completed" in 2 places (PR #62) |
| PR #62 H1/H2: agent_flow_executor race conditions | Status guards on complete_flow/fail_flow (PR #62) |
| PR #62 H3: data_source_workflows missing org auth | Added require_org_membership to run_workflow + list_recent_artifacts (PR #62) |
| PR #62 H4: AgentFlowSummary current_phase crash | Made optional (PR #62) |
| PR #62 H5: OverviewTab unsafe `as string` cast | Replaced with typeof guard (PR #62) |
| PR #63 C1/C2: intake table name mismatch | person_organization_contacts → contact_organization_links (PR #63) |
| PR #63 C3: note update/delete no ownership check | Added require_contact_org_access pre-check (PR #63) |
| PR #63 H1/H2: trigger_contact_research/get_intel_status unauthed | Added org membership check via organization_id field (PR #63) |
| PR #63 H3: contact_note.rs author_id DbUuid.map() compile error | Fixed: Some(user_id.to_uuid()) (PR #63) |
| PR #63 M1: stage_transition "complete" typo (inherited) | Fixed → "completed" in 2 places (PR #63) |
| PR #63 M2: ContactResearchPass/SocialProfile/Note missing TS export | Added #[derive(TS)] + #[ts(export)] (PR #63) |
| PR #63 L1-L5: intelligence.rs let _ = silent DB failures | Replaced with if let Err(e) + tracing::warn! (PR #63) |

---

## CI Infrastructure (2026-03-19)

### Rust `cargo fmt` — 200+ Files Need Formatting
**Source:** CI `backend-fmt` job failure (pre-existing on main)
**What:** `cargo fmt --all -- --check` reports diffs in 200+ files across all crates. The import ordering change in `apn-app/src-tauri/src/main.rs` is the simplest, but running `cargo fmt --all` touches files in `alpha-protocol-core`, `discord-bots`, `nora`, `services`, etc.
**Recommendation:** Run `cargo fmt --all` on a dedicated branch and commit as a single formatting-only commit. Do NOT mix with feature work.
**Status:** NOT STARTED — too large for PR #50 scope

### Rust Clippy — 29+ Errors in `discord-bots`, `utils`, `pcg-cli`
**Source:** CI `backend-clippy` job (pre-existing on main)
**What:** `uninlined_format_args`, `useless_conversion`, `manual_find`, and other clippy warnings across non-core crates. Core crates (`server`, `db`) can't be clippy'd in isolation due to deep dependency chains.
**Fix applied (PR #50):** `continue-on-error: true` on clippy CI job so it doesn't block PRs.
**Recommendation:** Fix crate-by-crate: `discord-bots` (29 errors), `utils` (23), `pcg-cli` (65), `alpha-protocol-core` (compile errors). Consider `#![allow(clippy::...)]` at crate root for non-critical lints.
**Status:** DEFERRED — `continue-on-error` is a temporary bridge

### Rust Tests — Compile Errors in `alpha-protocol-core`
**Source:** CI `backend-test` job (pre-existing on main)
**What:** `alpha-protocol-core` has 2 compile errors preventing `cargo test --workspace`. Borrow checker issue in test code.
**Fix applied (PR #50):** `continue-on-error: true` on test CI job.
**Recommendation:** Fix the 2 compile errors in `alpha-protocol-core` test module.
**Status:** DEFERRED

### ESLint — 62 Errors, 180 Warnings (Pre-existing)
**Source:** CI `frontend-check` job, local `npm run lint`
**What:** Mostly `@typescript-eslint/no-explicit-any` (180 warnings, threshold 110) and unused disable directives (62 errors). All pre-existing, none from PR #50.
**Recommendation:** Run `eslint --fix` to clear unused disable directives. Apply stashed lint-staged setup (see P2.5 section). Raise `--max-warnings` threshold or fix remaining `: any` types.
**Status:** NOT STARTED

---

## PR #50 — Deferred Items

### Dead Code Cleanup: companies.rs run_company_research
**Source:** PR #50 QA regression review (2026-03-18)
**What:** `run_company_research()` and helpers (`extract_summary`, `extract_confidence`, `extract_text_from_response`) are dead code — replaced by `intelligence::run_company_research_direct()` in sloperation317 port. Currently annotated with `#[allow(dead_code)]`.
**Recommendation:** Delete after confirming `run_company_research_direct` covers all use cases. Check if any other code calls the old function.
**Status:** DEFERRED — commented out, not blocking

### Hardcoded Operator Assignment in CRM Deals
**Source:** PR #50 QA regression review (2026-03-18)
**What:** `create_review_task_if_needed` has hardcoded username-to-org mapping (Sirak → "Sirak", PowerClub/PCG → "Bodhi"). Breaks if org names or usernames change.
**Recommendation:** Move to a configurable mapping (org_settings table or env var).
**Status:** DEFERRED — working as designed for current orgs

### Intake Pipeline — Silent DB Failures (10 `let _ =` locations)
**Source:** PR #63 regression audit (2026-04-01)
**What:** 10 `let _ =` assignments in `crates/server/src/routes/intake/pipeline.rs` silently drop database errors: company description updates, contact_organization_links inserts, assigned_agent_id updates, company_id links, intelligence_status updates, knowledge_source inserts, deal-to-intake links. Pre-existing on main; not introduced by this branch.
**Recommendation:** Replace each `let _ =` with `if let Err(e) = ... { tracing::warn!(...) }` to surface pipeline failures for debugging.
**Status:** DEFERRED — pre-existing, not blocking

### BLOB Column uuid::Uuid Usage in Pre-existing Code
**Source:** PR #50 QA regression review (2026-03-18)
**What:** 5 pre-existing functions still use `uuid::Uuid::parse_str()` for BLOB column binding: `trigger_who_is_research`, `trigger_company_research_if_idle`, `generate_phase1_business_report`. These should use `DbUuid::parse().to_uuid()` pattern.
**Recommendation:** Convert in a dedicated DbUuid batch migration sprint.
**Status:** DEFERRED — functionally correct, style debt only

### Company Profile Page Rendering Gap
**Source:** PR #50 E2E test observations (2026-03-18)
**What:** `/companies/:id` page loads with correct URL but doesn't display the company name or data in the main content area. Sidebar renders correctly. May be a missing data fetch or component rendering issue.
**Recommendation:** Investigate CompanyProfilePage data loading — check if it queries by TEXT or BLOB ID.
**Status:** DEFERRED — documented via test.fixme() in quarantine tests

---

## Modularity — Backend (from Phase 0 Sprint research, 2026-03-19)

### Env Var Helpers
**What:** 96+ scattered `std::env::var()` calls across 37 route files → shared `get_env_required()`/`get_env_optional()` in `crates/utils/src/config.rs`. `quickbooks.rs` already has good local pattern to follow.
**Effort:** 45-60 min + migration across files
**Status:** NOT STARTED

### Model Error Macro
**What:** 30+ files with identical `#[derive(Debug, Error)]` boilerplate → `define_model_error!` macro in `crates/db/src/lib.rs`.
**Effort:** 30 min + migration
**Status:** NOT STARTED

### Inline FromRow Cleanup
**What:** 83+ inline `#[derive(sqlx::FromRow)]` structs in route files → shared types in `crates/db/src/query_results.rs` (`UuidRow`, `CountRow`, `OptionalIdRow`).
**Effort:** 2 hours
**Status:** NOT STARTED

### DB Error Wrapping Trait
**What:** 1,060+ `ApiError::InternalError(format!("..."))` conversions → `DbErrorExt` trait with consistent logging.
**Effort:** 50 min + migration
**Status:** NOT STARTED

### Kanban Deal Enrichment Dedup
**Source:** Contacts unification Phase 1 (2026-03-27)
**What:** `crm_deal.rs` has the same deal enrichment logic duplicated between `get_kanban_data()` (~line 900) and `find_by_id_with_contact()` (~line 1075). Both do: contact lookup → person_id resolution → report_id → fetch_intel_data → company intel → active agent flow. Extract to a shared `enrich_deal_with_contact(pool, deal, contact_info) → CrmDealWithContact` helper.
**Effort:** 30 min
**Status:** NOT STARTED

### Large Backend File Splits
**What:** Files beyond `crm_deals.rs` that exceed 1,000 lines:
- `sovereign_storage.rs` (2,699L) — extract conflict resolution
- `brand.rs` (1,863L) → settings, guidelines, voice
- `tasks.rs` (1,572L) → crud, bulk ops, dependencies
- `intelligence.rs` (1,443L) → artifact index, entity resolution, topology
- `workflow_staging.rs` (1,401L) → validation, commit handlers
- `meet.rs` (1,372L) → sessions, participants
**Effort:** 2-4 hours per file
**Status:** NOT STARTED

---

## Modularity — Frontend (from Phase 0 Sprint research, 2026-03-19)

### useAsyncModalAction Hook
**What:** 72 NiceModal dialogs share identical loading/error state pattern (~20 lines each). Extract to shared hook.
**Effort:** 1.5 hours
**Status:** NOT STARTED

### LoadingButton Component
**What:** 20+ dialog submit buttons duplicate disabled+spinner logic. Create `<LoadingButton>` in `components/ui/`.
**Effort:** 45 min
**Status:** NOT STARTED

### EmptyState Component
**What:** Inconsistent empty state rendering across components. Create standardized `<EmptyState>` component.
**Effort:** 30 min
**Status:** NOT STARTED

### localStorage Typed Wrapper
**What:** 63 raw `localStorage` calls with magic string keys → typed `appStorage` object in `lib/storage.ts`.
**Effort:** 1.5 hours
**Status:** NOT STARTED

### Validation Utilities
**What:** 20+ inline `required` field checks with `toast.error()` → shared `validators` module in `lib/validation.ts`.
**Effort:** 1 hour
**Status:** NOT STARTED

### Spinner Standardization
**What:** Mixed `Loader`/`Loader2`/`RefreshCw`/custom CSS spinners → standardize on existing `components/ui/loader.tsx`.
**Effort:** 1 hour
**Status:** NOT STARTED

### Stream Hook Backoff Utility
**What:** 4 hooks (`useJsonPatchStream`, `useJsonPatchWsStream`, `useLogStream`, `useDiffStream`) duplicate exponential backoff retry logic.
**Effort:** 1-2 hours
**Status:** NOT STARTED

### Query/Mutation Hook Factory
**What:** Simple CRUD hooks repeat identical `useQuery`/`useMutationWithToast` boilerplate → factory functions.
**Effort:** 2 hours
**Status:** NOT STARTED

### Large Frontend Component Splits
**What:** Components exceeding 800 lines:
- `NoraAssistant.tsx` (1,027L) → conversation, tool execution, controls
- `StagingReviewPanel.tsx` (933L) → diff viewer, commit panel
- `topsi.tsx` (937L) → meeting mode, controls, chat
- `AgentSettings.tsx` (918L) → model config, capabilities, test panel
- `crm.tsx` (898L) → pipeline view, filters, deal management
- `TaskDetailsPanel.tsx` (837L) → artifacts tab, execution tab, collaboration
- `WelcomeWizard.tsx` (783L) → individual step components
**Effort:** 2-4 hours per component
**Status:** NOT STARTED

---

### Pipeline Stage Config Visual Builder (Option B) [ROI: HIGH]

**Source:** 2026-03-23 pipeline settings audit — Playwright-verified gap
**What:** Build a visual editor for `on_enter_actions` and `on_exit_validations` in the Stage Config dialog, replacing the current situation where these can only be set via SQL migrations.

**Rationale:**

The CRM pipeline's behavior is driven by two JSON arrays in `stage_config`:
- `on_enter_actions`: what happens when a deal enters a stage (trigger agent, create review task, create delivery deal)
- `on_exit_validations`: what must be true before a deal can leave (require field, require intel, require pending tasks = 0)

Currently:
- **UI** exposes 5 simple fields (`assigned_agent`, `auto_trigger`, `cancel_window_secs`, `required_fields`, `approval_gate`) — these are now wired to the backend via Option A (synthesizes effective actions/validations from simple fields when explicit arrays are empty).
- **Migrations** set the full `on_enter_actions` / `on_exit_validations` arrays with rich logic (chained agents, intel requirements, etc.) that the simple UI fields can't express.
- **Admins** can see existing automations read-only in the Stage Config editor but cannot add, remove, or reorder them via UI.

This means:
1. **New pipelines** can only get basic automations (single agent trigger, field requirements, approval gate) unless someone writes SQL.
2. **Existing pipeline automations** can't be modified without DB access.
3. **Complex chains** (e.g., "trigger Astra → on completion trigger Cash → create review task") can't be composed via UI.

**Implementation (estimated 3-5 days):**

1. **Action Builder component** — drag-and-drop or ordered list of actions:
   - `TriggerAgent`: agent selector + flow_type selector + auto_trigger toggle
   - `CreateReviewTask`: description text input
   - `CreateDeliveryDeal`: no params (shown as a toggle)
   - Add/remove/reorder via up/down buttons

2. **Validation Builder component** — ordered list of exit gates:
   - `RequireField`: field selector (from DEAL_FIELDS) + custom message input
   - `RequireIntel`: entity selector (person/company) + status selector (done/complete)
   - `RequirePendingTasks`: count input (default 0 = all tasks must be done)
   - Add/remove/reorder

3. **Stage Owner editor** — label + type (agent/human/team) inputs

4. **Preview panel** — shows the effective JSON that will be saved, so admins can verify

5. **Migration path** — Option A's synthesis logic (`build_effective_entry_actions` / `build_effective_exit_validations`) remains as fallback for stages with simple configs. Stages with explicit action arrays bypass synthesis.

**Dependencies:** None — Option A is already shipped and provides the backend wiring. This is purely a frontend enhancement.
**Effort:** 3-5 days | **Sprint:** Phase 1 | **Status:** NOT STARTED

---

### Pipeline ↔ Workflow Shared Services Extraction [ROI: HIGH]

**Source:** 2026-03-23 pipeline settings audit — architectural analysis of CRM Pipeline vs Workflow Builder overlap
**What:** Extract duplicated infrastructure from both systems into shared service modules, then add a `TriggerWorkflow` stage action to connect them.

**Rationale:**

The CRM Pipeline (`stage_transition.rs`, `crm_deal_automations.rs`) and Workflow Builder (`workflow/orchestrator.rs`, `workflow_engine.rs`, `workflow_triggers.rs`) solve different problems — linear deal funnel vs batch DAG extraction — but independently implement the same infrastructure:

| Shared Concern | Pipeline Implementation | Workflow Implementation |
|---|---|---|
| Agent flow spawning | `schedule_agent_flow()` in `stage_transition.rs` | Executor spawns via `agent_flows` table |
| LLM dispatch | `call_llm()` in `crm_deal_automations.rs` (OpenAI → Claude fallback) | LLM node in workflow executor (same fallback) |
| Cancel windows | `cancel_deadline` on `agent_flows`, checked in `cancel_agent_flow()` | Same `cancel_deadline` column, separate cancel logic |
| Review/approval gates | Creates tasks, blocks on `pending_tasks == 0` | Stages records, blocks on `status == 'approved'` |
| Cost tracking | `agent_flows.usage_token_count` | `workflow_run.usage` (aggregated from node_results) |

This duplication means bug fixes in one system don't propagate to the other, and the two systems can drift apart on behavior (e.g., different LLM fallback order, different cancel window clamping).

**Why not merge the systems:**

They have fundamentally different execution models:
- Pipeline: single-item, synchronous, linear (position N → N+1), human-triggered
- Workflow: batch-item, async, DAG (fan-out/fan-in), event-triggered (webhook/cron/data source)

Merging would force either unnecessary DAG complexity on the pipeline or CRM-specific concepts (review tasks, deal artifacts) into the workflow builder. The right boundary is: **workflows produce CRM records, pipelines consume them**.

**Implementation (estimated 5-7 days):**

**Phase 1: Extract shared services (3-4 days)**

Create `crates/services/src/` modules:

1. **`agent_dispatch.rs`** — Unified agent flow spawning
   - `schedule_agent_flow(pool, entity_id, agent, flow_type, cancel_window_secs) → (flow_id, deadline)`
   - `cancel_agent_flow(pool, entity_id) → bool`
   - Both systems call this instead of their own implementations
   - Single place to enforce cancel window clamping, known-agent validation, dedup

2. **`llm_client.rs`** — Unified LLM call with provider fallback
   - `call_llm(config: LlmCallConfig) → Result<String>` with: system prompt, user message, model preference, max_tokens, timeout
   - Fallback chain: preferred provider → OpenAI → Anthropic → Ollama (configurable)
   - Single place to add retry logic, rate limiting, cost recording
   - Replaces `call_llm()` in `crm_deal_automations.rs` and LLM node dispatch in workflow executor

3. **`cost_tracker.rs`** — Unified token usage recording
   - `record_usage(pool, context: UsageContext, tokens: TokenUsage)`
   - Context: agent_flow_id OR workflow_run_id OR deal_id
   - Single aggregation point for the AI Usage dashboard

4. **`approval_gate.rs`** — Shared review/approval pattern
   - `create_review_task(pool, entity_id, entity_type, description) → task_id`
   - `check_pending_approvals(pool, entity_id, entity_type) → count`
   - Abstracts over CRM review tasks and workflow staging approval

**Phase 2: Pipeline → Workflow trigger (2-3 days)**

1. Add new `StageAction` variant:
   ```rust
   StageAction::TriggerWorkflow {
       workflow_id: String,
       input_mapping: Option<HashMap<String, String>>, // deal fields → workflow input
   }
   ```

2. In `process_transition()`, when this action fires:
   - Look up the workflow definition
   - Create a workflow run with deal data mapped to input nodes
   - Fire asynchronously (don't block the stage transition)

3. Use cases this enables:
   - Deal enters "Intel" → trigger a research workflow (richer than a single agent flow)
   - Deal reaches "Won" → trigger an onboarding workflow on the delivery pipeline
   - Deal gets a new transcript → trigger an extraction workflow to pull contacts/companies

4. Add `TriggerWorkflow` to the `StageConfigEditor` UI (new dropdown: select workflow from org's definitions)

**Phase 3: Workflow → Pipeline feedback (future)**

- Workflow completion fires a callback that can advance a deal or update deal fields
- Enables: "Research workflow completes → auto-advance deal from Intel to BA"
- Requires: event system or webhook callback from workflow orchestrator to pipeline

**Dependencies:** Both systems must be stable (no active refactors). Shared services are pure extraction — no behavioral changes.
**Effort:** 5-7 days | **Sprint:** Phase 1-2 | **Status:** NOT STARTED

---

### Pipeline Stage Flow Redesign [ROI: MEDIUM]

**Source:** 2026-03-23 pipeline settings review — manual reordering is dangerous in trigger-based pipelines
**What:** Replace manual up/down stage reordering with a flow-oriented design. Define explicit allowed transitions instead of implicit position+1 advancement.

**Rationale:**

The current pipeline uses `position` for stage ordering, and `advance_deal()` simply moves to `position + 1`. This has two problems:

1. **Manual reordering breaks automations.** If an admin swaps Polish and Proposal via the up/down arrows, deals would skip proposal generation and go straight to deck design. The stage order IS the pipeline logic — it's not cosmetic.

2. **No conditional transitions.** The 317 plan describes specific transitions (any stage → Lost, Won triggers delivery deal creation) that can't be expressed as simple position+1. Future pipelines may need branching (if proposal rejected → back to Discovery, if approved → Polish).

**Implementation (estimated 3-5 days):**

**Short term (1 day):**
- Remove up/down reorder buttons from Pipeline Settings → Stages tab
- Add position number badges to stages to show flow order
- Add flow arrows (→) between stages in the list
- Won/Lost shown as special terminal states (visually distinct)

**Medium term (2-4 days):**
- Add `allowed_transitions` field to `StageConfig`:
  ```json
  {
    "allowed_transitions": ["next", "lost"],
    "back_transitions": ["previous"]
  }
  ```
- Backend enforces: `advance_deal()` checks `allowed_transitions` instead of blindly going to position+1
- UI: "Move to..." context menu only shows allowed target stages
- Special transitions: "Mark Lost" always available, "Back to [stage]" for revision flows

**Long term (backlog):**
- Visual pipeline designer: horizontal flow with connected nodes
- Conditional transitions: if field X == Y → go to stage A, else stage B
- Parallel stages: some stages could run concurrently (e.g., Polish and Invoice prep simultaneously)

**Dependencies:** Pipeline Stage Config Visual Builder (Option B) for the long-term visual designer.
**Effort:** 1 day (short) + 2-4 days (medium) | **Sprint:** Phase 1 | **Status:** NOT STARTED

---

## E2E Pipeline Flow Timing (2026-03-27)

**Bug**: `pipeline-flow.spec.ts:212` — "move deal to Proposal (setup for DD-2)" fails because the kanban board SSE update doesn't arrive within 10s after an API-driven stage transition.
- The deal is moved via `PATCH /api/crm/deals/:id/stage`, and the API confirms the stage change
- But `getByTestId('stage-column-proposal').getByText(dealName)` times out — the board UI doesn't reflect the move
- Likely cause: stage transition triggers agent automations (auto-skip, BA agent) which further advance the deal before SSE fires, or the SSE event is delayed/not emitted for API-driven transitions
- **Affects**: tests 6-9 in pipeline-flow (serial dependency cascade)
- **Category**: E2E test timing / SSE reliability
- **Resolution**: Either increase timeout, add page reload after API transition, or ensure SSE emits on API-driven stage changes
- **Effort**: 0.5 day | **Status:** NOT STARTED

# Phase 0 Sprint Candidates — ROI-Ordered Prioritization

**Date**: 2026-03-19
**Branch**: `research/5-year-product-roadmap`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Context**: Synthesized from 5-year roadmap v1.1, architecture-gaps-analysis, 45-item research-derived backlog, existing BACKLOG--remaining-work.md, and 27 research reports.

---

## ROI Scoring Methodology

**ROI = (Revenue Impact × Probability of Success) / Effort**

- **Revenue Impact** (1-10): How much this accelerates Stage 0 exit → Stage 1 revenue
- **Probability of Success** (0.5-1.0): How likely to succeed given existing codebase, team of 3 + Claude Code
- **Effort** (days): Calendar days for a senior dev + Claude Code at 90% gen rate
- **Existing Foundation**: Work already done that reduces effort (critical for accurate ROI)
- **Dependency Score**: How many downstream items this unblocks

Items are ordered **strictly by ROI score**. No grandfathered priorities — everything re-evaluated from scratch.

### Dogfooding = Client Discovery + Product Development

Phase 0's core thesis: **using ORCHA daily for PCG's real agency work is both the product development methodology AND the client discovery process.** Every friction point the team hits while running Editron jobs, managing social media pipelines, and tracking CRM deals through ORCHA is a product insight that would cost months to discover through external user interviews.

This means Phase 0 items serve dual purpose:
1. **Revenue capture** — moving existing agency work onto ORCHA to demonstrate platform value
2. **Product discovery** — finding UX gaps, workflow limitations, and missing features through daily use

Items that accelerate daily dogfooding (stability, cost visibility, workflow reliability) have outsized ROI because they generate compounding product insights. Items that make the platform *usable enough for real client work* are strictly higher priority than items that make it *architecturally elegant*.

The Agent Flow Engine (#S0-06) is the keystone: without autonomous agent progression, the team manually shepherds every agent task — which defeats the purpose of an orchestration platform. Cost visibility (#S0-02, #S0-07) is second: the team can't optimize what they can't measure, and CAPO < $1 is the proof point for pre-seed conversations.

**Client Discovery Through Dogfood**: Every complete Editron delivery cycle and social media campaign run through ORCHA generates:
- UX friction logs → prioritized feature backlog
- Cost data → CAPO benchmarks for pitch deck
- Workflow completion metrics → reliability evidence
- Agent success/failure patterns → orchestration improvements
- Real deliverables → portfolio proof of concept

---

## Tier 1 — Highest ROI (Do First)

### S0-01. CI Strictness Fix
**ROI Score: 50.0** | Impact: 8 | P(success): 1.0 | Effort: 0.5 days
**What:** Remove `continue-on-error: true` from clippy and test steps in `.github/workflows/ci.yml`. Currently broken code can merge to main.
**Existing Foundation:** One-line change per step. The steps already run — they just don't block merge on failure.
**Why highest ROI:** Near-zero effort, prevents every future regression. Every other sprint item is wasted if broken code can silently merge. This is the "fix the safety net before doing trapeze acts" item.
**Unblocks:** All stability work. Stage 0 exit criterion "zero critical crashes in 2-week window."
**Source:** [`roadmap/research/04-infra--cicd-gaps.md`](roadmap/research/04-infra--cicd-gaps.md), [`roadmap/architecture-gaps-analysis.md` §GAP-XC-03](roadmap/architecture-gaps-analysis.md)
**Backlog Ref:** NEW (from research)

### S0-02. Bridge Dual Cost System (Fix $0 Dashboard)
**ROI Score: 18.0** | Impact: 9 | P(success): 1.0 | Effort: 1 day
**What:** `ai-usage.tsx` reads `TokenUsage.cost_cents` which is never populated at INSERT time. Meanwhile, `VibeTransaction` records real costs via `ModelPricing.calculate_cost()`. Fix: either populate `cost_cents` during `TokenUsage::create()`, or rewrite dashboard to query `VibeTransaction` aggregates.
**Existing Foundation:** Both cost systems exist and work independently. VibeTransaction already has correct cost data. This is a wiring fix, not a build.
**Why high ROI:** Without visible cost data, CAPO tracking (Stage 0 exit criterion) is impossible. Every cost optimization item downstream depends on seeing real numbers. CAPO < $1/action can't be validated if the dashboard shows $0.
**Unblocks:** CAPO tracking (#S0-07), cost-tiered routing (#S0-08), all cost economics work.
**Source:** [`roadmap/architecture-gaps-analysis.md` §GAP-S0-03](roadmap/architecture-gaps-analysis.md)
**Backlog Ref:** NEW (from research)

### S0-03. Structured Response Protocol (Agent Envelope)
**ROI Score: 13.5** | Impact: 9 | P(success): 0.9 | Effort: 1.5 days
**What:** Define a standard JSON response envelope for agent-to-orchestrator communication: `{ status: "Done|NeedsClarification|Failed", message, artifacts, clarification_needed }`. Currently agents return freeform conversation text.
**Existing Foundation:** Agent flow data model exists (`agent_flow.rs` 406 lines, `agent_flow_event.rs` 350 lines). Response types can be added as enum variants + serde. ACP harness already parses some agent output.
**Why high ROI:** This is the *interface contract* that all agent orchestration builds on. Without it, the Agent Flow Engine (#S0-06) has no structured way to determine agent outcomes. Highest-ROI pattern from RooRoo research. Must be defined BEFORE building the background worker.
**Unblocks:** Agent Flow Engine (#S0-06), Clarification Status (#S0-04), all Stage 1 agent orchestration.
**Source:** [`roadmap/research/10-ai--agent-orchestration.md`](roadmap/research/10-ai--agent-orchestration.md), research-derived-backlog #1
**Backlog Ref:** Research backlog #1

### S0-04. Clarification as First-Class Status
**ROI Score: 12.0** | Impact: 8 | P(success): 0.9 | Effort: 1 day
**What:** Agents get an explicit `NeedsClarification` response type rather than embedding ad-hoc questions in freeform text. When an agent encounters uncertainty, it returns a typed clarification request that bubbles up to the user or parent orchestrator.
**Existing Foundation:** Builds directly on the response envelope (#S0-03). One additional enum variant + UI handling for the "agent is asking a question" state.
**Why high ROI:** Prevents cascading errors from incorrect agent assumptions. The "Principle of Least Assumption" — agents clarify, never guess. Without this, agents guess wrong and burn tokens on wasted work.
**Unblocks:** Reliable agent delegation, human-in-the-loop workflows.
**Source:** [`roadmap/research/10-ai--agent-orchestration.md`](roadmap/research/10-ai--agent-orchestration.md), research-derived-backlog #4
**Backlog Ref:** Research backlog #4

### S0-05. Cooldown Race Condition Fix
**ROI Score: 10.0** | Impact: 5 | P(success): 1.0 | Effort: 0.5 days
**What:** Cooldown check reads `last_triggered_at`, then the caller updates it — but concurrent requests can both pass the check before either updates. Fix: atomic `UPDATE ... WHERE last_triggered_at < ? RETURNING *`.
**Existing Foundation:** Single-statement SQL change in `workflow_trigger.rs`. Pattern well-known (optimistic locking).
**Why high ROI:** Prevents double-fire of workflows under concurrent webhooks. Low effort, high reliability impact. Required for Stage 0 production use.
**Unblocks:** Reliable workflow triggers for Editron and social media pipelines.
**Source:** [`roadmap/architecture-gaps-analysis.md` §GAP-S0-01](roadmap/architecture-gaps-analysis.md), BACKLOG #4 (PR #49 review)
**Backlog Ref:** BACKLOG P2 "Webhook Cooldown Race Condition"

---

## Tier 2 — High ROI (Sprint 1, Weeks 1-2)

### S0-06. Agent Flow Orchestration Engine (Background Worker)
**ROI Score: 9.0** | Impact: 10 | P(success): 0.9 | Effort: 5 days
**What:** Build the background worker that progresses agent flows through phases, enforces gates, handles delegation and retry. This is the #1 architectural gap and a Stage 0 exit criterion.
**Existing Foundation:**
- Data model complete: `agent_flow.rs` (406 lines), `agent_flow_event.rs` (350 lines), `agent_task_plan.rs` (250 lines)
- Phase enums (Planning, Executing, Verifying) and CRUD routes mounted
- Proven pattern: `spawn_workflow_schedule_loop()` in `data_source_workflows.rs` (tokio interval + DB polling)
- Dead `TaskScheduler` (310 lines) has agent-task monitoring logic that can be adapted
- Response envelope (#S0-03) provides the structured interface
**Why critical:** Without this, agent flows are a data model with no engine. Stage 0 exit requires "Agent Flow Orchestration Engine functional." This is the single largest remaining build.
**Approach:** Tokio interval worker (like workflow schedule loop), poll `agent_flows` with `status = 'active'`, check current phase, dispatch to executor, parse structured response, advance or retry.
**Unblocks:** Stage 0 exit, all Stage 1 agent orchestration, pilot client delivery.
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 1](roadmap/5-year-product-roadmap.md), [`roadmap/architecture-gaps-analysis.md` §GAP-S0-04](roadmap/architecture-gaps-analysis.md)
**Backlog Ref:** BACKLOG P0 #1

### S0-07. CAPO Per-Task Tracking
**ROI Score: 8.0** | Impact: 8 | P(success): 0.8 | Effort: 2 days
**What:** Instrument every agent invocation to track: input tokens, output tokens, model used, wall-clock time. Calculate CAPO per task type: `(inference_cost + retry_cost) / accepted_outcomes`. Store alongside task completion status.
**Existing Foundation:**
- `VibeTransaction` already records per-invocation costs via `ModelPricing.calculate_cost()`
- `TokenUsage` records token counts per invocation
- `ai-usage.tsx` dashboard exists (just needs correct data source after #S0-02)
- Need: link VibeTransaction to task_id, aggregate per task type
**Why high ROI:** Stage 0 exit criterion is "Internal CAPO tracked for all agent actions (target: < $1/action)." Can't exit Stage 0 without this metric. Also required for pre-seed fundraising (proving unit economics work).
**Unblocks:** Stage 0 exit validation, cost-tiered routing optimization, pre-seed fundraise evidence.
**Source:** [`roadmap/research/20-ai--unit-economics-agent-metrics.md`](roadmap/research/20-ai--unit-economics-agent-metrics.md), research-derived-backlog #40
**Backlog Ref:** Research backlog #40

### S0-08. Cost-Tiered Model Routing
**ROI Score: 7.5** | Impact: 9 | P(success): 0.75 | Effort: 3 days
**What:** Route agent tasks to cheapest capable model. Orchestration/routing → Haiku ($1/MTok), analysis → Sonnet ($3/MTok), complex coding → Opus ($5/MTok). RooRoo's Navigator pattern.
**Existing Foundation:**
- `pcg_router.rs` (622 lines) — OpenRouter-compatible LLM routing layer with model listing and chat completions
- `ModelPricing` already has per-model cost data
- Multi-model support exists (9 executors)
- Need: task complexity classifier + routing rules + tier config
**Why high ROI:** 30-85% cost reduction on agent operations. Directly enables CAPO < $1 target. Most orchestration work doesn't need Opus — a Haiku-class router at $1/MTok vs Opus at $5/MTok for dispatch decisions saves 80% on routing tokens.
**Unblocks:** Sustainable agent economics, CAPO target, Stage 1 cost competitiveness.
**Source:** [`roadmap/research/10-ai--agent-orchestration.md`](roadmap/research/10-ai--agent-orchestration.md), research-derived-backlog #2
**Backlog Ref:** Research backlog #2

---

## Tier 3 — Medium-High ROI (Sprint 2, Weeks 3-4)

### S0-09. Append-Only Event Logging (Agent JSONL)
**ROI Score: 6.0** | Impact: 6 | P(success): 0.9 | Effort: 1.5 days
**What:** Lightweight JSONL event log for agent orchestration events (task dispatched, completed, failed, clarification requested). Append-only, corruption-resistant.
**Existing Foundation:** `tracing` crate already in use. Could be as simple as a structured tracing subscriber that writes to a JSONL file or a new `agent_events` table. `execution_artifacts` table exists but stores blobs, not structured events.
**Why medium-high ROI:** Enables debugging agent flows, replay for incident analysis, and audit trails required for enterprise pilots. Without event logs, debugging the Agent Flow Engine is blind.
**Unblocks:** Agent flow debugging, CAPO audit trails, Stage 1 observability.
**Source:** [`roadmap/research/10-ai--agent-orchestration.md`](roadmap/research/10-ai--agent-orchestration.md), research-derived-backlog #6
**Backlog Ref:** Research backlog #6

### S0-10. Prompt Caching Strategy
**ROI Score: 6.0** | Impact: 7 | P(success): 0.8 | Effort: 2 days
**What:** Implement Anthropic prompt caching for agent system prompts and project context. Cache reads cost 10% of standard input (90% savings). Pays for itself after 1 re-use.
**Existing Foundation:** Anthropic API supports cache_control blocks. System prompts are already static text per agent type. Need: structure prompts with cache_control markers, ensure stable prefix ordering.
**Why medium-high ROI:** The orchestrator sends similar context with every agent dispatch. Caching this alone reduces input token costs by 50-70%. Combined with model routing (#S0-08), total savings reach 60-80%.
**Unblocks:** Affordable high-frequency agent orchestration, CAPO < $1 target.
**Source:** [`roadmap/research/20-ai--unit-economics-agent-metrics.md`](roadmap/research/20-ai--unit-economics-agent-metrics.md), research-derived-backlog #42
**Backlog Ref:** Research backlog #42

### S0-11. Vision/Mission Anchor Document
**ROI Score: 5.0** | Impact: 5 | P(success): 1.0 | Effort: 1 day
**What:** Create a formal project vision document that all agents must reference before starting work. Include: project goals, user personas, architectural decisions, design principles, success metrics, scope constraints. Agents detecting contradictions between task and vision must escalate.
**Existing Foundation:** `CLAUDE.md` covers coding conventions but not project vision. The 5-year roadmap has the vision content — this is about distilling it into an agent-readable format that becomes mandatory context.
**Why medium ROI:** Prevents agent drift toward speculative features. Without a formal constraint anchor, agents optimize locally while violating project-level goals. Low effort (writing, not code), high leverage for all future agent work.
**Unblocks:** Agent alignment, consistent agent behavior across all orchestration.
**Source:** [`roadmap/research/12-ai--sudolang-aidd-framework.md`](roadmap/research/12-ai--sudolang-aidd-framework.md), research-derived-backlog #14
**Backlog Ref:** Research backlog #14

### S0-12. Graceful Shutdown
**ROI Score: 4.5** | Impact: 6 | P(success): 0.9 | Effort: 1 day
**What:** Replace bare `axum::serve()` at main.rs line 586 with `axum::serve().with_graceful_shutdown(signal)`. Add drain period for in-flight agent executions and workflow runs.
**Existing Foundation:** Tokio signal handling is standard. The pattern is well-documented. Need: signal handler, drain period, CancellationToken propagation to active tasks.
**Why medium ROI:** Without graceful shutdown, server restarts during agent execution or workflow runs lose state silently. Required for production stability during Stage 0 dogfooding.
**Unblocks:** Reliable production deployments, durable workflow execution.
**Source:** [`roadmap/architecture-gaps-analysis.md`](roadmap/architecture-gaps-analysis.md), [`roadmap/research/04-infra--cicd-gaps.md`](roadmap/research/04-infra--cicd-gaps.md)
**Backlog Ref:** NEW (from research)

### S0-13. Input Validation Framework
**ROI Score: 4.0** | Impact: 5 | P(success): 0.9 | Effort: 1.5 days
**What:** Add `validator` crate for Rust struct validation. Currently only serde deserialization + manual checks. No email format validation, no string length limits, no URL validation on user inputs.
**Existing Foundation:** `validator` crate is a drop-in with `#[derive(Validate)]` on existing serde structs. Start with highest-risk endpoints: auth, CRM contact creation, workflow definitions, webhook URLs.
**Why medium ROI:** OWASP risk — no input validation is a security gap that blocks enterprise pilots. Low effort with `validator` derive macros.
**Unblocks:** Stage 1 security requirements, enterprise pilot readiness.
**Source:** [`roadmap/research/08-legal--compliance-security.md`](roadmap/research/08-legal--compliance-security.md), [`roadmap/5-year-product-roadmap.md` §OSS Leverage](roadmap/5-year-product-roadmap.md)
**Backlog Ref:** NEW (from research)

---

## Tier 4 — Medium ROI (Sprint 3 / Late Phase 0)

### S0-14. Webhook Retry Worker
**ROI Score: 3.5** | Impact: 4 | P(success): 0.9 | Effort: 1 day
**What:** `max_retries`, `retry_count`, and `next_retry_at` fields exist in DB but are never read. Implement a retry check in `spawn_workflow_schedule_loop` that re-executes failed triggers.
**Existing Foundation:** Fields and schema exist. Just needs a polling query + re-execution call in the existing schedule loop.
**Unblocks:** Reliable webhook-triggered workflows for Editron pipeline.
**Source:** BACKLOG P2 "Webhook Retry Logic is Dead Code"
**Backlog Ref:** BACKLOG P2

### S0-15. Editron Workflow Migration
**ROI Score: 3.3** | Impact: 8 | P(success): 0.5 | Effort: 5 days
**What:** Map current Editron post-production jobs from manual/external into ORCHA workflows. Test with 3 real client deliverables.
**Existing Foundation:** 19+ workflow node types available including `http_request`, `llm_extract`, `llm_analyze`, `assign_to_agent`, CRM output nodes. Schedule and webhook triggers operational.
**Why medium ROI:** High impact (captures existing revenue, Stage 0 exit criterion) but lower probability — requires understanding Editron's current manual process and mapping it to workflow nodes. May need `file_upload`/`file_download` nodes not yet built.
**Unblocks:** Stage 0 exit criterion "100% of Editron jobs through ORCHA."
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 2](roadmap/5-year-product-roadmap.md)
**Backlog Ref:** Roadmap Priority 2

### S0-16. Social Media Pipeline Migration
**ROI Score: 2.7** | Impact: 7 | P(success): 0.5 | Effort: 5 days
**What:** Wire content pipeline (compose → review → schedule → publish) to ORCHA workflows. Dashboard for cross-platform analytics.
**Existing Foundation:** Workflow engine can handle the pipeline structure. Social media API integrations may need new executor nodes.
**Unblocks:** Stage 0 exit criterion "100% of social media management through ORCHA."
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 4](roadmap/5-year-product-roadmap.md)
**Backlog Ref:** Roadmap Priority 4

### S0-17. Rate Limiting (tower_governor)
**ROI Score: 2.5** | Impact: 5 | P(success): 0.8 | Effort: 2 days
**What:** Add global and per-tenant rate limiting. Currently only Nora has rate limits (20 req/min chat, 30 req/min voice).
**Existing Foundation:** `tower_governor` or `tower::limit` integrates as middleware. Nora's TokenBucket pattern can be adapted.
**Unblocks:** Stage 1 multi-tenant hardening, DoS protection.
**Source:** [`roadmap/research/25-team--scaling-api-design.md`](roadmap/research/25-team--scaling-api-design.md)
**Backlog Ref:** Roadmap Priority 3

### S0-18. Dogfood Metrics Dashboard
**ROI Score: 2.4** | Impact: 6 | P(success): 0.8 | Effort: 3 days
**What:** Dashboard showing CAPO per task type, agent success rate, first-pass success rate, workflow completion rate, time savings vs. manual process.
**Existing Foundation:** `ai-usage.tsx` exists. VibeTransaction has cost data. After #S0-02 (bridge fix), cost data flows. After #S0-07 (CAPO tracking), per-task metrics exist. This is the UI layer on top.
**Depends on:** #S0-02, #S0-07
**Unblocks:** Stage 0 exit validation, pre-seed fundraise evidence.
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 7](roadmap/5-year-product-roadmap.md)
**Backlog Ref:** Roadmap Priority 7

### S0-19. Dogfood Friction Logging (Client Discovery Instrument)
**ROI Score: 3.0** | Impact: 7 | P(success): 0.9 | Effort: 1.5 days
**What:** Structured friction logging for internal dogfood sessions. When the team hits a pain point during real client work, capture it in a lightweight format: `{timestamp, workflow_step, friction_type, severity, workaround, feature_request}`. Could be a simple feedback form accessible from any ORCHA page (sidebar "Report Friction" button → structured form → stored in existing feedback table with `source: "dogfood"` tag).
**Existing Foundation:** Feedback submission already exists (FeedbackDialog, feedback.rs, Feedback Triage Pipeline). Extend with structured fields for dogfood-specific metadata.
**Why medium-high ROI:** This is the *client discovery engine*. Every friction point captured during real Editron/social media work is a validated product insight. Without structured capture, insights get lost in Slack messages and verbal conversations. This data directly feeds the pre-seed pitch ("we found and fixed 47 friction points in 3 months of daily use").
**Unblocks:** Data-driven product prioritization, pre-seed narrative, Stage 1 onboarding friction reduction.
**Source:** [`roadmap/5-year-product-roadmap.md` §Stage 0](roadmap/5-year-product-roadmap.md) — "Document onboarding friction for future self-service" (Stage 1 entry condition derived from Stage 0 discovery)
**Backlog Ref:** NEW

### S0-20. Editron + Social Media Workflow Templates
**ROI Score: 2.8** | Impact: 7 | P(success): 0.7 | Effort: 3 days
**What:** Pre-built workflow templates for the two primary revenue-generating activities: Editron post-production and social media content pipeline. Template library accessible from workflow creation dialog. Templates provide the skeleton; team fills in client-specific parameters.
**Existing Foundation:** Workflow engine supports all needed node types. `CopyWorkflowDialog` already exists for duplicating workflows. Template concept is just a workflow definition with placeholder variables.
**Why medium ROI:** Accelerates the revenue capture sprints (S0-15, S0-16) by giving the team a starting point instead of building from scratch. Templates also become the foundation for Stage 1 pilot onboarding — "here's how agencies like yours use ORCHA."
**Unblocks:** Faster Editron migration, faster social media migration, Stage 1 template library.
**Source:** [`roadmap/5-year-product-roadmap.md` §Priority 4](roadmap/5-year-product-roadmap.md) — "Template library for recurring content types"
**Backlog Ref:** Roadmap Priority 4 (sub-item)

---

## Existing Backlog Items — Reclassified

Items from `BACKLOG--remaining-work.md` that are NOT Phase 0 candidates, with updated priority rationale:

| Item | Old Priority | New Assessment | Rationale |
|------|-------------|----------------|-----------|
| Unify MCP Config Systems (#3) | P1 | **Phase 0 — Deprioritize** | Confusing for users but doesn't block dogfooding. Internal team knows which config goes where. Stage 1 when onboarding pilots. |
| DbUuid Phase C/D (#7) | P2 | **Phase 0 — Deprioritize** | Code quality, not functional. 80 remaining `Path<Uuid>` work but don't block any features. |
| `http_request` Node Guardrails (#9) | P2 | **Late Phase 0** | Security concern for production but internal dogfood team won't hit URL allowlisting issues. Becomes P1 for Stage 1 pilots. |
| Demo Script Improvements | P1.5 | **Phase 0 — Deprioritize** | Demo narration doesn't affect dogfooding. Important for pre-seed fundraise but that's Month 3. |
| Lint-Staged Setup | P2.5 | **Phase 0 — Worth Doing** | Stashed, ready to apply. 30-min effort prevents formatting drift on all future commits. Apply early. |
| Large Frontend File Splits | P2.5 | **Phase 0 — Deprioritize** | Code quality. Doesn't block features. Continue opportunistically when touching these files. |
| Workflow UX — Deferred Polish | P2.5 | **Phase 0 — Deprioritize** | UX polish for features that work. Stage 1 when pilot UX matters. |
| Seed DB Refresh (#15) | P3 | **Phase 0 — Nice to Have** | Faster manual QA. Low urgency when team of 3 knows the seed state. |
| E2E Test Infrastructure Issues | P2.7 | **Phase 0 — Batch Fix** | Fix as a group in one session. Doesn't block features but improves CI reliability which supports #S0-01. |
| React Router v7 Migration | P3 | **NOT Phase 0** | Console warnings only. Zero functional impact. |

---

## Phase 0 Sprint Plan (12 Weeks / 3 Months)

### Sprint 0.1 (Week 1-2): Foundation & Safety
| # | Item | Effort | Dependencies |
|---|------|--------|-------------|
| S0-01 | CI Strictness Fix | 0.5d | None |
| S0-02 | Bridge Dual Cost System | 1d | None |
| S0-03 | Structured Response Protocol | 1.5d | None |
| S0-04 | Clarification as First-Class Status | 1d | S0-03 |
| S0-05 | Cooldown Race Fix | 0.5d | None |
| S0-11 | Vision/Mission Anchor Document | 1d | None |
| S0-12 | Graceful Shutdown | 1d | None |
| — | Apply lint-staged stash | 0.5d | None |
| **Total** | | **7d** | |

### Sprint 0.2 (Week 3-4): Agent Engine
| # | Item | Effort | Dependencies |
|---|------|--------|-------------|
| S0-06 | Agent Flow Orchestration Engine | 5d | S0-03, S0-04 |
| S0-09 | Append-Only Event Logging | 1.5d | S0-06 (co-develop) |
| S0-13 | Input Validation Framework | 1.5d | None |
| S0-14 | Webhook Retry Worker | 1d | None |
| **Total** | | **9d** | |

### Sprint 0.3 (Week 5-6): Cost Optimization
| # | Item | Effort | Dependencies |
|---|------|--------|-------------|
| S0-07 | CAPO Per-Task Tracking | 2d | S0-02 |
| S0-08 | Cost-Tiered Model Routing | 3d | S0-06 |
| S0-10 | Prompt Caching Strategy | 2d | S0-08 |
| S0-17 | Rate Limiting | 2d | None |
| **Total** | | **9d** | |

### Sprint 0.4 (Week 7-8): Revenue Capture — Editron
| # | Item | Effort | Dependencies |
|---|------|--------|-------------|
| S0-20 | Editron + Social Media Workflow Templates | 3d | S0-06 |
| S0-15 | Editron Workflow Migration (3 real deliverables) | 5d | S0-06, S0-20 |
| S0-19 | Dogfood Friction Logging | 1.5d | None |
| **Total** | | **9.5d** | |

> **Dogfood milestone**: By end of Sprint 0.4, the team should be running ALL Editron jobs through ORCHA. Every delivery cycle generates CAPO data, friction logs, and workflow completion metrics.

### Sprint 0.5 (Week 9-10): Revenue Capture — Social + Metrics
| # | Item | Effort | Dependencies |
|---|------|--------|-------------|
| S0-16 | Social Media Pipeline Migration | 5d | S0-06, S0-20 |
| S0-18 | Dogfood Metrics Dashboard | 3d | S0-02, S0-07 |
| — | E2E test infrastructure batch fix | 2d | S0-01 |
| **Total** | | **10d** | |

> **Dogfood milestone**: By end of Sprint 0.5, BOTH revenue streams (Editron + social) run through ORCHA. Metrics dashboard shows real CAPO data. Team has 4+ weeks of friction logs for product prioritization.

### Sprint 0.6 (Week 11-12): Validate & Prepare Stage 1
| # | Item | Effort | Dependencies |
|---|------|--------|-------------|
| — | Bug fixes from 10 weeks of dogfooding | 3d | All |
| — | Website/hosting service evaluation | 2d | None |
| — | Stage 0 exit criteria validation pass | 2d | All |
| — | Pre-seed fundraise prep (deck + demo + CAPO data) | 3d | S0-18 |
| **Total** | | **10d** | |

> **Exit milestone**: 3+ complete client delivery cycles documented. CAPO < $1 validated. Zero critical crashes in final 2 weeks. Friction log analyzed → Stage 1 feature backlog derived from real usage.

---

## Stage 0 Exit Criteria Mapping

| Exit Criterion | Sprint Item(s) |
|---------------|----------------|
| 100% Editron through ORCHA | S0-15 |
| 100% social media through ORCHA | S0-16 |
| Agent Flow Engine functional | S0-06 + S0-03 + S0-04 |
| 3+ client delivery cycles | S0-15 + S0-16 (real deliverables) |
| CAPO tracked < $1/action | S0-02 + S0-07 + S0-08 + S0-10 |
| Zero critical crashes in 2 weeks | S0-01 + S0-05 + S0-12 |

---

## Research-Derived Items Deferred to Stage 1+

| Research # | Item | Stage | Rationale for Deferral |
|-----------|------|-------|----------------------|
| #3 | Context Isolation | 1 | Needs working agent engine first |
| #5 | Task Briefing Documents | 1 | Optimize after engine works |
| #7 | Task Dependency Graph (DAG) | 1-2 | Flat tasks work for dogfood |
| #8 | Completion Criteria Fields | 1 | Nice-to-have for internal use |
| #9 | Structured Task Delegation | 1 | Optimize after engine works |
| #10 | Knowledge as Entity | 2 | Advanced — needs foundation |
| #11 | Server-Side Gate Enforcement | 1-2 | After FSM lifecycle (#22) |
| #12 | Cross-Task Learning | 2-3 | Advanced — needs knowledge layer |
| #13 | Constraint-Based Agent Specs | 2 | Declarative agent definitions |
| #15 | Phased Agent Workflow | 1 | After basic engine works |
| #16 | Parallel Boundary Detection | 2 | Multi-agent prerequisite |
| #17 | Mandatory Review Agent | 1-2 | After engine + spec pipeline |
| #18 | Structured Spec Artifacts | 1 | Stage 1 quality improvement |
| #19-45 | All remaining | 2+ | Foundation must come first |

---

## Summary

**20 items for Phase 0** across 6 two-week sprints (12 weeks total).

**Critical path:** S0-01 (CI) → S0-03 (Response Protocol) → S0-04 (Clarification) → S0-06 (Agent Engine) → S0-20 (Templates) → S0-15/S0-16 (Revenue Capture) → S0-18 (Metrics Dashboard) → Stage 0 Exit

**Cost path:** S0-02 (Bridge Fix) → S0-07 (CAPO Tracking) → S0-08 (Model Routing) → S0-10 (Prompt Caching) → CAPO < $1 validated

**Discovery path:** S0-19 (Friction Logging) → S0-15 (Editron dogfood) → S0-16 (Social dogfood) → Friction analysis → Stage 1 feature backlog

**Total estimated effort:** ~57 engineer-days across 60 calendar days (with buffer for bug fixes and unplanned work). Feasible for team of 3 + Claude Code at 90% generation rate.

### Why Dogfooding Is the Strategy, Not Just a Phase

Phase 0 is not "build features then test them." It's "use the platform for real work, and let real work tell us what to build." The Editron and social media migrations (#S0-15, #S0-16) are not just feature work — they're the *primary client discovery mechanism*. Every delivery cycle run through ORCHA generates:

1. **CAPO data** proving unit economics (pre-seed pitch evidence)
2. **Friction logs** revealing the actual gaps between v1.0 and product-market fit
3. **Workflow templates** that become the onboarding kit for Stage 1 pilots
4. **Portfolio proof** — real client deliverables produced through the platform

The team should begin attempting Editron workflows through ORCHA from **Week 1**, even before the Agent Flow Engine is complete. Manual agent shepherding during Weeks 1-4 generates friction data that directly informs the engine's design. The friction logging system (#S0-19) ensures these insights are captured structurally, not lost in conversation.

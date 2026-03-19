# ORCHA 5-Year Product Roadmap

**Version**: 1.1
**Date**: 2026-03-19 (updated from codebase verification pass)
**Author**: PCG Founding Team + Claude Research Sprint
**Status**: Internal Planning Document
**Supporting Research**: 27 reports in [`planning/roadmap/research/`](research/) + 12 reference studies in [`planning/roadmap/reference/`](reference/)

> *"Vision to reality as a service."*

---

## Executive Summary

ORCHA is Powerclub Global's sovereign AI orchestration platform — a full-stack dashboard that transforms how creative agencies deliver client work. Over five years, ORCHA will evolve from an internal agency tool into a self-service platform where anyone can turn vision into reality, powered by AI agents, workflow automation, and decentralized compute.

This roadmap is structured around **revenue-first staging**: each stage unlocks the next through validated metrics, not calendar dates. Every decision is filtered through two principles:

1. **Speed as a Habit** — "Why can't this be done sooner?" applied to every decision [^1]
2. **ROI-Ordered Prioritization** — features ranked by (revenue impact × probability of success) / effort

The roadmap spans 6 stages from dogfooding (Q2 2026) to full sovereignty (2031), with a founding team of 3 using Claude Code at 90% code generation rate as a force multiplier.

**Platform as of 2026-03-19** (verified against codebase):
- **20 workspace members** (18 Rust crates + 2 Tauri apps), 120 route modules, 140+ DB models, 215 migrations
- **765 TypeScript/TSX files**, 155K LOC frontend, 68 component dirs, ~60 pages, 15 Zustand stores
- **9 AI executors** (Claude, Gemini, Qwen, OpenCode, Cursor, Duck, ACP, AMP, Codex)
- **4 MCP servers** (TaskServer with 26+ tools, TopsiServer, NoraServer, PulseServer) — [`research/02-arch--backend.md`](research/02-arch--backend.md)
- **19+ workflow node types** with schedule triggers (every 5m to weekly), webhook + data source event triggers
- **Sentry** integrated on both backend and frontend for error tracking
- **VIBE economy** operational internally: VibeTransaction ledger, ModelPricing cost calculation, marketplace with 85% provider credit
- **i18n** framework with 3 locales (en, es, ja)

**Key strategic bets:**
- Agency→SaaS hybrid transition (revenue from day 1)
- VIBE token economy with 2x API markup (high-margin revenue stream)
- Proprietary→Source-Available licensing (D→C strategy)
- Voice-first interaction as core principle
- Replit/Lovable/Render-class hosting with superior AI integration
- Speed over perfection — ship, learn, iterate

---

## Table of Contents

1. [Stage 0: Dogfood & Capture](#stage-0-dogfood--capture)
2. [Stage 1: First Pilots & Pre-Seed](#stage-1-first-pilots--pre-seed)
3. [Stage 2: SaaS Launch & Seed](#stage-2-saas-launch--seed)
4. [Stage 3: Product-Market Fit & VIBE Economy](#stage-3-product-market-fit--vibe-economy)
5. [Stage 4: Growth & Series A](#stage-4-growth--series-a)
6. [Stage 5: Full Sovereignty](#stage-5-full-sovereignty)
7. [Cross-Cutting Concerns](#cross-cutting-concerns)
8. [Dependency Graph](#dependency-graph)
9. [Risk Register](#risk-register)
10. [References](#references)

---

## Stage 0: Dogfood & Capture

**Duration**: Q2 2026 (April–June) — 3 months, monthly resolution
**Theme**: Make ORCHA the operating system for PCG's daily work
**Budget**: $0 additional (bootstrapped, team already full-time)

### Entry Conditions
- [x] Core platform functional (CRM, tasks, workflows, agents)
- [x] 28/31 demo tests passing
- [x] Team committed full-time
- [x] 4 MCP servers operational (TaskServer, TopsiServer, NoraServer, PulseServer)
- [x] 9 executor backends configured (Claude, Gemini, Qwen, OpenCode, Cursor, Duck, ACP, AMP, Codex)
- [x] Workflow schedule triggers running (5m/15m/30m/hourly/daily/weekly intervals)
- [x] Sentry error tracking on both backend and frontend

### Exit Criteria / Targets
- [ ] PCG runs 100% of Editron jobs through ORCHA workflows
- [ ] PCG runs 100% of social media management through ORCHA
- [ ] Agent Flow Orchestration Engine functional (background worker, phase progression, gates)
- [ ] 3+ complete client delivery cycles executed through platform
- [ ] Internal CAPO tracked for all agent actions (target: < $1/action)
- [ ] Zero critical crashes in 2-week window (unwrap elimination on hot paths)

### Month 1 (April 2026): Foundation

**Priority 1 — Agent Flow Orchestration Engine** [ROI: Critical blocker]
Build the background worker that progresses agent flows through phases, enforces gates, handles delegation and retry. This is the #1 architectural gap. Data model is complete (`agent_flow.rs` 406 lines, `agent_flow_event.rs` 350 lines, `agent_task_plan.rs` 250 lines) with phase enums (Planning, Executing, Verifying) and CRUD routes mounted — but **no background worker exists to drive autonomous progression**. — [`research/02-arch--backend.md` §13](research/02-arch--backend.md)
- Implement structured response envelope protocol (status, message, artifacts, clarification_needed) [^2]
- Background task runner modeled on existing `spawn_workflow_schedule_loop()` pattern in `data_source_workflows.rs` (tokio interval + DB polling — already proven for workflow triggers)
- 5-level error handling: retry → fallback → reassign → circuit break → human escalate [^3] — [`research/10-ai--agent-orchestration.md`](research/10-ai--agent-orchestration.md)
- Phase gate enforcement with named triggers (start, complete, block, cancel) [^7]
- Wire up dead `TaskScheduler` (`task_scheduler.rs`, 310 lines, never called from main.rs) or replace with flow-aware equivalent

> **Note**: `TaskScheduler` monitors tasks with `assigned_agent` but is never started. Could be adapted for agent flow progression or deleted in favor of a new flow engine.

**Priority 2 — Editron Workflow Migration** [ROI: Captures existing revenue]
Move Editron post-production jobs from manual/external into ORCHA workflows.
- Map current Editron manual steps to workflow nodes
- ~~Build missing node types needed for Editron (http_request, file operations)~~ `http_request` node already exists (verified in `workflow_execution.rs` lines 2368-2494). 19+ node types available including `llm_extract`, `llm_analyze`, `assign_to_agent`, CRM output nodes — [`architecture-gaps-analysis.md` §GAP-S0-02](architecture-gaps-analysis.md)
- Test with 3 real client deliverables
- If file operations needed: add `file_upload`/`file_download` node types (not yet implemented)

**Priority 3 — Critical Stability & CI Fixes** [ROI: Unblocks dogfooding]
- **P0: Remove `continue-on-error: true`** from clippy and test steps in `.github/workflows/ci.yml` — currently broken code can merge to main — [`research/04-infra--cicd-gaps.md`](research/04-infra--cicd-gaps.md)
- Eliminate unwraps on hot paths (login, CRM pipeline, workflow execution, agent dispatch)
- Add rate limiting to public-facing endpoints (tower_governor) — TokenBucket exists for Nora (20 req/min chat, 30 req/min voice) but no global/per-tenant limits — [`research/25-team--scaling-api-design.md`](research/25-team--scaling-api-design.md)
- ~~Structured logging (replace eprintln with tracing)~~ `tracing` crate already in use with Sentry integration (`sentry-tracing` v0.41.0). Focus instead on adding OpenTelemetry spans for AI agent calls — [`research/16-ops--observability-monitoring.md`](research/16-ops--observability-monitoring.md)
- Fix workflow trigger cooldown race condition (cooldown check before spawn, update after — concurrent webhooks can double-fire)

### Month 2 (May 2026): Capture Revenue Streams

**Priority 4 — Social Media Pipeline Migration** [ROI: Captures existing revenue]
- Wire content pipeline to ORCHA workflows (compose → review → schedule → publish)
- Dashboard for cross-platform analytics
- Template library for recurring content types

**Priority 5 — Workflow Triggers — Hardening** [ROI: Enables automation]
- ~~Cron-based triggers~~ **Already implemented**: `spawn_workflow_schedule_loop()` runs every 5 min, supports `every_5m`/`every_15m`/`every_30m`/`hourly`/`daily`/`weekly` — [`architecture-gaps-analysis.md` §GAP-S0-01](architecture-gaps-analysis.md)
- ~~Webhook triggers~~ **Already implemented**: HMAC-SHA256 validation, `x-webhook-signature` header, cooldown periods
- **Remaining**: Event triggers for internal events (on CRM stage change, on task status change) — needs event bus
- **Fix**: Cooldown race condition (check-before-spawn, update-after-spawn allows double-fire under concurrent load)
- **Fix**: Durable execution — if server restarts mid-workflow, state is lost (no checkpoint/resume)

**Priority 6 — Cost-Tiered Model Routing** [ROI: 30-85% cost savings]
Implement the RooRoo pattern: route agent tasks to cheapest capable model. `pcg_router.rs` (622 lines) already provides an OpenRouter-compatible LLM routing layer with model listing and chat completions endpoints — extend with tier logic. — [`research/10-ai--agent-orchestration.md`](research/10-ai--agent-orchestration.md)
- Task complexity classifier (Haiku-class model, ~$0.001/classification)
- Routing rules: simple tasks → Haiku ($1/MTok), medium → Sonnet ($3/MTok), complex → Opus ($5/MTok)
- Prompt caching for system prompts (90% savings on cache reads) — [`research/20-ai--context-compaction-caching.md`](research/20-ai--context-compaction-caching.md) (if exists) or [^10]
- **Bridge dual cost system**: VibeTransaction records costs properly via `ModelPricing.calculate_cost()`, but `TokenUsage.cost_cents` is never populated at insert time → `ai-usage.tsx` shows $0. Fix: populate `cost_cents` during `TokenUsage::create()` or rewrite dashboard to query VibeTransaction aggregates — [`architecture-gaps-analysis.md` §GAP-S0-03](architecture-gaps-analysis.md)
[^2] [^10]

### Month 3 (June 2026): Validate & Measure

**Priority 7 — Dogfood Metrics Dashboard**
- CAPO per task type (target: < $1)
- Agent success rate (target: > 85%)
- First-pass success rate (no retry needed)
- Workflow completion rate
- Time savings vs. manual process

**Priority 8 — Website/Hosting Service Foundation**
- Evaluate technical approach for Replit/Lovable-style deploy (container-based, preview URLs)
- Prototype single-command deploy from ORCHA workflow output
- This lays groundwork for Stage 1 hosting revenue

### OSS Leverage (Stage 0)
| Tool | Decision | Rationale | License | Source |
|------|----------|-----------|---------|--------|
| XState | **Adopt** for workflow FSM | Mature statechart library, deterministic orchestration envelope | MIT | [^4] |
| Langfuse | **Evaluate** for cost tracking | Self-hosted, 50K events/mo free; could replace dual TokenUsage/VibeTransaction gap | MIT (core) | [`research/16-ops--observability-monitoring.md`](research/16-ops--observability-monitoring.md) |
| OpenTelemetry | **Adopt** for tracing | Industry standard; `tracing-opentelemetry` crate integrates with existing `tracing` + Sentry setup | Apache 2.0 | [`research/16-ops--observability-monitoring.md`](research/16-ops--observability-monitoring.md) |
| `validator` crate | **Adopt** for input validation | No validation framework currently — only serde + manual checks; OWASP risk | MIT/Apache 2.0 | [`research/08-legal--compliance-security.md`](research/08-legal--compliance-security.md) |

### Revenue & Fundraising Actions
- PCG agency revenue continues ($5-20K/mo) — now partially through ORCHA
- Track and document time savings from platform usage (fundraising evidence)
- Begin pre-seed investor conversations informally

### Team
- Current: 3 (senior full-stack, junior dev, vision/sales lead)
- No new hires this stage
- Claude Code at 90% — team is effectively 3 humans + AI multiplier

### Key Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| Client deliverables through ORCHA | 3+ complete cycles | Manual tracking |
| CAPO (Cost per Accepted Outcome) | < $1.00 | Per-task instrumentation |
| Agent success rate | > 80% | Structured response tracking |
| Critical crashes | 0 in final 2 weeks | Sentry monitoring (already integrated — [`research/04-infra--cicd-gaps.md`](research/04-infra--cicd-gaps.md)) |
| Workflow automations active | 5+ | Platform count (schedule triggers already running) |

---

## Stage 1: First Pilots & Pre-Seed

**Duration**: Q3–Q4 2026 (July–December) — 6 months, monthly resolution
**Theme**: First external users, first platform revenue, raise pre-seed
**Budget**: $15-25K/month (post-pre-seed)

### Entry Conditions
- [ ] PCG fully dogfooding ORCHA for Editron + social media + CRM
- [ ] Agent Flow Orchestration Engine operational
- [ ] CAPO < $1 validated
- [ ] 3+ complete client delivery cycles documented

### Exit Criteria / Targets
- [ ] 3-5 paying pilot organizations on platform
- [ ] $10-20K/month revenue (agency + pilot platform fees)
- [ ] Pre-seed raised ($250-500K)
- [ ] VIBE token prototype (testnet, internal only)
- [ ] Website/hosting MVP functional
- [ ] Voice interaction (Nora) demo-ready

### Month 4-5 (July–August 2026): Pilot Onboarding

**Priority 9 — Multi-Tenant Hardening** [ROI: Enables pilot revenue]
- Tenant resource isolation (per-org API rate limits, storage quotas) — currently only Nora has rate limits (20 req/min chat, 30 req/min voice)
- Org-level billing hooks (usage metering, not payment processing yet) — VibeTransaction already tracks per-project costs; extend to per-org aggregation
- Admin dashboard for pilot management
- Data isolation audit (ensure org A can never see org B's data) — tasks lack direct `organization_id` column, relying on `task → project → organization_id` JOINs; legacy projects with `NULL organization_id` are visible to all — [`architecture-gaps-analysis.md` §GAP-S1-06](architecture-gaps-analysis.md)
- **No Row-Level Security**: single shared SQLite, no RLS. Application-layer org filtering only — [`research/06-data--database-migration.md`](research/06-data--database-migration.md)

**Priority 10 — Sirak Studios Pilot** [ROI: First external validation]
- Onboard Sirak Studios as managed platform client
- Premium pricing: $500-2K/month (managed, white-glove support)
- Weekly feedback cycles — product decisions driven by real usage
- Document onboarding friction for future self-service

**Priority 11 — Spec-Driven Development Pipeline** [ROI: Client delivery quality]
Implement lightweight version of the Spec Kit pattern for agent work quality.
- Structured task specs (what) + implementation plans (how) before agent execution
- Completion criteria as first-class fields on all tasks [Source: atlas-task-management-mcp#Completion Criteria]
- QA agent with fresh context reviews implementation output [Source: speckit-spec-driven-development#Review Agent]
- File-path-based parallel boundary detection for concurrent agent work [Source: speckit-spec-driven-development#Parallelization]

**Priority 12 — Nora Voice MVP** [ROI: Differentiation + demo wow-factor]
VoiceGateway is **ACTIVE** (`crates/nora/src/voice/gateway.rs`), with `NoraVoiceControls.tsx` (688 lines) on frontend. Chatterbox TTS configured. Discord voice integration via serenity. Key gap: unclear if end-to-end voice conversations work reliably outside Discord context. — [`research/14-ux--voice-email-desktop.md`](research/14-ux--voice-email-desktop.md)
- Voice query for status ("Nora, what's the Acme deal status?")
- Voice-triggered actions ("Nora, create a task for the Smith proposal")
- WebRTC or browser speech API for low-latency interaction (Topsi voice routes exist at `topsi/voice.rs`, 638 lines)
- Voice-first as design principle: if it can't be done by voice, the UX needs work
- **Verification needed**: end-to-end test of web-based voice input → Nora processing → voice output

### Month 6-7 (September–October 2026): Revenue & Raise

**Priority 13 — Website/Hosting Service MVP** [ROI: New revenue stream]
- Container-based deployments from ORCHA workflow output
- Preview URLs for client review
- Custom domain mapping
- Target: agencies deploying client websites through ORCHA
- Pricing: usage-based (compute + bandwidth), competitive with Render/Vercel

**Priority 14 — Pre-Seed Fundraise**
Milestones to hit before raise:
- 3+ pilot clients (including Sirak Studios)
- $10K+ MRR (agency + platform combined)
- Agent orchestration working end-to-end
- CAPO data showing unit economics work
- Demo: complete client delivery cycle in < 1 hour (research → proposal → deliverable)

**Priority 15 — VIBE Token Prototype** [ROI: Revenue model validation]
**Internal VIBE economy already operational**: `VibePricingService` calculates per-model costs, `billing.rs` helpers enforce balance checks, `marketplace.rs` (546 lines) runs APN service marketplace with VIBE ledger (85% provider credit). — [`architecture-gaps-analysis.md` §GAP-S1-01](architecture-gaps-analysis.md)
- Testnet deployment on Aptos (bridge internal VibeTransaction ledger to on-chain settlement)
- Internal PCG usage only (not external)
- 2x markup mechanism: user deposits VIBE → system settles API credits → margin captured (ModelPricing already has `multiplier` field for markup)
- Settlement window design (batch settlements every 1hr to manage price risk)
- **Regulatory flag**: consult counsel on token classification (utility vs. security) before any external distribution — [`research/08-legal--compliance-security.md`](research/08-legal--compliance-security.md)

### Month 8-9 (November–December 2026): Expand Pilots

**Priority 16 — 2-4 Additional Pilot Clients**
- Target: agencies in PCG's network (creative, marketing, consulting)
- Managed onboarding with white-glove support
- Collect NPS and usage data for seed pitch

**Priority 17 — FSM Workflow Validation** [ROI: Platform reliability]
- Event-State Analysis (ESA) matrix for all workflow definitions [Source: fsm-requirement-models#ESA]
- Traceback transitions (failed step returns to appropriate prior state, not restart) [Source: fsm-requirement-models#Traceback]
- Artifact state machines (draft → in-review → approved → complete) [Source: fsm-requirement-models#Artifact FSM]
- Guard conditions on all state transitions

**Priority 18 — Prompt Caching Infrastructure** [ROI: 50-70% token cost reduction]
- Cache system prompts across agent invocations
- Cache project context (specs, plans, codebase summaries)
- Implement stable-prefix / variable-suffix context compilation [Source: multi-agent-system-design#Context Compilation]
- Track cache hit rate (target: > 60%)

### OSS Leverage (Stage 1)
| Tool | Decision | Rationale | License | Source |
|------|----------|-----------|---------|--------|
| Langfuse | **Adopt** if Stage 0 eval positive | Self-hosted observability | MIT (core) | [`research/16-ops--observability-monitoring.md`](research/16-ops--observability-monitoring.md) |
| Docker Compose | **Already in use** | Production stack with app, db-backup, apn-bridge, nginx, cloudflared services | Apache 2.0 | [^9] |
| Prometheus + Grafana | **Adopt** for monitoring | Standard, self-hostable; no metrics endpoint exists yet | Apache 2.0 | [`research/16-ops--observability-monitoring.md`](research/16-ops--observability-monitoring.md) |
| Vitest | **Adopt** for frontend unit tests | No unit test framework exists — 100% E2E only | MIT | [`research/19-ops--testing-strategy.md`](research/19-ops--testing-strategy.md) |
| cargo-chef | **Adopt** for CI build caching | Full rebuild every CI run; no Docker layer caching | MIT/Apache 2.0 | [`research/21-infra--build-performance-dx.md`](research/21-infra--build-performance-dx.md) |
| SOPS/age | **Evaluate** for secrets | All secrets in `.env` plaintext — no vault, no encryption at rest | MPL 2.0 / BSD | [`research/08-legal--compliance-security.md`](research/08-legal--compliance-security.md) |

### Revenue & Fundraising Actions
- Agency revenue: $5-20K/mo (continuing)
- Pilot platform fees: $1.5-10K/mo (3-5 orgs × $500-2K)
- Website hosting: $0.5-2K/mo (early usage)
- **Total target: $10-20K/mo by end of Stage 1**
- Pre-seed raise: $250-500K at $2-4M valuation

### Team
- Hire #1: DevOps/Platform Engineer ($3-6K/mo remote) — owns deployment, monitoring, PostgreSQL evaluation
- Current + hire = 4 people

### Key Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| Paying pilot orgs | 3-5 | Billing records |
| Monthly revenue | $10-20K | Combined streams |
| CAPO | < $0.75 | Langfuse tracking |
| Agent success rate | > 85% | Structured responses |
| Pilot NPS | > 40 | Survey |
| Pre-seed raised | $250-500K | Bank balance |

---

## Stage 2: SaaS Launch & Seed

**Duration**: Q1–Q3 2027 (January–September) — 9 months, quarterly resolution
**Theme**: Self-service platform, billing infrastructure, seed round
**Budget**: $25-40K/month (pre-seed runway + growing revenue)

### Entry Conditions
- [ ] 3-5 paying pilot clients validated
- [ ] Pre-seed raised
- [ ] CAPO < $0.75 sustained
- [ ] Agent orchestration reliable (> 85% success rate)
- [ ] DevOps hire onboarded

### Exit Criteria / Targets
- [ ] Self-service signup with billing live
- [ ] 20+ paying organizations
- [ ] $25-50K MRR
- [ ] Seed round raised ($1-3M)
- [ ] VIBE token live on mainnet (limited release)
- [ ] Workflow marketplace with 10+ templates
- [ ] Website/hosting service generating recurring revenue

### Q1 2027: Self-Service Infrastructure

**Priority 19 — Billing & Subscription System** [ROI: Enables scalable revenue]
Internal billing infrastructure already exists (VibeTransaction ledger, ModelPricing with per-model costs, marketplace with VIBE charging). Need: external payment processing bridge. — [`research/15-biz--saas-billing-patterns.md`](research/15-biz--saas-billing-patterns.md)
- Stripe integration for seat + usage billing (bridge to existing VIBE economy)
- Hybrid pricing tiers:
  - **Free**: 2 users, 500 AI actions/month, 1 org
  - **Starter** ($29/mo): 5 users, 2,000 AI actions
  - **Pro** ($79/mo): 15 users, 10,000 AI actions
  - **Scale** ($199/mo): 50 users, 50,000 AI actions
  - **Enterprise**: Custom
- Usage metering for AI actions, hosting compute, storage
- Overage billing at model-dependent rates ($0.005-0.05/action)

**Priority 20 — Self-Service Onboarding** [ROI: Removes sales bottleneck]
- Public signup flow (email + OAuth)
- Guided setup wizard (already built in PR #48, extend for external users)
- Template gallery: "Start with Editron workflow", "Start with CRM pipeline", etc.
- In-app help system + documentation site

**Priority 21 — Database Strategy** [ROI: Unblocks scale]
PostgreSQL is already feature-gated in code (`#[cfg(feature = "postgres")]` in auth, deployment, and db crates with dual auth paths compiled conditionally). 215 SQLite migrations exist. — [`research/06-data--database-migration.md`](research/06-data--database-migration.md)
- Evaluate hybrid approach: SQLite for edge/local, PostgreSQL for cloud multi-tenant
- Consider: SQLite for single-tenant self-hosted, PostgreSQL for managed cloud
- If PostgreSQL: use `pgloader` for data migration; convert BLOB UUIDs to native PostgreSQL UUID (DbUuid abstraction simplifies); enable RLS with `organization_id` as tenant discriminator; add PgBouncer for connection pooling
- **Pitfalls**: SQLite permissive typing → Postgres strict types; BLOB UUID → native UUID; Boolean 0/1 → true/false; JSON TEXT → jsonb
- Alternative: Turso (libSQL) for distributed SQLite — evaluate fit with sovereign data model
- **No graceful shutdown currently**: bare `axum::serve()` with no drain period — deploy/restart can interrupt active agent executions, SSE streams, workflow runs. Add graceful shutdown before PostgreSQL migration.

**Priority 22 — BDI + Contract Net Orchestration** [ROI: Multi-agent maturity]
- Implement BDI model: agents maintain beliefs (context), desires (goals), intentions (adopted plans) [Source: multi-agent-system-design#BDI]
- Contract Net Protocol for dynamic task allocation (agents bid on tasks based on capability + availability) [Source: multi-agent-system-design#Contract Net]
- Blackboard pattern for shared project state (addresses 36.9% visibility gap failure mode) [Source: multi-agent-system-design#Blackboard]
- OTP-style supervision hierarchy with typed restart strategies [Source: multi-agent-system-design#OTP]

### Q2 2027: Marketplace & VIBE

**Priority 23 — Workflow Marketplace** [ROI: Network effects + distribution]
- User-created workflow templates published to marketplace
- Revenue share: 70% creator / 30% platform
- Featured templates for common agency tasks (content calendar, client onboarding, proposal generation)
- Template quality gates (automated testing + community ratings)

**Priority 24 — VIBE Token Mainnet (Limited)** [ROI: High-margin revenue]
- Aptos mainnet deployment
- 2x markup settlement mechanism:
  - User purchases VIBE tokens (via fiat on-ramp or DEX)
  - Platform accepts VIBE for AI actions at posted rate
  - Platform settles with LLM providers at wholesale rate
  - Margin captured per settlement batch
- Settlement batching: every 1 hour to manage price risk
- Price oracle for VIBE↔USD rate
- **Regulatory**: utility token classification (pays for compute, not investment contract)
- **Risk mitigation**: maintain USD fiat payment option alongside VIBE (users choose)

**VIBE Token Concerns & Mitigations:**
| Concern | Mitigation |
|---------|------------|
| Securities classification (Howey test) | Utility token: pays for specific compute services, not profit-sharing. Legal review required. |
| Price volatility | Batch settlements (1hr windows), reserve buffer, option to price in USD equivalent |
| User friction (acquiring tokens) | Fiat on-ramp (credit card → VIBE), DEX liquidity pool |
| Custody risk | Non-custodial wallet integration (user holds keys) or custodial with insurance |
| Regulatory jurisdiction | Start with non-US users if needed; structure entity accordingly |

**Priority 25 — Website/Hosting Service Expansion** [ROI: Recurring infrastructure revenue]
- Multi-framework support (React, Next.js, static sites, Node.js backends)
- Preview environments per branch/PR
- Custom domains with auto-SSL
- Usage-based pricing (compute hours + bandwidth + storage)
- Target: superior DX to Render, AI-native (generate → deploy → iterate via agent)

### Q3 2027: Seed Round

**Priority 26 — Seed Fundraise**
Milestones:
- 20+ paying organizations
- $25-50K MRR
- VIBE token live with 50+ active wallets
- Net revenue retention > 110%
- Agent success rate > 90%
- Workflow marketplace with 10+ templates
- Clear path to $100K MRR

**Priority 27 — Enterprise Features (Seed-funded)**
- SSO (SAML/OIDC) for enterprise clients
- Audit log (who did what, when)
- Advanced RBAC (role-based access control per resource type)
- Data export/portability
- SLA commitments (99.5% uptime)

### OSS Leverage (Stage 2)
| Tool | Decision | Rationale | License | Source |
|------|----------|-----------|---------|--------|
| Stripe | **Adopt** | Industry standard billing; bridge to existing VIBE economy | Proprietary API | [`research/15-biz--saas-billing-patterns.md`](research/15-biz--saas-billing-patterns.md) |
| Turso/libSQL | **Evaluate** | Distributed SQLite, sovereign-friendly | MIT | [`research/06-data--database-migration.md`](research/06-data--database-migration.md) |
| PostgreSQL | **Adopt** for cloud tier | Multi-tenant standard; already feature-gated in code | PostgreSQL | [`research/06-data--database-migration.md`](research/06-data--database-migration.md) |
| Apache AGE | **Evaluate** | Graph queries for task dependencies if recursive CTEs insufficient | Apache 2.0 | [`research/07-data--graph-databases.md`](research/07-data--graph-databases.md) |

### Revenue Model
- Agency services: $5-15K/mo (declining % as SaaS grows)
- SaaS subscriptions: $5-25K/mo (growing)
- Website hosting: $2-8K/mo
- VIBE token margin: $1-5K/mo (early)
- **Total target: $25-50K MRR by end of Stage 2**

### Team
- Hire #2: Full-stack engineer ($4-8K/mo remote)
- Current: 5 people (3 founding + DevOps + engineer)
- Seed funding enables hiring plan for Stage 3

### Key Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| Paying organizations | 20+ | Stripe dashboard |
| MRR | $25-50K | Combined streams |
| CAPO | < $0.50 | Langfuse |
| Net revenue retention | > 110% | Cohort analysis |
| Workflow marketplace templates | 10+ | Platform count |
| VIBE active wallets | 50+ | On-chain data |
| Seed raised | $1-3M | Bank balance |

---

## Stage 3: Product-Market Fit & VIBE Economy

**Duration**: Q4 2027 – Q2 2028 (9 months)
**Theme**: Prove PMF, scale VIBE economy, advanced agent capabilities
**Budget**: $60-110K/month (seed runway + revenue)

### Entry Conditions
- [ ] 20+ paying organizations
- [ ] $25-50K MRR
- [ ] Seed raised
- [ ] VIBE token live on mainnet

### Exit Criteria / Targets
- [ ] $100K+ MRR
- [ ] 100+ paying organizations
- [ ] VIBE token generating > $10K/mo margin
- [ ] Voice-first interaction production-ready
- [ ] ATLAS-style graph dependencies operational
- [ ] Desktop app (Tauri) in beta
- [ ] Clear product-market fit signal (NRR > 120%, organic growth > 30%)

### Q4 2027: Advanced Agent Intelligence

**Priority 28 — ATLAS Graph Dependencies** [ROI: Advanced task management]
- Task dependency graph with circular detection [Source: atlas-task-management-mcp#Graph Model]
- Blocking/blocked status propagation
- Critical path visualization
- Auto-scheduling based on dependency order + agent availability
- Knowledge layer: promote execution artifacts to searchable, citable entities [Source: atlas-task-management-mcp#Knowledge Layer]

**Priority 29 — Context Compaction Layer** [ROI: 10-100x token reduction]
- Pre-compute ontological summaries for CRM entities (engagement scores, intent signals, risk indicators) [Source: crm-pm-mcp-servers#Ontological Reduction]
- Replace raw database rows with interpreted business state in agent context
- Dual clock architecture: track both state AND events for temporal reasoning [Source: crm-pm-mcp-servers#Dual Clock]
- Target: 500 tokens per entity summary (down from 5,000-50,000 raw)

**Priority 30 — Multi-Agent Collision Prevention** [ROI: Reliability at scale]
- Policy engine (YAML rules for what agents can/cannot do) [Source: crm-pm-mcp-servers#Collision Prevention]
- Decision ledger (reasoning + confidence for every agent decision)
- Trust gates (high-risk actions → human approval)
- Entity ownership locks (prevent concurrent agent edits)

### Q1 2028: Voice-First & Desktop

**Priority 31 — Nora Voice-First Production** [ROI: Differentiation + accessibility]
- Full conversational interface for platform operations
- Voice → intent → action pipeline with structured response
- Multi-modal: voice input, visual output (dashboards, reports)
- Mobile-friendly voice interaction
- "Nora, generate the Q1 report for Acme" → agent orchestration → formatted deliverable

**Priority 32 — Desktop App (Tauri) Beta** [ROI: User experience + offline capability]
Tauri app already exists in `apn-app/` and `frontend/src-tauri/` with CI builds for Linux (AppImage/deb), macOS (dmg/app), Windows (msi/exe). CORS supports `tauri://` scheme. — [`research/04-infra--cicd-gaps.md`](research/04-infra--cicd-gaps.md)
- ~~Tauri wrapper around web app~~ Already built; refine native OS integration
- Offline mode for local data (SQLite) — [`research/23-arch--offline-local-first.md`](research/23-arch--offline-local-first.md)
- System tray with Nora voice activation
- File system integration (drag-and-drop from desktop to ORCHA)
- **Missing**: Auto-update mechanism, code signing/notarization for macOS/Windows, crash reporting for desktop

**Priority 33 — Hosting Service Advanced** [ROI: Infrastructure revenue scaling]
- Database hosting (managed PostgreSQL, Redis)
- Background job processing
- Edge deployments (multi-region)
- CI/CD pipelines integrated with ORCHA workflows
- Agent-assisted debugging ("Nora, why is the deploy failing?")

### Q2 2028: PMF Validation

**Priority 34 — Data Sovereignty Research & Positioning**
- Survey pilot clients on data sovereignty concerns
- Competitive analysis: which agencies care about data location/ownership?
- If signal is strong: add self-hosted deployment option, data residency controls
- If signal is weak: keep as technical differentiator, don't lead marketing with it

**Priority 35 — Patent Applications**
- File provisional patents on key innovations:
  - Categorical architecture for agent composition (Topos-based morphism system)
  - VIBE token settlement mechanism (markup-based API cost passthrough)
  - Agent orchestration with structured response envelope + cost-tiered routing
- Work with IP counsel to identify additional patentable elements

### Revenue Model (Stage 3)
- SaaS subscriptions: $30-60K/mo
- Website/hosting: $10-20K/mo
- VIBE token margin: $5-15K/mo
- Agency services: $5-10K/mo (maintenance)
- Marketplace revenue share: $1-5K/mo
- **Total target: $100K+ MRR**

### Team
- Hire #3: Developer advocate / technical marketer ($5-8K/mo)
- Hire #4: Full-stack engineer ($4-8K/mo)
- Hire #5: Product designer ($4-7K/mo)
- **Team: 8 people**

### Licensing Transition
- **Source-Available (BSL)**: Core platform code published under Business Source License
- **MIT**: MCP server definitions, workflow node specs, APN protocol specifications
- Community can read, learn, contribute bug fixes. Cannot offer competing managed service.

---

## Stage 4: Growth & Series A

**Duration**: Q3 2028 – Q4 2029 (18 months)
**Theme**: Scale go-to-market, advanced platform capabilities, Series A
**Budget**: $100-200K/month (seed runway + revenue → Series A)

### Entry Conditions
- [ ] $100K+ MRR
- [ ] 100+ paying organizations
- [ ] PMF validated (NRR > 120%)
- [ ] VIBE economy self-sustaining

### Exit Criteria / Targets
- [ ] $300-500K MRR
- [ ] 500+ paying organizations
- [ ] Series A raised ($5-15M)
- [ ] VIBE token with 500+ active wallets
- [ ] Full hosting platform competitive with Render/Vercel
- [ ] Desktop app GA
- [ ] SudoLang-style constraint specs for advanced agent instructions

### Key Capabilities

**Priority 36 — Docker Service-Per-Agent Architecture** [ROI: Scaling + isolation]
- Containerized agent execution with resource limits [Source: docker-compose-multi-agent#Service Pattern]
- MCP Gateway as service mesh (consolidate MCP servers, per-tool network isolation) [Source: docker-compose-multi-agent#MCP Gateway]
- Sandboxed code execution (agent-generated code in isolated containers) [Source: docker-compose-multi-agent#Sandboxes]
- Health check infrastructure for all agent services
- Auto-scaling agent pool based on queue depth (KEDA pattern) [Source: docker-compose-multi-agent#KEDA]

**Priority 37 — SudoLang Constraint Specifications** [ROI: Advanced agent instructions]
- Constraint-based agent definitions (declare invariants, not step-by-step) [Source: sudolang-aidd-framework#Constraints]
- Behavioral modifiers for per-task customization (depth, format, scope) [Source: sudolang-aidd-framework#Modifiers]
- Progressive context loading (lazy load project context, 20-30% token savings) [Source: sudolang-aidd-framework#Progressive Discovery]
- Vision document as authoritative source for all agents [Source: sudolang-aidd-framework#Vision]

**Priority 38 — Advanced Hosting Platform**
- GPU compute for AI workloads (image generation, model fine-tuning)
- Serverless functions
- Managed databases (PostgreSQL, Redis, SQLite edge)
- Storage (S3-compatible object storage)
- CDN integration
- Price competitive: 20-40% below AWS, comparable to Render/Vercel

**Priority 39 — Series A Fundraise**
Milestones:
- $300-500K MRR
- 500+ organizations
- NRR > 120%
- VIBE token economy validated ($10K+/mo margin)
- Clear category leadership in "AI-powered agency platform"
- Patent portfolio filed

### Revenue Model (Stage 4)
- SaaS subscriptions: $100-200K/mo
- Hosting/compute: $50-100K/mo
- VIBE token margin: $30-80K/mo
- Marketplace: $10-30K/mo
- Enterprise contracts: $50-100K/mo
- **Total target: $300-500K MRR**

### Team
- Post-Series A: 15-25 people
- Engineering: 8-12
- Sales/marketing: 3-5
- Product/design: 2-3
- Operations: 1-2

---

## Stage 5: Full Sovereignty

**Duration**: 2030–2031 (24 months)
**Theme**: Decentralized compute, VIBELAND, complete sovereign stack
**Budget**: Series A runway + self-sustaining revenue

### Entry Conditions
- [ ] $300K+ MRR self-sustaining
- [ ] Series A raised
- [ ] 500+ organizations on platform
- [ ] Core platform mature and stable

### Exit Criteria / Targets
- [ ] APN mesh network with 100+ compute nodes
- [ ] VIBELAND virtual environment beta
- [ ] Full data sovereignty: users own all data, can self-host entirely
- [ ] VIBE token economy with 5,000+ active wallets
- [ ] $1M+ MRR

### Key Capabilities

**Priority 40 — APN Mesh Network Production**
Foundation exists: `alpha-protocol-core` (libp2p, X25519 encryption, Ed25519 keys, BIP39 mnemonics), `apn-client` library, functional NATS relay bridge (`Dockerfile.apn-bridge`), `MeshNode`/`AlphaNode` structs, `NodeReputation`/`ResourceContribution` tracking, `RewardDistributor`. — [`research/17-p2p--decentralized-compute.md`](research/17-p2p--decentralized-compute.md), [`research/18-p2p--oss-mesh-tools.md`](research/18-p2p--oss-mesh-tools.md)
- Distributed agent execution across APN nodes
- Task routing based on node capabilities (GPU, memory, location)
- Resource accounting and VIBE token settlement for compute usage (marketplace already tracks with 85% provider credit)
- Integration with Omega Wireless hardware compute nodes
- Federated learning: agents improve from cross-org patterns without leaking data
- **Missing**: Byzantine fault tolerance, mesh consensus, production-scale peer discovery

**Priority 41 — VIBELAND Virtual Environment**
`virtual-environment/` page and `vibeland/UserAvatar.tsx` (846 lines — 3D avatar logic, materials, animations) already exist in codebase.
- 3D virtual workspace (Three.js + physics engine already in codebase)
- VRM avatars for team presence
- Spatial audio for voice collaboration
- Virtual project rooms with real-time data visualization
- Gamification: VIBE rewards for platform contributions

**Priority 42 — Complete Sovereign Stack**
- Self-hosted deployment option (Docker Compose, one-command setup)
- Data residency controls (choose where data lives)
- End-to-end encryption for sensitive data
- P2P sync between instances (APN protocol)
- Zero-knowledge proofs for cross-org collaboration without data exposure

**Priority 43 — Federated Agent Marketplace**
APN marketplace already functional (`marketplace.rs`, 546 lines) with service listings, subscriptions, VIBE ledger, 85% provider credit. 4 MCP servers already expose ORCHA capabilities. — [`architecture-gaps-analysis.md` §GAP-XC-05](architecture-gaps-analysis.md)
- Third-party agents publish to marketplace via MCP (add Streamable HTTP transport to existing stdio-only servers for remote access)
- Revenue share on agent usage (85%/15% split already implemented in APN marketplace)
- Agent reputation system (success rate, cost efficiency, user ratings)
- Cross-platform agent interop via A2A protocol — [`research/11-ai--agent-orchestration.md`](research/11-ai--agent-orchestration.md) [^9]
- Register ORCHA MCP servers in official MCP Registry (18,000+ servers) — [`research/10-ai--mcp-protocol-ecosystem.md`](research/10-ai--mcp-protocol-ecosystem.md)

### Revenue Model (Stage 5)
- SaaS subscriptions: $200-400K/mo
- Hosting/compute: $200-400K/mo
- VIBE token margin: $100-200K/mo
- APN compute network fees: $50-100K/mo
- Marketplace + enterprise: $100-200K/mo
- **Total target: $1M+ MRR**

---

## Cross-Cutting Concerns

### Research Principles Mapped to Stages

| Principle | Source | Stage | Application | Research Report |
|-----------|--------|-------|-------------|----------------|
| Cost-tiered model routing | RooRoo [^2] | 0 | Immediate cost savings, 30-85% reduction | [`10-ai--agent-orchestration.md`](research/10-ai--agent-orchestration.md) |
| Output Envelope protocol | RooRoo [^2] | 0 | Agent Flow Engine structured responses | [`reference/rooroo-agent-orchestration.md`](reference/2026-03-18--research--rooroo-agent-orchestration.md) |
| Per-task cost tracking (CAPO) | Unit Economics [^10] | 0 | Bridge VibeTransaction→dashboard (data exists) | [`architecture-gaps-analysis.md`](architecture-gaps-analysis.md) |
| CI pipeline strictness | Verification Pass | 0 | Remove continue-on-error from clippy/tests | [`04-infra--cicd-gaps.md`](research/04-infra--cicd-gaps.md) |
| Input validation framework | Security Research | 0-1 | No `validator`/`garde` crate — OWASP risk | [`08-legal--compliance-security.md`](research/08-legal--compliance-security.md) |
| Prompt caching | Unit Economics [^10] | 1 | 50-70% input token savings | [`10-ai--agent-orchestration.md`](research/10-ai--agent-orchestration.md) |
| Spec-Driven Development | Spec Kit [^5] | 1 | Client delivery quality | [`reference/speckit-spec-driven-development.md`](reference/2026-03-18--research--speckit-spec-driven-development.md) |
| FSM/ESA workflow validation | FSM Research [^4] | 1 | Platform reliability | [`12-ai--workflow-engines.md`](research/12-ai--workflow-engines.md) |
| Five-Guardrail FinOps | Unit Economics [^10] | 1-2 | Cost control at scale | [`15-biz--saas-billing-patterns.md`](research/15-biz--saas-billing-patterns.md) |
| Secrets management | Security Research | 1 | All secrets in `.env` plaintext | [`08-legal--compliance-security.md`](research/08-legal--compliance-security.md) |
| BDI + Contract Net orchestration | Multi-Agent [^3] | 2 | Multi-agent maturity | [`11-ai--agent-orchestration.md`](research/11-ai--agent-orchestration.md) |
| Context compaction (ontological) | CRM/PM MCP [^8] | 3 | 10-100x token reduction | [`reference/crm-pm-mcp-servers.md`](reference/2026-03-18--research--crm-pm-mcp-servers.md) |
| ATLAS graph dependencies | ATLAS [^7] | 3 | Advanced task management | [`07-data--graph-databases.md`](research/07-data--graph-databases.md) |
| Multi-agent collision prevention | CRM/PM MCP [^8] | 3 | Reliability at scale | [`24-ai--safety-guardrails.md`](research/24-ai--safety-guardrails.md) |
| MCP HTTP transport + registry | MCP Ecosystem | 3 | Remote access to existing 4 MCP servers | [`10-ai--mcp-protocol-ecosystem.md`](research/10-ai--mcp-protocol-ecosystem.md) |
| Docker service-per-agent | Docker Research [^9] | 4 | Scaling architecture | [`reference/docker-compose-multi-agent.md`](reference/2026-03-18--research--docker-compose-multi-agent-architecture.md) |
| SudoLang constraint specs | SudoLang/AIDD [^6] | 4 | Advanced agent instructions | [`reference/sudolang-aidd-framework.md`](reference/2026-03-18--research--sudolang-aidd-framework.md) |
| Full APN mesh integration | APN Architecture [^23] | 5 | Sovereignty | [`17-p2p--decentralized-compute.md`](research/17-p2p--decentralized-compute.md) |

### Lusser's Law Mitigation Strategy

System reliability = product of individual agent reliability. With 5 agents at 95% each, system success = 77%. Strategy:

| Stage | Agents in Chain | Per-Agent Target | System Reliability | Mitigation |
|-------|----------------|------------------|--------------------|------------|
| 0 | 1-2 | 80% | 64-80% | Human review, retry |
| 1 | 2-3 | 85% | 61-73% | Validation gates, fallback |
| 2 | 3-4 | 90% | 66-73% | Circuit breakers, reassignment |
| 3 | 4-5 | 92% | 72-66% | Compound failure dashboard |
| 4 | 5-8 | 95% | 66-77% | Full supervision hierarchy |

### Voice-First Design Principle

Every new feature should pass the voice test: "Can a user accomplish this by talking to Nora?" This doesn't mean voice is the only interface — it means voice is always *an* interface. Visual dashboards complement voice, they don't replace it.

### Agent Identity Strategy

Agent names (Nora, Scout, Astra, Cash, Lux, etc.) serve as domain shorthands:
- Users say "ask Scout to research Acme" — familiar and intuitive
- Under the hood, orchestration routes to the best-available agent/model for the task
- Names map to capability profiles, not to specific LLM instances
- Invisible orchestration + familiar names = accessible complexity

### Patent Timeline

| Filing | Stage | Subject Matter |
|--------|-------|----------------|
| Provisional #1 | 3 (Q2 2028) | Categorical agent composition architecture |
| Provisional #2 | 3 (Q2 2028) | VIBE token API settlement mechanism |
| Provisional #3 | 4 (2029) | Distributed agent execution protocol (APN) |
| Full utility filings | 4-5 | Convert provisionals + additional claims |

---

## Dependency Graph

```
Stage 0                  Stage 1                  Stage 2
────────                 ────────                 ────────
Agent Flow Engine ──────→ Pilot Clients ──────────→ Self-Service SaaS
  │                        │                        │
  ├─ Editron Migration     ├─ Sirak Studios         ├─ Billing (Stripe)
  ├─ Social Migration      ├─ Nora Voice MVP        ├─ BDI Orchestration
  ├─ Cost-Tier Routing     ├─ Hosting MVP           ├─ Workflow Marketplace
  ├─ Stability Fixes       ├─ VIBE Prototype        ├─ VIBE Mainnet
  └─ Workflow Triggers     ├─ FSM Validation        └─ Database Strategy
                           ├─ Prompt Caching             │
                           └─ Pre-Seed Raise              │
                                                          │
Stage 3                  Stage 4                  Stage 5
────────                 ────────                 ────────
PMF & VIBE Economy ─────→ Growth & Series A ─────→ Full Sovereignty
  │                        │                        │
  ├─ Graph Dependencies    ├─ Docker Agents         ├─ APN Mesh Production
  ├─ Context Compaction    ├─ SudoLang Specs        ├─ VIBELAND
  ├─ Collision Prevention  ├─ Advanced Hosting      ├─ Sovereign Stack
  ├─ Voice Production      ├─ Series A Raise        ├─ Federated Marketplace
  ├─ Desktop Beta          └─ Source-Available       └─ P2P Sync
  ├─ Data Sovereignty
  └─ Patent Filings
```

**Critical Path**: Agent Flow Engine → Pilot Clients → Pre-Seed → Self-Service → Seed → PMF → Series A

---

## Risk Register

### Technical Risks

| Risk | Probability | Impact | Stage | Mitigation |
|------|------------|--------|-------|------------|
| Agent orchestration engine takes longer than expected | Medium | Critical | 0 | Time-box to 4 weeks; fallback to simplified queue-based system; `spawn_workflow_schedule_loop()` pattern already proven |
| CI `continue-on-error` lets broken code reach main | High | High | 0 | **Immediate fix**: remove from clippy/test steps — [`research/04-infra--cicd-gaps.md`](research/04-infra--cicd-gaps.md) |
| Dual cost system (TokenUsage vs VibeTransaction) causes reporting confusion | High | Medium | 0 | Bridge tables or consolidate to single source of truth |
| No graceful shutdown interrupts active operations on deploy | Medium | High | 1 | Add drain period to `axum::serve()`; checkpoint agent executions |
| SQLite performance ceiling hit before DB migration | Low | High | 1-2 | Monitor query latency; Turso/libSQL as intermediate step |
| VIBE token price volatility erodes margin | Medium | High | 2-3 | Settlement batching (1hr); USD pricing fallback; reserve buffer |
| LLM provider price changes break unit economics | Medium | High | 1+ | Multi-provider routing (9 executors); local model fallback; prompt caching |
| Multi-tenant data leakage via NULL org projects | Medium | Critical | 1 | Audit all queries; delete/assign NULL-org projects; no RLS until PostgreSQL — [`research/08-legal--compliance-security.md`](research/08-legal--compliance-security.md) |
| 90% AI-generated code creates maintenance debt | Medium | Medium | 1+ | Mandatory code review by senior dev; architectural tests |
| 145MB frontend bundle size impacts load times | Low | Medium | 1 | Audit dist/ output; tree-shaking; lazy loading (already in use) — [`research/03-arch--frontend.md`](research/03-arch--frontend.md) |

### Business Risks

| Risk | Probability | Impact | Stage | Mitigation |
|------|------------|--------|-------|------------|
| Pre-seed doesn't close in time | Medium | High | 1 | Agency revenue funds development; extend runway by reducing burn |
| Pilot clients churn before seed | Medium | High | 1-2 | Weekly feedback cycles; white-glove support; pivot quickly |
| VIBE token regulatory challenge | Medium | Critical | 2-3 | Legal review before mainnet; utility classification; geographic restrictions; maintain fiat option |
| Competitor launches similar integrated platform | Low | Medium | 2+ | Speed advantage; agency domain expertise; sovereign differentiation |
| Team burnout at 3-person scale | High | Critical | 0-1 | Sustainable pace; Claude Code force multiplier; hire early |

### Market Risks

| Risk | Probability | Impact | Stage | Mitigation |
|------|------------|--------|-------|------------|
| AI agent market correction (Gartner: 40% agentic projects cancelled by 2027) | Medium | High | 2-3 | Focus on proven ROI; agency services as revenue floor |
| MCP standard fragments or loses adoption | Low | Medium | 3+ | Multi-protocol support; own MCP servers provide value regardless |
| Crypto regulatory crackdown affects VIBE | Medium | High | 2-3 | Fiat payment always available; token is optional layer |

---

## Revenue Summary by Stage

| Stage | Timeline | Monthly Revenue Target | Revenue Sources | Team Size |
|-------|----------|----------------------|-----------------|-----------|
| 0 | Q2 2026 | $5-20K (existing) | Agency services | 3 |
| 1 | Q3-Q4 2026 | $10-20K | Agency + pilots | 4 |
| 2 | Q1-Q3 2027 | $25-50K | Agency + SaaS + hosting | 5 |
| 3 | Q4 2027-Q2 2028 | $100K+ | SaaS + hosting + VIBE | 8 |
| 4 | Q3 2028-Q4 2029 | $300-500K | Full platform | 15-25 |
| 5 | 2030-2031 | $1M+ | Full platform + APN | 25-40 |

---

## Fundraising Summary

| Round | Target | Timing | Valuation | Key Metric |
|-------|--------|--------|-----------|------------|
| Pre-Seed | $250-500K | Q3-Q4 2026 | $2-4M | 3+ pilots, $10K MRR |
| Seed | $1-3M | Q3 2027 | $10-20M | 20+ orgs, $50K MRR |
| Series A | $5-15M | Q1-Q2 2029 | $40-80M | 500+ orgs, $500K MRR |

---

## Guiding Principles

1. **Speed as a Habit** — every decision starts with "why can't this be done sooner?" [^1]
2. **Revenue funds development** — agency work isn't a distraction, it's the business model
3. **Dogfood everything** — if PCG won't use it, customers won't either
4. **Voice-first design** — every feature should work through Nora
5. **Invisible orchestration, familiar names** — agents are helpful colleagues, not black boxes
6. **Cost-tiered everything** — cheap models for cheap tasks, expensive models only when needed
7. **Ship, measure, iterate** — CAPO and agent success rate are the north star metrics
8. **Proprietary now, open later** — protect IP while small, open strategically at scale
9. **VIBE margin is real revenue** — 2x markup on API costs is a high-margin business
10. **Sovereignty is a moat, not a feature** — build it into the architecture, market it when customers care

---

## References

### External Sources
[^1]: Girouard, Dave. "Speed as a Habit." First Round Review. https://review.firstround.com/speed-as-a-habit/
[^11]: AI Agents Market Size. Markets and Markets, 2026. https://www.marketsandmarkets.com/Market-Reports/ai-agents-market-15761548.html
[^12]: Agentic AI Market CAGR 43.8%. Market.us, 2026. https://market.us/report/agentic-ai-market/
[^13]: MCP Roadmap 2026. The New Stack. https://thenewstack.io/model-context-protocol-roadmap-2026/
[^14]: Cursor $1B ARR, $29.3B valuation. AI Tool Discovery, 2026. https://www.aitooldiscovery.com/guides/cursor-ai-pricing
[^15]: Linear $400M valuation, $35K marketing. Eleken Case Study. https://www.eleken.co/blog-posts/linear-app-case-study
[^16]: GPU Rendering: Render vs Akash vs AWS. Securities.io, 2026. https://www.securities.io/gpu-rendering-wars-render-akash-aws/
[^17]: CrewAI Enterprise Platform. https://crewai.com/
[^18]: LangGraph Documentation. https://github.com/langchain-ai/langgraph
[^19]: Tech Startup Running Costs. Financial Models Lab, 2026. https://financialmodelslab.com/blogs/operating-costs/technology-start-up
[^25]: Open Core vs SaaS Business Model. Teleport. https://goteleport.com/blog/open-core-vs-saas-business-model/

### Internal Research (Reference Studies — `planning/roadmap/reference/`)
[^2]: RooRoo Agent Orchestration Framework. [`reference/2026-03-18--research--rooroo-agent-orchestration.md`](reference/2026-03-18--research--rooroo-agent-orchestration.md)
[^3]: Multi-Agent System Design Research. [`reference/2026-03-18--research--multi-agent-system-design.md`](reference/2026-03-18--research--multi-agent-system-design.md)
[^4]: FSM & Requirement Models. [`reference/2026-03-18--research--fsm-requirement-models.md`](reference/2026-03-18--research--fsm-requirement-models.md)
[^5]: Spec Kit & Spec-Driven Development. [`reference/2026-03-18--research--speckit-spec-driven-development.md`](reference/2026-03-18--research--speckit-spec-driven-development.md)
[^6]: SudoLang & AIDD Framework. [`reference/2026-03-18--research--sudolang-aidd-framework.md`](reference/2026-03-18--research--sudolang-aidd-framework.md)
[^7]: ATLAS Task Management MCP. [`reference/2026-03-18--research--atlas-task-management-mcp.md`](reference/2026-03-18--research--atlas-task-management-mcp.md)
[^8]: CRM & PM MCP Servers. [`reference/2026-03-18--research--crm-pm-mcp-servers.md`](reference/2026-03-18--research--crm-pm-mcp-servers.md)
[^9]: Docker Compose Multi-Agent Architecture. [`reference/2026-03-18--research--docker-compose-multi-agent-architecture.md`](reference/2026-03-18--research--docker-compose-multi-agent-architecture.md)
[^10]: Unit Economics & Agent Metrics. [`reference/2026-03-18--research--unit-economics-agent-metrics.md`](reference/2026-03-18--research--unit-economics-agent-metrics.md)

### Internal Planning Documents
[^20]: Consolidated Pipeline Roadmap. [`reference/2026-03-18--reference--consolidated-pipeline-roadmap.md`](reference/2026-03-18--reference--consolidated-pipeline-roadmap.md)
[^21]: Dealflow Pipeline Status. [`reference/2026-03-18--reference--dealflow-pipeline-status.md`](reference/2026-03-18--reference--dealflow-pipeline-status.md)
[^22]: TOPOS Categorical Architecture. `docs/TOPOS_CATEGORICAL_ARCHITECTURE.md`
[^23]: APN Integration Architecture. `docs/APN_INTEGRATION_ARCHITECTURE.md`
[^24]: BACKLOG Remaining Work. `planning/BACKLOG--remaining-work.md`

### Architecture Gap Analysis Research (2026-03-19 — `planning/roadmap/research/`)
27 codebase verification and web research reports covering backend architecture, frontend architecture, CI/CD, deployment, database migration, graph databases, legal/compliance, MCP ecosystem, agent orchestration, workflow engines, billing patterns, OSS tools, voice/email/desktop UX, observability, decentralized compute, P2P mesh, testing strategy, onboarding/ETL, build performance, product analytics, offline/local-first, AI safety, and team scaling. See [`architecture-gaps-analysis.md`](architecture-gaps-analysis.md) for the full gap inventory with cross-references.

---

*This roadmap is a living document. Review monthly, revise quarterly. Speed as a habit means this plan changes when reality changes.*

# ORCHA 5-Year Product Roadmap

**Version**: 1.0
**Date**: 2026-03-18
**Author**: PCG Founding Team + Claude Research Sprint
**Status**: Internal Planning Document

> *"Vision to reality as a service."*

---

## Executive Summary

ORCHA is Powerclub Global's sovereign AI orchestration platform — a full-stack dashboard that transforms how creative agencies deliver client work. Over five years, ORCHA will evolve from an internal agency tool into a self-service platform where anyone can turn vision into reality, powered by AI agents, workflow automation, and decentralized compute.

This roadmap is structured around **revenue-first staging**: each stage unlocks the next through validated metrics, not calendar dates. Every decision is filtered through two principles:

1. **Speed as a Habit** — "Why can't this be done sooner?" applied to every decision [^1]
2. **ROI-Ordered Prioritization** — features ranked by (revenue impact × probability of success) / effort

The roadmap spans 6 stages from dogfooding (Q2 2026) to full sovereignty (2031), with a founding team of 3 using Claude Code at 90% code generation rate as a force multiplier.

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

### Exit Criteria / Targets
- [ ] PCG runs 100% of Editron jobs through ORCHA workflows
- [ ] PCG runs 100% of social media management through ORCHA
- [ ] Agent Flow Orchestration Engine functional (background worker, phase progression, gates)
- [ ] 3+ complete client delivery cycles executed through platform
- [ ] Internal CAPO tracked for all agent actions (target: < $1/action)
- [ ] Zero critical crashes in 2-week window (unwrap elimination on hot paths)

### Month 1 (April 2026): Foundation

**Priority 1 — Agent Flow Orchestration Engine** [ROI: Critical blocker]
Build the background worker that progresses agent flows through phases, enforces gates, handles delegation and retry. This is the #1 architectural gap.
- Implement structured response envelope protocol (status, message, artifacts, clarification_needed) [Source: rooroo-agent-orchestration#Response Envelope]
- Background task runner with 5-level error handling: retry → fallback → reassign → circuit break → human escalate [Source: multi-agent-system-design#Error Handling]
- Phase gate enforcement with named triggers (start, complete, block, cancel) [Source: atlas-task-management-mcp#State Transitions]
- Per-task cost tracking (input/output/cached tokens, model, wall-clock time) [Source: unit-economics-agent-metrics#CAPO]

**Priority 2 — Editron Workflow Migration** [ROI: Captures existing revenue]
Move Editron post-production jobs from manual/external into ORCHA workflows.
- Map current Editron manual steps to workflow nodes
- Build missing node types needed for Editron (likely: http_request, file operations)
- Test with 3 real client deliverables

**Priority 3 — Critical Stability Fixes** [ROI: Unblocks dogfooding]
- Eliminate unwraps on hot paths (login, CRM pipeline, workflow execution, agent dispatch)
- Add rate limiting to public-facing endpoints (tower_governor)
- Structured logging (replace eprintln with tracing)

### Month 2 (May 2026): Capture Revenue Streams

**Priority 4 — Social Media Pipeline Migration** [ROI: Captures existing revenue]
- Wire content pipeline to ORCHA workflows (compose → review → schedule → publish)
- Dashboard for cross-platform analytics
- Template library for recurring content types

**Priority 5 — Workflow Triggers** [ROI: Enables automation]
- Cron-based triggers (schedule workflows to run on interval)
- Event triggers (on CRM stage change, on task status change)
- Webhook triggers already built (PR #49) — extend to inbound

**Priority 6 — Cost-Tiered Model Routing** [ROI: 30-85% cost savings]
Implement the RooRoo pattern: route agent tasks to cheapest capable model.
- Task complexity classifier (Haiku-class model, ~$0.001/classification)
- Routing rules: simple tasks → Haiku ($1/MTok), medium → Sonnet ($3/MTok), complex → Opus ($5/MTok)
- Prompt caching for system prompts (90% savings on cache reads)
[Source: rooroo-agent-orchestration#Cost Tiers, unit-economics-agent-metrics#Model Routing]

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
| Tool | Decision | Rationale |
|------|----------|-----------|
| XState | **Adopt** for workflow FSM | Mature statechart library, deterministic orchestration envelope [Source: fsm-requirement-models#XState] |
| Langfuse | **Evaluate** for cost tracking | Self-hosted, 50K events/mo free [Source: unit-economics-agent-metrics#Observability] |
| OpenTelemetry | **Adopt** for tracing | Industry standard, Rust support via tracing crate |

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
| Critical crashes | 0 in final 2 weeks | Sentry monitoring |
| Workflow automations active | 5+ | Platform count |

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
- Tenant resource isolation (per-org API rate limits, storage quotas)
- Org-level billing hooks (usage metering, not payment processing yet)
- Admin dashboard for pilot management
- Data isolation audit (ensure org A can never see org B's data)

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
- Voice query for status ("Nora, what's the Acme deal status?")
- Voice-triggered actions ("Nora, create a task for the Smith proposal")
- WebRTC or browser speech API for low-latency interaction
- Voice-first as design principle: if it can't be done by voice, the UX needs work

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
- Testnet deployment on Aptos
- Internal PCG usage only (not external)
- 2x markup mechanism: user deposits VIBE → system settles API credits → margin captured
- Settlement window design (batch settlements every 1hr to manage price risk)
- **Regulatory flag**: consult counsel on token classification (utility vs. security) before any external distribution

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
| Tool | Decision | Rationale |
|------|----------|-----------|
| Langfuse | **Adopt** if Stage 0 eval positive | Self-hosted observability |
| Docker Compose | **Adopt** for deployment | Service-per-agent architecture [Source: docker-compose-multi-agent#Service Pattern] |
| Prometheus + Grafana | **Adopt** for monitoring | Standard, self-hostable |

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
- Stripe integration for seat + usage billing
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
- Evaluate hybrid approach: SQLite for edge/local, PostgreSQL for cloud multi-tenant
- Consider: SQLite for single-tenant self-hosted, PostgreSQL for managed cloud
- If PostgreSQL: migrate critical tables first (users, orgs, CRM, tasks), keep SQLite for local dev
- Alternative: Turso (libSQL) for distributed SQLite — evaluate fit with sovereign data model

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
| Tool | Decision | Rationale |
|------|----------|-----------|
| Stripe | **Adopt** | Industry standard billing |
| Turso/libSQL | **Evaluate** | Distributed SQLite, sovereign-friendly |
| PostgreSQL | **Adopt** for cloud tier | Multi-tenant standard |

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
- Tauri wrapper around web app with native OS integration
- Offline mode for local data (SQLite)
- System tray with Nora voice activation
- File system integration (drag-and-drop from desktop to ORCHA)
- Auto-update mechanism

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
- Distributed agent execution across APN nodes
- Task routing based on node capabilities (GPU, memory, location)
- Resource accounting and VIBE token settlement for compute usage
- Integration with Omega Wireless hardware compute nodes
- Federated learning: agents improve from cross-org patterns without leaking data

**Priority 41 — VIBELAND Virtual Environment**
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
- Third-party agents publish to marketplace via MCP
- Revenue share on agent usage
- Agent reputation system (success rate, cost efficiency, user ratings)
- Cross-platform agent interop via A2A protocol [Source: docker-compose-multi-agent#A2A]

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

| Principle | Source | Stage | Application |
|-----------|--------|-------|-------------|
| Cost-tiered model routing | RooRoo | 0 | Immediate cost savings, 30-85% reduction |
| Output Envelope protocol | RooRoo | 0 | Agent Flow Engine structured responses |
| Per-task cost tracking (CAPO) | Unit Economics | 0 | Foundation for all optimization |
| Prompt caching | Unit Economics | 1 | 50-70% input token savings |
| Spec-Driven Development | Spec Kit | 1 | Client delivery quality |
| FSM/ESA workflow validation | FSM Research | 1 | Platform reliability |
| Five-Guardrail FinOps | Unit Economics | 1-2 | Cost control at scale |
| BDI + Contract Net orchestration | Multi-Agent | 2 | Multi-agent maturity |
| Context compaction (ontological) | CRM/PM MCP | 3 | 10-100x token reduction |
| ATLAS graph dependencies | ATLAS | 3 | Advanced task management |
| Multi-agent collision prevention | CRM/PM MCP | 3 | Reliability at scale |
| Docker service-per-agent | Docker Research | 4 | Scaling architecture |
| SudoLang constraint specs | SudoLang/AIDD | 4 | Advanced agent instructions |
| Full APN mesh integration | APN Architecture | 5 | Sovereignty |

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
| Agent orchestration engine takes longer than expected | Medium | Critical | 0 | Time-box to 4 weeks; fallback to simplified queue-based system |
| SQLite performance ceiling hit before DB migration | Low | High | 1-2 | Monitor query latency; Turso/libSQL as intermediate step |
| VIBE token price volatility erodes margin | Medium | High | 2-3 | Settlement batching (1hr); USD pricing fallback; reserve buffer |
| LLM provider price changes break unit economics | Medium | High | 1+ | Multi-provider routing; local model fallback; prompt caching |
| 90% AI-generated code creates maintenance debt | Medium | Medium | 1+ | Mandatory code review by senior dev; architectural tests |

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

[^1]: Girouard, Dave. "Speed as a Habit." First Round Review. https://review.firstround.com/speed-as-a-habit/
[^2]: RooRoo Agent Orchestration Framework. [Source: 2026-03-18--research--rooroo-agent-orchestration.md]
[^3]: Multi-Agent System Design Research. [Source: 2026-03-18--research--multi-agent-system-design.md]
[^4]: FSM & Requirement Models. [Source: 2026-03-18--research--fsm-requirement-models.md]
[^5]: Spec Kit & Spec-Driven Development. [Source: 2026-03-18--research--speckit-spec-driven-development.md]
[^6]: SudoLang & AIDD Framework. [Source: 2026-03-18--research--sudolang-aidd-framework.md]
[^7]: ATLAS Task Management MCP. [Source: 2026-03-18--research--atlas-task-management-mcp.md]
[^8]: CRM & PM MCP Servers. [Source: 2026-03-18--research--crm-pm-mcp-servers.md]
[^9]: Docker Compose Multi-Agent Architecture. [Source: 2026-03-18--research--docker-compose-multi-agent-architecture.md]
[^10]: Unit Economics & Agent Metrics. [Source: 2026-03-18--research--unit-economics-agent-metrics.md]
[^11]: AI Agents Market Size. Markets and Markets, 2026. https://www.marketsandmarkets.com/Market-Reports/ai-agents-market-15761548.html
[^12]: Agentic AI Market CAGR 43.8%. Market.us, 2026. https://market.us/report/agentic-ai-market/
[^13]: MCP Roadmap 2026. The New Stack. https://thenewstack.io/model-context-protocol-roadmap-2026/
[^14]: Cursor $1B ARR, $29.3B valuation. AI Tool Discovery, 2026. https://www.aitooldiscovery.com/guides/cursor-ai-pricing
[^15]: Linear $400M valuation, $35K marketing. Eleken Case Study. https://www.eleken.co/blog-posts/linear-app-case-study
[^16]: GPU Rendering: Render vs Akash vs AWS. Securities.io, 2026. https://www.securities.io/gpu-rendering-wars-render-akash-aws/
[^17]: CrewAI Enterprise Platform. https://crewai.com/
[^18]: LangGraph Documentation. https://github.com/langchain-ai/langgraph
[^19]: Tech Startup Running Costs. Financial Models Lab, 2026. https://financialmodelslab.com/blogs/operating-costs/technology-start-up
[^20]: Consolidated Pipeline Roadmap. [Source: notes/2026-03-18--reference--consolidated-pipeline-roadmap.md]
[^21]: Dealflow Pipeline Status. [Source: notes/2026-03-18--reference--dealflow-pipeline-status.md]
[^22]: TOPOS Categorical Architecture. [Source: docs/TOPOS_CATEGORICAL_ARCHITECTURE.md]
[^23]: APN Integration Architecture. [Source: docs/APN_INTEGRATION_ARCHITECTURE.md]
[^24]: BACKLOG Remaining Work. [Source: planning/BACKLOG--remaining-work.md]
[^25]: Open Core vs SaaS Business Model. Teleport. https://goteleport.com/blog/open-core-vs-saas-business-model/

---

*This roadmap is a living document. Review monthly, revise quarterly. Speed as a habit means this plan changes when reality changes.*

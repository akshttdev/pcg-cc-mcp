# ORCHA Architecture Gaps Analysis

**Date:** 2026-03-19 (updated with codebase verification pass)
**Branch:** `research/5-year-product-roadmap`
**Scope:** Comprehensive review of current implementation vs roadmap targets
**Based on:** 25 research reports (codebase analysis + web research), verified against actual code

---

## Executive Summary

ORCHA is a **155K-line frontend + 44K-line backend** platform with production-grade foundations across CRM, project management, AI agents, workflows, and sovereign data. However, significant architecture gaps exist between the current implementation and the 5-year roadmap targets. This document catalogs every identified gap, organized by roadmap stage and architectural domain, with severity ratings and recommended approaches informed by current industry research.

**Key findings:**
- **1 P0 blocker** (Agent Flow Orchestration Engine) blocks Stage 0 exit
- **11 critical gaps** block revenue readiness (billing, rate limiting, production deployment, etc.)
- **20 significant gaps** affect pilot readiness (Stage 1)
- **40+ strategic gaps** mapped to Stages 2-5

**Corrections from verification pass (2026-03-19):**
- MCP servers: ORCHA has **4 MCP servers** (TaskServer, TopsiServer, NoraServer, PulseServer) with 26+ tools — previously reported as "missing"
- Workflow scheduling: `spawn_workflow_schedule_loop()` runs every 5 min — previously reported as "manual-only"
- Node types: **19+ implemented** (not "11 of 17") — conditional, http_request, CRM nodes all exist
- Cost tracking: **Dual system** — VibeTransaction records costs properly; TokenUsage.cost_cents is never populated (ai-usage page shows $0)
- Marketplace: **functional** with VIBE ledger charging — not a stub
- billing_helpers: **backend business logic** (ledger checks, transaction recording) — not UI-only

---

## Table of Contents

1. [Stage 0 Gaps: Dogfood & Capture](#stage-0-gaps-dogfood--capture)
2. [Stage 1 Gaps: First Pilots & Pre-Seed](#stage-1-gaps-first-pilots--pre-seed)
3. [Stage 2 Gaps: SaaS Launch & Seed](#stage-2-gaps-saas-launch--seed)
4. [Stages 3-5 Gaps: PMF Through Sovereignty](#stages-3-5-gaps-pmf-through-sovereignty)
5. [Cross-Cutting Architecture Gaps](#cross-cutting-architecture-gaps)
6. [Gap-by-Domain Matrix](#gap-by-domain-matrix)
7. [Competitive Landscape Implications](#competitive-landscape-implications)
8. [Recommended Priority Stack](#recommended-priority-stack)

---

## Stage 0 Gaps: Dogfood & Capture

**Target:** Make ORCHA the operating system for PCG's internal delivery
**Exit criteria:** PCG using ORCHA for 100% of workflows, 3+ complete client cycles, CAPO < $1/action

### P0 — Agent Flow Orchestration Engine (BLOCKER)

**Current state:** Data model exists (`agent_flow.rs`, `agent_flow_event.rs`, `agent_task_plan.rs`) but **no background worker** to progress agent flows through phases. Routes exist (`agent_flows.rs`, `agent_flow_events.rs`) for CRUD and event streaming, but the actual orchestration loop is missing.

**Gap:** Without a background worker that monitors flow state, evaluates phase transitions, and triggers agent execution, the entire multi-agent workflow concept is UI-only. This is the single blocker for Stage 0 exit.

**What's needed:**
- Tokio background task polling for eligible flows (similar pattern to `spawn_workflow_schedule_loop()` in `data_source_workflows.rs` which already runs every 5 min for workflow triggers)
- Phase transition logic with guard conditions
- Agent dispatch (selecting executor, passing context)
- Event emission for SSE streaming to frontend
- Error recovery with retry/fallback/circuit-break patterns

> **Note:** `TaskScheduler` in `crates/server/src/task_scheduler.rs` (310 lines) is **dead code** — never called from `main.rs`. It monitors tasks with `assigned_agent` but was never wired into the startup sequence. Could be adapted or replaced.

**Industry context:** The Jido framework (Elixir) demonstrates OTP-style supervision for agent processes — each agent as a lightweight supervised process with crash recovery. Translatable to Rust/Tokio with structured concurrency patterns. Microsoft Agent Framework uses graph-based orchestration with declarative definitions.

**Estimated effort:** 2-3 weeks

### GAP-S0-01: Workflow Trigger System — Hardening (Partially Implemented)

**Current state:** ~~Workflow execution is manual-only.~~ **CORRECTION:** Scheduled workflow execution IS implemented:
- `spawn_workflow_schedule_loop()` in `data_source_workflows.rs` (lines 1556-1661) runs as a background Tokio task
- Polls every 5 minutes via `tokio::time::interval(Duration::from_secs(300))`
- Calls `WorkflowTrigger::find_due_schedules()` for due workflows
- Supports intervals: `every_5m`, `every_15m`, `every_30m`, `hourly`, `daily`, `weekly`
- Webhook triggers work with HMAC-SHA256 validation (`x-webhook-signature` header)
- `data_source_created` and `data_source_updated` event triggers also exist

**Remaining gaps:**
- **Cooldown race condition**: cooldown check happens before async spawn, but `last_triggered_at` updates after execution starts — concurrent webhooks can double-fire (no database-level lock)
- **No event bus** for internal triggers (e.g., task status change → workflow start)
- **No durable execution** — if server restarts mid-workflow, state is lost

**Industry context:** Temporal.io provides durable workflow execution with built-in cron scheduling and signal-based triggers. The cooldown race condition is a common pattern — Inngest solves it with idempotency keys.

### GAP-S0-02: Workflow Node Type Completeness — Mostly Resolved

**Current state:** ~~11 of 17 planned node types implemented.~~ **CORRECTION:** **19+ node types verified** in codebase:

**Implemented (verified in `node-types.ts` + `workflow_execution.rs`):**
- LLM: `llm_extract`, `llm_analyze`, `llm_summarize`
- Data: `data_source`, `transform`, `filter`, `merge`
- CRM Output: `output_crm_contacts`, `output_crm_companies`, `output_crm_deals`, `output_tasks`
- Control: `conditional` (if/then/else branching)
- Actions: `send_notification`, `assign_to_agent`, `http_request` (external API calls), `update_crm_contact`, `update_crm_deal`, `update_crm_company`

**Still missing:**
- ❌ Loop/iteration nodes (for processing arrays)
- ❌ Parallel execution (fan-out/fan-in)
- ❌ Human approval gates / manual review nodes
- ❌ Sub-workflow invocation
- ❌ Wait/delay nodes
- ❌ Error handling/catch nodes

**Impact:** Core workflow operations are well-covered. Missing nodes matter for advanced automation (batch processing, multi-path execution, human-in-the-loop).

### GAP-S0-03: Per-Task Cost Tracking (CAPO) — Dual System Misalignment

**Current state:** Two parallel cost systems exist but are **not connected**:

1. **TokenUsage table** (`token_usage.rs`): Has `cost_cents` field and aggregation queries (`total_cost_cents` via SUM), but `cost_cents` is **never populated during INSERT** — defaults to NULL. The `ai-usage.tsx` page reads from this table and **displays $0.00 for all costs**.

2. **VibeTransaction table** (`vibe_pricing.rs`, `billing.rs` helpers): Properly calculates costs via `ModelPricing.calculate_cost()` (input tokens × price per million), records to `VibeTransaction` with `calculated_cost_cents`. Backend helpers `ensure_vibe_balance()` and `record_llm_vibe_usage()` do real cost tracking.

**Gap:** The frontend cost dashboard reads from the wrong table. Cost data exists in VibeTransactions but the ai-usage page queries TokenUsage.cost_cents (always NULL).

**What's needed:**
- **Bridge the dual system**: Either populate `TokenUsage.cost_cents` at insert time using `ModelPricing.calculate_cost()`, or rewrite `ai-usage.tsx` to query `VibeTransaction` aggregates
- Aggregation by task, agent, workflow, organization (VibeTransaction already has project-level tracking)
- CAPO formula: sum(VibeTransaction.cost_cents) / count(completed_actions) per period

**Industry context:** Helicone, LangSmith, and Braintrust all provide per-call cost tracking. The pattern is: intercept LLM calls → record token counts + model ID → multiply by pricing table → aggregate. ORCHA's VibeTransaction system already follows this pattern — it just needs to feed the dashboard.

### GAP-S0-04: Cost-Tiered Model Routing

**Current state:** Multiple executors exist (Claude, Gemini, Qwen, Duck, Ollama) but routing is manual (user selects model). No automatic cheap→expensive escalation.

**Gap:** Research-derived backlog estimates 30-85% cost savings from automatic model routing. If 80% of traffic is simple queries, routing to 30x cheaper models is transformative.

**What's needed:**
- Intent classifier (can be a fast cheap model) that routes to appropriate tier
- Tier definitions: Haiku/Gemini Flash for simple tasks → Sonnet for moderate → Opus for complex
- Fallback chain: if cheap model fails quality check, escalate
- `pcg_router.rs` (622 lines) exists — may already have routing infrastructure to extend

### GAP-S0-05: Topology System (Commented Out)

**Current state:** `topology_node.rs`, `topology_edge.rs`, `topology_cluster.rs`, `topology_issue.rs`, `topology_route.rs` models exist but exports are commented out due to `sqlx prepare` requirements.

**Gap:** Topology visualization and routing (Topsi's core feature) is data-model complete but not active.

**What's needed:**
- Run `cargo sqlx prepare` with topology queries enabled
- Uncomment model exports
- Verify Topsi frontend pages work against topology data

---

## Stage 1 Gaps: First Pilots & Pre-Seed

**Target:** External validation with 3-5 paying pilot clients, raise pre-seed
**Key requirements:** Multi-tenant hardening, billing, documentation, production deployment

### GAP-S1-01: Billing System (Stripe Integration Missing)

**Current state:** No Stripe integration. No subscription management. No external payment processing. However, **internal billing infrastructure exists**:
- `billing.rs` helpers (backend, not UI): `ensure_vibe_balance()` pre-chat balance checks, `record_llm_vibe_usage()` post-chat VibeTransaction recording
- `model_pricing.rs`: Per-model pricing with `input_cost_per_million`, `output_cost_per_million`, and `multiplier` fields, plus `calculate_cost()` and `calculate_vibe_cost()` methods
- `vibe_pricing.rs`: `VibePricingService` with `estimate_cost()`, `record_llm_usage()`, budget checking
- `marketplace.rs` (546 lines): **Functional APN marketplace** with VIBE ledger — consumer deducts `vibe_cost`, provider credited 85%, usage history tracked
- `VibeTransaction` table records actual cost per LLM call

**Gap:** Internal VIBE economy works, but **cannot charge real money**. No Stripe, no credit card processing, no subscription tiers, no invoicing.

**What's needed:**
- Stripe integration: subscriptions, usage-based metering, invoicing
- Usage tracking: API calls, agent actions, workflow runs, compute minutes
- Billing page in settings UI
- Organization-level billing (not user-level)
- Free tier limits enforcement

**Industry context:** Hybrid billing (seat-based + usage-based) is the emerging standard for AI platforms. Vercel, Supabase, and PlanetScale all use this model. Stripe's metered billing API handles the complexity. Implementation pattern: track events → aggregate per billing period → report to Stripe → Stripe generates invoice.

### GAP-S1-02: Rate Limiting (Minimal)

**Current state:** TokenBucket implementation exists for Nora chat (20 req/min) and voice (30 req/min). No global rate limiting. No per-tenant limits. No nginx-level rate limiting.

**Gap:** Any user can exhaust server resources. No protection against abuse. No way to enforce tier-based usage limits.

**What's needed:**
- Global rate limiting at nginx level (connection limits, request rate)
- Per-tenant rate limiting at application level (tied to billing tier)
- Per-endpoint rate limiting for expensive operations (LLM calls, workflow runs)
- 429 responses with Retry-After headers

### GAP-S1-03: Production Deployment Pipeline

**Current state:** Manual `docker-compose up`. No automated deployment. No staging environment. No rollback procedures. No blue-green or canary deployment.

**Gap:** Cannot reliably deploy to production. No staging for testing before release. Rollbacks are manual and risky.

**What's needed:**
- CI/CD deployment to staging + production (GitHub Actions → Fly.io or similar)
- Database migration as part of deploy pipeline
- Health check verification before traffic cutover
- Rollback automation

### GAP-S1-04: Secrets Management

**Current state:** All secrets in `.env` files. No encryption at rest. No rotation policies.

**Gap:** Not enterprise-acceptable. Secrets visible in plaintext on disk.

**What's needed:**
- Vault integration (HashiCorp Vault, AWS Secrets Manager, or Doppler)
- Secret rotation for OAuth tokens, API keys
- Encrypted `.env` for development (age/sops)

### GAP-S1-05: User Documentation

**Current state:** No user-facing documentation. No API docs. No onboarding guides. Nginx serves `/docs` endpoint but no content exists.

**Gap:** Pilot customers need documentation. PCG team needs internal docs.

**What's needed:**
- API documentation (OpenAPI spec generation from Axum routes)
- User guide (onboarding, workflows, CRM, agents)
- Developer docs (MCP integration, webhook setup)

### GAP-S1-06: Multi-Tenant Isolation Hardening

**Current state:** Organization-based scoping via `organization_id` in queries. No Row-Level Security. Single SQLite shared by all orgs. Legacy projects with NULL `organization_id` visible to all users.

**Gap:** Medium risk of data leakage. SQL injection would expose all org data. NULL org projects are a vulnerability.

**What's needed (immediate):**
- Audit all queries for org scoping (ensure no cross-tenant leakage)
- Delete or assign NULL-org projects
- Add integration tests for tenant isolation

**What's needed (PostgreSQL migration):**
- Row-Level Security policies on all tenant-scoped tables
- `SET LOCAL app.current_tenant` per transaction
- Automated RLS testing

### GAP-S1-07: Monitoring & Alerting

**Current state:** `tracing` crate for structured logging to stdout. Sentry integration exists. Docker log rotation configured. No metrics, no alerting, no distributed tracing.

**Gap:** Cannot detect issues proactively. No visibility into performance. Cannot troubleshoot production issues effectively.

**What's needed:**
- OpenTelemetry integration (Rust SDK via `tracing-opentelemetry`)
- Metrics export (Prometheus endpoint)
- Alerting (PagerDuty/Slack for error spikes, latency, resource exhaustion)
- Dashboard (Grafana or hosted alternative)

**Industry context:** OpenTelemetry Rust SDK is mature (tracing-opentelemetry crate). For AI-specific observability, Helicone and Braintrust provide LLM call monitoring. The pattern is: instrument with OpenTelemetry → export to collector → visualize in Grafana/Datadog.

### GAP-S1-08: Graceful Shutdown

**Current state:** CancellationToken used in code. Tini as PID 1 in Docker. But no explicit drain period for in-flight requests. No graceful shutdown of SSE streams.

**Gap:** Deploy/restart can interrupt active agent executions, SSE streams, and workflow runs.

**What's needed:**
- Axum graceful shutdown with configurable drain timeout
- SSE stream cleanup on shutdown signal
- Agent execution checkpointing (resume after restart)
- Health check transitions: healthy → draining → stopped

### GAP-S1-09: Nora Voice MVP

**Current state:** Nora voice controls exist (`NoraVoiceControls.tsx`, 688 lines). Voice routes (`topsi/voice.rs`, 638 lines). VoiceGateway is **ACTIVE** (`crates/nora/src/voice/gateway.rs`). Chatterbox TTS configured. Discord voice integration via serenity.

**Gap per roadmap:** "Nora voice MVP" is a Stage 1 deliverable. Current state unclear on whether end-to-end voice conversations work reliably outside of Discord context.

**Needs verification:** End-to-end test of voice input → Nora processing → voice output (web, not just Discord).

### GAP-S1-10: Website/Hosting MVP

**Current state:** No public website. No hosted version. Desktop app only (Tauri).

**Gap:** Pilot customers need a URL to access, not a desktop app download.

**What's needed:**
- Hosted instance on Fly.io (Dockerfile.fly exists)
- Public website (landing page, pricing, signup)
- SSL/TLS via Cloudflare (tunnel config exists)

---

## Stage 2 Gaps: SaaS Launch & Seed

**Target:** Self-service signup, 20+ paying organizations
**Key requirements:** PostgreSQL migration, self-service onboarding, workflow marketplace

### GAP-S2-01: PostgreSQL Migration

**Current state:** SQLite with `SQLX_OFFLINE=true`. PostgreSQL feature-gated in code (`feature = "postgres"`). Dual auth paths exist (SQLite vs PostgreSQL). 215 SQLite migrations.

**Gap:** SQLite's single-writer limitation blocks multi-tenant scale. No RLS support. No connection pooling.

**Migration approach (from research):**
1. Use `pgloader` for data migration (handles SQLite→Postgres type mapping)
2. Convert BLOB UUIDs to native PostgreSQL UUID type (DbUuid abstraction simplifies this)
3. Enable RLS with `organization_id` as tenant discriminator
4. Add PgBouncer for connection pooling (transaction mode)
5. Use `SET LOCAL app.current_tenant` pattern for RLS context

**Pitfalls to watch:**
- SQLite permissive typing → Postgres strict types (data cleanup needed)
- BLOB UUID → native UUID conversion
- Boolean `0/1` → `true/false`
- JSON TEXT → native `jsonb`
- 215 migrations need translation (or fresh schema from models)

### GAP-S2-02: Self-Service Onboarding

**Current state:** WelcomeWizard (PR #48) handles initial setup for existing users. No public signup flow. No email verification. No self-service org creation.

**Gap:** Cannot acquire customers without sales touch.

**What's needed:**
- Public signup page with email verification
- Self-service organization creation
- Plan selection + Stripe checkout
- Initial data seeding (sample project, workflow templates)
- Onboarding wizard for new orgs

### GAP-S2-03: Workflow Marketplace/Templates

**Current state:** `workflow_template.rs` model exists. `marketplace.rs` (546 lines) is a **functional APN service marketplace** with VIBE ledger (not workflow templates). It handles service listings, subscriptions, gateway routing, and cost tracking (85% provider credit).

**Gap:** The existing marketplace is for **APN services**, not workflow templates. No shareable workflow templates. No template discovery UI. No import/export.

**What's needed:**
- Workflow template CRUD with versioning (separate from APN marketplace)
- Template gallery UI
- One-click install into organization
- Template categories and search

### GAP-S2-04: BDI + Contract Net Orchestration

**Current state:** Agent execution is direct dispatch (user selects agent → agent runs). No automatic task allocation. No agent bidding.

**Gap per roadmap:** Stage 2 targets "BDI + Contract Net orchestration for multi-agent coordination."

**What's needed:**
- Agent capability advertisement (what can each agent do?)
- Task routing based on capability matching
- Multi-agent coordination protocol (announce → bid → award)
- Agent performance tracking for routing optimization

**Industry context:** A2A (Agent-to-Agent) protocol from Google provides capability discovery via "Agent Cards." This is the modern equivalent of Contract Net's capability advertisement. MCP handles agent-to-tool, A2A handles agent-to-agent.

### GAP-S2-05: Graph Capabilities for Task Dependencies

**Current state:** `task_dependency.rs` model exists. No recursive query infrastructure for transitive dependency traversal. No DAG visualization. No cycle detection.

**Gap:** Cannot visualize or enforce task dependency chains. Critical path analysis not possible.

**Recommended approach (from research):**
- **Phase 1 (SQLite, now):** Adjacency table (`task_dependencies`) + recursive CTEs for traversal. SQLite supports recursive CTEs. Sufficient for hundreds to low thousands of tasks per project.
- **Phase 2 (PostgreSQL):** Same pattern with better query planning. Add Apache AGE extension only if Cypher-style queries become necessary.
- **No separate graph DB needed** for task dependencies, CRM networks, or workflow DAGs at this scale. PostgreSQL recursive CTEs handle the workload.
- **Frontend:** Add Cytoscape.js for dependency graph visualization. React Flow (likely already used for workflow editor) for DAG editing.

---

## Stages 3-5 Gaps: PMF Through Sovereignty

These are strategic gaps mapped to later roadmap stages. Listed for completeness.

### Stage 3: Product-Market Fit & VIBE Economy

| Gap | Current State | Target |
|-----|--------------|--------|
| ATLAS graph dependencies | `task_dependency.rs` exists, no traversal engine | Full DAG with critical path, blockers, propagation |
| Context compaction layer | No context management | 50-80% context cost reduction via ontological reduction |
| Voice-first production | Nora voice exists, not production-grade | Voice as primary interface for agents |
| Desktop app beta | Tauri app exists, builds in CI | Auto-update, code signing, crash reporting |
| Knowledge as first-class entity | `knowledge.rs` route exists | Persistent, queryable, cross-project knowledge base |
| Agent memory architecture | Agent conversations persist | 5-tier memory: working, episodic, semantic, procedural, shared |

### Stage 4: Growth & Series A

| Gap | Current State | Target |
|-----|--------------|--------|
| Docker service-per-agent | Single process | Container-per-agent with resource isolation |
| SudoLang constraint specs | No formal agent specs | PICS pattern (Preamble, Interface, Components, Start) |
| Advanced hosting | Docker Compose | GPU support, serverless, auto-scaling |
| 500+ organizations | Single SQLite | Multi-region PostgreSQL with RLS |

### Stage 5: Full Sovereignty

| Gap | Current State | Target |
|-----|--------------|--------|
| APN mesh production | Phase 1 (libp2p + NATS relay, functional bridge) | 100+ compute nodes, real workload distribution |
| VIBE token economy | Internal ledger functional (VibeTransaction, marketplace 85% split) | Live token settlement, BME economics |
| VIBELAND virtual environment | `virtual-environment/` page exists | Full 3D collaborative workspace |
| Federated agent marketplace | `marketplace.rs` functional (APN services, VIBE ledger) | Cross-org agent sharing with VIBE payments |
| Self-hosted deployment | Docker Compose | One-click sovereign deployment package |

---

## Cross-Cutting Architecture Gaps

### GAP-XC-01: Error Recovery Model

**Current state:** Binary success/fail. No structured retry, fallback, or escalation.

**Target (from research):** Five-level error recovery:
1. Retry (same agent, same model)
2. Fallback (same agent, different model)
3. Reassign (different agent)
4. Circuit break (pause workflow, alert operator)
5. Human escalate (create approval gate)

**Industry context:** Circuit breaker patterns are well-established in Rust (tower-layer, backon crate for retries). For LLM-specific circuit breaking, track failure rates per model/provider and automatically route around outages.

### GAP-XC-02: Structured Telemetry

**Current state:** `tracing` crate with text logging. Sentry for error tracking. No metrics, no spans, no cost attribution.

**Target:** Every agent action produces structured telemetry: model, tokens, latency, cost, success/failure, parent span.

**Industry context:** OpenTelemetry Rust SDK (`tracing-opentelemetry`) integrates directly with the existing `tracing` crate. For AI observability, layer cost tracking on top of OTel spans.

### GAP-XC-03: Prompt Caching Strategy

**Current state:** No prompt caching. System prompts resent with every request.

**Target (from research):** 90% savings on Anthropic cached input tokens, 50% on OpenAI. For agents with long system prompts, this means 40-70% monthly input cost savings.

**What's needed:**
- Cache-aware system prompt construction
- Stable prefix ordering (cacheable content first)
- Cache hit rate monitoring

### GAP-XC-04: Input Validation Framework

**Current state:** Serde + manual checks. No validator crate. Relies on type system + custom logic.

**Gap:** No comprehensive input validation at API boundary. OWASP risk.

**What's needed:**
- `validator` or `garde` crate for declarative validation
- Consistent validation errors (structured, actionable)
- Input sanitization for user-provided content entering LLM prompts (prompt injection defense)

### ~~GAP-XC-05: MCP Server Architecture~~ — RESOLVED (Previously Misreported)

**Current state:** ~~No MCP servers exposing ORCHA capabilities.~~ **CORRECTION: ORCHA has 4 fully implemented MCP servers:**

1. **TaskServer** (`crates/server/src/mcp/task_server/`, 2,400+ lines across 6 modules):
   - Binary: `crates/server/src/bin/mcp_task_server.rs`
   - **26+ tools**: `create_task`, `list_tasks`, `update_task`, `delete_task`, `assign_task`, `bulk_create_tasks`, `bulk_update_tasks`, `search_tasks`, `list_projects`, `list_project_members`, `scaffold_project`, `manage_task_dependencies`, `check_dependencies`, `add_knowledge`, `list_knowledge`, `manage_knowledge`, search across all entities
   - User-scoped access: `TaskServer::new_for_user(pool, user_id, is_admin)`
   - Registered in `default_mcp.json` as `orcha_task_server`

2. **TopsiServer** (`crates/server/src/mcp/topsi_server.rs`, ~27K bytes):
   - Binary: `crates/server/src/bin/mcp_topsi_server.rs`
   - Topology intelligence, orchestration, monitoring, policy enforcement

3. **NoraServer** (`crates/server/src/mcp/nora_server.rs`, ~26K bytes):
   - Strategic planning, task coordination, performance analysis

4. **PulseServer** (`crates/server/src/mcp/pulse_server.rs`, ~16K bytes):
   - Real-time metrics and event streaming

All use the `rmcp` crate (v0.5.0) with stdio transport per MCP spec. Frontend has MCP management UI (`McpSettings.tsx`). API endpoint `GET/POST /api/mcp-config` for configuration.

**Remaining gaps (smaller than originally assessed):**
- No Streamable HTTP transport (only stdio — limits remote access)
- No `.well-known` MCP Server Cards for registry discovery
- Not registered in official MCP Registry
- CRM operations not yet exposed as MCP tools (only task/project/knowledge)

### GAP-XC-06: A2A Protocol (Agent-to-Agent)

**Current state:** No A2A implementation. Agents communicate only through shared database state.

**Gap:** Multi-agent coordination requires standardized agent-to-agent communication. Google's A2A protocol is the emerging standard (150+ supporting organizations, governed by AAIF/Linux Foundation).

**What's needed (Stage 2+):**
- Agent Cards describing capabilities (JSON metadata)
- A2A message passing for task delegation
- Credential exchange between agents
- Discovery mechanism for available agents

### GAP-XC-07: Frontend Component Size Debt

**Current state:** 13 components exceed 800 lines (guideline is 500 max):
1. `NoraAssistant.tsx` — 1027 lines
2. `StagingReviewPanel.tsx` — 933 lines
3. `topsi/meeting-mode/index.tsx` — 887 lines
4. `workflows/editor/index.tsx` — 864 lines
5. `vibeland/UserAvatar.tsx` — 846 lines
6. `TaskDetailsPanel.tsx` — 837 lines
7. `WelcomeWizard.tsx` — 783 lines
8. `DisplayConversationEntry.tsx` — 762 lines
9. `Toolbar/CurrentAttempt.tsx` — 750 lines
10. `AgentChatConsole.tsx` — 739 lines
11. `CrmPipelineSettings.tsx` — 729 lines
12. `NodeConfigPanel.tsx` — 727 lines
13. `Sidebar.tsx` — 708 lines

**Impact:** Harder to maintain, test, and review. Increases merge conflict probability.

### GAP-XC-08: Backend Route File Size Debt

**Current state:** Several route files exceed 1000 lines:
1. `crm_deals.rs` — 2923 lines
2. `data_source_workflows.rs` — 1691 lines
3. `tasks.rs` — 1572 lines
4. `intelligence.rs` — 1443 lines
5. `workflow_staging.rs` — 1401 lines
6. `meet.rs` — 1372 lines
7. `projects.rs` — 1180 lines

### GAP-XC-09: Unit Test Coverage

**Current state:** E2E tests via Playwright (strong, 50+ specs). Minimal unit/integration tests in Rust crates. No frontend unit tests.

**Gap:** Changes to business logic require full E2E runs to validate. No fast feedback loop.

**What's needed:**
- Unit tests for critical business logic (cost calculation, RLS queries, workflow transitions)
- Frontend unit tests for hooks and utility functions (vitest)
- Integration tests for API endpoints

### GAP-XC-10: CI Pipeline Strictness

**Current state:** Clippy and tests use `continue-on-error: true` — failures don't block merges.

**Gap:** Code quality issues and test failures can reach main.

**What's needed:**
- Remove `continue-on-error` from clippy and test steps
- Add E2E test execution to CI (at least smoke tests)
- Add security scanning (SAST, dependency scanning)

---

## Gap-by-Domain Matrix

| Domain | Stage 0 | Stage 1 | Stage 2 | Stage 3+ |
|--------|---------|---------|---------|----------|
| **Agent Orchestration** | P0 flow engine, model routing | Voice MVP, supervision | BDI/Contract Net, A2A | Memory tiers, OTP supervision |
| **Workflows** | Loop/parallel/approval nodes | Templates | Marketplace, versioning | FSM engine, event sourcing |
| **CRM** | — (85% complete) | Multi-tenant hardening | Self-service setup | Relationship intelligence |
| **Task Management** | Cost tracking (CAPO) | Dependency visualization | DAG traversal engine | ATLAS graph, cross-project |
| **Database** | — | Audit queries, null org cleanup | PostgreSQL + RLS migration | Multi-region, graph patterns |
| **Infrastructure** | — | Deploy pipeline, monitoring, secrets | Auto-scaling, staging env | Container-per-agent, GPU |
| **Billing** | Bridge VIBE→dashboard | Stripe integration | Usage metering, tiers | Marketplace transactions |
| **Security** | — | Rate limiting, input validation | RLS, SAST/DAST | WAF, SOC2 |
| **Observability** | — | OpenTelemetry, alerting | Cost dashboards | Agent performance analytics |
| **MCP/Protocols** | 4 servers exist (stdio) | HTTP transport, CRM tools | A2A implementation | Registry presence |
| **APN/Mesh** | — | — | Testnet validation | Production mesh, VIBE tokens |
| **Frontend** | — | Component splits, docs | i18n, mobile, a11y | VIBELAND 3D |

---

## Competitive Landscape Implications

Based on web research across AI orchestration, CRM, PM, and sovereign compute:

### Unique Positioning (Defensible)
1. **Full-stack integration** — No competitor combines CRM + PM + agents + workflows + sovereign deployment. Notion adds agents but has no CRM. Asana adds AI but has no workflow orchestration. CrewAI/LangGraph are pure frameworks.
2. **Sovereign/self-hosted** — $22.58B market growing 14.6% YoY. EU AI Act + data residency requirements driving demand. Microsoft investing heavily in sovereign cloud.
3. **MCP-native** — MCP has won the protocol war (97M monthly SDK downloads, 18K+ servers, AAIF governance). ORCHA already has **4 MCP servers** (26+ tools) and MCP client integration in all executors — this is ahead of most competitors.
4. **Rust performance** — Rust MCP servers show 180ms response vs 3.2s for TypeScript under concurrent load. Backend performance is a structural advantage.
5. **Categorical architecture** — TOPOS theory foundation provides formal composability claims that no competitor can match.

### Risks
1. **Agent orchestration convergence** — Microsoft Agent Framework, LangGraph, and CrewAI are all converging on graph-based orchestration. ORCHA must ship the flow engine before these become commoditized.
2. **MCP gateway commoditization** — Gateway vendors are adding MCP-specific features. ORCHA's MCP integration must go deeper than commodity gateways.
3. **Pricing pressure** — AI coding tools (Cursor, Windsurf) are racing to the bottom on pricing. ORCHA's value must be in orchestration, not just AI access.

---

## Recommended Priority Stack

### Immediate (Weeks 1-4) — Unblock Stage 0

1. **Agent Flow Orchestration Engine** — background worker, phase transitions, SSE events (P0 blocker)
2. **CAPO tracking** — bridge VibeTransaction cost data to ai-usage dashboard (data exists, just wrong table)
3. **CI strictness** — remove `continue-on-error: true` from clippy/test steps (P0 quality gate)
4. **Topology system** — uncomment models, run sqlx prepare
5. **Workflow cooldown fix** — database-level lock for trigger deduplication

### Short-term (Weeks 5-12) — Revenue Readiness

6. **Stripe billing integration** — subscriptions + usage metering (bridge to existing VIBE economy)
7. **Rate limiting** — global + per-tenant + per-endpoint
8. **Production deployment pipeline** — CI/CD to hosted instance
9. **Monitoring** — OpenTelemetry + alerting (extend existing Sentry with metrics)
10. **User documentation** — API docs + onboarding guide
11. **Multi-tenant audit** — verify org scoping, fix NULL org projects

### Medium-term (Months 3-6) — Pilot Scale

11. **Model routing** — automatic cheap→expensive escalation
12. **PostgreSQL migration** — pgloader, RLS, connection pooling
13. **MCP server HTTP transport** — add Streamable HTTP to existing 4 MCP servers for remote access; add CRM tools to TaskServer
14. **Error recovery** — 5-level recovery model (retry→fallback→reassign→circuit break→escalate)
15. **Task dependency DAG** — recursive CTEs + Cytoscape.js visualization

### Long-term (Months 6-12) — SaaS Launch

16. **Self-service onboarding** — public signup, plan selection, Stripe checkout
17. **Workflow marketplace** — template sharing, one-click install
18. **A2A protocol** — agent-to-agent communication for multi-agent coordination
19. **Prompt caching** — 40-70% input cost savings
20. **Graph capabilities** — PostgreSQL recursive CTEs for knowledge graph patterns

---

## Supporting Research Files

All underlying research is available in `planning/roadmap/research/`:

| # | File | Content |
|---|------|---------|
| 01 | `01-state--planning-docs-summary.md` | Full roadmap, state assessment, backlog synthesis |
| 02 | `02-arch--backend.md` | 20 workspace members, 120 routes, 140+ models, auth, real-time |
| 03 | `03-arch--frontend.md` | 765 files, ~60 pages, 68 component dirs, state management |
| 04 | `04-infra--cicd-gaps.md` | CI/CD, Docker, security, production readiness |
| 05 | `05-infra--deployment-hosting.md` | Fly.io, Cloudflare Tunnel, Docker Compose, hosting options |
| 06 | `06-data--database-migration.md` | SQLite→PostgreSQL, RLS, connection pooling, analytics |
| 07 | `07-data--graph-databases.md` | Neo4j, SurrealDB, AGE, GraphRAG, visualization libraries |
| 07b | `07b-data--postgresql-graph-patterns.md` | PostgreSQL recursive CTEs, graph patterns, AGE extension |
| 08 | `08-legal--compliance-security.md` | GDPR, SOC2, AI Act, prompt injection, dependency scanning |
| 09 | `09-oss--infra-billing-tools.md` | Permissive license infra & billing tools |
| 10 | `10-ai--mcp-protocol-ecosystem.md` | MCP spec, 18K+ servers, AAIF governance, Rust SDK |
| 11 | `11-ai--agent-orchestration.md` | CrewAI/LangGraph/AutoGen, A2A, OTP supervision, cost optimization |
| 12 | `12-ai--workflow-engines.md` | Temporal, Inngest, Restate, FSM, event sourcing, DAG execution |
| 13 | `13-oss--agent-workflow-tools.md` | MCP/ACP compatible OSS tools (permissive license) |
| 14 | `14-ux--voice-email-desktop.md` | Voice UX, email integration, desktop app patterns |
| 15 | `15-biz--saas-billing-patterns.md` | Hybrid billing, Stripe metering, free tier economics, PLG |
| 15b | `15b-biz--saas-billing-supplemental.md` | Supplemental billing research |
| 16 | `16-ops--observability-monitoring.md` | OpenTelemetry Rust, AI observability, circuit breakers |
| 17 | `17-p2p--decentralized-compute.md` | Akash/Render/Nosana, DePIN, token economics, libp2p, NATS |
| 18 | `18-p2p--oss-mesh-tools.md` | Permissive license P2P/crypto/mesh tools |
| 19 | `19-ops--testing-strategy.md` | Testing strategy, E2E, integration, unit test gaps |
| 20 | `20-data--onboarding-etl.md` | Onboarding flows, data ETL patterns |
| 21 | `21-infra--build-performance-dx.md` | Build performance, DX improvements, cargo-chef |
| 22 | `22-biz--product-analytics.md` | Product analytics, telemetry, usage tracking |
| 23 | `23-arch--offline-local-first.md` | Offline/local-first architecture patterns |
| 24 | `24-ai--safety-guardrails.md` | AI safety, guardrails, alignment, red teaming |
| 25 | `25-team--scaling-api-design.md` | API versioning, team scaling, rate limiting |

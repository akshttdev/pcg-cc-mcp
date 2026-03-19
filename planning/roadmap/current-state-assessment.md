# ORCHA Platform — Current State Assessment

**Document Type**: Day 2 Deliverable — 5-Year Product Roadmap Research Sprint
**Date**: 2026-03-18
**Version**: 1.0
**Author**: Roadmap Research Sprint

---

## 1. Executive Summary

ORCHA is a sovereign full-stack project management and AI orchestration platform built for Powerclub Global (PCG). It combines agency deal flow, AI agent orchestration, CRM pipeline management, workflow automation, and distributed compute via APN mesh networking into a single integrated platform.

As of 2026-03-18, the platform consists of:

- **20 workspace members** (18 Rust crates + 2 Tauri apps) forming the backend monorepo
- **60+ frontend pages** and **311+ React components** in the TypeScript/React frontend
- **215 database migrations** against SQLite (PostgreSQL feature-gated)
- **4 MCP servers** exposing **26+ tools** for agent interaction
- **8 executor backends** enabling multi-provider AI agent orchestration
- **9 named agents** (Nora, Topsi, Scout, Astra, Cash, Lux, Maci, Auri, Editron)
- **9 social platform integrations** for content pipeline distribution

The platform is in late alpha — CRM pipeline and manual workflow execution are dogfood-ready, but the Agent Flow Orchestration Engine (the background worker that drives autonomous multi-agent workflows) remains the critical P0 blocker for full operational use. Revenue readiness is estimated at 8-12 weeks beyond current state.

---

## 2. Feature Completion Matrix

| System | Completion | Status | Blocking Issues |
|--------|-----------|--------|-----------------|
| **CRM Pipeline (9-stage)** | 85% | Functional | F11: call scheduling display-only; F3: won stage person invite stub; Agent Flow Engine missing |
| **Agentic Intake Pipeline** | 100% | Complete | None |
| **Workflow Automation** | 80% | Functional | ~~Manual run only~~ Schedule triggers run every 5m (interval: 5m/15m/30m/hourly/daily/weekly); webhook + event triggers exist; 19+ node types implemented; missing: loop, parallel, approval gates, sub-workflow |
| **Agent Orchestration** | 60% | Partial | **P0 BLOCKER**: No Agent Flow Orchestration Engine (data model exists, no background worker) |
| **Task Management** | 80% | Functional | No subtask hierarchy UI; no dependency visualization; no Gantt/DAG view |
| **Notification System** | 95% | Complete | Minor: Activity button text includes count badge [Source: MEMORY.md#E2E Testing] |
| **Content Pipeline** | 75% | Functional | 9 platforms wired; post composer + review queue done; scheduling partial |
| **Sovereign Data/Intelligence** | 65% | Partial | 59% files unclassified; Phase 2 (enrichment, search, dashboards) not started [Source: 2026-03-17--plan--intelligence-roadmap.md] |
| **APN Mesh Network** | 40% | Phase 1 | Mesh + crypto + heartbeat done; task distribution, remote execution, economics pending |
| **VIBE Token Economy** | 40% | Partial | Internal VIBE ledger operational (VibeTransaction, ModelPricing, marketplace with 85% provider credit); no on-chain settlement; no external payment processing |
| **Authentication/RBAC** | 70% | Functional | No session timeout, no CSRF; role_based_ui exists but not enforced |
| **E2E Testing** | 80% | Good | 30 passed, 3 failed (env), 5 skipped; 28/31 demo tests passing [Source: MEMORY.md#E2E Testing] |
| **CI/CD Pipeline** | 50% | Basic | GitHub Actions exists (fmt, clippy, test, audit); no deployment pipeline |
| **Documentation** | 60% | Partial | Architecture docs good; no API docs, no user docs |

### Completion Summary

- **Complete (90%+)**: 2 systems (Agentic Intake, Notifications)
- **Functional (70-89%)**: 5 systems (CRM, Task Mgmt, Content, E2E Testing, Auth)
- **Partial (50-69%)**: 3 systems (Workflow, Agent Orchestration, Sovereign Data, Documentation)
- **Early/Stub (<50%)**: 3 systems (APN Mesh, VIBE Token, CI/CD)

---

## 3. External Dependency Inventory

| Dependency | Purpose | Cost | Migration Difficulty | Risk |
|-----------|---------|------|---------------------|------|
| **Claude API (Anthropic)** | Primary LLM for agents | $5-25/MTok | Medium (multi-provider routing possible) | Vendor pricing changes |
| **SQLite** | Primary database | Free | **HIGH** (202 migrations to port to PostgreSQL) | Scale ceiling |
| **GitHub OAuth** | Authentication | Free | Low (standard OAuth) | Platform dependency |
| **Dropbox** | Sovereign data source | $12-20/mo | **HIGH** (45K+ files indexed, one-way sync architecture) | API changes, rate limits |
| **SendGrid** | Email intake | Free tier | Low (standard SMTP) | Tier limits |
| **Twilio** | Call/SMS intake | Pay-per-use | Low (standard API) | Cost scaling |
| **NATS** | Event messaging | Free (self-hosted) | Medium (custom event schema) | Ops overhead |
| **Flox** | Dev environment | Free | Medium (could use Nix directly) | Niche tooling |
| **sccache** | Build caching | Free | None (optional) | None |
| **Sentry** | Error tracking | Free tier | Low (standard SDK) | Tier limits |
| **Docker** | Containerization | Free | None | None |
| **Cloudflare Tunnel** | Port forwarding | Free tier | Low | None |
| **ComfyUI** | Image generation | Free (self-hosted) | N/A (not fully wired yet) | Integration incomplete |
| **Aptos** | VIBE settlement | Free (testnet) | N/A (stubs only) | Blockchain viability |
| **ts-rs (custom fork)** | Type generation | Free | Medium (custom xazukx/ts-rs branch) | Fork maintenance burden |
| **libp2p** | P2P mesh networking | Free | **HIGH** (core APN dependency) | Protocol complexity |

### Dependency Risk Profile

- **3 HIGH migration difficulty**: SQLite, Dropbox, libp2p — these are load-bearing and deeply integrated
- **3 MEDIUM migration difficulty**: Claude API, NATS, ts-rs fork — abstraction layers exist or could be built
- **10 LOW/NONE migration difficulty**: Standard integrations with clean interfaces
- **Monthly fixed cost**: ~$12-20 (Dropbox only); all other costs are usage-based or free
- **Custom fork risk**: `ts-rs` on `xazukx/ts-rs` branch requires ongoing maintenance [Source: MEMORY.md#Technology Stack Summary]

---

## 4. Technology Stack Summary

### Backend

| Layer | Technology | Version |
|-------|-----------|---------|
| Language | Rust (nightly-2025-05-18) | — |
| Web framework | Axum | 0.8.4 |
| Async runtime | Tokio | — |
| Database driver | SQLx | 0.8.6 |
| Database | SQLite (PostgreSQL feature-gated) | — |
| Build caching | sccache | — |
| Linker | lld | — |

### Frontend

| Layer | Technology | Version |
|-------|-----------|---------|
| Framework | React | 18 |
| Language | TypeScript | 5.9 |
| Build tool | Vite | 5.4 |
| CSS | Tailwind CSS | 3.4 |
| UI primitives | Radix UI + shadcn/ui | — |
| State management | Zustand (15 stores) | — |
| Data fetching | TanStack Query | — |

### Infrastructure

| Layer | Technology | Notes |
|-------|-----------|-------|
| Desktop | Tauri | 2.9 |
| Real-time | SSE + WebSocket + NATS | SSE primary, WS for specific features |
| MCP | 4 servers (Task, Nora, Topsi, Pulse) | 26+ tools, 6 resources |
| AI Executors | 8 backends | Claude, Gemini, Qwen, Cursor, CodeX, Duck, AMP, OpenCode |
| P2P | APN (libp2p) | gossipsub, Kademlia, mDNS |
| Dev environment | Flox | manifest.toml-driven |
| CI | GitHub Actions | fmt, clippy, test, audit |
| Containers | Docker | 6 services |

### Agent Ecosystem

| Agent | Role |
|-------|------|
| Nora | Primary AI assistant |
| Topsi | Topic/content specialist |
| Scout | Research/discovery |
| Astra | Strategic analysis |
| Cash | Financial/deal analysis |
| Lux | Design/creative |
| Maci | Marketing/content |
| Auri | Audio/media |
| Editron | Code editing |

---

## 5. Codebase Metrics

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
| MCP servers | 4 |
| MCP tools | 26+ |
| Executor backends | 8 |
| Named agents | 9 |
| Social platforms | 9 |
| Docker services | 6 |
| Frontend version | 0.0.55 |
| Backend version | 0.0.96 |

### Codebase Health Indicators

| Indicator | Value | Assessment |
|-----------|-------|------------|
| E2E tests passing | 30/33 (91%) | Good |
| Demo tests passing | 28/31 (90%) | Good |
| Remaining `: any` types | 29 | Acceptable (library-level constraints) [Source: MEMORY.md#Backlog] |
| Remaining `Path<Uuid>` | 84 | Needs migration to DbUuid [Source: MEMORY.md#Backlog] |
| Inline query keys | 15 | Minor tech debt |
| Rust warnings (nora/alpha) | 27 | Structural dead code [Source: MEMORY.md#Backlog] |

---

## 6. Technical Debt Summary

### Critical (4 items)

| # | Issue | Impact | Remediation |
|---|-------|--------|-------------|
| C1 | 100+ `.unwrap()` calls in production Rust | Crash risk — any unwrap on unexpected data panics the server | Replace with proper error handling; top 30 first |
| C2 | SQLite not suitable for production scale | Single-writer bottleneck; no concurrent write support | PostgreSQL migration (Phase 3) |
| C3 | Global Mutex deadlock risk in worktree manager | Server hang under concurrent access | Phase 0 fix (< 1 day) |
| C4 | No CI pipeline for production deployment | Manual deployment only; no automated rollout/rollback | Phase 1 (1 week) |

### High (9 items)

| # | Issue | Impact |
|---|-------|--------|
| H1 | No rate limiting on most endpoints | DDoS/abuse vulnerability |
| H2 | No input validation framework | Injection and malformed data risk |
| H3 | Missing database indexes | Query performance degradation at scale |
| H4 | N+1 query patterns | API latency under load |
| H5 | Monolithic files (api.ts 6.6K lines, org-profile 7K) | Developer velocity drag [Source: MEMORY.md#Backlog] |
| H6 | 5 custom hooks > 300 lines each | Maintenance complexity |
| H7 | Missing graceful shutdown | Data corruption risk on restart |
| H8 | No frontend unit tests | Regression risk on UI changes |
| H9 | Missing structured logging | Debugging and observability gaps |

### Debt Remediation Timeline

| Phase | Scope | Duration | Key Items |
|-------|-------|----------|-----------|
| **Phase 0** | Emergency fixes | < 1 day | Deadlock fix (C3), HTTP client timeout, error boundaries |
| **Phase 1** | Foundation | 1 week | CI pipeline (C4), rate limiting (H1), top-30 unwraps (C1), structured logging (H9) |
| **Phase 2** | Stabilization | 2-3 weeks | Graceful shutdown (H7), index audit (H3), modularize monoliths (H5), validation (H2) |
| **Phase 3** | Scale readiness | 1-2 months | Remaining unwraps (C1), PostgreSQL migration (C2), frontend tests (H8) |

**Production-minimum (Phase 0-2): 4-6 weeks**

---

## 7. Dogfooding Readiness Checklist

| # | Requirement | Status | Notes |
|---|-------------|--------|-------|
| 1 | Login/auth works reliably | PASS | GitHub OAuth device flow |
| 2 | CRM pipeline end-to-end | PARTIAL | 85% complete — F11 call scheduling display-only, F3 won stage invite stub |
| 3 | Agent task execution | PASS | MCP tools + executor wiring done |
| 4 | Agent flow orchestration | **FAIL** | **P0 BLOCKER** — data model exists, no background worker |
| 5 | Workflow manual run | PASS | End-to-end working |
| 6 | Workflow triggers | FAIL | No cron/event triggers |
| 7 | Task creation/management | PASS | Full CRUD + kanban board |
| 8 | Notifications | PASS | Bell + toast + activity page |
| 9 | Data source to staging to CRM | PASS | Full pipeline working |
| 10 | Real-time updates (SSE) | PASS | Process logs, task diffs |
| 11 | Multiple orgs | PASS | Org switcher + scoping [Source: MEMORY.md#CRM Architecture] |
| 12 | Seed data for testing | PARTIAL | Sparse — needs 5-10 contacts, deals minimum |
| 13 | E2E test coverage | PARTIAL | 28/31 demos, 30/33 e2e passing [Source: MEMORY.md#E2E Testing] |
| 14 | Error handling graceful | PARTIAL | Error boundaries added but unwraps remain in backend |
| 15 | Performance acceptable | PASS | Workflow exec 300-400ms |

### Dogfooding Verdict

**Can dogfood NOW**: CRM pipeline management, manual workflow execution, task management, content pipeline, notification-driven workflows.

**Cannot dogfood yet**: Autonomous agent flow orchestration (P0 blocker), trigger-based workflow automation.

The platform is usable for its core agency management functions today. The critical gap is the Agent Flow Orchestration Engine — the background worker that would allow agents to autonomously progress through multi-step workflows without manual intervention.

---

## 8. Revenue Readiness Checklist

| # | Requirement | Status | Notes |
|---|-------------|--------|-------|
| 1 | Multi-tenant isolation | PARTIAL | Org-scoped queries but no tenant resource limits or quotas |
| 2 | Billing infrastructure | FAIL | None — no payment processing, no subscription management |
| 3 | User onboarding flow | PASS | WelcomeWizard implemented (PR #48) [Source: MEMORY.md#UX Gaps] |
| 4 | API rate limiting | FAIL | Only on Nora routes; all other endpoints unprotected |
| 5 | Data backup/recovery | PARTIAL | Docker backup service exists; no automated schedule or verification |
| 6 | SSL/TLS in production | PARTIAL | Cloudflare tunnel handles termination |
| 7 | Monitoring/alerting | PARTIAL | Sentry + Prometheus basic; no SLA alerting |
| 8 | SLA-grade uptime | FAIL | No production deployment pipeline; no health checks; no failover |
| 9 | Usage metering | FAIL | No per-tenant tracking of API calls, storage, or compute |
| 10 | Documentation (user-facing) | FAIL | None — no user guides, no API reference, no help center |

### Revenue Verdict

**NOT revenue-ready.** The platform lacks the fundamental infrastructure required for paid customers:

- No billing or payment processing
- No usage metering or resource limits
- No production deployment pipeline
- No user-facing documentation
- Rate limiting only on agent routes

**Estimated timeline to minimum viable revenue**: 8-12 weeks, assuming parallel work on billing integration, production deployment, rate limiting, monitoring, and basic user documentation.

---

## 9. Fundraising Readiness Checklist

| # | Requirement | Status | Notes |
|---|-------------|--------|-------|
| 1 | Working demo | PASS | 28/31 demo tests passing; full CRM + workflow walkthrough possible |
| 2 | Architecture documentation | PASS | TOPOS categorical architecture + APN mesh networking docs |
| 3 | Research foundation | PASS | 9 research papers published [Source: planning/2026-03-18--research--*] |
| 4 | Differentiation story | PASS | Sovereign data + MCP-native agents + multi-agent orchestration + APN mesh |
| 5 | Competitive analysis | PARTIAL | Needs formal write-up comparing against CrewAI, LangGraph, AutoGen, etc. |
| 6 | Team credentials | PASS | PCG agency track record |
| 7 | Market sizing data | PASS | AI agent market $7.8B growing to $52.6B by 2030 |
| 8 | Unit economics model | PARTIAL | Research exists; needs productized model with margin analysis [Source: 2026-03-18--research--unit-economics-agent-metrics.md] |
| 9 | IP/moat analysis | PARTIAL | APN + TOPOS architecture are differentiators; needs formal moat documentation |
| 10 | Revenue traction | FAIL | No revenue; no paying customers |

### Fundraising Verdict

**Demo-ready for pre-seed/angel.** The platform has a compelling technical demo, strong research foundation, and clear differentiation story. The TOPOS categorical architecture and APN mesh networking provide genuine technical moats.

**For seed round**: Needs revenue traction (even pilot customers), polished competitive analysis, and formalized unit economics model. The gap between current state and seed-ready is primarily commercial validation, not technical.

---

## 10. Key Architecture Decisions & Constraints

These are binding architectural decisions that constrain the roadmap. Reversing any of these would require significant rework.

### Decision 1: CRM Org-Scoped (not Project-Scoped)

**Status**: Migration complete (2026-03-09)
**Implication**: All CRM entities (contacts, deals, pipelines) are scoped to `organization_id` with optional `client_id`. `project_id` removed from all Create structs. This means CRM data is shared across projects within an organization, enabling cross-project deal intelligence but preventing project-level CRM isolation.
[Source: MEMORY.md#CRM Architecture]

### Decision 2: MCP as Agent Interface

**Status**: Implemented — 4 servers, 26+ tools
**Implication**: All agents interact with the platform exclusively through MCP (Model Context Protocol). This provides a standardized, auditable interface layer but means agent capabilities are bounded by available MCP tools. Adding new agent capabilities requires defining new MCP tools, not ad-hoc API calls.

### Decision 3: Workflow Builder as Extensibility Engine

**Status**: 70% complete
**Implication**: New platform capabilities are added as workflow node types rather than bespoke features. This provides a composable, user-configurable automation layer but means 6 of 17 planned node types are still missing, blocking certain automation patterns.

### Decision 4: SSE for Real-Time (not WebSocket Primary)

**Status**: Implemented
**Implication**: Server-Sent Events chosen for HTTP/1.1 compatibility and simpler server-side implementation. WebSocket used only where bidirectional communication is required. This simplifies deployment (no WebSocket upgrade handling needed) but limits real-time interaction patterns to server-push.

### Decision 5: SQLite to PostgreSQL Migration Planned

**Status**: SQLite in use; PostgreSQL feature-gated in code
**Implication**: 202 migrations need porting. This is the single largest infrastructure migration on the roadmap. Until completed, the platform has a single-writer bottleneck and cannot support horizontal scaling of the backend.

### Decision 6: DbUuid TEXT-Primary Strategy

**Status**: Phases A+B done; Phases C+D pending
**Implication**: UUIDs stored as TEXT in SQLite (not BLOB) with `DbUuid` abstraction throughout the codebase. 84 route handlers still use `Path<Uuid>` and need migration to `DbUuid::parse`. This decision simplifies debugging and SQLite compatibility but adds conversion overhead.
[Source: MEMORY.md#UUID Handling]

### Decision 7: One-Way Dropbox Sync

**Status**: Implemented
**Implication**: Dropbox is the source of truth for sovereign data. The platform indexes and classifies files but never writes back to Dropbox. This simplifies conflict resolution but means any platform-generated artifacts need a separate storage layer.

### Decision 8: HMAC Webhook Trigger Pattern

**Status**: Defined but triggers not yet implemented
**Implication**: External systems will trigger workflows via HMAC-signed webhooks. This provides a secure, standard integration pattern but requires webhook endpoint deployment and secret management infrastructure that does not yet exist.

---

## Appendix A: System Interdependency Map

```
Intake Pipeline (100%)
    └─> CRM Pipeline (85%)
            ├─> Agent Flow Engine (60%) ← P0 BLOCKER
            │       └─> Workflow Automation (70%)
            │               └─> Content Pipeline (75%)
            └─> Task Management (80%)
                    └─> Notification System (95%)

Sovereign Data (65%)
    └─> Intelligence Dashboard (not started)

APN Mesh (40%)
    └─> VIBE Token (25%)
            └─> Agent Economics (not started)
```

## Appendix B: Version History

| Component | Current Version | Last Major Change |
|-----------|----------------|-------------------|
| Frontend | 0.0.55 | — |
| Backend | 0.0.96 | — |
| CRM Architecture | v2 (org-scoped) | 2026-03-09 |
| DbUuid Migration | Phase B complete | 2026-03-18 |
| WelcomeWizard | PR #48 | 2026-03-18 |

---

*This document serves as the baseline for the 5-year product roadmap. All roadmap milestones should reference this assessment for current-state context and gap analysis.*

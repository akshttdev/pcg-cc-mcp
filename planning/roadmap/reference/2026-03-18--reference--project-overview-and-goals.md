# ORCHA / PCG Dashboard MCP — Project Overview & Goals

**Date:** 2026-03-18
**Type:** Reference
**Purpose:** Consolidated overview of the project, its goals, architecture, and current state — derived from all root-level docs, planning files, memory, and the powerclubglobal.com website.

---

## 1. The Organization: Powerclub Global (PCG)

Powerclub Global is an **international event management, branding, and digital innovation agency** founded in 2018 by Jessy Artman. They specialize in helping technology startups grow through live events, roadshows, and digital marketing.

**Key stats:**
- 5+ years operations, 25+ countries, 85,000+ engaged event participants
- Heavy presence in the blockchain/crypto conference circuit (26 conferences planned in 2025)
- Services: roadshow management, social media, web development, experiences, blockchain solutions, branding, influencer relations, press relations
- Contact: jessy@powerclubglobal.com
- Website: https://powerclubglobal.com (Next.js)

**Relevance to this project:** PCG is the primary user/client of the ORCHA platform. The CRM pipeline, deal flow, client management, and event coordination features are built to serve PCG's agency workflow — managing prospects through a 9-stage sales pipeline, coordinating multi-city events, and tracking client relationships.

---

## 2. What is ORCHA / PCG Dashboard MCP?

A **full-stack project management platform with AI agent orchestration**, purpose-built for PCG's operations. The platform combines:

- **Project & task management** (kanban boards, task dependencies, knowledge management)
- **CRM pipeline** (9-stage deal flow from Lead to Closed Won/Lost)
- **AI agent orchestration** (multiple specialized agents for different domains)
- **MCP servers** (Model Context Protocol for AI tool integration)
- **Sovereign data platform** (Dropbox integration, file indexing, org cloud)
- **On-chain identity** (Aptos wallet, VIBE token economy)
- **Real-time collaboration** (SSE, WebSocket, NATS pub/sub)

### Relevant Docs
- `pcg-cc-mcp/README.md` — Quick start, tech stack, architecture overview
- `pcg-cc-mcp/CLAUDE.md` — Development commands, conventions, crate structure
- `pcg-cc-mcp/docs/INDEX-OF-DOCS.md` — Complete documentation index
- `pcg-cc-mcp/docs/TOPOS_CATEGORICAL_ARCHITECTURE.md` — Category theory formalization
- `pcg-cc-mcp/docs/CLIENT_README.md` — PCG client/node setup

---

## 3. Architecture

### Tech Stack
| Layer | Technology |
|-------|-----------|
| Backend | Rust + Axum + Tokio + SQLx |
| Frontend | React 18 + TypeScript + Vite + Tailwind + shadcn/ui |
| Database | SQLite with SQLx migrations |
| Type sharing | ts-rs (Rust structs → TypeScript) |
| MCP | rmcp 0.5.0+ (Rust Model Context Protocol) |
| Dev environment | Flox (Rust nightly, Node.js, pnpm, cargo-watch, sqlx-cli) |
| Error tracking | Sentry |

### Crate Structure
```
crates/
├── server/         # Axum HTTP server, API routes, MCP servers, SSE
├── db/             # Database models, migrations, SQLx queries
├── executors/      # AI executor integrations (Claude, Gemini)
├── services/       # Business logic, GitHub auth, git operations
├── nora/           # Nora executive agent system
├── topsi/          # Topsi platform orchestrator
├── utils/          # Shared utilities, APN node management
├── cli/            # CLI tools
└── local-deployment/  # Local deployment tooling
```

### MCP Servers (4 specialized)
1. **Task Server** (`crates/server/src/mcp/task_server/`) — 26+ tools for project/task CRUD, knowledge, topology
2. **Topsi Server** (`crates/server/src/mcp/topsi_server.rs`) — Platform orchestration, policy enforcement
3. **Nora Server** (`crates/server/src/mcp/nora_server.rs`) — Executive assistant, strategic coordination
4. **Pulse Server** (`crates/server/src/mcp/pulse_server.rs`) — Content aggregation, real-time data, alerts

### Relevant Docs
- `pcg-cc-mcp/docs/APN_INTEGRATION_ARCHITECTURE.md` — Three-layer architecture (App → Integration → Transport)
- `planning/notes/APN-README.md` — Alpha Protocol Network details
- `pcg-cc-mcp/docs/TOPSI.md` — Topsi agent system documentation
- `pcg-cc-mcp/docs/EDITRON.md` — Editron post-production agent

---

## 4. Specialized Agent Systems

| Agent | Role | Key Capabilities |
|-------|------|-------------------|
| **Nora** | Executive assistant | Strategic planning, task coordination, performance analysis |
| **Topsi** | Platform orchestrator | Topology intelligence, policy enforcement, node management |
| **Editron** | Post-production architect | Dropbox batch ingestion, iMovie/Compressor automation |
| **Spectra** | Master cinematographer | Stable Diffusion/Runway motion plates, LUT kits |
| **Maci** | AI cinematic specialist | Visual generation |
| **Scout** | Research agent | Data gathering, intelligence |
| **Auri** | Audio specialist | Voice processing |

### Relevant Docs
- `pcg-cc-mcp/docs/EDITRON.md` — Editron detailed documentation
- `pcg-cc-mcp/docs/TOPSI.md` — Topsi system documentation
- `planning/2026-03-18--research--rooroo-agent-orchestration.md` — RooRoo orchestration patterns
- `planning/2026-03-18--research--multi-agent-system-design.md` — MAS design patterns

---

## 5. CRM & Pipeline Architecture

**9-stage client pipeline:** Lead → Business Analysis → Discovery → Build Proposal → Polish → Proposal Meeting → Follow Up → Closed Won / Closed Lost

- CRM scoped to `organization_id` (primary), `client_id` (optional)
- Stage-aware triggers (who_is research, review gates, business report generation)
- Persons table unified with CRM contacts
- Commit dedup via `find_by_email()` and `find_by_name_contact_pipeline()`
- Delivery pipeline auto-creation on Closed Won gate

### Relevant Docs
- `planning/2026-03-18--reference--consolidated-pipeline-roadmap.md` — Master pipeline roadmap (~85% complete)
- `planning/2026-03-18--reference--dealflow-pipeline-status.md` — Detailed implementation checklist
- `planning/notes/2026-03-18--reference--consolidated-pipeline-roadmap.md` — Pipeline config reference

---

## 6. Alpha Protocol Network (APN)

A **sovereign mesh networking layer** for distributed computing:
- Devices contribute resources (CPU, storage, bandwidth) and earn VIBE tokens
- libp2p 0.54 + Gossipsub + Kademlia DHT + Noise Protocol + ChaCha20
- Wearable integration (rings, glasses) via USB bridge
- Long-range radio mesh via Meshtastic
- PyQt6 desktop dashboard with FastAPI backend

### Relevant Docs
- `planning/notes/APN-README.md` — Full APN specification
- `pcg-cc-mcp/docs/APN_INTEGRATION_ARCHITECTURE.md` — Integration architecture
- `apn-core/` — Source code directory

---

## 7. Sovereign Data Platform

Phase 1 complete (as of 2026-03-17):
- Dropbox scraper running, APN Cloud in Explorer mode
- 45,896 indexed files (3,895 org + 1,650 personal)
- Phase 2 planned: data classification, intelligence dashboards, Pulse Engine integration, search/discovery

### Relevant Docs
- `planning/2026-03-17--plan--intelligence-sovereign-data-platform.md` — Platform roadmap

---

## 8. Current Development State (2026-03-18 Snapshot)

### Active PRs/Sprints
| PR | Sprint | Status | Key Deliverables |
|----|--------|--------|-----------------|
| #49 | Frontend Polish | Pending | Query key fixes, mutation standardization, 100-file repo cleanup |
| #48 | UX Polish & Quality 2 | Complete | WelcomeWizard, 107→29 `:any`, 341→84 Path<Uuid> |
| #47 | Dev Velocity | Complete | DbUuid Phase A/B, unwrap elimination, route auth |
| #45 | Sloperation316 Integration | QA complete | 9-stage CRM pipeline, sovereign stack, security fixes |

### Modularity Trajectory (Sprints 3-5)
- Large monoliths (1,000+ lines) systematically split into directory modules
- Query-key factories, useMutationWithToast hooks, API module patterns
- Sprint 6 planned as continuation

### Relevant Docs
- `planning/2026-03-18--plan--frontend-polish-sprint.md` — PR #49 details
- `planning/2026-03-18--plan--ux-polish-quality-sprint-2.md` — PR #48 details
- `planning/2026-03-18--plan--dev-velocity-sprint.md` — PR #47 details
- `planning/2026-03-17--plan--sloperation316-integration.md` — PR #45 integration

---

## 9. P0/P1 Backlog (Critical Missing Pieces)

| Priority | Item | Status | Description |
|----------|------|--------|-------------|
| **P0** | Agent Flow Orchestration Engine | Not started | Background worker for phase progression, biggest gap |
| **P0** | Structured response protocol | Not started | Output Envelope for agent-to-orchestrator communication |
| **P1** | Agent capability discovery | Not started | Dynamic tool registration |
| **P1** | Prompt caching optimization | Not started | 90% savings on cached reads |
| **P1** | Webhook infrastructure | Not started | Event-driven integrations |
| **P2** | Query optimization | Partial | Remaining N+1 queries, index gaps |
| **P2** | Observability | Partial | Error handling patterns, structured logging |

### Relevant Docs
- `planning/BACKLOG--remaining-work.md` — Full 83KB backlog with all items, sources, and recommendations

---

## 10. Research Applied & Strategic Direction

8 research papers (2026-03-18) inform the architecture roadmap:

| Research | Key Insight | Application |
|----------|-------------|-------------|
| ATLAS Task MCP | Graph dependencies, not flat lists | Task management evolution |
| CRM/PM MCP Servers | Expose workflows as discoverable actions | Agent-accessible pipeline transitions |
| Docker Multi-Agent | Service-per-agent with explicit ordering | Agent deployment model |
| FSM/RFSM | Event-State Analysis for completeness | Workflow validation |
| Multi-Agent Systems | BDI model, Contract Net Protocol | Orchestrator commitment strategy |
| RooRoo | Cost-tiered model routing, Output Envelope | Model selection + agent comms |
| Spec-Driven Dev | Constitution → Specify → Plan → Tasks → Implement | Agent work pipeline |
| SudoLang/AIDD | Constraint-based specifications | Agent instruction format |

### Relevant Docs
- `planning/2026-03-18--research--*.md` — All 8 research papers
- `planning/2026-03-18--analysis--unit-economics-agent-metrics.md` — Cost analysis ($0.30-$8/task)

---

## 11. Key Conventions & Standards

- **Planning files:** `YYYY-MM-DD--<type>--<topic>.md` (see `planning/README.md`)
- **UUID handling:** DbUuid-first, never `uuid::Uuid` in route handlers
- **Git workflow:** Never push to main without approval; preserve commit history on feature branches
- **Dev server:** `pnpm run dev` inside `flox activate`
- **Type generation:** `npm run generate-types` after Rust struct changes
- **DB changes:** `DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace`
- **Login:** admin/admin123

### Relevant Docs
- `.claude/rules/rust-standards.md` — Rust coding standards
- `planning/notes/2026-03-15--plan--claude-code-platform-standards.md` — Platform standards
- `planning/README.md` — Planning file convention spec

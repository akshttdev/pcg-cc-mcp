# Graph Visualization Enhancement — Operational Surface over All Data

**Date**: 2026-04-16 (reframed 2026-04-18)
**Branch**: `feature/graph-viz-enhancement` (created from `main` @ 8640f2ee on 2026-04-18)
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp` (root)
**Base**: `main`
**Status**: Reframing — awaiting expansion + sign-off

## Vision (reframed 2026-04-18)

The knowledge graph is **not a visualization feature** — it is the dashboard's
**operational surface over all accumulated data**, scoped to users and the
organizations they belong to, with the ability to act on any artifact via
agents. Two distinct UX modes share one canvas:

### Surface 1 — Interface Tab (Topsi conversational mode)

Route: `/interface`

- Live conversation with **Topsi** while navigating the graph as a live canvas.
- Zoom from the global view down to **any individual piece of data** — each
  node has an inline **artifact preview** (per node type).
- **Multi-open**: multiple nodes/artifacts open simultaneously as workspaces.
- **Multi-threaded conversations**: each open artifact can anchor its own
  Topsi thread. Topsi surfaces artifacts the user may need to see for a
  decision, and the user can pop those open without breaking an existing
  conversation.
- Purpose: **decision-making while executing workflows** — the graph + Topsi
  together are the workspace.

### Surface 2 — Intelligence › Topology (Org-scoped manual mode)

Route: `/organizations/:orgId/intelligence/topology`

- Same canvas + preview system, different shell.
- Optimized for **manual browse/explore/audit** within one org.
- Scoped to that organization's data, filtered by the user's access level
  within that org.
- Less conversational; the Topsi side-panel is optional/collapsed by default.
- Purpose: **understand / navigate / audit** the org's accumulated data.

### Scope of "all data"

The graph must include (not exhaustive — enumerate during expansion):
companies, organizations, persons, contacts (post contacts-first),
clients, deals, proposals, projects, deliverables, activities, decks /
deck documents, tasks / task attempts, knowledge sources, research passes
(company + contact), pipelines + stages, brand profiles, inquiries.
This is **far larger** than the current `entity_graph_*` coverage
(company/org/person/proposal/project).

### Perspectives (lenses) — first-class, not filters

Distinct query + layout shapes, switchable without losing position:

- **Pipeline view** — deals through pipeline stages
- **Delivery view** — projects → deliverables → tasks → agents
- **Intelligence view** — research → knowledge sources → entities (today's KG)
- **Temporal view** — activity over time, cohort slicing
- **Relationship view** — person ↔ person via shared orgs / deals / activities
- **Workflow view** — agent flows, their triggers, and the artifacts they touch
- (Enumerate the rest during expansion)

Each perspective is a **named query + layout preset**, not a checkbox filter.

### Agent actions on nodes

From any node the user (or Topsi) can invoke agent actions. The action
set is node-type-specific and wired into existing agent endpoints.
Sketch (fill in during expansion):

- Company → research, summarize, create deck, add deal
- Person → research, summarize, draft outreach
- Deal → advance stage, create proposal, assign task
- Knowledge source → re-ingest, summarize, extract facts
- Deck → open in Lux, duplicate, export
- Task → reassign, retry, inspect logs

Needs a **node-type → action registry** with the routing to existing
handlers.

### Multi-tenant scoping rules

- `/interface` Topsi view = union of all orgs the user belongs to, each
  filtered by the user's per-org access level.
- `/organizations/:orgId/intelligence/topology` = only that org's nodes,
  filtered by the user's access level in that org.
- Admin-only surfaces (e.g., `sync_global_graph`) are separately gated.
- Enumerate the access matrix (node_type × role × membership) during
  expansion.

### Open architectural questions (to resolve in expansion)

1. **Storage model**: do we expand `entity_graph_*` to materialize all node
   types, query live tables per perspective, or do a hybrid
   (materialized "core" graph + perspective-specific live joins)?
2. **Artifact preview**: registry of `node_type → PreviewComponent` — where
   does this live, and how do existing detail components get reused?
3. **Multi-thread Topsi**: how does Topsi track which artifacts anchor which
   threads? Does each node open create a new SSE channel, or is there one
   channel multiplexed by thread ID?
4. **Agent action registry**: where is the `node_type → available_actions`
   map defined, and how does it stay in sync with the real agent handler
   surface?
5. **Live updates**: do graph mutations (Topsi-driven or user-driven) reflect
   in real-time via SSE? If so, which streams?
6. **Access control**: explicit rules for what a user sees in the
   `/interface` global view vs. an org-scoped topology view, per node type.
7. **Perspective switching**: client-side layout swap over one fetched data
   set, or each perspective is its own query? Latency budget?

## Original Goal (superseded — kept for context)

Turn the orphaned `KnowledgeGraphViz.tsx` into a first-class, user-facing data
visualization **page** that anyone can open and explore — the way Obsidian's
graph view invites casual exploration, but over our typed CRM entity graph.

**Success criteria**

1. User can open a dedicated `/graph` page (global scope) and a scoped variant
   `/organizations/:orgId/graph` (segmented scope) and see their entity graph.
2. User can toggle between **2D** and **3D** views with no data refetch.
3. Filters for node types (Company / Org / Person / Proposal) and edge types
   (`employs`, `targets`, `lead_on`, `part_of_org`, `manages`) work live.
4. Hover highlights neighborhood; click opens an entity drawer with a
   "Open full profile" link that navigates to the existing detail page.
5. Page renders ≤300 nodes at 60fps and ≤2000 nodes at ≥30fps.
6. First-time visitor understands what they're looking at within 5 seconds
   (legend + help tooltip + non-empty empty state).

## Audit findings (2026-04-18)

Research against current `main @ 8640f2ee`. Corrects and supersedes the
earlier "Problems in the current state" section of this plan.

| Claim in v1 plan | Verified? | Correction |
|---|---|---|
| `KnowledgeGraphViz.tsx` is an orphan (no importers) | **False** | Lives at `frontend/src/components/vibeland/KnowledgeGraphViz.tsx` (500 LOC), imported by the Vibeland page. Not orphaned — but it calls `/api/graph/global` which doesn't exist, so it's broken at runtime. |
| `graph.rs:65-70` only wires `/graph/company/:id` + `/graph/sync/company/:id` | **True** | `crates/server/src/routes/graph.rs:67-68` |
| `entity_graph.rs:151` has `company_subgraph` | **True** | Only reader that exists. Only writer is `sync_company_graph` at same file. |
| `IntelligenceTab.tsx:25-57` has path→view mapping | **Off by half** | Actual mapping is `IntelligenceTab.tsx:25-31`; `VIEW_PATHS` reverse map at `44-56`. Current views: overview / datasources / artifacts / workflows / pulse / topology. |
| `TopologyIntelView` is a placeholder | **False** | Renders a grid of `topology_snapshot`-typed knowledge-source cards via `useQueries`. Not a graph canvas, but not empty either. |
| `/interface` is a "quick-access tile surface" | **False** | `frontend/src/pages/home/index.tsx` (939 LOC) is already the full dual-mode Nora (admin) / Topsi (non-admin) conversational home with orb visualizer, voice I/O, and agent-network pill. Adding a graph canvas requires a ~500 LOC layout refactor, not a tile. |
| Current `entity_graph_*` coverage | — | 5 node types (`company`, `person`, `organization`, `proposal`, `project`) and 5 edge types (`employs`, `targets`, `lead_on`, `part_of_org`, `manages`) — but only `company`, `person` (via `crm_contacts`), `organization`, `proposal` are actually synced. `project`, `lead_on`, `manages` are defined but unused. |
| Agent handlers that exist today | — | Company research `companies.rs:161`, Contact research `intelligence.rs:71`, Agent chat `agent_chat.rs:126`, Wide research `wide_research.rs:72`, SSE stream `event_stream.rs:24`. |
| Agent handlers the plan will need | — | Create-deck, draft-outreach, advance-stage, create-proposal, assign-task, reassign-task, retry-task, inspect-logs — **none exist**. |
| `agent_conversations` table | — | Exists (`crates/db/src/models/agent_conversation.rs`); linear, keyed by `agent_id + session_id`. No artifact anchoring, no thread multiplexing, no `conversation_threads` table yet. |
| Frontend graph libraries installed | — | `three.js` + `@react-three/fiber` (already used by `KnowledgeGraphViz` and `WorkflowGraphView`). `d3-force`, `react-force-graph`, `cytoscape` **not** installed. |

**Single biggest correction:** `/interface` is not a tile shell — it is already
the Topsi/Nora conversational home. Making it the graph-driven operational
surface is a **refactor of an existing production page**, not a new page.

## Architectural decisions (resolving Vision Qs)

One recommendation per open question from the Vision section. Each is
grounded in the audit; alternatives are flagged where the trade-off is real.

### Q1 — Storage model: **hybrid**

Keep `entity_graph_*` as the canonical materialized graph for **stable
entities** (organizations, companies, persons/contacts, clients, proposals,
deals, projects, deliverables, pipelines, brand profiles, decks, knowledge
sources). Expand `sync_*_graph` coverage to include them. For **fast-changing
entities** (tasks, task attempts, activities, execution logs, research
passes), query **live tables** per perspective — don't materialize churn.

- **Alt A — fully materialized**: sync everything on a timer. Rejected:
  sync lag on tasks/activities is unacceptable for the operational surface.
- **Alt B — fully live**: query raw tables per request. Rejected: N+1
  across ~18 tables, no unified access-scoping chokepoint.
- **Why hybrid**: matches the code pattern (`sync_company_graph` already
  materializes; activities/tasks already have fast detail endpoints).

### Q2 — Artifact preview: **registry of existing detail components**

`node_type → PreviewComponent` registry at
`frontend/src/components/topology/previews/`. Existing detail pages are
reused by one of three strategies per node type: direct import (low cost),
extract header/summary subcomponent (medium), or new lightweight card
(high). Extraction cost matrix in the **Artifact preview registry**
section below.

### Q3 — Topsi multi-thread: **thread-per-artifact with multiplexed SSE**

New table `conversation_threads(artifact_type, artifact_id, organization_id, …)`.
A thread is created on first open of an artifact in a conversational
context and persists across page reloads. Extend `agent_conversation_messages`
with nullable `thread_id` for backward compatibility. Replace the
per-connection SSE pattern with one multiplexed stream keyed by `user_id`
carrying thread-tagged events; the frontend fans out to open panels.

- **Alt**: thread-per-session (no persistence). Rejected: the vision
  requires conversations that survive reloads and drive decision-making.

### Q4 — Agent action registry: **static Rust registry + handler gaps flagged**

Static `NodeActionRegistry` struct mapping `(NodeType, AgentAction) →
ActionHandler{route, method, scope}`. Grounded in existing handlers where
possible; **8 handlers the vision requires don't exist yet** and are
scheduled in Phase 5 (see Registry section for the full list). Registry
lives in `crates/server/src/services/node_actions.rs` and is exposed to
the frontend via `GET /api/graph/actions?node_type=<t>` so the UI doesn't
drift from the real handler surface.

### Q5 — Live updates: **SSE, reuse `useEventSourceManager`**

Graph mutations (sync runs, agent action completions, new research passes)
emit events through the existing event stream. No WebSockets — the
existing SSE pattern is good enough for the node counts in play (≤2000).
Multiplexing solves the "one connection per graph" problem.

### Q6 — Access control: **reuse existing primitives, add bulk helper**

The current `AccessContext` + `require_org_membership` pattern is correct
but was designed for one-resource-per-endpoint. The graph fetches hundreds
of entities at once. Add a new helper `filter_visible_nodes(ctx, nodes) ->
Vec<Node>` in `crates/server/src/services/graph_access.rs` that batch-resolves
org membership once and applies per-node-type rules in-memory. Matrix is
specified in the **Access-control matrix** section below.

### Q7 — Perspective switching: **single fetch + client-side layout swap** (for the core set)

Every perspective in the initial set is derivable from a single
`{nodes, edges, metadata}` payload. The perspective decides layout + dim
rules + default filters client-side. Exceptions: **Temporal** and
**Workflow** views require extra joins; they each get a dedicated endpoint.

### Diagram — two surfaces, one canvas

```
┌───────────────────────────────────────────────────────────────────────┐
│  /interface  (HomePage, 939 LOC today)    /organizations/:orgId/      │
│                                             intelligence/topology     │
│  ┌─────────────────────────────────┐     ┌──────────────────────────┐ │
│  │  ┌──────────────────┐           │     │ ┌─────────────────────┐  │ │
│  │  │ TopologyCanvas   │◀── SAME ──┼─────┼▶│ TopologyCanvas      │  │ │
│  │  │ scope=user-orgs  │ COMPONENT │     │ │ scope=org(:orgId)   │  │ │
│  │  │ perspective swap │           │     │ │ perspective swap    │  │ │
│  │  └──────────────────┘           │     │ └─────────────────────┘  │ │
│  │       │                         │     │        │                 │ │
│  │       ▼                         │     │        ▼                 │ │
│  │  ┌──────────┐   ┌─────────────┐ │     │  ┌──────────────────┐    │ │
│  │  │ Artifact │   │ Topsi orb   │ │     │  │ Preview drawer   │    │ │
│  │  │ drawer   │   │ + thread    │ │     │  │ (single-focus)   │    │ │
│  │  │ (multi-  │   │ panels      │ │     │  └──────────────────┘    │ │
│  │  │  open)   │   │ (per        │ │     │  ┌──────────────────┐    │ │
│  │  └──────────┘   │  artifact)  │ │     │  │ Archive: legacy  │    │ │
│  │                 └─────────────┘ │     │  │ topology_snapshot│    │ │
│  └─────────────────────────────────┘     │  └──────────────────┘    │ │
│                                           └──────────────────────────┘ │
│  Mode: conversational                     Mode: manual browse         │
│  Driver: Topsi surfaces artifacts         Driver: user navigates      │
│  Scope: all orgs user belongs to          Scope: this org only        │
│  Use: decide inside live workflows        Use: audit / explore        │
└───────────────────────────────────────────────────────────────────────┘
```

## Entity taxonomy (authoritative)

Tables participating in the graph, grouped by domain. `Scoped` = how a node
inherits `organization_id` for access control (`direct` = has the column;
`via <table>` = transitive).

| Domain | Table | node_type | Scoped | In graph today? |
|---|---|---|---|---|
| Core | `organizations` | `organization` | self | ✓ |
| Core | `companies` | `company` | direct (optional) | ✓ |
| Core | `clients` | `client` | direct | — add |
| Core | `crm_contacts` | `person` (aliased) | direct (optional) | ✓ (as `person`) |
| Core | `persons` (legacy — verify) | `person` | via crm_contact | ? [FLAG] |
| CRM | `crm_deals` | `deal` | direct | — add |
| CRM | `crm_pipelines` | `pipeline` | direct | — add |
| CRM | `crm_pipeline_stages` | `pipeline_stage` | via pipeline | — add |
| CRM | `proposals` | `proposal` | direct (optional) | ✓ |
| CRM | `scheduled_meetings` | `meeting` | via proposal | — optional |
| Delivery | `projects` | `project` | direct (optional) | defined, not synced |
| Delivery | `project_deliverables` | `deliverable` | via project | — add |
| Delivery | `tasks` | `task` | via project | — add (live-query) |
| Delivery | `task_attempts` | `task_attempt` | via task | — add (live-query) |
| Intelligence | `project_knowledge_sources` | `knowledge_source` | direct | — add |
| Intelligence | `user_knowledge_sources` | `knowledge_source` | direct | — add |
| Intelligence | `contact_research_passes` | `research_pass` | via contact | — add (live-query) |
| Intelligence | `activity_logs` | `activity` | via project | — add (live-query) |
| Agents | `agents` | `agent` | global | — add |
| Agents | `agent_conversations` | `conversation` | via agent/user | — optional |
| Lux | `decks` (from deck-authoring migration) | `deck` | via deal | — add |
| Brand | `brand_profile` | `brand_profile` | via project | — add |

**FLAG:** The audit reports both "`entity_graph` uses `crm_contacts` for
persons" and "`proposals.lead_id → persons.id`". Confirm during Phase 1:
is `persons` a live table, a renamed column pointing at `crm_contacts`, or
legacy cruft? This affects the `lead_on` edge wiring.

**New edge types to introduce:**
`has_deal`, `in_pipeline`, `in_stage`, `has_deliverable`, `assigned_to_agent`,
`produced_artifact`, `authored_research`, `owns_knowledge`, `executes_task`,
`has_deck`, `has_brand_profile`, `has_client`, `activity_on`.

## Storage model — concrete plan

| Layer | What | How |
|---|---|---|
| Materialized (SQLite, `entity_graph_*`) | Stable entities + their structural edges | Expand `sync_*_graph` functions: `sync_org_graph`, `sync_client_graph`, `sync_deal_graph`, `sync_project_graph`, `sync_proposal_graph`, `sync_deck_graph`, `sync_brand_profile_graph`, `sync_knowledge_source_graph`. Keep `sync_company_graph`. Add `sync_global_graph(pool)` that composes them. |
| Live (query time) | Tasks, task attempts, activities, research passes | Perspective endpoints JOIN live tables and return `{nodes, edges}` in the same shape. No writes to `entity_graph_*`. |
| Virtual (computed) | Derived edges like `person ↔ person via shared-deal` | Computed by the Relationship perspective endpoint using SQL; never persisted. |

Migration needed? **Yes**, one small additive migration to extend the
`edge_type` CHECK constraint. No changes to `entity_graph_nodes` schema.

### Diagram — storage tiers

```
┌─────────────────────────────────────────────────────────────────┐
│                      STORAGE TIERS                              │
│                                                                 │
│  ┌─────────────────────┐     ┌─────────────────────┐            │
│  │  MATERIALIZED       │     │  LIVE-QUERY         │            │
│  │  entity_graph_nodes │     │  JOIN at request    │            │
│  │  entity_graph_edges │     │  (no persistence)   │            │
│  │                     │     │                     │            │
│  │  organizations      │     │  tasks              │            │
│  │  companies          │     │  task_attempts      │            │
│  │  clients            │     │  activities         │            │
│  │  persons/contacts   │     │  research_passes    │            │
│  │  deals              │     │                     │            │
│  │  proposals          │     │  (churn > 1/min)    │            │
│  │  projects           │     └─────────────────────┘            │
│  │  deliverables       │                                        │
│  │  pipelines          │     ┌─────────────────────┐            │
│  │  decks              │     │  VIRTUAL (computed) │            │
│  │  knowledge_sources  │     │                     │            │
│  │  brand_profiles     │     │  person↔person via  │            │
│  └──────────┬──────────┘     │   shared deals      │            │
│             │                │  person↔person via  │            │
│   sync_*_graph(pool)         │   shared orgs       │            │
│             │                │                     │            │
│             ▼                │  (SQL, never saved) │            │
│  sync_global_graph(pool)     └─────────────────────┘            │
│     (schedule + admin)                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Perspective system

Each perspective is a **query shape + layout preset + default filter set**.
Shared `TopologyCanvas` component accepts a `perspective` prop.

| Perspective | Query | Layout | Filter defaults | Endpoint |
|---|---|---|---|---|
| **Graph** (default) | `GET /api/graph/subgraph?scope=…` returns all materialized nodes + edges | Force-directed (2D or 3D) | all node types visible | shared |
| **Pipeline** | Deals + pipeline stages + contacts. `deal → stage`, `deal → contact` | Hierarchy: stages as columns, deals as cards, radial contacts | node types: deal/stage/contact/company | shared |
| **Delivery** | Projects → deliverables → tasks → task_attempts → agents. Live-query. | Tree / layered DAG | node types: project/deliverable/task/task_attempt/agent | `GET /api/graph/perspective/delivery?org_id=` |
| **Intelligence** (today's KG) | Research passes → knowledge sources → companies/persons | Force-directed, weighted by research depth | node types: research_pass/knowledge_source/company/person | shared |
| **Temporal** | Activities + created_at of all entities, windowed | Timeline (x=time, y=entity-stack) | any node type | `GET /api/graph/perspective/temporal?from=&to=` |
| **Relationship** | Person ↔ person via shared orgs/deals/proposals/activities | Force-directed, edge-bundled | node types: person/company/organization | shared + virtual edges |
| **Workflow** | Agent flows + triggers + artifacts they touched | DAG (agent flow engine shape) | node types: agent/task/task_attempt/artifact | `GET /api/graph/perspective/workflow?flow_id=` |

**Added (not in Vision):**
- **Account view** — one company at the center, radial rings of deals,
  contacts, proposals, research passes, activities. The "client 360".
- **Ownership view** — which user owns which entities (great for admins).

## Node → Agent action registry

Registry structure (Rust):

```rust
pub enum NodeType { Organization, Company, Client, Person, Deal,
    Proposal, Project, Deliverable, Task, TaskAttempt, KnowledgeSource,
    ResearchPass, Pipeline, BrandProfile, Deck, Activity, Agent }

pub enum AgentAction { Research, Summarize, DraftOutreach,
    CreateDeck, CreateProposal, AdvanceStage, AssignTask, Reassign,
    Retry, InspectLogs, ReIngest, ExtractFacts, OpenInLux,
    Duplicate, Export, OpenProfile, StartConversation }

pub struct ActionHandler {
    pub route: &'static str, pub method: &'static str,
    pub requires: Required, pub description: &'static str,
}
```

Action ↔ handler mapping (partial — full map in Phase 5 plan doc):

| Node | Action | Handler | Status |
|---|---|---|---|
| Company | Research | `POST /api/companies/:id/research` (companies.rs:161) | ✓ exists |
| Company | Summarize | `GET /api/companies/:id` (intel summary field) | ✓ exists |
| Company | CreateDeck | **none** | ✗ BUILD in Phase 5 |
| Company | OpenProfile | route: `/organizations/:orgId/companies/:id` | ✓ exists |
| Person | Research | `POST /api/contacts/:id/research` (intelligence.rs:71) | ✓ exists |
| Person | DraftOutreach | **none** | ✗ BUILD |
| Deal | AdvanceStage | **none** | ✗ BUILD (just a `PATCH`) |
| Deal | CreateProposal | **none** | ✗ BUILD |
| Deal | AssignTask | **none** | ✗ BUILD |
| Task | Reassign | **none** | ✗ BUILD |
| Task | Retry | **none** | ✗ BUILD |
| Task | InspectLogs | `GET /api/events/processes/:id/logs` | ~ partial (logs exist, no node wiring) |
| KnowledgeSource | ReIngest | varies (several re-ingest endpoints) | ~ partial — unify |
| Any | StartConversation | **none** (needs thread model — Phase 4) | ✗ BUILD |

Gaps become a line-item on Phase 5.

## Nora & Topsi — Interface Mode (2026-04-18)

The `/interface` tab is a **Jarvis-style operational surface**. Two
distinct voice agents inhabit it, not one in two modes. This section
consolidates the vision decisions into concrete architecture.

### Two agents, distinct roles

```
┌─────────────────────────────────────────────────────────────────────┐
│                       AGENT IDENTITIES                              │
│                                                                     │
│                    NORA                  │         TOPSI            │
│  ──────────────────────────────────────  │  ─────────────────────── │
│   owner    │ Powerclub Global (PCG)      │ the platform             │
│   driver   │ Bodhi + PCG admins          │ every user (personalized)│
│   persona  │ existing Nora persona       │ Jarvis-ish chief-of-staff│
│   scope    │ unified cross-org (PCG's    │ user's home org primary, │
│            │   reach: all clients)       │   plus other memberships │
│   tiered   │ YES — Admin/Client/         │ NO — access via          │
│   capability│  Prospect/Stranger         │   AccessContext roles    │
│   threads  │ PCG-owned; dual-stored for  │ user/org-owned           │
│            │   cross-org counterparties  │                          │
│   voice    │ own STT/TTS stack           │ own STT/TTS stack        │
│   appears  │ Bodhi's sessions only       │ every user's sessions    │
└─────────────────────────────────────────────────────────────────────┘
```

**Nora** — Powerclub Global's executive agent + Bodhi's personal
assistant. A **PCG-owned asset**. Admin/executive capabilities only
unlock for Bodhi + PCG admins. For everyone else, Nora's capability is
gated by the counterparty's relationship to PCG (execution tiers below).
She already has an established persona.

**Topsi** — the **default agent** every platform user gets. Global
surface, **personalized per user** via that user's footprint (data, org
role, history, preferences). Many orgs use Topsi; many users per org use
Topsi. Terse chief-of-staff persona.

A PCG admin session can see **both** — Nora drives PCG's reach, Topsi
drives the admin's participation in their own home org if that differs.
Non-PCG users see Topsi only.

### Execution tiers (Nora only)

Nora does what Bodhi tells her. For everyone else, capability is
gated by tier. Tier is computed at session start from
`(user_id, org_membership, deal_stage_with_PCG, …)`.

```
┌───────────────────────────────────────────────────────────────────┐
│                 NORA — EXECUTION TIERS                            │
│                                                                   │
│   ADMIN    ████████████████████████████████████████  full autonomy│
│   (Bodhi,  │ read │ write │ delete │ mutate PCG │ act on clients  │
│    PCG     │      │       │        │ internals  │                 │
│    admins) │                                                      │
│                                                                   │
│   CLIENT   █████████████████████████  their-stuff only            │
│   (paying  │ read │ draft │ execute on their data                 │
│    cust.)  │      │       │ (research, outreach,                  │
│            │      │       │  advance own deal)                    │
│                                                                   │
│   PROSPECT █████████████████  read + schedule                     │
│   (pre-    │ read │ schedule │ info packets │                     │
│    contract)│                                                     │
│                                                                   │
│   STRANGER █████████  demo                                        │
│   (unknown)│ FAQ │ contact capture │ book intro call │            │
└───────────────────────────────────────────────────────────────────┘
```

Topsi has **no tier system** — standard `AccessContext` primitives
govern what each user sees and can do in their own org.

### Storage & visibility — dual-store for cross-org Nora

```
┌──────────────────────────────────────────────────────────────────┐
│          DUAL STORAGE — NORA CROSS-ORG INTERACTIONS              │
│                                                                  │
│   Scenario: Nora talks to David Woods (client @ Fusion Fashion)  │
│                                                                  │
│            ┌──────────────────┐                                  │
│            │  David's session │                                  │
│            │  (client tier)   │                                  │
│            └────────┬─────────┘                                  │
│                     │  conversation                              │
│                     ▼                                            │
│            ┌──────────────────┐                                  │
│            │      Nora        │                                  │
│            └────────┬─────────┘                                  │
│                     │   writes to BOTH                           │
│        ┌────────────┴────────────┐                               │
│        ▼                         ▼                               │
│  ┌──────────────┐          ┌──────────────┐                      │
│  │  PCG DB      │          │  Fusion DB   │                      │
│  │  (owner)     │          │  (counter-   │                      │
│  │              │          │   party)     │                      │
│  │ visible to:  │          │ visible to:  │                      │
│  │  - Bodhi     │          │  - David     │                      │
│  │  - PCG admins│          │  - Fusion    │                      │
│  │              │          │    admins    │                      │
│  └──────────────┘          └──────────────┘                      │
│                                                                  │
│  Other Fusion users see thread only if explicitly shared,        │
│  and subject to their access to the underlying artifacts.        │
└──────────────────────────────────────────────────────────────────┘
```

**Rules:**
- Every Nora interaction is stored in the **PCG database** — always.
- Cross-org interactions (client, prospect, stranger) are **also**
  stored in the counterparty's DB. Both copies live; no source-of-truth
  distinction for conversation text.
- Conversation logs are visible to: the user in the conversation, that
  user's org admins, and Bodhi + PCG admins (always).
- Task outputs produced from conversations can be shared with teammates,
  subject to their access to the underlying data.
- Topsi interactions are stored in the user's home-org DB only.

### Task-card lifecycle — the primary UI grammar

Interacting with an artifact means **spawning a task card**. Cards are
the workspace; the graph is the map.

```
┌──────────────────────────────────────────────────────────────────┐
│                  TASK CARD LIFECYCLE                             │
│                                                                  │
│   ┌─ graph ─┐         ┌─────────────────┐     ┌─ graph ─┐        │
│   │         │  click  │   task card     │     │    ●    │        │
│   │    ●    │ ───────▶│  ┌───────────┐  │     │  open-  │        │
│   │  node   │         │  │ preview   │  │     │  card   │        │
│   │         │         │  │ thread    │  │     │  indic. │        │
│   └─────────┘         │  │ sub-agents│  │     └────┬────┘        │
│        ▲              │  └───────────┘  │          │             │
│        │              └──────┬──────────┘          │             │
│        │                     │                     │             │
│        │              ┌──────┴──────┐              │             │
│        │              ▼             ▼              │             │
│        │         [minimize]      [close]           │             │
│        │              │             │              │             │
│        │              │             ▼              │             │
│        │              │       ┌──────────────────┐ │             │
│        │              │       │  stop work;      │ │             │
│        │              │       │  5-min undo      │ │             │
│        │              │       └──────────────────┘ │             │
│        │              │                            │             │
│        │              ▼                            │             │
│        │      ┌─ graph ─┐                          │             │
│        │      │   ●●●   │  ◀── work continues      │             │
│        │      │ pulsing │      in background       │             │
│        └──────┤  node   │                          │             │
│               └─────────┘  click to restore ───────┘             │
│                                                                  │
│  Multiple cards open simultaneously. Global agent orchestrates.  │
└──────────────────────────────────────────────────────────────────┘
```

- **Spawn**: card detaches from its source node into a workspace layer
  above the canvas. Source node shows an open-card indicator.
- **Minimize**: card collapses back into its source node. The node
  pulses; sub-agents keep running in the background.
- **Close**: stops all work on that card; mutations enter the 5-min
  undo window.
- **Multi-monitor**: cards can pop out to another window entirely.

While cards run sub-models/sub-agents, the **global agent** (Nora or
Topsi) orchestrates the sub-interactions on your behalf. You keep one
fluid conversation with the global agent while dozens of task cards
execute in parallel.

### Voice & thread model

- Nora and Topsi each **own a persistent thread**. A session has exactly
  one active global-agent thread (Nora for Bodhi/admin, Topsi for
  everyone else; Bodhi may toggle between them).
- Sub-agents inside task cards have their own threads but are driven
  by the global agent — the user doesn't context-switch.
- Other agents joining a conversation (multi-party) is **deferred**.
- Wake word (`Hey Nora` / `Hey Topsi`), barge-in enabled, text+voice
  hybrid. English only for v1.
- **Clarification-first behavior**: when the agent doesn't understand
  or is missing information, it asks — it does not fabricate.
  This is a cross-cutting prompt engineering requirement.

### Session kickoff — MVP briefs

Both agents deliver a short brief on session start. Evolves via
conversation ("Nora, drop calendar from the brief").

**Nora's PCG Status Update:**
- Deal pipeline — new, stalled, closing this week
- Research passes completed since last login
- Active projects + deliverables due
- Client health flags (cold contacts, escalations, wins)
- Agent activity (sub-agents run, artifacts produced)
- Calendar / meetings today
- Unread inquiries / inbound messages
- **Pulse Intelligence summary**

**Topsi's Home-Org Status Update:**
Same shape, scoped to the user's home org.

### Proactivity gates

```
┌──────────────────────────────────────────────────────────────────┐
│         PROACTIVITY — WHEN THE AGENT VOLUNTEERS                  │
│                                                                  │
│    ┌───────────────┐                                             │
│    │  REACTIVE     │  "stay quiet" / "meeting mode"              │
│    │  speak only   │◀──── user command ────┐                     │
│    │  when spoken  │                       │                     │
│    │  to           │                       │                     │
│    └───────┬───────┘                       │                     │
│            │  "you can talk now"           │                     │
│            ▼                               │                     │
│    ┌───────────────┐                       │                     │
│    │  NORMAL       │──────────────────────▶│                     │
│    │  volunteer on │  triggers:                                  │
│    │  events       │   - task completion                         │
│    └───────┬───────┘   - blocker detected                        │
│            │           - research lands                          │
│            │           - contact cold                            │
│            │           - stage advance                           │
│            │                                                     │
│            │  high-signal events      ┌────────────────┐         │
│            ├──────────────────────────▶  SPEAK         │         │
│            │                          │  (short)       │         │
│            │                          └────────────────┘         │
│            │  low-signal events       ┌────────────────┐         │
│            └──────────────────────────▶ QUEUE for next │         │
│                                       │ brief / batch  │         │
│                                       └────────────────┘         │
│                                                                  │
│  Thresholds evolve from user corrections over time.              │
└──────────────────────────────────────────────────────────────────┘
```

Mode switches are **in-conversation, natural language** —
`"Nora, only speak if spoken to"` flips to reactive; tell her you're
done and she resumes normal. Thresholds for what counts as
"high-signal" learn from corrections.

### Spatial layout — anchor zones

Predictable region per node-type domain, dynamic positions within a
region. Compromise between "standardized template" (orientation you can
trust) and "dynamic visualization" (reactive to real interaction graph).

```
┌──────────────────────────────────────────────────────────────────┐
│                      CANVAS ANCHOR ZONES                         │
│                                                                  │
│                   ┌───────── NORTH ────────┐                     │
│                   │  orgs + companies      │                     │
│                   │  (who you work with)   │                     │
│                   └────────────────────────┘                     │
│                                                                  │
│    ┌─ WEST ─────┐  ┌─── CENTER ───┐  ┌─ EAST ───────────┐        │
│    │ knowledge  │  │ primary focus│  │ deals +          │        │
│    │ sources    │  │  (home org,  │  │ proposals +      │        │
│    │ research   │  │   or PCG for │  │ pipelines        │        │
│    │ activities │  │   Bodhi/     │  │                  │        │
│    │            │  │   admins)    │  │                  │        │
│    └────────────┘  └──────────────┘  └──────────────────┘        │
│                                                                  │
│                   ┌───────── SOUTH ────────┐                     │
│                   │  projects + deliver-   │                     │
│                   │  ables + tasks + decks │                     │
│                   └────────────────────────┘                     │
│                                                                  │
│  Within each zone: d3-force places nodes dynamically.            │
│  Same entity lands in the same zone across sessions —            │
│  spatial memory without brittle pin-all-positions.               │
│  Minimap (top-right, Vibeland-style) always shows your           │
│  position in the whole graph.                                    │
└──────────────────────────────────────────────────────────────────┘
```

### Always-evolving state — returning to the dashboard

Dashboard works on your behalf when you're gone. You never return to
the same state.

- **MVP**: silent — you see the current state, no callout.
- **Fast-follow**: nodes that changed since last session pulse briefly
  (~30s) so your eye catches the deltas. (User noted the pulse idea is
  nice but might be too much for Phase 1.)
- **Not in scope**: snapshot diffs, timeline scrubbers.
- Kickoff brief (above) covers the narrative summary regardless.

### Trust, safety, override

- **Audit log** — users see their own interactions and workflow logs
  surfaced via task cards that ran during the work.
- **Red-line data** — any field/entity can be marked "agent-blind";
  agent sees it exists but cannot read contents. Supersedes the
  "visibility levels" question — this is the visibility-control primitive.
- **Safe word / mute** — `"Nora, quiet"` / `"Topsi, quiet"` stops
  listening + speaking until re-enabled.
- **Graceful degradation** — graph works manually if the voice agent
  is offline; banner says "`<agent>` is offline."
- **Visible errors** — failures reported ("tried X, failed because Y;
  try Z?"), not silently retried.

### Storage choice — federated self-hosting

User raised the option of user/org self-hosted storage (local device
vs paid cloud). For this development cycle:
**local storage only, single-machine dev.** Federation / network /
payment primitives are deferred to a separate planning doc.

## Topsi multi-thread conversation model

Current conversations (`agent_conversations`) are agent-scoped and linear.
The vision needs **artifact-anchored threads** that can coexist.

**New schema** (one additive migration):

```sql
CREATE TABLE conversation_threads (
  id TEXT PRIMARY KEY,
  artifact_type TEXT NOT NULL CHECK (artifact_type IN
    ('company','person','deal','proposal','project','task',
     'deck','knowledge_source','research_pass','activity','client',
     'pipeline','brand_profile','agent','organization')),
  artifact_id TEXT NOT NULL,
  organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  title TEXT,
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active','archived','closed')),
  created_by TEXT NOT NULL,  -- user_id
  created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
  last_message_at TEXT,
  message_count INTEGER NOT NULL DEFAULT 0,
  UNIQUE(artifact_type, artifact_id, organization_id, created_by)
);
ALTER TABLE agent_conversation_messages ADD COLUMN thread_id TEXT
  REFERENCES conversation_threads(id) ON DELETE SET NULL;
CREATE INDEX idx_threads_artifact ON conversation_threads(artifact_type, artifact_id);
CREATE INDEX idx_threads_user ON conversation_threads(created_by, status);
```

**SSE multiplex:** one new endpoint
`GET /api/events/threads?user_id=<me>` that emits tagged events
`{thread_id, artifact_key, kind, payload}`. Frontend fans out by `thread_id`.
Replaces N open connections with 1.

**Routes**:
- `POST /api/threads` — create-or-get thread for `(artifact_type, artifact_id)`
- `GET /api/threads?artifact_type=&artifact_id=` — list threads anchored here
- `POST /api/threads/:id/messages` — append message (routes through existing Nora/Topsi chat internally)
- `GET /api/threads/:id/messages` — paginated history
- `PATCH /api/threads/:id` — status/title updates

**Multi-open UX**: the Interface surface maintains an ordered list of
open artifacts; each renders its own thread panel subscribed to the same
multiplexed SSE, filtered by `thread_id`.

### Diagram — thread anchoring + SSE multiplex

```
┌─────────────────────────────────────────────────────────────────┐
│  ONE user. MANY artifacts open. ONE SSE connection.             │
│                                                                 │
│   user_42 opens on /interface:                                  │
│    ├─ company:TIACA                                             │
│    ├─ deal:fusion-deal                                          │
│    └─ person:david-woods                                        │
│                                                                 │
│                       GET /api/events/threads                   │
│                       ─────────────────────▶  ╔═══════════════╗ │
│                                                ║ Axum SSE       ║│
│           tagged events flow back             ║ one connection ║│
│          {thread_id, kind, payload}   ◀─────── ╚═══════════════╝ │
│                        │                                         │
│                        ▼                                         │
│       frontend fans out by thread_id                             │
│           │           │           │                              │
│           ▼           ▼           ▼                              │
│      ┌────────┐  ┌────────┐  ┌────────┐                          │
│      │thread A│  │thread B│  │thread C│                          │
│      │TIACA   │  │fusion  │  │david   │                          │
│      │panel   │  │panel   │  │panel   │                          │
│      └────────┘  └────────┘  └────────┘                          │
│                                                                  │
│   DB:                                                            │
│   conversation_threads                                           │
│     (id, artifact_type, artifact_id, organization_id,            │
│      title, status, created_by, created_at, …)                   │
│     UNIQUE(artifact_type, artifact_id, org, user)                │
│                                                                  │
│   agent_conversation_messages.thread_id  ← nullable, additive    │
└──────────────────────────────────────────────────────────────────┘
```

## Artifact preview registry

`frontend/src/components/topology/previews/<NodeType>Preview.tsx`. Reused
where possible from the existing detail pages audit.

| node_type | Source component | Cost | Strategy |
|---|---|---|---|
| `company` | `pages/company-profile/*` (436 LOC) | high | Extract `<CompanyPreviewCard>` — logo, name, domain, headcount, intel summary |
| `person` | `pages/person-profile.tsx` (1130 LOC) | high | New lightweight `<PersonPreviewCard>` — name, title, photo, socials, last research date |
| `contact` | `pages/crm-contact-detail.tsx` (324 LOC) | **low** | Reuse header subtree directly |
| `deal` | `components/crm/deal-detail/DealHeader.tsx` | **low** | Import directly |
| `proposal` | `pages/proposals.tsx` detail modal | medium | Extract `<ProposalPreviewCard>` |
| `project` | `pages/projects.tsx` → hero sections | medium | Reuse `ProjectHeroCard` + summary |
| `client` | `pages/client-overview.tsx` (1021 LOC) | high | Extract `<ClientPreviewCard>` — logo, contact count, recent activity |
| `deliverable` | `pages/project-deliverables.tsx` (689 LOC) | medium | Extract per-deliverable card |
| `task` | `components/tasks/TaskCard.tsx` + details | **low** | Reuse card + summary block |
| `task_attempt` | `pages/agent-executions.tsx` | medium | Extract execution summary strip |
| `activity` | `components/crm/deal-detail/tabs/ActivityTab.tsx` | **low** | Reuse timeline entry component |
| `knowledge_source` | `intelligence/KnowledgeTab.tsx` card | medium | Extract single-source card |
| `research_pass` | — none — | **new** | Build `<ResearchPassPreviewCard>` (id, status, date range, findings count) |
| `pipeline` | `components/crm/CrmPipelineBoard.tsx` | medium | Extract stage summary strip |
| `brand_profile` | `organization-profile/tabs/WikiTab.tsx` | **low** | Reuse brand section |
| `deck` | `pages/video-studio.tsx` (1800+ LOC) | **new** | Don't extract — build minimal `<DeckPreviewCard>` (thumbnail, slide count, status) |
| `agent` | — none — | **new** | Build `<AgentPreviewCard>` (name, role, recent tasks) |
| `organization` | `pages/organization-profile/index.tsx` (436 LOC) | medium | Extract `<OrgPreviewCard>` |

Total: **5 low** (direct reuse), **7 medium** (small extraction), **4 high** (significant extraction), **4 new** (no starting point).

## Access-control matrix

Rule is applied by `filter_visible_nodes(ctx, nodes)` in one batch call.
Admin bypass is global. `org(u)` = the set of org IDs the user belongs to.

| node_type | Rule |
|---|---|
| `organization` | `node.id ∈ org(u)` |
| `company` | `node.organization_id ∈ org(u)` OR `node.organization_id IS NULL` (public KG entity) |
| `client` | `node.organization_id ∈ org(u)` |
| `person` / `contact` | `node.organization_id ∈ org(u)` OR contact is global KG (null org) |
| `deal` | `node.organization_id ∈ org(u)` |
| `proposal` | `node.organization_id ∈ org(u)` OR null (draft KG) |
| `project` | `node.organization_id ∈ org(u)` OR user is explicit project_member OR project is shared-board source |
| `deliverable` | transitive: `project.organization_id ∈ org(u)` |
| `task` / `task_attempt` | transitive via project; plus direct task assignee |
| `knowledge_source` | `owner_scope` match (project / org / user / company / deal) |
| `research_pass` | transitive via `contact.organization_id` |
| `pipeline` / `pipeline_stage` | `pipeline.organization_id ∈ org(u)` |
| `brand_profile` | transitive via project |
| `deck` | transitive via deal/proposal |
| `agent` | global — always visible, but actions gated per destination |
| `activity` | transitive via project |

**Known gap:** the current primitives can't express "see the node exists
but details are hidden" (visibility levels). Phase plan **does not**
depend on this — if access check fails, node is absent. Flagged for a
later follow-up if needed.

**Admin-only op:** `POST /graph/sync/global` uses `require_admin()`
(`access_control.rs:104`) — same gate as `permissions.rs` routes.

## Revised phasing

Replaces the old Phase 0–5. Each phase is independently shippable.

### Diagram — phase dependency graph

```
┌────────────────────────────────────────────────────────────────┐
│  ──▶ blocks                                                    │
│                                                                │
│   Phase 0 ──┬──▶ Phase 1 ──┬──▶ Phase 2 ──▶ Phase 3 ◀─ SHIP 1  │
│  foundation │   materialize│   canvas +      intel topology    │
│             │    coverage  │   previews      (low-risk ship)   │
│             │              │      │                            │
│             │              ▼      ▼                            │
│             │          Phase 4 ─────────▶ augments Phase 3     │
│             │          perspectives                            │
│             │              │                                   │
│             ├──────────────┴──▶ Phase 5 (agent actions)        │
│             │                        │                         │
│             │                        ▼                         │
│             └──────────────────▶ Phase 6 (Topsi threads)       │
│                                      │                         │
│                                      ▼                         │
│                                  Phase 7 ◀─ SHIP 2             │
│                                 /interface remodel             │
│                                      │                         │
│                                      ▼                         │
│                                  Phase 8 (polish/a11y)         │
└────────────────────────────────────────────────────────────────┘
```

### Phase 0 — Foundation (2–3 days, backend only)

Goal: unblock all downstream phases without committing to UX choices.

- `crates/db/src/models/entity_graph.rs` — add `global_subgraph`,
  `org_subgraph`, `subgraph_focused`, `filter_visible_nodes` (access helper).
- `crates/server/src/routes/graph.rs` — add `GET /graph/global`,
  `GET /graph/org/:org_id`, `GET /graph/subgraph`, `POST /graph/sync/global`
  (admin-gated), `GET /graph/actions?node_type=` (returns registry entries).
- `crates/server/src/services/node_actions.rs` — the `NodeActionRegistry`
  with the handlers that already exist (Phase 5 adds the rest).
- Migration: extend `edge_type` CHECK constraint to include the 13 new
  edge types (additive, no drop).
- Tests: `test_global_subgraph_access_filter`, `test_org_subgraph_scopes`,
  `test_subgraph_focused_depth_2`, `test_sync_global_idempotent`,
  `test_action_registry_coverage`.

Acceptance: `curl -H "Authorization: …" /api/graph/global` returns the
current 5-type graph filtered by org membership; `/api/graph/actions?node_type=company` lists Research / Summarize / OpenProfile.

### Phase 1 — Expand materialization (3–4 days, backend)

Add `sync_*_graph` functions for: org, client, deal, project, proposal,
deck, brand_profile, knowledge_source. Compose into `sync_global_graph`.
Answer the **persons vs crm_contacts** flag from the taxonomy. Seed on
existing demo data and verify coverage.

Acceptance: `GET /graph/global` returns every node type in the
taxonomy table except the live-query ones (task, task_attempt, activity,
research_pass).

### Phase 2 — Shared TopologyCanvas + preview registry (4–5 days, frontend)

- `frontend/src/components/topology/TopologyCanvas.tsx` — continues using
  the existing three.js + `@react-three/fiber` stack; refactor
  `KnowledgeGraphViz` physics from O(n²) to `d3-force` (node layout only,
  kept in 3D via three.js) to lift the 250-node cap.
- `frontend/src/components/topology/useTopologyData.ts` — React Query hook,
  keyed by `['topology', scope, perspective, filters]`.
- `frontend/src/components/topology/previews/` — the 18 preview components
  from the registry, lazy-loaded.
- Toolbar (scope, mode, filter pane, search) as composable sub-components.

Acceptance: canvas can render a 500-node graph at 60fps in 2D and 30fps
in 3D; clicking a node opens the correct preview.

### Phase 3 — Intelligence › Topology shell (2–3 days, frontend — ship this first)

- Replace `TopologyIntelView` (currently the knowledge-snapshot grid)
  with a shell mounting `<TopologyCanvas scope="org" perspective="graph">`.
- Keep the existing `topology_snapshot` view as an "Archive"
  sub-tab — it's surfaced from real knowledge data and shouldn't be lost.
- Add `?perspective=` and `?focus_id=` deep-link params.
- Deep-link buttons from company / person / proposal profiles
  ("View in Topology").

This is the **low-risk ship** — it lives inside an existing tab,
does not touch the already-production `/interface`, and proves the
canvas + previews + access filter end-to-end.

### Phase 4 — Perspective framework (3–4 days, full stack)

- `perspective` prop on `<TopologyCanvas>`; layout presets: Force,
  Hierarchy, Timeline, DAG.
- Client-side: `Pipeline`, `Intelligence`, `Relationship`, `Account`,
  `Ownership` — all from the single `/graph/subgraph` payload.
- Backend: dedicated endpoints for `Delivery`, `Temporal`, `Workflow`
  (each joins live tables).

### Phase 5 — Node → Agent action wiring + missing handlers (4–5 days, backend-heavy)

- Build the **8 missing handlers** flagged in the registry (advance-stage,
  create-proposal, assign-task, reassign, retry, draft-outreach,
  create-deck, inspect-logs node-wiring).
- Frontend: context menu on every node surfacing available actions from
  `GET /graph/actions`.
- Toast + graph-event feedback when an action completes.

### Phase 6 — Agent identities + multi-thread (7–8 days, full stack)

**Expanded from original scope.** Implements the Nora/Topsi duality in
addition to the thread mechanics.

- Agent identity model: two persistent agents (`Nora`, `Topsi`) with
  distinct records in `agents` table, each with its own persona
  config, STT/TTS stack, and prompt templates.
- **Tier resolver** for Nora: `fn resolve_nora_tier(counterparty_user_id,
  pcg_org_id) -> Tier { Admin | Client | Prospect | Stranger }` based
  on org membership and deal stage.
- **Dual-store logic** for Nora cross-org interactions:
  message/thread writes go to PCG DB **and** counterparty's DB
  transactionally; reads prefer local DB; Bodhi/admin reads union both.
- Migration + `ConversationThread` model + routes (as previously scoped).
- Multiplexed SSE stream (`/api/events/threads`).
- Agent-selection helper on login: `resolve_global_agent(user) -> Nora |
  Topsi` based on PCG admin membership.
- **Clarification-first prompt scaffolding** — shared system-prompt
  fragment both agents load.

### Phase 7 — `/interface` Jarvis mode (8–10 days, frontend-heavy)

**Expanded from original scope.** Implements the task-card workspace,
anchor-zone layout, and kickoff brief in addition to the basic shell
refactor.

- Refactor `pages/home/index.tsx` → `<InterfaceShell>`:
  graph canvas (full-bleed) + global-agent orb/chat (right rail, voice)
  + **task-card workspace layer** (floating above canvas, spawn/minimize/
  close, draggable, pop-out-to-window supported).
- **Task card lifecycle component** — `<TaskCard>` with:
  - spawn animation from source node
  - preview content + thread panel + live sub-agent progress
  - minimize → node-pulse indicator + background work
  - close → stop-work + 5-min undo
  - dock / pop-out to separate browser window
- **Node-state indicators** on the canvas: open-card glyph, pulsing
  ring for active background work.
- **Anchor-zone layout engine** — N/E/S/W/Center regions enforced by
  a zone-biased d3-force layout.
- **Minimap** (top-right, Vibeland design).
- **Kickoff brief** component rendering the MVP briefs (PCG Status or
  Home-Org Status) at session start.
- **Proactivity controls** — mute toggle, in-conversation mode flips
  ("stay quiet" / "meeting mode") detected by the agent.
- Voice/orb persistent dockable element.
- Agent surface: **Nora for Bodhi/PCG-admin sessions; Topsi for
  everyone else.** Both routed through the same orb UI; the identity
  visible in the persona + voice.

This is the biggest UX change and the right point to user-test.

### Phase 8 — Polish, a11y, performance (2–3 days)

- Color-blind glyphs, keyboard nav, screen-reader labels.
- Progressive reveal at >1000 nodes; collapse-by-org aggregation > 2000.
- Empty/loading/error states per perspective.
- Playwright smoke tests on `/organizations/:orgId/intelligence/topology`
  and `/interface`.

## Testing plan

**Backend (`cargo test --workspace`):**
- Phase 0: the 5 tests listed.
- Phase 1: `test_sync_all_entity_types_covers_taxonomy`,
  `test_persons_vs_crm_contacts_resolution`.
- Phase 5: one test per newly built handler (happy path + unauthorized).
- Phase 6: `test_thread_anchored_to_artifact`, `test_thread_stream_multiplex`.

**Frontend (Playwright):**
- Phase 3: `e2e/topology-intel.spec.ts` — open tab, assert canvas,
  filter, click node → preview opens.
- Phase 4: `e2e/topology-perspectives.spec.ts` — toggle perspectives,
  verify layout changes.
- Phase 6: `e2e/threads-multi-open.spec.ts` — open two artifact threads,
  send message in each, verify SSE fan-out.
- Phase 7: `e2e/interface-workspace.spec.ts` — end-to-end user journey.

**Access control (cargo test):**
- `test_filter_visible_nodes_respects_org_membership`,
  `test_cross_org_leakage`, `test_admin_sees_global`.

## Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| `persons` vs `crm_contacts` ambiguity breaks Phase 1 | Medium | Resolve in Phase 1 first day; if it's legacy, drop `persons` from taxonomy |
| `/interface` refactor regresses Nora/Topsi conversational flow | **High** | Ship Phases 3–6 first; `/interface` (Phase 7) only after preview + threading are proven on the Intelligence surface |
| Three.js O(n²) physics still caps node count after d3-force swap | Low | Benchmark early in Phase 2; fallback is react-force-graph-3d (adds ~300 KB) |
| Multiplexed SSE dropped events under load | Medium | Ack-based resume via `?since=<event_id>` query (pattern already exists in `event_stream.rs`) |
| Missing agent handlers take longer than 4–5 days | Medium | Ship Phase 5 incrementally; registry degrades gracefully — actions without handlers are hidden in the UI |
| Access-control batch helper misses a node type | High | Registry-driven: adding a node_type requires adding its access rule — enforced via a `match` without default |
| Topsi thread model conflicts with existing `agent_conversations` | Medium | Additive migration; threads reference conversations via nullable `thread_id` — old code paths keep working |
| Perspective switching causes refetch storms | Medium | Single-fetch payload + client-side layout swap (Q7 decision); only live-query perspectives hit the network |
| Vibeland `KnowledgeGraphViz` breaks when its `/graph/global` call finally works | Low | Phase 0 answers that call — Vibeland starts rendering real data. Verify Vibeland still behaves as a cinematic prop; likely fine. |

## Decisions — resolved in this round (2026-04-18)

- ✓ `/interface` phasing: Phase 7 ships **after** Phases 3–6.
- ✓ Visibility levels: **skip**; use red-line data ("agent-blind"
  fields) as the visibility primitive instead.
- ✓ `conversation_threads.unique(artifact, user)`: one thread per user
  per artifact for v1. Multiple named threads deferred.
- ✓ Storage location: **local single-machine** for this dev cycle;
  federated self-hosted storage is deferred to a separate planning doc.
- ✓ Agent identities: two — Nora (PCG, Bodhi/admin) and Topsi (platform
  default, per-user). Each with its own persona, voice stack, thread.
- ✓ Nora execution tiers: Admin / Client / Prospect / Stranger.
- ✓ Dual storage for Nora cross-org interactions.
- ✓ Task-card lifecycle: spawn from node → detach → minimize back to
  pulsing node → close with undo.
- ✓ Layout: anchor zones per entity domain + dynamic within zone.
- ✓ Minimap: Vibeland-style.
- ✓ Clarification-first agent behavior (both agents).
- ✓ Proactivity: default-on, tunable live in conversation.
- ✓ MVP kickoff brief content (pipeline, research, projects, health,
  agent activity, calendar, inquiries, **Pulse Intelligence**).

## Decisions resolved (2026-04-18 round 2)

- ✓ **Persons vs crm_contacts**: both tables live; `persons` is
  legacy being retired. `proposals.lead_id` still FK's `persons`. Plan:
  in Phase 1, migrate `proposals.lead_id` → `crm_contacts.id` (separate
  additive migration), then ingest `crm_contacts` as the `person`
  node_type. Drop `persons` from the graph taxonomy.
- ✓ **Phase 5 handler scope**: ship 5 mutation-only handlers
  (`advance-stage`, `assign-task`, `reassign`, `retry`, `inspect-logs`).
  The 3 generative ones (`create-proposal`, `create-deck`,
  `draft-outreach`) move to **Phase 5b** after Nora/Topsi land in
  Phase 6 — natural fits for agent-driven generation.
- ✓ **Vibeland fate**: leave untouched. Phase 0's `/api/graph/global`
  endpoint will make it start working against real data; decide
  post-Phase-0 whether to evolve.
- ✓ **Three.js + d3-force layout**: stay on existing three.js +
  `@react-three/fiber` stack; add `d3-force` only for layout math.
  No bundle-cost migration to react-force-graph-3d.
- ✓ **Admin gate on `/graph/sync/global`**: `require_admin()`
  (`access_control.rs:104`). Platform-admin only.
- ✓ **Changed-node pulse**: defer to Phase 8 polish. Phase 7 stays
  focused on the core task-card UX.
- ✓ **PCG org in DB**: exists in seed data.
  PCG = `0x01010101010101010101010101010101` (admin-owned).
  Sirak Studios = `0x02020202020202020202020202020202` (sirak-owned).
  Distinct orgs. Dual-storage model uses the PCG UUID as anchor.
- ✓ **Nora persona config**: currently split across
  `crates/server/src/routes/nora/config.rs` (runtime presets:
  `casual_british`, `british_executive_assistant`), the `agents`
  table schema (no explicit Nora seed row found), and the external
  `nora` crate's `PersonalityConfig::*` methods. Phase 6 will **seed
  a canonical Nora agent row** + a Topsi agent row during the
  agent-identity migration, consolidating the runtime config into
  the DB.

## Out of scope

- Editing the graph from the UI (add/remove nodes/edges directly)
- Saving graph snapshots to disk as artifacts
- Cross-tenant graphs (org-to-org edges spanning tenants)
- Graph diffing / time-travel UI beyond the Temporal perspective
- Replacing `agent_conversations` — the thread model is additive

## Rollout

1. Phase 0 + 1 merged behind no flag (pure additive backend, safe).
2. Phase 2 + 3 merged together — gives users a real feature
   (Intelligence › Topology) that works end-to-end.
3. Phase 4 + 5 as a follow-up PR — adds perspectives + actions.
4. Phase 6 as its own PR — threading is isolated and testable.
5. Phase 7 as its own PR — `/interface` remodel, user-visible change.
6. Phase 8 polish rolled into whichever PR needs it last.

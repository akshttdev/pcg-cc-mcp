# Graph Visualization — Integration Plan (aligned with 5-year roadmap)

**Date**: 2026-04-21
**Branch**: `feature/graph-viz-enhancement` (continues from Phases 0–3 already committed)
**Base**: `main @ 8640f2ee`
**Status**: Draft — awaiting sign-off on phase additions and stage sequencing

## 1. Executive summary

Phases 0–3 of `2026-04-16--plan--graph-viz-enhancement.md` shipped an
end-to-end entity graph: backend sync, access-filtered read APIs, action
registry scaffold, a shared `TopologyShell` component, and a mounted
Intelligence › Topology tab with fullscreen + deep-link support.

This plan covers **Phases 4–10** — the remaining integrations that turn
the visualization into the dashboard's operational surface per the
Vision section of the v1 plan. It reconciles each phase with the 5-year
roadmap so we:

1. **Don't ship ahead of Stage 0/1 blockers** (Agent Flow Orchestration
   Engine, multi-tenant hardening).
2. **Back-fill roadmap priorities that assumed this feature existed**
   — Stage 2 P20 (task DAG viz) and Stage 3 P28 (ATLAS Graph
   Dependencies) are implicit consumers of what we're building.
3. **Expand scope where it's additive** — `/interface` Jarvis remodel,
   perspective framework, task-card workspaces are not in the current
   roadmap but extend Stage 3 P31 (Nora voice production) and Stage 5
   P41 (VIBELAND 3D workspaces) naturally.

Two new phases are proposed beyond the v1 plan:

- **Phase 9 — ATLAS Graph Dependencies** (Stage 3 P28 alignment)
- **Phase 10 — Task-card → VIBELAND bridge** (Stage 5 P41 alignment)

## 2. Roadmap alignment

### 2.1 Where we sit today (post-Phase 3)

| Layer | Status | Roadmap stage |
|---|---|---|
| Entity graph DB tables | ✓ shipped | Stage 2 prep |
| `sync_*_graph` coverage for 9 entity types | ✓ shipped | Stage 2 prep |
| `/graph/global`, `/graph/org/:id`, `/graph/subgraph`, `/graph/sync/global` | ✓ shipped | Stage 2 prep |
| `NodeActionRegistry` (6 real handlers wired) | ✓ scaffold | Stage 2 P19 precursor |
| `filter_visible_nodes` (multi-tenant access) | ✓ shipped | Stage 1 P9 supporting |
| `TopologyShell` + d3-force + three.js canvas | ✓ shipped | Stage 2 P20 precursor |
| Intelligence › Topology tab (SHIP 1) | ✓ shipped | Stage 2 P19 |
| Preview registry | scaffold only | Stage 2–3 |
| Perspectives framework | not started | Stage 2 ext |
| Node → agent actions (beyond scaffold) | not started | Stage 2 P22 prep |
| Topsi multi-thread conversations | not started | Stage 3 P31 prep |
| `/interface` Jarvis remodel | not started | Stage 3 P31 |
| Polish / a11y / perf | not started | Stage 3 polish |
| ATLAS task DAG on top of this canvas | not started | Stage 3 P28 |
| VIBELAND 3D workspace bridge | not started | Stage 5 P41 |

### 2.2 Stage-by-stage fit

```
┌─────────────────────────────────────────────────────────────────────┐
│                     ROADMAP ALIGNMENT MAP                           │
│                                                                     │
│  Stage 0      Stage 1      Stage 2       Stage 3        Stage 4–5  │
│  Q2 2026     Q3 2026     Q1–Q3 2027     Q4 2027–Q2'28   2029–2031  │
│  ─────────  ──────────  ─────────────  ──────────────  ──────────  │
│  Agent Flow  Multi-      Task DAG viz   ATLAS graph     VIBELAND   │
│  Engine      tenant       (P19-20)      deps (P28)      3D world   │
│  (P0)        hardening                  Nora voice      (P41)      │
│              (P9)         BDI + CNP     prod (P31)                 │
│                           (P22)         Trust gates     Sovereign  │
│              Billing      DB strategy   (P30)           Stack      │
│              & metering   (P21)         Desktop app     (P42)      │
│                                         (P32)                      │
│                                         Data sovereignty           │
│                                         (P34)                      │
│                                                                     │
│  OUR PHASES (this plan):                                            │
│  ────────────                                                       │
│     ↓                                                               │
│  Phase 0-3 shipped ─┐                                               │
│                     ├── Phase 4 ─ perspectives                      │
│                     │                     Stage 2 ext               │
│                     ├── Phase 5 ─ agent actions                     │
│                     │                     Stage 2 P22 prep          │
│                     ├── Phase 6 ─ Topsi multi-thread                │
│                     │                     Stage 3 P31 prep          │
│                     ├── Phase 7 ─ /interface remodel                │
│                     │                     Stage 3 P31 ship          │
│                     ├── Phase 8 ─ polish / a11y / perf              │
│                     ├── Phase 9 ─ ATLAS task DAG                    │
│                     │                     Stage 3 P28               │
│                     └── Phase 10 ─ VIBELAND bridge (stretch)        │
│                                           Stage 5 P41               │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.3 Gaps we're filling ahead of roadmap

The 5-year doc treats the graph viz as **supporting infrastructure**
for ATLAS (Stage 3). By shipping it in Q2 2026 we unlock:

- **Earlier visibility into multi-tenant data shape** — stresses the
  access matrix in Stage 1 P9 (org isolation) before billing goes live
- **Operational surface for agent debugging** — gives Stage 0's Agent
  Flow Engine a visual companion without waiting for Stage 2
- **User-facing evidence of platform depth** — sales/demo asset that
  Stage 5's sovereign-stack narrative will need

Risk: building ahead of a stage priority means we carry the
maintenance burden longer. Mitigation: keep Phase 4–8 scope inside the
existing tab shell; Phase 7 (`/interface` remodel) is the only phase
that touches a production surface, and it's gated behind Phases 3–6
proving stable first.

## 3. Integration architecture

### 3.1 Diagram — graph → MCP tools → agent execution

The `NodeActionRegistry` exposes existing MCP tools (TaskServer has 26+;
Stage 0's Agent Flow Engine adds more). Click a node → dispatch a tool
→ artifact writes back to the graph via `sync_*_graph`.

```
┌───────────────────────────────────────────────────────────────────────┐
│                 GRAPH → MCP TOOLS → AGENT EXECUTION                   │
│                                                                       │
│   ┌─────────────────┐          ┌────────────────────────┐             │
│   │  TopologyCanvas │          │   NodeActionRegistry   │             │
│   │  (user click)   │──────▶   │  (NodeType, Action)   │             │
│   └─────────────────┘          │  → ActionHandler       │             │
│                                 └───────────┬────────────┘             │
│                                             │                          │
│                                             ▼                          │
│                                 ┌────────────────────────┐             │
│                                 │  /api/graph/actions    │             │
│                                 │  (GET — auth-gated)    │             │
│                                 └───────────┬────────────┘             │
│                                             │ dispatches               │
│                                             ▼                          │
│   ┌─────────────────────────────────────────────────────────┐          │
│   │           EXISTING HANDLER SURFACE                      │          │
│   │                                                         │          │
│   │  TaskServer MCP tools:                                  │          │
│   │    assign_task, manage_task_dependencies,               │          │
│   │    update_task, advance_stage …                         │          │
│   │                                                         │          │
│   │  HTTP handlers:                                         │          │
│   │    POST /api/companies/:id/research                     │          │
│   │    POST /api/contacts/:id/research                      │          │
│   │    POST /api/agents/:id/chat                            │          │
│   │    POST /api/wide-research                              │          │
│   └─────────────┬───────────────────────────────────────────┘          │
│                 │ runs                                                  │
│                 ▼                                                       │
│   ┌────────────────────────┐        ┌───────────────────────┐          │
│   │  Agent execution       │────────▶  Artifact produced    │          │
│   │  (Scout, Astra, Topsi) │        │  (research pass,      │          │
│   │                        │        │   deck, task result)  │          │
│   └────────────────────────┘        └───────────┬───────────┘          │
│                                                 │                       │
│                                                 ▼                       │
│                                  ┌──────────────────────────┐           │
│                                  │  sync_*_graph upserts    │           │
│                                  │  new nodes + edges       │           │
│                                  └──────────────┬───────────┘           │
│                                                 │                       │
│              SSE (Phase 6) ──────────────────── │                       │
│                                                 ▼                       │
│                                  ┌──────────────────────────┐           │
│                                  │  Canvas auto-refreshes   │           │
│                                  │  (new node pulses in)    │           │
│                                  └──────────────────────────┘           │
└───────────────────────────────────────────────────────────────────────┘
```

### 3.2 Diagram — Nora/Topsi orchestration loop

The `/interface` Jarvis surface closes the loop: user speaks → agent
selects an artifact → user approves → dispatch → result visible on graph.

```
┌──────────────────────────────────────────────────────────────────────┐
│             NORA/TOPSI ORCHESTRATION LOOP (/interface)               │
│                                                                      │
│        ┌──────────────┐                                              │
│        │  User voice  │                                              │
│        │  "Pull up    │                                              │
│        │   TIACA"     │                                              │
│        └──────┬───────┘                                              │
│               │                                                      │
│               ▼                                                      │
│    ┌─────────────────────┐        ┌─────────────────────┐            │
│    │   Nora (admin)      │  or    │   Topsi (user)      │            │
│    │   — PCG asset       │        │   — per-user        │            │
│    │   — tiered exec     │        │   — standard exec   │            │
│    └──────────┬──────────┘        └──────────┬──────────┘            │
│               │                               │                       │
│               └───────────────┬───────────────┘                       │
│                               │  intent → node id                     │
│                               ▼                                       │
│                    ┌────────────────────┐                             │
│                    │  /api/graph/       │                             │
│                    │   subgraph?focus_* │                             │
│                    └──────────┬─────────┘                             │
│                               │                                       │
│                               ▼                                       │
│            ┌──────────────────────────────────┐                       │
│            │   TopologyCanvas zooms to node   │                       │
│            │   + spawns task card             │                       │
│            └──────────────┬───────────────────┘                       │
│                           │                                           │
│            ┌──────────────┴──────────────┐                            │
│            ▼                             ▼                            │
│    ┌───────────────┐          ┌─────────────────────┐                 │
│    │ Task card     │          │ Thread panel        │                 │
│    │ (artifact     │          │ (Nora/Topsi chat    │                 │
│    │  preview)     │          │  anchored to node)  │                 │
│    └───────┬───────┘          └──────────┬──────────┘                 │
│            │  agent action                │  "research this"          │
│            │  click                       │                           │
│            └──────────┬───────────────────┘                           │
│                       ▼                                               │
│            ┌────────────────────────┐                                 │
│            │ Dispatch via           │                                 │
│            │ NodeActionRegistry     │                                 │
│            └──────────┬─────────────┘                                 │
│                       ▼                                               │
│            ┌────────────────────────┐                                 │
│            │ Agent runs             │                                 │
│            │ (minimized card keeps  │                                 │
│            │  running in bg)        │                                 │
│            └──────────┬─────────────┘                                 │
│                       │  SSE thread event                             │
│                       ▼                                               │
│            ┌────────────────────────┐                                 │
│            │ Canvas node pulses     │                                 │
│            │ Nora narrates result   │                                 │
│            └────────────────────────┘                                 │
│                                                                       │
│  Loop closes: user sees outcome on graph, continues conversation.    │
└──────────────────────────────────────────────────────────────────────┘
```

### 3.3 Diagram — perspective framework shape

Shared `<TopologyCanvas>` accepts a `perspective` prop. Client-side
presets reshape layout + dim rules over one fetched payload
(materialized perspectives). Live perspectives have their own
endpoints.

```
┌──────────────────────────────────────────────────────────────────────┐
│                  PERSPECTIVE FRAMEWORK                               │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │            <TopologyCanvas perspective="X" />                │   │
│  │                                                               │   │
│  │   ┌─────────────────────────────────────────────────────┐    │   │
│  │   │  One EntitySubgraph payload (Phase 1 shape)         │    │   │
│  │   └────────────────────────┬────────────────────────────┘    │   │
│  │                            │                                  │   │
│  │                            ▼                                  │   │
│  │            ┌────────────────────────────────┐                 │   │
│  │            │  PerspectivePreset (lookup)    │                 │   │
│  │            └──────────────┬─────────────────┘                 │   │
│  │                           │                                   │   │
│  │     ┌─────────────────────┼─────────────────────┐             │   │
│  │     ▼                     ▼                     ▼             │   │
│  │  filter set        layout preset        default visibility    │   │
│  │  (which types)    (force/hier/DAG/      (starts focused on    │   │
│  │                    timeline)             certain nodes)       │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  PERSPECTIVES (client-side over /graph/subgraph):                    │
│   ┌──────────────────────────────────────────────────────────┐       │
│   │ Graph      │ force      │ all types            │ default │       │
│   │ Pipeline   │ hierarchy  │ deal/stage/contact   │ CRM     │       │
│   │ Account    │ radial     │ one co + neighbors   │ 360     │       │
│   │ Relation.  │ force+bund │ person↔person        │ net     │       │
│   │ Intel      │ force w8d  │ research/ks/entities │ KG      │       │
│   │ Ownership  │ treemap    │ user → entities      │ admin   │       │
│   └──────────────────────────────────────────────────────────┘       │
│                                                                      │
│  PERSPECTIVES (dedicated endpoints — live-query):                    │
│   ┌──────────────────────────────────────────────────────────┐       │
│   │ Delivery   │ layered DAG  │ project→deliv→task→agent     │       │
│   │            │              │ /api/graph/perspective/delivery│     │
│   │ Temporal   │ timeline     │ activities windowed by time  │       │
│   │            │              │ /api/graph/perspective/temporal│     │
│   │ Workflow   │ flow DAG     │ agent_flows + triggers       │       │
│   │            │              │ /api/graph/perspective/workflow│     │
│   │ ATLAS      │ DAG crit path│ tasks + deps, blocking chain │       │
│   │ (Phase 9)  │              │ /api/graph/perspective/atlas │       │
│   └──────────────────────────────────────────────────────────┘       │
└──────────────────────────────────────────────────────────────────────┘
```

### 3.4 Diagram — federated storage roadmap

Stage 1 = local-only (current). Stage 3 P34 = sovereignty research +
first self-hosted bundle. Stage 5 P42 = full P2P sync. The dual-store
pattern for Nora cross-org interactions (v1 plan §Storage & visibility)
is a **Stage 3 feature** — it can't land before self-hosted deployment
exists.

```
┌──────────────────────────────────────────────────────────────────────┐
│            FEDERATED STORAGE — STAGE-GATED PROGRESSION               │
│                                                                      │
│  Stage 1 (today)       Stage 3 P34            Stage 5 P42            │
│  Q3 2026               Q2 2028                2030+                  │
│  ───────────────       ──────────────────     ──────────────         │
│  ┌────────────┐        ┌────────────────┐     ┌───────────────┐      │
│  │ Central    │        │ Self-hosted    │     │  Sovereign    │      │
│  │ SQLite     │───────▶│ option         │────▶│  Stack        │      │
│  │ on PCG     │        │ + data         │     │  P2P sync     │      │
│  │ server     │        │   residency    │     │  E2E encrypt  │      │
│  └────────────┘        └────────────────┘     └───────────────┘      │
│       ▲                                                               │
│       │  this is where Phases 0–8 live                                │
│       │                                                               │
│       └── Nora dual-store (v1 plan) = Stage 3+                        │
│           Until self-hosted exists, Nora writes PCG DB only.          │
└──────────────────────────────────────────────────────────────────────┘
```

## 4. Phase-by-phase detailed plan

### 4.1 Phase 4 — Perspective framework (4–5 days, full stack)

**Stage alignment:** Stage 2 extension — not on roadmap but additive.

**Goal:** One canvas, many named views. Perspective is a configuration
bundle: `{ filter, layout, dim_rules, default_focus }`.

**Backend (crates/server/src/routes/graph.rs):**
- `GET /api/graph/perspective/delivery?org_id=` — joins `projects →
  project_deliverables → tasks → task_attempts → agents` (live-query)
- `GET /api/graph/perspective/temporal?from=&to=&org_id=` — unions
  `activity_logs`, `research_passes`, entity `created_at` columns;
  returns tagged `{node, timestamp}` pairs
- `GET /api/graph/perspective/workflow?flow_id=` — `agent_flows`
  + their edges from the existing orchestration tables

**Frontend (components/topology/perspectives/):**
- `PerspectivePreset` type: `{ name, filter, layout, defaultFocus? }`
- `layouts/` directory — `forceLayout.ts` (default), `hierarchyLayout.ts`
  (tiered by `level` hint), `timelineLayout.ts` (x = timestamp),
  `dagLayout.ts` (sugiyama for workflow/atlas)
- `PerspectivePicker.tsx` — dropdown + `?perspective=` URL param
- Six client-side perspectives over the single fetch: Graph (default),
  Pipeline, Account, Relationship, Intel, Ownership
- Three endpoint-backed perspectives: Delivery, Temporal, Workflow

**Tests:**
- `cargo test test_delivery_perspective_joins`
- `cargo test test_temporal_perspective_windowing`
- `e2e/topology-perspectives.spec.ts` — toggle each preset, assert
  layout change + node-count change

**Files new:**
- `frontend/src/components/topology/perspectives/` (new directory)
- `crates/server/src/routes/graph.rs` (+3 handler functions)

### 4.2 Phase 5 — Node → agent action wiring (4–5 days, backend-heavy)

**Stage alignment:** Stage 2 P22 prep (BDI + Contract Net) — this is
the UI-side of Stage 3 P30 trust gates.

**Goal:** Every node offers its real action surface. Confirms the
"click anything → dispatch something" Jarvis promise.

**Phase 5a — the 5 mutation handlers (this phase):**
- `PATCH /api/crm/deals/:id/stage` — advance-stage
- `POST /api/tasks/:id/assign` — assign-task
- `PATCH /api/tasks/:id/assignee` — reassign
- `POST /api/task-attempts/:id/retry` — retry (wrap existing retry)
- Node-wiring for `inspect-logs` (surface existing
  `/api/events/processes/:id/logs` via context menu)

**Phase 5b — generative handlers (deferred to Phase 6+):**
- Create-proposal, create-deck, draft-outreach — natural fit for
  Nora/Topsi once agent identities land

**Registry updates (node_actions.rs):**
- Flip the 5 new handlers' `implemented: false` → `true` as they land
- Add `requires: Required::Admin | ::Editor | ::Owner` per-action
  (Stage 3 P30 trust gate field)

**Frontend (components/topology/):**
- `NodeContextMenu.tsx` — right-click menu on canvas nodes pulling
  from `/api/graph/actions?node_type=`
- `ActionConfirmDialog.tsx` — shows handler + scope + side-effects
  before firing (pre-Stage 3 trust-gate UX)
- Success/failure toasts invalidate topology queries so the graph
  refreshes

**Tests:**
- `cargo test` per new handler — happy path + unauthorized
- `e2e/node-actions.spec.ts` — right-click deal → advance stage →
  assert graph reflects new stage edge

### 4.3 Phase 6 — Agent identities + Topsi multi-thread (7–8 days)

**Stage alignment:** Stage 3 P31 prep (Nora voice production) — this
phase makes conversational context persistent before the voice
pipeline leans on it.

**Goal:** Nora and Topsi live as distinct agent identities with
persistent, artifact-anchored threads. Replaces current stateless
conversation model.

**Migration (crates/db/migrations/):**

```sql
CREATE TABLE conversation_threads (
    id TEXT PRIMARY KEY,
    artifact_type TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,       -- Nora or Topsi row in agents table
    title TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    last_message_at TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(artifact_type, artifact_id, organization_id, created_by)
);
ALTER TABLE agent_conversation_messages ADD COLUMN thread_id TEXT
    REFERENCES conversation_threads(id) ON DELETE SET NULL;
```

**Rust (crates/db/src/models/conversation_thread.rs):**
- `ConversationThread::get_or_create(artifact_id, type, org, user, agent)`
- `append_message`, `find_by_artifact`

**Rust (crates/server/src/services/agent_identity.rs) — new:**
- `resolve_global_agent(user, org) -> AgentChoice::Nora | ::Topsi` —
  Nora only for Bodhi + PCG admins; else Topsi
- `resolve_nora_tier(counterparty, pcg_org)` → Admin / Client /
  Prospect / Stranger tier
- Dual-store **deferred to Phase 6b** (pending Stage 3 P34 self-hosted
  story). For now, Nora writes PCG DB only. Document the gap.

**Routes (crates/server/src/routes/threads.rs) — new:**
- `POST /api/threads` — get-or-create for `(artifact, user)`
- `GET /api/threads?artifact_type=&artifact_id=`
- `POST /api/threads/:id/messages`
- `GET /api/threads/:id/messages?since=`
- `GET /api/events/threads?user_id=<me>` — **multiplexed SSE**;
  tagged events `{thread_id, artifact_key, kind, payload}`

**Seed data:**
- Insert canonical Nora agent row in a new migration
- Insert canonical Topsi agent row in same migration

**Frontend (components/threads/):**
- `useThreadsForArtifact(artifactType, id)` — React Query hook
- `useMultiplexedThreadStream(userId)` — subscribes once, fans out by
  `thread_id`
- `ThreadPanel.tsx` — message list + input, lazy-mounted per artifact

**Clarification-first behavior prompt:**
- Prompt scaffolding in `crates/server/src/prompts/` (or wherever
  existing Nora prompts live — Phase 6 first day's scouting task)
- Instruction: "When uncertain about intent or missing required
  information, ask a clarifying question. Do not fabricate."

### 4.4 Phase 7 — `/interface` Jarvis remodel (8–10 days, frontend-heavy)

**Stage alignment:** Stage 3 P31 ship — Nora voice production surface.

**Goal:** Rebuild `/interface` around the canvas + Topsi/Nora chat +
multi-open task-card workspace. Displaces the current orb-only layout.

**Refactor (frontend/src/pages/home/index.tsx):**
- Extract current orb + voice logic into `<ConversationOrb>` — kept,
  dockable, not full-screen
- New shell layout:
  ```
  ┌──────────────────────────────┬─────────────────┐
  │                              │                 │
  │   <TopologyCanvas            │ <ConversationOrb>│
  │    scope={user-orgs}         │                 │
  │    perspective={active}      │ Nora/Topsi thread│
  │    onNodeClick={spawnCard}/> │ anchored to     │
  │                              │ focused card    │
  │                              │                 │
  ├──────────────────────────────┴─────────────────┤
  │ <WorkspaceTray>                                 │
  │  [TIACA] [fusion-deal] [david-woods] [+]        │
  │                                                 │
  │  Open task cards; each has preview + thread     │
  │  Minimize → back to pulsing node on canvas      │
  └─────────────────────────────────────────────────┘
  ```

**New components:**
- `TaskCard.tsx` — spawn/minimize/close lifecycle; draggable;
  multi-monitor pop-out via `window.open`
- `WorkspaceTray.tsx` — bottom dock of open cards (taskbar analogue)
- `NodeStateOverlay.tsx` — canvas overlay that draws pulse rings on
  nodes with open cards + active agent work
- `KickoffBrief.tsx` — session-start MVP brief: PCG Status Update for
  Nora; Home-Org Status for Topsi. Content source: new
  `/api/briefing/kickoff` endpoint assembling pipeline + research +
  project + pulse summary

**Anchor-zone layout:**
- Extend `forceLayout.ts` with `withZones({ north, south, east, west,
  center })` — uses `d3-force` collision + custom per-zone attractors

**Proactivity gates:**
- `useProactivityMode` Zustand store with `reactive | normal` states
- Natural-language mode switch detected in Nora/Topsi prompt
  (`"only speak if spoken to"` → flips to reactive)

### 4.5 Phase 8 — Polish / a11y / performance (2–3 days)

**Stage alignment:** Stage 3 polish across all previous phases.

- Color-blind glyphs per node type (▲ ■ ● ◆) as a11y toggle
- Keyboard navigation — `tab` cycles visible nodes, `enter` opens card,
  `esc` closes, `1`/`2` switches 2D/3D, `/` focuses search
- Screen-reader labels on all toolbar controls
- Progressive reveal > 1000 nodes (LOD: hide labels, simplify geometry)
- Collapse-by-org aggregation > 2000 nodes
- Changed-node pulse (fast-follow: nodes that changed since last
  session pulse for ~30s on return)
- Empty/loading/error states per perspective
- Playwright smoke on all perspectives

### 4.6 Phase 9 — ATLAS Graph Dependencies (5–6 days) ⚡ NEW

**Stage alignment:** Stage 3 P28 direct delivery — this replaces what
the roadmap scoped for Cytoscape.js with our existing three.js stack.

**Goal:** Visualize task dependencies with critical-path highlighting,
blocking/blocked propagation, circular-detection warnings.

**Backend (crates/db/src/models/task_dependency.rs):**
- Extend existing model with a **recursive CTE** query:
  ```sql
  WITH RECURSIVE task_chain AS (
    SELECT id, title, status, 0 AS depth, NULL AS blocker_id
    FROM tasks WHERE id = ?
    UNION ALL
    SELECT t.id, t.title, t.status, tc.depth + 1, td.blocker_task_id
    FROM tasks t
    INNER JOIN task_dependencies td ON td.task_id = t.id
    INNER JOIN task_chain tc ON td.blocker_task_id = tc.id
    WHERE tc.depth < 10  -- cycle guard
  )
  SELECT * FROM task_chain
  ```
- `compute_critical_path(project_id)` — longest dependency chain with
  pending tasks
- `detect_cycles(project_id)` — returns any cycles found

**Routes:**
- `GET /api/graph/perspective/atlas?project_id=` — returns
  `{ nodes: Task[], edges: Dependency[], critical_path: [task_id],
     cycles: [[task_id, task_id]] }`

**Frontend (components/topology/perspectives/atlas/):**
- `dagLayout.ts` — Sugiyama layered DAG (already listed in Phase 4;
  Phase 9 consumes it)
- `CriticalPathHighlight.tsx` — red glow on critical-path nodes +
  animated edge flow
- `CycleBadge.tsx` — warning pill on any node in a cycle

**Trust gate integration (Stage 3 P30):**
- Agent actions that would break a dependency (delete task, close
  blocking task) gate on `require_editor` + `ActionConfirmDialog`
  showing downstream-impact count

### 4.7 Phase 10 — VIBELAND bridge (stretch, 5 days) ⚡ NEW

**Stage alignment:** Stage 5 P41 precursor — task cards become VRM
avatar rooms.

**Goal:** Map task cards to VIBELAND virtual project rooms. When a
user opens a card, there's an optional "Enter VIBELAND" button that
spawns them as a VRM avatar in that card's 3D room. Multiple users in
the same card share the room.

**Scope-limited for Phase 10:**
- Detect existence of `frontend/src/components/vibeland/` + its
  existing VRM infrastructure
- Add `VibelandRoomLink.tsx` on TaskCard — deep-links to
  `/vibeland?room=<artifact_type>:<artifact_id>`
- Backend: extend `conversation_threads` with optional
  `vibeland_room_id TEXT` column
- Frontend: Vibeland page reads `?room=` and loads that artifact's
  existing `KnowledgeGraphViz` as in-world prop

**Out of scope (Stage 5 proper):**
- Multi-user presence sync
- VRM avatar customization per user
- Real-time voice in VIBELAND

## 5. Dependencies & sequencing

### 5.1 Diagram — extended phase dependency DAG

```
┌──────────────────────────────────────────────────────────────────────┐
│         EXTENDED PHASE DEPENDENCIES (Phases 0–10)                    │
│                                                                      │
│   ✓ P0 foundation                                                    │
│   ✓ P1 materialization                                               │
│   ✓ P2 canvas+previews                                               │
│   ✓ P3 intel topology (SHIP 1)                                       │
│        │                                                             │
│        ├──▶ P4 perspectives ──────┬──▶ P9 ATLAS task DAG             │
│        │    (Stage 2 ext)         │   (Stage 3 P28)                  │
│        │                           │                                 │
│        ├──▶ P5 agent actions ──┐  │                                  │
│        │    (Stage 2 P22 prep) │  │                                  │
│        │                        ▼  │                                 │
│        └──▶ P6 Topsi threads ──▶   │                                 │
│             (Stage 3 P31 prep)  │  │                                 │
│                                 │  │                                 │
│                                 ▼  ▼                                 │
│                           P7 /interface remodel (SHIP 2)             │
│                                (Stage 3 P31 ship)                    │
│                                    │                                 │
│                                    ▼                                 │
│                              P8 polish + a11y + perf                 │
│                                    │                                 │
│                                    ▼                                 │
│                       (stretch) P10 VIBELAND bridge                  │
│                                (Stage 5 P41 precursor)               │
│                                                                      │
│  Critical chain: P4 → P5 → P6 → P7 (22–28 days)                      │
│  Parallel: P9 can start once P4 lands (shares dagLayout)             │
└──────────────────────────────────────────────────────────────────────┘
```

### 5.2 Stage 0 blockers to respect

Do NOT touch these while Phases 4–10 are in flight:

- **Agent Flow Orchestration Engine** (P0) — Stage 0 is still
  stabilizing. Phase 5 can **call** the engine via MCP tools, but
  cannot reshape the engine itself.
- **Multi-tenant billing + isolation audit** (Stage 1 P9) — our
  `filter_visible_nodes` helper is already aligned; if Stage 1 lands
  stricter isolation rules, update the access matrix and move on.
- **Shared test seed** (`dev_assets_seed/test-seed.sqlite`) — Phase 6's
  new `conversation_threads` table needs to appear in the seed script
  but not break existing E2E tests.

### 5.3 MCP TaskServer integration points

Phase 5 and Phase 9 need to consume existing MCP tools rather than
building new endpoints where possible. The registry should prefer
MCP-backed routes:

| Our action | MCP tool (existing) |
|---|---|
| Task.AssignTask | `task_server.assign_task` |
| Task.Reassign | `task_server.update_task` (assignee field) |
| Task.AdvanceStage | — (Phase 5 builds new route) |
| Task.InspectLogs | `/api/events/processes/:id/logs` (HTTP, not MCP) |
| Task.ManageDependencies | `task_server.manage_task_dependencies` (Phase 9 uses) |
| Company.Research | `/api/companies/:id/research` (HTTP) |

Wiring rule: the registry `ActionHandler` struct already carries
`route` + `method`. Add an optional `mcp_tool: Option<&'static str>`
field so the frontend can prefer the MCP invocation when available
(lower auth overhead; aligned with Stage 0's MCP-first strategy).

## 6. Risks & decisions needed

### Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Phase 7 refactor regresses `/interface` orb/voice flow | **High** | Phase 3 SHIP 1 already landed; Phase 7 gated on Phases 4–6 stabilizing first. Ship behind `?interface=jarvis` flag for first week. |
| Phase 6 thread table schema drifts from Stage 3 P31 design | Medium | Seed minimal; leave room for nullable `vibeland_room_id`, `sse_channel` extensions. Document migration path. |
| Phase 9 ATLAS depends on Phase 4 dagLayout landing | Low | Phase 9 can prototype its own sugiyama if Phase 4 slips. Keep an adapter interface. |
| Roadmap Stage 3 P28 scopes to Cytoscape.js — we're using three.js | Low | Document the choice in this plan. Three.js + d3-force matches existing stack; bundle saving ~300 KB + visual consistency with Nora orb. |
| Dual-store for Nora cross-org breaks when self-hosted ships | Medium | Defer dual-store to **Phase 6b** (post-Stage-3 P34). Phase 6 stays single-store. |
| VIBELAND P10 stretch becomes priority creep | Low | Explicitly gated behind Phase 8. Skip cleanly if bandwidth tight. |

### Decisions still needed (user-facing)

1. **Phase 6 agent seeding** — confirm canonical Nora/Topsi rows get
   inserted by migration (vs. runtime API). Migration is simpler;
   runtime is more flexible.
2. **Phase 7 behind a flag?** — ship `/interface` remodel behind
   `?interface=jarvis` for first week so regression is reversible?
3. **Phase 9 ATLAS priority** — land before or after Phase 7? Stage 3
   P28 and P31 are both Q4 2027 in the roadmap. Starting Phase 9 in
   parallel with Phase 7 (different owners) is fastest.
4. **Phase 10 VIBELAND bridge** — keep in plan as explicit stretch
   phase, or move to a separate planning doc (Stage 5 territory)?
5. **MCP-first wiring** — confirm the `ActionHandler` struct should
   grow an `mcp_tool` field and the frontend prefers MCP invocation
   where available. Cleanest alignment with Stage 0's MCP-first
   direction.
6. **Perspective presets in URL** — persist `?perspective=pipeline`
   in the URL (shareable links) or in Zustand (per-user memory)? Both
   is the right answer; confirm priority.
7. **Trust gate UX** — for Phase 5 action confirmation, what's the
   confidence threshold where Nora auto-proceeds vs. always asks
   Bodhi? Per the existing vision doc "Nora does whatever I tell her"
   + 5-min undo = always proceed + always show in activity log?
   Confirm.
8. **Kickoff brief content in Phase 7** — freeze the MVP list from
   the v1 plan (pipeline / research / projects / client health / agent
   activity / calendar / inquiries / Pulse Intelligence) or iterate?

## 7. Rollout

1. **Phase 4** merged behind no flag (additive perspective endpoints +
   client-side preset switcher).
2. **Phase 5** merged as a PR — mutation handlers are reversible via
   5-min undo + activity log.
3. **Phase 6** merged as its own PR — migration + model + routes +
   thread UI. Isolated and testable.
4. **Phase 7** merged behind `?interface=jarvis` query flag for first
   week; promote to default after regression window.
5. **Phase 8** polish rolled into whichever PR needs it last.
6. **Phase 9** ATLAS — separate PR, can ship independently of Phase 7.
7. **Phase 10** VIBELAND bridge — separate PR only if bandwidth
   allows; otherwise defer to dedicated Stage 5 plan.

## 8. Use-case clarifications (2026-04-21 session)

Live refinement round with Bodhi. Answers shape Phase 4+ and add new
phases. Material extracted here so it survives context clears.

### 8.1 Canonical workflows

- **Tuesday morning**: Bodhi wakes Nora with a greeting (push-to-talk
  gate optional). Nora immediately delivers status since last
  conversation. Bodhi simultaneously scans the canvas, takes the
  conversation any direction.
- **New Topsi user**: guided discovery. Topsi introduces herself,
  extracts relevant data from the user while educating them on
  dashboard capabilities.
- **First three perspectives to ship**: Graph + Pipeline + Account
  (default recommendation accepted).
- **First mutation action**: `assign-task` (most important; exercises
  the agent-dispatch loop end-to-end).

### 8.2 Nora ≡ Topsi (unified engine, scoped roles)

This supersedes the v1 plan's "two distinct agents" framing.

- **Same underlying engine**: persona config, voice stack, memory
  system, MCP tools, reasoning capacity are shared.
- **Nora = Topsi with `scope: platform_admin + bodhi_personal`**:
  complete knowledge of the ecosystem + Bodhi-only privileges + APN
  control authority.
- **Topsi = Topsi with `scope: user + org`**: native platform
  orchestrator that users get to run their own orgs.
- Migration implication: Phase 6 seeds both as rows in `agents` with
  the same row shape; they differ by a `scope` field (or equivalent).
  Simpler than the original "two separate identities" design.

### 8.3 APN / Pythia / hive-mind

**APN (Agent Peer Network)** is a first-class vision component, not a
future stretch.

- **Today**: 3 basic PCs, master + peer nodes, not fully unified.
  Dashboard hosted on master node. RustDesk is the device-control
  layer Nora uses to drive agent sessions on peer machines.
- **Goal**: network of machines clustering to create hive-mind =
  **Pythia**. Nora directly controls all devices seeded into the
  network.
- Dashboard IS the cockpit for Pythia orchestration.
- Peer machines become first-class nodes in the entity graph
  (candidate node_type: `peer_node`, with edges to agents/tasks
  running on them).

### 8.4 VIBE token economy

- **Currency**: VIBE tokens (confirmed — matches `vibe_balance` +
  `vibe_budget_total` already present on `organizations` table).
- **Wallets**: every layer — org, agent, user. Users can accumulate
  + spend their own VIBE.
- **Current work**: mostly org-funded (on behalf of the org).
- **Budget exhausted**: hard stop. Nora stops acting; UI shows
  "budget exhausted" banner. No graceful degradation / cheaper-model
  fallback yet.

### 8.5 Agent dispatch + approval gates

- **Nora → Auri flow**: Nora delegates code work to Auri (code agent)
  via sub-invocation. Nora never edits repo directly.
- **Human approval required**: Human + Nora + Auri in same thread.
  Human is generally required to approve deliverables before they can
  be "delivered." Two-gate trust system:
  1. **Dispatch approval** (pre-action): confirm before agent fires
  2. **Delivery approval** (post-action): preview output, approve to
     ship. [Approve] / [Request changes] / [Reject], inline diff/
     preview per type.
- **Nora's planning role**: triages inputs to understand the intent;
  delegates actual planning to sub-agents as tasks. Madhav's
  orchestration layer handles the planning-as-tasks pattern.

### 8.6 Orchestration layer (Madhav's work)

- **Status**: in flight, separate workstream.
- **Boundary with Nora**: Nora handles voice + UI + triage
  (classification of intent). Orchestration layer takes the
  classified intent and handles planning + dispatch.
- **Our design implication**: Phase 5 handler wiring should be an
  adapter that plugs into Madhav's layer when it lands. Don't
  duplicate. Leave an interface boundary.

### 8.7 Workflow taxonomy

Workflows organize into four buckets:

- **Dev workflows** (code, builds, deploys)
- **Social workflows** (social media, community, events)
- **Sales workflows** (outreach, deals, proposals)
- **Client workflows** (delivery, support, retention)

These become **perspective presets** on the graph (extends Phase 4's
perspective framework) so Phase 4 ships with 3 + 4 = 7 core
perspectives.

### 8.8 External ingest

- **Sync**: real-time via webhooks (Gmail push, GitHub webhooks, etc.)
- **Window**: go-forward only from install date.
- **Past-content discovery**: user can ask Nora to dig into past
  content; Nora queries the graph first, then offers to backfill from
  the source if no match. Phase 1b ships with discovery fallback.
- **Output shape**: everything becomes `entity_graph_nodes` with
  edges. No separate data lake.
- **Priority**: aspirational "all sources" but **well-structured,
  leverageable data** is the gate. The 100K node target is a stress
  number, not a literal goal — the real mandate is "big and powerful
  tool."

### 8.9 Multi-user / thread handoff / access

- **Multi-user per org**: yes. David Woods at Fusion has his
  employees; each gets their own Topsi, personalized per user, with
  org-scope access.
- **Thread handoff (updated to C)**: when Bodhi hands off a thread to
  Ben, it becomes **co-owned** — Nora AND Ben's Topsi contribute;
  each sees the other's messages. **Owner (Bodhi) can revoke access
  at any time**.
- **Message label**: "from Bodhi (via Nora)" — ownership trail
  visible.
- **Transfer payload**: message history + context summary Nora writes
  for Ben's Topsi.

### 8.10 Security posture (stage-appropriate)

- **Current stage**: permissive. "Users have no right to privacy at
  this stage" — acknowledged, explicit. This is Bodhi's system;
  users are guests.
- **Exception tier**: Bodhi + Admin + Nora exempt. Can see everything.
- **Threat model**: primarily (A) agents leaking to unauthorized
  agents/users, plus (C) external attackers. Explicitly: **prevent
  other users from exploiting Nora** to extract data, make
  unauthorized changes, or trigger transfers.
- **Personal vs org data**: personal data private by default; org
  owns all org data + workflow logs; users share freely in short
  term to enhance collaboration.
- **HIPAA-grade**: aspirational North Star for Stage 3+. Not a
  current blocker.
- **Topsi's visibility**: same as its user's + Bodhi/Admin overlay.
  Topsi cannot see beyond user scope, but Bodhi/Admin can see any
  Topsi session's output.

### 8.11 Scale + aggregation

- **100K nodes** is a *stress-test framing*, not a literal 2026 goal.
- **Finding things at scale**: Nora is the primary navigator ("show
  me X" → canvas zooms/filters). Minimap + search remain as
  fallbacks.
- **Aggregation/collapse** moves from Phase 8 → **into Phase 4**
  alongside perspectives. Collapse-by-org, semantic clustering at
  zoom-out, LOD rendering are day-one concerns.

### 8.12 Scope boundaries

- **NOT a project management tool for external contractors**: to be
  an org in the dashboard, you run the stack on your own hardware.
  Orgs can resell access to their clients (the org is the tenant,
  not the client).
- **NOT an IDE yet**: agents handle code editing. Future: IDE-style
  workspace for devs in the dashboard.
- **NOT a general-purpose ChatGPT surface**: focused on collab of
  humans + agents across teams in a business sense. Personal use
  cases come later.

### 8.13 Open / pending clarifications (for next session)

Questions asked but not yet answered — deferred for next round:

- **APN-1**: current RustDesk flow details + aspirational MCP-daemon model
- **APN-2**: how peers get enrolled into the network
- **APN-3**: whether peers appear as graph nodes (my default: yes,
  `peer_node` type)
- **VIBE-1**: where a user gets VIBE from (user earn / org allocate / purchase / all)
- **VIBE-2**: cost-deduction timing (pre-debit / post-tally / pre-confirm)
- **Orch-1**: Madhav's layer status (exists / in-flight / not started)
- **Orch-2**: Nora-reasoning vs orchestration-layer boundary (my default: triage-only / delegate planning)
- **WF-1**: workflow taxonomy = perspective / nav / tags / all
- **Del-1**: deliverable approval UX specifics (thread-only / with-preview / queue / both)
- **Disc-1**: past-content discovery — cache-first vs always-live
- **Sec-1**: Topsi visibility when Bodhi/Admin read through it

These don't block Phase 4 start; they're Phase-5-and-beyond clarifications.

## 9. Plan impact summary (from 2026-04-21 session)

New phases / sub-phases introduced:

- **Phase 1b** — External ingest pipeline (webhook receivers, real-time
  normalizers → entity_graph, past-content discovery fallback).
- **Phase 5c** — VIBE wallet integration on action dispatch (org →
  agent → user cascade; hard-stop on budget exhaust).
- **Phase 5d** — Two-stage trust gate (dispatch approval + delivery
  approval with preview per type).
- **Phase 6 updates** — Single `agents` schema for Nora + Topsi; `scope`
  field differentiates. Madhav-adapter interface boundary.
- **Phase 11** — APN integration layer (peer-node registry, RustDesk
  controller bridge, peer_node graph entity).
- **Phase 4 expansion** — Aggregation/collapse moves forward from
  Phase 8; four workflow perspectives (Dev/Social/Sales/Client) added
  to the existing three (Graph/Pipeline/Account).
- **Phase 7 additions** — active-peer rail, VIBE balance pill,
  pending-deliverables drawer.

## 10. Success criteria (cross-phase)

- A Bodhi session on `/interface` opens and works end-to-end: talks to
  Nora → graph zooms and pulses → task card spawns → agent dispatches
  → result renders back on graph. **"I love this" moment**: Bodhi
  stops opening Claude Code CLI; voice to Nora runs builds via Auri +
  human approvals.
- A non-admin session on `/interface` gets the same shape with Topsi
  and tier-appropriate access, within the guided-discovery onboarding
  frame on first run.
- Intelligence › Topology tab remains the stable fallback (doesn't
  regress through Phases 4–10).
- All perspective presets load in < 500ms (client-side) or < 2s
  (live-query endpoints).
- Zero pre-existing pages regress — orb still works, CRM pipeline still
  works, Vibeland still works (whether Phase 10 ships or not).
- The 5-year roadmap's Stage 2 P19-20 and Stage 3 P28 can be ticked
  off using this work as the deliverable.
- Nora can dispatch a build to Auri on a peer machine; human approves
  the diff; the merge appears on the canvas as a new artifact.
- VIBE wallet deductions visible per action; running low = banner +
  hard stop (not silent degradation).

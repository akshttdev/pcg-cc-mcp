# Knowledge Graph & Pulse Engine — Advancement Blueprint

> **Status:** Draft v1 — codebase audit @ commit `db0df61` (branch `auri/55872736`)
> **Audience:** PCG engineering, Auri, Nora ops
> **Scope:** `pcg-cc-mcp` — Rust backend (`crates/db`, `crates/server`, `crates/services`) + React frontend (`frontend/src`)

---

## 1. Executive Summary

PCG already operates *two-and-a-half* knowledge systems running side-by-side:

1. **Knowledge Sheaf** — a polymorphic, scope-aware "knowledge sources" table (`project_knowledge_sources`) that unifies six content categories (conversations, artifacts, pulse content, context injections, entities, topology snapshots) behind a single coverage/staleness model. Augmented by a **user-private sheaf** (`user_knowledge_sources`) for personal skills/notes/bookmarks.
2. **Entity Graph** — a denormalised business topology (`entity_graph_nodes` + `entity_graph_edges`) materialised from CRM, project, and proposal tables via per-table `sync_*_graph` helpers. Renders the org → client → deal → pipeline → contact → proposal mesh.
3. **Pulse Engine (external)** — an out-of-process Rust service collecting RSS/Twitter/YouTube/Reddit/web signals, publishing to NATS, with `pcg-cc-mcp` acting as **consumer** (`pulse_consumer.rs`) and HTTP proxy (`routes/pulse.rs`). Triggers fire-and-forget tasks back into the Sheaf.

These three pieces work, but they do not yet **learn**. Today the Sheaf is filled by INSERT triggers, the Entity Graph is rebuilt on admin button-press (`POST /api/graph/sync/global`), and the Pulse Engine fires content one-way into the Sheaf without ever closing the loop back to recompute relevance or extract new entities. No background worker exists today for KG refresh, coverage decay, or entity resolution. Agents (Nora/Scout/Astra/Auri) write back through a single MCP tool (`add_knowledge`) and through `intelligence.rs` handlers, but cannot promote findings into the Entity Graph, the User Sheaf, or the Pulse tracking config.

This blueprint defines the gap analysis, target architecture, and three-sprint roadmap to turn the existing static structures into a **continuously-learning, org-scoped, proactive intelligence layer**.

---

## 2. Current State (Inventory)

### 2.1 Data layer (Rust, SQLite via SQLx)

| Concern | Tables / Models | File | Notes |
|---|---|---|---|
| Project-scoped sheaf | `project_knowledge_sources` (+ view `v_project_knowledge_completeness`) | `crates/db/src/models/project_knowledge_source.rs` | Polymorphic `owner_type` (`project|organization|company|user|deal`) + `owner_id`. `coverage_score` ∈ [0,1]. Six `source_type` enum values. `auto_registered`, `client_visible`, `is_stale`, `last_refreshed_at`. Completeness = `0.4 × (type_count/6) + 0.6 × avg(coverage)`. |
| User-private sheaf | `user_knowledge_sources` | `crates/db/src/models/user_knowledge_source.rs` | Eight `source_type` values incl. `skill`, `interaction`, `note`, `bookmark`, `research`. Cross-refs into `companies`, `persons`, `projects`. |
| Entity graph | `entity_graph_nodes`, `entity_graph_edges` | `crates/db/src/models/entity_graph.rs` | 11 node types, 10 edge types. `BFS subgraph_focused` (depth ≤ 5), `org_subgraph` (5-query + in-mem filter), `global_subgraph`, `sync_global_graph` (rebuild). |
| Entity catalogue | `entities`, `entity_aliases`, `entity_appearances` | `crates/db/src/models/entity.rs`, `entity_appearance.rs` | Speakers/sponsors/venues/etc. `data_completeness` field. Alias table supports `nickname|maiden_name|stage_name|former_name|abbreviation|typo`. |
| Pulse mirror | `pulse_sources`, `pulse_content_items`, `pulse_alert_rules`, `pulse_alerts`, `pulse_collection_runs`, `pulse_tracking_configs` | `crates/db/src/models/pulse_*.rs` | 7-state `pcg_status` lifecycle (`new → enriched → alerted → reviewed → tasked → actioned → dismissed`). Per-item `linked_task_id`, `linked_crm_contact_id`, `linked_entity_id`, `enrichment_vibe_cost`. |
| Auto-registration | SQL triggers in migrations `20260221100000`, `20260225000000`, `20260306100000`, `20260313200003`, `20260323100000` | `crates/db/migrations/*knowledge*.sql` | Five triggers: `conversation` (0.3), `pulse_content` (0.4), `artifact` (0.3–0.7 by type), `context_injection` (0.4–0.8 by type), `entity_appearance` (0.4–0.7 by appearance_type), `topology_snapshot` (0.5). Plus an UPDATE trigger that **bumps** coverage on `entity_appearance.status` transitions (`researched → 0.7`, `verified → 0.85`, `content_created → 1.0`). |

### 2.2 Service layer

There is **no dedicated `crates/services` module for KG, entity resolution, or pulse enrichment.** Knowledge-graph mutations live in:

- `crates/server/src/routes/knowledge.rs` — CRUD + visibility flag.
- `crates/server/src/routes/organizations/knowledge.rs` — `GET /api/organizations/:id/knowledge` aggregates org-scoped Sheaf rows + `data_sources` + brand profile.
- `crates/server/src/routes/intelligence.rs` — the heaviest writer. After Scout/Astra research it calls `ProjectKnowledgeSource::upsert_source` / `upsert_scoped` for project, org, and company scopes. Phase-1 → Phase-2 hand-off (`after_phase1_complete`) auto-creates the next task either via human review or trusted-pipeline auto-spawn.
- `crates/server/src/routes/graph.rs` — read-only subgraph endpoints + `POST /api/graph/sync/global` (admin).
- `crates/server/src/mcp/task_server/knowledge.rs` — MCP tools `add_knowledge`, `list_knowledge`, `get_knowledge_completeness`, `manage_knowledge`. This is the **agent write-back hatch**.
- `crates/server/src/pulse_consumer.rs` — NATS subscriber for `pulse.content.>` and `pulse.status.>`. Materialises content into `pulse_content_items` and dispatches Twilio SMS for matching alert rules.

### 2.3 Background workers

Inventory in `crates/server/src/workers/background_tasks.rs`:

- `VibeDepositWatcher` (30 s)
- `VibeWithdrawalExecutor` (60 s)
- `ApnPeerCleanup`
- `MeetingSessionCleanup`
- `NoraInboxPoller`
- `EmailSyncWorker`

**None of them touch the KG or Pulse data.** There is no staleness sweep, no coverage-decay job, no graph re-materialisation worker, no embedding/index builder, no orphan-entity resolver.

### 2.4 Frontend surface

`frontend/src/pages/organization-profile/tabs/intelligence/` is the only place that visualises this data:

- `KnowledgeTab.tsx` — aggregates `organizationsApi.getKnowledge(orgId)` and `knowledgeApi.getProjectKnowledge(projectId)` for every project. Renders avg completeness, total/stale counters, badges per source-type per project. Sub-views: `datasources`, `artifacts`, `workflows`, `pulse`, `topology`, conversation list.
- `TopologyView.tsx` — graph viz (consumes `/api/graph/...`).
- `PulseSection.tsx`, `pulse.tsx` — pulse content + alerts + tracking config UIs.

The UI is read-mostly: source rows can be marked stale/refreshed, but there is **no inline "ask the KG" or proactive-insight panel**. Org-scoped continuous-learning signals are not surfaced.

---

## 3. Gap Analysis

| Capability | State | Gap |
|---|---|---|
| Polymorphic ownership (`owner_type`/`owner_id`) | Migration 20260305200000 + later patches add the columns; `upsert_owner_scoped` uses them | UI does not yet expose **owner-scoped** roll-ups (org KG ≠ sum of project KGs). |
| Coverage scoring | Static — set on insert, never decays, never recomputed | No half-life, no recency weighting, no "fresh evidence" bump beyond the entity_appearance trigger. |
| Staleness detection | `is_stale` is **only** set by manual `POST /api/projects/:project_id/knowledge/:source_id/stale` or by user UI | No periodic sweep; no rule like "RSS items >30 d old → stale". |
| Auto-registration triggers | Cover 6 source types | No trigger for `crm_deals`, `proposals`, `social_posts`, `media_artifacts` (these flow through `sync_*_graph` instead, leaving the Sheaf blind to them). |
| Entity resolution | `entity_aliases` table exists, `entities.canonical_name`/`slug` populated by research | No matching service: when Scout finds "Jane Smith @ Acme" and "J. Smith" already exists in `crm_contacts`, there is **no dedup pass**. `extracted_entities` JSON on pulse items is never linked back to `entities`. |
| Entity graph freshness | `sync_global_graph` is admin-only, manual | No worker. Mutations to source tables do *not* trigger node/edge upserts (except where explicit calls exist in `intelligence.rs` and a handful of CRM handlers). The graph drifts. |
| Pulse → KG closure | Triggers register the *content item* | Entities extracted from pulse content are **not** promoted into `entity_graph` or `entities`. The `linked_entity_id` column is essentially unused. |
| Pulse → Entity Graph signal | None | A pulse item mentioning a tracked competitor should add/refresh a `mentioned_in` edge. Today it does not. |
| Org-scoped continuous learning | Org-scoped Sheaf rows exist; UI shows them as badges | No mechanism for the org-level KG to *evolve*: nothing aggregates project insights into org-level summaries; nothing promotes a recurring finding across N projects into an org-level fact. |
| Agent write-back | `MCP::add_knowledge` exists | Restricted to `KnowledgeSourceType::*`. **No MCP tool for adding entity-graph nodes/edges, user-knowledge entries, or pulse-tracking-config updates.** Agents cannot teach the system who matters. |
| Proactive intelligence | Manual: alert rules fire → tasks created | No "insights" pipeline. The KG never says "Acme Corp went silent for 14 days — flag it" or "your last 3 wins shared this attribute". The completeness gauge is the closest thing to a suggestion. |
| Vector search / semantic retrieval | None | No embedding column, no `sqlite-vss`, no external vector DB. Sheaf is keyword/title only. |
| Audit trail | `updated_at` only | No append-only `kg_events` log → impossible to answer "when did we first learn X". |

---

## 4. Subsystem deep-dives

### 4.1 Entity Resolver — current vs target

**Current path (`intelligence.rs::run_research_direct`):**

```
contact_id → Scout LLM prompt → free-text or JSON response
            → write to crm_contacts.intelligence_*
            → ProjectKnowledgeSource::upsert_source(KnowledgeSourceType::Entity, contact_id)
```

Identity is fixed by `contact_id`. New entities encountered in the prose are **never extracted**.

**Gaps:**
1. No NER on pulse content — `extracted_entities` JSON is set by the external Pulse Engine but never parsed in PCG.
2. No alias matching when ingesting (e.g. "Aaren K." vs "Aaren Kandola").
3. No company-disambiguation step (multiple "Bloom Studio" hits → which one is *ours*?).
4. `entity_aliases.alias_type` enum exists but no code populates it.

**Target:**
- A `services::services::entity_resolver` module with one purpose: take `(entity_kind, candidate_payload)` → return `(entity_id, confidence, action: matched|created|needs_review)`. Pure function over `(SqlitePool, candidate)`.
- Match precedence: `external_ids` exact → `slug` → fuzzy `canonical_name` (trigram or Levenshtein via SQL extension) → alias table.
- Below-threshold matches enqueue a `human_review` row.

### 4.2 Pulse Engine — current vs target

**Current path:**

```
External Pulse Engine ── NATS pulse.content.* ──▶ pulse_consumer.rs
        │                                            │
        │                                            ▼
        │                                   pulse_content_items
        ▼                                            │
   pulse_alert_rules (mirrored, eval'd            (INSERT trigger)
       inside external engine)                       ▼
                                          project_knowledge_sources
                                          (source_type='pulse_content',
                                           coverage_score=0.4)
```

Alert evaluation lives **in the external engine**. PCG only stores the result. The KG never *uses* the pulse content for anything beyond visualisation.

**Gaps:**
1. No PCG-side enrichment: no entity extraction, no link to `entity_graph_nodes`.
2. `extracted_entities` JSON arrives but is opaque to the rest of the system.
3. `relevance_score` is computed by Pulse but never decays — a "high-relevance" item from 6 months ago still scores the same.
4. No feedback loop: dismissing a pulse item teaches nothing to Pulse's collection logic.
5. Tracking config is per-project; no org-level inheritance.

**Target:**
- A `pulse_enricher` worker that consumes new `pulse_content_items` (or NATS), runs entity resolver + LLM extraction, populates `linked_entity_id`, and emits an `entity_graph_edge` of type `mentioned_in` from entity → pulse_content node.
- Org-level tracking config inherited by projects unless overridden.
- A feedback channel: every `dismiss` / `actioned` writes back a NATS event `pulse.feedback.*` that the external engine can use to retrain.

### 4.3 Org-Scoped Continuous Learning

**What exists:**
- `KnowledgeOwnerScope::Organization` rows in `project_knowledge_sources` (queryable via `find_by_owner`).
- `data_sources` table (org-scoped).
- `organization_brand_profiles` (research summary + research_status).
- `KnowledgeTab` shows org entries as a flat list of badges.

**What is missing:**
- Aggregation: no SQL view or worker that rolls `project_knowledge_sources` into org-level facts.
- Trend extraction: no "topic of the week" computed from new pulse items + new conversations.
- Cross-project entity unification: same person in two projects = two rows; no canonicalisation.
- Decay: org KG is permanent; nothing ages out a stale brand insight.

**Target:**
- A `services::kg_aggregator` running daily that:
  - Recomputes org-level coverage as a weighted blend of project sheaves.
  - Materialises "org-fact" rows: e.g. *"3 of 5 active deals mention budget concerns"*.
  - Decays `coverage_score` of org-scoped rows last refreshed > 90 d by 20%, marks stale at <0.2.
- A new `organization_knowledge_summaries` table: time-bucketed snapshots so the UI can show "this is what we knew on 2026-04-01 vs today".

### 4.4 Agent Write-Back

**Existing surfaces:**
| Surface | Who can call | What it writes | Where |
|---|---|---|---|
| MCP `add_knowledge` | Any MCP-connected agent | `project_knowledge_sources` row | `mcp/task_server/knowledge.rs` |
| MCP `manage_knowledge` | Same | stale/refresh/delete | same |
| `intelligence.rs::run_research_direct` | Server-side (Scout) | `crm_contacts.intelligence_*`, project/org/company KG rows | route handler |
| `intelligence.rs::run_company_research_direct` | Server-side (Scout) | `companies.intelligence_*`, `company_contact_methods`, project/company KG | route handler |
| SQL triggers | Implicit | Sheaf rows | migrations |

**Gaps:**
1. No MCP tool to create/update **`entity_graph_nodes`**. Agents cannot say "I discovered Acme Corp owns Beta Inc" → graph remains blind.
2. No MCP tool for **`user_knowledge_sources`**. Personal user memory is not agent-writable.
3. No MCP tool to **resolve an entity** (call the future resolver and return `(entity_id, action)`).
4. No write-back hook from Topsi (topology agent) → `topology_snapshots`. The auto-registration trigger exists but no agent fires it.
5. No structured audit: who wrote what, when, with what confidence?

**Target:**
- A `kg_writeback` service exposing typed primitives:
  - `add_entity_graph_node(kind, ref_id, label, metadata, edges_in, edges_out)`
  - `resolve_entity(kind, payload) → EntityResolution`
  - `add_user_knowledge(user_id, source_type, payload)`
  - `add_org_fact(org_id, summary, evidence_refs)` (writes to `organization_knowledge_summaries`)
- All primitives exposed via MCP **and** as Rust service methods so internal handlers can use them too.
- Append-only `kg_events` table: `(id, actor_kind, actor_id, action, target_table, target_id, payload_json, created_at)`.

### 4.5 Proactive Intelligence

**What exists:**
- `pulse_alert_rules` → triggered alerts → optional auto-task creation.
- `after_phase1_complete` auto-spawns Phase-2 task for trusted senders.
- `KnowledgeTab` shows completeness % and stale counts — but as numbers, not actions.

**What is missing:**
- No "insight" entity (something that says "X happened, here is why it matters, here is the recommended action").
- No surfacing of insights in the UI beyond raw alerts and tasks.
- No cross-source synthesis: alert-rules only see one pulse item at a time, never two.
- No org-level digest: there's no place where Nora can say "Here's what changed this week."

**Target:**
- A new `insights` table: `(id, org_id, project_id, kind, title, body, evidence_refs, priority, status, created_at)`.
- An `insight_generator` job (LLM-backed, runs hourly or on signal) that scans recent activity (new sheaf rows, new pulse content, status changes) and writes `insights`.
- New UI panel: "Proactive Intelligence" on KnowledgeTab + on each project dashboard.
- Each insight can be dismissed, snoozed, or promoted to a task — feeding the feedback loop.

---

## 5. Architecture diagrams

### 5.1 As-is

```mermaid
flowchart LR
    subgraph External
        PE[Pulse Engine\n(Rust, NATS)]
    end

    subgraph Backend[pcg-cc-mcp backend]
        PC[pulse_consumer.rs\n(NATS subscriber)]
        Routes[REST routes\n/api/knowledge\n/api/graph\n/api/pulse\n/api/intelligence]
        MCP[MCP server\n(add_knowledge, ...)]
        Workers[Background workers\nVibe / Email / Nora-inbox]
    end

    subgraph DB[SQLite]
        PKS[(project_knowledge_sources)]
        UKS[(user_knowledge_sources)]
        EGN[(entity_graph_nodes / _edges)]
        E[(entities / entity_aliases)]
        PCI[(pulse_content_items)]
        PAR[(pulse_alert_rules)]
        PA[(pulse_alerts)]
    end

    subgraph Agents
        Nora
        Scout
        Astra
        Auri
    end

    PE -- pulse.content.* --> PC --> PCI
    PE -.->|HTTP proxy| Routes
    PCI -- INSERT trigger --> PKS

    Scout -- intelligence.rs --> PKS
    Scout -- intelligence.rs --> EGN
    Nora -- delegates --> Scout
    Astra -- writes artifacts --> PKS
    Auri -- MCP add_knowledge --> MCP --> PKS

    Routes -- read --> PKS
    Routes -- read --> EGN
    Routes -- read --> PCI

    Workers -. nothing for KG/Pulse .-> DB

    FE[KnowledgeTab\n(React)] --> Routes
```

Static. INSERT-driven. Read-mostly UI.

### 5.2 To-be

```mermaid
flowchart LR
    subgraph External
        PE[Pulse Engine]
    end

    subgraph NewWorkers[New PCG workers]
        PE_W[pulse_enricher\n(extract + resolve)]
        KG_A[kg_aggregator\n(daily roll-ups)]
        Stale[staleness_sweeper]
        Insight[insight_generator]
        GraphSync[graph_resync\n(incremental)]
    end

    subgraph Services[New services modules]
        ER[entity_resolver]
        WB[kg_writeback]
        IS[insight_service]
    end

    subgraph Backend[Existing backend]
        Routes
        MCP_E[MCP: existing\nadd_knowledge]
        MCP_N[MCP: new\nresolve_entity\nadd_entity_node\nadd_user_knowledge\nadd_org_fact]
        PC[pulse_consumer.rs]
    end

    subgraph DB[SQLite]
        PKS[(project_knowledge_sources)]
        UKS[(user_knowledge_sources)]
        EGN[(entity_graph_nodes)]
        EGE[(entity_graph_edges)]
        E[(entities)]
        EA[(entity_aliases)]
        PCI[(pulse_content_items)]
        OKS[(organization_knowledge_summaries\nNEW)]
        INS[(insights NEW)]
        EVT[(kg_events NEW)]
    end

    subgraph Agents
        Nora
        Scout
        Astra
        Auri
        Topsi
    end

    PE --> PC --> PCI
    PCI -->|enqueue| PE_W
    PE_W --> ER
    ER --> E
    ER --> EA
    PE_W --> EGN
    PE_W --> EGE

    KG_A --> OKS
    KG_A --> PKS
    Stale --> PKS

    Insight --> IS --> INS
    INS --> Routes

    Nora & Scout & Astra & Auri & Topsi -->|via MCP_N| WB
    WB --> PKS
    WB --> UKS
    WB --> EGN
    WB --> EVT

    GraphSync --> EGN
    GraphSync --> EGE

    Routes --> PKS
    Routes --> EGN
    Routes --> INS
    Routes --> OKS

    FE[KnowledgeTab + Insights panel] --> Routes
```

Closed-loop. Continuous. Agent-writable beyond the Sheaf.

---

## 6. 3-Sprint Roadmap

Each sprint ≈ 2 calendar weeks. Acceptance criteria are testable.

### Sprint 1 — Foundations: resolver, write-back surface, audit log

**Goal:** make the system *writable* by agents beyond the Sheaf, and *resolvable* on entity ingestion.

**Workstreams:**

1. **Entity Resolver service** (`crates/services/src/services/entity_resolver.rs`)
   - Pure function `resolve(pool, ResolutionRequest) → ResolutionResult`.
   - Match precedence: `external_ids` → `slug` → trigram on `canonical_name` (use `sqlite-spellfix1` or in-Rust `strsim`) → `entity_aliases`.
   - Below `confidence=0.7` → write to a new `entity_resolution_reviews` table for human approval.
2. **`kg_events` audit table** (migration).
   - `(id, actor_kind, actor_id, action, target_table, target_id, payload_json, created_at)`.
   - Wrap every Sheaf/Graph mutation in `kg_writeback` to append a row.
3. **New MCP tools** (`mcp/task_server/`):
   - `resolve_entity(kind, payload)` → returns `(entity_id, action, confidence)`.
   - `add_entity_graph_node(kind, ref_id, label, metadata)` + `add_entity_graph_edge(from_kind, from_id, to_kind, to_id, edge_type, weight)`.
   - `add_user_knowledge(user_id, source_type, payload)`.
4. **`kg_writeback` service** — single funnel through which all the above (and existing handlers in `intelligence.rs`) write. Emits `kg_events`.

**Acceptance criteria:**
- [ ] `services::entity_resolver::resolve` returns `matched` for a known canonical name with 2 character difference (alias-table hit).
- [ ] `services::entity_resolver::resolve` writes a `entity_resolution_reviews` row for `confidence < 0.7`.
- [ ] Every successful `ProjectKnowledgeSource::upsert_*` call produces a `kg_events` row (verified by SQL `COUNT` test).
- [ ] MCP smoke test: Auri can call `resolve_entity` and `add_entity_graph_node` and the new edges appear in `GET /api/graph/global`.
- [ ] `cargo test -p services` + `cargo test -p server` pass; `cargo clippy --all -- -D warnings` clean.

### Sprint 2 — Continuous learning: workers, decay, pulse enrichment

**Goal:** make the KG *evolve* without manual button-presses.

**Workstreams:**

1. **`staleness_sweeper` worker** (`crates/server/src/workers/`)
   - Daily. Marks `is_stale = 1` on Sheaf rows whose `last_refreshed_at` is older than: 30 d for `pulse_content`/`conversation`, 90 d for `artifact`/`entity`/`topology_snapshot`, 7 d for `context_injection`.
   - Decays `coverage_score` by 20% on rows last-refreshed > 90 d.
2. **`graph_resync` worker** (hourly, incremental)
   - Pulls rows changed since last run via `updated_at` cursor on `crm_deals`, `proposals`, `companies`, `clients`, `projects`, `project_brand_profiles`, `crm_pipelines`, `crm_pipeline_stages`.
   - Reuses existing `sync_*_graph` helpers but driven by change-data-capture rather than full rebuild.
3. **`pulse_enricher` worker**
   - Consumes new `pulse_content_items` (DB poll or NATS).
   - LLM call to extract `{people, companies, topics}`.
   - For each candidate → `entity_resolver::resolve` → on match, set `linked_entity_id`, create `entity_graph_edge` of type `mentioned_in` from entity → content node (new node_type: `pulse_item`).
   - Decay `relevance_score` on read using `score * exp(-Δdays / 30)`.
4. **`kg_aggregator` worker** (daily)
   - Materialises `organization_knowledge_summaries` (NEW table): rolling 7d / 30d roll-ups of new sheaf rows, top entities, top topics.
   - Recomputes org-level coverage as weighted avg of project completeness × recency.
5. **Migration**: add `pulse_item` to `entity_graph_nodes.node_type` CHECK and `mentioned_in` to `entity_graph_edges.edge_type`. Add `organization_knowledge_summaries` table.

**Acceptance criteria:**
- [ ] After running migrations and starting workers locally, a freshly inserted `pulse_content_items` row gets `linked_entity_id` populated within 60 s when its body contains a known entity name.
- [ ] `organization_knowledge_summaries` row count > 0 for an org with > 5 sheaf entries after `kg_aggregator` runs once (test fixture).
- [ ] A sheaf row with `last_refreshed_at < now - 91 days` gets `coverage_score` reduced and `is_stale = 1` after sweeper.
- [ ] `graph_resync` picks up a new `crm_deals` row within one cycle and creates the corresponding `entity_graph_node` + `has_deal` edge.
- [ ] All workers register with `ShutdownRegistry` and respond to `CancellationToken` within 5 s (timeout test).

### Sprint 3 — Proactive intelligence + org-scoped UX

**Goal:** the KG starts *telling* users things they didn't ask.

**Workstreams:**

1. **`insights` table + `insight_service`** (`crates/services/src/services/insight_service.rs`)
   - Schema: `(id, org_id, project_id NULL, kind, title, body, evidence_refs JSON, priority, status, created_by_kind, created_at, dismissed_at NULL)`.
   - `kind` ∈ `{trend, anomaly, opportunity, risk, summary}`.
2. **`insight_generator` worker** (hourly, gated by `INSIGHT_GENERATION_ENABLED=1`)
   - Pulls the last N hours of `kg_events`, new pulse items, status changes on deals/contacts.
   - LLM call summarises → emits `insights` rows.
   - Uses `kg_writeback` (so it audits like everything else).
3. **REST endpoints**: `GET /api/organizations/:id/insights`, `POST /api/insights/:id/dismiss`, `POST /api/insights/:id/promote-to-task`.
4. **Frontend**:
   - New `InsightsPanel.tsx` component on `KnowledgeTab` overview.
   - Card list with priority badge, evidence quick-links, dismiss / snooze / promote-to-task actions.
   - Org-scoped digest tile: "What changed this week".
5. **Pulse feedback loop**: when an insight or pulse item is dismissed, publish NATS `pulse.feedback.{org_id}` so the external engine can adjust source weights.

**Acceptance criteria:**
- [ ] Seeded fixture (10 pulse items + 3 deal stage changes + 2 new contacts) produces ≥ 1 `insights` row of each `kind` after `insight_generator` runs.
- [ ] Dismissing an insight in the UI removes it from the list within the same session and writes a `kg_events` row with `action='insight_dismissed'`.
- [ ] Promoting an insight to a task creates a `tasks` row with `created_by='insight_engine'` and `evidence_refs` reproduced in the task description.
- [ ] Playwright e2e: org page → KnowledgeTab → see "Proactive Intelligence" panel with ≥ 1 card from seed; click promote → task appears in `tasks` page.
- [ ] No regression: existing KnowledgeTab badges, pulse SSE stream, and graph endpoints still work (e2e regression suite green).

---

## 7. Open Questions (decisions needed before/during Sprint 1)

1. **Vector search?** Sheaf is title+summary only. Do we add an `embedding BLOB` column + `sqlite-vss` (in-process) or punt to an external vector DB (Qdrant)? Sprint-1 decision needed because resolver fuzzy-match fallback might want embeddings.
2. **Coverage decay curve.** Linear 20% / 90 d (this draft) vs exponential `coverage * exp(-Δd/τ)`. Empirical pick — needs sample data.
3. **Org-scope ownership of pulse sources.** Today `pulse_sources` is project-scoped with optional `organization_id`. Should org-level sources cascade to all projects? Or stay project-only?
4. **Entity resolver confidence threshold.** 0.7 used in this plan, but `entity_aliases` exact match is 1.0 and slug is 0.95 — we need explicit scoring. Calibrate on a 50-row labelled set first.
5. **MCP tool authorisation.** `add_entity_graph_node` is more powerful than `add_knowledge` because it mutates the topology. Should we gate it on `actor_kind in (auri, nora, scout, astra, topsi)` server-side?
6. **`kg_events` retention.** Forever? Or rolling 365 d with quarterly archival? Affects table size estimates.
7. **Insight generation cost.** Hourly LLM calls × N orgs scales. Do we batch by org, by activity volume, or gate on a "noise floor" (≥ K events since last run)?
8. **Topsi → topology_snapshot write-back.** The trigger exists but the agent integration does not. Does Topsi run server-side as a worker, or as an MCP-driven external agent?
9. **External Pulse Engine source-of-truth for alert rules.** Today rules live in the external engine *and* are mirrored to `pulse_alert_rules`. If PCG agents start editing rules via MCP, who is authoritative? Need a contract before Sprint 3 feedback loop.
10. **User-private KG access by agents.** Should an agent be able to read another user's `user_knowledge_sources`? Default answer: no, unless explicit `share_with_org` flag. Needs a column.

---

## 8. Appendix — Files touched per sprint (cheat sheet)

| Sprint | New files | Modified files | Migrations |
|---|---|---|---|
| 1 | `crates/services/src/services/entity_resolver.rs`, `kg_writeback.rs`; `crates/server/src/mcp/task_server/entity_graph.rs`, `user_knowledge.rs` | `mcp/task_server/mod.rs` (register tools); `routes/intelligence.rs` (route through `kg_writeback`) | `..._kg_events.sql`, `..._entity_resolution_reviews.sql` |
| 2 | `crates/server/src/workers/staleness_sweeper.rs`, `graph_resync.rs`, `pulse_enricher.rs`, `kg_aggregator.rs`; `crates/services/src/services/pulse_enrichment.rs` | `workers/mod.rs`, `lib.rs` (spawn) | `..._organization_knowledge_summaries.sql`, `..._entity_graph_node_types_extension.sql` |
| 3 | `crates/services/src/services/insight_service.rs`; `crates/server/src/workers/insight_generator.rs`; `crates/server/src/routes/insights.rs`; `frontend/src/pages/organization-profile/tabs/intelligence/InsightsPanel.tsx` | `KnowledgeTab.tsx`, `pulse_consumer.rs` (feedback publisher), router wiring | `..._insights.sql` |

After every Rust struct change with `#[derive(TS)]`: `npm run generate-types`.

---

*End of blueprint v1. Reviewers — see §7 for items needing a call before Sprint 1 kicks off.*

# Topos Categorical Architecture

> *Formalizing the PCG ecosystem through the lens of category theory.*

## Overview

The Topos ecosystem is a compositional, multi-service platform where services compose like morphisms in a category. This document formalizes the architectural principles using category theory, providing both a theoretical foundation and practical guidance for integration patterns.

---

## 1. Objects

Objects in our category are the primary domain entities:

| Object | Description |
|---|---|
| **User** | Authenticated human actor with role/permissions |
| **Organization** | Multi-tenant container scoping all resources |
| **Project** | Work container with git repo, budget, and task graph |
| **Task** | Unit of work with status lifecycle |
| **Agent** | AI actor (Nora, Maci, Editron, Astra, Scout, Auri, Topsi) |
| **ContentItem** | Collected content from Pulse Engine sources |
| **Source** | Content origin (RSS, Twitter, YouTube, Reddit, Web) |
| **AlertRule** | Declarative trigger conditions on content |
| **CrmContact** | External contact linked to notifications/CRM |
| **VibeTransaction** | VIBE token ledger entry for resource accounting |

---

## 2. Morphisms (Typed Relationships)

Morphisms are the typed arrows connecting objects:

```
User ──(member_of)──→ Organization ──(owns)──→ Project ──(has_source)──→ Source ──(produces)──→ ContentItem
                                                 │
                                                 ├──(has_task)──→ Task ──(assigned_to)──→ Agent
                                                 │
                                                 ├──(has_alert_rule)──→ AlertRule ──(triggers)──→ Alert ──(references)──→ ContentItem
                                                 │
                                                 └──(has_budget)──→ VibeTransaction
```

Every morphism is enforced at the database level via foreign keys. Deletion cascades follow the arrow direction: deleting a Project cascades to its Tasks, Sources, Content, and Alerts.

---

## 3. Composition (Associativity)

Morphisms compose associatively. A User's content lineage is the composite:

```
User →(member_of)→ Org →(owns)→ Project →(has_source)→ Source →(produces)→ ContentItem
```

This composition is **associative**: `(member_of ∘ owns) ∘ has_source = member_of ∘ (owns ∘ has_source)`.

In practice, this means we can query a user's content by joining through any intermediate point — the result is always consistent. CASCADE deletes enforce this compositional integrity: removing any node cleanly removes everything downstream.

---

## 4. Functors

Functors are structure-preserving maps between categories:

### F_pulse: Cat(Pulse) → Cat(PCG)

Maps Pulse Engine's internal domain to PCG's unified model:

| Pulse Object | PCG Object |
|---|---|
| `Source` | `pulse_sources` |
| `ContentItem` | `pulse_content_items` |
| `AlertRule` | `pulse_alert_rules` |
| `CollectionRun` | `pulse_collection_runs` |

This functor preserves relationships: if a Source produces a ContentItem in Pulse, the corresponding `pulse_sources` row produces the corresponding `pulse_content_items` row in PCG.

### F_nats: Cat(Events) → Cat(PCG)

Maps NATS subject hierarchy to PCG activity log entries:

| NATS Subject Pattern | PCG Action |
|---|---|
| `pulse.content.{project}.new` | Insert into `pulse_content_items` + activity log |
| `pulse.content.{project}.alert` | Insert into `pulse_alerts` + create Task |
| `pulse.status.{project}` | Update engine health status |
| `pcg.pulse.config.{project}.*` | Propagate config changes to Pulse Engine |

### F_chain: Cat(PCG) → Cat(Aptos)

Maps VIBE transactions to on-chain settlements:

| PCG Object | Aptos Object |
|---|---|
| `VibeTransaction` | On-chain transfer event |
| `Agent.wallet_address` | Aptos account |

---

## 5. Natural Transformations

Natural transformations are systematic transitions between functors — in our case, lifecycle state transitions:

### ContentItem Lifecycle

```
new → enriched → alerted → tasked → actioned
```

Each transition is a natural transformation between endofunctors on the content lifecycle category:

- **η_enrich**: `Id → F_llm` — LLM pipeline enriches raw content with summary + relevance score
- **η_alert**: `F_llm → F_alert` — Alert evaluator matches enriched content against rules
- **η_task**: `F_alert → F_task` — High-priority alerts promote to Tasks
- **η_action**: `F_task → F_action` — Human/agent reviews and acts on task

The naturality condition ensures these transitions commute with project scoping: enriching content for Project A and then scoping to Org X gives the same result as scoping first and then enriching.

---

## 6. Subobject Classifier (Permission System)

The subobject classifier `Ω` classifies access levels:

```
Ω = { Full, Execute, ReadOnly, Denied }
```

For any User and Resource, the characteristic morphism `χ: User × Resource → Ω` determines access:

- **Full**: CRUD + admin operations
- **Execute**: Can trigger actions (collect, create task) but not modify config
- **ReadOnly**: Can view content, sources, alerts
- **Denied**: No access

**Multi-tenancy as Pullback**: A user's visible projects form a pullback:

```
UserProjects = User ×_Org (Org ×_Project AllProjects)
```

This is computed at query time via the `organization_id` scoping on all tables.

---

## 7. NATS as Internal Language

The NATS subject hierarchy is the **typed arrow space** of our event category:

```
pulse.content.{project}.new     — content creation arrows
pulse.content.{project}.alert   — alert trigger arrows
pulse.status.{project}          — status observation arrows
pcg.pulse.config.{project}.*    — configuration mutation arrows
pcg.pulse.command.{project}.*   — imperative command arrows
```

**Subscribers are contravariant functors**: they map from events (source) to handlers (target), reversing the direction. The `pulse_consumer` in PCG is a contravariant functor from `Cat(NATS Events)` to `Cat(DB Operations)`.

**Publishers are covariant functors**: they map domain operations to event emissions, preserving direction.

---

## 8. Pullbacks (Content Scoping)

Content scoped to a user's organization is a pullback:

```
UserContent = User ×_Org (Org ×_Project ProjectContent)
```

In SQL:

```sql
SELECT ci.* FROM pulse_content_items ci
JOIN projects p ON ci.project_id = p.id
WHERE ci.organization_id IN (
    SELECT organization_id FROM user_memberships WHERE user_id = ?
)
```

This pullback ensures that content is always viewed through the lens of organizational membership — a user can never see content from projects outside their org.

---

## 9. Pushouts (Multi-Source Content Merge)

Content from multiple sources is merged into a unified feed via pushout (coproduct with identification):

```
SourceA_Content ⊔_{dedup} SourceB_Content = UnifiedFeed
```

The identification is by `content_hash` — if two sources produce the same content (same URL + title hash), they are identified as a single item. This is the universal property of the pushout: any other merging strategy factors through ours.

---

## 10. Practical Architecture Diagram

```
┌──────────────┐     NATS Events      ┌──────────────────┐
│ Pulse Engine │ ──────────────────→   │   PCG Dashboard   │
│   (Python)   │ ←────────────────── │    (Rust/React)    │
│              │     Config Events     │                    │
│  Sources ──→ │                       │ ┌──────────────┐  │
│  Adapters    │                       │ │ pulse_consumer│  │
│  LLM Pipeline│                       │ │ (NATS→DB)    │  │
│  Alerts      │                       │ └──────┬───────┘  │
└──────────────┘                       │        │          │
                                       │ ┌──────▼───────┐  │
                                       │ │ pulse_* tables│  │
                                       │ │ (SQLite)      │  │
                                       │ └──────┬───────┘  │
                                       │        │          │
                                       │ ┌──────▼───────┐  │
                                       │ │ routes/pulse  │  │
                                       │ │ (REST API)    │  │
                                       │ └──────┬───────┘  │
                                       │        │          │
                                       │ ┌──────▼───────┐  │
                                       │ │ MCP Tools     │  │
                                       │ │ (Agent access)│  │
                                       │ └──────┬───────┘  │
                                       │        │          │
                                       │ ┌──────▼───────┐  │
                                       │ │ Frontend      │  │
                                       │ │ (React)       │  │
                                       │ └──────────────┘  │
                                       └──────────────────┘
```

---

## 11. Integration Contracts

### NATS Subject Schema

| Subject | Direction | Payload |
|---|---|---|
| `pulse.content.{project}.new` | Pulse → PCG | `PulseContentEvent` |
| `pulse.content.{project}.alert` | Pulse → PCG | `PulseAlertEvent` |
| `pulse.status.{project}` | Pulse → PCG | `{ status, adapters, timestamp }` |
| `pcg.pulse.config.{project}.sources.updated` | PCG → Pulse | `{ sources: [...] }` |
| `pcg.pulse.config.{project}.tracking.updated` | PCG → Pulse | `{ keywords, entities }` |
| `pcg.pulse.config.{project}.alerts.updated` | PCG → Pulse | `{ rules: [...] }` |
| `pcg.pulse.command.{project}.collect` | PCG → Pulse | `{ source_id?: string }` |

### REST API Contract

All Pulse endpoints live under `/api/pulse/projects/{project_id}/`:

| Method | Path | Description |
|---|---|---|
| GET | `/sources` | List sources for project |
| POST | `/sources` | Create new source |
| PUT | `/sources/{id}` | Update source |
| DELETE | `/sources/{id}` | Delete source |
| GET | `/content` | Search content |
| GET | `/content/latest` | Latest content items |
| GET | `/content/{id}` | Single content item |
| POST | `/content/{id}/action` | Review/dismiss/create-task/link-crm |
| GET | `/alerts` | Active alerts |
| GET | `/alert-rules` | List alert rules |
| POST | `/alert-rules` | Create alert rule |
| PUT | `/alert-rules/{id}` | Update alert rule |
| DELETE | `/alert-rules/{id}` | Delete alert rule |
| POST | `/collect` | Trigger collection |
| GET | `/runs` | Collection run history |
| GET | `/tracking` | Tracking config |
| PUT | `/tracking` | Update tracking config |
| GET | `/engine/status` | Pulse Engine health |

---

## 12. Design Principles

1. **Composition over configuration**: Services compose through well-typed interfaces (NATS subjects, REST endpoints), not ad-hoc config files.

2. **Scoping via pullback**: Every query is scoped to `(user, organization, project)` — content is never globally visible without explicit pullback through membership.

3. **Lifecycle as natural transformation**: Status transitions are systematic and commute with scoping — the order of "scope then transition" vs "transition then scope" doesn't matter.

4. **Events as arrows**: NATS subjects are typed arrows in an event category. Adding a new integration means adding new arrows, not modifying existing ones.

5. **Functorial persistence**: The mapping from domain events to database writes preserves structure — related events produce related rows, and deletions cascade correctly.

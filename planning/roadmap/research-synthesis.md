# ORCHA Platform — 5-Year Product Roadmap Research Synthesis

**Date**: 2026-03-18
**Type**: Research Synthesis (Day 1 Deliverable)
**Sources**: 9 research papers from `planning/2026-03-18--research--*.md`
**Purpose**: Extract convergent patterns across all research, organized by theme, to inform the 5-year ORCHA product roadmap.

---

## Table of Contents

1. [Orchestration Patterns (What to Build)](#1-orchestration-patterns-what-to-build)
2. [Cost/ROI Data (What to Prioritize)](#2-costroi-data-what-to-prioritize)
3. [OSS Tools to Leverage (Buy vs Build)](#3-oss-tools-to-leverage-buy-vs-build)
4. [Architecture Principles (How to Build It)](#4-architecture-principles-how-to-build-it)
5. [Lessons Learned from Completed Sprints](#5-lessons-learned-from-completed-sprints)
6. [Architectural Constraints from Prior Decisions](#6-architectural-constraints-from-prior-decisions)
7. [References](#7-references)

---

## 1. Orchestration Patterns (What to Build)

### 1.1 Deterministic Envelope, Non-Deterministic Core

This is the single strongest convergent finding across the research corpus. Five independent sources arrive at the same structural insight: **wrap unpredictable LLM behavior in deterministic, typed control structures**.

- **RooRoo** defines structured output envelopes with typed status codes (`status`, `message`, `artifacts`, `clarification_needed`), ensuring every agent response conforms to a parseable contract regardless of the LLM's creative output. [Source: rooroo-agent-orchestration#Structured Output Envelopes]
- **FSM research** separates workflow control (deterministic finite state machine transitions) from the work performed within each state (non-deterministic LLM reasoning). The state machine guarantees progress and completion semantics; the LLM provides intelligence within bounded contexts. [Source: fsm-requirement-models#RFSM/DFSM Cascade]
- **SudoLang** expresses constraints as first-class invariants rather than procedural steps. The constraint system is deterministic ("never exceed token budget", "always cite sources"), while reasoning within those constraints remains flexible. [Source: sudolang-aidd-framework#Constraint System]
- **Spec Kit** enforces a deterministic contract layer (spec and plan files) while explicitly encouraging implementation creativity. The spec defines *what*; the plan defines *how*; neither constrains the agent's approach to individual coding tasks. [Source: speckit-spec-driven-development#What/How Separation]
- **Stately Agent** (from FSM research) demonstrates this pattern in production: XState statecharts define the deterministic envelope, LLM tool calls provide the non-deterministic core, and the statechart guarantees that every LLM decision maps to a valid transition. [Source: fsm-requirement-models#Stately Agent Pattern]

**Implication for ORCHA**: The workflow builder already provides a deterministic envelope (node graph with typed connections). The roadmap should formalize this by adding typed output envelopes to every agent node and enforcing state machine semantics on workflow transitions.

### 1.2 Hybrid Reactive/Deliberative Architecture

Multi-agent research identifies a two-layer architecture as "the most practical template for agentic AI" [Source: multi-agent-system-design#Hybrid Architecture]:

- **Fast reactive layer**: health checks, timeout enforcement, circuit breakers, heartbeat monitoring. Operates on millisecond timescales. No LLM calls.
- **Slow deliberative layer**: task planning, agent assignment, strategy revision, goal decomposition. Operates on second-to-minute timescales. LLM-powered.

This maps directly to the BDI (Belief-Desire-Intention) model [Source: multi-agent-system-design#BDI Model]:
- **Beliefs**: world state and memory (what the system knows)
- **Desires**: goals and objectives (what the system wants)
- **Intentions**: adopted plans with commitment strategies (what the system is doing)

Commitment strategies prevent "thrashing" — the pathological pattern where an agent constantly re-plans without making progress. The research recommends **bold commitment** (stick with a plan unless it becomes impossible) over **cautious commitment** (re-evaluate on every new observation). [Source: multi-agent-system-design#Commitment Strategies]

### 1.3 Hierarchical Task DAG (HTDAG)

The Deep Agent pattern from multi-agent research introduces recursive subtask decomposition with two key innovations [Source: multi-agent-system-design#HTDAG]:

- **Atomic vs non-atomic distinction**: atomic tasks execute directly; non-atomic tasks decompose further. This is decided at runtime, not design time.
- **Lazy generation**: subtasks are only generated when their parent becomes active, reducing upfront planning overhead and enabling adaptation.
- **Error containment**: failures in a subtask do not automatically propagate to the parent. The parent can retry, substitute, or absorb the failure.

ATLAS extends this with a three-tier model (Projects -> Tasks -> Knowledge layer) backed by a graph data model that enables dependency analysis and critical-path computation. [Source: atlas-task-management-mcp#Three-Tier Model]

### 1.4 Contract Net Protocol

Dynamic task allocation via agent bidding, where [Source: multi-agent-system-design#Contract Net]:

1. A manager broadcasts a task announcement
2. Available agents evaluate the task against their capabilities and current load
3. Agents submit bids (capability match, estimated cost, estimated time)
4. The manager selects the winning bid
5. The winning agent commits to the task

This is particularly relevant for ORCHA's multi-agent pipeline where different agents have different model tiers and specializations. Combined with cost-tiered routing, the Contract Net becomes an economic optimization mechanism.

### 1.5 Error Handling and Supervision

#### 5-Level Error Hierarchy

Research converges on a graduated error handling model rather than binary succeed/fail [Source: multi-agent-system-design#Error Handling]:

| Level | Strategy | When |
|-------|----------|------|
| 1 | **Retry** | Transient failures (API timeout, rate limit) |
| 2 | **Fallback** | Use alternative approach or model |
| 3 | **Reassign** | Hand task to different agent |
| 4 | **Circuit break** | Stop attempting, protect system resources |
| 5 | **Human escalate** | Require human decision |

#### OTP Supervision Trees

Borrowed from Erlang/OTP, three restart strategies for agent groups [Source: multi-agent-system-design#OTP Supervision]:

- **One-for-one**: only the failed agent restarts (independent tasks)
- **One-for-all**: all agents in group restart (tightly coupled pipeline)
- **Rest-for-one**: failed agent and all agents started after it restart (ordered dependencies)

### 1.6 Memory Architecture

A five-tier memory taxonomy emerges across multiple papers [Source: multi-agent-system-design#Memory Taxonomy] [Source: atlas-task-management-mcp#Wisdom Accumulation]:

| Type | Scope | Lifecycle | Example |
|------|-------|-----------|---------|
| **Working** | Single task | Ephemeral | Current file contents, tool outputs |
| **Episodic** | Task history | Session | "Last time I tried X, it failed because Y" |
| **Semantic** | Domain facts | Durable | "This codebase uses Axum, not Actix" |
| **Procedural** | Learned workflows | Durable | "To deploy, run X then Y then Z" |
| **Shared** | Cross-agent | Coordination | Blackboard state, task assignments |

ATLAS adds a **wisdom accumulation system** — persistent learnings extracted from completed tasks that inform future task execution. This maps to the semantic/procedural tiers with explicit extraction triggers. [Source: atlas-task-management-mcp#Wisdom Accumulation]

### 1.7 FSM-Based Workflow Patterns

The FSM research paper provides the most detailed workflow engineering patterns [Source: fsm-requirement-models]:

- **RFSM -> DFSM cascade**: requirements-level FSM (what states exist) cascades to design-level FSM (how transitions are implemented). This two-phase approach catches completeness gaps early.
- **Event-State Analysis (ESA) matrix**: a completeness check where every event is evaluated against every state. Empty cells represent either impossible combinations (documented as such) or missing requirements.
- **MetaAgent auto-construction**: LLMs can generate FSMs from natural language specs with an 85% checkpoint passage rate, but human review of the generated state machine is essential. [Source: fsm-requirement-models#MetaAgent]
- **Traceback transitions**: return to a prior state *with accumulated context*, not a cold restart. This preserves work already done when recovering from errors.
- **Parallel state regions**: independent concerns (e.g., data fetching and UI rendering) run in parallel regions with `onDone` synchronization barriers.
- **Artifact state machines**: documents and deliverables have their own lifecycle (`draft -> review -> approved -> published`) independent of the task state machine.
- **Layered validation**: run deterministic checks (type validation, schema conformance, boundary checks) *before* expensive LLM review. This saves 60-80% of review costs on inputs that would fail basic validation.

### 1.8 Spec-Driven Development Pipeline

Spec Kit defines a five-phase development lifecycle [Source: speckit-spec-driven-development#Five-Phase Pipeline]:

1. **Constitution**: project-level invariants and values (CLAUDE.md, Vision.md)
2. **Specify**: formal specification of what to build (the *what*)
3. **Plan**: implementation strategy and file-level assignments (the *how*)
4. **Tasks**: decomposed, assignable work units with completion criteria
5. **Implement**: execution with spec as contract

Key findings:
- **File-path-based parallel boundary detection**: if two tasks touch different files, they can safely execute in parallel. If they share files, they need sequencing or merge resolution.
- **Optimal team size**: 2-4 agents working on ~5-6 tasks each. Beyond this, coordination overhead dominates. [Source: speckit-spec-driven-development#Team Sizing]
- **Self-organizing task claiming**: agents evaluate available tasks against their capabilities and claim work autonomously, similar to Contract Net but simplified.

### 1.9 SudoLang Constraint and Context Patterns

SudoLang introduces several patterns for efficient agent configuration [Source: sudolang-aidd-framework]:

- **Constraints as first-class citizens**: express invariants ("never modify files outside src/"), not step-by-step procedures. LLMs reason better with declarative constraints than imperative instructions.
- **PICS framework** (Preamble -> Interface -> Components -> Start): a structured prompt architecture that separates identity, capabilities, behavioral rules, and initialization.
- **Behavioral modifiers**: per-task customization of agent behavior (e.g., `verbosity=low`, `risk_tolerance=conservative`) without rewriting the full system prompt.
- **Vision.md as authoritative source**: a single document that prevents agent drift across long sessions by serving as the canonical reference for project goals and constraints.
- **Progressive discovery**: load context lazily based on what the agent actually needs. Research shows 20-30% token savings compared to loading everything upfront. [Source: sudolang-aidd-framework#Progressive Discovery]

### 1.10 ATLAS Task Management

ATLAS provides the most detailed task delegation framework [Source: atlas-task-management-mcp]:

- **6-section delegation template**: TASK, EXPECTED OUTCOME, REQUIRED TOOLS, MUST DO, MUST NOT DO, CONTEXT. This structure reduces ambiguity and prevents agents from inventing scope.
- **Completion criteria as first-class fields**: every task has explicit, verifiable criteria for "done" — not just "implement feature X" but "feature X passes tests Y and Z, handles error case W, and updates documentation D."
- **Graph data model (Neo4j)**: tasks, projects, and knowledge nodes are connected by typed edges (depends_on, blocks, relates_to). This enables automated dependency analysis, critical path computation, and impact assessment.
- **Continuation safety gates**: before resuming a paused task, verify that assumptions still hold (no conflicting changes, dependencies still met, context still valid).

### 1.11 CRM/PM MCP Integration Patterns

The CRM/PM MCP research provides patterns specific to ORCHA's domain [Source: crm-pm-mcp-servers]:

- **Ontological reduction**: pre-digest raw CRM/PM data into business-meaningful states before presenting to agents. A contact with 50 fields becomes "warm lead, last contacted 3 days ago, budget confirmed." This achieves 10-100x token reduction while improving agent reasoning quality.
- **Dual clock architecture**: maintain both state snapshots (current values) and event logs (what changed when) for temporal reasoning. Agents need both "what is the deal worth?" and "when did the value change?"
- **Multi-agent collision prevention**: three mechanisms work together:
  - **Policy engine**: declarative rules about who can modify what
  - **Decision ledger**: append-only log of all agent decisions for audit and conflict detection
  - **Trust gates**: operations above a confidence threshold proceed automatically; below it, they queue for human review
- **Discoverable workflow transitions**: agents query available next-actions rather than hardcoding workflow knowledge.
- **Modular tool loading by tier**: Core (7 tools, always loaded) -> Standard (15 tools, loaded on demand) -> Full (36 tools, loaded for complex tasks). This controls context window consumption. [Source: crm-pm-mcp-servers#Tiered Tool Loading]
- **Requirements traceability matrix**: map every feature to its originating requirement, enabling impact analysis when requirements change.

### 1.12 Docker Multi-Agent Architecture

The Docker research addresses deployment and operational patterns [Source: docker-compose-multi-agent-architecture]:

- **Service-per-agent pattern**: each agent runs in its own container with defined resource limits, health checks, and restart policies. This provides process isolation and independent scaling.
- **Sequential pipeline with `service_completed_successfully`**: Docker Compose dependency conditions enforce pipeline ordering without custom orchestration code.
- **MCP Gateway as service mesh**: a single gateway container consolidates multiple MCP servers, providing unified tool discovery, authentication, and rate limiting.
- **Network segmentation**: agents that should not communicate directly are placed on separate Docker networks. The `internal: true` flag prevents external access for sandboxed execution.
- **Health check grace periods**: `start_period` prevents false-positive failures during agent initialization (model loading, context hydration).
- **Docker Sandboxes (microVM isolation)**: for untrusted code execution, each agent's sandbox runs in a microVM with no host filesystem access.
- **LGTM observability stack**: Loki (logs), Grafana (dashboards), Tempo (traces), Mimir (metrics) — all self-hosted, all integrated with OpenTelemetry.
- **KEDA queue-depth auto-scaling**: scale agent containers based on task queue depth, not CPU utilization. This matches the bursty, IO-bound nature of agent workloads.

---

## 2. Cost/ROI Data (What to Prioritize)

### 2.1 Token Economics

The unit economics research provides the foundational cost model [Source: unit-economics-agent-metrics#Token Economics]:

- **Multi-agent overhead**: multi-agent systems consume 15-77x more tokens than single-agent approaches for equivalent tasks. This is the cost of coordination (system prompts replicated per agent, inter-agent messaging, shared context synchronization).
- **Token usage explains 80% of performance variance**: the strongest predictor of task success is how many tokens the agent used, not which model or prompt strategy was employed.
- **Output token pricing asymmetry**: output tokens are priced 4-8x higher than input tokens across all major providers. This means agent *reasoning* is far more expensive than agent *context*.
- **Run-to-run variance**: identical tasks can vary 10x in token consumption across runs due to LLM non-determinism. Budget planning must account for this variance.

### 2.2 Model Pricing Tiers (2026)

Current pricing landscape [Source: unit-economics-agent-metrics#Model Pricing]:

| Tier | Example | Input/Output per MTok | Relative Cost |
|------|---------|----------------------|---------------|
| Premium | Claude Opus | $5 / $25 | 1.0x |
| Mid-tier | Claude Sonnet | $3 / $15 | 0.6x |
| Lightweight | Claude Haiku | $1 / $5 | 0.2x |
| Local/OSS | Llama, Mistral | ~$0.0001 | ~0.00002x |

The cost difference between premium and lightweight tiers is **60-300x**, making model routing the single highest-leverage cost optimization.

### 2.3 Cost Optimization Strategies

Ranked by impact [Source: unit-economics-agent-metrics#Cost Optimization]:

| Strategy | Savings | Mechanism |
|----------|---------|-----------|
| **Model routing by complexity** | 30-85% | Route 30% simple tasks to Haiku, 50% medium to Sonnet, 20% complex to Opus |
| **Prompt caching** | Up to 90% on cached portions | Repeated system prompts charged at 10% of input price |
| **Batch API** | 50% | Non-real-time work submitted in bulk |
| **Context isolation** | ~67% fewer tokens | Subagent scoping prevents full-context replication |
| **Semantic caching** | Variable (73% hit rate typical) | Cache LLM responses for semantically similar queries |
| **Progressive context loading** | 20-30% | Load context lazily, not eagerly |

### 2.4 Key Metric: CAPO (Cost-per-Accepted-Outcome)

The research introduces CAPO as the primary cost metric [Source: unit-economics-agent-metrics#CAPO]:

```
CAPO = Total_loaded_cost / Accepted_outcomes
```

Where `Total_loaded_cost` includes:
- Direct API costs (tokens consumed)
- Retry costs (failed attempts that consumed tokens)
- Review costs (human time to evaluate outputs)
- Infrastructure costs (compute, storage, networking)

**Example**: A task costing $0.30 per attempt that requires 3 attempts before acceptance has a CAPO of $1.50, not $0.30. Systems without quality gates dramatically underestimate true costs.

### 2.5 Lusser's Law (Compound Failure)

A critical insight for multi-agent pipeline design [Source: unit-economics-agent-metrics#Lusser's Law]:

```
System_success = Π(agent_success_rates)
```

For a pipeline of 5 agents each with 95% success rate:
- System success = 0.95^5 = **77%**
- Nearly 1 in 4 pipeline executions will fail

For 5 agents at 90% each: 0.90^5 = **59%**

**This makes validation gates between agents economically necessary**, not optional. A $0.02 validation check that catches 80% of failures before they propagate to downstream agents (each consuming additional tokens) pays for itself within a few runs.

### 2.6 Current Benchmarks

Real-world cost data points [Source: unit-economics-agent-metrics#Benchmarks]:

| Scenario | Cost |
|----------|------|
| Claude on SWE-bench (with caching) | ~$0.30/issue |
| Unconstrained single agent | $5-8/task |
| Production without guardrails | $10-100+/task |
| ORCHA per-deal CRM pipeline | $5-8/lead |

### 2.7 Five-Guardrail FinOps Framework

Five concurrent budget enforcement mechanisms [Source: unit-economics-agent-metrics#FinOps Framework]:

1. **Loop/step limit**: maximum iterations per agent (prevents infinite loops)
2. **Tool-call cap**: maximum tool invocations per task (prevents runaway exploration)
3. **Per-task token budget**: hard ceiling on token consumption per task
4. **Wall-clock timeout**: maximum elapsed time per task
5. **Tenant budgets**: per-organization spending limits with alerts at 50%, 80%, 100%

### 2.8 Productivity and ROI Reality Check

The research presents a nuanced picture [Source: unit-economics-agent-metrics#Productivity Data]:

**Optimistic signals**:
- 3.6 hours/week saved on average across studies
- 10-30% productivity boost in controlled settings

**Cautionary signals**:
- **METR study**: experienced developers were 19% *slower* with AI assistance, despite *perceiving* a 20% speedup [^1]
- Only 5% of enterprises report meaningful returns from AI investments
- Gains concentrate in specific task types (boilerplate, test generation, documentation)

**Conservative ROI model**: 15% productivity gain, discounted by 30% for failure/rework, yields approximately **150% ROI** — positive but not transformative. The roadmap should not assume 10x productivity gains.

---

## 3. OSS Tools to Leverage (Buy vs Build)

### 3.1 Agent Orchestration

| Tool | Stars | License | Relevance to ORCHA |
|------|-------|---------|---------------------|
| **LangGraph** | 97k | OSS | State machine agents with checkpointing; closest to ORCHA's FSM-based workflow model [Source: multi-agent-system-design#Tool Landscape] |
| **CrewAI** | 45.9k | Enterprise $99/mo | Role-based multi-agent teams; good reference architecture but heavy vendor lock-in [Source: multi-agent-system-design#Tool Landscape] |
| **XState/Stately** | — | OSS | Production-grade statechart implementation; Stately Agent demonstrates LLM-FSM integration [Source: fsm-requirement-models#XState] |

**Recommendation**: Study LangGraph's checkpointing and XState's statechart semantics. Build ORCHA's orchestration natively (it is the core product) but adopt proven patterns from these frameworks.

### 3.2 Task Management and Specification

| Tool | Stars | License | Relevance to ORCHA |
|------|-------|---------|---------------------|
| **Spec Kit** | 78k+ | OSS | Spec-driven development pipeline; directly applicable to ORCHA's workflow definition layer [Source: speckit-spec-driven-development] |
| **ATLAS MCP Server** | — | OSS | Graph-based task management with MCP interface; reference for task delegation patterns [Source: atlas-task-management-mcp] |
| **Task Master** | 25.3k | OSS | PRD-to-task decomposition; useful for ORCHA's project planning features [Source: atlas-task-management-mcp#Alternatives] |

### 3.3 CRM and Project Management

| Tool | License | Relevance to ORCHA |
|------|---------|---------------------|
| **Twenty CRM** | GPL OSS | Modern Salesforce alternative; reference for CRM data model and API design [Source: crm-pm-mcp-servers#Twenty CRM] |
| **Plane** | OSS | AI-native project management; reference for PM-agent integration patterns [Source: crm-pm-mcp-servers#Plane] |

### 3.4 Infrastructure and Observability

| Tool | Pricing | Relevance to ORCHA |
|------|---------|---------------------|
| **Docker MCP Gateway** | OSS | Consolidates MCP servers into a single gateway; directly applicable to ORCHA's multi-agent deployment [Source: docker-compose-multi-agent-architecture#MCP Gateway] |
| **SWE-ReX** | OSS | Runtime abstraction for sandboxed code execution; reference for ORCHA's agent sandbox [Source: docker-compose-multi-agent-architecture#Sandboxing] |
| **Langfuse** | Self-hosted, 50K events/mo free | LLM observability with cost tracking; plug-and-play for ORCHA's monitoring needs [Source: unit-economics-agent-metrics#Observability] |
| **Helicone** | 100K requests/mo free | Request-level cost tracking and analytics [Source: unit-economics-agent-metrics#Observability] |
| **OpenTelemetry** | OSS | Auto-instrumentation standard; foundation for all observability [Source: docker-compose-multi-agent-architecture#LGTM Stack] |
| **KEDA** | OSS | Kubernetes event-driven autoscaling by queue depth [Source: docker-compose-multi-agent-architecture#KEDA] |

### 3.5 Build vs Buy Decision Framework

Based on the research, the build/buy boundary for ORCHA should be:

**Build** (core differentiator):
- Workflow orchestration engine (deterministic envelope)
- Agent supervision and error recovery
- CRM/PM ontological reduction layer
- Multi-agent collision prevention

**Buy/Adopt** (commodity infrastructure):
- Statechart primitives (adopt XState patterns)
- Observability stack (Langfuse + OpenTelemetry)
- Container orchestration (Docker Compose -> Kubernetes)
- Autoscaling (KEDA)
- Code sandboxing (SWE-ReX patterns)

**Study but do not adopt** (reference architectures):
- CrewAI (too opinionated, vendor lock-in risk)
- LangGraph (Python-centric, ORCHA is Rust)
- Twenty CRM (GPL license, different architecture)

---

## 4. Architecture Principles (How to Build It)

Ten convergent principles emerge across all nine research papers. These should serve as the architectural constitution for the ORCHA 5-year roadmap.

### Principle 1: Deterministic Envelope, Non-Deterministic Core

Every agent interaction must pass through a typed, validated contract layer. LLM creativity operates within bounded contexts, never at the system coordination level.

*Sources*: [rooroo-agent-orchestration], [fsm-requirement-models], [sudolang-aidd-framework], [speckit-spec-driven-development], [multi-agent-system-design]

### Principle 2: Formal Specification as Communication Substrate

Agents communicate through structured specifications (schemas, state machines, typed envelopes), not through natural language. Natural language is for human interfaces and within-agent reasoning only.

*Sources*: [speckit-spec-driven-development#What/How Separation], [rooroo-agent-orchestration#Structured Output], [fsm-requirement-models#RFSM/DFSM]

### Principle 3: Cost-Tiered Model Routing

No single model tier is appropriate for all tasks. The system must route tasks to the cheapest model tier capable of acceptable quality, with automatic escalation on failure.

*Sources*: [unit-economics-agent-metrics#Model Routing], [crm-pm-mcp-servers#Tiered Tool Loading]

### Principle 4: Error Recovery Hierarchy (Not Binary Succeed/Fail)

Five-level error handling (retry -> fallback -> reassign -> circuit break -> human escalate) replaces binary success/failure. Each level has cost and latency implications that inform the routing decision.

*Sources*: [multi-agent-system-design#Error Handling], [fsm-requirement-models#Traceback Transitions]

### Principle 5: Supervision and Fault Tolerance Structure

Agent groups require explicit supervision strategies (OTP-inspired). The supervisor's restart policy must match the coupling between agents in the group.

*Sources*: [multi-agent-system-design#OTP Supervision], [docker-compose-multi-agent-architecture#Health Checks]

### Principle 6: Shared State Management (Blackboard Pattern)

Agents share state through a structured, versioned blackboard — not through direct messaging. This enables audit, conflict detection, and replay.

*Sources*: [multi-agent-system-design#Shared Memory], [crm-pm-mcp-servers#Decision Ledger], [atlas-task-management-mcp#Graph Model]

### Principle 7: Memory Architecture with Lifecycle Policies

Five memory tiers (working, episodic, semantic, procedural, shared) with explicit retention policies, extraction triggers, and garbage collection. Memory is not "store everything forever."

*Sources*: [multi-agent-system-design#Memory Taxonomy], [atlas-task-management-mcp#Wisdom Accumulation], [sudolang-aidd-framework#Progressive Discovery]

### Principle 8: Human Approval and Escalation as Non-Negotiable

Every pipeline must include human approval gates for high-stakes decisions. The system must degrade gracefully when humans are unavailable (queue, not fail).

*Sources*: [crm-pm-mcp-servers#Trust Gates], [rooroo-agent-orchestration#Human Escalation], [unit-economics-agent-metrics#CAPO]

### Principle 9: Context Engineering as Highest-ROI Optimization

The most impactful performance and cost improvements come from giving agents better context, not better models. Ontological reduction, progressive loading, and context isolation collectively deliver 50-80% cost reduction.

*Sources*: [crm-pm-mcp-servers#Ontological Reduction], [sudolang-aidd-framework#Progressive Discovery], [unit-economics-agent-metrics#Context Isolation]

### Principle 10: Observability Infrastructure Built-In from Day 1

Tracing, cost tracking, and decision logging are architectural requirements, not afterthoughts. Every agent action must emit structured telemetry. Without observability, cost optimization and debugging are impossible.

*Sources*: [docker-compose-multi-agent-architecture#LGTM Stack], [unit-economics-agent-metrics#FinOps Framework], [crm-pm-mcp-servers#Decision Ledger]

---

## 5. Lessons Learned from Completed Sprints

These lessons are extracted from ORCHA's development history (sprint archives and backlog analysis) and represent hard-won operational knowledge that must inform the roadmap.

### 5.1 Root Design Decisions Compound

**BLOB UUID cascading cost**: The decision to use BLOB UUIDs for `users.id` propagated to 21 files requiring boilerplate hex-to-text conversion. This single design choice has consumed multiple sprint days across several months and remains partially unresolved (Phase C migration planned).

**Lesson**: Database schema decisions in Year 1 will still be paying interest in Year 5. The roadmap must allocate explicit time for schema migration when introducing new data models (especially agent state, memory, and telemetry tables).

### 5.2 Integration Testing Catches What Unit Tests Miss

- **Dogfood testing found 16 bugs in a single QA session** — none were caught by unit tests.
- **Playwright smoke tests caught a `UserSession.id` BLOB issue** that was invisible to Rust-side tests.
- **Query key mismatches** caused 10 critical cache invalidation bugs with zero console errors.

**Lesson**: The roadmap must budget for end-to-end testing of agent pipelines, not just unit testing of individual agent components. Agent orchestration bugs are emergent — they only appear when agents interact.

### 5.3 QA and Merge Resolution Are Consistently Underestimated

Time estimate accuracy from sprint history:

| Category | Estimated | Actual | Ratio |
|----------|-----------|--------|-------|
| Tech debt sprint | 5d | 5d | 1.0x |
| Sidebar fix | 1d | 1.5d | 1.5x |
| API split | 1.5d | 1.5d | 1.0x |
| **Dogfood QA** | **1d** | **3d** | **3.0x** |
| **Merge resolution** | **0.5d** | **1.5d** | **3.0x** |
| Org-profile split | 1d | 1.5d | 1.5x |

**Pattern**: QA and merge resolution are underestimated by 2-3x consistently. The roadmap should apply a 2.5x multiplier to all QA and integration estimates.

### 5.4 Seed Data and Onboarding Directly Affect Velocity

- **Zero seed data = 20-30 minutes of manual setup per QA pass**. Across a sprint, this accumulates to hours of wasted time.
- **Onboarding fragmentation (4 sequential modals) broke every E2E test**, requiring manual dismissal logic in test setup.

**Lesson**: Agent pipeline testing will require rich seed data (sample workflows, pre-populated CRM data, realistic task graphs). Budget for seed data creation as a first-class deliverable, not an afterthought.

### 5.5 Planning Docs Prevent Merge Conflicts

The safe-zone pattern (defining in planning documents which files each branch can modify) prevented merge conflicts on PR #49. Cherry-picking across branches, conversely, was identified as high-risk.

**Lesson**: As the team scales and multiple agent feature branches run in parallel, formalized safe zones and planning docs become essential infrastructure, not bureaucratic overhead.

---

## 6. Architectural Constraints from Prior Decisions

These are non-negotiable constraints that the 5-year roadmap must work within (or explicitly plan to migrate away from).

### 6.1 Database: SQLite -> PostgreSQL Migration Required

**Current**: SQLite as production database. No concurrent writes, no horizontal scaling.
**Constraint**: Any multi-agent feature requiring concurrent database access (which is most of them) will hit SQLite's write lock.
**Migration**: PostgreSQL migration is planned but not yet scheduled. This is a **blocking prerequisite** for production multi-agent workloads.

### 6.2 BLOB UUID Legacy

**Current**: `users.id` stored as BLOB, requiring hex-to-text conversion in 21+ files.
**Constraint**: Every new table that references `users.id` inherits this complexity. The `DbUuid` abstraction mitigates but does not eliminate the overhead.
**Migration**: Phase C migration planned but assessed as risky due to foreign key cascades.

### 6.3 Real-Time: SSE, Not WebSocket

**Current**: Server-Sent Events for real-time updates. HTTP/1.1 compatible, unidirectional.
**Constraint**: Agents cannot push messages back to the server through the real-time channel. All agent-to-server communication must go through HTTP requests.
**Implication**: Bidirectional agent communication patterns (e.g., negotiation, debate) require polling or a separate WebSocket layer.

### 6.4 MCP as Agent Interface

**Current**: All agents communicate via MCP (Model Context Protocol) tools.
**Constraint**: MCP is the public interface contract. Internal optimizations (direct function calls, shared memory) must not break MCP compatibility.
**Implication**: This is actually an advantage — MCP provides a stable abstraction boundary that enables agent replacement without system changes.

### 6.5 Workflow Builder as Primary Extensibility Mechanism

**Current**: Every new capability should be expressible as a workflow node type.
**Constraint**: Features that cannot be represented as workflow nodes require core platform changes, which are higher cost and higher risk.
**Implication**: The node type system must be designed for extensibility from Year 1. A rigid node type system will become the bottleneck for feature velocity.

### 6.6 CRM Organization-Scoped, Not Project-Scoped

**Current**: CRM data is scoped to `organization_id`. Project-level CRM routes are orphaned (7 routes).
**Constraint**: Multi-project organizations share a single CRM namespace. Project-specific CRM views require filtering, not separate data stores.
**Implication**: Agent pipelines that operate on CRM data must be org-aware, not project-aware.

### 6.7 Sovereign Sync: One-Way Only

**Current**: Dropbox -> E: drive synchronization is one-way. Peers cannot push changes back.
**Constraint**: Any collaborative editing or distributed agent workflow that requires bidirectional sync must use a different mechanism.

### 6.8 HMAC Webhook Triggers

**Current**: Webhook triggers use HMAC authentication.
**Constraint**: Future trigger types (scheduled, event-driven, agent-initiated) must follow the same authentication pattern for consistency.

---

## 7. References

### Agent Orchestration Frameworks

1. **RooRoo** — Structured agent orchestration with typed output envelopes and role-based delegation. [Source: rooroo-agent-orchestration]
2. **CrewAI** — Multi-agent orchestration framework with role-based teams. https://github.com/crewAIInc/crewAI (45.9k stars) [^2]
3. **LangGraph / LangChain** — State machine agent framework with checkpointing. https://github.com/langchain-ai/langchain (97k stars) [^3]
4. **XState / Stately** — Statechart implementation for JavaScript/TypeScript. Stately Agent extends this for LLM workflows. https://stately.ai [^4]
5. **Google Agent Development Kit (ADK)** — Google's multi-agent framework with workflow agents. [^5]

### Academic Papers

6. **MetaAgent** — LLM-generated multi-agent systems from task descriptions. arXiv:2502.03916 [^6]
7. **Deep Agent** — Hierarchical task decomposition with recursive subtask DAGs. arXiv:2502.07056 [^7]
8. **Hao et al. (2024)** — "Are More LLM Calls All You Need?" Multi-agent scaling analysis showing 15-77x token overhead. [^8]
9. **METR (2025)** — Controlled study finding experienced developers 19% slower with AI assistance despite perceiving 20% speedup. [^9]

### Industry Research

10. **Anthropic** — Model Context Protocol (MCP) specification; prompt caching documentation; Claude model pricing. https://docs.anthropic.com [^10]
11. **Anthropic (2025)** — "Building Effective Agents" — patterns for tool use, prompt engineering, and agent architectures. [^11]
12. **McKinsey (2024)** — Enterprise AI adoption survey: only 5% of enterprises reporting meaningful returns. [^12]
13. **Deloitte (2024)** — Developer productivity with AI assistants: 3.6 hours/week saved. [^13]
14. **O'Reilly (2024)** — Technology Radar assessment of AI agent frameworks. [^14]

### Tools and Platforms

15. **Spec Kit (Claude Code)** — Spec-driven development pipeline. Part of Claude Code ecosystem (78k+ stars). [Source: speckit-spec-driven-development] [^15]
16. **ATLAS MCP Server** — Graph-based task management with Neo4j backend and MCP interface. [Source: atlas-task-management-mcp] [^16]
17. **Task Master** — PRD-to-task decomposition tool. https://github.com/eyaltoledano/claude-task-master (25.3k stars) [^17]
18. **Twenty CRM** — Open-source CRM alternative to Salesforce. GPL licensed. https://twenty.com [^18]
19. **Plane** — Open-source, AI-native project management. https://plane.so [^19]
20. **Docker MCP Gateway** — MCP server consolidation via Docker. [Source: docker-compose-multi-agent-architecture] [^20]
21. **SWE-ReX** — Runtime abstraction layer for sandboxed code execution in agent systems. [^21]
22. **Langfuse** — Self-hosted LLM observability platform. 50K events/mo free tier. https://langfuse.com [^22]
23. **Helicone** — LLM request tracking and cost analytics. 100K requests/mo free tier. https://helicone.ai [^23]
24. **OpenTelemetry** — Vendor-neutral observability framework. https://opentelemetry.io [^24]
25. **KEDA** — Kubernetes Event-Driven Autoscaling. https://keda.sh [^25]

### Cost and Economics Sources

26. **SWE-bench** — Software engineering benchmark; Claude achieving ~$0.30/issue with caching. https://www.swebench.com [^26]
27. **Lusser's Law** — Compound probability of system failure in series-connected components. Originally from reliability engineering. [^27]
28. **FinOps Foundation** — Cloud financial management practices applied to LLM spending. https://www.finops.org [^28]

### ORCHA Internal Sources

29. **Sprint Archives** — `planning/archive/` — historical sprint plans, reviews, and retrospectives.
30. **Backlog** — `planning/BACKLOG--remaining-work.md` — tracked remaining quality work items.
31. **Research Papers** — `planning/2026-03-18--research--*.md` — the 9 source documents for this synthesis.

---

[^1]: METR (2025). "Measuring the Impact of AI on Developer Productivity." Controlled study with experienced developers.
[^2]: CrewAI. https://github.com/crewAIInc/crewAI — Multi-agent orchestration framework.
[^3]: LangChain / LangGraph. https://github.com/langchain-ai/langchain — LLM application framework with state machine agents.
[^4]: XState / Stately. https://stately.ai — Visual statechart editor and runtime.
[^5]: Google Agent Development Kit. https://google.github.io/adk-docs — Multi-agent framework with sequential, parallel, and loop agents.
[^6]: MetaAgent: Automated Multi-Agent System Generation. arXiv:2502.03916 (2025).
[^7]: Deep Agent: Hierarchical Task Decomposition for Complex Software Engineering. arXiv:2502.07056 (2025).
[^8]: Hao et al. "Are More LLM Calls All You Need? Towards Scaling Laws of Compound AI Systems" (2024).
[^9]: METR. "Measuring the Impact of Early-stage AI on Experienced Open-Source Developer Productivity" (2025).
[^10]: Anthropic Documentation. https://docs.anthropic.com — MCP specification, prompt caching, model pricing.
[^11]: Anthropic. "Building Effective Agents" (2025). https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/building-effective-agents
[^12]: McKinsey Global Survey on AI (2024). "The State of AI in Early 2024."
[^13]: Deloitte. "Generative AI and the Future of Work" (2024).
[^14]: O'Reilly. "Technology Radar: AI Agent Frameworks" (2024).
[^15]: Claude Code / Spec Kit. https://github.com/anthropics/claude-code
[^16]: ATLAS MCP Server. Task management via Model Context Protocol with graph-based knowledge management.
[^17]: Task Master. https://github.com/eyaltoledano/claude-task-master — AI-driven task decomposition.
[^18]: Twenty CRM. https://github.com/twentyhq/twenty — Open-source CRM.
[^19]: Plane. https://github.com/makeplane/plane — Open-source project management.
[^20]: Docker MCP Toolkit. Docker-native MCP server management and gateway.
[^21]: SWE-ReX. Runtime execution abstraction for sandboxed agent environments.
[^22]: Langfuse. https://github.com/langfuse/langfuse — Open-source LLM observability.
[^23]: Helicone. https://helicone.ai — LLM observability and cost tracking.
[^24]: OpenTelemetry. https://opentelemetry.io — Vendor-neutral telemetry standard.
[^25]: KEDA. https://keda.sh — Kubernetes Event-Driven Autoscaling.
[^26]: SWE-bench. https://www.swebench.com — Software engineering agent benchmark.
[^27]: Lusser, R. Reliability engineering principle: series system reliability equals product of component reliabilities.
[^28]: FinOps Foundation. https://www.finops.org — Cloud financial management best practices.

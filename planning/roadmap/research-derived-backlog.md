# Research-Derived Backlog — Future Improvements

**Date:** 2026-03-18
**Source:** 9 research papers from the 5-year product roadmap sprint
**Context:** These items were synthesized from research into agent orchestration, multi-agent systems, workflow modeling, spec-driven development, CRM/PM MCP patterns, containerized architecture, and unit economics. They are prioritized by ROI and grouped by research domain. All items map to stages in `5-year-product-roadmap.md`.

---

## P1 — Agent Orchestration Core (RooRoo Research)

### 1. Structured Response Protocol
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #1)
**What:** Define a standard JSON response envelope for all agent-to-orchestrator communication: `{ status: "Done|NeedsClarification|Failed", message, artifacts, clarification_needed }`. Currently agents return freeform conversation text — the orchestrator must interpret natural language to determine outcome. RooRoo's Output Envelope pattern eliminates this ambiguity.
**Why:** Structured responses enable mechanical result parsing, reliable error escalation, and formalized "I need help" pathways (preventing cascading errors from bad assumptions). This is the highest-ROI pattern from the RooRoo research.
**Roadmap Stage:** 0
**Status:** NOT STARTED

### 2. Cost-Tiered Model Routing
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #2)
**What:** Map agent roles to explicit model tiers: orchestration/routing → Haiku-class (cheap/fast), analysis/documentation → Sonnet-class (mid), planning/creative/complex coding → Opus-class (expensive). RooRoo's Navigator runs on the cheapest tier — it's a router, not a thinker. We already have multi-model support but don't tier by role.
**Why:** Massive cost reduction. Most orchestration work (triage, dispatch, logging) doesn't need frontier intelligence. Reserve expensive models for where they matter.
**Roadmap Stage:** 0-1
**Status:** NOT STARTED

### 3. Context Isolation Between Sub-tasks
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #3)
**What:** When the orchestrator dispatches work to a sub-agent, the sub-agent should operate in isolation (own conversation history) and return only a summary. Parent receives completion summary, not full execution trace. Modeled on Roo Code's Boomerang Tasks pattern.
**Why:** Prevents context window bloat in the parent orchestrator. The parent doesn't need to know *how* work was done, just *what was accomplished*. Enables longer orchestration chains without hitting token limits.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 4. Clarification as First-Class Status
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #4)
**What:** Agents should have an explicit `NeedsClarification` response type rather than embedding ad-hoc questions in freeform text. When an agent encounters uncertainty, it returns a typed clarification request that bubbles up cleanly to the user or parent orchestrator. RooRoo calls this the "Principle of Least Assumption" — agents clarify, never guess.
**Why:** Prevents cascading errors from incorrect assumptions. Makes agent uncertainty visible and actionable rather than hidden in prose.
**Roadmap Stage:** 0
**Status:** NOT STARTED

### 5. Task Briefing Documents (context.md)
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #5)
**What:** For each dispatched task, generate a lean briefing document that links to relevant files/docs rather than embedding content. This is the "contract" between orchestrator and executor. RooRoo uses `context.md` per task with workspace-relative paths and links (not inlined content).
**Why:** Reduces token usage, ensures agents reference authoritative sources, and creates an auditable record of what each agent was told to do.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 6. Append-Only Event Logging
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #6)
**What:** Add a lightweight JSONL event log for agent orchestration events (task dispatched, completed, failed, clarification requested). RooRoo's v0.4.0 shifted from monolithic JSON state to append-only JSONL — more robust for concurrent agent operations, simpler file operations, corruption-resistant.
**Why:** Enables debugging, replay, and audit trails for agent workflows. JSONL format is append-friendly and doesn't require complex document mutations.
**Roadmap Stage:** 0-1
**Status:** NOT STARTED

---

## P1 — Task Management & Dependencies (ATLAS Research)

### 7. Task Dependency Graph with DAG Validation
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #2)
**What:** Add dependency edges between tasks with circular dependency detection on create/update, "blocked" status when dependencies are incomplete, and query support ("what tasks are blocked by X?"). ATLAS uses Neo4j for this but the pattern works in SQLite with a `task_dependencies` junction table + recursive CTE for cycle detection. Transforms our flat task list into a DAG-aware workflow engine.
**Why:** Without dependencies, agents can't express "do A before B" — they must manually sequence work or rely on prompts. DAG-aware task management enables automatic blocking, unblocking, and execution ordering.
**Roadmap Stage:** 1-2
**Status:** NOT STARTED

### 8. Formalize Completion Criteria on Tasks/Projects
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #1)
**What:** Add `completion_requirements` (text) and `output_format` (text) as structured fields on tasks and projects. Every task explicitly defines what "done" looks like *at creation time*. ATLAS makes these first-class fields on every entity.
**Why:** Forces agents and humans to think about success criteria upfront rather than as freeform text buried in descriptions. Enables automated completion validation — the orchestrator can check deliverables against stated requirements.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 9. Structured Task Delegation Template
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #3, from oh-my-opencode Atlas pattern)
**What:** Adopt a six-section delegation format for all agent-dispatched work: TASK (atomic goal), EXPECTED OUTCOME (success criteria), REQUIRED TOOLS (explicit whitelist), MUST DO (exhaustive requirements), MUST NOT DO (forbidden actions), CONTEXT (file paths, patterns, constraints). Prevents ambiguous task assignments and provides explicit anti-patterns.
**Why:** "Atlas thinks, workers do" — separation of concerns. Unstructured delegation leads to tool sprawl, scope creep, and wasted tokens. The MUST NOT DO section is particularly valuable for preventing known failure modes.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 10. Knowledge as First-Class Entity
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #4)
**What:** Promote agent-discovered information from ephemeral execution artifacts to persistent, queryable knowledge items with domain categorization, tagging, citation/source tracking, and cross-project search. ATLAS's knowledge layer prevents the "genius with amnesia" problem — findings from one session are discoverable in future sessions.
**Why:** Currently agent findings are stored as execution artifact blobs. They can't be searched, cited, or reused across projects. A structured knowledge layer makes agent-discovered information a durable asset.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 11. Server-Side Gate Enforcement for Task Transitions
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #6, from task-orchestrator MCP)
**What:** Replace raw status updates with named transition triggers (`start`, `complete`, `block`, `cancel`) that validate prerequisites server-side. The server checks "planning floors" (required documentation/artifacts per phase) and returns `{canAdvance, missing, guidancePointer}` when gates aren't satisfied. Invalid transitions become impossible at the API level.
**Why:** Relying on agents to self-manage state transitions via prompts is brittle. Server-enforced gates provide "deterministic workflow progression that doesn't depend on prompt discipline."
**Roadmap Stage:** 1-2
**Status:** NOT STARTED

### 12. Cross-Task Learning Propagation (Wisdom Accumulation)
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #5, from oh-my-opencode Atlas pattern)
**What:** When agents discover patterns, gotchas, or successful approaches during task execution, record them in structured notes (learnings, decisions, issues, verification results) that subsequent agents can query. "Learnings from Task 1 prevent mistakes in Task 5."
**Why:** Without cross-task learning, every agent starts from zero. Accumulated wisdom reduces redundant exploration and prevents repeating known failure modes.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

---

## P1 — Agent Specifications & Quality (SudoLang/AIDD + Spec Kit Research)

### 13. Constraint-Based Agent Specifications (PICS Pattern)
**Source:** `2026-03-18--research--sudolang-aidd-framework.md` (Recommendation #1)
**What:** Define agent behavior as interfaces with explicit state, constraints, commands, and components (SudoLang's PICS pattern: Preamble, Interface, Components, Start). Constraints should be structured declarations that the LLM maintains across all interactions — not buried in freeform prompt text. Example: "This agent must never modify files outside its assigned worktree" is a constraint, not an instruction.
**Why:** Declarative constraints survive context window limits; procedural instructions degrade over long conversations. Research shows 12-38% response quality improvement with structured pseudocode vs plain language. Constraints auto-synchronize state, preventing drift. This is the foundational pattern from SudoLang — everything else builds on it.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 14. Vision/Mission Anchor Document for Agent Alignment
**Source:** `2026-03-18--research--sudolang-aidd-framework.md` (Recommendation #2)
**What:** Create a formal project vision document that all agents must reference before starting work. Include: project goals, user personas, architectural decisions, design principles, success metrics, and scope constraints. Agents detecting contradictions between task and vision must escalate, not proceed. AIDD's `vision.md` is mandatory first-read for all agents.
**Why:** Without a formal constraint anchor, agents optimize locally while violating project-level goals. CLAUDE.md covers coding conventions but not project vision. This prevents agent drift toward speculative features or locally-optimal-but-globally-wrong decisions.
**Roadmap Stage:** 0-1
**Status:** NOT STARTED

### 15. Phased Agent Workflow with Quality Gates
**Source:** `2026-03-18--research--sudolang-aidd-framework.md` (Recommendation #4)
**What:** Structure agent task lifecycle as explicit phases (discover → plan → execute → review → test) with quality gates between each. Don't let agents skip from "assigned" to "done" without review and testing. AIDD's six-command pipeline (/discover → /task → /execute → /review → /user-test → /commit) enforces this. Complements the server-side gate enforcement from ATLAS research.
**Why:** Without explicit phases and gates, AI agents accumulate technical debt. GitClear tracked 8x more code duplication with AI adoption; DORA reports show 9% higher bug rates. The pipeline IS the quality assurance.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 16. File-Path-Based Parallel Boundary Detection
**Source:** `2026-03-18--research--speckit-spec-driven-development.md` (Recommendation #2)
**What:** When orchestrating parallel agent work, analyze task descriptions for file paths to determine safe parallel boundaries. Tasks touching overlapping files must be serialized or assigned to the same agent. Spec Kit's `team-implement` uses regex to extract paths, groups by two-segment prefix, and builds a conflict graph where connected components = independent work streams. "File paths are unambiguous" — keyword detection creates false matches.
**Why:** Parallel agent execution without file-conflict analysis causes merge conflicts and overwrites. File ownership must be a hard guarantee enforced by construction, not a guideline agents might ignore. This is the single most important pattern for safe multi-agent parallelism.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 17. Mandatory Review Agent in Multi-Agent Workflows
**Source:** `2026-03-18--research--speckit-spec-driven-development.md` (Recommendation #3)
**What:** Every multi-agent workflow must include a dedicated QA/security gatekeeper agent that is separate from implementing agents. The reviewer has fresh context and catches: placeholder code marked complete, security violations passing local tests, quality gate failures implementers ignore, undocumented TODOs. Even for single-directory projects with 5-6 tasks, the reviewer justifies token costs.
**Why:** "Who watches the watcher?" Implementers have cognitive blind spots about their own work. A separate reviewer with no shared context catches issues that are invisible to the agents who created them. Spec Kit found this was the single highest-value addition to their parallel execution system.
**Roadmap Stage:** 1-2
**Status:** NOT STARTED

### 18. Structured Specification Artifacts (Spec + Plan Separation)
**Source:** `2026-03-18--research--speckit-spec-driven-development.md` (Recommendation #1)
**What:** For complex agent tasks, generate a structured spec (what to build, who it's for, acceptance criteria) and a separate plan (how to build it, architecture, constraints) before implementation begins. Spec Kit generates ~8 files per feature: specification, implementation plan, tasks, data model, API contract, component architecture, research notes. The spec captures user intent; the plan captures technical approach. Changing "how" shouldn't require rewriting "what."
**Why:** "With parallel agents working independently, vague specs lead to inconsistent assumptions across teammates." Spec quality pays exponential dividends with parallel execution. This also creates inspectable, persistent documents that serve as inter-agent communication substrate — more reliable than conversational hand-off.
**Roadmap Stage:** 1
**Status:** NOT STARTED

---

## P1 — CRM/PM MCP Patterns

### 19. Context Compaction Layer for Agent Consumption
**Source:** `2026-03-18--research--crm-pm-mcp-servers.md` (Recommendation #1)
**What:** Build a semantic layer between our database and agent consumption that pre-computes ontological summaries: deal scores, engagement signals, risk indicators, relationship graphs. Don't expose raw DB rows to agents. A contact becomes *"Sarah Chen (CFO, Champion), visited pricing 12x, similar accounts convert 73% in 45 days"* — 500 tokens instead of 100,000 raw events.
**Why:** Raw CRM/PM data overwhelms agent context windows and degrades decision quality. Enterprise MCP implementations (Warmly, Salesforce) report 10-100x token reduction while improving accuracy. "Raw metadata alone causes misinterpretation" — agents need interpreted context, not database dumps.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 20. Multi-Agent Collision Prevention (Policy + Ledger + Trust Gates)
**Source:** `2026-03-18--research--crm-pm-mcp-servers.md` (Recommendation #3)
**What:** Implement three governance layers for multi-agent coordination: (1) **Policy engine** — YAML/JSON rules constraining agent actions per entity type ("max 1 touch per account per day, 72h cooldown"), (2) **Decision ledger** — every action logged with reasoning, confidence score, and world-model snapshot, (3) **Trust gates** — high-risk actions require policy + trust + authorization; low-confidence routes to human review queue. Plus **entity ownership locks** preventing concurrent edits.
**Why:** Prompt-based coordination doesn't scale. Without governance layers, agents over-contact prospects, make conflicting edits, and take high-risk actions without authorization. This is the enterprise-grade pattern for multi-agent safety.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 21. Discoverable Workflow Transitions via MCP
**Source:** `2026-03-18--research--crm-pm-mcp-servers.md` (Recommendation #2)
**What:** Our task/deal MCP should expose valid transitions per entity state, not accept any status string. Pattern: `getAvailableTransitions(taskId)` returns permitted actions (like Atlassian's `getTransitionsForJiraIssue`). Agents discover what they *can* do before attempting actions. Complements server-side gate enforcement from ATLAS research and named transition triggers from task-orchestrator research.
**Why:** Prevents invalid transitions at the protocol level. Agent prompt discipline is brittle; protocol-level enforcement is deterministic. Also provides self-documenting API — agents learn the workflow by querying it.
**Roadmap Stage:** 1-2
**Status:** NOT STARTED

---

## P1 — FSM & Workflow Engine

### 22. FSM-Based Task Lifecycle with Guard Conditions
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #1)
**What:** Replace the string `status` field with a formal state machine. Define valid transitions with guard conditions (e.g., `start` requires `assigned_to IS NOT NULL`, `complete` requires all subtasks done), enforce server-side via named transition triggers. Converges with ATLAS (server-side gate enforcement), task-orchestrator (named triggers), and CRM/PM MCP (discoverable transitions) research — FSMs are the formal foundation for all three patterns.
**Why:** Currently any status string can be set on any task regardless of preconditions. FSMs make invalid transitions impossible at the API level, not just by prompt discipline. This is the single most impactful architectural pattern across all six research documents.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 23. Deterministic Workflow Engine (State Machine Orchestrator)
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #2)
**What:** The orchestration engine should be a state machine, not a procedural script. Each workflow step is a state; transitions fire on events (agent completion, approval, timeout, error); guards prevent invalid progression. Agents are invoked services within states — the FSM controls *when* agents run, agents control *what* they produce. XState/statecharts provide hierarchical states (phase → sub-steps) and parallel regions (concurrent agent work streams with `onDone` synchronization).
**Why:** Makes workflow behavior predictable, inspectable, and debuggable. The FSM is the source of truth for "what happens next" — not the agent's prompt. "Deterministic envelope, non-deterministic core" — the workflow is reliable, the agent's creativity is preserved within each state.
**Roadmap Stage:** 1-2
**Status:** NOT STARTED

### 24. Traceback + Null Transitions for Error Recovery
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #3)
**What:** When an agent's output fails review at step 5 of 8, don't restart from step 1. Add two transition types beyond standard: (a) **traceback** — return to a specific prior state with accumulated context about what went wrong, and (b) **null transitions** (self-loops) — let the agent retry within the current state with feedback. MetaAgent's three-transition-type FSM achieves 85% checkpoint passage on software tasks.
**Why:** Current error handling is binary: succeed or restart. Traceback preserves work from successful earlier steps. Null transitions enable iterative refinement without state machine progression. Together they dramatically reduce wasted agent computation on partial failures.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 25. Event-State Analysis for Workflow Definitions
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #4)
**What:** When defining a new workflow, construct an Event × State matrix where every (event, state) pair is explicitly decided: transition, impossible, ignored, or error. Empty cells = specification gaps. Tooling addition: given a workflow definition with states and events, auto-generate the ESA matrix and highlight undecided cells.
**Why:** Catches the class of bugs where "what happens if the user cancels while the agent is mid-execution?" has no answer. The ESA matrix is the cheapest completeness-checking technique — it's a table, not a proof.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 26. Artifact State Machines (Draft → Review → Approved)
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #6)
**What:** Every significant output (spec, plan, code, test result) gets its own lifecycle state machine: `draft → in_review → approved → complete` with `rejected → draft` traceback. Artifacts can't be consumed by downstream workflow states until approved. Prevents agents from building on unapproved intermediate outputs.
**Why:** Currently artifacts are blobs with no lifecycle. An agent can produce a spec and immediately start implementing it with no review gate. Artifact FSMs enforce quality gates between workflow phases.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 27. Layered Validation (Deterministic Checks Before LLM Review)
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #7)
**What:** Before running expensive LLM validation on agent outputs, run cheap deterministic checks in order: (1) schema validation — does output match expected format? (2) completeness — are all required fields present? (3) consistency — does output reference existing entities correctly? (4) regression — does output break previously passing conditions? Only outputs passing all deterministic checks advance to LLM review.
**Why:** Most rejections are structural, not semantic. Catching them with cheap checks reduces LLM review costs while maintaining quality. McKinsey QuantumBlack reports this as a key pattern for enterprise agent workflows.
**Roadmap Stage:** 2
**Status:** NOT STARTED

---

## P1 — Multi-Agent System Architecture

### 28. Hybrid Reactive/Deliberative Orchestration Architecture
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #1)
**What:** Split the orchestration engine into two layers: (a) a **reactive event loop** handling health checks, timeouts, status transitions, and circuit breaker logic with sub-second latency; (b) a **deliberative planner** handling task decomposition, agent selection, re-planning on failure, and workflow progression. The reactive layer runs continuously; the deliberative layer activates on significant events. This is "the most practical template for agentic AI" per architecture surveys covering 2,500+ papers.
**Why:** Current architecture has no runtime engine at all (P0 #1). When building it, the two-layer split prevents the orchestrator from being either too slow (all deliberative) or too brittle (all reactive). Fast safety responses + slow intelligent planning.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 29. OTP-Style Supervision Hierarchy for Agent Lifecycle
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #2)
**What:** Implement Erlang/OTP supervision tree patterns: orchestrator is the supervisor, agents are workers. Three restart strategies: **one-for-one** (retry failed agent), **one-for-all** (restart all agents when shared state corrupted), **rest-for-one** (restart downstream agents when upstream fails). Fragile components (agent tasks) deep in tree; stable components (orchestrator, memory, tools) shallow. Converges with the 5-level error handling hierarchy: tool retries → agent fallback → orchestrator reassignment → circuit breaker → human escalation.
**Why:** Without supervision, failed agents are just "failed" — no automatic recovery, no cascading failure management, no restart strategies. OTP patterns are battle-tested across 30+ years of telecom-grade systems and map directly to agent orchestration.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 30. Blackboard Pattern for Shared Agent State
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #4)
**What:** Implement a shared knowledge store where agents post and retrieve project state asynchronously: file changes, test results, build status, discovered patterns, error contexts. Agents don't communicate directly — they read/write to the blackboard. Solves the 36.9% failure rate from interagent misalignment (agents lacking visibility into what others have completed). Converges with ATLAS knowledge-as-first-class-entity and cross-task learning propagation recommendations.
**Why:** Research shows 36.9% of multi-agent failures stem from state visibility gaps — not communication breakdowns. Without shared state, agents make decisions based on incomplete information, leading to conflicting edits, redundant work, and cascading assumption errors.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 31. HTDAG Lazy Task Decomposition
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #5)
**What:** Replace flat task lists with a recursive Hierarchical Task DAG. Tasks are either **atomic** (single file edit) or **non-atomic** (decomposable). Decomposition happens lazily — only break down the next level when an agent begins work. Failed nodes trigger context-aware re-planning while successful nodes persist. Converges with task dependency graph (ATLAS #7) but adds hierarchical nesting and lazy decomposition.
**Why:** Premature full decomposition wastes planning effort on tasks that may change. Lazy decomposition adapts to discovered complexity. Partial progress preservation means a failure at step 5 doesn't discard successful steps 1-4.
**Roadmap Stage:** 3
**Status:** NOT STARTED

### 32. Circuit Breaker Pattern for LLM API Calls
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #7)
**What:** Add circuit breakers (closed → open → half-open) to all LLM provider integrations. Monitor failure rates and error codes (429, 502, 503). When thresholds exceeded, trip the breaker: stop sending to failing provider, route to fallback model, implement cooldown before recovery. Three-layer resilience: retries (transient) → fallbacks (provider switch) → circuit breakers (systemic protection).
**Why:** "Retries don't know when a failure is persistent." Current retry logic hammers failing endpoints. Circuit breakers proactively stop traffic, preventing cascading failures and wasted tokens/cost.
**Roadmap Stage:** 1-2
**Status:** NOT STARTED

### 33. Context Compilation Pipeline
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #8)
**What:** Replace raw conversation replay with ordered context processors: (a) compile stable prefix (instructions, identity, tool schemas) separately from variable suffix (latest turns); (b) summarize older events via sliding-window compaction; (c) externalize large payloads by handle, loaded on-demand. Google ADK's approach — observable, testable transformation stages instead of ad-hoc prompt concatenation.
**Why:** Single agents use ~4x tokens over chat; multi-agent uses ~15x. Context degradation is contagious — Agent A's poor output enters Agent B's context as ground truth. Structured compilation with caching and compaction dramatically reduces costs while maintaining accuracy.
**Roadmap Stage:** 2
**Status:** NOT STARTED

---

## P1 — Containerized Architecture (Docker/Compose Research)

### 34. Containerized Agent Execution with Resource Isolation
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #1)
**What:** Wrap each coding agent execution in a Docker container with: resource limits (memory, CPU), network isolation (internal network with no internet), workspace volume mount (Git worktree), and ephemeral lifecycle (destroy after task completion). One container image per agent type (Claude, Gemini, etc.), spawned per task. This is the industry standard — Docker's compose-for-agents, OpenHands, SWE-agent, and Cerebras all use container-per-agent isolation.
**Why:** Currently agents run as in-process executors sharing server resources. A crashed or misbehaving agent can affect other agents and the orchestrator. Container isolation provides resource limits, filesystem isolation, and network segmentation. Ephemeral containers prevent state contamination between tasks. This is the infrastructure foundation that the OTP supervision hierarchy (MAS #29) operates on.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 35. Docker Compose Service Architecture for ORCHA
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #4)
**What:** Define the full ORCHA stack as a Docker Compose configuration: orchestrator service, agent worker pool, database, message queue (Redis for task distribution), MCP Gateway, and optional observability stack (LGTM). Uses profiles for environment modes (`--profile dev`, `--profile gpu`, `--profile monitoring`). Three-file LLM provider strategy: `compose.yaml` (base) + `compose.local-llm.yaml` + `compose.cloud-llm.yaml`. Init container chain for automated setup (migrations → seed → verify → orchestrator).
**Why:** Makes the entire system portable, reproducible, and deployable with `docker compose up`. Eliminates "works on my machine" problems. Enables: environment switching via profiles (not code changes), LLM provider swapping via override files, and automated setup via init containers. Single command to bring up the full development stack.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 36. Sandboxed Code Execution for Agent-Generated Code
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #3)
**What:** Agent-generated code must execute in isolated containers, not in the server process. Use Docker Sandboxes (microVM) or `withNetworkMode("none")` containers. The agent writes code to its workspace volume; a sandboxed executor runs it and returns results. Per-tool network isolation: code execution sandboxes have no network; documentation tools have full network. MCP Gateway routes transparently.
**Why:** Currently agent code runs with server process permissions — no isolation from the host filesystem, network, or other agents. Agent-generated code could: access sensitive files, make unauthorized network calls, consume unbounded resources, or interfere with other agent executions. Container sandboxing is the minimum security baseline for production coding agents.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 37. MCP Gateway Service for Centralized Tool Access
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #2)
**What:** Deploy Docker's MCP Gateway as a shared Compose service consolidating all MCP server connections. Agent containers connect to one gateway endpoint instead of individual servers. Enables: centralized access control (which agents can use which tools), per-tool network isolation (code sandbox has no internet; documentation tools do), and simplified agent configuration. Uses catalog YAML for server definitions with per-server isolation settings.
**Why:** Currently each agent independently connects to MCP servers — no centralized control, no per-tool isolation, no access auditing. A gateway service reduces connection overhead and provides a single point for access control and observability.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 38. OpenTelemetry + LGTM Observability Stack
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #7)
**What:** Add `tracing-opentelemetry` to the Rust backend. Deploy LGTM stack (Loki/Grafana/Tempo/Prometheus) as Compose services behind a `monitoring` profile. OpenTelemetry Collector routes: traces → Tempo, logs → Loki, metrics → Prometheus. Grafana dashboards for agent fleet metrics: task completion rate, LLM API latency, token usage, cost per task, error rates, queue depth. Correlation IDs link operations across agent containers.
**Why:** Current observability is basic execution logs with Rust `tracing`. Multi-agent containerized systems require distributed tracing to debug cross-service issues. The LGTM stack deploys alongside agents with no external infrastructure. OpenTelemetry reached GA for all signals in 2025 with Rust support.
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 39. Agent Health Check Infrastructure
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #6)
**What:** Every agent service exposes a health endpoint verifying: LLM API connectivity, database connection, workspace filesystem access, MCP server availability. The orchestrator only dispatches tasks to agents reporting healthy status. Health checks use `start_period` for graceful startup. Downstream services use `depends_on: condition: service_healthy` for dependency ordering. This is the container-native implementation of the circuit breaker pattern (MAS research).
**Why:** Without health checks, the orchestrator dispatches tasks to agents that may have lost LLM connectivity, have corrupted workspaces, or have exhausted resources. Health-gated dispatch prevents wasted computation and improves task success rates.
**Roadmap Stage:** 2
**Status:** NOT STARTED

---

## P1 — Cost Economics & Metrics (Unit Economics Research)

### 40. Per-Task Cost Tracking and CAPO Calculation
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #1)
**What:** Instrument every agent invocation to track: input tokens, output tokens, cached tokens, tool calls, retries, model used, wall-clock time. Calculate **CAPO (Cost per Accepted Outcome)** per task type: `CAPO = (inference_cost + tool_cost + retry_cost + infra_overhead) / accepted_outcomes`. Store in the task record alongside completion status. This is the foundational metric — without it, all cost optimization is blind.
**Why:** No cost tracking exists today. A task appearing to cost $0.30 in tokens but requiring 3 attempts and human review has a true CAPO of ~$1.50 (5x apparent cost). Multi-agent token amplification is 15-77x vs single-agent. Only 5% of enterprises see real returns from AI — per-task cost tracking is what separates the 5% from the 95%.
**Roadmap Stage:** 0-1
**Status:** NOT STARTED

### 41. Five-Guardrail Cost Control System
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #2)
**What:** Implement all five guardrails from the InfoWorld FinOps framework: (1) loop/step limit per task, (2) tool-call cap with sub-caps for expensive tools, (3) per-task token budget (research: 100K, formatting: 5K — not global), (4) wall-clock timeout, (5) per-tenant/project budgets with anomaly alerts. Auto-downgrade model tier when 80% of budget consumed. 3x rate-of-change detectors catch infinite retry loops within hours.
**Why:** Without guardrails, a single stuck agent can burn an entire daily budget in an infinite retry loop. Every retry is a fresh charge — 5 retries = 6x expected cost. The most dangerous pattern: growing context + identical error + retry = exponentially increasing cost. Token budgets are per-task, not global, because a research task needs 100K tokens but a formatting task needs 5K.
**Roadmap Stage:** 1
**Status:** NOT STARTED

### 42. Prompt Caching Strategy for Agent Instructions
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #3)
**What:** Implement Anthropic prompt caching for: agent system prompts (repeated every invocation), project context (CLAUDE.md, codebase summaries), tool schemas. Cache reads cost 10% of standard input price (90% savings), paying for itself after 1 re-use. The orchestrator sends similar context with every agent dispatch — caching this alone reduces input token costs by 50-70%.
**Why:** Agent system prompts and project context are sent with every invocation but rarely change. Without caching, we pay full price for identical content every time. With caching at 5-minute TTL, 1.25x write premium breaks even after a single re-read. For high-frequency orchestration calls, the savings are massive. Combined with batch API (50% discount) for non-real-time tasks, total savings reach 60-80%.
**Roadmap Stage:** 0-1
**Status:** NOT STARTED

### 43. Compound Failure Dashboard (Lusser's Law)
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #5)
**What:** Track per-agent success rates and calculate system-level reliability using Lusser's Law (`system_success = Π(agent_success_rates)`). Display on dashboard: if any agent drops below 95%, flag compound impact. 5 agents at 95% each = 77% system success. Validation gates that catch 50% of failures between two 95% agents raise effective pair reliability from 90% to 95%. Makes the economic case for validation gates visible.
**Why:** Compound failure is the hidden economics of multi-agent systems — 10 agents at 95% each = only 59.9% system success. Without visibility into this, the system appears to work (each agent mostly succeeds) but the end-to-end workflow fails 40% of the time. "Without explicit fault tolerance, agentic systems are economically unsustainable."
**Roadmap Stage:** 2-3
**Status:** NOT STARTED

### 44. Agent Performance Dashboard (Goal Accuracy, CAPO, Trends)
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #6)
**What:** Track and display: goal accuracy (target 85%+), first-pass success rate, human escalation rate (target <10%), token efficiency, retry rate, CAPO per task type, cost trends over time. Use self-hosted Langfuse or LGTM stack. Answer: "Which task types are cost-effective for agents vs humans?" Includes ROI calculator: `ROI% = ((hours_saved × rate × failure_discount) - tool_cost) / tool_cost × 100`.
**Why:** Only 28% of leaders report clear measurable value from AI. The METR study found developers were 19% slower with AI but believed they were 20% faster — self-reported gains are unreliable. Outcome-based metrics (CAPO, goal accuracy) reveal actual economics. Without them, we can't distinguish profitable from money-losing workflows.
**Roadmap Stage:** 2
**Status:** NOT STARTED

### 45. LLM-as-Judge for Automated Quality Gates
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #8)
**What:** Replace human review with LLM-as-judge for intermediate workflow steps. 53% of deployed agent teams already use this. Economics: 80% agreement with human preferences at 500x-5,000x lower cost. Use Haiku-class model as judge — cost is negligible. Catches structural quality issues before expensive downstream agents process bad inputs. Converges with layered validation (FSM research) and mandatory review agent (Spec Kit research).
**Why:** Human review is the bottleneck in agent workflows — expensive, slow, and doesn't scale. LLM-as-judge provides affordable quality gates. A Haiku judge at $1/MTok catching 80% of issues before an Opus agent at $25/MTok processes them saves 80% × $25 = $20/MTok on avoided downstream waste.
**Roadmap Stage:** 2
**Status:** NOT STARTED

---

## Cross-Reference: Roadmap Stage Summary

| Stage | Items |
|-------|-------|
| **0** | #1 (Response Protocol), #4 (Clarification Status), #14 (Vision Doc) |
| **0-1** | #2 (Model Routing), #6 (Event Logging), #40 (CAPO Tracking), #42 (Prompt Caching) |
| **1** | #3 (Context Isolation), #5 (Task Briefing), #8 (Completion Criteria), #9 (Delegation Template), #15 (Quality Gates), #18 (Spec Artifacts), #22 (FSM Lifecycle), #28 (Hybrid Architecture), #41 (Cost Guardrails) |
| **1-2** | #7 (Task DAG), #11 (Gate Enforcement), #17 (Review Agent), #21 (Discoverable Transitions), #23 (Workflow Engine), #32 (Circuit Breaker) |
| **2** | #10 (Knowledge Entity), #13 (PICS Pattern), #16 (Parallel Boundaries), #19 (Context Compaction), #24 (Traceback), #25 (ESA Matrix), #26 (Artifact FSMs), #27 (Layered Validation), #29 (OTP Supervision), #33 (Context Compilation), #35 (Docker Compose), #39 (Health Checks), #44 (Performance Dashboard), #45 (LLM-as-Judge) |
| **2-3** | #12 (Cross-Task Learning), #20 (Collision Prevention), #30 (Blackboard), #34 (Containerized Execution), #36 (Sandboxed Code), #37 (MCP Gateway), #38 (OpenTelemetry), #43 (Compound Failure Dashboard) |
| **3** | #31 (HTDAG Decomposition) |

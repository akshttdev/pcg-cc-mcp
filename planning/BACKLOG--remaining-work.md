# Backlog — Remaining Work

**Last updated:** 2026-03-18 (post frontend polish sprint + RooRoo + ATLAS + SudoLang/AIDD + Spec Kit + CRM/PM MCP + FSM/RFSM + MAS design + Docker/Compose + Unit Economics research)
**Context:** Consolidated from all completed planning docs. Items prioritized by impact and dependency.

---

## P0 — Blockers / Critical

### 1. Agent Flow Orchestration Engine (Phase 3H)
**Source:** `archive/2026-03-12--plan--agent-task-mcp-wiring.md`
**What:** Agent flows exist as data model only — no background worker to progress phases, enforce gates, or handle delegation. Largest remaining architecture gap.
**Status:** NOT STARTED

### ~~2. ACP Agents Get No MCP Servers~~ → RESOLVED
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F5)
**Resolution:** Already implemented — `load_platform_mcp_servers()` in `acp/harness.rs` loads from `default_mcp.json` and passes to all ACP sessions (Gemini, Qwen). Discovered during pipeline enhancement sprint (2026-03-16).

---

## P1 — Major / High Impact

### 3. Agent Orchestration — Structured Response Protocol
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #1)
**What:** Define a standard JSON response envelope for all agent-to-orchestrator communication: `{ status: "Done|NeedsClarification|Failed", message, artifacts, clarification_needed }`. Currently agents return freeform conversation text — the orchestrator must interpret natural language to determine outcome. RooRoo's Output Envelope pattern eliminates this ambiguity.
**Why:** Structured responses enable mechanical result parsing, reliable error escalation, and formalized "I need help" pathways (preventing cascading errors from bad assumptions). This is the highest-ROI pattern from the RooRoo research.
**Status:** NOT STARTED

### 4. Agent Orchestration — Cost-Tiered Model Routing
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #2)
**What:** Map agent roles to explicit model tiers: orchestration/routing → Haiku-class (cheap/fast), analysis/documentation → Sonnet-class (mid), planning/creative/complex coding → Opus-class (expensive). RooRoo's Navigator runs on the cheapest tier — it's a router, not a thinker. We already have multi-model support but don't tier by role.
**Why:** Massive cost reduction. Most orchestration work (triage, dispatch, logging) doesn't need frontier intelligence. Reserve expensive models for where they matter.
**Status:** NOT STARTED

### 5. Agent Orchestration — Context Isolation Between Sub-tasks
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #3)
**What:** When the orchestrator dispatches work to a sub-agent, the sub-agent should operate in isolation (own conversation history) and return only a summary. Parent receives completion summary, not full execution trace. Modeled on Roo Code's Boomerang Tasks pattern.
**Why:** Prevents context window bloat in the parent orchestrator. The parent doesn't need to know *how* work was done, just *what was accomplished*. Enables longer orchestration chains without hitting token limits.
**Status:** NOT STARTED

### 6. Task Dependency Graph with DAG Validation
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #2)
**What:** Add dependency edges between tasks with circular dependency detection on create/update, "blocked" status when dependencies are incomplete, and query support ("what tasks are blocked by X?"). ATLAS uses Neo4j for this but the pattern works in SQLite with a `task_dependencies` junction table + recursive CTE for cycle detection. Transforms our flat task list into a DAG-aware workflow engine.
**Why:** Without dependencies, agents can't express "do A before B" — they must manually sequence work or rely on prompts. DAG-aware task management enables automatic blocking, unblocking, and execution ordering.
**Status:** NOT STARTED

### 7. Formalize Completion Criteria on Tasks/Projects
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #1)
**What:** Add `completion_requirements` (text) and `output_format` (text) as structured fields on tasks and projects. Every task explicitly defines what "done" looks like *at creation time*. ATLAS makes these first-class fields on every entity.
**Why:** Forces agents and humans to think about success criteria upfront rather than as freeform text buried in descriptions. Enables automated completion validation — the orchestrator can check deliverables against stated requirements.
**Status:** NOT STARTED

### 8. Structured Task Delegation Template
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #3, from oh-my-opencode Atlas pattern)
**What:** Adopt a six-section delegation format for all agent-dispatched work: TASK (atomic goal), EXPECTED OUTCOME (success criteria), REQUIRED TOOLS (explicit whitelist), MUST DO (exhaustive requirements), MUST NOT DO (forbidden actions), CONTEXT (file paths, patterns, constraints). Prevents ambiguous task assignments and provides explicit anti-patterns.
**Why:** "Atlas thinks, workers do" — separation of concerns. Unstructured delegation leads to tool sprawl, scope creep, and wasted tokens. The MUST NOT DO section is particularly valuable for preventing known failure modes.
**Status:** NOT STARTED

### 9. Constraint-Based Agent Specifications (PICS Pattern)
**Source:** `2026-03-18--research--sudolang-aidd-framework.md` (Recommendation #1)
**What:** Define agent behavior as interfaces with explicit state, constraints, commands, and components (SudoLang's PICS pattern: Preamble, Interface, Components, Start). Constraints should be structured declarations that the LLM maintains across all interactions — not buried in freeform prompt text. Example: "This agent must never modify files outside its assigned worktree" is a constraint, not an instruction.
**Why:** Declarative constraints survive context window limits; procedural instructions degrade over long conversations. Research shows 12-38% response quality improvement with structured pseudocode vs plain language. Constraints auto-synchronize state, preventing drift. This is the foundational pattern from SudoLang — everything else builds on it.
**Status:** NOT STARTED

### 10. Vision/Mission Anchor Document for Agent Alignment
**Source:** `2026-03-18--research--sudolang-aidd-framework.md` (Recommendation #2)
**What:** Create a formal project vision document that all agents must reference before starting work. Include: project goals, user personas, architectural decisions, design principles, success metrics, and scope constraints. Agents detecting contradictions between task and vision must escalate, not proceed. AIDD's `vision.md` is mandatory first-read for all agents.
**Why:** Without a formal constraint anchor, agents optimize locally while violating project-level goals. CLAUDE.md covers coding conventions but not project vision. This prevents agent drift toward speculative features or locally-optimal-but-globally-wrong decisions.
**Status:** NOT STARTED

### 11. Phased Agent Workflow with Quality Gates
**Source:** `2026-03-18--research--sudolang-aidd-framework.md` (Recommendation #4)
**What:** Structure agent task lifecycle as explicit phases (discover → plan → execute → review → test) with quality gates between each. Don't let agents skip from "assigned" to "done" without review and testing. AIDD's six-command pipeline (/discover → /task → /execute → /review → /user-test → /commit) enforces this. Complements the server-side gate enforcement from ATLAS research.
**Why:** Without explicit phases and gates, AI agents accumulate technical debt. GitClear tracked 8x more code duplication with AI adoption; DORA reports show 9% higher bug rates. The pipeline IS the quality assurance.
**Status:** NOT STARTED

### 12. File-Path-Based Parallel Boundary Detection
**Source:** `2026-03-18--research--speckit-spec-driven-development.md` (Recommendation #2)
**What:** When orchestrating parallel agent work, analyze task descriptions for file paths to determine safe parallel boundaries. Tasks touching overlapping files must be serialized or assigned to the same agent. Spec Kit's `team-implement` uses regex to extract paths, groups by two-segment prefix, and builds a conflict graph where connected components = independent work streams. "File paths are unambiguous" — keyword detection creates false matches.
**Why:** Parallel agent execution without file-conflict analysis causes merge conflicts and overwrites. File ownership must be a hard guarantee enforced by construction, not a guideline agents might ignore. This is the single most important pattern for safe multi-agent parallelism.
**Status:** NOT STARTED

### 13. Mandatory Review Agent in Multi-Agent Workflows
**Source:** `2026-03-18--research--speckit-spec-driven-development.md` (Recommendation #3)
**What:** Every multi-agent workflow must include a dedicated QA/security gatekeeper agent that is separate from implementing agents. The reviewer has fresh context and catches: placeholder code marked complete, security violations passing local tests, quality gate failures implementers ignore, undocumented TODOs. Even for single-directory projects with 5-6 tasks, the reviewer justifies token costs.
**Why:** "Who watches the watcher?" Implementers have cognitive blind spots about their own work. A separate reviewer with no shared context catches issues that are invisible to the agents who created them. Spec Kit found this was the single highest-value addition to their parallel execution system.
**Status:** NOT STARTED

### 14. Structured Specification Artifacts (Spec + Plan Separation)
**Source:** `2026-03-18--research--speckit-spec-driven-development.md` (Recommendation #1)
**What:** For complex agent tasks, generate a structured spec (what to build, who it's for, acceptance criteria) and a separate plan (how to build it, architecture, constraints) before implementation begins. Spec Kit generates ~8 files per feature: specification, implementation plan, tasks, data model, API contract, component architecture, research notes. The spec captures user intent; the plan captures technical approach. Changing "how" shouldn't require rewriting "what."
**Why:** "With parallel agents working independently, vague specs lead to inconsistent assumptions across teammates." Spec quality pays exponential dividends with parallel execution. This also creates inspectable, persistent documents that serve as inter-agent communication substrate — more reliable than conversational hand-off.
**Status:** NOT STARTED

### 15. Context Compaction Layer for Agent Consumption
**Source:** `2026-03-18--research--crm-pm-mcp-servers.md` (Recommendation #1)
**What:** Build a semantic layer between our database and agent consumption that pre-computes ontological summaries: deal scores, engagement signals, risk indicators, relationship graphs. Don't expose raw DB rows to agents. A contact becomes *"Sarah Chen (CFO, Champion), visited pricing 12x, similar accounts convert 73% in 45 days"* — 500 tokens instead of 100,000 raw events.
**Why:** Raw CRM/PM data overwhelms agent context windows and degrades decision quality. Enterprise MCP implementations (Warmly, Salesforce) report 10-100x token reduction while improving accuracy. "Raw metadata alone causes misinterpretation" — agents need interpreted context, not database dumps.
**Status:** NOT STARTED

### 16. Multi-Agent Collision Prevention (Policy + Ledger + Trust Gates)
**Source:** `2026-03-18--research--crm-pm-mcp-servers.md` (Recommendation #3)
**What:** Implement three governance layers for multi-agent coordination: (1) **Policy engine** — YAML/JSON rules constraining agent actions per entity type ("max 1 touch per account per day, 72h cooldown"), (2) **Decision ledger** — every action logged with reasoning, confidence score, and world-model snapshot, (3) **Trust gates** — high-risk actions require policy + trust + authorization; low-confidence routes to human review queue. Plus **entity ownership locks** preventing concurrent edits.
**Why:** Prompt-based coordination doesn't scale. Without governance layers, agents over-contact prospects, make conflicting edits, and take high-risk actions without authorization. This is the enterprise-grade pattern for multi-agent safety.
**Status:** NOT STARTED

### 17. Discoverable Workflow Transitions via MCP
**Source:** `2026-03-18--research--crm-pm-mcp-servers.md` (Recommendation #2)
**What:** Our task/deal MCP should expose valid transitions per entity state, not accept any status string. Pattern: `getAvailableTransitions(taskId)` returns permitted actions (like Atlassian's `getTransitionsForJiraIssue`). Agents discover what they *can* do before attempting actions. Complements server-side gate enforcement from ATLAS research and named transition triggers from task-orchestrator research.
**Why:** Prevents invalid transitions at the protocol level. Agent prompt discipline is brittle; protocol-level enforcement is deterministic. Also provides self-documenting API — agents learn the workflow by querying it.
**Status:** NOT STARTED

### 18. FSM-Based Task Lifecycle with Guard Conditions
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #1)
**What:** Replace the string `status` field with a formal state machine. Define valid transitions with guard conditions (e.g., `start` requires `assigned_to IS NOT NULL`, `complete` requires all subtasks done), enforce server-side via named transition triggers. Converges with ATLAS (server-side gate enforcement), task-orchestrator (named triggers), and CRM/PM MCP (discoverable transitions) research — FSMs are the formal foundation for all three patterns.
**Why:** Currently any status string can be set on any task regardless of preconditions. FSMs make invalid transitions impossible at the API level, not just by prompt discipline. This is the single most impactful architectural pattern across all six research documents.
**Status:** NOT STARTED

### 19. Deterministic Workflow Engine (State Machine Orchestrator)
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #2)
**What:** The orchestration engine should be a state machine, not a procedural script. Each workflow step is a state; transitions fire on events (agent completion, approval, timeout, error); guards prevent invalid progression. Agents are invoked services within states — the FSM controls *when* agents run, agents control *what* they produce. XState/statecharts provide hierarchical states (phase → sub-steps) and parallel regions (concurrent agent work streams with `onDone` synchronization).
**Why:** Makes workflow behavior predictable, inspectable, and debuggable. The FSM is the source of truth for "what happens next" — not the agent's prompt. "Deterministic envelope, non-deterministic core" — the workflow is reliable, the agent's creativity is preserved within each state.
**Status:** NOT STARTED

### 20. Traceback + Null Transitions for Error Recovery
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #3)
**What:** When an agent's output fails review at step 5 of 8, don't restart from step 1. Add two transition types beyond standard: (a) **traceback** — return to a specific prior state with accumulated context about what went wrong, and (b) **null transitions** (self-loops) — let the agent retry within the current state with feedback. MetaAgent's three-transition-type FSM achieves 85% checkpoint passage on software tasks.
**Why:** Current error handling is binary: succeed or restart. Traceback preserves work from successful earlier steps. Null transitions enable iterative refinement without state machine progression. Together they dramatically reduce wasted agent computation on partial failures.
**Status:** NOT STARTED

### 21. Hybrid Reactive/Deliberative Orchestration Architecture
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #1)
**What:** Split the orchestration engine into two layers: (a) a **reactive event loop** handling health checks, timeouts, status transitions, and circuit breaker logic with sub-second latency; (b) a **deliberative planner** handling task decomposition, agent selection, re-planning on failure, and workflow progression. The reactive layer runs continuously; the deliberative layer activates on significant events. This is "the most practical template for agentic AI" per architecture surveys covering 2,500+ papers.
**Why:** Current architecture has no runtime engine at all (P0 #1). When building it, the two-layer split prevents the orchestrator from being either too slow (all deliberative) or too brittle (all reactive). Fast safety responses + slow intelligent planning.
**Status:** NOT STARTED

### 22. OTP-Style Supervision Hierarchy for Agent Lifecycle
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #2)
**What:** Implement Erlang/OTP supervision tree patterns: orchestrator is the supervisor, agents are workers. Three restart strategies: **one-for-one** (retry failed agent), **one-for-all** (restart all agents when shared state corrupted), **rest-for-one** (restart downstream agents when upstream fails). Fragile components (agent tasks) deep in tree; stable components (orchestrator, memory, tools) shallow. Converges with the 5-level error handling hierarchy: tool retries → agent fallback → orchestrator reassignment → circuit breaker → human escalation.
**Why:** Without supervision, failed agents are just "failed" — no automatic recovery, no cascading failure management, no restart strategies. OTP patterns are battle-tested across 30+ years of telecom-grade systems and map directly to agent orchestration.
**Status:** NOT STARTED

### 23. Blackboard Pattern for Shared Agent State
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #4)
**What:** Implement a shared knowledge store where agents post and retrieve project state asynchronously: file changes, test results, build status, discovered patterns, error contexts. Agents don't communicate directly — they read/write to the blackboard. Solves the 36.9% failure rate from interagent misalignment (agents lacking visibility into what others have completed). Converges with ATLAS knowledge-as-first-class-entity and cross-task learning propagation recommendations.
**Why:** Research shows 36.9% of multi-agent failures stem from state visibility gaps — not communication breakdowns. Without shared state, agents make decisions based on incomplete information, leading to conflicting edits, redundant work, and cascading assumption errors.
**Status:** NOT STARTED

### 24. Containerized Agent Execution with Resource Isolation
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #1)
**What:** Wrap each coding agent execution in a Docker container with: resource limits (memory, CPU), network isolation (internal network with no internet), workspace volume mount (Git worktree), and ephemeral lifecycle (destroy after task completion). One container image per agent type (Claude, Gemini, etc.), spawned per task. This is the industry standard — Docker's compose-for-agents, OpenHands, SWE-agent, and Cerebras all use container-per-agent isolation.
**Why:** Currently agents run as in-process executors sharing server resources. A crashed or misbehaving agent can affect other agents and the orchestrator. Container isolation provides resource limits, filesystem isolation, and network segmentation. Ephemeral containers prevent state contamination between tasks. This is the infrastructure foundation that the OTP supervision hierarchy (MAS #22) operates on.
**Status:** NOT STARTED

### 25. Docker Compose Service Architecture for ORCHA
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #4)
**What:** Define the full ORCHA stack as a Docker Compose configuration: orchestrator service, agent worker pool, database, message queue (Redis for task distribution), MCP Gateway, and optional observability stack (LGTM). Uses profiles for environment modes (`--profile dev`, `--profile gpu`, `--profile monitoring`). Three-file LLM provider strategy: `compose.yaml` (base) + `compose.local-llm.yaml` + `compose.cloud-llm.yaml`. Init container chain for automated setup (migrations → seed → verify → orchestrator).
**Why:** Makes the entire system portable, reproducible, and deployable with `docker compose up`. Eliminates "works on my machine" problems. Enables: environment switching via profiles (not code changes), LLM provider swapping via override files, and automated setup via init containers. Single command to bring up the full development stack.
**Status:** NOT STARTED

### 26. Sandboxed Code Execution for Agent-Generated Code
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #3)
**What:** Agent-generated code must execute in isolated containers, not in the server process. Use Docker Sandboxes (microVM) or `withNetworkMode("none")` containers. The agent writes code to its workspace volume; a sandboxed executor runs it and returns results. Per-tool network isolation: code execution sandboxes have no network; documentation tools have full network. MCP Gateway routes transparently.
**Why:** Currently agent code runs with server process permissions — no isolation from the host filesystem, network, or other agents. Agent-generated code could: access sensitive files, make unauthorized network calls, consume unbounded resources, or interfere with other agent executions. Container sandboxing is the minimum security baseline for production coding agents.
**Status:** NOT STARTED

### 27. Per-Task Cost Tracking and CAPO Calculation
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #1)
**What:** Instrument every agent invocation to track: input tokens, output tokens, cached tokens, tool calls, retries, model used, wall-clock time. Calculate **CAPO (Cost per Accepted Outcome)** per task type: `CAPO = (inference_cost + tool_cost + retry_cost + infra_overhead) / accepted_outcomes`. Store in the task record alongside completion status. This is the foundational metric — without it, all cost optimization is blind.
**Why:** No cost tracking exists today. A task appearing to cost $0.30 in tokens but requiring 3 attempts and human review has a true CAPO of ~$1.50 (5x apparent cost). Multi-agent token amplification is 15-77x vs single-agent. Only 5% of enterprises see real returns from AI — per-task cost tracking is what separates the 5% from the 95%.
**Status:** NOT STARTED

### 28. Five-Guardrail Cost Control System
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #2)
**What:** Implement all five guardrails from the InfoWorld FinOps framework: (1) loop/step limit per task, (2) tool-call cap with sub-caps for expensive tools, (3) per-task token budget (research: 100K, formatting: 5K — not global), (4) wall-clock timeout, (5) per-tenant/project budgets with anomaly alerts. Auto-downgrade model tier when 80% of budget consumed. 3x rate-of-change detectors catch infinite retry loops within hours.
**Why:** Without guardrails, a single stuck agent can burn an entire daily budget in an infinite retry loop. Every retry is a fresh charge — 5 retries = 6x expected cost. The most dangerous pattern: growing context + identical error + retry = exponentially increasing cost. Token budgets are per-task, not global, because a research task needs 100K tokens but a formatting task needs 5K.
**Status:** NOT STARTED

### 29. Prompt Caching Strategy for Agent Instructions
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #3)
**What:** Implement Anthropic prompt caching for: agent system prompts (repeated every invocation), project context (CLAUDE.md, codebase summaries), tool schemas. Cache reads cost 10% of standard input price (90% savings), paying for itself after 1 re-use. The orchestrator sends similar context with every agent dispatch — caching this alone reduces input token costs by 50-70%.
**Why:** Agent system prompts and project context are sent with every invocation but rarely change. Without caching, we pay full price for identical content every time. With caching at 5-minute TTL, 1.25x write premium breaks even after a single re-read. For high-frequency orchestration calls, the savings are massive. Combined with batch API (50% discount) for non-real-time tasks, total savings reach 60-80%.
**Status:** NOT STARTED

### 18. Unify MCP Config Systems
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F9)
**What:** Settings > MCP Servers configures Claude Code CLI's `~/.claude.json`. The executor pipeline reads `default_mcp.json`. Completely separate — confusing for users.

### ~~4. Workflow Trigger System~~ → RESOLVED
**Source:** `archive/2026-03-12--review--ui-backend-capability-gaps.md` (F12)
**Resolution:** Frontend polish sprint (2026-03-18) added webhook triggers with HMAC-SHA256 validation, execution audit trail (`trigger_executions` table), per-trigger cooldown, retry logic, and full frontend UI. Event triggers and schedule triggers already existed. All three trigger types now operational.
**Known issues (from PR #49 review):** ~~Webhook route behind `require_auth` (P1)~~ FIXED, retry logic dead code (P2), cooldown race condition (P2). See P2 section below.

### ~~5. E2E Demo Test Run~~ → RESOLVED
**Source:** `2026-03-14--plan--e2e-demo-refactor.md`
**Resolution:** All suites run and passing — health checks 40/40, demos 28/31 (1 pre-existing flaky locator in bug-report-lifecycle Step 6). Structured test commands added to package.json with env pre-flight check (PR #34).

---

## P1.5 — Notification & Demo Enhancements

### ~~Notification Quick Action Buttons~~ → RESOLVED
**Source:** 2026-03-15 dogfood session
**Resolution:** Implemented in PR #39 — InboxNotificationItem has source-aware action buttons (View Task, View Run), dismiss button; ActivityNotificationItem has status chips and view buttons; NotificationCenter has improved deep-linking (2026-03-16).

### Demo Script Improvements
**Source:** 2026-03-15 dogfood session
**What:** Add better context/narration to demo scripts — annotations explaining what each step demonstrates, why it matters. Currently the demos work but don't convey the product story.
**Status:** PARTIALLY DONE — workflow CRM demo extended with Parts 5-8 (approve, runs tab, CRM contacts, pipeline). Bug report + manual QA demos still need narration.

---

## P2 — Medium / UX Polish

### ~~6. AgentWatcherPanel Error State~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #3)
**Resolution:** PR #41 added `loadError` state with error UI (AlertTriangle icon + "Failed to load agent reviewers" message + RefreshCw retry button). Distinguishes "no reviewers configured" from "API error".

### Knowledge as First-Class Entity
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #4)
**What:** Promote agent-discovered information from ephemeral execution artifacts to persistent, queryable knowledge items with domain categorization, tagging, citation/source tracking, and cross-project search. ATLAS's knowledge layer prevents the "genius with amnesia" problem — findings from one session are discoverable in future sessions.
**Why:** Currently agent findings are stored as execution artifact blobs. They can't be searched, cited, or reused across projects. A structured knowledge layer makes agent-discovered information a durable asset.
**Status:** NOT STARTED

### Server-Side Gate Enforcement for Task Transitions
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #6, from task-orchestrator MCP)
**What:** Replace raw status updates with named transition triggers (`start`, `complete`, `block`, `cancel`) that validate prerequisites server-side. The server checks "planning floors" (required documentation/artifacts per phase) and returns `{canAdvance, missing, guidancePointer}` when gates aren't satisfied. Invalid transitions become impossible at the API level.
**Why:** Relying on agents to self-manage state transitions via prompts is brittle. Server-enforced gates provide "deterministic workflow progression that doesn't depend on prompt discipline."
**Status:** NOT STARTED

### Cross-Task Learning Propagation (Wisdom Accumulation)
**Source:** `2026-03-18--research--atlas-task-management-mcp.md` (Recommendation #5, from oh-my-opencode Atlas pattern)
**What:** When agents discover patterns, gotchas, or successful approaches during task execution, record them in structured notes (learnings, decisions, issues, verification results) that subsequent agents can query. "Learnings from Task 1 prevent mistakes in Task 5."
**Why:** Without cross-task learning, every agent starts from zero. Accumulated wisdom reduces redundant exploration and prevents repeating known failure modes.
**Status:** NOT STARTED

### Event-State Analysis for Workflow Definitions
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #4)
**What:** When defining a new workflow, construct an Event × State matrix where every (event, state) pair is explicitly decided: transition, impossible, ignored, or error. Empty cells = specification gaps. Tooling addition: given a workflow definition with states and events, auto-generate the ESA matrix and highlight undecided cells.
**Why:** Catches the class of bugs where "what happens if the user cancels while the agent is mid-execution?" has no answer. The ESA matrix is the cheapest completeness-checking technique — it's a table, not a proof.
**Status:** NOT STARTED

### Artifact State Machines (Draft → Review → Approved)
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #6)
**What:** Every significant output (spec, plan, code, test result) gets its own lifecycle state machine: `draft → in_review → approved → complete` with `rejected → draft` traceback. Artifacts can't be consumed by downstream workflow states until approved. Prevents agents from building on unapproved intermediate outputs.
**Why:** Currently artifacts are blobs with no lifecycle. An agent can produce a spec and immediately start implementing it with no review gate. Artifact FSMs enforce quality gates between workflow phases.
**Status:** NOT STARTED

### Layered Validation (Deterministic Checks Before LLM Review)
**Source:** `2026-03-18--research--fsm-requirement-models.md` (Recommendation #7)
**What:** Before running expensive LLM validation on agent outputs, run cheap deterministic checks in order: (1) schema validation — does output match expected format? (2) completeness — are all required fields present? (3) consistency — does output reference existing entities correctly? (4) regression — does output break previously passing conditions? Only outputs passing all deterministic checks advance to LLM review.
**Why:** Most rejections are structural, not semantic. Catching them with cheap checks reduces LLM review costs while maintaining quality. McKinsey QuantumBlack reports this as a key pattern for enterprise agent workflows.
**Status:** NOT STARTED

### HTDAG Lazy Task Decomposition
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #5)
**What:** Replace flat task lists with a recursive Hierarchical Task DAG. Tasks are either **atomic** (single file edit) or **non-atomic** (decomposable). Decomposition happens lazily — only break down the next level when an agent begins work. Failed nodes trigger context-aware re-planning while successful nodes persist. Converges with task dependency graph (ATLAS #6) but adds hierarchical nesting and lazy decomposition.
**Why:** Premature full decomposition wastes planning effort on tasks that may change. Lazy decomposition adapts to discovered complexity. Partial progress preservation means a failure at step 5 doesn't discard successful steps 1-4.
**Status:** NOT STARTED

### Circuit Breaker Pattern for LLM API Calls
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #7)
**What:** Add circuit breakers (closed → open → half-open) to all LLM provider integrations. Monitor failure rates and error codes (429, 502, 503). When thresholds exceeded, trip the breaker: stop sending to failing provider, route to fallback model, implement cooldown before recovery. Three-layer resilience: retries (transient) → fallbacks (provider switch) → circuit breakers (systemic protection).
**Why:** "Retries don't know when a failure is persistent." Current retry logic hammers failing endpoints. Circuit breakers proactively stop traffic, preventing cascading failures and wasted tokens/cost.
**Status:** NOT STARTED

### Context Compilation Pipeline
**Source:** `2026-03-18--research--multi-agent-system-design.md` (Recommendation #8)
**What:** Replace raw conversation replay with ordered context processors: (a) compile stable prefix (instructions, identity, tool schemas) separately from variable suffix (latest turns); (b) summarize older events via sliding-window compaction; (c) externalize large payloads by handle, loaded on-demand. Google ADK's approach — observable, testable transformation stages instead of ad-hoc prompt concatenation.
**Why:** Single agents use ~4x tokens over chat; multi-agent uses ~15x. Context degradation is contagious — Agent A's poor output enters Agent B's context as ground truth. Structured compilation with caching and compaction dramatically reduces costs while maintaining accuracy.
**Status:** NOT STARTED

### MCP Gateway Service for Centralized Tool Access
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #2)
**What:** Deploy Docker's MCP Gateway as a shared Compose service consolidating all MCP server connections. Agent containers connect to one gateway endpoint instead of individual servers. Enables: centralized access control (which agents can use which tools), per-tool network isolation (code sandbox has no internet; documentation tools do), and simplified agent configuration. Uses catalog YAML for server definitions with per-server isolation settings.
**Why:** Currently each agent independently connects to MCP servers — no centralized control, no per-tool isolation, no access auditing. A gateway service reduces connection overhead and provides a single point for access control and observability.
**Status:** NOT STARTED

### OpenTelemetry + LGTM Observability Stack
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #7)
**What:** Add `tracing-opentelemetry` to the Rust backend. Deploy LGTM stack (Loki/Grafana/Tempo/Prometheus) as Compose services behind a `monitoring` profile. OpenTelemetry Collector routes: traces → Tempo, logs → Loki, metrics → Prometheus. Grafana dashboards for agent fleet metrics: task completion rate, LLM API latency, token usage, cost per task, error rates, queue depth. Correlation IDs link operations across agent containers.
**Why:** Current observability is basic execution logs with Rust `tracing`. Multi-agent containerized systems require distributed tracing to debug cross-service issues. The LGTM stack deploys alongside agents with no external infrastructure. OpenTelemetry reached GA for all signals in 2025 with Rust support.
**Status:** NOT STARTED

### Agent Health Check Infrastructure
**Source:** `2026-03-18--research--docker-compose-multi-agent-architecture.md` (Recommendation #6)
**What:** Every agent service exposes a health endpoint verifying: LLM API connectivity, database connection, workspace filesystem access, MCP server availability. The orchestrator only dispatches tasks to agents reporting healthy status. Health checks use `start_period` for graceful startup. Downstream services use `depends_on: condition: service_healthy` for dependency ordering. This is the container-native implementation of the circuit breaker pattern (MAS research).
**Why:** Without health checks, the orchestrator dispatches tasks to agents that may have lost LLM connectivity, have corrupted workspaces, or have exhausted resources. Health-gated dispatch prevents wasted computation and improves task success rates.
**Status:** NOT STARTED

### Compound Failure Dashboard (Lusser's Law)
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #5)
**What:** Track per-agent success rates and calculate system-level reliability using Lusser's Law (`system_success = Π(agent_success_rates)`). Display on dashboard: if any agent drops below 95%, flag compound impact. 5 agents at 95% each = 77% system success. Validation gates that catch 50% of failures between two 95% agents raise effective pair reliability from 90% to 95%. Makes the economic case for validation gates visible.
**Why:** Compound failure is the hidden economics of multi-agent systems — 10 agents at 95% each = only 59.9% system success. Without visibility into this, the system appears to work (each agent mostly succeeds) but the end-to-end workflow fails 40% of the time. "Without explicit fault tolerance, agentic systems are economically unsustainable."
**Status:** NOT STARTED

### Agent Performance Dashboard (Goal Accuracy, CAPO, Trends)
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #6)
**What:** Track and display: goal accuracy (target 85%+), first-pass success rate, human escalation rate (target <10%), token efficiency, retry rate, CAPO per task type, cost trends over time. Use self-hosted Langfuse or LGTM stack. Answer: "Which task types are cost-effective for agents vs humans?" Includes ROI calculator: `ROI% = ((hours_saved × rate × failure_discount) - tool_cost) / tool_cost × 100`.
**Why:** Only 28% of leaders report clear measurable value from AI. The METR study found developers were 19% slower with AI but believed they were 20% faster — self-reported gains are unreliable. Outcome-based metrics (CAPO, goal accuracy) reveal actual economics. Without them, we can't distinguish profitable from money-losing workflows.
**Status:** NOT STARTED

### LLM-as-Judge for Automated Quality Gates
**Source:** `2026-03-18--research--unit-economics-agent-metrics.md` (Recommendation #8)
**What:** Replace human review with LLM-as-judge for intermediate workflow steps. 53% of deployed agent teams already use this. Economics: 80% agreement with human preferences at 500x-5,000x lower cost. Use Haiku-class model as judge — cost is negligible. Catches structural quality issues before expensive downstream agents process bad inputs. Converges with layered validation (FSM research) and mandatory review agent (Spec Kit research).
**Why:** Human review is the bottleneck in agent workflows — expensive, slow, and doesn't scale. LLM-as-judge provides affordable quality gates. A Haiku judge at $1/MTok catching 80% of issues before an Opus agent at $25/MTok processes them saves 80% × $25 = $20/MTok on avoided downstream waste.
**Status:** NOT STARTED

### Agent Orchestration — Clarification as First-Class Status
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #4)
**What:** Agents should have an explicit `NeedsClarification` response type rather than embedding ad-hoc questions in freeform text. When an agent encounters uncertainty, it returns a typed clarification request that bubbles up cleanly to the user or parent orchestrator. RooRoo calls this the "Principle of Least Assumption" — agents clarify, never guess.
**Why:** Prevents cascading errors from incorrect assumptions. Makes agent uncertainty visible and actionable rather than hidden in prose.
**Status:** NOT STARTED

### Agent Orchestration — Task Briefing Documents (context.md)
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #5)
**What:** For each dispatched task, generate a lean briefing document that links to relevant files/docs rather than embedding content. This is the "contract" between orchestrator and executor. RooRoo uses `context.md` per task with workspace-relative paths and links (not inlined content).
**Why:** Reduces token usage, ensures agents reference authoritative sources, and creates an auditable record of what each agent was told to do.
**Status:** NOT STARTED

### Agent Orchestration — Append-Only Event Logging
**Source:** `2026-03-18--research--rooroo-agent-orchestration.md` (Recommendation #6)
**What:** Add a lightweight JSONL event log for agent orchestration events (task dispatched, completed, failed, clarification requested). RooRoo's v0.4.0 shifted from monolithic JSON state to append-only JSONL — more robust for concurrent agent operations, simpler file operations, corruption-resistant.
**Why:** Enables debugging, replay, and audit trails for agent workflows. JSONL format is append-friendly and doesn't require complex document mutations.
**Status:** NOT STARTED

### 7. DbUuid Migration — 4-Phase Plan
**Source:** `2026-03-17--plan--dbuuid-migration.md`, `notes/2026-03-14--reference--dbuuid-phase3-remaining.md`
**What:** Eliminate all Uuid/Vec<u8>/BLOB boilerplate. 4 phases:
- ~~**Phase A** (highest ROI): `AccessContext.user_id: Uuid` → `DbUuid`~~ → **DONE** (PR #47, 20 files)
- ~~**Phase B**: `Path<Uuid>` → `Path<String>` in CRM route handlers~~ → **DONE** (PR #47, 71 handlers across 7 files)
- **Phase B remainder**: `Path<Uuid>` → `Path<String>` in non-CRM routes — ~345 sites across ~72 files
- **Phase C**: Migrate `users.id` BLOB → TEXT — eliminates ALL remaining `.as_bytes()` / `Vec<u8>` / `bind_uuid_blob` code
- **Phase D**: Batch convert remaining ~104 models `Uuid` → `DbUuid`
**Status:** Phases A+B (CRM) complete in PR #47. Phase B (non-CRM) and C/D remain.

### ~~8. Agent "View Profile" Link~~ → RESOLVED
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md` (item 7)
**Resolution:** PR #41 added agent profile page at `/agents/:agentId/profile` showing agent description, capabilities, status, default model, and autonomy level. AgentWatcherPanel agent names now link to the profile page. Route registered in App.tsx with ProtectedRoute wrapper.

### ~~8b. `any` Type Cleanup in New CRM/Pipeline Frontend Code~~ → RESOLVED
**Source:** PR #45 QA review
**Resolution:** PR #47 eliminated all 182 `: any` in targeted files. 107 remain in PR #46 org-profile files (not in scope). See `2026-03-18--plan--dev-velocity-sprint.md`.

### ~~8c. Sovereign Stack Volume Path Deduplication~~ → RESOLVED
**Source:** PR #45 QA review
**Resolution:** PR #47 extracted `resolve_volume_path()` to `crates/utils/src/volume.rs`. Both `org_cloud.rs` and `org_cloud_indexer.rs` now use the shared utility.

### 9. `http_request` Node Guardrails
**Source:** `archive/2026-03-12--tracker--sprint1-issues.md`
**What:** No OAuth/auth token management, no URL allowlisting, no rate limiting. Security concern for production.

### ~~10. Rename "Bug Triage Pipeline"~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #12)
**Resolution:** Renamed to "Feedback Triage Pipeline" in `data_source_workflows.rs`, `feedback.rs`, and E2E tests (2026-03-16).

### 11. ~~No Success Toast for Feedback Submission~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #4)
**Resolution:** FeedbackDialog now uses toast (commit `7cd4b22c9`)

### ~~12. Project Task Count Doesn't Auto-Refresh~~ → RESOLVED
**Source:** `archive/2026-03-13--review--dogfood-e2e-qa.md` (Bug #5)
**Resolution:** Fixed in PR #38 — sidebar task count refresh on task creation/update (2026-03-16).

---

## P2.5 — Code Quality Infrastructure

### Lint-Staged + Import Sort + Prettier Pre-Commit
**Source:** PR #49 wrap-up (2026-03-18)
**What:** Full setup for auto-formatting on every commit:
- `lint-staged` + `eslint --fix` + `prettier --write` on staged `.ts`/`.tsx` files
- `eslint-plugin-simple-import-sort` added (auto-sorts imports on `--fix`)
- `.githooks/pre-commit` hook with `git config core.hooksPath .githooks`
- `eslint --fix` already run across entire codebase (658 files, import reordering)
- `--max-warnings` threshold needs updating after import sort warnings cleared
**Stash:** `git stash list` → "lint-staged + import-sort setup + eslint --fix" on `refactor/frontend-polish-sprint`. Apply with `git stash pop` on the target branch.
**Status:** STASHED — ready to apply on a new branch

---

## P2.5 — Modularity Sprint 6 Candidates

### Modularity Sprint 6 — Hook Mutations + Remaining Debt
**Source:** `archive/2026-03-16--plan--modularity-sprint-5.md` (deferred 2e), PR #46 review
**What:** Remaining raw `useMutation` → `useMutationWithToast`:
- ~~`hooks/useOrgOnboarding.ts` (4 mutations)~~ → Done (PR #46)
- ~~`hooks/useWorkflowTemplates.ts` (1 mutation)~~ → Done (PR #46)
- ~~`hooks/useCrmActivities.ts` (2 mutations)~~ → Done (PR #46)
- `hooks/useAgentFlows.ts` (4 mutations)
- `hooks/useTaskMutations.ts` (3 mutations) — complex: async activity logging side effects
- `pages/oss-library-listener.tsx` (4 mutations)
- `pages/data-sources/RunWorkflowFromSourceDialog.tsx` `runMutation` — conditional toast logic (split from data-sources.tsx in PR #47)
Also: remaining conversation helper adoption (nora/voice, agent_chat, twilio), ~90 inline query keys (down from ~318)
- Over-invalidation in autonomy/bowser/collaboration mutations (broad `autonomyKeys.all` instead of targeted keys)
- `MembersTab` raw `fetch()` → `makeRequest` migration
- Query key string mismatches: `['brandProfile', orgId]` vs factory `['orgBrandProfile', orgId]`, `['workflowTemplates']` vs factory `['workflow-templates']` — need coordinated rename
**Source also:** `archive/2026-03-16--plan--modularity-sprint-4.md` (deferred items)
**Status:** PARTIALLY DONE (PR #46 converted 19 mutations, centralized ~228 keys, ~90 inline keys remain)

### Modularity — Large Frontend File Splits
**Source:** `archive/2026-03-15--plan--modularity-sprint-2.md` (remaining large files section)
**What:** Largest remaining frontend files:
- ~~`virtual-environment.tsx` (1,282 lines)~~ → Done (PR #47, split to `virtual-environment/`)
- `TaskFormDialog.tsx` (1,240 lines) — complex form, high-traffic
- `company-profile.tsx` (1,218 lines) — similar pattern to project-detail split
- ~~`MeetingMode.tsx` (1,207 lines)~~ → Done (PR #46, split to `meeting-mode/`)
- `CrmDealDetailPanel.tsx` (1,202 lines) — grew in PR #36
- ~~`data-sources.tsx` (991 lines)~~ → Done (PR #47, split to `data-sources/`)
**Status:** PARTIALLY DONE. PR #46 split 3 files, PR #47 split 2 more. 5 files >900 lines remain (8 including preserved originals from PR #46 pending deletion).

### Component Hook Extraction — Research Similar Patterns
**Source:** Frontend polish sprint (2026-03-18), task card refactor
**What:** Extracting shared hooks from task cards (`useResolvedAssignee`, `useResolvedAgent`, `useScrollIntoView`) + shared sub-components (`PriorityBadge`, `DueDateBadge`, `CollaboratorAvatars`) reduced TaskCard 396→259 lines and EnhancedTaskCard 541→428 lines while eliminating duplication. Research similar opportunities across the codebase:
- **CRM cards** (`CrmDealCard`, `CrmContactCard`) — likely duplicate assignee resolution, priority badges
- **Project cards** (`ProjectCard`) — may have inline member avatar logic that parallels `CollaboratorAvatars`
- **Detail panels** — `CrmDealDetailPanel`, `TaskDetailsPanel` likely duplicate the assignee IIFE pattern
- **Scroll-into-view** — search for `scrollIntoView` calls across components, consolidate to `useScrollIntoView`
- **Agent name resolution** — any component showing agent names should use `useResolvedAgent` instead of inline lookup
**Approach:** Audit with `grep -r "usersMap?.get\|scrollIntoView\|agentsMap" frontend/src/` to find candidates. Prioritize files >400 lines with inline data resolution patterns.
**Status:** NOT STARTED — research item for next modularity sprint

### Workflow UX — Deferred Polish
**Source:** `archive/2026-03-16--plan--workflow-ux-sprint.md` (deferred items + PR #44 review)
**What:**
- Extract `DataSourceWorkflowRunner` from `data-source-detail.tsx` (~730 lines) — Day 2 deferral
- ~~`CopyWorkflowDialog` NiceModal migration~~ → Already on NiceModal (PR #46 converted its mutation)
- ~~`window.confirm()` → shadcn `AlertDialog`~~ → All `window.confirm()` calls eliminated (PR #46 Day 1)
- Ownership filter toggle (My/Org/All) on workflow cards — badges exist but no filtering
**Status:** PARTIALLY DONE (PR #46 resolved confirm/mutation items)

### Modularity — Deferred Stretch Items
**Source:** `archive/2026-03-15--plan--modularity-sprint-2.md` (Day 5c)
**What:** Hook splits deferred (below priority threshold):
- `useConversationHistory.ts` (540 lines) → extract `flattenEntries`, `executionHelpers`, `patchWithKey`
- `useAutonomy.ts` (478 lines) → extract `useCheckpoints`, `useApprovalGates`
**Status:** NOT STARTED

### Modularity — formatRelativeDate Centralization
**Source:** PR #37 QA review
**What:** 3 independent `formatRelativeDate` implementations remain in EmailInbox, CommunicationsInbox, WorkflowRunsPanel. Different logic in each (today/yesterday vs "Xm ago" format). Could centralize with a configurable formatter.
**Status:** NOT STARTED — not a regression, just incomplete DRY

---

## P1 — PR #49 Review: Critical Deferred Issues

### ~~Webhook Route Behind `require_auth` Middleware~~ → RESOLVED
**Source:** PR #49 backend review (2026-03-18)
**Resolution:** Split `workflow_triggers::router()` into authenticated `router()` (CRUD) + public `public_router()` (webhook handler). Webhook endpoint now registered in `base_routes` (before `require_auth` layer). HMAC-SHA256 is the sole auth mechanism for webhooks.

---

## P2 — PR #49 Review: Medium Priority Deferred Issues

### Webhook Retry Logic is Dead Code
**Source:** PR #49 backend review (2026-03-18)
**File:** `crates/db/src/models/workflow_trigger.rs`
**What:** `max_retries`, `retry_count`, and `next_retry_at` fields are stored in the DB but never read or acted upon. No background worker checks `next_retry_at` to re-execute failed triggers.
**Recommendation:** Either (a) implement a retry worker in `spawn_workflow_schedule_loop` that checks for triggers past `next_retry_at` with `retry_count < max_retries`, or (b) remove the fields if retry isn't needed yet.
**Status:** NOT STARTED

### Webhook Cooldown Race Condition
**Source:** PR #49 backend review (2026-03-18)
**File:** `crates/db/src/models/workflow_trigger.rs` — `is_past_cooldown()`
**What:** Cooldown check reads `last_triggered_at`, then the caller updates it — but under concurrent requests, two webhooks could both pass the cooldown check before either updates the timestamp.
**Recommendation:** Use a database-level atomic check-and-update (e.g., `UPDATE ... WHERE last_triggered_at < ? RETURNING *`) or add a per-trigger mutex/advisory lock.
**Status:** NOT STARTED

### `webhook_url` Stored as Relative Path
**Source:** PR #49 backend review (2026-03-18)
**File:** `crates/db/src/models/workflow_trigger.rs`
**What:** `webhook_url` field stores a relative path (like `/webhooks/abc123`) rather than a full URL. Without the server hostname, the stored URL is not useful for display or documentation.
**Recommendation:** Either store the full URL (constructed from `HOST`/`BACKEND_PORT` at creation time) or construct it dynamically in the API response.
**Status:** NOT STARTED — low impact

### `useResolvedAgent` O(n) Scan Per Card
**Source:** PR #49 frontend review (2026-03-18)
**File:** `frontend/src/components/tasks/task-card-parts/useResolvedAgent.ts`
**What:** When `agentsMap` doesn't have a direct hit, the hook does `Array.from(agentsMap.values()).find()` — O(n) per card per render. With many agents and cards, this adds up.
**Recommendation:** Build a secondary `Map<string, Agent>` keyed by `short_name` at the page level (alongside `agentsMap`), pass it as an optional prop. Falls back to O(n) scan only if secondary map not provided.
**Status:** NOT STARTED — low impact unless agent count grows

### ~~WorkflowTriggersPanel Toggle Toast Not Descriptive~~ → RESOLVED
**Resolution:** Changed `successMessage` to `(result) => \`Trigger ${result.enabled ? 'enabled' : 'disabled'}\``.

### ~~`useBranchLoader` Swallows Errors Silently~~ → RESOLVED
**Resolution:** Added `console.error` to empty catch block.

### ~~`useTopsiVoice` Exports Unused Functions~~ → RESOLVED
**Resolution:** Removed `startRecording`/`stopRecording` from `TopsiVoiceActions` interface and return object. Functions remain internal for use by `handlePushToTalkStart`.

### ~~`useMoveDeal` Missing Error Toast~~ → RESOLVED
**Resolution:** Added `toast.error('Failed to move deal')` + `console.error` in `onError` handler. Kept raw `useMutation` for optimistic update pattern.

### `workflowKeys` Mixed Naming Convention
**Source:** PR #49 frontend review (2026-03-18)
**File:** `frontend/src/lib/query-keys.ts`
**What:** `workflowKeys` uses mixed casing — some keys use camelCase, others use kebab-case strings internally. Inconsistent but not buggy (keys match between factory and usage).
**Recommendation:** Standardize to camelCase in a future query key sweep.
**Status:** NOT STARTED — cosmetic

### NetworkSettings Wasted API Call
**Source:** PR #49 frontend review (2026-03-18)
**File:** `frontend/src/pages/settings/NetworkSettings.tsx`
**What:** Fetches `/api/mesh/stats` on mount even when the mesh stats panel isn't visible.
**Recommendation:** Gate the query with `enabled: isPanelExpanded` or lazy-load the stats section.
**Status:** NOT STARTED — low impact

---

## P2.7 — E2E Test Infrastructure Issues

### E2E: Removed Assertions Without Replacement (PR #49)
**Source:** PR #49 e2e review (2026-03-18)
**Files:** `e2e/demos/bug-report-lifecycle.spec.ts`, `e2e/demos/manual-qa-trigger.spec.ts`
**What:** PR removed `waitForToast` assertions for task state transitions and `page.reload()` before watcher badge assertions. These were removed to fix test failures, but no equivalent assertions replaced them — the tests now skip verification of those behaviors.
**Recommendation:** Re-add assertions using a more resilient pattern (e.g., `page.waitForSelector` for toast elements, or poll-based checks for badge appearance without hard reload).
**Status:** NOT STARTED — reduces test coverage

### ~~E2E: `DEMO_PR_NUMBER` Used Without Null Guard~~ → RESOLVED
**Resolution:** Added `test.skip(!DEMO_PR_NUMBER, "Skipping — Step 4 did not create a PR")` guard at top of Step 6.

### E2E: Conditional Visual Checks Are No-Op
**Source:** PR #49 e2e review (2026-03-18)
**Files:** Multiple demo specs
**What:** Several visual checks are wrapped in `if (element) { expect... }` — if the element isn't found, the assertion silently passes. These are no-op assertions that give false confidence.
**Recommendation:** Either make the assertions unconditional (fail if element missing) or use `test.fixme()` to mark as known incomplete.
**Status:** NOT STARTED — reduces test value

### E2E: Fragile Title-Based Button Selectors
**Source:** PR #49 e2e review (2026-03-18)
**Files:** Workflow demo specs
**What:** Button selectors use `[title="..."]` which breaks if button text/title changes. Also `addExtractNode` matches placeholder strings instead of `data-testid`.
**Recommendation:** Add `data-testid` attributes to workflow editor buttons and use those in selectors.
**Status:** NOT STARTED — fragile but functional

### E2E: `timing.ts` Pace Detection Doesn't Detect `demos` Project
**Source:** PR #49 e2e review (2026-03-18)
**File:** `e2e/helpers/timing.ts`
**What:** Pace detection logic checks for `--project=` arg but may not correctly identify the `demos` project, causing demo tests to run at default (fast) pace instead of demo pace.
**Recommendation:** Verify pace detection logic handles `--project=demos` correctly. Add a test or log output.
**Status:** NOT STARTED

### E2E: GITHUB_TOKEN Required for Agent Simulation Tests
**Source:** Frontend polish sprint QA (2026-03-18)
**Tests affected:** `bug-report-lifecycle.spec.ts` Step 4, `manual-qa-trigger.spec.ts` Step 3
**What:** `simulateDevAgentWork()` and `createPrForTask()` in `e2e/helpers/demo/simulation.ts` call `ghApi()` which requires `GITHUB_TOKEN` env var. Tests fail immediately with "GITHUB_TOKEN env var required for demo simulation".
**Recommendation:** Either: (a) set `GITHUB_TOKEN` in `.env.test` for CI, (b) add mock mode to `simulation.ts` that fakes GitHub API responses for local testing, or (c) skip these steps gracefully with `test.skip(!process.env.GITHUB_TOKEN, "GITHUB_TOKEN required")`.
**Status:** NOT STARTED

### E2E: Pipeline Intelligence Expects 8 Stages, Seed Has 7
**Source:** Frontend polish sprint QA (2026-03-18)
**Tests affected:** `pipeline-intelligence-workflow.spec.ts` Part 1
**What:** Test asserts `stageList.length >= 8` but seed DB only provides 7 pipeline stages.
**Recommendation:** Either: (a) update seed DB to include all 8 expected stages, or (b) update test to match actual seed data (7 stages), or (c) add a test setup step that creates the missing stage via API.
**Status:** NOT STARTED

### E2E: Workflow CRM/Spanish Pipeline Demos Need LLM Backend
**Source:** Frontend polish sprint QA (2026-03-18)
**Tests affected:** `workflow-crm-pipeline.spec.ts` Part 3+, `workflow-spanish-pipeline.spec.ts` Part 3+
**What:** Workflow execution requires PCG Router (LLM backend) to produce staged records. Parts 1-2 (build workflow + create data source) pass, but Part 3+ (run workflow, verify staged output) times out without an active LLM backend.
**Recommendation:** These tests are integration-level and require full stack. Tag with `@requires-llm` annotation and skip in CI unless LLM backend is available. Add `test.skip(!process.env.LLM_BACKEND_URL, "LLM backend required")` guard.
**Status:** NOT STARTED

---

## P3 — Developer Experience & Testing Infrastructure

### Error Messages Should Include Actionable Steps (Role-Scoped)
**Source:** Frontend polish sprint QA (2026-03-18) — user feedback
**What:** When operations fail (e.g., workflow run, API calls), error messages should:
1. Include actionable steps the user can take to fix the issue
2. Scope detail level by user role: platform-level users see system details (ports, services, config), end users see only user-facing guidance without platform internals
3. Never leak infrastructure details (server ports, internal service names, file paths) to non-platform users
**Examples:**
- Workflow run failure → Platform user: "Workflow execution failed: LLM backend not reachable at PCG Router. Check service status or configure alternative model." End user: "Workflow could not complete. Please try again or contact your administrator."
- Missing config → Platform user: "GITHUB_TOKEN not configured. Set in .env or environment." End user: "Integration not configured. Contact your administrator."
**Recommendation:** Create an `AppError` utility that accepts error detail + user role, returns appropriate message. Add `isSystemError` flag for errors that should only show technical details to admins.
**Status:** NOT STARTED

### E2E: Missing Env Vars Should Warn, Not Silently Fail
**Source:** Frontend polish sprint QA (2026-03-18) — user feedback
**What:** When essential env vars (GITHUB_TOKEN, LLM_BACKEND_URL, etc.) are missing, tests throw cryptic errors deep in execution instead of warning upfront. The `simulation.ts` helper throws at line 15 but only after 3 prior steps pass, wasting test time.
**Recommendation:** Add a pre-flight env check to `e2e/helpers/index.ts` or `playwright.config.ts` globalSetup that logs warnings for optional env vars and fails fast for required ones. Pattern: `console.warn("⚠️ GITHUB_TOKEN not set — agent simulation tests will be skipped")`.
**Status:** NOT STARTED

### 13. Onboarding Dialog Bypass for Test Environments
**Source:** PR #47 smoke testing (2026-03-18)
**What:** Fresh sessions trigger 4 sequential modals (safety notice → agent/editor config → GitHub connect → feedback opt-in) before the app is usable. Blocks all automated E2E and Playwright smoke testing.
**Rationale:** Every smoke test run requires manually dismissing 4 dialogs. This makes automated QA impractical and wastes ~30s per test session. Critical for CI/CD pipeline.
**Proposal:** Persist onboarding completion in `users` table (e.g., `onboarding_completed_at`). Skip for admin users. Add `SKIP_ONBOARDING=1` env var for test environments. Alternatively, add a `?skip_onboarding=1` URL param that sets a session flag.
**Status:** NOT STARTED

### 14. SSE Connections Should Not Fire Before Authentication
**Source:** PR #47 smoke testing (2026-03-18)
**What:** `useAgentDirectory` SSE connection (`/api/events/agent-directory`) fires on the login page before the user is authenticated, causing console errors (`The connection to ... was interrupted`).
**Rationale:** Unnecessary network requests on unauthenticated pages. Creates noisy console errors that obscure real issues during debugging. The SSE hook should check auth state before connecting.
**Proposal:** Guard SSE hooks with `isAuthenticated` check from AuthContext. Only establish SSE connections after successful login.
**Status:** NOT STARTED

### 15. Seed Database Refresh for Smoke Testing
**Source:** PR #47 smoke testing (2026-03-18)
**What:** Seed DB (`dev_assets_seed/duck_kanban.db`) lacks CRM deals, communications, and org cloud data. Smoke testing these features requires creating data manually via the UI or API, which is slow and fragile.
**Rationale:** Every QA pass on CRM/comms features starts from zero. A richer seed with sample pipelines, deals, contacts, and comms would make smoke testing 10x faster.
**Proposal:** Add seed data generation script or extend existing seed with: 1 pipeline per org with 3-5 deals across stages, 5-10 CRM contacts, sample call/SMS records. Run as part of `flox activate` or `npm run seed:dev`.
**Status:** NOT STARTED

## P3.5 — CRM UX Polish

### 16. CRM Deal Card — Explain "Research Needed" Badge
**Source:** PR #47 smoke testing (2026-03-18)
**What:** New deals immediately show an orange "Research needed" badge and "5%" confidence score with no explanation of what triggers research or what the percentage represents.
**Rationale:** Users creating simple deals to track prospects don't expect AI research status. The badge implies action is needed but provides no path to take that action. Confidence score has no tooltip or explanation.
**Proposal:** Add tooltip on hover explaining the badge ("AI research has not been run for this deal. Click to start."). Show confidence explanation ("Based on available data about this prospect"). Consider making research opt-in rather than defaulting to "needed".
**Status:** NOT STARTED

### 17. Pipeline Aggregate Should Show Currency
**Source:** PR #47 smoke testing (2026-03-18)
**What:** Pipeline header shows "$50,000" but the deal form supports multiple currencies (USD, EUR, etc.). If deals have mixed currencies, the aggregate sum would be misleading.
**Rationale:** Financial data accuracy is critical for CRM. Mixing currencies in a sum is a data integrity issue.
**Proposal:** Either: (a) normalize all values to pipeline's base currency with conversion, (b) show aggregate per currency, or (c) only show aggregate when all deals share the same currency — otherwise show "Mixed currencies".
**Status:** NOT STARTED

---

## Active Branch Conflict Notes (2026-03-18)

### `sloperation316-pipeline-progress` — Updated from main (2026-03-18)
PR #45 merged `integration/sloperation316` (a separate integration branch) — NOT this branch. Pipeline-progress has 77 files of unmerged work (CRM pipeline automation, company profiles, brand guides, e2e tests).
**Status**: Merged `origin/main` — no conflicts. Ready to merge to main when pipeline features are approved.

### `sloperation316-vibe-integration` — Updated from main (2026-03-18)
Superset of pipeline-progress (Dockerfile fix + VIBE tokenomics plan). Merged `origin/main` with 17 conflicts resolved:
- `company.rs`: kept main's `hex()` UUID lookup
- `crm_deals.rs`: kept vibe's `call_llm()`, Astra Pass 2, `generate_*_core()` pattern
- `App.tsx`: kept `org_viewer` for `/people` routes (consistent with CRM)
- Frontend: kept vibe's pipeline features, fixed `personsApi.get()` and `tasksApi.getAll()` method names
**Note**: If PR #46 merges to main before these branches, they'll need another merge from main to pick up query key factories, mutation conversions, and file splits.

### `refactor/dev-velocity-sprint` — PR #47 (open)
Dev velocity sprint: DbUuid Phases A+B, 136 unwrap eliminations, 182 any type removals, 31 route handlers secured, 2 file splits, sovereign hardening. Rebased on main post-PR #46. See `2026-03-18--plan--dev-velocity-sprint.md`.

### Pre-existing DB/Server Issues (found during e2e setup)
1. **Seed DB BLOB→TEXT**: `users` table still has BLOB UUIDs. Runtime fix: `UPDATE users SET id = lower(substr(hex(id),...))`. Seed needs regeneration.
2. **Onboarding warning**: `table projects has no column named slug` — `user_onboarding.rs` references a column missing from seed schema. Non-blocking.
3. **Migration checksum mismatch**: `20260409000000_org_cloud.sql` was modified after being applied to seed. Fresh seed + `sqlx migrate run` required.

---

## P3 — Low Priority / Future Sprints

### 13. Schema Improvements for Dogfood Pipeline
**Source:** `archive/2026-03-13--plan--dogfood-pipeline-activation.md`
- Add `qa` status to distinguish QA review from human approval on kanban
- Add `deployed` status to distinguish "code merged" from "manually done"
- Add `task_id` FK on `merges` table for direct task→PR lookup
- Add `task_status_history` table for full audit trail

### 14. Sprint 2-3 Roadmap Items
**Source:** `archive/2026-03-12--plan--three-sprint-roadmap.md`
- Convert hardcoded automations to editable system workflows
- ~~Bulk task operations (select multiple, batch status change)~~ → Done (PR #38)
- Progress indicator for agent execution on task cards
- Undo reject / re-open task workflow

### 15. Known Limitations (Design Choices)
- Conditional node: only simple string matching + `count>N`
- CRM update nodes: match by email/name only (no fuzzy matching)
- ACP MCP loading: path depends on server working directory
- Task FTS search index: auto-sync triggers dropped — needs app-level reindexing

### 16. UI Polish — Future Improvements
- Task card inline editing
- Keyboard shortcuts for common actions
- Board column WIP limits
- Workflow versioning/history
- Mobile user menu shortcut (avatar in navbar)
- View-as keyboard shortcut (Cmd+Shift+V)
- Sidebar section animated transitions
- ~~Settings tab URL persistence~~ → Done (PR #39, `?scope=` param)

### 18. UX Audit — Remaining Items (from `2026-03-16--review--ux-design-audit.md`)
**P2**: Topsi chat suggested prompts (#14)
~~Login "session expired" on first visit (#16)~~ → Already fixed (PR #38, `hadPriorSession` localStorage check)
~~Dev banner space reduction (#17)~~ → Fixed (PR #41, inline pill in navbar header)

### 19. UX Audit — Partially Addressed (depth improvements)
**Source:** `archive/2026-03-16--review--ux-design-audit.md`
- Pipeline: stage visual differentiation (gradient backgrounds), agent action tooltip improvements (#2)
- ~~Notifications: tabs (All/Unread/Mentions), "Mark all read" button, full notifications page (#4)~~ → RESOLVED (PR #41)
- KPI trends: trend arrows with percentages (requires API), time period selector, sparklines (#10)
- Project cards: last activity timestamp, assignee avatars (#18)
- Task card data population: seed data needs richer metadata (assignees, due dates, tags) (#3)
- Breadcrumbs: "Jungleverse" still appears as default org name in some views (#15)

### 20. User Account Onboarding (deferred from UX Sprint 2)
**Source:** `archive/2026-03-16--plan--ux-engagement-polish-sprint2.md`
**What:** First-login walkthrough — profile setup, preferences, Topsi intro. Org onboarding (PR #39) covers org-level setup; user account onboarding covers individual user first-run experience.
**Status:** NOT STARTED

### 17. React Router v7 Migration Warnings
**Source:** `archive/2026-03-12--review--sprint1-qa-results.md`
- Future flag deprecation warnings in console
- `validateDOMNesting` warning (button nested in button in workflow editor)

---

## TODO — Manual Verification

- [ ] Edit Task / Feedback / Project Form dialogs: verify sticky footer
- [ ] ResizableDrawer: drag resize, expand/collapse, click-outside-to-close
- [ ] Fullscreen BreadcrumbNav visible, Esc to close
- [ ] Start dev server → Settings > Agents → agents load (no 500)
- [ ] Create Task → Agent picker → agents listed
- [ ] Task detail → Agent Watcher panel → no 500 errors
- [ ] Run E2E demos headed: `FRONTEND_PORT=3001 npx playwright test --project=demos`

---

## Resolved (Removed from Backlog)

| Item | Resolution |
|------|-----------|
| BLOB UUID agent endpoints (QA #1, #2, #7, #8) | DbUuid Phase 1-2 (commit `2cf77c82b`) |
| Edit Task modal not scrollable (QA #14) | FormDialogBody helper (commit `e72d80187`) |
| Task detail no back navigation (QA #16) | ResizableDrawer + BreadcrumbNav (commits `761bdb609`–`fad6c9fd7`) |
| QA watcher refactor (Phase 6) | Complete (PR #23) |
| Topsi-Workflow Bridge | Complete (PR #22) |
| Dogfood Pipeline phases 7-11 | Complete |
| Task activity logging (Bug #9) | DbUuid migration + ActivityLog calls (commit `bb2e8dfb7`, `ace91ffcb`) |
| Feedback tasks no dev agent (Bug #10) | Topsi assignment in feedback.rs (commit `49363911c`) |
| Manual InReview QA trigger (Bug #11) | Watcher trigger in update_task (commit `ace91ffcb`) |
| Intelligence Overview 0 sources (Bug #6) | find_by_organization query (commit `49363911c`) |
| send_notification node + in-app notifications | Full stack: migration, model, routes, frontend bell (commit `bc07ba53e`) |
| Workflow trigger hardening | Tracing spans + CancellationToken (commit `5165b4091`) |
| Status dropdown in task detail (Backlog #10) | Combobox in task detail header (commit `3b40cf272`) |
| Loading skeletons (Backlog #11) | Skeleton components for My Tasks (commit `7d2e910f1`) |
| BLOB binding for project creation | bind_uuid_blob helpers (commit `e6f34f140`) |
| Drawer click-outside closes on portaled overlays | Radix overlay detection (commit `cb31ba595`) |
| E2E demo script duplication + headed mode | Dedicated fixtures + shared helpers (commit `55d24889a`) |
| PR #27 C1: notifications BLOB/TEXT mismatch | BLOB bytes bind for project_members query (commit `dd3f20388`) |
| PR #27 C2: bind_uuid_blob panic | Returns Result (commit `1b65c73a1`) |
| PR #27 C3: fragile GitHub URL parsing | url::Url parser (commit `dd3f20388`) |
| PR #27 C4: contentFullscreen stuck in localStorage | partialize excludes it (commit `1b65c73a1`) |
| PR #27 C5: status dropdown cache miss | queryClient.invalidateQueries (commit `1b65c73a1`) |
| PR #27 W3: Notification/ActivityLog String→DbUuid | Consistency fix (commit `dd3f20388`) |
| PR #27 W4: feedback agent arbitrary selection | Agent::find_default_assignee (commit `dd3f20388`) |
| PR #27 W8: FeedbackDialog nested JSX | Success → toast notification (commit `7cd4b22c9`) |
| PR #27 W14: DbUuid encoding undocumented | Module docs expanded (commit `dd3f20388`) |
| No success toast for feedback submission (Bug #4) | Toast via sonner (commit `7cd4b22c9`) |
| PR #27 W1: manual InReview doesn't spawn QA review | spawn_watcher_reviews() standalone (commit `f03dab011`) |
| Notification UUIDs instead of display names | Backend LEFT JOIN for actor_name (2026-03-15) |
| Notification icons not visually distinct | Colored icon circles + actor type badges (2026-03-15) |
| Run Workflow button has no loading indicator | Added Loader2 spinner to data-sources.tsx (2026-03-15) |
| Workflow CRM demo incomplete (no commit/verify) | Extended with Parts 5-8: approve, runs, contacts, pipeline (2026-03-15) |
| E2E demo test run (Backlog #5) | Health checks 40/40, demos 28/31, structured test commands (PR #34) |
| Tech debt sprint: .unwrap() elimination | 40 unwraps → proper error handling across 8 files (PR #34) |
| Tech debt sprint: api.ts monolith | 6,669 lines → 21 domain modules (PR #34) |
| Tech debt sprint: org-profile.tsx monolith | 7,086 lines → 33 modular files with lazy-loaded tabs (PR #34) |
| Tech debt sprint: CI pipeline | fmt + clippy + test + lint + types + audit (PR #34) |
| Tech debt sprint: eprintln in prod code | 4 calls → 0, replaced with tracing (PR #34) |
| Modularity Sprint 2: formatters centralization | `formatDate`/`formatCurrency`/`formatCompactNumber`/`parseJsonArray` → `lib/formatters.ts`, 20 duplicates eliminated (PR #37) |
| Modularity Sprint 2: EmptyState adoption | Component adopted in 7 files, 13 inline patterns replaced (PR #37) |
| Modularity Sprint 2: raw fetch() consolidation | 10 consumers migrated to typed API client modules (PR #37) |
| Modularity Sprint 2: nora/tools.rs split | 7,347 lines → 8 modules in `tools/` directory (PR #37) |
| Modularity Sprint 2: workflows.tsx split | 2,418 lines → 9 files in `pages/workflows/` (PR #37) |
| Modularity Sprint 2: WorkflowEditor.tsx split | 2,240 lines → 7 files in `editor/` with barrel re-export (PR #37) |
| Modularity Sprint 2: project-detail.tsx split | 2,293 lines → 9 files in `project-detail/` (PR #37) |
| Modularity Sprint 2: task_server.rs split | 3,457 lines → 9 files in `task_server/` (PR #37) |
| Modularity Sprint 2: brand-guide.tsx split | 1,703 lines → 17 files in `brand-guide/` (PR #37) |
| Modularity Sprint 2: shared query hooks | `useProjectList`, `useOrganizationById`, `useCrmContacts` extracted (PR #37) |
| ACP agents get no MCP servers (Backlog #2) | Already implemented — `load_platform_mcp_servers()` + `default_mcp.json` (2026-03-16) |
| Rename Bug Triage Pipeline (Backlog #10) | → "Feedback Triage Pipeline" in code + E2E tests (2026-03-16) |
| Pipeline enhancement merge + DbUuid fixes | sloperation314 merged, 12 Rust + 4 TS compile fixes (2026-03-16) |
| Pipeline intelligence E2E demo | 5-part 8-stage deal progression demo (2026-03-16) |
| UX Sprint 2: Org-scoped onboarding system | New DB tables, API module, React Query hook, org overview integration (PR #39) |
| UX Sprint 2: Notification quick actions | Source-aware action buttons, dismiss, improved deep-linking (PR #39) |
| UX Sprint 2: Dark mode hardcoded color fixes | CSS `--brand-gold` variable, OnboardingCarousel dark variants (PR #39) |
| UX Sprint 2: KPI trend indicators | TrendIndicator + workflow run stats on org overview (PR #39) |
| UX Sprint 2: Project card progress bar | Task completion Progress bar on ProjectCard (PR #39) |
| UX Sprint 2: Workflow last-run status | Status indicator on workflow definition cards (PR #39) |
| UX Sprint 2: Pipeline empty state | Onboarding prompt card in empty pipeline first column (PR #39) |
| UX Sprint 2: Settings "Coming Soon" cleanup | Planned items grouped at bottom with reduced opacity (PR #39) |
| Task.collaborators missing from struct | Field existed in SQL + DB but not in Task struct — SQLx silently discarded (PR #39) |
| Notification quick action buttons (Backlog P1.5) | Implemented: source-aware actions, dismiss, deep-linking (PR #39) |
| Project task count auto-refresh (Backlog #12) | Sidebar task count refresh on create/update (PR #38) |
| UX Sprint 3: Sidebar merged sections (Audit #5) | Management + Global Views → "Views & Management", "More" popover for external links (PR #41) |
| UX Sprint 3: Drawer wider default (Audit #6) | Default width 600→800px in resizable-drawer.tsx (PR #41) |
| UX Sprint 3: Notification tabs + full page (Audit #4) | All/Inbox/Activity tabs in dropdown, `/notifications` page with search/filter (PR #41) |
| UX Sprint 3: Integrations hub redesign (Audit #9) | Recommended/Optional split, progress ring, muted gray for missing optional (PR #41) |
| UX Sprint 3: Task card priority borders (Audit #3) | 3px left-border colored by priority level (PR #41) |
| UX Sprint 3: Hide test tasks toggle (Audit #8) | "Hide Tests" button on kanban filters `[E2E]`/`[Test]` prefix tasks (PR #41) |
| UX Sprint 3: CRM sub-nav dedup (Audit #19) | CRM removed from project sub-nav in ProjectFolder.tsx (PR #41) |
| AgentWatcherPanel error state (Backlog #6) | Error UI with retry button, distinguishes empty vs error (PR #41) |
| Agent profile page (Backlog #8) | `/agents/:agentId/profile` page with capabilities, status, model (PR #41) |
| Route conflict fix (pre-existing) | Merged duplicate routes in org_onboarding.rs (committed to main) |
| Migration version collision fix (pre-existing) | Renamed 20260405000000→20260405000001 to avoid collision (committed to main) |
| Modularity Sprint 5: VIBE billing duplication | `ensure_vibe_balance` + `record_llm_vibe_usage` in `helpers/billing.rs`, 5 blocks eliminated (PR #43) |
| Modularity Sprint 5: topsi raw fetch() | `topsi.ts` API module wrapping 7 endpoints + `topsiKeys` factory (PR #43) |
| Modularity Sprint 5: topiclips raw fetch() | `topiclips.ts` API module wrapping 7 endpoints + `topiclipsKeys` factory (PR #43) |
| Modularity Sprint 5: twilio.rs monolith | 2285 lines → 8-file `twilio/` directory module (PR #43) |
| Modularity Sprint 5: task_attempts.rs monolith | 1909 lines → 6-file `task_attempts/` directory module (PR #43) |
| Modularity Sprint 5: nora/mod.rs decomposition | 1172→718 lines, extracted config.rs, rate_limiter.rs, initialization.rs (PR #43) |
| Modularity Sprint 5: conversation persistence | `helpers/conversations.rs` with `persist_chat_exchange`, adopted in topsi + nora chat (PR #43) |
| Modularity Sprint 5: query key migration (8 files) | ~43 inline keys → factories, new `socialKeys`/`networkKeys` factories (PR #43) |
| Workflow UX: Inline staging review | `RunAndReviewPanel` composite, `RunWorkflowDialog` `onRunComplete` callback (PR #44) |
| Workflow UX: Shared workflow card grid | `WorkflowCardGrid` with ownership badges, used in BuilderTab + WorkflowsView (PR #44) |
| Workflow UX: Copy/promote workflows | `CopyWorkflowDialog` for user ↔ org workflow copying (PR #44) |
| Workflow UX: Node picker search + categories | Search/filter, category headers with counts (PR #44) |
| Workflow UX: Workflow ID auto-generation | ID derived from name with lock/unlock toggle (PR #44) |
| Workflow UX: Graph view zoom + selection | Zoom controls, node selection highlight, connection validation (PR #44) |
| Workflow UX: Prompt templates + output schema | 3 starter templates, schema Select dropdown (PR #44) |
| Workflow UX: Sidebar workflow sub-links | Builder/Runs/Staging sub-links with pending count badge (PR #44) |
| Workflow UX: Real-time updates | AgentWatcher → React Query 10s polling, staging `refetchInterval` (PR #44) |
| Workflow UX: Deleted source display | "(Deleted source)" instead of truncated UUIDs in runs (PR #44) |
| Workflow UX: `any` type cleanup + code quality | `WorkflowRunResult` interface, `cn()`, `useEffect` fixes (PR #44) |
| Modularity Sprint 4: billing helper deferred | Done in Sprint 5 — `helpers/billing.rs` (PR #43) |

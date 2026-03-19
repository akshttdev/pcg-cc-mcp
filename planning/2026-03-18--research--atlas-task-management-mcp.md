# ATLAS Task Management MCP — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, tools, and techniques from the ATLAS MCP server (and related task-orchestrator MCP) applicable to our ORCHA agent orchestration system and task management processes.

---

## What is ATLAS?

**ATLAS** (Adaptive Task & Logic Automation System) is a [Neo4j-powered MCP server](https://github.com/cyanheads/atlas-mcp-server) that gives LLM agents full project management capabilities through a three-tier architecture: **Projects → Tasks → Knowledge**. Built in TypeScript, it exposes structured tools via the Model Context Protocol for creating, tracking, and managing complex workflows with dependency graphs and knowledge enrichment.

The key innovation: ATLAS treats task management as a **graph problem**, not a flat list. Dependencies, knowledge associations, and project hierarchies are all first-class relationships in Neo4j, enabling queries like "what tasks are blocked by this one?" or "what knowledge is related to this project phase?"

---

## Three-Tier Architecture

### 1. Projects (Top-Level Containers)

```
Project Node:
  id, name, description, status
  urls[]              — {title, url} pairs for external references
  completionRequirements  — what "done" looks like
  outputFormat        — expected deliverable format
  taskType            — categorization
  dependencies[]      — inter-project dependency edges
  createdAt, updatedAt
```

**Key insight:** `completionRequirements` and `outputFormat` are first-class fields, not afterthoughts. Every project explicitly defines its success criteria and expected deliverable shape *at creation time*.

### 2. Tasks (Actionable Work Items)

```
Task Node:
  id, projectId, title, description
  priority, status, assignedTo
  urls[], tags[]
  completionRequirements  — task-level done criteria
  outputFormat        — expected output shape
  taskType            — categorization
  dependencies[]      — inter-task dependency edges
  createdAt, updatedAt
```

Tasks inherit project context but have their own completion criteria. Dependencies create a directed acyclic graph (DAG) with automatic circular dependency detection.

### 3. Knowledge (Structured Information)

```
Knowledge Node:
  id, projectId, text
  tags[], domain       — categorization
  citations[]          — source attribution
  createdAt, updatedAt
```

Knowledge items attach to both projects and tasks, providing context and evidence. Citations enable **source tracking** — agents can trace where information came from.

**Key insight:** Knowledge is not just notes — it's a structured, searchable, citable layer that agents can query independently of the task that created it. This prevents the "genius with amnesia" problem where agents lose context between sessions.

---

## Neo4j Graph Model

ATLAS uses Neo4j for three critical reasons:

1. **Native relationship modeling** — Dependencies, knowledge associations, and project hierarchies are graph edges, not foreign keys in flat tables
2. **Graph traversal queries** — "Find all tasks blocked by task X" is a single Cypher query, not a recursive join
3. **ACID transactions** — Data integrity across multi-entity operations

### Relationship Types

- `Project → CONTAINS → Task` (project scoping)
- `Task → DEPENDS_ON → Task` (execution ordering)
- `Project → DEPENDS_ON → Project` (inter-project dependencies)
- `Project → HAS_KNOWLEDGE → Knowledge` (project context)
- `Task → HAS_KNOWLEDGE → Knowledge` (task-specific context)

### Dependency Validation

On every create/update:
1. Validate referenced entities exist
2. **Circular dependency detection** — prevents DAG violations
3. Status transition validation — blocked tasks can't proceed until dependencies resolve

---

## MCP Tool Interface

### Project Operations

| Tool | Modes | Key Feature |
|------|-------|-------------|
| `atlas_project_create` | single/bulk | Client-generated IDs optional; supports `completionRequirements` + `outputFormat` |
| `atlas_project_list` | all/details | Include related tasks + knowledge; filter by status/taskType |
| `atlas_project_update` | single/bulk | Selective field updates via `updates` object |
| `atlas_project_delete` | single/bulk | Cascading deletion of child tasks + knowledge |

### Task Operations

| Tool | Modes | Key Feature |
|------|-------|-------------|
| `atlas_task_create` | single/bulk | `dependencies[]` with circular detection; `assignedTo` for agent routing |
| `atlas_task_list` | — | Filter by status/assignedTo/priority/tags/taskType; sort + paginate |
| `atlas_task_update` | single/bulk | Status transitions with dependency validation |
| `atlas_task_delete` | single/bulk | Cascading knowledge removal |

### Knowledge Operations

| Tool | Modes | Key Feature |
|------|-------|-------------|
| `atlas_knowledge_add` | single/bulk | Domain categorization + citations for source tracking |
| `atlas_knowledge_list` | — | Full-text search + tag/domain filtering |
| `atlas_knowledge_delete` | single/bulk | Bulk cleanup |

### Search & Research

| Tool | Key Feature |
|------|-------------|
| `atlas_unified_search` | Cross-entity regex/Lucene search with fuzzy matching; filters by entityType/taskType/assignedTo |
| `atlas_deep_research` | Creates hierarchical research plan with sub-topics → auto-generates tasks; accepts `subTopics[]` with questions, search queries, priority, assignedTo |

### Database Operations

| Tool | Key Feature |
|------|-------------|
| `atlas_database_clean` | Destructive reset requiring explicit `acknowledgement: true` |

**Design pattern:** All tools support `responseFormat: 'formatted' | 'json'` and `mode: 'single' | 'bulk'`. This dual-mode pattern avoids tool proliferation — one tool handles both single and batch operations.

---

## MCP Resource Endpoints

ATLAS exposes structured resources for direct data access:

```
atlas://projects                        — paginated project listing
atlas://projects/{projectId}            — single project
atlas://projects/{projectId}/tasks      — project-scoped tasks
atlas://projects/{projectId}/knowledge  — project-scoped knowledge
atlas://tasks/{taskId}                  — single task
atlas://knowledge/{knowledgeId}         — single knowledge item
```

**Key insight:** Resources provide read-only access separate from tools. Agents can browse project state without invoking mutation tools.

---

## Deep Research Feature

`atlas_deep_research` is a structured planning tool that:

1. Accepts a research topic, goal, scope definition
2. Takes an array of `subTopics`, each with:
   - `question` (required) — the research question
   - `initialSearchQueries[]` — suggested search terms
   - `priority`, `assignedTo` — for routing to specific agents
   - `initialStatus` (default: 'todo')
3. Creates a hierarchical plan within the knowledge base
4. Optionally auto-generates tasks (`createTasks: true` default)
5. Outputs a markdown summary + raw JSON export

**Application to ORCHA:** This pattern — structured research decomposition that auto-generates both knowledge artifacts and actionable tasks — maps directly to our workflow definition process. A "research" workflow step could decompose a topic into sub-questions, each becoming a task with assigned agent and priority.

---

## Atlas as Plan Executor (oh-my-opencode Pattern)

A separate implementation ([oh-my-opencode](https://deepwiki.com/code-yeongyu/oh-my-opencode/4.3-atlas-hook-and-agent)) uses Atlas as a **plan execution orchestrator** with sophisticated patterns:

### Separation of Concerns

> "Atlas **thinks**, workers **do**."

Atlas is explicitly blocked from write operations. It can only:
- Read files, search patterns, run diagnostics
- Delegate to worker agents via structured briefings
- Accumulate learnings across tasks

This enforces clean separation between planning/coordination and execution.

### Six-Section Delegation Structure

Every task delegation must include:

1. **TASK** — Atomic, specific goal (single action per delegation)
2. **EXPECTED OUTCOME** — Concrete deliverables with success criteria
3. **REQUIRED TOOLS** — Explicit whitelist (prevents tool sprawl)
4. **MUST DO** — Exhaustive requirements (nothing left implicit)
5. **MUST NOT DO** — Forbidden actions (anticipate rogue behavior)
6. **CONTEXT** — File paths, existing patterns, constraints

**Application to ORCHA:** This structured delegation template prevents ambiguous task assignments. Every dispatched task has explicit success criteria, tool constraints, and anti-patterns.

### Wisdom Accumulation System

Atlas maintains persistent notepads across `.sisyphus/` directory:

- `learnings.md` — Patterns and successful approaches discovered
- `decisions.md` — Architectural choices and rationales
- `issues.md` — Problems, blockers, and gotchas
- `verification.md` — Test results and validation outcomes
- `problems.md` — Unresolved issues and technical debt

> "Learnings from Task 1 prevent mistakes in Task 5."

**Application to ORCHA:** Cross-task learning propagation. When one agent discovers a pattern or gotcha, it should be recorded and made available to subsequent agents working on related tasks.

### Continuation Decision Gates

Before continuing execution, the system checks:
- Session validation (confirms plan execution mode)
- Abort flag checking (respects stop commands)
- Failure count monitoring (<5 consecutive failures)
- Background task completion verification
- Agent match validation
- Plan completeness assessment
- Cooldown enforcement (>5s since last injection)

**Application to ORCHA:** Multiple safety gates before proceeding prevents runaway agent loops. Failure count thresholds and cooldown periods are simple but effective guardrails.

---

## Bonus: task-orchestrator MCP (Complementary Patterns)

The [task-orchestrator](https://github.com/jpicklyk/task-orchestrator) MCP server (by jpicklyk) provides complementary patterns worth noting:

### Server-Enforced Quality Gates ("Planning Floors")

Unlike ATLAS which trusts agents to self-manage, task-orchestrator **blocks** state transitions until requirements are met:

- Custom YAML schemas define required documentation per phase
- `advance_item(trigger="start")` validates dependency satisfaction AND gate completion
- `get_context()` returns `canAdvance`, `missing`, and `guidancePointer` fields
- The server prevents progression until work meets defined standards

> "Deterministic workflow progression that doesn't depend on prompt discipline."

**Application to ORCHA:** Server-side gate enforcement is more reliable than hoping agents follow instructions. Our task status transitions should validate prerequisites server-side, not just in agent prompts.

### Composable Notes as Context Engineering

Notes are keyed text documents attached to work items, serving as persistent memory layers:
- A 200-token note replaces 5,000+ tokens of conversation history
- `query_notes(includeBody=false)` enables token-efficient metadata checks
- Phase-scoped documentation guides agents without overwhelming context

**Application to ORCHA:** Structured, queryable notes per task phase reduce token consumption dramatically compared to replaying conversation history.

### Unified WorkItem Model

Everything is a WorkItem — no separate Project/Feature/Task types. Items nest hierarchically (up to 4 levels) via `parentId`. Dependencies use typed BLOCKS edges with patterns (linear, fan-out, fan-in).

**Application to ORCHA:** Consider whether our separate project/task entities would benefit from a unified hierarchical model with typed dependency edges.

### Named Transition Triggers

Items flow through `queue → work → review → terminal` using named triggers (`start`, `complete`, `block`, `resume`, `cancel`) rather than raw status assignments. The server enforces valid transitions.

**Application to ORCHA:** Named triggers instead of raw status updates make invalid transitions impossible at the API level.

---

## Comparison: ATLAS vs ORCHA Patterns

| Dimension | ATLAS MCP | ORCHA (Current) | Gap/Opportunity |
|-----------|-----------|-----------------|-----------------|
| **Data model** | Graph (Neo4j) — Projects/Tasks/Knowledge with relationship edges | Relational (SQLite) — projects + tasks with FK joins | Graph queries for dependency analysis; knowledge as first-class entity |
| **Dependencies** | First-class DAG with circular detection | No formal dependency tracking between tasks | Add dependency edges with validation |
| **Knowledge layer** | Structured, citable, searchable knowledge nodes | Execution artifacts stored as blobs | Promote agent-discovered knowledge to queryable entities |
| **Completion criteria** | `completionRequirements` + `outputFormat` per task/project | Informal — defined in task description | Formalize success criteria as structured fields |
| **Cross-entity search** | Unified search across projects/tasks/knowledge | Separate queries per entity type | Add unified search endpoint |
| **Deep research** | Structured decomposition → auto-generated tasks | Manual workflow definition | Auto-generate tasks from research plans |
| **Bulk operations** | All tools support `mode: 'single' | 'bulk'` | Individual CRUD operations | Add bulk create/update endpoints |
| **Session persistence** | Full state persists across sessions via Neo4j | In-memory + SQLite | Already comparable (SQLite) |
| **Source tracking** | Citations on knowledge items | No provenance tracking | Add source attribution to agent findings |
| **Gate enforcement** | Trust-based (ATLAS) vs server-enforced (task-orchestrator) | Trust-based | Consider server-side gate validation |

---

## Actionable Recommendations for ORCHA

### High Priority

1. **Formalize Completion Criteria** — Add `completion_requirements` and `output_format` as structured fields on tasks and projects. Every task should explicitly define what "done" looks like *at creation time*, not as freeform text in the description. This is ATLAS's most impactful pattern — it forces agents (and humans) to think about success criteria upfront.

2. **Task Dependency Graph** — Add dependency edges between tasks with:
   - Circular dependency detection on create/update
   - "Blocked" status when dependencies are incomplete
   - Query support: "what tasks are blocked by X?" and "what are X's prerequisites?"
   - This transforms our flat task list into a DAG-aware workflow engine.

3. **Structured Task Delegation Template** — Adopt the six-section delegation format (TASK, EXPECTED OUTCOME, REQUIRED TOOLS, MUST DO, MUST NOT DO, CONTEXT) for all agent-dispatched work. Prevents ambiguous assignments and provides explicit anti-patterns.

### Medium Priority

4. **Knowledge as First-Class Entity** — Promote agent-discovered information from ephemeral execution artifacts to persistent, queryable knowledge items with:
   - Domain categorization and tagging
   - Citation/source tracking (where did this information come from?)
   - Cross-project search (knowledge discovered in project A is findable from project B)
   - This solves the "genius with amnesia" problem across sessions.

5. **Cross-Task Learning Propagation** — Implement a wisdom accumulation pattern: when agents discover patterns, gotchas, or successful approaches, record them in structured notes that subsequent agents can query. Prevents repeating mistakes across related tasks.

6. **Server-Side Gate Enforcement** — Don't rely on agents to self-manage state transitions. Add server-side validation:
   - Named transition triggers (`start`, `complete`, `block`, `cancel`) instead of raw status updates
   - Prerequisites check before state advancement
   - "What's missing?" API response when gates aren't satisfied
   - This is the task-orchestrator's strongest pattern.

7. **Bulk Operation Support** — Add `mode: 'single' | 'bulk'` to task/project CRUD endpoints. Agents decomposing complex goals need to create 10+ tasks atomically, not one-by-one.

### Lower Priority

8. **Deep Research as Workflow Pattern** — Implement a research decomposition tool that accepts a topic + sub-questions and auto-generates:
   - Knowledge items for each finding
   - Tasks for each sub-question
   - Proper priority/assignment routing

9. **Unified Cross-Entity Search** — Single search endpoint that queries across projects, tasks, and knowledge with regex/fuzzy matching. Agents shouldn't need to know which entity type contains the information they need.

10. **Continuation Safety Gates** — Before agents continue autonomous execution, check:
    - Consecutive failure count (<5)
    - Cooldown period (>5s since last action)
    - Abort flag (user stop request)
    - Plan completeness
    - These prevent runaway agent loops without heavy infrastructure.

---

## Key Takeaways

1. **Graph > flat list for dependencies** — ATLAS's Neo4j foundation makes dependency queries trivial. Even without Neo4j, adding dependency edges to our SQLite model would unlock DAG-aware workflows.

2. **Define "done" upfront** — `completionRequirements` + `outputFormat` as structured fields forces clarity at task creation, not as an afterthought.

3. **Knowledge is not ephemeral** — Agent-discovered information should outlive the task that discovered it. Structured, citable, searchable knowledge prevents re-discovery costs.

4. **Structured delegation > freeform instructions** — The six-section template (TASK/OUTCOME/TOOLS/MUST DO/MUST NOT/CONTEXT) eliminates ambiguity in agent task assignments.

5. **Server enforces, agents execute** — State transition validation belongs on the server, not in prompts. Named triggers make invalid transitions impossible.

6. **Cross-task learning accumulates** — Wisdom from early tasks should propagate to later tasks. Structured notepads (learnings, decisions, issues) are a lightweight pattern for this.

7. **Dual-mode tools reduce proliferation** — `mode: 'single' | 'bulk'` on every tool avoids separate endpoints for batch operations.

---

## Sources

- [ATLAS MCP Server — GitHub (cyanheads)](https://github.com/cyanheads/atlas-mcp-server)
- [ATLAS on npm](https://www.npmjs.com/package/atlas-mcp-server)
- [ATLAS on MCP Servers Directory](https://mcpservers.org/servers/cyanheads/atlas-mcp-server)
- [ATLAS on Glama](https://glama.ai/mcp/servers/@cyanheads/atlas-mcp-server)
- [ATLAS on LobeHub](https://lobehub.com/mcp/cyanheads-atlas-mcp-server)
- [ATLAS — SkyWork Analysis](https://skywork.ai/skypage/en/atlas-mcp-server-ai-agents/1977608491173474304)
- [Atlas Plan Executor — DeepWiki (oh-my-opencode)](https://deepwiki.com/code-yeongyu/oh-my-opencode/4.3-atlas-hook-and-agent)
- [Task Orchestrator MCP — GitHub (jpicklyk)](https://github.com/jpicklyk/task-orchestrator)
- [Roo Code Boomerang Tasks](https://docs.roocode.com/features/boomerang-tasks)

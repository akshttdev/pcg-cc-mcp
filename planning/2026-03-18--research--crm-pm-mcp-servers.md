# CRM & Project Management MCP Servers — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, tools, and techniques from the best CRM and project management MCP servers applicable to our ORCHA agent orchestration system and task management processes.

---

## Landscape Overview

The MCP ecosystem for CRM and project management has matured rapidly. Key players:

| Category | Server | Stars/Status | Key Differentiator |
|----------|--------|-------------|-------------------|
| **CRM (Enterprise)** | [HubSpot MCP](https://developers.hubspot.com/mcp) | GA, first major CRM | Read-only, OAuth 2.1, open interop |
| **CRM (Enterprise)** | Salesforce Agentforce 3 | Pilot | Deepest CRM coupling, premium-gated |
| **CRM (Open Source)** | [Twenty CRM MCP](https://github.com/mhenry3164/twenty-crm-mcp-server) | OSS | Dynamic schema discovery, full CRUD |
| **PM (Enterprise)** | [Atlassian Rovo MCP](https://github.com/atlassian/atlassian-mcp-server) | GA | Jira+Confluence+Compass+JSM, 40+ tools |
| **PM (Enterprise)** | [Linear MCP](https://linear.app/docs/mcp) | GA | OAuth 2.1, remote hosted, 25 tools |
| **PM (Open Source)** | [Plane MCP](https://developers.plane.so/dev-tools/mcp-server) | GA | AI-native, 30+ tools, agent lifecycle tracking |
| **Task Mgmt (AI)** | [Task Master](https://github.com/eyaltoledano/claude-task-master) | 25.3k stars | PRD→tasks.json decomposition, 36 MCP tools |
| **Task Mgmt (AI)** | [Shrimp Task Manager](https://github.com/cjo4m06/mcp-shrimp-task-manager) | 2.1k stars | Chain-of-thought, reflection, style consistency |
| **Task Orchestration** | [GitHub Project Manager MCP](https://github.com/kunwarVivek/mcp-github-project-manager) | OSS | PRD→traceability matrix, AI task generation |

---

## Enterprise CRM MCPs

### HubSpot MCP Server

**Architecture:** Remote OAuth 2.0 gateway at `mcp.hubspot.com`. Read-only access to CRM objects.

**Supported Objects:** Contacts, companies, deals, tickets, invoices, products, line items, quotes, subscriptions, orders, carts, users, plus object associations.

**Security:** OAuth 2.1 with PKCE + refresh token rotation (2025+). Scopes determine object access. Sensitive data properties (PHI) are explicitly excluded.

**Key design pattern:** HubSpot is built for **open interoperability** — it works with any MCP-compatible client (Claude, Copilot, Gemini, etc.) rather than locking users into a proprietary assistant.

**Application to ORCHA:** Our CRM MCP integration should follow HubSpot's open interop model — expose CRM data via standard MCP tools that any agent can consume, not tight-coupled to a specific LLM provider.

### Salesforce MCP Architecture

Salesforce's approach adds a critical **context enrichment layer** between raw CRM data and agent consumption:

- **Pre-processed logic graphs** mapping business rules and routing logic
- **Dependency mapping** surfacing critical relationships
- **Heuristic edge-case detection** preventing nonsensical outputs
- **Process mining** to interpret automation complexity

> Raw metadata alone causes misinterpretation. Example: counting demos under an auto-assigned salesperson without filtering `no_touch = true` flags yields inflated metrics.

**Application to ORCHA:** Don't expose raw database tables to agents. Add a semantic layer that interprets business rules, surfaces relationships, and prevents common misinterpretation patterns. Our CRM MCP should return interpreted context, not raw rows.

---

## Enterprise PM MCPs

### Atlassian Rovo MCP Server (40+ Tools)

The most comprehensive PM MCP. Full tool inventory:

**Jira (12 tools):**
- `createJiraIssue`, `editJiraIssue`, `getJiraIssue`, `transitionJiraIssue`
- `addCommentToJiraIssue`, `addWorklogToJiraIssue`
- `searchJiraIssuesUsingJql` — JQL-based issue queries
- `getJiraIssueTypeMetaWithFields` — field config for creation
- `getTransitionsForJiraIssue` — permitted workflow actions
- `getVisibleJiraProjectsList`, `getJiraIssueRemoteIssueLinksList`, `lookupJiraAccountId`

**Confluence (10 tools):**
- `createConfluencePage`, `getConfluencePage`, `updateConfluencePage`
- `getConfluencePageDescendants`, `getPagesInConfluenceSpace`, `getConfluenceSpaces`
- `createConfluenceFooterComment`, `createConfluenceInlineComment`
- `getConfluencePageFooterComments`, `getConfluencePageInlineComments`
- `searchConfluenceUsingCql` — CQL content discovery

**Compass (12 tools):** Service catalog CRUD with custom fields, relationships, labels, activity events, team ownership filtering.

**JSM (4 tools):** Ops alerts, on-call schedules, team/escalation info, alert actions.

**Cross-product (4 tools):**
- `atlassianUserInfo`, `getAccessibleAtlassianResourcesList`
- `search` (beta) — Natural language search across Jira + Confluence
- `getTeamworkGraphContext` — **Fetches full context of a Jira issue**: linked deployments, PRs, builds, branches, commits, designs, feature flags, vulnerabilities, sprints

**Key patterns:**

1. **Workflow-aware operations** — `transitionJiraIssue` executes workflow state changes; `getTransitionsForJiraIssue` enumerates *permitted* actions. The agent discovers what it *can* do rather than assuming.

2. **Cross-product context graph** — `getTeamworkGraphContext` links a Jira issue to its PRs, builds, deployments, and vulnerabilities. This is the "knowledge graph" pattern applied to PM.

3. **Query languages exposed as tools** — JQL and CQL give agents powerful, structured query capabilities without building custom search endpoints.

**Application to ORCHA:**
- Our task MCP should expose workflow transitions as discoverable actions, not raw status updates
- Cross-entity context (task → PR → deployment → test results) should be queryable as a graph
- Expose structured query capabilities (like our existing SQL/SQLite) as MCP tools for agent self-service

### Linear MCP Server (25 Tools)

**Architecture:** Remote hosted at `https://mcp.linear.app/mcp`, Streamable HTTP transport, OAuth 2.1 with dynamic client registration.

**Capabilities:** Issue CRUD, project management, cycle/initiative/milestone support, comments. Notable for being **centrally hosted** — no local credentials needed.

**Application to ORCHA:** Remote MCP hosting (vs local stdio) simplifies agent deployment — agents don't need local API keys or database access.

### Plane MCP Server (30+ Tools)

**Key differentiator:** Plane is **AI-native** — not retrofitted. Features include:
- Agent framework with `@mention` support
- Full Agent Run lifecycle tracking
- Issues, cycles, modules, work logs
- Four transport methods: HTTP+OAuth, HTTP+PAT, Stdio, SSE

**Application to ORCHA:** Our system should track agent run lifecycles natively, not as a bolt-on. Agent mentions in issues/tasks enable direct agent-human collaboration within the PM tool itself.

---

## AI Task Management MCPs

### Task Master (25.3k Stars, 36 MCP Tools)

The most popular AI task management MCP. Core architecture:

**PRD → tasks.json Decomposition:**
1. User creates PRD at `.taskmaster/docs/prd.txt`
2. AI parses requirements into structured task hierarchy
3. Dependencies auto-mapped: "authentication before API endpoints, Redis before rate limiting"
4. Output: `tasks.json` with IDs, titles, descriptions, statuses, dependencies, complexity scores

**Dependency Management:**
- Cross-tag task movement with dependency tracking
- `--with-dependencies` moves dependent tasks together
- `--ignore-dependencies` for override
- Prevents broken task chains during workflow transitions

**Complexity Analysis:**
- `analyze_project_complexity` provides relative difficulty scoring
- Guides prioritization and sequencing
- AI recommends task ordering based on implementation prerequisites

**Subtask Expansion:**
- `expand_task` decomposes parent tasks into implementable subtasks
- Subtasks maintain parent references for coherent navigation
- TDD workflows: test requirements precede implementation

**Modular Tool Loading:**
- 3 modes: Core (7 tools), Standard (15), All (36 ~21,000 tokens)
- `TASK_MASTER_TOOLS` env var for selective loading
- Custom comma-separated tool lists for specialized workflows

**Application to ORCHA:**
- **Modular tool loading** is critical — not every agent needs all 36 tools. Loading only relevant tools saves ~14,000 tokens
- **PRD → task decomposition** pipeline maps directly to our workflow execution system
- **Complexity scoring** could inform model tier selection (simple tasks → cheap model, complex → expensive)

### GitHub Project Manager MCP

**Unique patterns:**

**AI-Powered PRD Pipeline:**
- `generate_prd` → `parse_prd` → tasks with dependencies
- `add_feature` with automatic impact assessment
- `enhance_prd` with gap analysis

**Requirements Traceability Matrix:**
- `create_traceability_matrix` — bidirectional linking: business requirements → features → use cases → tasks
- Coverage analysis identifying gaps and orphaned tasks
- Change impact propagation across requirement hierarchy

**Dual Context Generation:**
1. **Traceability-based** (default) — derives context from requirement links, no AI dependency
2. **AI-enhanced** (optional) — adds business, technical, and implementation guidance

**Graceful degradation:** Works without AI keys by falling back to traceability-based context.

**Application to ORCHA:**
- **Traceability matrix** — linking requirements → features → tasks with coverage analysis is a powerful QA tool
- **Graceful degradation** — agent orchestration should work without AI keys (fallback to rule-based)
- **Impact propagation** — when a requirement changes, automatically identify affected tasks

### Shrimp Task Manager

**Chain-of-thought task management:**
- `plan task: [description]` triggers deep analysis before implementation
- Agent examines codebase, identifies requirements, creates decomposed subtasks
- `reflect task [id]` enables iterative refinement (review and improve existing tasks)
- `research [topic]` supports investigation-driven development before implementation

**Three operational modes:**
1. **Planning** — systematic decomposition and analysis
2. **Execution** — task-by-task implementation
3. **Continuous** — automated sequential execution of all tasks

**Project Rules:** `init project rules` establishes coding standards that agents reference during execution.

**Application to ORCHA:**
- **Reflection as a first-class operation** — agents should be able to review and improve their own task decomposition
- **Research before implementation** — add an explicit research phase to the agent task lifecycle
- **Project rules** as persistent constraints that agents reference during all work

---

## CRM Agent Integration Patterns (from Warmly/Enterprise)

### Ontological Reduction

> Don't give agents raw data streams. Pre-digest information into ontological states.

A contact becomes: *"Sarah Chen (CFO, Champion), visited pricing 12x, ROI calc 3x, similar accounts convert 73% in 45 days, key concern: security."*

This reduces token burden by 10-100x while increasing decision quality by providing **interpreted** rather than raw context.

**Application to ORCHA:** Our CRM MCP should return pre-computed summaries, not raw database rows. A deal summary is more useful to an agent than 50 activity log entries.

### Dual Clock Architecture

Traditional CRM tracks **state** ("deal is Closed Lost") but not **events** ("champion left, competitor evaluated, budget froze"). Production systems need both:

- **State Clock** — Current CRM data (deal stage, contact, ARR)
- **Event Clock** — Temporal context (when/why things changed)

**Application to ORCHA:** Our task/CRM data should capture both current state AND the temporal event stream. This enables agents to reason about *why* things are in their current state, not just *what* the state is.

### Multi-Agent Collision Prevention

Three governance layers for preventing agent conflicts:

1. **Policy Engine** — YAML rules constraining behavior ("max 1 touch per account per day, 72h cooldown after email")
2. **Decision Ledger** — Every action logged with reasoning, confidence scores, and world-model snapshot
3. **Trust Gate** — High-risk actions require policy + trust score + authorization. Low-confidence routes to human review.

**Ownership locks** prevent simultaneous agent control over the same entity within a decision window.

**Application to ORCHA:** Our multi-agent orchestration needs:
- Policy-based constraints on agent actions (not just prompt-based)
- Decision audit trail with reasoning and confidence
- Trust gates that route uncertain actions to humans
- Entity-level ownership locks preventing concurrent agent edits

### Context Compaction

> "100,000 website visits over 2 years compacts into roughly 500 tokens of ontological state."

Raw data → scored abstractions:
- Engagement scores instead of raw event logs
- Intent signals aggregated from behavior
- Risk indicators (single-threaded relationships)

**Application to ORCHA:** Agent context should include pre-computed scores and signals, not raw activity logs. Build a context compaction layer between our database and agent consumption.

---

## Cross-Cutting Design Patterns

### 1. Dynamic Schema Discovery (Twenty CRM)

The server auto-adapts to CRM configuration by dynamically discovering available fields. No manual field mapping — supports custom fields without code changes.

**Application to ORCHA:** Our MCP tools should auto-discover schema changes rather than hardcoding field lists. When a new custom field is added to tasks/deals, the MCP should expose it automatically.

### 2. Workflow Transition Discovery (Atlassian)

`getTransitionsForJiraIssue` returns *permitted* workflow actions. The agent discovers what it *can* do rather than assuming valid transitions.

**Application to ORCHA:** Our task status MCP should expose valid transitions per task, not just accept any status string. Agents query available actions before attempting transitions.

### 3. Modular Tool Loading (Task Master)

36 tools = ~21,000 tokens of tool definitions. Loading all tools wastes context on capabilities the agent doesn't need. Three modes (Core 7 / Standard 15 / All 36) with custom lists.

**Application to ORCHA:** Our MCP tool set should be loadable in tiers. A triage agent needs read-only tools; a developer agent needs CRUD; an orchestrator needs workflow tools. Don't load all tools for all agents.

### 4. Traceability Matrix (GitHub PM)

Bidirectional linking: business requirements → features → use cases → tasks. Coverage analysis identifies gaps and orphaned tasks. Change impact propagation.

**Application to ORCHA:** Link our CRM deals → project requirements → tasks → PRs → deployments. When a deal's requirements change, automatically identify affected tasks. When a task is completed, update requirement coverage.

### 5. Reflection & Iterative Refinement (Shrimp)

`reflect task [id]` lets agents review and improve their own task decomposition. Not just plan-and-execute, but plan-execute-reflect-improve.

**Application to ORCHA:** Add a reflection phase to agent task execution. After completing work, agents should review their output and identify improvements before marking tasks done.

### 6. Context Graph (Atlassian Teamwork Graph)

`getTeamworkGraphContext` returns the full web of relationships around an issue: PRs, builds, deployments, branches, commits, designs, vulnerabilities, sprints.

**Application to ORCHA:** Our task context should include the full relationship graph — not just the task itself, but its PR, its deployment, its test results, its related CRM deal, and its blocking/blocked-by tasks.

---

## Comparison: CRM/PM MCPs vs ORCHA

| Dimension | Best-in-Class Pattern | ORCHA (Current) | Gap/Opportunity |
|-----------|----------------------|-----------------|-----------------|
| **Schema discovery** | Dynamic (Twenty CRM) | Hardcoded types | Auto-discover schema changes |
| **Workflow transitions** | Discoverable valid actions (Atlassian) | Raw status updates | Expose permitted transitions per entity |
| **Tool loading** | Modular tiers (Task Master) | All tools always | Tier-based tool loading per agent role |
| **Context enrichment** | Pre-computed ontological states (Warmly) | Raw DB rows | Context compaction layer |
| **Temporal reasoning** | Dual clock — state + events (Warmly) | State only | Add event clock for "why" reasoning |
| **Cross-entity graph** | Teamwork Graph (Atlassian) | Separate queries | Unified context graph per entity |
| **Collision prevention** | Policy engine + decision ledger + trust gates | None | Multi-agent governance layers |
| **Requirements traceability** | Bidirectional matrix (GitHub PM) | None | Link deals → requirements → tasks → PRs |
| **Task decomposition** | PRD → tasks.json (Task Master) | Manual | AI-powered decomposition from requirements |
| **Reflection** | Plan-execute-reflect cycle (Shrimp) | Plan-execute only | Add reflection phase |
| **Agent lifecycle** | Native tracking (Plane) | Execution artifacts | Track full agent run lifecycle |
| **Query languages** | JQL/CQL exposed as tools (Atlassian) | No agent query tools | Expose structured query capabilities |

---

## Actionable Recommendations for ORCHA

### High Priority

1. **Context Compaction Layer** — Don't expose raw database rows to agents. Build a semantic layer that pre-computes ontological summaries: deal scores, engagement signals, risk indicators, relationship graphs. Reduces token consumption 10-100x while improving agent decision quality. This is the single highest-ROI pattern from enterprise CRM MCP implementations.

2. **Discoverable Workflow Transitions** — Our task/deal MCP should expose valid transitions per entity state, not accept any status string. Agents query `getAvailableTransitions(taskId)` before attempting status changes. This prevents invalid transitions at the protocol level rather than relying on agent prompt discipline. Complements the "server-side gate enforcement" recommendation from ATLAS research.

3. **Multi-Agent Collision Prevention** — Implement three governance layers:
   - **Policy engine** — YAML/JSON rules constraining agent actions per entity type
   - **Decision ledger** — Every agent action logged with reasoning and confidence score
   - **Trust gates** — High-risk actions require policy + trust + authorization; low-confidence routes to human
   - **Entity ownership locks** — Prevent concurrent agent edits to the same entity

### Medium Priority

4. **Modular Tool Loading by Agent Role** — Don't load all MCP tools for all agents. Define tool tiers: read-only (triage), standard (CRUD), full (workflow + admin). Task Master's approach (7/15/36 tools across Core/Standard/All modes) saves ~14,000 tokens per agent session.

5. **Cross-Entity Context Graph** — When an agent queries a task, return the full relationship web: parent project, blocking/blocked-by tasks, related CRM deal, associated PRs, deployment status, test results. Atlassian's `getTeamworkGraphContext` is the gold standard.

6. **Requirements Traceability Matrix** — Link CRM deals → project requirements → tasks → PRs → deployments. Coverage analysis identifies gaps (requirements without tasks) and orphaned work (tasks without requirements). Change impact propagation when requirements evolve.

7. **Dual Clock (State + Events)** — Track both current state AND the temporal event stream for CRM entities. Agents need to know *why* a deal is in its current state, not just *what* the state is. Enables temporal reasoning like "re-engage TechCorp — when you lost them in Q2, they had 50 employees; they now have 180."

### Lower Priority

8. **Dynamic Schema Discovery** — MCP tools should auto-discover schema changes. When custom fields are added to tasks/deals, they should be queryable without code changes. Twenty CRM's auto-adaptation pattern.

9. **Reflection Phase in Agent Lifecycle** — After completing work, agents review their output and identify improvements before marking tasks done. Shrimp Task Manager's `reflect task` pattern makes self-improvement a first-class operation.

10. **Agent-Native PM Integration** — Track agent run lifecycles natively in the PM system (Plane's approach). Agents are first-class actors in the project, not external tools. Agent mentions in issues enable direct agent-human collaboration.

11. **Structured Query Language for Agents** — Expose structured query capabilities (JQL-like) as MCP tools. Agents can self-serve complex queries without custom endpoints. More powerful than keyword search, more constrained than raw SQL.

---

## Key Takeaways

1. **Interpret, don't expose** — Raw CRM/PM data overwhelms agents. Pre-compute ontological summaries (scores, signals, risk indicators) that reduce tokens 10-100x while improving decisions.

2. **Transitions are discoverable, not assumed** — Agents should query what they *can* do before attempting actions. Expose valid workflow transitions per entity state.

3. **Load only what's needed** — 36 tools = 21,000 tokens of definitions. Tier-based tool loading per agent role saves context and improves focus.

4. **State + events = temporal reasoning** — Track both what things are AND why they got there. Event clocks enable resurrection patterns and historical reasoning.

5. **Cross-entity graphs beat flat queries** — A task's full context includes its PR, deployment, test results, CRM deal, and blocking tasks. Graph queries surface this in one call.

6. **Multi-agent collision prevention is non-optional** — Policy engines, decision ledgers, trust gates, and entity locks prevent agent conflicts at scale. Prompt-based coordination doesn't scale.

7. **Traceability enables coverage analysis** — Bidirectional requirement → task linking reveals gaps and orphaned work. Impact propagation when requirements change.

8. **Reflection improves quality** — Plan-execute-reflect is better than plan-execute. Agents that review their own output catch issues humans would flag.

---

## Sources

- [HubSpot MCP Server — Developer Docs](https://developers.hubspot.com/mcp)
- [Salesforce MCP Explained — Salesforce Ben](https://www.salesforceben.com/salesforce-model-context-protocol-explained-how-mcp-bridges-ai-and-your-crm/)
- [Atlassian Rovo MCP — GitHub](https://github.com/atlassian/atlassian-mcp-server)
- [Atlassian Rovo MCP — Supported Tools](https://support.atlassian.com/atlassian-rovo-mcp-server/docs/supported-tools/)
- [Linear MCP Server — Docs](https://linear.app/docs/mcp)
- [Plane MCP Server — Developer Docs](https://developers.plane.so/dev-tools/mcp-server)
- [Task Master — GitHub (25.3k stars)](https://github.com/eyaltoledano/claude-task-master)
- [Shrimp Task Manager — GitHub](https://github.com/cjo4m06/mcp-shrimp-task-manager)
- [GitHub Project Manager MCP — GitHub](https://github.com/kunwarVivek/mcp-github-project-manager)
- [Twenty CRM MCP Server — GitHub](https://github.com/mhenry3164/twenty-crm-mcp-server)
- [MCP for Sales Teams — Warmly](https://www.warmly.ai/p/blog/mcp-sales-teams-revenue-model)
- [Martin Fowler — SDD Tools Comparison](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)

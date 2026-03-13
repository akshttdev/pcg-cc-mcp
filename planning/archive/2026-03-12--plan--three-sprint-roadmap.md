# Dashboard Improvement Roadmap — Task, Agent & Workflow Automation

**Date:** 2026-03-12
**Companion doc:** `2026-03-12-dashboard-task-agent-usability-review.md`
**Planning horizon:** 3 sprints (medium-term)

---

## Guiding Principles

1. **Admin-first, org-member-ready** — Build for platform admins now, but architect for org/client member access
2. **Workflow builder as the extensibility engine** — Move automations into the builder system where possible; extend through custom node types
3. **Standardize agent infrastructure** — Get the plumbing right (MCP wiring, completion criteria) before per-agent tuning
4. **Maximize UX improvement per unit of effort** — Prioritize visible, usable features over backend-only work

---

## Current State (Better Than Expected)

The workflow system already has **11 node types** — significantly more than what's visible in the single system workflow:

| Category | Node Types | Status |
|----------|-----------|--------|
| LLM Processing | `llm_extract`, `llm_analyze`, `llm_summarize` | Working |
| Data Transform | `transform`, `filter`, `merge` | Defined in registry |
| Data Source | `data_source` | Working |
| CRM Output | `output_crm_contacts`, `output_crm_companies`, `output_crm_deals` | Working (staging → commit) |
| Task Output | `output_tasks` | **Defined but needs backend verification** |

The `output_tasks` node type already exists in the frontend registry. This means workflow → task creation may be closer to working than the review suggested.

**Extension pattern is clean:** Add entry to `NODE_TYPES[]` array (frontend) + add execution branch in `execute_node_with_llm()` (backend). Each new node type is ~50-100 lines of code.

---

## Sprint 1: Foundation & Quick Wins

**Theme:** Make what exists visible and usable

### 1A. Completion Criteria in Task Form (Quick Fix)
- Add `completion_criteria` (textarea) and `output_format` (dropdown/text) fields to `TaskFormDialog`
- Show in both simple and advanced modes
- Display on task detail view and Kanban card tooltip
- **Effort:** Small (frontend-only, fields already in API)
- **Impact:** Agents can receive structured success criteria; admins can define "done"

### 1B. Project Scaffolding UI
- Add "Start from Template" option in New Project dialog
- Expose the 4 `scaffold_project` templates: Software, Research, Marketing, Client Onboarding
- After project creation, show pre-populated Kanban board with template tasks
- Each template task includes completion_criteria and output_format (already defined in MCP tool)
- **Effort:** Medium (new UI component + API call to scaffold_project logic)
- **Impact:** Projects start with structure instead of empty boards; biggest UX improvement for admins

### 1C. ORCHA Task Server in MCP Popular Servers
- Add `orcha_task_server` to the popular servers carousel in Settings → MCP
- Include description: "26 tools for tasks, knowledge, dependencies, search, and project scaffolding"
- Pre-populate the JSON config when clicked
- **Effort:** Small (add entry to `default_mcp.json` meta, update frontend carousel data source)
- **Impact:** Users discover the platform's own MCP server exists

### 1D. Verify & Wire `output_tasks` Node Type
- Confirm `output_tasks` has backend execution logic in `execute_node_with_llm()`
- If missing, implement: extract task records from LLM output → create staging records → commit as tasks
- Add output schema in `output_schemas.rs` for task fields (title, description, priority, assignee, completion_criteria)
- Create a second system workflow that uses `output_tasks` (e.g., "Bug Triage Pipeline" or "Sprint Planning from Requirements")
- **Effort:** Medium (backend wiring + new system workflow)
- **Impact:** First working workflow → task pipeline; proves the integration pattern

### 1E. Basic Agent Execution Dashboard
- New page or widget showing: active task attempts, recent completions, agent queue
- Table view: Task name, Assigned agent, Status, Started at, Duration, Outcome
- Filter by agent, project, status
- Link from agent cards in Settings → Agents to their execution history
- **Effort:** Medium (new page, queries against existing `task_attempts` table)
- **Impact:** Admins can see what agents are doing; critical for trust and debugging

---

## Sprint 2: Workflow Builder Extension

**Theme:** Expand the workflow builder toward a general-purpose automation engine

### 2A. New Node Types — Quick Wins

Investigate and implement these node types (each ~50-100 lines backend + frontend registry entry):

| Node Type | Category | Description | Effort |
|-----------|----------|-------------|--------|
| `conditional` | Control Flow | Branch execution based on field value/threshold | Medium |
| `update_crm_contact` | CRM Action | Update contact fields (lifecycle_stage, tags, etc.) | Small |
| `update_crm_deal` | CRM Action | Update deal stage, value, close date | Small |
| `send_notification` | Action | Send in-app notification or email | Small |
| `assign_to_agent` | Agent | Assign a task to a specific agent and trigger execution | Medium |
| `http_request` | Integration | Make external API calls, transform response | Medium |

**Priority order:** `conditional` → `assign_to_agent` → `update_crm_*` → `send_notification` → `http_request`

The `conditional` node unlocks branching logic. The `assign_to_agent` node bridges workflows to the executor system.

### 2B. Convert Automations to System Workflows

The 5 hardcoded automations map cleanly to workflow definitions:

| Current Automation | Workflow Equivalent |
|---|---|
| Waiting-on-client → follow-up task | `filter`(3 days stale) → `output_tasks`(follow-up) |
| 5+ follow-ups → churn lead | `filter`(attempts ≥ 5) → `update_crm_contact`(churned) |
| Lead reaches SQL → draft proposal | `filter`(stage = SQL) → `output_tasks`(draft proposal) |
| Signed proposal → create project | `filter`(signed) → *new* `create_project` node |
| Complete project → draft invoice | `filter`(complete) → *new* `create_invoice` node |

**Approach:**
- Keep existing hardcoded automations running (they work)
- Create equivalent system workflows as "v2" replacements
- Once validated, deprecate hardcoded versions
- Platform-level automations that are truly infrastructure (not user-configurable) can stay hardcoded

### 2C. Workflow Trigger System (Foundation)

Currently workflows only run on manual trigger or data source upload. Add:

1. **`trigger_schedule` node** — Cron-based (hourly, daily, weekly, custom)
2. **`trigger_entity_change` node** — Watch for task/contact/deal state changes
3. **Workflow scheduler background worker** — Checks trigger conditions, executes matching workflows

This is the **largest effort item** in Sprint 2 but unlocks the automation → workflow migration and makes the builder a true automation engine.

### 2D. More System Workflows

Create system workflows that demonstrate platform capabilities and are immediately useful:

| Workflow | Nodes Used | Purpose |
|----------|-----------|---------|
| Bug Triage Pipeline | `llm_analyze` → `filter`(severity) → `output_tasks` | Analyze bug reports, create prioritized tasks |
| Sprint Planning | `llm_extract`(requirements) → `output_tasks`(with completion_criteria) | Break down requirements into agent-ready tasks |
| Client Onboarding | `data_source` → `output_crm_contacts` → `output_tasks`(onboarding checklist) | Process new client data, create contacts + tasks |
| Content Pipeline | `llm_summarize` → `llm_analyze`(SEO) → `output_tasks`(content tasks) | Content analysis to task creation |

---

## Sprint 3: Agent Infrastructure & Platform Config

**Theme:** Wire agents properly and separate platform vs user configuration

### 3A. Wire ACP Agents to MCP (Phase 3G)
- Update `harness.rs` to forward MCP server config to Gemini/Qwen ACP sessions
- Use `default_mcp.json` as the source of truth for executor MCP config
- **Impact:** Non-Claude agents gain access to task management tools

### 3B. Platform vs User MCP Config Separation
- **Platform-level** (`default_mcp.json`): Controlled by admins, applied to all executor agents. Managed in a new "Platform Settings" admin section.
- **User-level** (`~/.claude.json` / Settings → MCP): Per-user customization for their own workflow runs and Claude Code CLI usage.
- **Per-project override** (future): Allow projects to specify additional MCP servers relevant to that project's domain.
- Settings → MCP UI stays as user-level config. Add a separate admin-only "Platform MCP Config" page.

### 3C. Enhanced Task Templates
- Extend global task templates to include: completion_criteria, output_format, assigned_agent, MCPs, priority, tags
- Templates become "agent-ready task presets" — one click to create a well-specified agent task
- Allow templates to be scoped per-project or per-organization (not just global)

### 3D. Agent Capability Profiles
- Extend Settings → Agents cards to show: assigned MCP tools, preferred model, execution history summary, current workload
- Add "Test Agent" button that creates a simple test task and runs it
- Display success/failure rates per agent

### 3E. Agent Flow Orchestration (Phase 3H — Design Only)
- Design the orchestration engine architecture: background worker, phase progression, gate enforcement
- Define the data model for flow templates (multi-phase, multi-agent workflows)
- **Implementation deferred** to a dedicated sprint — this is the largest effort item in the entire roadmap

---

## Effort/Impact Matrix

```
                        HIGH IMPACT
                            │
         1B Scaffolding ────┤──── 2C Trigger System
         1D output_tasks ───┤──── 2A assign_to_agent
         1A Completion ─────┤──── 2B Convert Automations
         1E Dashboard ──────┤──── 3A ACP MCP Wiring
         1C MCP Popular ────┤
                            │
    LOW EFFORT ─────────────┼──────────── HIGH EFFORT
                            │
         3C Templates ──────┤──── 3E Orchestration Engine
         2A update_crm_* ──┤──── 3B Config Separation
         2D System Workflows┤──── 2A http_request
                            │
                        LOW IMPACT
```

---

## Key Architectural Decisions

1. **Workflow builder is the extensibility engine** — New platform capabilities should be expressed as node types where possible, not as one-off route handlers. This gives admins visual composition and users eventual self-service.

2. **System workflows vs user workflows** — System workflows (`owner_type: "system"`) ship with the platform and can use privileged node types. User workflows are sandboxed. Organization workflows sit in between.

3. **Node type access control** — Some node types (e.g., `assign_to_agent`, `create_project`) should be restricted to admin/system workflows. Add a `requires_role` field to `NodeTypeDefinition`.

4. **Automation migration is incremental** — Don't remove hardcoded automations until workflow equivalents are proven. Platform-level infra automations (e.g., orphan cleanup, health checks) can stay hardcoded permanently.

5. **MCP config stays separate** — Platform config (what executors get) and user config (personal CLI/workflow tuning) are different concerns with different access control. Unify later only if there's a clear UX win.

---

## Open Questions for Future Sprints

- **Subtask creation UI** — How deep should task hierarchy go? Simple parent/child or full work breakdown structure?
- **Task dependency visualization** — Gantt chart, DAG view, or just blocker indicators on Kanban cards?
- **Autonomy level enforcement (Phase 3I)** — Should this be per-agent, per-project, or per-task-type? What are the escalation paths?
- **Workflow marketplace** — Should organizations be able to share workflow templates? Export/import?
- **Real-time agent collaboration** — Should agents be able to communicate with each other during execution, or always go through the orchestration engine?

# Dashboard Task & Agent Automation — Usability & Functionality Review

**Date:** 2026-03-12
**Branch:** `feature/fraze-2026-03-12`
**Status:** REVIEW COMPLETE
**Method:** Playwright Firefox MCP live interaction + code-level Explore agents

---

## Executive Summary

The ORCHA dashboard has a solid foundation for project management, task creation, and AI agent execution. However, there are significant gaps between the **backend capabilities** (26 MCP tools, completion criteria, agent flows, workflow engine) and what's **exposed in the UI**. The primary finding is that the dashboard is built for human project management but lacks the bridging UX needed to make AI agent automation a first-class workflow.

---

## 1. Task Creation & Management

### What Works Well
- **Task form** is clean and comprehensive: Title, Markdown description (split editor), Priority, Board, Due Date, Assignee, Agent, MCPs, Tags, Approval toggle, Images, Quickstart
- **"Create & Start"** button enables one-click task creation + agent execution
- **Kanban board** with drag-and-drop between status columns (todo → inprogress → inreview → done)
- **Global task templates** (Settings → General): 3 templates exist (Add Unit Tests, Bug Analysis, Code Refactoring)

### Usability Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| U1 | **No completion criteria in UI** | High | `completion_criteria` and `output_format` fields exist in DB/model (Phase 1A) but are NOT exposed in TaskFormDialog or task detail view. Agents have no way to receive structured success criteria from the UI. |
| U2 | **No subtask creation** | Medium | `parent_task_id` exists in the model but there's no UI to create subtasks or view task hierarchy. Complex work can't be decomposed visually. |
| U3 | **No scheduled start/end dates** | Low | `scheduled_start` and `scheduled_end` fields exist in DB but not in the task form. Only `due_date` is exposed. |
| U4 | **Task templates lack agent fields** | Medium | Global task templates (Settings → General) only store name/title/description — no completion_criteria, output_format, assigned_agent, MCPs, or priority. Templates can't encode agent automation presets. |
| U5 | **MCPs field is raw text** | Medium | "Assigned MCPs (comma-separated)" is a plain textarea. No autocomplete, no validation, no way to browse available MCP servers. Users must know MCP server names by heart. |
| U6 | **Agent combobox has no context** | Low | "Assigned Agent" dropdown shows agent names but no role/description preview. User must go to Settings → Agents to learn what each agent does. |
| U7 | **Empty Kanban offers only "Create First Task"** | Low | Empty project kanban shows generic empty state. Could suggest templates, common task patterns, or scaffold_project templates. |
| U8 | **No task dependency visualization** | Medium | Task dependencies exist in the model and `check_dependencies` MCP tool works, but there's no UI to create, view, or manage dependencies between tasks. |

### Functionality Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| F1 | **No project scaffolding in UI** | High | `scaffold_project` MCP tool (Phase 2E) creates projects with 4 templates (software, research, marketing, client_onboarding) but there's no UI equivalent. New projects start empty. |
| F2 | **No bulk task creation** | Medium | `bulk_create_tasks` MCP tool exists but no UI for batch task creation. Users must create tasks one at a time. |
| F3 | **No task-to-workflow connection** | High | Workflow Builder outputs (companies, contacts, opportunities) don't create tasks. There's no "Create tasks from workflow results" action. |
| F4 | **`custom_properties` not exposed** | Low | Task model has `custom_properties` JSON field but no UI to set or view custom metadata. |

---

## 2. AI Agent Configuration & Automation

### What Works Well
- **13 autonomous agents** configured with persona descriptions (Astra, Auri, Editron, Genesis, Maci, Nora, etc.)
- **Agent cards** in Settings → Agents with search, status filter, name sort
- **Default Agent Configuration** in Settings → General (CLAUDE_CODE / DEFAULT profile)
- **Executor pattern** supports multiple backends (Claude Code, Gemini, Duck, Codex, Qwen)

### Usability Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| U9 | **No agent capability visibility** | High | Agent cards show name + 1-line description but no capability details: what MCP tools they have access to, what models they use, their execution history, success rates, or current workload. |
| U10 | **No agent assignment suggestions** | Medium | When creating a task, the system doesn't suggest which agent is best suited. No matching based on task content, tags, or project type. |
| U11 | **No agent execution dashboard** | High | No centralized view of what agents are currently doing, their active tasks, queued work, or recent performance. "My Tasks" only shows the user's tasks. |
| U12 | **No autonomy level controls in UI** | Medium | Backend has `autonomy.rs` routes for autonomy level enforcement (Phase 3I) but no UI to configure trust levels per agent, per project, or per task type. |

### Functionality Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| F5 | **ACP agents get no MCP servers** | Critical | `harness.rs` passes `mcp_servers: vec![]` to Gemini/Qwen sessions (Phase 3G — NOT STARTED). These agents literally have zero tools. |
| F6 | **No agent flow orchestration engine** | Critical | Agent flows exist as data model only (Phase 3H — NOT STARTED). No background worker to progress phases, enforce gates, or handle delegation. The "Agent Flows" tab shows "No Agent Flows" with no way to create them. |
| F7 | **Automations are read-only** | Medium | 5 hourly automations exist (follow-up, churn detection, SQL→proposal, signed→project, complete→invoice) but there's no CRUD — users can't create, edit, enable/disable, or configure triggers. |
| F8 | **No agent-to-agent delegation UI** | Medium | Nora has `delegate_task` and `assign_task_to_agent` tools internally, but there's no UI to set up delegation rules or view delegation chains. |

---

## 3. MCP Server Configuration

### What Works Well
- **MCP Settings page** exists at Settings → MCP Servers
- Per-agent MCP configuration with JSON editor
- "Popular servers" carousel for quick insertion (Duck Kanban, Context7, Playwright)

### Usability Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| U13 | **ORCHA Task Server not in popular servers** | High | The built-in `orcha_task_server` (26 tools + 6 resources) is NOT listed as a popular server. Users have no visual indication that the platform's own task management is available as an MCP server. |
| U14 | **Raw JSON editor is intimidating** | Medium | MCP config is a raw JSON textarea. No visual server manager, no enable/disable toggles, no tool inspection. |
| U15 | **Saves to ~/.claude.json only** | Medium | The MCP settings UI saves to `/Users/.../.claude.json` (Claude Code CLI). This is separate from `default_mcp.json` which configures the executor pipeline. Two disconnected config systems. |
| U16 | **No tool/resource browser** | Medium | No way to see what tools and resources an MCP server exposes without reading docs. A "Test Connection" or "Browse Tools" button would help. |

### Functionality Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| F9 | **Two disconnected MCP config systems** | High | `Settings → MCP Servers` configures Claude Code CLI's `~/.claude.json`. The executor pipeline reads `default_mcp.json`. These are completely separate — changes in the UI don't affect task execution agents. |
| F10 | **No per-project MCP scoping** | Medium | MCP servers are configured globally per agent, not per project. A web scraping project and a code analysis project get the same tools. |

---

## 4. Workflow System

### What Works Well
- **Visual n8n-style node editor** with excellent UX: node list, dependency arrows, per-node config panel
- **Node configuration**: Name, Type (LLM Extract/Analyze), Input Connections, Model override, Prompt Template with `{{variable}}` support, Output Schema
- **Workflow tabs**: Builder, Runs, Staging, Automations + More (Active Executions, Recent Executions, Agent Flows, Research, Conference, Editron)
- **Graph view** and **Preview** modes
- **"New Workflow"** button for custom workflows

### Usability Gaps

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| U17 | **Only 1 system workflow exists** | Medium | "Data Source Analysis" is the only workflow. No templates for common patterns (onboarding, bug triage, sprint planning, content pipeline). |
| U18 | **No visual graph layout** | Low | Node list is linear — the "Graph" button likely shows a proper DAG visualization, but the default list view doesn't show the dependency structure clearly for complex workflows. |
| U19 | **No workflow execution feedback inline** | Low | Must switch to "Runs" tab to see execution results. No inline status indicators on the workflow card. |

### Functionality Gaps (Workflow ↔ Backend Integration)

| # | Gap | Severity | Details |
|---|-----|----------|---------|
| F11 | **Workflows don't trigger task creation** | Critical | The biggest integration gap. Workflow nodes extract data (companies, contacts, opportunities) and stage it, but there's no "Create Task" node type. Workflow results can't automatically spawn tasks for agents to execute. |
| F12 | **No workflow trigger system** | High | Workflows must be manually run. No scheduled triggers (cron), event triggers (new data source, task status change), or webhook triggers. Automations exist separately but aren't connected to the workflow builder. |
| F13 | **No "Agent Task" node type** | High | Workflow nodes are all LLM Extract/Analyze. There's no node type that delegates work to an agent (e.g., "assign to Auri for code implementation"). This would bridge workflows → agent automation. |
| F14 | **Staging → Task pipeline missing** | Medium | Workflow staging (contacts, companies, opportunities) has commit actions to CRM but no "Create task from staged item" action. |
| F15 | **Automations disconnected from Builder** | Medium | The 5 automations (hourly cron jobs) are hardcoded and separate from the visual workflow builder. Users should be able to create automations visually. |

---

## 5. End-to-End Agent Automation Flow (Gap Analysis)

The ideal flow for "assign task to agent for automation" should be:

```
Create Project → Create Task (with criteria) → Assign Agent → Agent Executes → Review → Done
       ↑                                              ↑              ↑
  scaffold_project                              MCP tools      completion_criteria
  (no UI)                                       (partially     (not in UI)
                                                 wired)
```

### Current Breakpoints in the Flow

1. **Project creation** — Works but starts empty (no scaffolding UI)
2. **Task creation** — Works but missing completion_criteria/output_format fields
3. **Agent assignment** — Works via dropdown but no smart suggestions
4. **Agent receives task** — Works for Claude Code executor; broken for Gemini/Qwen (no MCP tools)
5. **Agent accesses project tools** — Partially works via `default_mcp.json`; MCP Settings UI is disconnected
6. **Agent self-evaluates** — No mechanism (completion_criteria not sent to UI, agent flows not orchestrated)
7. **Review/approval** — Backend approval routes exist; UI has approval toggle but no review workflow UI
8. **Workflow → Task pipeline** — Completely missing

---

## 6. Recommended Priority Actions

### Tier 1 — High Impact, Moderate Effort

| # | Action | Addresses |
|---|--------|-----------|
| P1 | **Add completion_criteria + output_format to TaskFormDialog** | U1 |
| P2 | **Add "Agent Task" node type to Workflow Builder** | F11, F13 |
| P3 | **Add ORCHA Task Server to MCP popular servers carousel** | U13 |
| P4 | **Create agent execution dashboard** (active tasks, queue, history) | U11 |
| P5 | **Wire ACP agents to MCP servers** (Phase 3G) | F5 |

### Tier 2 — High Impact, Higher Effort

| # | Action | Addresses |
|---|--------|-----------|
| P6 | **Build scaffold_project UI** — project templates accessible from "New Project" | F1, U7 |
| P7 | **Unify MCP config** — Settings → MCP should configure executor pipeline, not just CLI | F9, U15 |
| P8 | **Build agent flow orchestration engine** (Phase 3H) | F6 |
| P9 | **Add workflow trigger system** (cron, event, webhook) | F12 |
| P10 | **Enhance task templates** with agent fields, completion criteria, MCPs | U4 |

### Tier 3 — Quality of Life

| # | Action | Addresses |
|---|--------|-----------|
| P11 | Add MCP autocomplete to task form (browse available servers/tools) | U5 |
| P12 | Add task dependency creation/visualization UI | U8 |
| P13 | Add subtask creation UI (parent_task_id) | U2 |
| P14 | Add agent capability detail view (tools, model, history, success rate) | U9 |
| P15 | Connect automations to workflow builder (visual automation creation) | F15 |

---

## 7. Screenshots Reference

| File | Description |
|------|-------------|
| `01-create-project-dialog.png` | Project creation dialog |
| `02-create-task-form.png` | Task creation form (full fields) |
| `03-empty-kanban-board.png` | Empty Kanban board |
| `04-workflows-page.png` | Workflows page with Builder tab |
| `05-automations-tab.png` | Automations tab (5 hourly jobs) |
| `06-my-tasks-empty.png` | My Tasks empty state |
| `07-settings-page.png` | Settings → General (task execution, templates) |
| `08-mcp-settings.png` | Settings → MCP Servers (JSON editor, popular servers) |
| `09-agents-settings.png` | Settings → Agents (13 agent cards) |
| `10-workflows-builder.png` | Workflow Builder with system workflow |
| `11-workflow-editor.png` | Visual workflow editor (node list + config panel) |
| `12-workflow-node-config.png` | Node configuration (LLM Extract with prompt template) |
| `13-project-page.png` | Project detail page (Dashboard Bug Reports) |
| `14-create-task-dialog.png` | Create Task dialog from project context |

---

## 8. i18n Note

The Settings pages generate ~100+ `i18next::translator: missingKey` console warnings. All settings labels are untranslated (using raw English keys). This is a cosmetic issue but indicates the i18n system needs settings namespace translations.

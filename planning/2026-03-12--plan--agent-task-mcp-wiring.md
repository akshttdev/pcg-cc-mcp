# Agent-Task Integration Enhancement Plan

**Date:** 2026-03-12
**Branch:** `feature/blob-to-text-scoped` (originally `feature/fraze-2026-03-12`, ported to main-based branch)
**Status:** MOSTLY COMPLETE — Phases 1, 2, 3F, 3G, 3I done. 3H (orchestration engine) not started. All bug fixes verified via Playwright (`ff6b76654`).

---

## Context

Reviewed the current task management system against ATLAS Task MCP to identify gaps in agent interaction with the project management system. Found that while the built-in MCP task server (`mcp_task_server`) has 23 tools, it is **not connected to any agents** — executors are configured to use the external `duck-kanban` npm package instead.

### Current Architecture Gaps

1. **MCP server misconfiguration** — `default_mcp.json` registers `duck_kanban` (external npm), not the built-in `mcp_task_server` binary
2. **ACP agents get no MCP servers** — `harness.rs` passes `mcp_servers: vec![]` to Gemini/Qwen sessions
3. **No orchestration engine** — agent flows exist as data model but no background worker progresses phases
4. **No completion criteria** — tasks have no structured "definition of done" for agents to self-evaluate
5. **No cross-entity search** — MCP search only covers tasks, not projects/knowledge/persons
6. **No research scaffolding** — agents can't auto-create research projects from a topic
7. **Dependencies informational only** — no way for agents to check if blockers are resolved

---

## Implementation Plan

### Phase 1: Foundation (Current Sprint — Tier 1)

#### A. Task Completion Criteria Fields
- **What:** Add `completion_criteria` and `output_format` columns to `tasks` table
- **Why:** Agents need structured success criteria to self-evaluate task completion
- **Files:** migration `20260323000000`, `task.rs` (model + all queries), `task_server.rs` (MCP tools), all `CreateTask` sites across 8 crates
- **Status:** DONE — migration, model, all SQL queries, MCP request/response types, summary builders, create/update handlers, all 13 `CreateTask` call sites updated

#### B. MCP Resources for Read-Only Browsing
- **What:** Expose `orcha://projects`, `orcha://projects/{id}/tasks`, etc. as MCP resources
- **Why:** Agents can discover project structure without tool calls (lower token overhead)
- **Files:** `task_server.rs` (resource templates + resolve)
- **Status:** DONE — renamed `pcg://` → `orcha://` across all 6 resource templates and resolver

#### C. Check Dependencies Tool
- **What:** New MCP tool `check_dependencies` — returns whether all blockers are resolved
- **Why:** Agents can gate their own work on dependency completion
- **Files:** `task_server.rs` (new tool), uses existing `TaskDependency` model
- **Status:** DONE — tool returns structured report with all_resolved, blocker details, resolution status

### Phase 2: Intelligence (Current Sprint — Tier 2)

#### D. Unified Cross-Entity Search
- **What:** New MCP tool `unified_search` — searches across projects, tasks, knowledge, persons
- **Why:** Agents need to discover related context across entity types
- **Files:** `task_server.rs` (new tool), uses existing `Person::list()` with query filter
- **Status:** DONE — searches across projects, tasks, knowledge, persons with entity_type filtering and project scoping

#### E. Deep Research / Scaffold Project Tool
- **What:** New MCP tool `scaffold_project` — auto-creates project with tasks from a template
- **Why:** Agents should be able to plan and structure projects autonomously
- **Files:** `task_server.rs` (new tool), uses existing `Project::create` and `Task::create`
- **Status:** DONE — 4 templates (software, research, marketing, client_onboarding) with completion_criteria and output_format per task. Research and client_onboarding are deeply invested with contextual task descriptions

### Phase 3: Wiring (Future — Critical for Agent Autonomy)

#### F. Connect Built-In MCP Server to Executors
- **What:** Replace `duck_kanban` in `default_mcp.json` with built-in `mcp_task_server`
- **Why:** Currently THE biggest blocker — agents literally cannot access task tools
- **Files:** `crates/executors/default_mcp.json`, build `mcp_task_server` binary
- **Impact:** All executor agents (Claude Code, Gemini, Duck, Codex, Qwen) gain task management
- **Status:** DONE — `orcha_task_server` added to `default_mcp.json` alongside existing servers (duck_kanban retained for backward compat)
- **Dependency:** Phase 1 features should land first so agents get the enhanced toolset

#### G. Wire ACP Agents to MCP
- **What:** Update `harness.rs` to forward MCP server config to ACP sessions
- **Why:** Gemini/Qwen agents currently get `mcp_servers: vec![]` — no tools at all
- **Files:** `crates/executors/src/executors/acp/harness.rs`
- **Status:** DONE — `load_platform_mcp_servers()` reads `default_mcp.json`, converts to `proto::McpServer` format, passes to ACP sessions. Both Stdio and HTTP configs supported.

#### H. Agent Flow Orchestration Engine
- **What:** Background worker that progresses agent flow phases, enforces gates, handles delegation
- **Why:** Agent flows exist as data model only — no execution engine
- **Files:** New worker in `crates/server/src/`, reads `agent_flows`/`agent_flow_events` tables
- **Status:** NOT STARTED — largest effort item

#### I. Autonomy Level Enforcement
- **What:** Check autonomy settings before allowing task execution/approval
- **Why:** Routes exist but are never checked — agents bypass all governance
- **Files:** `crates/server/src/routes/autonomy.rs`
- **Status:** DONE — Full autonomy API (autonomy mode get/set, checkpoint CRUD, execution checkpoint trigger/review, approval gates, can-proceed validation) implemented in `autonomy.rs`. `can_execution_proceed()` endpoint verifies all gates before task execution.

---

## Implementation Order

```
Phase 1 (A → C → B) — Foundation
  ↓
Phase 2 (D → E) — Intelligence
  ↓
Phase 3 (F → G → H → I) — Wiring & Orchestration
```

Phase 1 and 2 enhance the MCP tool surface (now 26 tools + 6 resources). Phase 3 connects it to agents.

---

## Feature Comparison: ORCHA vs ATLAS

| Capability | ORCHA (Current) | ATLAS MCP | After Enhancement |
|---|---|---|---|
| Completion criteria | ✗ | ✓ (required fields) | ✓ (Phase 1A) |
| MCP resources | Partial (`pcg://`) | ✓ (`atlas://`) | ✓ (Phase 1B) |
| Dependency checking | Informational only | ✓ (completion check) | ✓ (Phase 1C) |
| Cross-entity search | Tasks only | ✓ (unified) | ✓ (Phase 2D) |
| Research scaffolding | ✗ | ✓ (deep_research) | ✓ (Phase 2E) |
| Agent execution | ✓ (but disconnected) | ✗ | ✓ (Phase 3F) |
| Approval workflows | ✓ | ✗ | ✓ (existing) |
| Multi-agent assignment | ✓ | ✗ | ✓ (existing) |
| VIBE budgets | ✓ | ✗ | ✓ (existing) |
| UI (Kanban, detail) | ✓ | ✗ | ✓ (existing) |

---

## Decisions & Notes

- **Not adopting Neo4j now** — SQLite works well, graph queries are simple enough in SQL
- **ATLAS integration deferred for evaluation** — not running alongside now, but revisit as a potential MCP-facing facade for external/third-party agent interop. Key tradeoffs:
  - *Pro*: Standardized schema agents already know; inherit upstream improvements; graph model natural for dependencies/knowledge
  - *Con*: Two sources of truth (dashboard reads SQLite, agents read ATLAS); Neo4j adds operational complexity; ORCHA has features ATLAS lacks (VIBE budgets, approval workflows, execution attempts, worktree mgmt, SSE streaming); tight coupling between Task model and execution pipeline
  - *Hybrid option*: ATLAS as MCP facade with bidirectional sync adapter — ORCHA remains system of record, ATLAS provides standardized agent interface. Worth evaluating if we need to support external agent ecosystems beyond ORCHA-managed executors
  - *Decision*: Enhance built-in MCP server first (current sprint), evaluate ATLAS hybrid integration as separate initiative
- **Adopting ATLAS ideas** — completion criteria, resources, unified search, research scaffolding
- **Phase 3F included in this sprint** — connecting the built-in MCP server to executors is low-effort/high-impact; without it, new tools are unreachable by agents
- **duck-kanban can be replaced** — the built-in server is a strict superset of its features
- **Project templates**: Create templates for all categories (software, research, marketing, client onboarding), invest deeply in research/intelligence and client onboarding as highest-impact
- **Nora tool exposure**: Expose Nora's internal tools (delegate_task, assign_task_to_agent) via MCP, but with namespacing and access control gates — agents should not be able to escalate privileges or access cross-org data without authorization
- **MCP namespacing**: Tools exposed by the built-in server should be prefixed/scoped (e.g., `orcha.create_task`) to avoid collisions if other MCP servers are co-registered; access control should respect the requesting agent's role and org membership

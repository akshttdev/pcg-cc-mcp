# Sprint Implementation Issues & Notes — 2026-03-12

**Companion docs:** `2026-03-12-improvement-roadmap.md`, `2026-03-12-dashboard-task-agent-usability-review.md`

---

## Issues Found During Implementation

### Sprint 1

#### 1A. Completion Criteria in Task Form
- **RESOLVED**: `completion_criteria` and `output_format` fields added to `TaskFormDialog.tsx` (state, form fields, create/update payloads)
- **Issue found**: `ImportDialog.tsx` was missing `completion_criteria` and `output_format` in its `CreateTask` call → fixed by adding `null` values

#### 1B. Project Scaffolding UI
- **RESOLVED**: Template selector added to `ProjectFormDialog.tsx` with 4 templates (software, research, marketing, client_onboarding)
- **Issue found**: `handleDirectCreate` was not calling `scaffoldTemplateTasks()` after project creation → fixed
- **Design note**: Templates create tasks via sequential `tasksApi.create()` calls because the `scaffold_project` MCP tool doesn't have a REST endpoint

#### 1C. ORCHA Task Server in MCP Popular Servers
- **Status**: Already configured in `default_mcp.json` with meta entry → server appears in carousel
- **Issue found**: Carousel only shows 3 items at `sm:basis-1/3`, so ORCHA might not be visible without scrolling — low priority viewport issue

#### 1D. Verify output_tasks Node
- **RESOLVED**: `output_tasks` confirmed fully working in backend (`execute_node_with_llm`) — extraction → staging → approval → commit flow
- **No issues found**

#### 1E. Agent Execution Dashboard
- **Status**: Mission Control page already exists at `/mission-control` with `ExecutionTimeline`, `useExecutionEvents` hook
- **No new page needed** — just needs better discoverability (link from agent cards)

---

### Sprint 2

#### 2A. New Node Types (Frontend)
- **RESOLVED**: 7 new node types added to `WorkflowEditor.tsx` NODE_TYPES array:
  - `conditional`, `send_notification`, `assign_to_agent`, `http_request`
  - `update_crm_contact`, `update_crm_deal`, `update_crm_company`
- Full configuration panels and NodePicker categories implemented
- **Icons added**: GitBranch, Bell, Bot, Globe, PenSquare, RefreshCw, Building2

#### 2A. New Node Types (Backend)
- **RESOLVED**: `execute_action_node()` function added to `data_source_workflows.rs`
- All 7 node types have backend execution logic
- **Design decisions**:
  - `conditional` supports `count>N`, `contains:keyword`, and simple presence checks
  - `http_request` uses `reqwest` with 30s timeout, supports GET/POST/PUT/DELETE/PATCH
  - `assign_to_agent` creates real tasks via `Task::create()` with workflow context
  - CRM update nodes use `find_by_email` (contacts), `find_by_name_and_org` (deals), `find_by_name` (companies)
  - Context IDs (`project_id`, `organization_id`) threaded from data source through to action nodes
- **Potential issue**: CRM update nodes require `organization_id` in the input records or from workflow context. If neither is available, updates will fail silently with logged errors.

#### 2B. Convert Automations to System Workflows
- **Deferred**: Kept existing hardcoded automations running. Created equivalent workflow node types that enable building the same logic in the workflow builder.
- **Rationale**: Per user guidance — "platform-level can stay hardcoded" + the new node types now make it possible to build these as user workflows

#### 2C. Workflow Trigger System
- **RESOLVED**: Extended `WorkflowTrigger` model with `find_due_schedules()` for `schedule` trigger type
- **RESOLVED**: `spawn_workflow_schedule_loop()` background worker added, checks every 5 minutes
- Supports intervals: `every_5m`, `every_15m`, `every_30m`, `hourly`, `daily`, `weekly`
- Interval config stored in `filter_tags` JSON field: `{"interval": "hourly"}`
- **Design note**: Schedule triggers run without a data source (empty content), relying on action nodes

#### 2D. System Workflows
- **RESOLVED**: 4 new system workflows seeded:
  1. **Bug Triage Pipeline**: `llm_analyze` → `conditional` → `output_tasks`
  2. **Sprint Planning**: `llm_extract` → `output_tasks`
  3. **Client Onboarding**: `llm_extract` → `output_crm_contacts` + `output_tasks`
  4. **Content Pipeline**: `llm_summarize` → `llm_analyze` (SEO) → `output_tasks`

---

### Sprint 3

#### 3A. Wire ACP Agents to MCP
- **RESOLVED**: `harness.rs` updated to load `default_mcp.json` and pass MCP servers to ACP sessions
- `load_platform_mcp_servers()` converts JSON config to `proto::McpServer` enum (Stdio, Http)
- Searches multiple paths: `crates/executors/default_mcp.json`, `./default_mcp.json`, exe-relative
- **Potential issue**: File path for `default_mcp.json` depends on working directory at runtime. If the server runs from a different cwd, MCP servers won't load. Logged at debug level.

#### 3B. Platform vs User MCP Config Separation
- **Status**: Already separate by design
  - Platform: `default_mcp.json` → executor pipeline
  - User: `~/.claude.json` → personal CLI/workflow
- **No code changes needed** — config separation is architectural, not a code issue
- **Future**: Add admin-only "Platform MCP Config" page when needed

#### 3C. Enhanced Task Templates
- **RESOLVED**: Migration `20260324000000` adds 6 new columns to `task_templates`:
  - `priority`, `completion_criteria`, `output_format`, `assigned_agent`, `tags`, `organization_id`
- Rust model updated with new fields, org-scoped query added
- **Issue found**: `TaskTemplateEditDialog.tsx` needed update for new required TS fields → fixed with null defaults

#### 3D. Agent Capability Profiles
- **RESOLVED**: `GET /api/agents/:id/profile` endpoint added
- Returns: agent details, execution stats (total/completed/failed/in_progress/success_rate), recent attempts, MCP tools list
- Stats queried from `task_attempts` table by `agent_name`
- **Frontend integration needed**: Agent cards in Settings should link to this profile data (future frontend work)

#### 3E. Agent Flow Orchestration (Design Only)
- **Status**: Per roadmap, this is design-only for Sprint 3
- The workflow trigger system (2C) and assign_to_agent node (2A) provide foundational pieces
- Full orchestration engine (multi-phase, multi-agent) deferred to dedicated sprint

---

## Files Modified

### Backend (Rust)
| File | Changes |
|------|---------|
| `crates/server/src/routes/data_source_workflows.rs` | Added 7 action node types, 4 system workflows, schedule loop, imports |
| `crates/server/src/main.rs` | Registered `spawn_workflow_schedule_loop` |
| `crates/executors/src/executors/acp/harness.rs` | Added `load_platform_mcp_servers()`, pass MCP to ACP sessions |
| `crates/db/src/models/workflow_trigger.rs` | Added `find_due_schedules()` for schedule triggers |
| `crates/db/src/models/task_template.rs` | Added 6 new fields, org-scoped query, updated all CRUD |
| `crates/server/src/routes/agents.rs` | Added `/agents/:id/profile` endpoint |
| `crates/db/migrations/20260324000000_enhance_task_templates.sql` | New migration |

### Frontend (TypeScript/React)
| File | Changes |
|------|---------|
| `frontend/src/components/workflows/WorkflowEditor.tsx` | 7 new node types, config panels, NodePicker categories |
| `frontend/src/components/dialogs/tasks/TaskFormDialog.tsx` | completion_criteria, output_format fields |
| `frontend/src/components/dialogs/projects/ProjectFormDialog.tsx` | Template scaffolding UI |
| `frontend/src/components/export/ImportDialog.tsx` | Added missing CreateTask fields |
| `frontend/src/components/dialogs/tasks/TaskTemplateEditDialog.tsx` | Updated for new template fields |
| `shared/types.ts` | Regenerated with new types |

---

## Known Limitations & Future Work

1. **Conditional node**: Only supports simple string matching and `count>N` — complex boolean expressions need LLM preprocessing
2. **CRM update nodes**: Match by email (contacts) or name (deals/companies) only — no fuzzy matching
3. **http_request node**: No OAuth/auth token management — headers must be static
4. **send_notification node**: Logs only — no actual email/in-app delivery (placeholder for future integration)
5. **ACP MCP loading**: Path resolution depends on server working directory
6. **Task template UI**: Backend supports new fields but the `TaskTemplateEditDialog` doesn't expose them yet (passes null)
7. **Agent profile endpoint**: Frontend doesn't consume it yet — needs UI integration in Settings → Agents

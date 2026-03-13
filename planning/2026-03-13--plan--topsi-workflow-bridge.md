# Topsi–Workflow Bridge Implementation Plan

**Date:** 2026-03-13 (updated after Phase 0+1 completion)
**Branch:** `feature/topsi-workflow-bridge`
**PR:** #22
**Depends on:** PR #21 (LLM dogfooding infrastructure)
**Related:** `2026-03-12--plan--three-sprint-roadmap.md`, `2026-03-12--plan--agent-task-mcp-wiring.md`

---

## Context

The workflow editor's LLM execution uses `pcg_router::route_completion()` — a database-backed routing layer with 8+ providers and cost tracking but **no tool calling, no conversation history, no structured output**. Nora's `LLMClient` has tool calling but only 3 providers. Topsi has zero tools for workflows, CRM, or project data.

**Goal:** Build a new multi-provider LLM service with tool-calling support for all 8 PCG Router providers, then bridge Topsi into the workflow/CRM ecosystem.

### Decisions
- **LLM Layer:** New abstraction over PCG Router (all 8 providers + tool calling), not wrapping Nora's LLMClient
- **Execution priority:** Phase 0 + Phase 1 in parallel
- **Workflow refactor:** Full service extraction first (~2000+ lines from route handler), then Topsi triggers
- **Node output mode:** Configurable per node — text (with repair) or structured (tool-call JSON)
- **CRM scope:** Validate requested org_id against user's `organization_members` membership
- **System prompt:** Admin-editable via `system_settings` table, with SudoLang mode toggle
- **Confirmation:** Moved earlier (Phase 3) since unprotected write tools already exist

### Two Separate LLM Stacks

| | PCG Router | Nora LLMClient |
|---|---|---|
| **Location** | `crates/server/src/routes/pcg_router.rs` | `crates/nora/src/brain/mod.rs` |
| **Providers** | 8+ (Anthropic, OpenAI, Gemini, Grok, Mistral, DeepSeek, Groq, Qwen) | 3 (Anthropic, OpenAI, Ollama) |
| **Model registry** | DB-backed (`pcg_router_models`), priority-based fallback | Config-based |
| **Cost tracking** | Yes (DB logging + `RoutingMetadata`) | No |
| **Tool calling** | **Yes** (via WorkflowLLMService) | Yes |
| **Provider dispatch** | 3 categories: Anthropic, OpenAI-compat (7), Gemini | Per-provider trait impl |

---

## Phase 0: Multi-Provider LLM Service with Tool Calling — DONE ✅

Built `WorkflowLLMService` in `crates/services/src/services/workflow_llm.rs` (772 lines).

### What shipped
- **`completion()`** — drop-in replacement for `route_completion()`, returns String directly
- **`completion_with_tools()`** — sends tool definitions in provider-native format, returns `LLMResponse::Text` or `LLMResponse::ToolCalls`
- **`completion_with_structured_output()`** — JSON extraction via single-tool-call pattern (`extract_data` tool)
- Provider dispatch for Anthropic (native format), OpenAI-compat (7 providers), Gemini
- Priority-based model fallback with `supports_tools` filtering
- Message helpers for multi-turn tool conversations
- Wired into `execute_node_with_llm()` — the single LLM entry point for all 4 workflow call sites

### QA findings fixed
- Removed dead `resolve_model()` method (duplicated `resolve_candidates()`)
- Fixed UTF-8 boundary risk in error message truncation

### Not yet exercised
- `completion_with_tools()` only called internally by `completion()` with empty tools
- `completion_with_structured_output()` available but not used by any node type yet
- These will be activated in Phase 1B (node output modes) and Phase 2B (Topsi triggers)

---

## Phase 1: Topsi Reads the System — DONE ✅

### What shipped (7 new tools)
| Tool | Description | Scope |
|------|-------------|-------|
| `get_project_detail` | Project + task count breakdown | AccessScope-filtered |
| `list_crm_contacts` | Org-scoped contacts with search/lifecycle filter | org_id required |
| `list_crm_deals` | Deals by org/pipeline/stage | org_id or pipeline_id |
| `list_crm_pipelines` | Pipelines with stages | org_id required |
| `list_workflow_definitions` | Saved workflows | System=all, org=admin-only |
| `get_workflow_definition` | Full definition with nodes | scope param (not yet validated) |
| `search_entities` | Cross-entity keyword search | Partial (projects/tasks unscoped) |

### QA findings fixed
- 3 critical BLOB-as-String extraction bugs (crm_deals.id, workflow_definitions.owner_id)
- Added AccessScope filtering to workflow definition tools
- Added owner_id to list_workflow_definitions response

### Phase 1B: Gap Closures — DONE ✅

#### 1B.1: CRM Org-Scope Validation ✅
Added `verify_org_membership()` helper that checks `organization_members` table. Admin bypasses. All CRM tools (`list_crm_contacts`, `list_crm_deals`, `list_crm_pipelines`, `search_entities`) now require `user_context` and validate org membership before executing queries.

#### 1B.2: System Prompt Update ✅
Added CRM & Data tools section to `TOPSI_SYSTEM_PROMPT` with descriptions for all 7 tools. Added CRM query examples to the "Gather data first" section.

#### 1B.3: Admin-Editable System Prompt with SudoLang Toggle ✅
Added `get_effective_system_prompt()` method that loads from `system_settings`:
- `topsi_system_prompt` — custom standard prompt (falls back to compiled-in const)
- `topsi_prompt_mode` — `"standard"` (default) or `"sudolang"`
- `topsi_system_prompt_sudolang` — SudoLang variant

No new migration needed — uses existing `system_settings` table. All 3 LLM call sites in the agentic loop now use the DB-backed prompt. Admin can edit via existing `PUT /api/config/system-settings/{key}` endpoint.

#### 1B.4: Node Output Mode (Configurable) ✅
Added `output_mode` parameter to `execute_node_with_llm()`:
- `"text"` — returns raw LLM text, no JSON parsing or repair
- `"structured"` — forces JSON parsing + repair retry (previous default behavior)
- `"auto"` (default) — structured if `target_schemas` present, text otherwise

System prompt also adapts: structured mode gets extraction-focused prompt, text mode gets general-purpose prompt.

**Remaining for future:** Frontend workflow editor UI for output_mode dropdown.

---

## Phase 2: Service Extraction + Topsi Triggers

### 2A: Extract workflow execution into service (FIRST)
**Full extraction** of workflow execution logic from `data_source_workflows.rs` (~2000+ lines) into `crates/services/src/services/workflow_execution.rs`.

**What moves:**
- `execute_node_with_llm()` and all node execution logic
- Topological sort / dependency resolution
- Staging record creation (`WorkflowStagingRecord`)
- Commit logic (staging → CRM/task creation)
- Mock/fallback extraction
- Schema prompt generation (`build_schema_prompt_text()`)

**What stays in route handler:**
- HTTP endpoint handlers (thin wrappers)
- Request/response type definitions
- Router setup

**Service API:**
```rust
pub struct WorkflowExecutionService;

impl WorkflowExecutionService {
    pub async fn execute(pool, definition, trigger_data, org_id) -> Result<WorkflowRun>
    pub async fn execute_preview(pool, definition, content, org_id) -> Result<Vec<PreviewNodeResult>>
    pub async fn get_run_status(pool, run_id) -> Result<WorkflowRun>
    pub async fn list_runs(pool, filters) -> Result<Vec<WorkflowRun>>
    pub async fn commit_staged_data(pool, run_id, approver_id) -> Result<CommitResult>
}
```

### 2B: Topsi workflow trigger tools
| Tool | Description | Uses |
|------|-------------|------|
| `trigger_workflow` | Execute a saved workflow by ID | `WorkflowExecutionService::execute()` |
| `get_workflow_run_status` | Check running workflow status | `WorkflowExecutionService::get_run_status()` |
| `list_workflow_runs` | List recent executions | `WorkflowExecutionService::list_runs()` |
| `review_staged_data` | Show staged CRM data pending review | `WorkflowStagingRecord::find_by_run()` |
| `commit_staged_data` | Approve and commit staged output | `WorkflowExecutionService::commit_staged_data()` |

### Files
- **NEW** `crates/services/src/services/workflow_execution.rs` (~2000 lines extracted)
- **MODIFY** `crates/server/src/routes/data_source_workflows.rs` — thin handlers delegating to service
- **MODIFY** `crates/topsi/src/tools/mod.rs` + `agent.rs` — 5 new tools

---

## Phase 3: Topsi Writes + Configurable Confirmation

**Moved earlier** because unprotected write tools (`create_task`, `update_task`, `create_project`, `update_project`, `start_task_execution`) already exist in Topsi with zero confirmation gates.

### Confirmation system

**Infrastructure already exists:**
- `TopsiConfig.autonomy_level` enum: `Full`, `Supervised`, `ApprovalRequired`, `Manual` — defined but **not enforced**
- `system_settings` table — can store per-tool overrides
- `organization_members` — role-based access (Admin, Member, Viewer)

**Implementation:**
1. Add confirmation check before all write tool executions in `execute_tool_calls()`
2. Tool classification:
   - **Green** (reads): always execute — all Phase 1 tools
   - **Yellow** (creates/updates): execute by default, confirmable — `create_task`, `update_task`, `create_project`, `update_project`, `create_crm_contact`, `update_crm_deal`, `create_crm_deal`
   - **Red** (deletes, bulk, execution): confirm by default — `delete_task`, `bulk_update_tasks`, `start_task_execution`, `trigger_workflow`, `commit_staged_data`
3. Settings via `system_settings`:
   - `topsi_confirmation_mode` → `"confirm_destructive"` (default), `"always_confirm"`, `"autonomous"`
   - `topsi_tool_overrides` → JSON: `{"create_task": "always_confirm", "start_task_execution": "autonomous"}`
4. Pending action pattern: return `{"pending_confirmation": true, "action": "description"}` and store in conversation state

### New write tools (if not already present)
| Tool | Risk | Description |
|------|------|-------------|
| `create_crm_contact` | Yellow | Create CRM contact in org |
| `update_crm_deal` | Yellow | Update deal fields |
| `create_crm_deal` | Yellow | Create new deal |
| `delete_task` | Red | Delete a task |

### Files
- **MODIFY** `crates/topsi/src/agent.rs` — confirmation gate in `execute_tool_calls()`
- **MODIFY** `crates/topsi/src/tools/mod.rs` — new CRM write tool schemas
- **NO new migration** — uses existing `system_settings` key-value store

---

## Phase 4: Topsi Builds Workflows (NL → Node Graph)

Tools: `create_workflow`, `modify_workflow`.

### Implementation
- Use `WorkflowLLMService::completion_with_tools()` with node type registry as LLM context
- Provide all 6 node types with their parameter schemas as tool definitions
- Multi-turn: Topsi asks clarifying questions → generates node JSON → validates → saves
- Save via raw SQL insert to `workflow_definitions` table
- Validate via `WorkflowExecutionService::execute_preview()` (dry run)

### Files
- **MODIFY** `crates/topsi/src/tools/mod.rs` + `agent.rs` — 2 new tools
- **REUSE** `WorkflowLLMService` for the generation LLM call
- **REUSE** `WorkflowExecutionService` for validation

---

## Phase 5: Bidirectional UI

1. **Topsi action feed** — tool calls appear as activity in project/CRM feeds
2. **"Ask Topsi" context button** — from project/task/CRM views, pre-loads entity context
3. **Workflow progress in chat** — SSE events for workflow execution status
4. **Admin settings UI** — System prompt editor with SudoLang toggle, confirmation preferences
5. **Topsi settings per-user** — per-tool confirmation overrides

### Files
- **MODIFY** frontend components (project detail, task detail, CRM views)
- **MODIFY** `crates/topsi/src/agent.rs` — emit SSE events for tool executions
- **NEW** frontend admin settings page for Topsi prompt + confirmation config
- **REUSE** existing SSE infrastructure (`/api/events/...`)

---

## Updated Implementation Order

```
Phase 0 (WorkflowLLMService)  ──┐
                                 ├── DONE ✅
Phase 1 (Topsi reads)          ──┘
    ↓
Phase 1B (Close gaps: org-scope, system prompt, node output modes)
    ↓
Phase 2A (Service extraction ~2000 lines)
    ↓
Phase 2B (Topsi trigger/commit tools)  ──┐
                                          ├── CAN PARALLEL
Phase 3 (Confirmation system)           ──┘
    ↓
Phase 4 (Topsi builds workflows — NL → node graph)
    ↓
Phase 5 (Bidirectional UI + admin settings)
```

---

## Key Infrastructure Discovered During Audit

| Resource | Location | Status |
|----------|----------|--------|
| `organization_members` table | Migration 20251004000000 | Exists — `(org_id, user_id, role)` |
| `Organization::get_user_role()` | `db/models/user.rs` | Exists — returns role for user in org |
| `system_settings` table | Migration 20260327000000 | Exists — key-value store with `get()`/`set()` |
| `TopsiConfig.autonomy_level` | `topsi/src/config.rs` | Exists — `Full/Supervised/ApprovalRequired/Manual` — NOT enforced |
| `TopsiConfig.system_prompt` | `topsi/src/config.rs` | Exists — optional override field, not wired to DB |
| `AccessScope` enum | `topsi/src/agent/access_control.rs` | Project-scoped only — no org awareness |
| `WorkflowNode.parameters` | `data_source_workflows.rs:36-44` | Untyped JSON — can add `output_mode` without schema change |

---

## Critical Files Reference

| File | Role |
|------|------|
| `crates/services/src/services/workflow_llm.rs` | WorkflowLLMService (Phase 0 — DONE) |
| `crates/server/src/routes/data_source_workflows.rs` | Workflow engine (~4070 lines, to be extracted in Phase 2A) |
| `crates/server/src/routes/pcg_router.rs` | Model registry, direct API route (KEEP) |
| `crates/db/src/models/pcg_router_model.rs` | Model registry DB model |
| `crates/topsi/src/tools/mod.rs` | Topsi tool schemas |
| `crates/topsi/src/agent.rs` | Topsi agent loop, tool dispatch (~3730 lines) |
| `crates/topsi/src/agent/access_control.rs` | AccessScope (project-only, needs org extension) |
| `crates/topsi/src/config.rs` | TopsiConfig with autonomy_level + system_prompt |
| `crates/db/src/models/system_settings.rs` | Key-value settings store |
| `crates/db/src/models/user.rs` | User model with org membership queries |

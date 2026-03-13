# Topsi–Workflow Bridge Implementation Plan

**Date:** 2026-03-13 (updated after Phase 5 completion — all phases done)
**Branch:** `feature/topsi-workflow-bridge`
**PR:** #22
**Depends on:** PR #21 (LLM dogfooding infrastructure), PR #23 (dogfood pipeline), PR #24 (QA review fixes) — all merged to main
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

## Phase 2: Service Extraction + Topsi Triggers — DONE ✅

### 2A: Extract unified execution engine ✅
Extracted `WorkflowExecutionResult`, `ExecutionOptions`, `execute_workflow_nodes()`, `finalize_workflow_run()`, `check_for_llm_errors()`, and `auto_approve_staged_records()` into `crates/server/src/routes/workflow_engine.rs`. This shared engine is used by run_workflow, preview_workflow, fire_triggers, and schedule triggers.

**Auto-approve encapsulation (post-Phase 2F):** Moved ~105-line inline auto-approve block from `fire_triggers_for_data_source()` into `auto_approve_staged_records()` in the engine. Added `auto_approve: bool` field to `ExecutionOptions` and `MAX_AUTO_APPROVE_RECORDS = 50` rate limit guard. The full pipeline (approve valid → reject duplicates → commit in dependency order → link contacts to companies → auto-start agent execution) is now reusable by any caller.

### 2B: Topsi workflow trigger/review tools ✅
Added 5 tools: `list_workflow_runs`, `get_workflow_run_status`, `review_staged_data`, `approve_staged_records`, and `build_workflow` (specialist delegation).

### 2C: Workflow Builder specialist ✅
Added `crates/topsi/src/workflow_builder.rs` — specialist agent that generates workflow node graphs from natural language. Topsi delegates to it via the `build_workflow` tool. Contains full node type registry, output schema reference, and validation.

### 2D: CRM write tools ✅
Added `create_crm_contact`, `create_crm_deal`, `update_crm_deal` tools.

### 2E: Platform Data Service extraction ✅
Extracted all 22 database CRUD tools from `agent.rs` into `crates/topsi/src/platform_data.rs` (PlatformDataService). Agent.rs reduced from ~3420 to ~2382 lines. PlatformDataService is reusable by any agent.

### 2F: Full service extraction ✅
Extracted ~2200 lines of workflow execution logic from `data_source_workflows.rs` into `crates/services/src/services/workflow_execution.rs`. Includes text extraction, mock LLM generation, record helpers, dedup checks, schema validation, action node execution, and LLM node execution. Route handler reduced from 3523 to 1337 lines (thin handlers only). `workflow_engine.rs` imports directly from the service.

**What stays in route handler:**
- HTTP endpoint handlers (thin wrappers)
- Request/response type definitions
- Router setup

**Extracted API** (free functions, not a struct):
```rust
// crates/services/src/services/workflow_execution.rs — core logic (~2400 lines)
pub fn execute_node_with_llm(), execute_action_node()
pub fn extract_records_from_output(), check_*_duplicate()
pub fn validate_record_against_schema(), compute_confidence()

// crates/server/src/routes/workflow_engine.rs — orchestration (~590 lines)
pub fn execute_workflow_nodes(), finalize_workflow_run()
pub fn auto_approve_staged_records(), check_for_llm_errors()
pub const MAX_AUTO_APPROVE_RECORDS: i64 = 50;
```

### Files
- **NEW** `crates/services/src/services/workflow_execution.rs` (~2000 lines extracted)
- **MODIFY** `crates/server/src/routes/data_source_workflows.rs` — thin handlers delegating to service
- **MODIFY** `crates/topsi/src/tools/mod.rs` + `agent.rs` — 5 new tools

---

## Phase 3: Topsi Writes + Configurable Confirmation — DONE ✅

### What shipped

**Per-user confirmation settings (new table):**
- Migration `20260313000000_topsi_user_settings.sql` — `topsi_user_settings` table
- Model `crates/db/src/models/topsi_user_settings.rs`:
  - `ConfirmationMode` enum: `AlwaysConfirm`, `ConfirmDestructive` (default), `Autonomous`
  - `ToolRisk` enum: `Green` (reads), `Yellow` (creates/updates), `Red` (deletes/bulk)
  - `classify_tool_risk()` — maps tool names to risk levels
  - `TopsiUserSettings::get_or_default()` / `upsert()` — per-user settings with per-tool JSON overrides
  - `confirmation_mode_for_tool()` — resolves overrides → default → risk classification

**Confirmation gate in agent.rs:**
- Loads user settings once per tool batch
- Before executing non-Green tools, checks `requires_confirmation()`
- Returns `{"pending_confirmation": true, "action": "...", "risk_level": "..."}` instead of executing
- `describe_tool_action()` generates human-readable descriptions

**New Red-level tools:**
| Tool | Risk | Description |
|------|------|-------------|
| `delete_task` | Red | Permanently delete a task (with scope check) |
| `bulk_update_tasks` | Red | Batch update up to 100 tasks (status, priority, agent) |

**CRM write tools (shipped in Phase 2D):**
| Tool | Risk | Description |
|------|------|-------------|
| `create_crm_contact` | Yellow | Create CRM contact in org |
| `create_crm_deal` | Yellow | Create new deal |
| `update_crm_deal` | Yellow | Update deal fields |

**Default behavior:** Red tools require confirmation, Yellow execute immediately, Green always execute. Users can override per-tool via `per_tool_overrides` JSON column.

---

## Phase 4: Topsi Builds Workflows (NL → Node Graph) — DONE ✅

Consolidated into single `build_workflow` tool with `action: "create"|"modify"` parameter. Specialist agent in `workflow_builder.rs` (378 lines) generates node graphs from natural language.

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

## Phase 5: Bidirectional UI — DONE ✅

### What shipped

**Step 1: useAgentChatStore — Global Widget Control ✅**
- New `frontend/src/stores/useAgentChatStore.ts` — Zustand store (no persist)
- State: `widgetState`, `activeAgent` (default `'topsi'`), `pendingMessage`, `pendingContext`
- Actions: `openChat(message?, context?, agent?)`, `collapse()`, `clearPending()`, `setWidgetState()`
- TopsiWidget refactored: internal `useState<WidgetState>` replaced by store subscription
- Auto-sends pending messages when widget opens (via polling ref pattern)

**Step 2: Activity Type Extension + Tool Call Logging ✅**
- 3 new `ActivityType` variants: `agent_tool_call`, `agent_workflow_triggered`, `agent_workflow_completed`
- ActivityFeed icon map (robot/gear/check) and color map (cyan-500) updated
- TopsiWidget logs tool calls to ActivityStore after each chat response

**Step 3: "Ask Topsi" Context Button ✅**
- New `frontend/src/components/topsi/AskTopsiButton.tsx`
- Popover with entity-specific suggested questions + free-text input
- 4 entity types: project (4 questions), task (4), crm_contact (3), crm_deal (3)
- Injected into: project-tasks header, TaskDetailsPanel sidebar, crm-contact-detail header, CrmDealDetailPanel quick actions

**Step 4: Admin Settings — System Prompt Editor ✅**
- Backend: `GET/PUT /topsi/admin/prompt` in `topsi.rs` — admin-only, no debug gate
  - Keys: `topsi_system_prompt`, `topsi_prompt_mode`, `topsi_system_prompt_sudolang`
- Frontend: `TopsiAdminSettings.tsx` — Standard/SudoLang toggle, monospace textarea, reset/save
- Settings nav entry: "Topsi" under system admin scope

**Step 5: Per-User Topsi Settings ✅**
- Backend: `GET/PUT /topsi/user-settings` in `topsi.rs` — uses AccessContext.user_id
  - Delegates to `TopsiUserSettings::get_or_default()` / `upsert()`
- Frontend: `TopsiUserSettings.tsx` — confirmation mode select, per-tool override table (grouped by risk), auto-approve timeout
- Settings nav entry: "Topsi Preferences" under user scope

**Step 6: Workflow Progress in Chat (Poll-Based) ✅**
- TopsiWidget detects `trigger_workflow`/`build_workflow` tool calls with `workflow_run_id`
- Polls `GET /api/workflows/runs/{id}` every 5 seconds
- Shows progress and completion/failure summary in chat
- Logs `agent_workflow_triggered`/`agent_workflow_completed` to ActivityStore
- Cleans up poll intervals on unmount

**Topsi Activity Page ✅**
- New `frontend/src/pages/topsi-activity.tsx` — filters ActivityFeed to `agent_*` types
- Route: `/topsi-activity` (admin-only)
- Sidebar entry: "Topsi Activity" in admin nav section

### New Files (5)
| File | Purpose |
|------|---------|
| `frontend/src/stores/useAgentChatStore.ts` | Global widget state + openChat() (agent-agnostic) |
| `frontend/src/components/topsi/AskTopsiButton.tsx` | Context button + popover |
| `frontend/src/pages/topsi-activity.tsx` | Dedicated Topsi activity log |
| `frontend/src/pages/settings/TopsiAdminSettings.tsx` | System prompt editor |
| `frontend/src/pages/settings/TopsiUserSettings.tsx` | Per-user confirmation config |

### Modified Files (11)
| File | Changes |
|------|---------|
| `frontend/src/components/topsi/TopsiWidget.tsx` | Store-driven state, tool call logging, workflow polling |
| `frontend/src/types/activity.ts` | 3 new ActivityType variants |
| `frontend/src/components/activity/ActivityFeed.tsx` | Icons + colors for agent types |
| `frontend/src/pages/project-tasks.tsx` | AskTopsiButton in header toolbar |
| `frontend/src/components/tasks/TaskDetailsPanel.tsx` | AskTopsiButton in fullscreen sidebar |
| `frontend/src/pages/crm-contact-detail.tsx` | AskTopsiButton in header |
| `frontend/src/components/crm/CrmDealDetailPanel.tsx` | AskTopsiButton in quick actions |
| `frontend/src/pages/settings/SettingsLayout.tsx` | 2 new nav entries (admin + user) |
| `frontend/src/components/layout/sidebar.tsx` | Topsi Activity nav item |
| `frontend/src/App.tsx` | 3 new routes + lazy imports |
| `crates/server/src/routes/topsi.rs` | 4 new endpoints (admin prompt GET/PUT, user settings GET/PUT) |

---

## Updated Implementation Order

```
Phase 0 (WorkflowLLMService)              ── DONE ✅
Phase 1 (Topsi reads + 1B gaps)           ── DONE ✅
Phase 2 (Triggers, builder, CRM write,    ── DONE ✅
         PlatformDataService extract,
         full service extraction)
Phase 3 (Confirmation system)             ── DONE ✅
Phase 2F (Full service extraction)        ── DONE ✅
Phase 4 (Topsi builds workflows)          ── DONE ✅ (build_workflow tool + specialist)
Auto-approve encapsulation                ── DONE ✅ (post-Phase 2F cleanup)
Phase 5 (Bidirectional UI + admin settings) ── DONE ✅
```

---

## Key Infrastructure Discovered During Audit

| Resource | Location | Status |
|----------|----------|--------|
| `organization_members` table | Migration 20251004000000 | Exists — `(org_id, user_id, role)` |
| `Organization::get_user_role()` | `db/models/user.rs` | Exists — returns role for user in org |
| `system_settings` table | Migration 20260327000000 | Exists — key-value store with `get()`/`set()` |
| `TopsiConfig.autonomy_level` | `topsi/src/config.rs` | Exists — `Full/Supervised/ApprovalRequired/Manual` — NOT enforced |
| `TopsiConfig.system_prompt` | `topsi/src/config.rs` | Now wired to DB via `get_effective_system_prompt()` (Phase 1B.3) |
| `AccessScope` enum | `topsi/src/agent/access_control.rs` | Project-scoped; org validation via `verify_org_membership()` (Phase 1B.1) |
| `WorkflowNode.parameters` | `workflow_execution.rs` | Untyped JSON — `output_mode` added (Phase 1B.4) |

---

## Critical Files Reference

| File | Lines | Role |
|------|-------|------|
| `crates/services/src/services/workflow_llm.rs` | ~772 | WorkflowLLMService — multi-provider LLM with tool calling |
| `crates/services/src/services/workflow_execution.rs` | ~2396 | Core execution logic — LLM nodes, action nodes, dedup, validation |
| `crates/server/src/routes/workflow_engine.rs` | ~590 | Orchestration — topo sort, staging, auto-approve, finalization |
| `crates/server/src/routes/data_source_workflows.rs` | ~1349 | Thin HTTP handlers + trigger/schedule loops |
| `crates/server/src/routes/pcg_router.rs` | — | Model registry, direct API route (KEEP) |
| `crates/db/src/models/pcg_router_model.rs` | — | Model registry DB model |
| `crates/topsi/src/tools/mod.rs` | ~1005 | Topsi tool schemas (31 tools + respond_to_user) |
| `crates/topsi/src/agent.rs` | ~2455 | Topsi agent loop, tool dispatch, confirmation gate |
| `crates/topsi/src/platform_data.rs` | ~1974 | PlatformDataService — 22 CRUD tools |
| `crates/topsi/src/workflow_builder.rs` | ~378 | Workflow Builder specialist agent |
| `crates/db/src/models/topsi_user_settings.rs` | — | Per-user confirmation settings model |
| `crates/server/src/routes/workflow_staging.rs` | — | Staging commit logic, agent auto-start |
| `crates/topsi/src/agent/access_control.rs` | — | AccessScope + verify_org_membership |
| `crates/topsi/src/config.rs` | — | TopsiConfig with autonomy_level + DB-backed system prompt |
| `crates/db/src/models/system_settings.rs` | — | Key-value settings store |

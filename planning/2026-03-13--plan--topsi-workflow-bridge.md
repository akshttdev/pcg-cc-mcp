# Topsi–Workflow Bridge Implementation Plan

**Date:** 2026-03-13
**Branch:** `feature/topsi-workflow-bridge`
**Depends on:** PR #21 (LLM dogfooding infrastructure)
**Related:** `2026-03-12--plan--three-sprint-roadmap.md`, `2026-03-12--plan--agent-task-mcp-wiring.md`

---

## Context

The workflow editor's LLM execution uses `pcg_router::route_completion()` — a database-backed routing layer with 8+ providers and cost tracking but **no tool calling, no conversation history, no structured output**. Nora's `LLMClient` has tool calling but only 3 providers. Topsi has zero tools for workflows, CRM, or project data.

**Goal:** Build a new multi-provider LLM service with tool-calling support for all 8 PCG Router providers, then bridge Topsi into the workflow/CRM ecosystem.

### Decisions
- **LLM Layer:** New abstraction over PCG Router (all 8 providers + tool calling), not wrapping Nora's LLMClient
- **Execution priority:** Phase 0 + Phase 1 in parallel
- **Workflow refactor:** Full service extraction (~2000+ lines from route handler)
- **Confirmation:** Confirm destructive by default + fine-grained per-tool configuration

### Two Separate LLM Stacks (Current State)

| | PCG Router | Nora LLMClient |
|---|---|---|
| **Location** | `crates/server/src/routes/pcg_router.rs` | `crates/nora/src/brain/mod.rs` |
| **Providers** | 8+ (Anthropic, OpenAI, Gemini, Grok, Mistral, DeepSeek, Groq, Qwen) | 3 (Anthropic, OpenAI, Ollama) |
| **Model registry** | DB-backed (`pcg_router_models`), priority-based fallback | Config-based |
| **Cost tracking** | Yes (DB logging + `RoutingMetadata`) | No |
| **Tool calling** | No | Yes |
| **Provider dispatch** | 3 categories: Anthropic, OpenAI-compat (7), Gemini | Per-provider trait impl |

---

## Phase 0: Multi-Provider LLM Service with Tool Calling (PRIORITY)

Build `WorkflowLLMService` — extends PCG Router's 3-category provider dispatch with tool-calling support, borrowing Nora's provider patterns.

### Architecture

PCG Router groups 8 providers into 3 dispatch categories:
- **Anthropic** — `input_schema`, content blocks, `stop_reason: "tool_use"`
- **OpenAI-compat** (7 providers) — `function.parameters`, string arguments, `tool_calls` array
- **Gemini** — OpenAI-compat wrapper with different URL

### Service Design

```rust
// crates/services/src/workflow_llm.rs
pub struct WorkflowLLMService {
    http: reqwest::Client,
    db_pool: SqlitePool,
}

impl WorkflowLLMService {
    // Model selection from pcg_router_models, filter supports_tools for tool calls
    async fn resolve_model(&self, hint, needs_tools) -> Result<PcgRouterModel>

    // Drop-in replacement for route_completion
    pub async fn completion(&self, messages, model_hint, config) -> Result<(String, RoutingMetadata)>

    // NEW: tool-calling completion
    pub async fn completion_with_tools(&self, messages, tools, model_hint) -> Result<(LLMResponse, RoutingMetadata)>

    // NEW: structured JSON output
    pub async fn completion_with_structured_output(&self, messages, schema, model_hint) -> Result<(Value, RoutingMetadata)>
}
```

### Files
- **NEW** `crates/services/src/workflow_llm.rs` + `providers.rs`
- **MODIFY** `crates/server/src/routes/data_source_workflows.rs` — swap `route_completion()` calls
- **PORT** tool patterns from `crates/nora/src/brain/providers/{anthropic,openai}.rs`
- **KEEP** `pcg_router.rs` for model registry and direct API route

### Verification
- Run existing CRM contact extraction workflow end-to-end
- Test tool-calling with Anthropic + OpenAI providers
- Verify cost tracking still records via `RoutingMetadata`

---

## Phase 1: Topsi Reads the System (PARALLEL with Phase 0)

### New tools (8 total):
`list_projects`, `get_project_detail`, `list_crm_contacts`, `list_crm_deals`, `list_crm_pipelines`, `list_workflow_definitions`, `get_workflow_definition`, `search_entities`

### Pattern: schema in `tools/mod.rs` → match arm in `agent.rs:execute_tool_calls()` → scope by `AccessScope`

---

## Phase 2: Service Extraction + Topsi Triggers

### 2A: Extract ~2000 lines from `data_source_workflows.rs` into `crates/services/src/workflow_execution.rs`
### 2B: Topsi tools: `trigger_workflow`, `get_workflow_run_status`, `list_workflow_runs`, `review_staged_data`, `commit_staged_data`

---

## Phase 3: Topsi Builds Workflows (NL → Node Graph)

Tools: `create_workflow`, `modify_workflow`. Uses `WorkflowLLMService` with node type registry as context.

---

## Phase 4: Topsi Writes + Configurable Confirmation

### Tools with risk classification:
- **Yellow** (default: execute): `create_task`, `update_task`, `create_crm_contact`, `update_crm_deal`, `create_crm_deal`
- **Red** (default: confirm): `delete_task`, `bulk_update_tasks`

### Confirmation system:
- **`topsi_user_settings` table** with `default_confirmation_mode` and `per_tool_overrides` (JSON)
- Modes: `always_confirm`, `confirm_destructive` (default), `autonomous`
- Auto-approve timeout support via existing `ExecutionCheckpoint` infrastructure
- Org-level defaults from `Agent.autonomy_level`

---

## Phase 5: Bidirectional UI

Topsi action feed, "Ask Topsi" context buttons, workflow progress in chat, Topsi settings page.

---

## Implementation Order

```
Phase 0 (WorkflowLLMService)  ──┐
                                 ├── IN PARALLEL
Phase 1 (Topsi reads)          ──┘
    ↓
Phase 2 (Service extraction + triggers)
    ↓
Phase 3 (Topsi builds workflows)
    ↓
Phase 4 (Topsi writes + confirmation)
    ↓
Phase 5 (Bidirectional UI)
```

# LLM Workflow End-to-End Configuration Reference

**Created**: 2026-03-13
**Branch**: `feature/llm-workflows-2026-03-12` (PR #21)
**Status**: Code complete, configuration pending

---

## Overview

The LLM dogfooding pipeline is code-complete across 5 phases. This document records the configuration needed to activate the full end-to-end flow:

```
Feedback/GitHub webhook
  -> DataSource creation
    -> Trigger matching (org + data_type filter)
      -> Workflow execution (LLM triage via PCG Router)
        -> Staging records
          -> Batch commit (manual or auto_approve)
            -> Task creation (with agent_id, board_id, etc.)
              -> Auto-execute (agent runs against repo)
                -> Auto-PR (push branch + create GitHub PR)
```

---

## 1. LLM Provider API Keys

**Status**: Not configured
**Impact**: Workflow LLM nodes fall back to mock responses without a valid key

### How it works

`execute_node_with_llm()` in `crates/server/src/routes/data_source_workflows.rs` calls `pcg_router::route_completion()`. The router resolves API keys via:

1. `api_key_value` — stored directly in `pcg_router_models` table
2. `api_key_env_var` — environment variable fallback (e.g., `ANTHROPIC_API_KEY`)

### What to do

Set at least one env var before starting the server:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."   # Claude models (recommended)
export OPENAI_API_KEY="sk-..."          # GPT models
export GOOGLE_API_KEY="..."             # Gemini models
```

Or update `api_key_value` directly in the `pcg_router_models` table for specific models.

### Model selection priority

1. Node parameter (`parameters.model` in workflow definition JSON)
2. Trigger `model_override` field
3. Workflow `default_model` field
4. Highest-priority enabled model in PCG Router with a valid API key

### Seed file

`crates/db/migrations/20260101000000_seed_pcg_router.sql` — seeds 31 models with `api_key_env_var` references.

---

## 2. Agent Execution Configs

**Status**: Seeded (migration `20260330000000_seed_dogfood_agents.sql`)
**Impact**: Auto-execution after task creation now works for seeded agents

### How it works

When `commit_task()` in `workflow_staging.rs` creates a task with `agent_id` set, the auto-execute code calls `AgentExecutionConfig::find_by_agent_id()` to look up the `execution_profile_id`. No config row means no execution happens.

### Seeded agents

| Agent | ID | Profile | Auto-PR | Use Case |
|---|---|---|---|---|
| ORCHA Dev | `a0000000-...-001` | `profile-standard` | Yes | Code changes, bug fixes |
| ORCHA QA | `a0000000-...-002` | `profile-standard` | No | PR review, analysis only |

### Manual override (if needed)

```sql
UPDATE agent_execution_config
SET execution_profile_id = 'profile-ralph-standard'
WHERE agent_id = 'a0000000-0000-0000-0000-000000000001';
```

### Available executor profiles

Defined in `crates/local-deployment/src/default_profiles.json`:

| Profile ID | Description |
|---|---|
| `CLAUDE_CODE:DEFAULT` | Claude Code with default settings |
| `CLAUDE_CODE:PLAN` | Claude Code in plan mode |
| `CLAUDE_CODE:APPROVALS` | Claude Code with approval gates |
| `GEMINI:DEFAULT` | Gemini CLI default |
| `GEMINI:FLASH` | Gemini Flash (faster, cheaper) |
| `CODEX:DEFAULT` | OpenAI Codex |
| `AMP:DEFAULT` | Sourcegraph AMP |

### Related code

- Auto-execute logic: `workflow_staging.rs` → `auto_start_agent_execution()`
- Auto-PR logic: `crates/local-deployment/src/container.rs` → `try_auto_create_pr()`
- Config model: `crates/db/src/models/agent_execution_config.rs`

---

## 3. GitHub Token for Auto-PR

**Status**: Not configured
**Impact**: `try_auto_create_pr()` fails silently — tasks complete but no PR is created

### How it works

`container.rs` → `try_auto_create_pr()` uses `GitHubService::create_pr()` which needs a token for:
- Pushing branches to the remote
- Creating pull requests via the GitHub API

`GitHubConfig::token()` checks `pat` first, then `oauth_token`.

### What to do

Set the token in local deployment config:

```toml
[github]
pat = "ghp_xxxxxxxxxxxx"
```

Or set via environment (check `GitHubConfig` struct for env var support).

### Permissions needed

The PAT needs: `repo` (full), `workflow` (if touching CI), `pull_requests:write`.

---

## 4. Wire `auto_approve` into Trigger Flow

**Status**: ✅ Complete (Phase 8A)
**Impact**: When trigger has `auto_approve = 1`, valid non-duplicate records are auto-approved and committed

### Implementation

`fire_triggers_for_data_source()` in `data_source_workflows.rs` now:
1. Captures `trigger.auto_approve` before spawning the tokio task
2. After staging records are created, checks `trigger_auto_approve`
3. If `true`: calls `auto_approve_valid()` → `reject_duplicates()` → commits approved records
4. Post-commit: links contacts to companies by matching `company_name`
5. Post-commit: auto-starts agent execution for tasks with `agent_id`

The ORCHA Bug Triage trigger seed sets `auto_approve = 1` via migration `20260330000000`.

---

## 5. Model Pinning (Optional)

**Status**: Works, but uses default model selection
**Impact**: Triage quality depends on which model the router picks

### Current state

The Bug Triage Pipeline's `Investigate & Triage` node doesn't specify a model, so it uses the PCG Router's default (highest priority enabled model with a valid key).

### Optional improvement

Pin the triage node to a specific model by updating the workflow definition:

```sql
-- In the workflow definition's nodes JSON, add to the Investigate & Triage node:
-- "parameters": { "model": "claude-sonnet-4-20250514", "prompt_template": "..." }
```

Or set `default_model` on the workflow definition itself.

---

## Component Status Matrix

| Component | Code | Config | Notes |
|---|---|---|---|
| Feedback → DataSource → Trigger | Done | Done | POST /api/feedback |
| GitHub webhook → DataSource → Trigger | Done | Done | POST /api/webhooks/github |
| Trigger matching (org + data_type) | Done | Done | ORCHA Bug Triage trigger seeded |
| LLM triage (4-tier prompt) | Done | **Needs API key** | Falls back to mock without key |
| Staging record creation | Done | Done | All records created as pending_review |
| `auto_approve` bypass | Done | Done | Wired in fire_triggers (Phase 8A) |
| Batch commit → Task creation | Done | Done | commit_task() reads all fields |
| Auto-execute on task creation | Done | Done | Dev/QA agents seeded (Phase 7A) |
| Auto-PR on execution complete | Done | **Needs GitHub token** | try_auto_create_pr() in container.rs |
| QA agent review hook | Done | Done | Auto-creates QA task after PR (Phase 8B) |
| PR feedback loop | Done | Done | QA verdict → PR comment → iteration (Phase 9) |

---

## Key Files

| File | Role |
|---|---|
| `crates/server/src/routes/data_source_workflows.rs` | Workflow execution, LLM calls, trigger matching, staging |
| `crates/server/src/routes/workflow_staging.rs` | Batch commit, task creation, auto-execute wiring |
| `crates/server/src/routes/feedback.rs` | Feedback → Task + DataSource + trigger |
| `crates/server/src/routes/webhooks.rs` | GitHub webhook → DataSource + trigger |
| `crates/local-deployment/src/container.rs` | Agent execution, auto-PR |
| `crates/db/src/models/agent_execution_config.rs` | Agent-to-profile mapping |
| `crates/db/migrations/20260329000000_seed_dogfood_project.sql` | Project, boards, trigger seed |
| `crates/db/migrations/20260101000000_seed_pcg_router.sql` | PCG Router model seed |
| `crates/local-deployment/src/default_profiles.json` | Executor profile definitions |

---

## Quick Start Checklist

- [ ] Set `ANTHROPIC_API_KEY` (or other provider key) in environment
- [x] Create agent execution config rows in DB (seeded via migration)
- [ ] Set GitHub PAT in deployment config (`[github] pat = "ghp_..."`)
- [x] Wire `auto_approve` in `fire_triggers_for_data_source()` (Phase 8A)
- [ ] Set `GITHUB_WEBHOOK_SECRET` for production webhook validation
- [ ] Optional: pin model on Bug Triage Pipeline node
- [ ] Test: submit feedback → verify LLM triage runs with real model
- [ ] Test: approve staging → verify task created with correct fields
- [ ] Test: task with agent_id → verify execution starts
- [ ] Test: execution completes → verify PR created
- [ ] Test: QA agent reviews PR → verify comment posted
- [ ] Test: QA needs_changes → dev agent iterates

# Ralph Wiggum Methodology Integration

**Date**: 2026-01-16
**Session**: Agent Execution Configuration & Ralph Loop Implementation

## Overview

This document captures the implementation of the Ralph Wiggum methodology for autonomous task completion in the PCG platform. The system enables agents like AURI to execute development tasks iteratively until completion, using completion detection and backpressure validation.

## What is Ralph Wiggum?

Ralph Wiggum is an AI coding methodology created by Geoffrey Huntley that uses a bash loop to repeatedly run AI coding CLIs until task completion. Named after The Simpsons character for its persistent, undeterred approach to problem-solving.

**Key Concepts**:
- **Three Phases, Two Prompts, One Loop**: Planning, Building, and iteration
- **Dual-Gate Completion Detection**: Both completion promise AND exit signal required
- **Backpressure Validation**: Running tests/lints between iterations
- **Session Persistence**: Using `spawn_follow_up` with session IDs across iterations

## Implementation Summary

### Backend (Rust)

#### Database Migration
**File**: `crates/db/migrations/20260117100000_add_agent_execution_config.sql`

Tables created:
- `agent_execution_profiles` - Reusable execution configuration templates
- `agent_execution_config` - Agent-specific execution settings
- `ralph_loop_state` - Tracks active Ralph loop executions
- `ralph_iterations` - Logs each iteration within a Ralph loop
- `backpressure_definitions` - Project-type specific validation commands

Seeded with:
- 4 default profiles: standard, ralph_standard, ralph_aggressive, ralph_quick
- Backpressure definitions for Rust, Node, Python, and Go projects

#### Rust Models
**File**: `crates/db/src/models/agent_execution_config.rs`

```rust
// Execution modes
pub enum ExecutionMode {
    Standard,  // Single execution (default)
    Ralph,     // Iterative execution until completion
    Parallel,  // Concurrent sub-task execution
    Pipeline,  // Sequential stages with handoff
}

// Key structs
pub struct AgentExecutionProfile { ... }
pub struct AgentExecutionConfig { ... }
pub struct RalphLoopState { ... }
pub struct RalphIteration { ... }
pub struct BackpressureDefinition { ... }
```

#### Ralph Orchestrator Module
**Directory**: `crates/executors/src/ralph/`

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `completion.rs` | Dual-gate completion detection (promise + exit signal) |
| `backpressure.rs` | Validation command execution between iterations |
| `prompt.rs` | Agent-specific prompt building |
| `orchestrator.rs` | Main loop execution engine with session persistence |

#### Ralph Service
**File**: `crates/services/src/services/ralph.rs`

```rust
pub struct RalphService {
    db: DBService,
    git: GitService,
}

impl RalphService {
    pub async fn resolve_config(&self, task: &Task) -> Result<ResolvedRalphConfig, ...>;
    pub async fn should_use_ralph(&self, task: &Task) -> Result<bool, ...>;
    pub async fn start_ralph_loop(&self, request: StartRalphRequest, ...) -> Result<RalphLoopState, ...>;
    pub async fn get_loop_state(&self, loop_id: &str) -> Result<Option<RalphLoopState>, ...>;
    pub async fn cancel_loop(&self, loop_id: &str) -> Result<(), ...>;
}
```

#### API Routes
**File**: `crates/server/src/routes/agent_config.rs`

| Route | Method | Purpose |
|-------|--------|---------|
| `/execution-profiles` | GET | List all execution profiles |
| `/execution-profiles/{id}` | GET | Get specific profile |
| `/agents/{id}/execution-config` | GET/POST/PUT | Agent execution config CRUD |
| `/backpressure-definitions` | GET | List backpressure definitions |
| `/ralph/start` | POST | Start a Ralph loop |
| `/ralph/loops/{id}` | GET | Get loop state |
| `/ralph/loops/{id}/cancel` | POST | Cancel running loop |
| `/ralph/loops/{id}/iterations` | GET | Get iteration history |
| `/ralph/by-attempt/{id}` | GET | Get loop by task attempt |

#### Task Integration
**File**: `crates/server/src/routes/task_attempts.rs`

Modified `create_task_attempt` to:
1. Check if assigned agent uses Ralph mode via `RalphService::should_use_ralph()`
2. If Ralph mode, create container and start Ralph loop
3. If standard mode, use existing `container.start_attempt()` flow

### Frontend (React/TypeScript)

#### API Types & Methods
**File**: `frontend/src/lib/api.ts`

Added types:
- `ExecutionMode`, `ContextWindowStrategy`, `RalphLoopStatus`, `RalphIterationStatus`
- `AgentExecutionProfile`, `AgentExecutionConfig`, `RalphLoopState`, `RalphIteration`
- `BackpressureDefinition`, `StartRalphRequest`, `ResolvedRalphConfig`

Added API namespaces:
- `agentExecutionConfigApi` - Profile and config management
- `ralphApi` - Ralph loop management

#### Agent Execution Config Panel
**File**: `frontend/src/components/agents/AgentExecutionConfigPanel.tsx`

Scalable UI for configuring any agent's execution behavior:
- Base profile selection
- Execution mode override (Standard, Ralph, Parallel, Pipeline)
- Ralph-specific settings (max iterations, backpressure commands)
- Feature flags (auto-commit, auto-PR, require tests pass)
- Advanced settings (system prompt prefix/suffix)

#### Ralph Loop Progress
**File**: `frontend/src/components/agents/RalphLoopProgress.tsx`

Real-time progress display for Ralph executions:
- Current iteration / max iterations
- Status badge (initializing, running, validating, complete, etc.)
- Duration and token usage stats
- Iteration history timeline
- Compact mode for inline display

#### UI Integration

**Agent Detail Dialog** (`frontend/src/components/dialogs/agent-detail-dialog.tsx`):
- Added "Execution" tab alongside "Profile" tab
- Renders `AgentExecutionConfigPanel` for any agent

**Task Details Panel** (`frontend/src/components/tasks/TaskDetailsPanel.tsx`):
- Added `RalphLoopProgress` component after `AttemptHeaderCard`
- Shows only when task attempt has an active Ralph loop

## How It Works

### Configuration Flow
1. Navigate to agent settings (e.g., AURI)
2. Go to "Execution" tab
3. Select a Ralph profile (ralph_standard, ralph_aggressive, etc.) OR
4. Enable Ralph mode override with custom settings
5. Configure backpressure commands (tests, linting, etc.)
6. Save configuration

### Execution Flow
1. User creates a task and assigns an agent configured for Ralph mode
2. User starts the task (Create & Start or manual attempt creation)
3. System checks `should_use_ralph()` for the assigned agent
4. If Ralph mode:
   - Creates container/worktree
   - Starts Ralph loop via `RalphService::start_ralph_loop()`
   - Loop runs in background via `tokio::spawn`
5. Each iteration:
   - Executes coding agent with task prompt
   - Checks for completion promise and exit signal
   - Runs backpressure validation (if configured)
   - Updates iteration record in database
6. Loop terminates when:
   - Completion detected (dual-gate: promise + exit signal)
   - Max iterations reached
   - Error/cancellation

### Completion Detection
```
Dual-Gate Check:
├── Completion Promise: "<promise>TASK_COMPLETE</promise>"
└── Exit Signal: "EXIT_SIGNAL: true"

Both must be present for task to be marked complete.
```

### Backpressure Validation
```
Between iterations, run validation commands:
├── Rust: cargo test, cargo clippy
├── Node: npm test, npm run lint
├── Python: pytest, ruff check
└── Go: go test, golangci-lint run

If any fail and fail_on_any=true, iteration continues without completion.
```

## Files Modified/Created

### Created
- `crates/db/migrations/20260117100000_add_agent_execution_config.sql`
- `crates/db/src/models/agent_execution_config.rs`
- `crates/executors/src/ralph/mod.rs`
- `crates/executors/src/ralph/completion.rs`
- `crates/executors/src/ralph/backpressure.rs`
- `crates/executors/src/ralph/prompt.rs`
- `crates/executors/src/ralph/orchestrator.rs`
- `crates/services/src/services/ralph.rs`
- `crates/server/src/routes/agent_config.rs`
- `frontend/src/components/agents/AgentExecutionConfigPanel.tsx`
- `frontend/src/components/agents/RalphLoopProgress.tsx`

### Modified
- `crates/db/src/models/mod.rs` - Added `agent_execution_config` export
- `crates/executors/src/lib.rs` - Added `ralph` module export
- `crates/services/src/services/mod.rs` - Added `ralph` module export
- `crates/server/src/routes/mod.rs` - Added `agent_config` routes
- `crates/server/src/routes/task_attempts.rs` - Ralph mode integration
- `frontend/src/lib/api.ts` - Added types and API methods
- `frontend/src/components/dialogs/agent-detail-dialog.tsx` - Added Execution tab
- `frontend/src/components/tasks/TaskDetailsPanel.tsx` - Added Ralph progress display

## Architecture Benefits

1. **Scalable**: Works for ALL agents, not just AURI
2. **Configurable**: Profiles can be reused across agents
3. **Observable**: Full iteration history and real-time progress
4. **Extensible**: Easy to add new execution modes (Parallel, Pipeline)
5. **Resilient**: Falls back to standard execution on Ralph loop failure

## Next Steps

1. Run database migration: `sqlx migrate run`
2. Generate TypeScript types: `npm run generate-types`
3. Build and test the application
4. Configure AURI with Ralph profile
5. Test end-to-end Ralph execution with a development task

## References

- [Claude Code Ralph Wiggum Plugin](https://github.com/anthropics/claude-code)
- [Geoffrey Huntley's Ralph Wiggum Methodology](https://www.youtube.com/watch?v=5xvP9O4msLM)

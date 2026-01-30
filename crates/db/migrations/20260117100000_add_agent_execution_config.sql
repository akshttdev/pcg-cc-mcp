-- Migration: Agent Execution Configuration System
-- A scalable system for configuring execution behavior for any agent
-- Supports Ralph Wiggum loop methodology and other execution patterns

-- ============================================================================
-- AGENT EXECUTION PROFILES
-- Reusable execution configuration templates that can be assigned to agents
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_execution_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,                          -- "ralph_standard", "ralph_aggressive", "single_shot"
    description TEXT,

    -- Execution Mode
    execution_mode TEXT NOT NULL DEFAULT 'standard'
        CHECK (execution_mode IN ('standard', 'ralph', 'parallel', 'pipeline')),

    -- Ralph Loop Configuration (when execution_mode = 'ralph')
    max_iterations INTEGER DEFAULT 50,
    completion_promise TEXT DEFAULT '<promise>TASK_COMPLETE</promise>',
    exit_signal_key TEXT DEFAULT 'EXIT_SIGNAL: true',

    -- Backpressure Configuration (JSON array of validation commands)
    -- e.g., ["cargo test --quiet", "cargo clippy -- -D warnings"]
    backpressure_commands TEXT DEFAULT '[]',
    backpressure_fail_threshold INTEGER DEFAULT 0,       -- 0 = any failure triggers retry

    -- Timing Configuration
    iteration_delay_ms INTEGER DEFAULT 2000,             -- Delay between iterations
    iteration_timeout_ms INTEGER DEFAULT 600000,         -- 10 min per iteration max
    total_timeout_ms INTEGER DEFAULT 3600000,            -- 1 hour total max

    -- Context Management
    preserve_session BOOLEAN DEFAULT TRUE,               -- Use spawn_follow_up with session_id
    context_window_strategy TEXT DEFAULT 'fresh'
        CHECK (context_window_strategy IN ('fresh', 'cumulative', 'sliding')),

    -- Prompt Templates (JSON object with mode-specific prompts)
    -- e.g., {"planning": "...", "building": "...", "followup": "..."}
    prompt_templates TEXT DEFAULT '{}',

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    created_by TEXT
);

-- ============================================================================
-- AGENT EXECUTION CONFIG
-- Links agents to their execution profiles with overrides
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_execution_config (
    id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,

    -- Base profile (can be NULL for custom config)
    execution_profile_id TEXT REFERENCES agent_execution_profiles(id) ON DELETE SET NULL,

    -- Override values (NULL = use profile default)
    execution_mode_override TEXT
        CHECK (execution_mode_override IS NULL OR execution_mode_override IN ('standard', 'ralph', 'parallel', 'pipeline')),
    max_iterations_override INTEGER,
    backpressure_commands_override TEXT,                -- JSON array

    -- Agent-specific prompt customization
    system_prompt_prefix TEXT,                          -- Prepended to all prompts
    system_prompt_suffix TEXT,                          -- Appended to all prompts

    -- Project-type specific backpressure (JSON object)
    -- e.g., {"rust": ["cargo test"], "node": ["npm test"], "python": ["pytest"]}
    project_type_backpressure TEXT DEFAULT '{}',

    -- Feature flags
    auto_commit_on_success BOOLEAN DEFAULT TRUE,
    auto_create_pr_on_complete BOOLEAN DEFAULT FALSE,
    require_tests_pass BOOLEAN DEFAULT TRUE,

    -- Metadata
    is_active BOOLEAN DEFAULT TRUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(agent_id)
);

-- ============================================================================
-- RALPH LOOP STATE
-- Tracks the state of Ralph loop executions
-- ============================================================================

CREATE TABLE IF NOT EXISTS ralph_loop_state (
    id TEXT PRIMARY KEY NOT NULL,
    task_attempt_id TEXT NOT NULL REFERENCES task_attempts(id) ON DELETE CASCADE,
    agent_id TEXT REFERENCES agents(id),

    -- Loop State
    current_iteration INTEGER NOT NULL DEFAULT 0,
    max_iterations INTEGER NOT NULL DEFAULT 50,
    session_id TEXT,                                    -- Claude Code session for follow-ups

    -- Status
    status TEXT NOT NULL DEFAULT 'initializing'
        CHECK (status IN ('initializing', 'running', 'validating', 'complete', 'max_reached', 'failed', 'cancelled')),

    -- Completion Tracking
    completion_promise TEXT,
    completion_detected_at TEXT,
    final_validation_passed BOOLEAN,

    -- Metrics
    total_tokens_used INTEGER DEFAULT 0,
    total_cost_cents INTEGER DEFAULT 0,

    -- Timing
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at TEXT,
    last_iteration_at TEXT,

    -- Error tracking
    last_error TEXT,
    consecutive_failures INTEGER DEFAULT 0,

    UNIQUE(task_attempt_id)
);

-- ============================================================================
-- RALPH ITERATION LOG
-- Detailed log of each iteration in a Ralph loop
-- ============================================================================

CREATE TABLE IF NOT EXISTS ralph_iterations (
    id TEXT PRIMARY KEY NOT NULL,
    ralph_loop_id TEXT NOT NULL REFERENCES ralph_loop_state(id) ON DELETE CASCADE,
    execution_process_id TEXT REFERENCES execution_processes(id),

    iteration_number INTEGER NOT NULL,

    -- Iteration Status
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'completed', 'failed', 'timeout')),

    -- Completion Detection
    completion_signal_found BOOLEAN DEFAULT FALSE,
    exit_signal_found BOOLEAN DEFAULT FALSE,

    -- Backpressure Results (JSON)
    -- e.g., {"cargo test": {"passed": true, "output": "..."}, ...}
    backpressure_results TEXT DEFAULT '{}',
    all_backpressure_passed BOOLEAN,

    -- Metrics
    tokens_used INTEGER DEFAULT 0,
    cost_cents INTEGER DEFAULT 0,
    duration_ms INTEGER,

    -- Timing
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at TEXT,

    -- Output summary
    output_summary TEXT,
    files_modified INTEGER DEFAULT 0,
    commits_made INTEGER DEFAULT 0
);

-- ============================================================================
-- BACKPRESSURE DEFINITIONS
-- Reusable backpressure validation configurations
-- ============================================================================

CREATE TABLE IF NOT EXISTS backpressure_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,                                 -- "rust_standard", "node_full", "python_strict"
    description TEXT,

    -- Project type this applies to (NULL = universal)
    project_type TEXT,                                  -- "rust", "node", "python", "go", etc.

    -- Commands to run (JSON array)
    commands TEXT NOT NULL,                             -- ["cargo test", "cargo clippy"]

    -- Behavior
    fail_on_any BOOLEAN DEFAULT TRUE,                   -- Fail if any command fails
    timeout_ms INTEGER DEFAULT 300000,                  -- 5 min default per command
    run_in_parallel BOOLEAN DEFAULT FALSE,

    -- Priority (higher = run first)
    priority INTEGER DEFAULT 0,

    is_active BOOLEAN DEFAULT TRUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- ============================================================================
-- INDEXES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_agent_execution_config_agent
    ON agent_execution_config(agent_id);

CREATE INDEX IF NOT EXISTS idx_ralph_loop_state_task_attempt
    ON ralph_loop_state(task_attempt_id);

CREATE INDEX IF NOT EXISTS idx_ralph_loop_state_status
    ON ralph_loop_state(status) WHERE status = 'running';

CREATE INDEX IF NOT EXISTS idx_ralph_iterations_loop
    ON ralph_iterations(ralph_loop_id);

CREATE INDEX IF NOT EXISTS idx_ralph_iterations_execution
    ON ralph_iterations(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_backpressure_definitions_project_type
    ON backpressure_definitions(project_type);

-- ============================================================================
-- SEED DEFAULT EXECUTION PROFILES
-- ============================================================================

INSERT INTO agent_execution_profiles (id, name, description, execution_mode, max_iterations, completion_promise, backpressure_commands, prompt_templates) VALUES
    -- Standard single-shot execution (current default behavior)
    ('profile-standard', 'standard', 'Single execution without looping', 'standard', 1, NULL, '[]', '{}'),

    -- Ralph Standard - balanced approach
    ('profile-ralph-standard', 'ralph_standard', 'Ralph Wiggum loop with standard settings - 50 iterations, basic validation', 'ralph', 50, '<promise>TASK_COMPLETE</promise>', '[]', '{
        "system": "Execute tasks iteratively. Output <promise>TASK_COMPLETE</promise> and EXIT_SIGNAL: true when done.",
        "followup": "Continue from where you left off. Check IMPLEMENTATION_PLAN.md for current state."
    }'),

    -- Ralph Aggressive - more iterations, stricter validation
    ('profile-ralph-aggressive', 'ralph_aggressive', 'Ralph Wiggum loop with aggressive settings - 100 iterations, strict validation', 'ralph', 100, '<promise>TASK_COMPLETE</promise>', '[]', '{
        "system": "Execute tasks iteratively with strict quality. All tests must pass. Output <promise>TASK_COMPLETE</promise> and EXIT_SIGNAL: true when fully complete.",
        "followup": "Continue execution. Review test failures and fix issues."
    }'),

    -- Ralph Quick - fewer iterations for simpler tasks
    ('profile-ralph-quick', 'ralph_quick', 'Ralph Wiggum loop for quick tasks - 20 iterations', 'ralph', 20, '<promise>TASK_COMPLETE</promise>', '[]', '{
        "system": "Execute this task efficiently. Output <promise>TASK_COMPLETE</promise> and EXIT_SIGNAL: true when done.",
        "followup": "Continue and complete the task."
    }');

-- ============================================================================
-- SEED DEFAULT BACKPRESSURE DEFINITIONS
-- ============================================================================

INSERT INTO backpressure_definitions (id, name, description, project_type, commands, priority) VALUES
    ('bp-rust-standard', 'rust_standard', 'Standard Rust validation', 'rust', '["cargo test --quiet", "cargo clippy --quiet -- -D warnings", "cargo fmt --check"]', 10),
    ('bp-rust-minimal', 'rust_minimal', 'Minimal Rust validation (tests only)', 'rust', '["cargo test --quiet"]', 5),
    ('bp-node-standard', 'node_standard', 'Standard Node.js validation', 'node', '["npm test", "npm run lint"]', 10),
    ('bp-node-full', 'node_full', 'Full Node.js validation with typecheck', 'node', '["npm test", "npm run lint", "npm run typecheck"]', 15),
    ('bp-python-standard', 'python_standard', 'Standard Python validation', 'python', '["pytest", "ruff check ."]', 10),
    ('bp-go-standard', 'go_standard', 'Standard Go validation', 'go', '["go test ./...", "go vet ./..."]', 10),
    ('bp-universal-build', 'universal_build', 'Universal build check', NULL, '["make build 2>/dev/null || npm run build 2>/dev/null || cargo build 2>/dev/null || true"]', 1);

-- ============================================================================
-- ADD EXECUTION CONFIG REFERENCE TO TASKS
-- ============================================================================

ALTER TABLE tasks ADD COLUMN execution_config TEXT;
-- JSON object for task-specific execution overrides
-- e.g., {"execution_mode": "ralph", "max_iterations": 30}

-- ============================================================================
-- ADD RALPH ITERATION TRACKING TO EXECUTION PROCESSES
-- ============================================================================

ALTER TABLE execution_processes ADD COLUMN ralph_iteration INTEGER;
ALTER TABLE execution_processes ADD COLUMN ralph_loop_id TEXT REFERENCES ralph_loop_state(id);

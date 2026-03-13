-- Seed dogfooding agents for the ORCHA self-development pipeline.
-- Two agents: Dev (code changes) and QA (PR review).

-- ─── ORCHA Dev Agent ────────────────────────────────────────────────────────

INSERT OR IGNORE INTO agents (
    id, short_name, designation, description,
    default_model, status, autonomy_level,
    max_concurrent_tasks, priority_weight,
    capabilities, tools, version,
    created_at, updated_at, created_by
) VALUES (
    'a0000000-0000-0000-0000-000000000001',
    'ORCHA Dev',
    'Platform Development Agent',
    'Automated code changes for bug fixes and features in the pcg-cc-mcp repository. Creates branches, implements changes, and pushes PRs.',
    'claude-sonnet-4-20250514',
    'active',
    'approval_required',
    2,
    100,
    '["code_execution", "git_operations", "testing", "debugging"]',
    '["read_file", "write_file", "run_command", "git_commit", "git_push", "create_pr"]',
    '1.0.0',
    datetime('now'),
    datetime('now'),
    'system'
);

-- ─── ORCHA QA Agent ─────────────────────────────────────────────────────────

INSERT OR IGNORE INTO agents (
    id, short_name, designation, description,
    default_model, status, autonomy_level,
    max_concurrent_tasks, priority_weight,
    capabilities, tools, version,
    created_at, updated_at, created_by
) VALUES (
    'a0000000-0000-0000-0000-000000000002',
    'ORCHA QA',
    'Quality Assurance Review Agent',
    'Reviews PRs created by the Dev agent. Checks completion criteria, code quality, and leaves structured review comments. Read-only analysis mode.',
    'claude-sonnet-4-20250514',
    'active',
    'supervised',
    3,
    90,
    '["code_review", "analysis", "testing"]',
    '["read_file", "run_command", "search_code"]',
    '1.0.0',
    datetime('now'),
    datetime('now'),
    'system'
);

-- ─── Execution Configs ──────────────────────────────────────────────────────

-- Dev agent: CLAUDE_CODE default profile, auto-create PRs, require tests
INSERT OR IGNORE INTO agent_execution_config (
    id, agent_id,
    execution_profile_id,
    auto_commit_on_success,
    auto_create_pr_on_complete,
    require_tests_pass,
    is_active,
    system_prompt_prefix,
    created_at, updated_at
) VALUES (
    'aec-dev-001',
    'a0000000-0000-0000-0000-000000000001',
    'profile-standard',
    1,   -- auto_commit_on_success
    1,   -- auto_create_pr_on_complete
    1,   -- require_tests_pass
    1,   -- is_active
    'You are the ORCHA Dev Agent. Your task is to implement code changes for the pcg-cc-mcp repository. Follow existing code patterns and conventions. Always run tests before completing.',
    datetime('now'),
    datetime('now')
);

-- ─── Update ORCHA Bug Triage trigger to auto-approve ────────────────────────

UPDATE workflow_triggers
SET auto_approve = 1, updated_at = datetime('now')
WHERE id = 'tr100000-0000-0000-0000-000000000001';

-- QA agent: CLAUDE_CODE:PLAN profile (analysis mode), no PR creation
INSERT OR IGNORE INTO agent_execution_config (
    id, agent_id,
    execution_profile_id,
    auto_commit_on_success,
    auto_create_pr_on_complete,
    require_tests_pass,
    is_active,
    system_prompt_prefix,
    created_at, updated_at
) VALUES (
    'aec-qa-001',
    'a0000000-0000-0000-0000-000000000002',
    'profile-standard',
    0,   -- auto_commit_on_success (QA doesn't commit)
    0,   -- auto_create_pr_on_complete (QA doesn't create PRs)
    0,   -- require_tests_pass (QA is read-only)
    1,   -- is_active
    'You are the ORCHA QA Agent. Review the PR diff and completion criteria. Output a structured JSON verdict with: verdict (pass/needs_changes/fail), summary, criteria_checks array, and issues array. Do not modify any files.',
    datetime('now'),
    datetime('now')
);

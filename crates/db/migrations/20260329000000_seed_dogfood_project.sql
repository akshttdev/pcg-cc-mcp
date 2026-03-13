-- Seed dogfooding project: Link pcg-cc-mcp repo to Power Club Global org
-- This enables using the ORCHA dashboard to manage its own development.

-- Update the existing "Dashboard Bug Reports" project to be the dogfood project
UPDATE projects
SET name = 'ORCHA Platform',
    organization_id = '01010101-0101-0101-0101-010101010101',
    git_repo_path = '.',
    updated_at = datetime('now')
WHERE id = '00000000-0000-0000-0000-000000000001';

-- Create boards for the dogfood project
-- Valid board_type values: executive_assets, brand_assets, dev_assets, social_assets, custom,
--   agent_flows, artifact_gallery, approval_queue, research_hub
INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES
    ('d0600000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001',
     'Bugs', 'bugs', 'custom', 'Bug reports and fixes for the ORCHA platform',
     datetime('now'), datetime('now')),
    ('d0600000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001',
     'Features', 'features', 'custom', 'New feature development and enhancements',
     datetime('now'), datetime('now')),
    ('d0600000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000001',
     'Infrastructure', 'infrastructure', 'custom', 'Database migrations, CI/CD, tooling, and devops',
     datetime('now'), datetime('now'));

-- Create a workflow trigger: Bug Triage Pipeline fires on github_issue data sources
-- scoped to the Power Club Global organization
INSERT OR IGNORE INTO workflow_triggers (
    id, workflow_id, trigger_type, name,
    filter_data_source_types, filter_organization_id,
    auto_approve, enabled,
    created_at, updated_at
) VALUES (
    'tr100000-0000-0000-0000-000000000001',
    'bug_triage_pipeline',
    'data_source_created',
    'ORCHA Bug Triage',
    '["github_issue", "report"]',
    '01010101-0101-0101-0101-010101010101',
    0,  -- require human review initially
    1,  -- enabled
    datetime('now'),
    datetime('now')
);

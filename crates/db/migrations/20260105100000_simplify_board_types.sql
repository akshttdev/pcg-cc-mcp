-- Simplify board types: convert all predefined types to 'custom', keep one 'default' per project
-- This migration consolidates the 9 board types down to just 2: default and custom

-- Step 1: Convert all old predefined types to 'custom'
UPDATE project_boards
SET board_type = 'custom'
WHERE board_type IN (
    'executive_assets',
    'brand_assets',
    'dev_assets',
    'social_assets',
    'agent_flows',
    'artifact_gallery',
    'approval_queue',
    'research_hub'
);

-- Step 2: For each project, ensure there's exactly one 'default' board
-- First, identify projects that don't have a default board
-- Then create one for them (this will be handled by the application on next access)

-- Note: Existing boards are preserved as 'custom' boards
-- The application will auto-create a 'default' board when needed

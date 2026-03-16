-- Update Clients pipeline stages from 6 to 8 stages with human review gates.
-- Maps existing deals to new stages:
--   Lead → Lead, Qualified → Lead, Proposal → Build Proposal,
--   Negotiation → Proposal Meeting, Closed Won → Closed Won, Closed Lost → Closed Lost

-- Step 1: Move existing client pipeline stages to high temp positions to free up 0-7
UPDATE crm_pipeline_stages SET position = position + 2000
WHERE pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

-- Step 2: Insert new stages at temp positions (1000+), skipping names that already exist
CREATE TABLE IF NOT EXISTS _temp_new_client_stages (
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    temp_position INTEGER NOT NULL,
    final_position INTEGER NOT NULL,
    is_closed INTEGER NOT NULL DEFAULT 0,
    is_won INTEGER NOT NULL DEFAULT 0,
    probability INTEGER NOT NULL DEFAULT 0
);

INSERT INTO _temp_new_client_stages (name, color, temp_position, final_position, is_closed, is_won, probability) VALUES
    ('Lead',              '#6B7280', 1000, 0, 0, 0, 5),
    ('Business Analysis', '#3B82F6', 1001, 1, 0, 0, 20),
    ('Discovery',         '#8B5CF6', 1002, 2, 0, 0, 40),
    ('Build Proposal',    '#F59E0B', 1003, 3, 0, 0, 55),
    ('Polish',            '#EC4899', 1004, 4, 0, 0, 70),
    ('Proposal Meeting',  '#EF4444', 1005, 5, 0, 0, 85),
    ('Closed Won',        '#22C55E', 1006, 6, 1, 1, 100),
    ('Closed Lost',       '#9CA3AF', 1007, 7, 1, 0, 0);

INSERT INTO crm_pipeline_stages (id, pipeline_id, name, color, position, is_closed, is_won, probability, created_at, updated_at)
SELECT
    randomblob(16),
    p.id,
    ns.name,
    ns.color,
    ns.temp_position,
    ns.is_closed,
    ns.is_won,
    ns.probability,
    datetime('now', 'subsec'),
    datetime('now', 'subsec')
FROM crm_pipelines p
CROSS JOIN _temp_new_client_stages ns
WHERE p.pipeline_type = 'clients'
AND NOT EXISTS (
    SELECT 1 FROM crm_pipeline_stages es
    WHERE es.pipeline_id = p.id AND es.name = ns.name
);

-- Step 3: Remap deals from old stages to new stages

-- "Qualified" → "Lead"
UPDATE crm_deals SET crm_stage_id = (
    SELECT new_s.id FROM crm_pipeline_stages new_s
    JOIN crm_pipeline_stages old_s ON old_s.id = crm_deals.crm_stage_id
    WHERE new_s.pipeline_id = old_s.pipeline_id AND new_s.name = 'Lead'
    LIMIT 1
)
WHERE crm_stage_id IN (
    SELECT s.id FROM crm_pipeline_stages s
    JOIN crm_pipelines p ON p.id = s.pipeline_id
    WHERE p.pipeline_type = 'clients' AND s.name = 'Qualified'
);

-- "Proposal" → "Build Proposal"
UPDATE crm_deals SET crm_stage_id = (
    SELECT new_s.id FROM crm_pipeline_stages new_s
    JOIN crm_pipeline_stages old_s ON old_s.id = crm_deals.crm_stage_id
    WHERE new_s.pipeline_id = old_s.pipeline_id AND new_s.name = 'Build Proposal'
    LIMIT 1
)
WHERE crm_stage_id IN (
    SELECT s.id FROM crm_pipeline_stages s
    JOIN crm_pipelines p ON p.id = s.pipeline_id
    WHERE p.pipeline_type = 'clients' AND s.name = 'Proposal'
);

-- "Negotiation" → "Proposal Meeting"
UPDATE crm_deals SET crm_stage_id = (
    SELECT new_s.id FROM crm_pipeline_stages new_s
    JOIN crm_pipeline_stages old_s ON old_s.id = crm_deals.crm_stage_id
    WHERE new_s.pipeline_id = old_s.pipeline_id AND new_s.name = 'Proposal Meeting'
    LIMIT 1
)
WHERE crm_stage_id IN (
    SELECT s.id FROM crm_pipeline_stages s
    JOIN crm_pipelines p ON p.id = s.pipeline_id
    WHERE p.pipeline_type = 'clients' AND s.name = 'Negotiation'
);

-- Step 4: Delete old stages that no longer exist in the new schema
DELETE FROM crm_pipeline_stages
WHERE pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients')
AND name IN ('Qualified', 'Proposal', 'Negotiation');

-- Step 5: Set final positions for all new stages
UPDATE crm_pipeline_stages SET position = 0, color = '#6B7280', probability = 5, is_closed = 0, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Lead' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 1, color = '#3B82F6', probability = 20, is_closed = 0, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Business Analysis' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 2, color = '#8B5CF6', probability = 40, is_closed = 0, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Discovery' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 3, color = '#F59E0B', probability = 55, is_closed = 0, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Build Proposal' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 4, color = '#EC4899', probability = 70, is_closed = 0, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Polish' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 5, color = '#EF4444', probability = 85, is_closed = 0, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Proposal Meeting' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 6, color = '#22C55E', probability = 100, is_closed = 1, is_won = 1, updated_at = datetime('now', 'subsec')
WHERE name = 'Closed Won' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

UPDATE crm_pipeline_stages SET position = 7, color = '#9CA3AF', probability = 0, is_closed = 1, is_won = 0, updated_at = datetime('now', 'subsec')
WHERE name = 'Closed Lost' AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'clients');

-- Cleanup
DROP TABLE IF EXISTS _temp_new_client_stages;

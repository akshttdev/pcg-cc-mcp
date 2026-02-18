-- Fix project-org linking: catch-all for unlinked projects + Sirak-specific assignments

-- 1. Catch-all: any project still without an organization → Powerclub Global
UPDATE projects
SET organization_id = (SELECT id FROM organizations WHERE slug = 'powerclub-global')
WHERE organization_id IS NULL
  AND EXISTS (SELECT 1 FROM organizations WHERE slug = 'powerclub-global');

-- 2. Ensure Sirak-specific projects are under Sirak Studios
UPDATE projects
SET organization_id = (SELECT id FROM organizations WHERE slug = 'sirak-studios')
WHERE LOWER(name) IN ('prime', 'sirak-studios')
  AND EXISTS (SELECT 1 FROM organizations WHERE slug = 'sirak-studios');

-- 3. Add sirak as project_member on Resonance (if it exists) for direct access
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, permissions, granted_at)
SELECT randomblob(16), p.id, u.id, 'editor', '{}', datetime('now')
FROM projects p, users u
WHERE LOWER(p.name) = 'resonance' AND u.username = 'sirak';

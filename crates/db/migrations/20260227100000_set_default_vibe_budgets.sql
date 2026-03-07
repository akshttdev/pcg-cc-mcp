-- Set a default VIBE budget for existing projects that have no limit set
-- 500 VIBE = $5.00 starter budget
-- Projects with existing budgets are not affected
UPDATE projects
SET vibe_budget_limit = 500
WHERE vibe_budget_limit IS NULL
  AND id IN (
    SELECT DISTINCT pm.project_id
    FROM project_members pm
    JOIN users u ON pm.user_id = u.id
    WHERE u.is_admin = 0
  );

-- Admin user projects keep unlimited (NULL) budget

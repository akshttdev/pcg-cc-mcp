-- Fix BUG-4: Delete leftover E2E test artifacts from Legendary Forms and any other boards
-- Safe to run multiple times

DELETE FROM tasks
WHERE title LIKE '[E2E]%'
  AND deleted_at IS NULL;

-- Fix artifact file_path: absolute paths → relative paths
--
-- The original seed migration stored absolute local paths like:
--   /home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits
-- These only exist on the dev machine. Change to relative paths that
-- resolve via asset_dir() (dev_assets/) at serve time.
--
-- Also fix absolute paths embedded in the JSON content field.

-- 1. Fix file_path column
UPDATE execution_artifacts
SET file_path = 'artifacts/le-chateau-edits'
WHERE file_path = '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits';

-- 2. Fix absolute paths inside the render_deliverable JSON content
UPDATE execution_artifacts
SET content = REPLACE(
    content,
    '/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits/',
    'artifacts/le-chateau-edits/'
)
WHERE artifact_type = 'render_deliverable'
  AND content LIKE '%/home/spaceterminal/topos/sirak-studios/clients/le-chateau/edits/%';

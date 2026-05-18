-- Repair: some dev databases recorded `20260424000000_client_kg_links` as
-- applied (in `_sqlx_migrations`) while the inline ALTER TABLE statements
-- never actually ran, so `clients` is missing `company_id`,
-- `primary_person_id`, `prospect_at`, `client_since`.
--
-- This migration unsticks the stuck row *only when the KG columns are still
-- missing*. After it runs, the next sqlx startup detects 20260424 is
-- unapplied and re-runs the original ALTERs — adding the missing columns.
--
-- On a clean DB (cols already present) this is a no-op: the WHERE clause
-- finds the column and skips the DELETE. The linked-table fields
-- (business_reports.client_id, deliverables.business_report_id) added by
-- the same original migration aren't touched here — they're orthogonal and
-- the clean-DB has them; on a drifted DB the re-run will add them too.

DELETE FROM _sqlx_migrations
WHERE version = 20260424000000
  AND NOT EXISTS (
    SELECT 1 FROM pragma_table_info('clients') WHERE name = 'company_id'
  );

-- Drop FTS5 sync triggers that conflict with SQLite's RETURNING clause.
-- SQLite does not allow AFTER triggers that write to virtual tables (FTS5)
-- when the triggering statement uses RETURNING. Since sqlx uses RETURNING
-- for INSERT/UPDATE queries, these triggers cause "unsafe use of virtual table" errors.
--
-- FTS reindexing should be handled at the application level instead.

DROP TRIGGER IF EXISTS tasks_fts_insert;
DROP TRIGGER IF EXISTS tasks_fts_update;
DROP TRIGGER IF EXISTS tasks_fts_delete;

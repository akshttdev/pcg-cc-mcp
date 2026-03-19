---
paths:
  - "crates/db/**/*.rs"
  - "crates/db/migrations/**/*.sql"
---

# Database Standards (SQLite + SQLx)

## Query Patterns

- Never use `SELECT *` — list columns explicitly
- Always parameterize queries — use `.bind()` for all values (SQLx enforces this, but verify raw SQL)
- Prefer `sqlx::query_as::<_, Type>()` with `#[derive(FromRow)]` over `query_as!()` macro for offline compatibility
- Use `RETURNING` clause on INSERT/UPDATE to get the modified row back
- For optional lookups: `.fetch_optional(pool)` → `Result<Option<T>>`
- For lists: `.fetch_all(pool)` → `Result<Vec<T>>` (empty vec on no rows, not error)
- For mutations without return data: `.execute(pool)` → check `rows_affected()`

## Model Struct Conventions

- Derive: `Debug, Clone, FromRow, Serialize, Deserialize, TS`
- Use `#[ts(export)]` on all API-facing models
- Timestamp fields: `DateTime<Utc>` with `#[ts(type = "Date")]`
- Optional fields: `Option<T>` with `#[ts(optional)]`
- JSON columns: `Option<Json<Value>>` with `#[ts(type = "Record<string, unknown> | null")]`
- Enum fields: `#[sqlx(type_name = "...", rename_all = "lowercase")]` + matching `#[serde(rename_all)]`

## Create/Update DTOs

- `CreateEntity`: required fields only, matches INSERT columns
- `UpdateEntity`: ALL fields `Option<T>` with `#[ts(optional)]` — partial updates
- Tags/arrays: accept `Option<Vec<String>>`, serialize to JSON string for storage
- Return the full model from create/update methods (via RETURNING)

## UUID Handling (DbUuid)

- New columns: use TEXT format for UUIDs
- Legacy BLOB columns exist: `users.id`, `project_members.user_id`, `organization_members.user_id`, etc.
- Use `bind_uuid()` for TEXT columns, `bind_uuid_blob()` for legacy BLOB columns
- `DbUuid` decodes both TEXT and BLOB transparently — always encodes as TEXT

## Database Setup

- **Dev DB**: `dev_assets/db.sqlite` — auto-seeded from `dev_assets_seed/db.sqlite` on first `flox activate`
- **Test DB**: `dev_assets_seed/test-seed.sqlite` — minimal fixtures for E2E testing. Create with `scripts/create-test-seed.sh`
- **DATABASE_URL**: Override DB path via env var (default: `sqlite://dev_assets/db.sqlite`). For test isolation: `DATABASE_URL=sqlite:dev_assets/test-db.sqlite`
- **Missing column errors on startup**: The BLOB→TEXT migration (20260406) expects columns that may not exist in older dev DBs. Fix with: `sqlite3 dev_assets/db.sqlite "ALTER TABLE <table> ADD COLUMN <col> TEXT DEFAULT '...';"` — see migration file for expected columns
- **Migration checksum mismatches**: If a migration file changes after being applied, update `_sqlx_migrations.checksum` or delete the row and re-run

## Migrations

- Naming: `YYYYMMDDhhmmss_description.sql` (SQLx convention)
- Always include `PRAGMA foreign_keys = ON` if modifying foreign key relationships
- Use `ON DELETE CASCADE` for child records that should be removed with parents
- Use CHECK constraints for enum columns: `CHECK (status IN ('todo','inprogress','done'))`
- Add indexes for ALL foreign key columns and frequently-queried fields
- Use `datetime('now', 'subsec')` for timestamp defaults (not `CURRENT_TIMESTAMP`)
- After migration changes: `DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace`
- **Data migrations** (e.g., BLOB→TEXT conversion): must handle missing columns gracefully. Use `PRAGMA table_info(<table>)` checks or wrap in conditional logic. These migrations are no-ops on fresh databases but may fail if column order differs from expectations
- **Renumbering**: When porting migrations from other branches, check for version collisions with `ls crates/db/migrations/ | grep <prefix>`. Renumber if needed (e.g., 20260409 → 20260412)

## Soft Deletes

- Use nullable `deleted_at` column for soft-deletable records
- ALL queries must filter `WHERE deleted_at IS NULL` unless explicitly querying deleted records
- Soft delete: `UPDATE ... SET deleted_at = datetime('now', 'subsec') WHERE id = ?`

## Error Handling

- Model methods return `Result<T, sqlx::Error>` — no custom error wrapping at this layer
- Route handlers convert via `From<sqlx::Error> for ApiError`
- Never `.unwrap()` query results — propagate with `?`

## SQLx Offline Mode

- `.sqlx/` directory contains cached query metadata — MUST be committed to git
- Set `SQLX_OFFLINE=true` in environment (handled by flox)
- Regenerate after ANY query change: `cargo sqlx prepare --workspace`
- Each `query_as!()` call gets a SHA256-hashed JSON file in `.sqlx/`

## Modularity

- One model per file — each file in `models/` corresponds to one database table
- Model methods are static (`Model::find_by_id(pool, id)`) — no mutable self
- Keep query methods focused: one method = one query, one responsibility
- Extract shared query fragments (e.g., common JOINs, filters) into helper methods
- Create/Update DTOs live in the same file as their model — co-locate related types
- Repository pattern is optional — only add when you need to abstract the data layer for testing

## Performance

- Use COALESCE for numeric defaults in aggregations: `COALESCE(SUM(...), 0)`
- Use EXISTS instead of COUNT for boolean checks: `CASE WHEN EXISTS(...) THEN 1 ELSE 0 END`
- Prefer single query with JOINs over N+1 queries in loops
- Add composite indexes for multi-column WHERE clauses

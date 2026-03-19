Here is a comprehensive research summary across all 10 topics.

---

## 1. SQLite in Production (2025-2026)

The "SQLite Renaissance" is real. Production-grade tooling has matured significantly:

**Turso/LibSQL** is the strongest option for new projects. LibSQL is an open-source fork of SQLite that adds native replication, primary + read replicas, and strong SDK ergonomics. It treats SQLite's limitations as engineering problems and solves them at the library level.

**Litestream** is the simplest option for disaster recovery -- a background process that continuously streams WAL changes to S3/object storage. Best balance of simplicity and reliability for small-to-medium apps. Does not provide read replicas though, only backup/restore.

**LiteFS** (Fly.io) provided FUSE-based transparent replication, but LiteFS Cloud was sunset in October 2024 and active development has been deprioritized. Not recommended for new projects.

**rqlite/dqlite** use Raft consensus + SQLite for multi-writer semantics, but you trade simplicity and latency for distributed capabilities.

**When SQLite is enough:** Read-heavy workloads, single-tenant or per-tenant-database patterns, side-project-to-medium scale (sub-10ms reads at low cost). Single-tenant apps with 20-200 orgs using database-per-tenant with SQLite files is a legitimate pattern.

**When you must migrate:** Many concurrent writers to the same tables, need for cross-tenant queries/reporting, row-level security requirements, connection from multiple application servers to the same database, or need for advanced query features (CTEs with write operations, LISTEN/NOTIFY, etc.).

**For your case (20-200 orgs):** SQLite with database-per-tenant is viable if each org's dataset is modest. But if you need cross-org analytics, shared infrastructure queries, or RLS-based security, PostgreSQL becomes necessary.

## 2. SQLite to PostgreSQL Migration

**Strategies by scale:**

- **Under 10GB / tolerance for 1-4 hours downtime:** Maintenance window approach. Export SQLite, transform schema, import to PostgreSQL. Simplest and most reliable.
- **Over 10GB / zero-downtime requirement:** Dual-write strategy:
  1. Configure app to write to both databases simultaneously
  2. Validate consistency for 24-48 hours
  3. Switch reads to PostgreSQL while continuing dual writes
  4. Make PostgreSQL primary, remove SQLite writes

**Tools:**
- **pgloader**: Open-source, uses PostgreSQL's native COPY command for high-performance bulk loading. Handles SQLite-to-Postgres directly.
- **pgroll** (by Xata): Schema migration tool inspired by Reshape, supports expand/contract pattern for zero-downtime schema changes once on Postgres.

**Common pitfalls:**
- SQLite's permissive typing vs PostgreSQL's strict types (TEXT columns containing integers, flexible date formats, etc.)
- BLOB UUIDs in SQLite need conversion to PostgreSQL's native UUID type
- Boolean representation (`0`/`1` in SQLite vs `true`/`false` in Postgres)
- `AUTOINCREMENT` vs `SERIAL`/`GENERATED ALWAYS AS IDENTITY`
- JSON stored as TEXT in SQLite vs native `jsonb` in Postgres
- SQLite's implicit type coercion masks data quality issues that Postgres will reject

## 3. Multi-Tenant Database Architecture

Three main patterns with clear tradeoffs:

**Shared Database, Shared Schema (recommended starting point):**
- All tenants in same tables with `tenant_id` column
- Onboarding = row insert, migrations ship once
- Infrastructure costs scale with usage not customer count
- Risk: query bugs can leak data (mitigated by RLS)
- Best for: 20-200 orgs where you need cross-tenant reporting

**Shared Database, Separate Schemas:**
- Each tenant gets own PostgreSQL schema
- Impossible to accidentally query another tenant's data
- Migrations applied across N schemas (operational complexity grows linearly)
- Best for: moderate tenant count with strong isolation needs

**Database per Tenant:**
- Strongest isolation, simplest compliance (data residency, deletion)
- Viable for dozens-to-hundreds of tenants, not tens of thousands
- Higher operational overhead
- Best for: enterprise customers with data sovereignty requirements

**2025 consensus:** Start with shared schema + RLS. Offer database-per-tenant as a premium enterprise feature. A hybrid approach where small teams share a schema while enterprise customers get isolated databases is increasingly common.

**For your case:** Shared schema with RLS is the right default for 20-200 orgs. You already have `organization_id` on most tables -- that becomes your `tenant_id`.

## 4. Row-Level Security (RLS) in PostgreSQL

**Core pattern for multi-tenant SaaS:**

```sql
-- Enable RLS on table
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;

-- Create policy using session variable
CREATE POLICY tenant_isolation ON tasks
  USING (organization_id = current_setting('app.current_tenant')::uuid);
```

**Implementation approach:**
- `tenant_id` (your `organization_id`) must be a first-class column on every tenant-scoped table -- not inferred via joins
- Application sets tenant context per-request: `SET LOCAL app.current_tenant = '<org_id>'` at the start of each database transaction
- RLS policies filter automatically on every query -- developers write tenant-unaware SQL

**Key details:**
- Multiple permissive policies for the same command are merged with OR (any match = visible)
- Use `AS RESTRICTIVE` for policies that must AND with other policies
- Defense in depth: RLS backs up application-level checks with a hard database boundary
- New features/endpoints are less likely to introduce cross-tenant leaks

**Best practices:**
- Use `SET LOCAL` (transaction-scoped) not `SET` (session-scoped) to prevent tenant context leaking across requests in connection pools
- Test that queries without tenant context return zero rows
- Create policies for all operations (SELECT, INSERT, UPDATE, DELETE)
- Index `organization_id` on all tenant-scoped tables (you already have migration `20260318000000` adding these)

## 5. SQLx with PostgreSQL -- Rust-Specific Patterns

**The `Any` driver for dual-database support:**
SQLx provides an `Any` database driver that proxies to the actual driver at runtime, selected by URL scheme. `AnyPool` connects to SQLite via `sqlite://file.db` or Postgres via `postgres://localhost/db` with no code change.

**Practical migration strategy using feature flags:**

```toml
# Cargo.toml
[features]
default = ["sqlite"]
sqlite = ["sqlx/sqlite"]
postgres = ["sqlx/postgres"]
```

However, there is a significant catch: **compile-time query checking** (`sqlx::query!()`) is database-specific. The `query!` macro validates against a specific database at compile time. For a migration period, you have two options:

1. **Use `SQLX_OFFLINE=true`** (which you already do) with separate `.sqlx` cache files for each database
2. **Use `sqlx::query()` (runtime, unchecked)** during the transition, then switch back to `query!()` once fully on Postgres

**Migration tooling:** `sqlx migrate` supports reversible migrations (`-r` flag creates `up.sql`/`down.sql` pairs). You can maintain parallel migration directories for SQLite and Postgres during transition.

## 6. Database Connection Pooling

**For PostgreSQL in production:**

**PgBouncer** remains the industry standard -- lightweight C-based pooler. Three modes:
- Session pooling: connection assigned per client session
- Transaction pooling: connection assigned per transaction (most common for web apps)
- Statement pooling: connection returned after each statement

**Supavisor** is Supabase's cloud-native replacement for PgBouncer, written in Elixir. Demonstrated handling ~1M concurrent client connections. Supabase is actively migrating all projects from PgBouncer to Supavisor.

**For Rust/Axum specifically:**
- SQLx has built-in connection pooling via `PgPool` / `SqlitePool`
- For a single-server deployment, SQLx's built-in pool is sufficient
- Add PgBouncer when you have multiple application servers or need to manage connection limits (PostgreSQL default is 100 connections)
- Set `max_connections` in SQLx pool to match your PgBouncer `default_pool_size`

**Critical with RLS:** If using transaction pooling (PgBouncer or Supavisor), you MUST use `SET LOCAL` (not `SET`) for tenant context, because `SET LOCAL` is scoped to the transaction and cleared when the connection returns to the pool.

**Connection limits rule of thumb:** PostgreSQL handles ~100-300 direct connections well. Beyond that, use PgBouncer. For 20-200 orgs on a single app server, SQLx's built-in pool is likely sufficient.

## 7. WAL and SQLite Performance at Scale

**Actual benchmarks (2025):**
- WAL mode: ~70,000 reads/s, ~3,600 writes/s
- Rollback mode: ~5,600 reads/s, ~291 writes/s
- WAL reduces p99 latency by 30-60% for concurrent access
- Write p99 stays under 10ms when concurrency < thread count, then increases logarithmically

**Fundamental limitation:** SQLite WAL still permits only one writer at a time. Concurrent readers are fine, but writes are serialized. This is architectural, not a configuration issue.

**WAL2 (SQLite 3.37+):** Uses two WAL files, allowing checkpointing without blocking writers. Up to 4x better write throughput in some scenarios.

**WAL file growth hazard:** If there are always active readers, no checkpoints complete and the WAL file grows unbounded. Ensure "reader gaps" where no processes are reading.

**For your case:** At 20-200 orgs with moderate write volume, WAL mode SQLite is likely fine per-tenant. The ceiling is roughly "a few hundred writes per second per database file." If any single tenant exceeds that, you need Postgres.

## 8. BLOB Storage and UUIDs in SQLite

**UUID storage: TEXT vs BLOB:**
- TEXT UUIDs: 36 chars, ~72+ bytes, human-readable, debuggable
- BLOB UUIDs: 16 bytes, ~45% less space, faster indexing
- However, BLOB requires conversion overhead (`sqlite.Binary()` for queries, `str(UUID())` for results)
- Your current approach (BLOB UUIDs with `DbUuid` wrapper) is the right pattern for performance

**Internal vs External BLOB storage:**
- SQLite reads/writes small BLOBs (thumbnails, icons) 35% faster than individual files
- Threshold: BLOBs < 100-250 KB are better stored internally
- BLOBs > 1 MB should be externalized (S3, filesystem)
- Make BLOB columns last in table definitions, or store BLOBs in a separate table (SQLite scans through BLOB content to reach subsequent fields)

**When migrating to Postgres:** PostgreSQL has native UUID type (16 bytes, indexed efficiently, human-readable in queries). Your `DbUuid` abstraction will simplify this transition -- just change the underlying storage from BLOB to Postgres UUID.

## 9. Database Backup Strategies

**For SQLite (current):**
- **Litestream** is the gold standard: continuously streams WAL changes to S3/GCS/Azure Blob. Supports point-in-time recovery within the WAL retention window (default 24 hours, configurable). Run as a sidecar process or systemd service.
- Restoration: `litestream restore -o /path/to/db.sqlite s3://bucket/db` then start replication again
- For your dev setup: even a cron job copying the SQLite file is acceptable

**For PostgreSQL (future):**
- **pg_basebackup + WAL archiving** is the native PITR approach
- **pgBackRest** is the production standard: incremental backups, parallel backup/restore, S3-compatible storage, encryption
- PostgreSQL 17 adds WAL synchronization during failovers to retain WALs for PITR
- Managed services (RDS, Supabase, Neon) handle this automatically

**Recommendation for transition period:** Start Litestream now for your SQLite databases (especially `dev_assets/db.sqlite`). When you move to Postgres, use pgBackRest or a managed service with automated PITR.

## 10. Embedded Analytics

**DuckDB as OLAP sidecar (recommended for your case):**
- "SQLite for analytics" -- embedded, zero-ops, ships as part of your binary
- Rust crate: `duckdb` on crates.io, ergonomic bindings based on DuckDB C API
- **Sidecar pattern**: Postgres stays your transactional backbone, DuckDB runs alongside for analytics. Periodically sync data from Postgres to DuckDB (or read Postgres directly via DuckDB's Postgres scanner extension)
- Best for B2B SaaS analytics at ~100GB-10TB scale

**ClickHouse:**
- Better for large-scale, real-time analytics with continuous ingestion
- Separate server process, more operational overhead
- Materialized views aggregate raw data into query-ready shapes
- Overkill for 20-200 orgs unless you have high-volume event/telemetry data

**Materialized views in PostgreSQL:**
- Simplest starting point: `CREATE MATERIALIZED VIEW` with `REFRESH MATERIALIZED VIEW CONCURRENTLY`
- Good enough for dashboards that tolerate minutes-old data
- No additional infrastructure
- Start here, graduate to DuckDB sidecar when query complexity or data volume demands it

**For your case:** Start with PostgreSQL materialized views for dashboards. If analytics queries become too heavy for your OLTP database, add DuckDB as an embedded sidecar -- it integrates naturally with your Rust binary and adds no operational overhead.

---

## Summary Recommendation for ORCHA

Given your current SQLite-based Rust/Axum app scaling to 20-200 organizations:

1. **Short term (now):** Add Litestream for SQLite backups to S3. You already have `organization_id` on most tables.
2. **Medium term (migration):** Move to PostgreSQL with shared schema + RLS. Use pgloader for the data migration. Your `DbUuid` abstraction will make UUID conversion straightforward. Use SQLx's feature flags to support both databases during transition.
3. **Long term (scale):** Add PgBouncer when you hit connection limits. Use PostgreSQL materialized views for dashboards first, then add DuckDB sidecar if analytics workload grows. Offer database-per-tenant as an enterprise tier feature.

---

Sources:
- [The SQLite Renaissance (DEV Community)](https://dev.to/pockit_tools/the-sqlite-renaissance-why-the-worlds-most-deployed-database-is-taking-over-production-in-2026-3jcc)
- [LiteFS vs Litestream vs rqlite vs dqlite on VPS in 2025](https://onidel.com/blog/sqlite-replication-vps-2025)
- [Post-PostgreSQL: Is SQLite on the Edge Production Ready? (SitePoint)](https://www.sitepoint.com/sqlite-edge-production-readiness-2026/)
- [Distributed SQLite: LibSQL and Turso (DEV Community)](https://dev.to/dataformathub/distributed-sqlite-why-libsql-and-turso-are-the-new-standard-in-2026-58fk)
- [How to migrate from SQLite to PostgreSQL (Render)](https://render.com/articles/how-to-migrate-from-sqlite-to-postgresql)
- [Best PostgreSQL Migration Tools (Medium)](https://medium.com/@philmcc/best-postgresql-migration-tools-schema-changes-without-downtime-4f2ed92e0a13)
- [Multi-Tenant Database Architecture Patterns (Bytebase)](https://www.bytebase.com/blog/multi-tenant-database-architecture-patterns-explained/)
- [Multi-Tenant Architecture Guide (WorkOS)](https://workos.com/blog/developers-guide-saas-multi-tenant-architecture)
- [Multi-Tenant Database Design 2025 (SqlCheat)](https://sqlcheat.com/blog/multi-tenant-database-design-2025/)
- [PostgreSQL RLS Multi-Tenant (AWS)](https://aws.amazon.com/blogs/database/multi-tenant-data-isolation-with-postgresql-row-level-security/)
- [RLS for Multi-Tenant Applications (Simplyblock)](https://www.simplyblock.io/blog/underated-postgres-multi-tenancy-with-row-level-security/)
- [PostgreSQL RLS for Multi-Tenant SaaS (TechBuddies)](https://www.techbuddies.io/2026/01/01/how-to-implement-postgresql-row-level-security-for-multi-tenant-saas/)
- [RLS for Tenants in Postgres (Crunchy Data)](https://www.crunchydata.com/blog/row-level-security-for-tenants-in-postgres)
- [SQLx Rust Documentation](https://docs.rs/sqlx/latest/sqlx/)
- [SQLx GitHub Repository](https://github.com/launchbadge/sqlx)
- [Supavisor GitHub Repository](https://github.com/supabase/supavisor)
- [Open Source Postgres Connection Poolers (Bytebase)](https://www.bytebase.com/blog/open-source-postgres-connection-pooler/)
- [SQLite WAL Documentation](https://sqlite.org/wal.html)
- [SQLite in Production Benchmark](https://shivekkhurana.com/blog/sqlite-in-production/)
- [Turso Concurrent Writes](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)
- [SQLite Internal vs External BLOBs](https://sqlite.org/intern-v-extern-blob.html)
- [SQLite 35% Faster Than Filesystem](https://sqlite.org/fasterthanfs.html)
- [Litestream Getting Started](https://litestream.io/getting-started/)
- [Litestream How It Works](https://litestream.io/how-it-works/)
- [Backup Strategies for SQLite in Production (Oldmoe)](https://oldmoe.blog/2024/04/30/backup-strategies-for-sqlite-in-production/)
- [PostgreSQL PITR Documentation](https://www.postgresql.org/docs/current/continuous-archiving.html)
- [ClickHouse vs DuckDB 2026 (Tasrie IT)](https://tasrieit.com/blog/clickhouse-vs-duckdb-2026)
- [DuckDB vs ClickHouse (DB Pro)](https://www.dbpro.app/blog/duckdb-vs-clickhouse)
- [Postgres + DuckDB Sidecar Patterns (Medium)](https://medium.com/@connect.hashblock/7-postgres-duckdb-sidecar-patterns-for-budget-htap-b4345b77a831)
- [DuckDB Rust Client](https://duckdb.org/docs/stable/clients/rust)
- [DuckDB Rust Crate (crates.io)](https://crates.io/crates/duckdb)
- [OLAP Databases 2026 (Tinybird)](https://www.tinybird.co/blog/best-database-for-olap)
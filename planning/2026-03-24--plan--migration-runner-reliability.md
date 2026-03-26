# Migration Runner Reliability Improvements

**Date**: 2026-03-24
**Status**: PLANNING
**Goal**: Fix migration collision issues and improve reliability of the SQLx migration system

---

## Problem Analysis

### Evidence of Collisions

Git history shows **7+ collision fix commits**:
- `320f64a2` - "fix: rename webhook migration to avoid version collision"
- `097252ed` - "chore: rename org_cloud migration to avoid version collision"
- `4fada867` - "fix: resolve route conflicts and migration version collision"
- `6c643b5c` - "fix: resolve timestamp collision and add missing meeting_sessions table"
- `817bae5c` - "fix: resolve migration timestamp collisions and duplicate route"

### Root Causes

1. **Same-day sequential timestamps** — Multiple devs create migrations on same day
   - 20260313200000, 20260313200001, 20260313200002... (8 in one day)
   - Git merge order != timestamp order → FK constraint failures

2. **No concurrency protection** — Multiple server instances can run migrations simultaneously
   - No SQLite busy_timeout configured
   - No distributed locking mechanism

3. **Partial failure risk** — Large migrations lack transaction wrapping
   - `20260328000000_uuid_blob_to_text.sql` (672 lines) - no explicit transaction
   - DROP TABLE before INSERT complete → data loss on failure

4. **Silent failures** — `set_ignore_missing(true)` hides missing migrations

5. **FK disabled during migration** — Invalid data can be inserted

---

## Implementation Plan

### Phase 1: Concurrency Protection (High Priority)

**File: `crates/db/src/lib.rs`**

```rust
// Before (current):
let migration_options = SqliteConnectOptions::from_str(&database_url)?
    .create_if_missing(true)
    .foreign_keys(false);

// After:
let migration_options = SqliteConnectOptions::from_str(&database_url)?
    .create_if_missing(true)
    .foreign_keys(false)
    .busy_timeout(Duration::from_secs(30))  // Wait for locks
    .pragma("journal_mode", "WAL");         // Better concurrent access
```

Add migration lock to prevent concurrent runs:

```rust
async fn acquire_migration_lock(pool: &SqlitePool) -> Result<(), Error> {
    // Create lock table if not exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migration_lock (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            locked_at TEXT NOT NULL,
            locked_by TEXT NOT NULL
        )"
    ).execute(pool).await?;

    // Try to acquire lock (fails if already locked)
    let hostname = hostname::get().unwrap_or_default().to_string_lossy().to_string();
    sqlx::query(
        "INSERT INTO _migration_lock (id, locked_at, locked_by) VALUES (1, datetime('now'), ?)"
    )
    .bind(&hostname)
    .execute(pool)
    .await
    .map_err(|_| Error::Protocol("Migration already in progress".into()))?;

    Ok(())
}

async fn release_migration_lock(pool: &SqlitePool) -> Result<(), Error> {
    sqlx::query("DELETE FROM _migration_lock WHERE id = 1")
        .execute(pool)
        .await?;
    Ok(())
}
```

### Phase 2: Better Error Handling & Logging

**File: `crates/db/src/lib.rs`**

```rust
// Before:
sqlx::migrate!("./migrations")
    .set_ignore_missing(true)
    .run(&migration_pool)
    .await?;

// After:
let migrator = sqlx::migrate!("./migrations");

// Log pending migrations
let applied: HashSet<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
    .fetch_all(&migration_pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

for migration in migrator.iter() {
    if !applied.contains(&migration.version) {
        tracing::info!(
            "Pending migration: {} - {}",
            migration.version,
            migration.description
        );
    }
}

// Run with explicit error context
match migrator.run(&migration_pool).await {
    Ok(_) => tracing::info!("All migrations completed successfully"),
    Err(e) => {
        tracing::error!("Migration failed: {}", e);
        // Query which migration failed
        let last_applied: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM _sqlx_migrations"
        ).fetch_one(&migration_pool).await.ok().flatten();

        if let Some(v) = last_applied {
            tracing::error!("Last successful migration: {}", v);
        }
        return Err(e);
    }
}
```

### Phase 3: Timestamp Collision Prevention

**New file: `scripts/new-migration.sh`**

```bash
#!/bin/bash
# Generate migration with high-precision timestamp to prevent collisions

NAME="${1:?Usage: ./scripts/new-migration.sh <migration_name>}"
TIMESTAMP=$(date +%Y%m%d%H%M%S)  # Include HHMMSS, not just YYYYMMDD000000
FILENAME="crates/db/migrations/${TIMESTAMP}_${NAME}.sql"

cat > "$FILENAME" << 'EOF'
-- Migration: ${NAME}
-- Created: $(date -Iseconds)
--
-- IMPORTANT: Wrap destructive operations in transactions
-- Use IF NOT EXISTS for idempotency

BEGIN TRANSACTION;

-- Your migration here

COMMIT;
EOF

echo "Created: $FILENAME"
```

**Add pre-commit hook: `.githooks/pre-commit`**

```bash
#!/bin/bash
# Check for migration timestamp collisions

MIGRATIONS=$(git diff --cached --name-only | grep 'crates/db/migrations/')
if [ -n "$MIGRATIONS" ]; then
    # Extract timestamps
    TIMESTAMPS=$(echo "$MIGRATIONS" | sed 's/.*\/\([0-9]*\)_.*/\1/' | sort)
    UNIQUE=$(echo "$TIMESTAMPS" | uniq)

    if [ "$TIMESTAMPS" != "$UNIQUE" ]; then
        echo "ERROR: Duplicate migration timestamps detected!"
        echo "$MIGRATIONS"
        exit 1
    fi

    # Check for same-day collisions with existing migrations
    for TS in $TIMESTAMPS; do
        DAY_PREFIX="${TS:0:8}"
        EXISTING=$(ls crates/db/migrations/${DAY_PREFIX}*.sql 2>/dev/null | wc -l)
        if [ "$EXISTING" -gt 1 ]; then
            echo "WARNING: Multiple migrations on same day ($DAY_PREFIX)"
            echo "Consider using full timestamp (YYYYMMDDHHMMSS)"
        fi
    done
fi
```

### Phase 4: FK Validation Post-Migration

**File: `crates/db/src/lib.rs`**

```rust
async fn validate_foreign_keys(pool: &SqlitePool) -> Result<(), Error> {
    // Re-enable FK and check for violations
    sqlx::query("PRAGMA foreign_keys = ON").execute(pool).await?;

    let violations: Vec<(String, String, i64)> = sqlx::query_as(
        "PRAGMA foreign_key_check"
    ).fetch_all(pool).await?;

    if !violations.is_empty() {
        tracing::error!("Foreign key violations detected after migration:");
        for (table, parent, rowid) in &violations {
            tracing::error!("  Table: {}, Parent: {}, RowID: {}", table, parent, rowid);
        }
        return Err(Error::Protocol(
            format!("{} FK violations found", violations.len())
        ));
    }

    tracing::info!("FK validation passed");
    Ok(())
}
```

### Phase 5: Cleanup Duplicate Migrations

**Consolidate these duplicates:**

| Keep | Remove/Merge |
|------|--------------|
| `20260305200001_crm_enhancements.sql` | `20260305200000_org_invite_and_company_org.sql` |
| `20260318000000_organization_brand_profile.sql` | `20260317100000_org_invite_and_company_org.sql` |
| Remove | `20260304000000_meeting_person_links.sql` (no-op) |

**Steps:**
1. Identify all applied migrations from `_sqlx_migrations` table
2. Create consolidated migration that handles all cases
3. Mark duplicates as applied in `_sqlx_migrations` without re-running

---

## Files Summary

| File | Action |
|------|--------|
| `crates/db/src/lib.rs` | **Modify** — Add concurrency lock, logging, FK validation |
| `scripts/new-migration.sh` | **Create** — Migration generator with precise timestamps |
| `.githooks/pre-commit` | **Create** — Collision detection hook |
| `crates/db/migrations/` | **Cleanup** — Remove/consolidate duplicates |

---

## Migration Runner Improvements Summary

```
BEFORE                              AFTER
──────────────────────────────────  ──────────────────────────────────
No concurrency protection     →     Migration lock table + busy_timeout
Silent failures               →     Explicit logging per migration
FK disabled, never validated  →     FK validation post-migration
YYYYMMDD000000 timestamps     →     YYYYMMDDHHMMSS precise timestamps
No collision prevention       →     Pre-commit hook + generator script
```

---

## Verification

1. **Concurrency test:**
   ```bash
   # Start 3 server instances simultaneously
   for i in 1 2 3; do ./server & done
   # Verify only one runs migrations (others wait/skip)
   ```

2. **Collision detection:**
   ```bash
   # Create migration with same timestamp
   touch crates/db/migrations/20260324000000_test1.sql
   touch crates/db/migrations/20260324000000_test2.sql
   git add . && git commit  # Should fail pre-commit hook
   ```

3. **FK validation:**
   ```bash
   # Insert orphaned FK reference
   sqlite3 dev_assets/db.sqlite "INSERT INTO tasks (id, project_id) VALUES ('x', 'nonexistent')"
   # Start server → should fail FK validation
   ```

---

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| Migration lock | Low | Lock auto-releases on crash (table deleted) |
| FK validation | Medium | Can disable with `SKIP_FK_VALIDATION=1` |
| Pre-commit hook | Low | Optional, developers can bypass |
| Duplicate cleanup | High | Must verify all envs have same migration state |

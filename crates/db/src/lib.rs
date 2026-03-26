use std::{str::FromStr, sync::Arc, time::Duration};

use sqlx::{
    Error, Pool, Postgres, Sqlite, SqlitePool,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteConnection, SqliteJournalMode, SqlitePoolOptions},
};
use utils::assets::asset_dir;

/// Default busy timeout for SQLite connections (prevents "database locked" errors)
const SQLITE_BUSY_TIMEOUT_SECS: u64 = 30;

pub mod constants;
pub mod db_uuid;
pub mod models;
pub mod repositories;
pub mod services;

pub use db_uuid::{DbUuid, bind_optional_uuid, bind_optional_uuid_blob, bind_uuid, bind_uuid_blob};

// ============================================================================
// Migration Locking - Prevents concurrent migration runs
// ============================================================================

/// Acquire an exclusive migration lock. Returns error if another process holds the lock.
async fn acquire_migration_lock(pool: &SqlitePool) -> Result<(), Error> {
    // Create lock table if not exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migration_lock (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            locked_at TEXT NOT NULL,
            locked_by TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Check for stale lock (older than 10 minutes - likely crashed process)
    let stale_threshold_minutes = 10;
    sqlx::query(
        "DELETE FROM _migration_lock
         WHERE datetime(locked_at) < datetime('now', ?)",
    )
    .bind(format!("-{} minutes", stale_threshold_minutes))
    .execute(pool)
    .await?;

    // Try to acquire lock
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let pid = std::process::id();
    let lock_holder = format!("{}:{}", hostname, pid);

    match sqlx::query(
        "INSERT INTO _migration_lock (id, locked_at, locked_by)
         VALUES (1, datetime('now'), ?)",
    )
    .bind(&lock_holder)
    .execute(pool)
    .await
    {
        Ok(_) => {
            tracing::info!("Acquired migration lock: {}", lock_holder);
            Ok(())
        }
        Err(sqlx::Error::Database(e)) if e.message().contains("UNIQUE constraint failed") => {
            // Another process holds the lock - check who
            let holder: Option<String> = sqlx::query_scalar(
                "SELECT locked_by FROM _migration_lock WHERE id = 1",
            )
            .fetch_optional(pool)
            .await?;

            let msg = format!(
                "Migration already in progress (held by: {})",
                holder.unwrap_or_else(|| "unknown".to_string())
            );
            tracing::warn!("{}", msg);
            Err(Error::Protocol(msg))
        }
        Err(e) => Err(e),
    }
}

/// Release the migration lock
async fn release_migration_lock(pool: &SqlitePool) -> Result<(), Error> {
    sqlx::query("DELETE FROM _migration_lock WHERE id = 1")
        .execute(pool)
        .await?;
    tracing::info!("Released migration lock");
    Ok(())
}

// ============================================================================
// Migration Runner with Logging
// ============================================================================

/// Run migrations with detailed logging
async fn run_migrations(pool: &SqlitePool) -> Result<(), Error> {
    let migrator = sqlx::migrate!("./migrations");

    // Get already-applied migrations
    let applied: std::collections::HashSet<i64> = sqlx::query_scalar(
        "SELECT version FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

    // Log pending migrations
    let pending: Vec<_> = migrator
        .iter()
        .filter(|m| !applied.contains(&m.version))
        .collect();

    if pending.is_empty() {
        tracing::info!("No pending migrations");
        return Ok(());
    }

    tracing::info!(
        "Running {} pending migration(s) (of {} total)",
        pending.len(),
        migrator.iter().len()
    );

    for migration in &pending {
        tracing::info!(
            "  Pending: {} - {}",
            migration.version,
            migration.description
        );
    }

    // Run migrations
    match migrator.run(pool).await {
        Ok(_) => {
            tracing::info!("All migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            // Try to identify which migration failed
            let last_applied: Option<i64> =
                sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
                    .fetch_one(pool)
                    .await
                    .ok()
                    .flatten();

            tracing::error!("Migration failed: {}", e);
            if let Some(v) = last_applied {
                tracing::error!("Last successful migration: {}", v);

                // Find the next migration that should have run
                if let Some(failed) = pending.iter().find(|m| m.version > v) {
                    tracing::error!(
                        "Failed migration: {} - {}",
                        failed.version,
                        failed.description
                    );
                }
            }

            Err(e.into())
        }
    }
}

// ============================================================================
// FK Validation - Checks foreign key integrity after migrations
// ============================================================================

/// Validate foreign key integrity after migrations complete.
/// By default, logs warnings but doesn't fail. Set STRICT_FK_VALIDATION=1 to fail on violations.
async fn validate_foreign_keys(pool: &SqlitePool) -> Result<(), Error> {
    // PRAGMA foreign_key_check returns rows for FK violations:
    // (table, rowid, parent_table, fkid)
    let violations: Vec<(String, i64, String, i64)> =
        sqlx::query_as("SELECT \"table\", rowid, parent, fkid FROM pragma_foreign_key_check()")
            .fetch_all(pool)
            .await?;

    if violations.is_empty() {
        tracing::info!("FK validation passed");
        return Ok(());
    }

    tracing::error!(
        "Foreign key violations detected: {} violation(s)",
        violations.len()
    );
    for (table, rowid, parent, _fkid) in &violations {
        tracing::error!(
            "  Table '{}' row {} -> missing parent in '{}'",
            table,
            rowid,
            parent
        );
    }

    // Only fail if STRICT_FK_VALIDATION is explicitly set
    if std::env::var("STRICT_FK_VALIDATION").unwrap_or_default() == "1" {
        return Err(Error::Protocol(format!(
            "{} foreign key violations found - set STRICT_FK_VALIDATION=0 to ignore",
            violations.len()
        )));
    }

    tracing::warn!("Continuing despite FK violations (set STRICT_FK_VALIDATION=1 to enforce)");
    Ok(())
}

// ============================================================================
// Database Services
// ============================================================================

#[derive(Clone)]
pub struct DBService {
    pub pool: Pool<Sqlite>,
}

#[derive(Clone)]
pub struct PgDBService {
    pub pool: Pool<Postgres>,
}

impl DBService {
    pub async fn new() -> Result<DBService, Error> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            format!(
                "sqlite://{}",
                asset_dir().join("db.sqlite").to_string_lossy()
            )
        });

        // Migration pool options:
        // - foreign_keys=OFF: Allow data-only migrations with production UUIDs
        // - busy_timeout: Wait for locks instead of failing immediately
        // - journal_mode=WAL: Better concurrent access
        let migration_options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true)
            .foreign_keys(false)
            .busy_timeout(Duration::from_secs(SQLITE_BUSY_TIMEOUT_SECS))
            .journal_mode(SqliteJournalMode::Wal);

        let migration_pool = SqlitePool::connect_with(migration_options).await?;

        if std::env::var("SKIP_MIGRATIONS").unwrap_or_default() != "1" {
            // Acquire lock to prevent concurrent migrations
            acquire_migration_lock(&migration_pool).await?;

            let result = run_migrations(&migration_pool).await;

            // Always release lock, even on failure
            let _ = release_migration_lock(&migration_pool).await;

            // Propagate migration error
            result?;

            // Validate FK integrity (after migrations, before app pool)
            if std::env::var("SKIP_FK_VALIDATION").unwrap_or_default() != "1" {
                validate_foreign_keys(&migration_pool).await?;
            }
        }

        migration_pool.close().await;

        // App pool with default settings (FK enforcement ON)
        let options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(SQLITE_BUSY_TIMEOUT_SECS))
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(options).await?;
        Ok(DBService { pool })
    }

    pub async fn new_with_after_connect<F>(after_connect: F) -> Result<DBService, Error>
    where
        F: for<'a> Fn(
                &'a mut SqliteConnection,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), Error>> + Send + 'a>,
            > + Send
            + Sync
            + 'static,
    {
        let pool = Self::create_pool(Some(Arc::new(after_connect))).await?;
        Ok(DBService { pool })
    }

    async fn create_pool<F>(after_connect: Option<Arc<F>>) -> Result<Pool<Sqlite>, Error>
    where
        F: for<'a> Fn(
                &'a mut SqliteConnection,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), Error>> + Send + 'a>,
            > + Send
            + Sync
            + 'static,
    {
        let database_url = format!(
            "sqlite://{}",
            asset_dir().join("db.sqlite").to_string_lossy()
        );

        // Base options with busy_timeout and WAL mode
        let base_options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(SQLITE_BUSY_TIMEOUT_SECS))
            .journal_mode(SqliteJournalMode::Wal);

        // Migration pool: FK enforcement OFF for data-only migrations
        let migration_options = base_options.clone().foreign_keys(false);
        let migration_pool = SqlitePool::connect_with(migration_options).await?;

        if std::env::var("SKIP_MIGRATIONS").unwrap_or_default() != "1" {
            // Acquire lock to prevent concurrent migrations
            acquire_migration_lock(&migration_pool).await?;

            let result = run_migrations(&migration_pool).await;

            // Always release lock, even on failure
            let _ = release_migration_lock(&migration_pool).await;

            // Propagate migration error
            result?;

            // Validate FK integrity (after migrations, before app pool)
            if std::env::var("SKIP_FK_VALIDATION").unwrap_or_default() != "1" {
                validate_foreign_keys(&migration_pool).await?;
            }
        }

        migration_pool.close().await;

        // App pool with FK enforcement ON
        let pool = if let Some(hook) = after_connect {
            SqlitePoolOptions::new()
                .after_connect(move |conn, _meta| {
                    let hook = hook.clone();
                    Box::pin(async move {
                        hook(conn).await?;
                        Ok(())
                    })
                })
                .connect_with(base_options)
                .await?
        } else {
            SqlitePool::connect_with(base_options).await?
        };

        Ok(pool)
    }
}

impl PgDBService {
    /// Create a new PostgreSQL database service
    /// Requires DATABASE_URL environment variable
    pub async fn new() -> Result<PgDBService, Error> {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for PostgreSQL connection");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations_pg").run(&pool).await?;

        Ok(PgDBService { pool })
    }

    /// Create a new PostgreSQL database service with custom URL
    pub async fn new_with_url(database_url: &str) -> Result<PgDBService, Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations_pg").run(&pool).await?;

        Ok(PgDBService { pool })
    }
}

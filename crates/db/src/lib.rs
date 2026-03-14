use std::{str::FromStr, sync::Arc};

use sqlx::{
    Error, Pool, Postgres, Sqlite, SqlitePool,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePoolOptions},
};
use utils::assets::asset_dir;

pub mod constants;
pub mod db_uuid;
pub mod models;
pub mod repositories;
pub mod services;

pub use db_uuid::DbUuid;

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
        let database_url = format!(
            "sqlite://{}",
            asset_dir().join("db.sqlite").to_string_lossy()
        );
        // sqlx 0.8+ defaults foreign_keys=ON; disable for migration to allow
        // data-only migrations with production UUIDs that may not exist in dev.
        // Re-enabled post-migration via after_connect hook on the app pool.
        let migration_options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true)
            .foreign_keys(false);
        let migration_pool = SqlitePool::connect_with(migration_options).await?;
        sqlx::migrate!("./migrations").run(&migration_pool).await?;
        migration_pool.close().await;

        let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
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
        let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);

        // Run migrations with FK enforcement OFF so data-only migrations with
        // production UUIDs don't fail on dev/staging databases.
        let migration_options = options.clone().foreign_keys(false);
        let migration_pool = SqlitePool::connect_with(migration_options).await?;
        sqlx::migrate!("./migrations").run(&migration_pool).await?;
        migration_pool.close().await;

        let pool = if let Some(hook) = after_connect {
            SqlitePoolOptions::new()
                .after_connect(move |conn, _meta| {
                    let hook = hook.clone();
                    Box::pin(async move {
                        hook(conn).await?;
                        Ok(())
                    })
                })
                .connect_with(options)
                .await?
        } else {
            SqlitePool::connect_with(options).await?
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

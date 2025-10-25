use std::{str::FromStr, sync::Arc};

use sqlx::{
    Error, Pool, Postgres, Sqlite, SqlitePool,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePoolOptions},
};
use utils::assets::asset_dir;

pub mod models;
pub mod repositories;
pub mod services;

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
        let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
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

        sqlx::migrate!("./migrations").run(&pool).await?;
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

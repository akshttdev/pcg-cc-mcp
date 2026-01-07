use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CmsSiteError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Site not found")]
    NotFound,
    #[error("Site with slug already exists")]
    SlugExists,
    #[error("Site with domain already exists")]
    DomainExists,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CmsSite {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub domain: String,
    pub theme_config: Option<String>,
    pub is_active: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCmsSite {
    pub slug: String,
    pub name: String,
    pub domain: String,
    pub theme_config: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCmsSite {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub theme_config: Option<String>,
    pub is_active: Option<bool>,
}

impl CmsSite {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsSite,
            r#"SELECT
                id as "id!: Uuid",
                slug,
                name,
                domain,
                theme_config,
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_sites
            WHERE is_active = 1
            ORDER BY name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsSite,
            r#"SELECT
                id as "id!: Uuid",
                slug,
                name,
                domain,
                theme_config,
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_sites
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_slug(pool: &SqlitePool, slug: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsSite,
            r#"SELECT
                id as "id!: Uuid",
                slug,
                name,
                domain,
                theme_config,
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_sites
            WHERE slug = $1"#,
            slug
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateCmsSite) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            CmsSite,
            r#"INSERT INTO cms_sites (id, slug, name, domain, theme_config)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id as "id!: Uuid",
                slug,
                name,
                domain,
                theme_config,
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.slug,
            data.name,
            data.domain,
            data.theme_config
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateCmsSite,
    ) -> Result<Self, sqlx::Error> {
        let current = Self::find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_ref().unwrap_or(&current.name);
        let domain = data.domain.as_ref().unwrap_or(&current.domain);
        let theme_config = data.theme_config.as_ref().or(current.theme_config.as_ref());
        let is_active = data.is_active.unwrap_or(current.is_active);

        sqlx::query_as!(
            CmsSite,
            r#"UPDATE cms_sites
            SET name = $2, domain = $3, theme_config = $4, is_active = $5, updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                slug,
                name,
                domain,
                theme_config,
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            domain,
            theme_config,
            is_active
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM cms_sites WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

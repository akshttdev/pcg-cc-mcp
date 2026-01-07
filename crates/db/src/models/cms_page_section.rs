use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CmsPageSectionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Page section not found")]
    NotFound,
    #[error("Section with this key already exists for this page")]
    SectionKeyExists,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CmsPageSection {
    pub id: Uuid,
    pub site_id: Uuid,
    pub page_slug: String,
    pub section_key: String,
    pub content: String,
    pub sort_order: i64,
    pub is_active: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCmsPageSection {
    pub page_slug: String,
    pub section_key: String,
    pub content: String,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCmsPageSection {
    pub content: Option<String>,
    pub sort_order: Option<i64>,
    pub is_active: Option<bool>,
}

impl CmsPageSection {
    pub async fn find_by_site(pool: &SqlitePool, site_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsPageSection,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_page_sections
            WHERE site_id = $1
            ORDER BY page_slug ASC, sort_order ASC"#,
            site_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_page(
        pool: &SqlitePool,
        site_id: Uuid,
        page_slug: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsPageSection,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_page_sections
            WHERE site_id = $1 AND page_slug = $2
            ORDER BY sort_order ASC"#,
            site_id,
            page_slug
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_active_by_page(
        pool: &SqlitePool,
        site_id: Uuid,
        page_slug: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsPageSection,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_page_sections
            WHERE site_id = $1 AND page_slug = $2 AND is_active = 1
            ORDER BY sort_order ASC"#,
            site_id,
            page_slug
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsPageSection,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_page_sections
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_key(
        pool: &SqlitePool,
        site_id: Uuid,
        page_slug: &str,
        section_key: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsPageSection,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_page_sections
            WHERE site_id = $1 AND page_slug = $2 AND section_key = $3"#,
            site_id,
            page_slug,
            section_key
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        site_id: Uuid,
        data: &CreateCmsPageSection,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let sort_order = data.sort_order.unwrap_or(0);

        sqlx::query_as!(
            CmsPageSection,
            r#"INSERT INTO cms_page_sections (id, site_id, page_slug, section_key, content, sort_order)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            site_id,
            data.page_slug,
            data.section_key,
            data.content,
            sort_order
        )
        .fetch_one(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        site_id: Uuid,
        data: &CreateCmsPageSection,
    ) -> Result<Self, sqlx::Error> {
        // Check if section exists
        if let Some(existing) = Self::find_by_key(pool, site_id, &data.page_slug, &data.section_key).await? {
            // Update existing
            Self::update(pool, existing.id, &UpdateCmsPageSection {
                content: Some(data.content.clone()),
                sort_order: data.sort_order,
                is_active: None,
            }).await
        } else {
            // Create new
            Self::create(pool, site_id, data).await
        }
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateCmsPageSection,
    ) -> Result<Self, sqlx::Error> {
        let current = Self::find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;

        let content = data.content.as_ref().unwrap_or(&current.content);
        let sort_order = data.sort_order.unwrap_or(current.sort_order);
        let is_active = data.is_active.unwrap_or(current.is_active);

        sqlx::query_as!(
            CmsPageSection,
            r#"UPDATE cms_page_sections SET
                content = $2, sort_order = $3, is_active = $4,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                page_slug,
                section_key,
                content,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            content,
            sort_order,
            is_active
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM cms_page_sections WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

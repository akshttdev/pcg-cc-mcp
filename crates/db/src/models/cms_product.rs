use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CmsProductError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Product not found")]
    NotFound,
    #[error("Product with slug already exists for this site")]
    SlugExists,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CmsProduct {
    pub id: Uuid,
    pub site_id: Uuid,
    pub slug: String,
    pub name: String,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub stripe_price_id: Option<String>,
    pub image_url: Option<String>,
    pub gallery_images: Option<String>,
    pub specs: Option<String>,
    pub features: Option<String>,
    pub is_active: bool,
    pub is_featured: bool,
    pub stock_status: Option<String>,
    pub sort_order: i64,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCmsProduct {
    pub slug: String,
    pub name: String,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub price_cents: i64,
    pub currency: Option<String>,
    pub stripe_price_id: Option<String>,
    pub image_url: Option<String>,
    pub gallery_images: Option<String>,
    pub specs: Option<String>,
    pub features: Option<String>,
    pub is_featured: Option<bool>,
    pub stock_status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCmsProduct {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub stripe_price_id: Option<String>,
    pub image_url: Option<String>,
    pub gallery_images: Option<String>,
    pub specs: Option<String>,
    pub features: Option<String>,
    pub is_active: Option<bool>,
    pub is_featured: Option<bool>,
    pub stock_status: Option<String>,
    pub sort_order: Option<i64>,
}

impl CmsProduct {
    pub async fn find_by_site(pool: &SqlitePool, site_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsProduct,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                slug,
                name,
                short_description,
                long_description,
                price_cents as "price_cents!: i64",
                currency,
                stripe_price_id,
                image_url,
                gallery_images,
                specs,
                features,
                is_active as "is_active!: bool",
                is_featured as "is_featured!: bool",
                stock_status,
                sort_order as "sort_order!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_products
            WHERE site_id = $1
            ORDER BY sort_order ASC, name ASC"#,
            site_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_active_by_site(pool: &SqlitePool, site_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsProduct,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                slug,
                name,
                short_description,
                long_description,
                price_cents as "price_cents!: i64",
                currency,
                stripe_price_id,
                image_url,
                gallery_images,
                specs,
                features,
                is_active as "is_active!: bool",
                is_featured as "is_featured!: bool",
                stock_status,
                sort_order as "sort_order!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_products
            WHERE site_id = $1 AND is_active = 1
            ORDER BY sort_order ASC, name ASC"#,
            site_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsProduct,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                slug,
                name,
                short_description,
                long_description,
                price_cents as "price_cents!: i64",
                currency,
                stripe_price_id,
                image_url,
                gallery_images,
                specs,
                features,
                is_active as "is_active!: bool",
                is_featured as "is_featured!: bool",
                stock_status,
                sort_order as "sort_order!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_products
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_slug(
        pool: &SqlitePool,
        site_id: Uuid,
        slug: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsProduct,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                slug,
                name,
                short_description,
                long_description,
                price_cents as "price_cents!: i64",
                currency,
                stripe_price_id,
                image_url,
                gallery_images,
                specs,
                features,
                is_active as "is_active!: bool",
                is_featured as "is_featured!: bool",
                stock_status,
                sort_order as "sort_order!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_products
            WHERE site_id = $1 AND slug = $2"#,
            site_id,
            slug
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        site_id: Uuid,
        data: &CreateCmsProduct,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let currency = data.currency.clone().unwrap_or_else(|| "USD".to_string());
        let is_featured = data.is_featured.unwrap_or(false);
        let sort_order = data.sort_order.unwrap_or(0);

        sqlx::query_as!(
            CmsProduct,
            r#"INSERT INTO cms_products (
                id, site_id, slug, name, short_description, long_description,
                price_cents, currency, stripe_price_id, image_url, gallery_images,
                specs, features, is_featured, stock_status, sort_order
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                slug,
                name,
                short_description,
                long_description,
                price_cents as "price_cents!: i64",
                currency,
                stripe_price_id,
                image_url,
                gallery_images,
                specs,
                features,
                is_active as "is_active!: bool",
                is_featured as "is_featured!: bool",
                stock_status,
                sort_order as "sort_order!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            site_id,
            data.slug,
            data.name,
            data.short_description,
            data.long_description,
            data.price_cents,
            currency,
            data.stripe_price_id,
            data.image_url,
            data.gallery_images,
            data.specs,
            data.features,
            is_featured,
            data.stock_status,
            sort_order
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateCmsProduct,
    ) -> Result<Self, sqlx::Error> {
        let current = Self::find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;

        let slug = data.slug.as_ref().unwrap_or(&current.slug);
        let name = data.name.as_ref().unwrap_or(&current.name);
        let short_description = data.short_description.as_ref().or(current.short_description.as_ref());
        let long_description = data.long_description.as_ref().or(current.long_description.as_ref());
        let price_cents = data.price_cents.unwrap_or(current.price_cents);
        let currency = data.currency.as_ref().unwrap_or(&current.currency);
        let stripe_price_id = data.stripe_price_id.as_ref().or(current.stripe_price_id.as_ref());
        let image_url = data.image_url.as_ref().or(current.image_url.as_ref());
        let gallery_images = data.gallery_images.as_ref().or(current.gallery_images.as_ref());
        let specs = data.specs.as_ref().or(current.specs.as_ref());
        let features = data.features.as_ref().or(current.features.as_ref());
        let is_active = data.is_active.unwrap_or(current.is_active);
        let is_featured = data.is_featured.unwrap_or(current.is_featured);
        let stock_status = data.stock_status.as_ref().or(current.stock_status.as_ref());
        let sort_order = data.sort_order.unwrap_or(current.sort_order);

        sqlx::query_as!(
            CmsProduct,
            r#"UPDATE cms_products SET
                slug = $2, name = $3, short_description = $4, long_description = $5,
                price_cents = $6, currency = $7, stripe_price_id = $8, image_url = $9,
                gallery_images = $10, specs = $11, features = $12, is_active = $13,
                is_featured = $14, stock_status = $15, sort_order = $16,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                slug,
                name,
                short_description,
                long_description,
                price_cents as "price_cents!: i64",
                currency,
                stripe_price_id,
                image_url,
                gallery_images,
                specs,
                features,
                is_active as "is_active!: bool",
                is_featured as "is_featured!: bool",
                stock_status,
                sort_order as "sort_order!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            slug,
            name,
            short_description,
            long_description,
            price_cents,
            currency,
            stripe_price_id,
            image_url,
            gallery_images,
            specs,
            features,
            is_active,
            is_featured,
            stock_status,
            sort_order
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM cms_products WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

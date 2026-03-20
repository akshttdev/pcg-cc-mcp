use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Error)]
pub enum CompanyError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Company not found")]
    NotFound,
    #[error("A company with that slug already exists")]
    SlugConflict,
}

/// A real-world company as a public knowledge-graph entity.
/// Distinct from `organizations` (which is a company INSIDE the platform).
/// When a company joins the platform, `organization_id` is set.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Company {
    pub id: DbUuid,
    pub name: String,
    pub slug: Option<String>,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub cover_image_url: Option<String>,
    pub headquarters: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,

    // Direct contact
    pub phone: Option<String>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub instagram_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub facebook_url: Option<String>,

    // Business details
    pub founded_year: Option<i32>,
    pub employee_count: Option<String>, // '1-10','11-50','51-200','201-500','500+'
    pub tags: Option<String>,           // JSON array
    pub business_hours: Option<String>, // JSON object {mon:'9am-5pm', ...}
    pub notes: Option<String>,

    // Google My Business
    pub gmb_rating: Option<f64>,
    pub gmb_review_count: Option<i32>,
    pub gmb_place_id: Option<String>,

    // Public intelligence
    pub intelligence_summary: Option<String>,
    pub intelligence_raw: Option<String>,
    pub intelligence_status: String,
    pub intelligence_last_run_at: Option<DateTime<Utc>>,
    pub intelligence_confidence: Option<f64>,
    pub intelligence_agent: Option<String>,

    // Platform links
    pub organization_id: Option<DbUuid>, // set when company joins as an org
    pub created_by_org_id: Option<DbUuid>, // admin org that discovered/created this

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCompany {
    pub name: String,
    pub slug: Option<String>,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub headquarters: Option<String>,
    pub created_by_org_id: Option<DbUuid>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCompany {
    pub name: Option<String>,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub cover_image_url: Option<String>,
    pub headquarters: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub instagram_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub facebook_url: Option<String>,
    pub founded_year: Option<i32>,
    pub employee_count: Option<String>,
    pub tags: Option<String>,
    pub business_hours: Option<String>,
    pub notes: Option<String>,
    pub gmb_rating: Option<f64>,
    pub gmb_review_count: Option<i32>,
    pub gmb_place_id: Option<String>,
    pub organization_id: Option<DbUuid>,
    pub intelligence_summary: Option<String>,
    pub intelligence_status: Option<String>,
}

impl Company {
    pub async fn find_by_id(pool: &SqlitePool, id: &DbUuid) -> Result<Option<Self>, CompanyError> {
        // Companies store id as BLOB; DbUuid encodes as TEXT so direct `= ?` misses.
        // Use hex(id) comparison which works for both BLOB and TEXT storage.
        let hex_no_dashes = id.to_string().replace('-', "");
        let row =
            sqlx::query_as::<_, Self>("SELECT * FROM companies WHERE lower(hex(id)) = lower(?)")
                .bind(&hex_no_dashes)
                .fetch_optional(pool)
                .await?;
        Ok(row)
    }

    pub async fn find_by_slug(pool: &SqlitePool, slug: &str) -> Result<Option<Self>, CompanyError> {
        let row = sqlx::query_as::<_, Self>("SELECT * FROM companies WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Self>, CompanyError> {
        let row = sqlx::query_as::<_, Self>(
            "SELECT * FROM companies WHERE lower(name) = lower(?) LIMIT 1",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Find a company by name scoped to a specific organization.
    /// Used for deduplication during workflow staging commits.
    pub async fn find_by_name_and_org(
        pool: &SqlitePool,
        name: &str,
        org_id: &DbUuid,
    ) -> Result<Option<Self>, CompanyError> {
        let row = sqlx::query_as::<_, Self>(
            "SELECT * FROM companies WHERE lower(name) = lower(?) AND created_by_org_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(name)
        .bind(org_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        org_id: &DbUuid,
    ) -> Result<Option<Self>, CompanyError> {
        let row = sqlx::query_as::<_, Self>("SELECT * FROM companies WHERE organization_id = ?")
            .bind(org_id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn list(
        pool: &SqlitePool,
        created_by_org_id: Option<DbUuid>,
        has_platform_org: Option<bool>,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, CompanyError> {
        let mut qb = sqlx::QueryBuilder::new("SELECT * FROM companies WHERE 1=1");
        if let Some(org_id) = created_by_org_id {
            qb.push(" AND created_by_org_id = ").push_bind(org_id);
        }
        if let Some(has_org) = has_platform_org {
            if has_org {
                qb.push(" AND organization_id IS NOT NULL");
            } else {
                qb.push(" AND organization_id IS NULL");
            }
        }
        qb.push(" ORDER BY name ASC LIMIT ")
            .push_bind(limit.unwrap_or(200));
        let rows = qb.build_query_as::<Self>().fetch_all(pool).await?;
        Ok(rows)
    }

    pub fn slugify(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    pub async fn create(pool: &SqlitePool, input: CreateCompany) -> Result<Self, CompanyError> {
        let id = DbUuid::new();
        let slug = input.slug.unwrap_or_else(|| Self::slugify(&input.name));

        sqlx::query(
            r#"INSERT INTO companies
               (id, name, slug, website, industry, description, logo_url, headquarters, created_by_org_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&slug)
        .bind(&input.website)
        .bind(&input.industry)
        .bind(&input.description)
        .bind(&input.logo_url)
        .bind(&input.headquarters)
        .bind(&input.created_by_org_id)
        .execute(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                CompanyError::SlugConflict
            } else {
                CompanyError::Database(e)
            }
        })?;

        // Use find_by_name instead of find_by_id to avoid BLOB/TEXT hex mismatch.
        // The name was just inserted and slug UNIQUE constraint prevents duplicates.
        Self::find_by_name(pool, &input.name)
            .await?
            .ok_or(CompanyError::NotFound)
    }

    /// Find or create a company by name. Used when provisioning orgs or
    /// linking persons to their company in the knowledge graph.
    pub async fn find_or_create(
        pool: &SqlitePool,
        name: &str,
        created_by_org_id: Option<DbUuid>,
        website: Option<String>,
    ) -> Result<Self, CompanyError> {
        // Prefer org-scoped lookup for deduplication when org is known
        if let Some(ref org_id) = created_by_org_id
            && let Some(existing) = Self::find_by_name_and_org(pool, name, org_id).await?
        {
            return Ok(existing);
        }

        // Fallback to global name lookup

        if let Some(existing) = Self::find_by_name(pool, name).await? {
            return Ok(existing);
        }

        // Deduplicate slug if needed
        let base_slug = Self::slugify(name);
        #[derive(sqlx::FromRow)]
        struct Count {
            count: i64,
        }
        let c: Count = sqlx::query_as("SELECT COUNT(*) as count FROM companies WHERE slug LIKE ?")
            .bind(format!("{base_slug}%"))
            .fetch_one(pool)
            .await?;
        let slug = if c.count == 0 {
            base_slug
        } else {
            format!("{}-{}", base_slug, c.count)
        };

        Self::create(
            pool,
            CreateCompany {
                name: name.to_string(),
                slug: Some(slug),
                website,
                industry: None,
                description: None,
                logo_url: None,
                headquarters: None,
                created_by_org_id,
            },
        )
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &DbUuid,
        input: UpdateCompany,
    ) -> Result<Option<Self>, CompanyError> {
        let mut qb =
            sqlx::QueryBuilder::new("UPDATE companies SET updated_at = datetime('now','subsec')");
        if let Some(v) = input.name {
            qb.push(", name = ").push_bind(v);
        }
        if let Some(v) = input.website {
            qb.push(", website = ").push_bind(v);
        }
        if let Some(v) = input.industry {
            qb.push(", industry = ").push_bind(v);
        }
        if let Some(v) = input.description {
            qb.push(", description = ").push_bind(v);
        }
        if let Some(v) = input.logo_url {
            qb.push(", logo_url = ").push_bind(v);
        }
        if let Some(v) = input.cover_image_url {
            qb.push(", cover_image_url = ").push_bind(v);
        }
        if let Some(v) = input.headquarters {
            qb.push(", headquarters = ").push_bind(v);
        }
        if let Some(v) = input.address {
            qb.push(", address = ").push_bind(v);
        }
        if let Some(v) = input.city {
            qb.push(", city = ").push_bind(v);
        }
        if let Some(v) = input.country {
            qb.push(", country = ").push_bind(v);
        }
        if let Some(v) = input.phone {
            qb.push(", phone = ").push_bind(v);
        }
        if let Some(v) = input.email {
            qb.push(", email = ").push_bind(v);
        }
        if let Some(v) = input.whatsapp {
            qb.push(", whatsapp = ").push_bind(v);
        }
        if let Some(v) = input.instagram_handle {
            qb.push(", instagram_handle = ").push_bind(v);
        }
        if let Some(v) = input.linkedin_url {
            qb.push(", linkedin_url = ").push_bind(v);
        }
        if let Some(v) = input.twitter_handle {
            qb.push(", twitter_handle = ").push_bind(v);
        }
        if let Some(v) = input.facebook_url {
            qb.push(", facebook_url = ").push_bind(v);
        }
        if let Some(v) = input.founded_year {
            qb.push(", founded_year = ").push_bind(v);
        }
        if let Some(v) = input.employee_count {
            qb.push(", employee_count = ").push_bind(v);
        }
        if let Some(v) = input.tags {
            qb.push(", tags = ").push_bind(v);
        }
        if let Some(v) = input.business_hours {
            qb.push(", business_hours = ").push_bind(v);
        }
        if let Some(v) = input.notes {
            qb.push(", notes = ").push_bind(v);
        }
        if let Some(v) = input.gmb_rating {
            qb.push(", gmb_rating = ").push_bind(v);
        }
        if let Some(v) = input.gmb_review_count {
            qb.push(", gmb_review_count = ").push_bind(v);
        }
        if let Some(v) = input.gmb_place_id {
            qb.push(", gmb_place_id = ").push_bind(v);
        }
        if let Some(v) = input.organization_id {
            qb.push(", organization_id = ").push_bind(v);
        }
        if let Some(v) = input.intelligence_summary {
            qb.push(", intelligence_summary = ").push_bind(v);
        }
        if let Some(v) = input.intelligence_status {
            qb.push(", intelligence_status = ").push_bind(v);
        }
        let hex_no_dashes = id.to_string().replace('-', "");
        qb.push(" WHERE lower(hex(id)) = lower(?)")
            .push_bind(hex_no_dashes);
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &DbUuid) -> Result<bool, CompanyError> {
        let hex_no_dashes = id.to_string().replace('-', "");
        let r = sqlx::query("DELETE FROM companies WHERE lower(hex(id)) = lower(?)")
            .bind(&hex_no_dashes)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Client not found")]
    NotFound,
    #[error("Client with this slug already exists in the organization")]
    SlugExists,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Client {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub crm_contact_id: Option<String>,
    pub is_active: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateClient {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub crm_contact_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateClient {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub crm_contact_id: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClientMember {
    pub id: String,
    pub client_id: String,
    pub user_id: String,
    pub role: String,
    pub granted_by: Option<String>,
    pub granted_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateClientMember {
    pub user_id: String,
    pub role: Option<String>,
}

impl Client {
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Client>(
            r#"SELECT id, organization_id, name, slug, description, logo_url, website,
                      crm_contact_id, is_active, created_at, updated_at, deleted_at, deleted_by
               FROM clients WHERE id = ? AND deleted_at IS NULL"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Client>(
            r#"SELECT id, organization_id, name, slug, description, logo_url, website,
                      crm_contact_id, is_active, created_at, updated_at, deleted_at, deleted_by
               FROM clients WHERE organization_id = ? AND deleted_at IS NULL
               ORDER BY name ASC"#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_slug(
        pool: &SqlitePool,
        organization_id: &str,
        slug: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Client>(
            r#"SELECT id, organization_id, name, slug, description, logo_url, website,
                      crm_contact_id, is_active, created_at, updated_at, deleted_at, deleted_by
               FROM clients WHERE organization_id = ? AND slug = ? AND deleted_at IS NULL"#,
        )
        .bind(organization_id)
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        id: &str,
        organization_id: &str,
        data: &CreateClient,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Client>(
            r#"INSERT INTO clients (id, organization_id, name, slug, description, logo_url, website, crm_contact_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, organization_id, name, slug, description, logo_url, website,
                         crm_contact_id, is_active, created_at, updated_at, deleted_at, deleted_by"#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(&data.name)
        .bind(&data.slug)
        .bind(&data.description)
        .bind(&data.logo_url)
        .bind(&data.website)
        .bind(&data.crm_contact_id)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        data: &UpdateClient,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_deref().unwrap_or(&existing.name);
        let slug = data.slug.as_deref().unwrap_or(&existing.slug);
        let description = data
            .description
            .as_deref()
            .or(existing.description.as_deref());
        let logo_url = data.logo_url.as_deref().or(existing.logo_url.as_deref());
        let website = data.website.as_deref().or(existing.website.as_deref());
        let crm_contact_id = data
            .crm_contact_id
            .as_deref()
            .or(existing.crm_contact_id.as_deref());
        let is_active = data.is_active.unwrap_or(existing.is_active);

        sqlx::query_as::<_, Client>(
            r#"UPDATE clients SET name = ?, slug = ?, description = ?, logo_url = ?,
                      website = ?, crm_contact_id = ?, is_active = ?,
                      updated_at = datetime('now')
               WHERE id = ?
               RETURNING id, organization_id, name, slug, description, logo_url, website,
                         crm_contact_id, is_active, created_at, updated_at, deleted_at, deleted_by"#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(logo_url)
        .bind(website)
        .bind(crm_contact_id)
        .bind(is_active)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn soft_delete(
        pool: &SqlitePool,
        id: &str,
        deleted_by: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE clients SET deleted_at = datetime('now'), deleted_by = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(deleted_by)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find clients accessible by a user: via org membership + direct client_members
    pub async fn find_accessible_by_user(
        pool: &SqlitePool,
        user_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Client>(
            r#"SELECT DISTINCT c.id, c.organization_id, c.name, c.slug, c.description,
                      c.logo_url, c.website, c.crm_contact_id, c.is_active,
                      c.created_at, c.updated_at, c.deleted_at, c.deleted_by
               FROM clients c
               WHERE c.deleted_at IS NULL AND (
                   c.organization_id IN (
                       SELECT om.organization_id FROM organization_members om WHERE om.user_id = ?
                   )
                   OR
                   c.id IN (
                       SELECT cm.client_id FROM client_members cm WHERE cm.user_id = ?
                   )
               )
               ORDER BY c.name ASC"#,
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    pub async fn add_member(
        pool: &SqlitePool,
        id: &str,
        client_id: &str,
        user_id: Uuid,
        role: &str,
        granted_by: Option<&str>,
    ) -> Result<ClientMember, sqlx::Error> {
        sqlx::query_as::<_, ClientMember>(
            r#"INSERT INTO client_members (id, client_id, user_id, role, granted_by)
               VALUES (?, ?, ?, ?, ?)
               RETURNING id, client_id, user_id, role, granted_by, granted_at"#,
        )
        .bind(id)
        .bind(client_id)
        .bind(user_id)
        .bind(role)
        .bind(granted_by)
        .fetch_one(pool)
        .await
    }

    pub async fn remove_member(
        pool: &SqlitePool,
        client_id: &str,
        user_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM client_members WHERE client_id = ? AND user_id = ?")
            .bind(client_id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_members(
        pool: &SqlitePool,
        client_id: &str,
    ) -> Result<Vec<ClientMember>, sqlx::Error> {
        sqlx::query_as::<_, ClientMember>(
            r#"SELECT id, client_id, user_id, role, granted_by, granted_at
               FROM client_members WHERE client_id = ?
               ORDER BY granted_at ASC"#,
        )
        .bind(client_id)
        .fetch_all(pool)
        .await
    }
}

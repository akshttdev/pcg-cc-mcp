use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

use crate::db_uuid::DbUuid;

// ---------------------------------------------------------------------------
// Contact ↔ Company (many-to-many)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ContactCompanyRole {
    pub id: Uuid,
    pub crm_contact_id: Option<String>,
    pub company_id: Uuid,
    /// 'employee' | 'founder' | 'advisor' | 'consultant' | 'board' | 'investor' | 'contact'
    pub role: String,
    pub title: Option<String>,
    pub is_primary: i32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    // Joined fields
    pub company_name: Option<String>,
    pub company_slug: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertContactCompanyRole {
    pub company_id: Uuid,
    pub role: Option<String>,
    pub title: Option<String>,
    pub is_primary: Option<bool>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct PatchContactCompanyRole {
    pub role: Option<String>,
    pub title: Option<String>,
    pub is_primary: Option<bool>,
}

impl ContactCompanyRole {
    pub async fn list_for_contact(pool: &SqlitePool, contact_id: &str) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                ccr.id, ccr.crm_contact_id, ccr.company_id, ccr.role, ccr.title,
                ccr.is_primary, ccr.start_date, ccr.end_date, ccr.notes, ccr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM contact_company_roles ccr
               LEFT JOIN companies c ON c.id = ccr.company_id
               WHERE ccr.crm_contact_id = ?1
               ORDER BY ccr.is_primary DESC, ccr.created_at ASC"#,
        )
        .bind(contact_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_for_company(pool: &SqlitePool, company_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                ccr.id, ccr.crm_contact_id, ccr.company_id, ccr.role, ccr.title,
                ccr.is_primary, ccr.start_date, ccr.end_date, ccr.notes, ccr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM contact_company_roles ccr
               LEFT JOIN companies c ON c.id = ccr.company_id
               WHERE ccr.company_id = ?1
               ORDER BY ccr.is_primary DESC, ccr.created_at ASC"#,
        )
        .bind(company_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        contact_id: &str,
        data: UpsertContactCompanyRole,
    ) -> sqlx::Result<Self> {
        let id = DbUuid::new();
        let role = data.role.unwrap_or_else(|| "contact".into());
        let is_primary = data.is_primary.unwrap_or(false) as i32;

        sqlx::query(
            r#"INSERT INTO contact_company_roles
               (id, crm_contact_id, company_id, role, title, is_primary, start_date, end_date, notes)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
               ON CONFLICT(crm_contact_id, company_id) DO UPDATE SET
                 role=excluded.role,
                 title=excluded.title,
                 is_primary=excluded.is_primary,
                 start_date=excluded.start_date,
                 end_date=excluded.end_date,
                 notes=excluded.notes"#,
        )
        .bind(id.to_string())
        .bind(contact_id)
        .bind(data.company_id.to_string())
        .bind(&role)
        .bind(&data.title)
        .bind(is_primary)
        .bind(&data.start_date)
        .bind(&data.end_date)
        .bind(&data.notes)
        .execute(pool)
        .await?;

        sqlx::query_as(
            r#"SELECT
                ccr.id, ccr.crm_contact_id, ccr.company_id, ccr.role, ccr.title,
                ccr.is_primary, ccr.start_date, ccr.end_date, ccr.notes, ccr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM contact_company_roles ccr
               LEFT JOIN companies c ON c.id = ccr.company_id
               WHERE ccr.crm_contact_id = ?1 AND ccr.company_id = ?2"#,
        )
        .bind(contact_id)
        .bind(data.company_id.to_string())
        .fetch_one(pool)
        .await
    }

    pub async fn patch(
        pool: &SqlitePool,
        contact_id: &str,
        company_id: &str,
        data: PatchContactCompanyRole,
    ) -> sqlx::Result<Option<Self>> {
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM contact_company_roles WHERE crm_contact_id = ?1 AND company_id = ?2",
        )
        .bind(contact_id)
        .bind(company_id)
        .fetch_optional(pool)
        .await?;

        if exists.is_none() {
            return Ok(None);
        }

        if let Some(role) = &data.role {
            sqlx::query(
                "UPDATE contact_company_roles SET role = ?1 WHERE crm_contact_id = ?2 AND company_id = ?3",
            )
            .bind(role)
            .bind(contact_id)
            .bind(company_id)
            .execute(pool)
            .await?;
        }
        if let Some(title) = &data.title {
            sqlx::query(
                "UPDATE contact_company_roles SET title = ?1 WHERE crm_contact_id = ?2 AND company_id = ?3",
            )
            .bind(title)
            .bind(contact_id)
            .bind(company_id)
            .execute(pool)
            .await?;
        }
        if let Some(primary) = data.is_primary {
            sqlx::query(
                "UPDATE contact_company_roles SET is_primary = ?1 WHERE crm_contact_id = ?2 AND company_id = ?3",
            )
            .bind(primary as i32)
            .bind(contact_id)
            .bind(company_id)
            .execute(pool)
            .await?;
        }

        sqlx::query_as(
            r#"SELECT
                ccr.id, ccr.crm_contact_id, ccr.company_id, ccr.role, ccr.title,
                ccr.is_primary, ccr.start_date, ccr.end_date, ccr.notes, ccr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM contact_company_roles ccr
               LEFT JOIN companies c ON c.id = ccr.company_id
               WHERE ccr.crm_contact_id = ?1 AND ccr.company_id = ?2"#,
        )
        .bind(contact_id)
        .bind(company_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(
        pool: &SqlitePool,
        contact_id: &str,
        company_id: &str,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM contact_company_roles WHERE crm_contact_id = ?1 AND company_id = ?2",
        )
        .bind(contact_id)
        .bind(company_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ---------------------------------------------------------------------------
// Contact ↔ Organization (many-to-many)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ContactOrgLink {
    pub id: Uuid,
    pub crm_contact_id: Option<String>,
    pub organization_id: Uuid,
    /// 'client' | 'vendor' | 'partner' | 'prospect' | 'contact'
    pub context: String,
    pub notes: Option<String>,
    pub added_at: String,
    // Joined fields
    pub org_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertContactOrgLink {
    pub organization_id: Uuid,
    pub context: Option<String>,
    pub notes: Option<String>,
}

impl ContactOrgLink {
    pub async fn list_for_contact(pool: &SqlitePool, contact_id: &str) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                col.id, col.crm_contact_id, col.organization_id, col.context, col.notes, col.added_at,
                o.name AS org_name
               FROM contact_organization_links col
               LEFT JOIN organizations o ON o.id = col.organization_id
               WHERE col.crm_contact_id = ?1
               ORDER BY col.added_at ASC"#,
        )
        .bind(contact_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_for_org(pool: &SqlitePool, org_id: &str) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                col.id, col.crm_contact_id, col.organization_id, col.context, col.notes, col.added_at,
                o.name AS org_name
               FROM contact_organization_links col
               LEFT JOIN organizations o ON o.id = col.organization_id
               WHERE col.organization_id = ?1
               ORDER BY col.added_at ASC"#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        contact_id: &str,
        data: UpsertContactOrgLink,
    ) -> sqlx::Result<Self> {
        let id = DbUuid::new();
        let context = data.context.unwrap_or_else(|| "contact".into());

        sqlx::query(
            r#"INSERT INTO contact_organization_links
               (id, crm_contact_id, organization_id, context, notes)
               VALUES (?1, ?2, ?3, ?4, ?5)
               ON CONFLICT(crm_contact_id, organization_id) DO UPDATE SET
                 context=excluded.context,
                 notes=excluded.notes"#,
        )
        .bind(id.to_string())
        .bind(contact_id)
        .bind(data.organization_id.to_string())
        .bind(&context)
        .bind(&data.notes)
        .execute(pool)
        .await?;

        sqlx::query_as(
            r#"SELECT
                col.id, col.crm_contact_id, col.organization_id, col.context, col.notes, col.added_at,
                o.name AS org_name
               FROM contact_organization_links col
               LEFT JOIN organizations o ON o.id = col.organization_id
               WHERE col.crm_contact_id = ?1 AND col.organization_id = ?2"#,
        )
        .bind(contact_id)
        .bind(data.organization_id.to_string())
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, contact_id: &str, org_id: &str) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM contact_organization_links WHERE crm_contact_id = ?1 AND organization_id = ?2",
        )
        .bind(contact_id)
        .bind(org_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ---------------------------------------------------------------------------
// Company contact methods (unchanged — already company-scoped)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CompanyContactMethod {
    pub id: Uuid,
    pub company_id: Uuid,
    /// 'email' | 'phone' | 'whatsapp' | 'linkedin' | 'instagram' | 'twitter' | 'website'
    pub method_type: String,
    pub label: Option<String>,
    pub value: String,
    pub is_primary: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCompanyContactMethod {
    pub method_type: String,
    pub label: Option<String>,
    pub value: String,
    pub is_primary: Option<bool>,
}

impl CompanyContactMethod {
    pub async fn list_for_company(pool: &SqlitePool, company_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT id, company_id, method_type, label, value, is_primary, created_at
               FROM company_contact_methods
               WHERE company_id = ?1
               ORDER BY is_primary DESC, created_at ASC"#,
        )
        .bind(company_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        company_id: Uuid,
        data: CreateCompanyContactMethod,
    ) -> sqlx::Result<Self> {
        let id = DbUuid::new();
        let is_primary = data.is_primary.unwrap_or(false) as i32;

        sqlx::query(
            r#"INSERT INTO company_contact_methods (id, company_id, method_type, label, value, is_primary)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        )
        .bind(id.to_string())
        .bind(company_id.to_string())
        .bind(&data.method_type)
        .bind(&data.label)
        .bind(&data.value)
        .bind(is_primary)
        .execute(pool)
        .await?;

        sqlx::query_as(
            "SELECT id, company_id, method_type, label, value, is_primary, created_at FROM company_contact_methods WHERE id = ?1",
        )
        .bind(id.to_string())
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM company_contact_methods WHERE id = ?1")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

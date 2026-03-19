use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Person ↔ Company (many-to-many)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonCompanyRole {
    pub id: Uuid,
    pub person_id: Uuid,
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
pub struct UpsertPersonCompanyRole {
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
pub struct PatchPersonCompanyRole {
    pub role: Option<String>,
    pub title: Option<String>,
    pub is_primary: Option<bool>,
}

impl PersonCompanyRole {
    pub async fn list_for_person(pool: &SqlitePool, person_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                pcr.id, pcr.person_id, pcr.company_id, pcr.role, pcr.title,
                pcr.is_primary, pcr.start_date, pcr.end_date, pcr.notes, pcr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM person_company_roles pcr
               LEFT JOIN companies c ON c.id = pcr.company_id
               WHERE pcr.person_id = ?1
               ORDER BY pcr.is_primary DESC, pcr.created_at ASC"#,
        )
        .bind(person_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn list_for_company(pool: &SqlitePool, company_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                pcr.id, pcr.person_id, pcr.company_id, pcr.role, pcr.title,
                pcr.is_primary, pcr.start_date, pcr.end_date, pcr.notes, pcr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM person_company_roles pcr
               LEFT JOIN companies c ON c.id = pcr.company_id
               WHERE pcr.company_id = ?1
               ORDER BY pcr.is_primary DESC, pcr.created_at ASC"#,
        )
        .bind(company_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        person_id: Uuid,
        data: UpsertPersonCompanyRole,
    ) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let role = data.role.unwrap_or_else(|| "contact".into());
        let is_primary = data.is_primary.unwrap_or(false) as i32;

        sqlx::query(
            r#"INSERT INTO person_company_roles
               (id, person_id, company_id, role, title, is_primary, start_date, end_date, notes)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
               ON CONFLICT(person_id, company_id) DO UPDATE SET
                 role=excluded.role,
                 title=excluded.title,
                 is_primary=excluded.is_primary,
                 start_date=excluded.start_date,
                 end_date=excluded.end_date,
                 notes=excluded.notes"#,
        )
        .bind(id.to_string())
        .bind(person_id.to_string())
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
                pcr.id, pcr.person_id, pcr.company_id, pcr.role, pcr.title,
                pcr.is_primary, pcr.start_date, pcr.end_date, pcr.notes, pcr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM person_company_roles pcr
               LEFT JOIN companies c ON c.id = pcr.company_id
               WHERE pcr.person_id = ?1 AND pcr.company_id = ?2"#,
        )
        .bind(person_id.to_string())
        .bind(data.company_id.to_string())
        .fetch_one(pool)
        .await
    }

    pub async fn patch(
        pool: &SqlitePool,
        person_id: Uuid,
        company_id: Uuid,
        data: PatchPersonCompanyRole,
    ) -> sqlx::Result<Option<Self>> {
        // Check exists
        let exists: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT id FROM person_company_roles WHERE person_id = ?1 AND company_id = ?2",
        )
        .bind(person_id.to_string())
        .bind(company_id.to_string())
        .fetch_optional(pool)
        .await?;

        if exists.is_none() {
            return Ok(None);
        }

        if let Some(role) = &data.role {
            sqlx::query(
                "UPDATE person_company_roles SET role = ?1 WHERE person_id = ?2 AND company_id = ?3",
            )
            .bind(role)
            .bind(person_id.to_string())
            .bind(company_id.to_string())
            .execute(pool)
            .await?;
        }
        if let Some(title) = &data.title {
            sqlx::query(
                "UPDATE person_company_roles SET title = ?1 WHERE person_id = ?2 AND company_id = ?3",
            )
            .bind(title)
            .bind(person_id.to_string())
            .bind(company_id.to_string())
            .execute(pool)
            .await?;
        }
        if let Some(primary) = data.is_primary {
            sqlx::query(
                "UPDATE person_company_roles SET is_primary = ?1 WHERE person_id = ?2 AND company_id = ?3",
            )
            .bind(primary as i32)
            .bind(person_id.to_string())
            .bind(company_id.to_string())
            .execute(pool)
            .await?;
        }

        let row = sqlx::query_as(
            r#"SELECT
                pcr.id, pcr.person_id, pcr.company_id, pcr.role, pcr.title,
                pcr.is_primary, pcr.start_date, pcr.end_date, pcr.notes, pcr.created_at,
                c.name AS company_name, c.slug AS company_slug
               FROM person_company_roles pcr
               LEFT JOIN companies c ON c.id = pcr.company_id
               WHERE pcr.person_id = ?1 AND pcr.company_id = ?2"#,
        )
        .bind(person_id.to_string())
        .bind(company_id.to_string())
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn delete(
        pool: &SqlitePool,
        person_id: Uuid,
        company_id: Uuid,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM person_company_roles WHERE person_id = ?1 AND company_id = ?2",
        )
        .bind(person_id.to_string())
        .bind(company_id.to_string())
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ---------------------------------------------------------------------------
// Person ↔ Organization (many-to-many)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonOrgContact {
    pub id: Uuid,
    pub person_id: Uuid,
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
pub struct UpsertPersonOrgContact {
    pub organization_id: Uuid,
    pub context: Option<String>,
    pub notes: Option<String>,
}

impl PersonOrgContact {
    pub async fn list_for_person(pool: &SqlitePool, person_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                poc.id, poc.person_id, poc.organization_id, poc.context, poc.notes, poc.added_at,
                o.name AS org_name
               FROM person_organization_contacts poc
               LEFT JOIN organizations o ON o.id = poc.organization_id
               WHERE poc.person_id = ?1
               ORDER BY poc.added_at ASC"#,
        )
        .bind(person_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn list_for_org(pool: &SqlitePool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"SELECT
                poc.id, poc.person_id, poc.organization_id, poc.context, poc.notes, poc.added_at,
                o.name AS org_name
               FROM person_organization_contacts poc
               LEFT JOIN organizations o ON o.id = poc.organization_id
               WHERE poc.organization_id = ?1
               ORDER BY poc.added_at ASC"#,
        )
        .bind(org_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        person_id: Uuid,
        data: UpsertPersonOrgContact,
    ) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let context = data.context.unwrap_or_else(|| "contact".into());

        sqlx::query(
            r#"INSERT INTO person_organization_contacts
               (id, person_id, organization_id, context, notes)
               VALUES (?1, ?2, ?3, ?4, ?5)
               ON CONFLICT(person_id, organization_id) DO UPDATE SET
                 context=excluded.context,
                 notes=excluded.notes"#,
        )
        .bind(id.to_string())
        .bind(person_id.to_string())
        .bind(data.organization_id.to_string())
        .bind(&context)
        .bind(&data.notes)
        .execute(pool)
        .await?;

        sqlx::query_as(
            r#"SELECT
                poc.id, poc.person_id, poc.organization_id, poc.context, poc.notes, poc.added_at,
                o.name AS org_name
               FROM person_organization_contacts poc
               LEFT JOIN organizations o ON o.id = poc.organization_id
               WHERE poc.person_id = ?1 AND poc.organization_id = ?2"#,
        )
        .bind(person_id.to_string())
        .bind(data.organization_id.to_string())
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, person_id: Uuid, org_id: Uuid) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM person_organization_contacts WHERE person_id = ?1 AND organization_id = ?2",
        )
        .bind(person_id.to_string())
        .bind(org_id.to_string())
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ---------------------------------------------------------------------------
// Company contact methods
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
        let id = Uuid::new_v4();
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

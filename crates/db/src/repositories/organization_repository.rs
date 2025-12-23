// Organization database repository
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{Organization, OrganizationRole, User};

pub struct OrganizationRepository;

impl OrganizationRepository {
    /// Create a new organization
    pub async fn create(
        pool: &PgPool,
        name: &str,
        slug: &str,
        created_by: Uuid,
    ) -> Result<Organization, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (name, slug, created_by)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    /// Find organization by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Organization>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE id = $1 AND is_active = true",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Find organization by slug
    pub async fn find_by_slug(
        pool: &PgPool,
        slug: &str,
    ) -> Result<Option<Organization>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE slug = $1 AND is_active = true",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    /// List all organizations
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Organization>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE is_active = true ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    /// Update organization
    pub async fn update(
        pool: &PgPool,
        org_id: Uuid,
        name: Option<String>,
        slug: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Organization, sqlx::Error> {
        let mut query = String::from("UPDATE organizations SET updated_at = NOW()");
        let mut bind_count = 1;

        if name.is_some() {
            query.push_str(&format!(", name = ${}", bind_count));
            bind_count += 1;
        }
        if slug.is_some() {
            query.push_str(&format!(", slug = ${}", bind_count));
            bind_count += 1;
        }
        if is_active.is_some() {
            query.push_str(&format!(", is_active = ${}", bind_count));
            bind_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", bind_count));

        let mut q = sqlx::query_as::<_, Organization>(&query);

        if let Some(n) = name {
            q = q.bind(n);
        }
        if let Some(s) = slug {
            q = q.bind(s);
        }
        if let Some(ia) = is_active {
            q = q.bind(ia);
        }
        q = q.bind(org_id);

        q.fetch_one(pool).await
    }

    /// Delete organization
    pub async fn delete(pool: &PgPool, org_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get organization members
    pub async fn get_members(
        pool: &PgPool,
        org_id: Uuid,
    ) -> Result<Vec<(User, OrganizationRole)>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct MemberRow {
            #[sqlx(flatten)]
            user: User,
            role: String,
        }

        let rows = sqlx::query_as::<_, MemberRow>(
            r#"
            SELECT u.id, u.username, u.email, u.password_hash, u.full_name, u.avatar_url,
                   u.is_active, u.is_admin, u.last_login_at, u.created_at, u.updated_at,
                   om.role
            FROM users u
            JOIN organization_members om ON u.id = om.user_id
            WHERE om.organization_id = $1 AND u.is_active = true
            ORDER BY om.joined_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let role = row.role.parse().unwrap_or(OrganizationRole::Member);
                (row.user, role)
            })
            .collect())
    }

    /// Remove user from organization
    pub async fn remove_member(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM organization_members WHERE organization_id = $1 AND user_id = $2")
            .bind(org_id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Update member role
    pub async fn update_member_role(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        role: OrganizationRole,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE organization_members SET role = $1 WHERE organization_id = $2 AND user_id = $3",
        )
        .bind(role.to_string())
        .bind(org_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check if user is member of organization
    pub async fn is_member(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM organization_members WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(count.0 > 0)
    }

    /// Get user's role in organization
    pub async fn get_user_role(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<OrganizationRole>, sqlx::Error> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM organization_members WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|(role,)| role.parse().unwrap_or(OrganizationRole::Member)))
    }
}

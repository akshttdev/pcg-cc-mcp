// User database repository
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{Organization, OrganizationRole, User, UserOrganization, UserProfile};

pub struct UserRepository;

impl UserRepository {
    /// Find user by username
    pub async fn find_by_username(
        pool: &PgPool,
        username: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1 AND is_active = true")
            .bind(username)
            .fetch_optional(pool)
            .await
    }

    /// Find user by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Get user with their organizations
    pub async fn get_user_profile(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Option<UserProfile>, sqlx::Error> {
        let user = match Self::find_by_id(pool, user_id).await? {
            Some(u) => u,
            None => return Ok(None),
        };

        #[derive(sqlx::FromRow)]
        struct OrgRow {
            id: Uuid,
            name: String,
            slug: String,
            role: String,
        }

        let org_rows = sqlx::query_as::<_, OrgRow>(
            r#"
            SELECT o.id, o.name, o.slug, om.role
            FROM organizations o
            JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1 AND o.is_active = true
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let organizations = org_rows
            .into_iter()
            .map(|row| UserOrganization {
                id: row.id,
                name: row.name,
                slug: row.slug,
                role: row.role.parse().unwrap_or(OrganizationRole::Member),
            })
            .collect();

        Ok(Some(UserProfile {
            id: user.id,
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            avatar_url: user.avatar_url,
            is_admin: user.is_admin,
            organizations,
        }))
    }

    /// Create a new user
    pub async fn create(
        pool: &PgPool,
        username: &str,
        email: &str,
        password_hash: &str,
        full_name: &str,
        is_admin: bool,
        created_by: Option<Uuid>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, full_name, is_admin, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(full_name)
        .bind(is_admin)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    /// List all users (admin only)
    pub async fn list_all(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    /// Update user
    pub async fn update(
        pool: &PgPool,
        user_id: Uuid,
        email: Option<String>,
        full_name: Option<String>,
        is_active: Option<bool>,
        is_admin: Option<bool>,
    ) -> Result<User, sqlx::Error> {
        let mut query = String::from("UPDATE users SET updated_at = NOW()");
        let mut bind_count = 1;

        if email.is_some() {
            query.push_str(&format!(", email = ${}", bind_count));
            bind_count += 1;
        }
        if full_name.is_some() {
            query.push_str(&format!(", full_name = ${}", bind_count));
            bind_count += 1;
        }
        if is_active.is_some() {
            query.push_str(&format!(", is_active = ${}", bind_count));
            bind_count += 1;
        }
        if is_admin.is_some() {
            query.push_str(&format!(", is_admin = ${}", bind_count));
            bind_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", bind_count));

        let mut q = sqlx::query_as::<_, User>(&query);

        if let Some(e) = email {
            q = q.bind(e);
        }
        if let Some(fn_) = full_name {
            q = q.bind(fn_);
        }
        if let Some(ia) = is_active {
            q = q.bind(ia);
        }
        if let Some(iadm) = is_admin {
            q = q.bind(iadm);
        }
        q = q.bind(user_id);

        q.fetch_one(pool).await
    }

    /// Update last login time
    pub async fn update_last_login(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Change password
    pub async fn change_password(
        pool: &PgPool,
        user_id: Uuid,
        new_password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_password_hash)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete user
    pub async fn delete(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Add user to organization
    pub async fn add_to_organization(
        pool: &PgPool,
        user_id: Uuid,
        org_id: Uuid,
        role: OrganizationRole,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO organization_members (user_id, organization_id, role) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(org_id)
        .bind(role.to_string())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get user's organizations
    pub async fn get_user_organizations(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Organization>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            r#"
            SELECT o.*
            FROM organizations o
            JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1 AND o.is_active = true
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
}

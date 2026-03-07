// Simplified User and authentication models
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub wallet_address: Option<String>,
    pub home_project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub owner_id: Uuid,
    pub settings: serde_json::Value,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrganizationRole {
    Admin,
    Member,
    Viewer,
}

impl std::fmt::Display for OrganizationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationRole::Admin => write!(f, "admin"),
            OrganizationRole::Member => write!(f, "member"),
            OrganizationRole::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for OrganizationRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(OrganizationRole::Admin),
            "member" => Ok(OrganizationRole::Member),
            "viewer" => Ok(OrganizationRole::Viewer),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrgMemberUser {
    pub username: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub user: Option<OrgMemberUser>,
}

// Auth DTOs
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub session_id: String,
}

// Session model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct Session {
    pub id: String,
    pub user_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
    pub organizations: Vec<UserOrganization>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserOrganization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: OrganizationRole,
}

// Admin DTOs
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub is_active: Option<bool>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateOrganization {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateOrganization {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
}

impl User {
    pub fn to_profile(&self) -> UserProfile {
        UserProfile {
            id: self.id,
            username: self.username.clone(),
            email: self.email.clone(),
            full_name: self.full_name.clone(),
            avatar_url: self.avatar_url.clone(),
            is_admin: self.is_admin,
            organizations: vec![],
        }
    }

    pub async fn set_wallet_address(
        pool: &sqlx::SqlitePool,
        user_id: Uuid,
        wallet_address: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET wallet_address = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(wallet_address)
        .bind(user_id.as_bytes().as_slice())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_home_project(
        pool: &sqlx::SqlitePool,
        user_id: Uuid,
        project_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET home_project_id = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(project_id.as_bytes().as_slice())
        .bind(user_id.as_bytes().as_slice())
        .execute(pool)
        .await?;
        Ok(())
    }
}

impl Organization {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at FROM organizations WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_slug(pool: &SqlitePool, slug: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at FROM organizations WHERE slug = ?"
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at FROM organizations ORDER BY name ASC"
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_all_active(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            "SELECT id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at FROM organizations WHERE is_active = 1 ORDER BY name ASC"
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_user(pool: &SqlitePool, user_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            r#"SELECT o.id, o.name, o.slug, o.description, o.avatar_url, o.owner_id, o.settings, o.is_active, o.created_at, o.updated_at
               FROM organizations o
               INNER JOIN organization_members om ON om.organization_id = o.id
               WHERE om.user_id = ? AND o.is_active = 1
               ORDER BY o.name ASC"#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        id: Uuid,
        owner_id: Uuid,
        data: &CreateOrganization,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Organization>(
            r#"INSERT INTO organizations (id, name, slug, description, avatar_url, owner_id)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at"#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.slug)
        .bind(&data.description)
        .bind(&data.avatar_url)
        .bind(owner_id)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateOrganization,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_deref().unwrap_or(&existing.name);
        let slug = data.slug.as_deref().unwrap_or(&existing.slug);
        let description = data.description.as_deref().or(existing.description.as_deref());
        let avatar_url = data.avatar_url.as_deref().or(existing.avatar_url.as_deref());

        sqlx::query_as::<_, Organization>(
            r#"UPDATE organizations SET name = ?, slug = ?, description = ?, avatar_url = ?, updated_at = datetime('now')
               WHERE id = ?
               RETURNING id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at"#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(avatar_url)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn deactivate(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE organizations SET is_active = 0, updated_at = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn activate(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE organizations SET is_active = 1, updated_at = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn add_member(
        pool: &SqlitePool,
        org_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<OrganizationMember, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO organization_members (id, organization_id, user_id, role)
               VALUES (?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await?;

        // Return member with user info via get_members helper
        let members = Organization::get_members(pool, org_id).await?;
        members
            .into_iter()
            .find(|m| m.user_id == user_id)
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn remove_member(
        pool: &SqlitePool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM organization_members WHERE organization_id = ? AND user_id = ?")
            .bind(org_id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_members(
        pool: &SqlitePool,
        org_id: Uuid,
    ) -> Result<Vec<OrganizationMember>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct MemberRow {
            id: Uuid,
            organization_id: Uuid,
            user_id: Uuid,
            role: String,
            joined_at: chrono::DateTime<chrono::Utc>,
            username: Option<String>,
            full_name: Option<String>,
            email: Option<String>,
            avatar_url: Option<String>,
        }

        let rows = sqlx::query_as::<_, MemberRow>(
            r#"SELECT om.id, om.organization_id, om.user_id, om.role, om.joined_at,
                      u.username, u.full_name, u.email, u.avatar_url
               FROM organization_members om
               LEFT JOIN users u ON u.id = om.user_id
               WHERE om.organization_id = ?
               ORDER BY om.joined_at ASC"#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| OrganizationMember {
            id: r.id,
            organization_id: r.organization_id,
            user_id: r.user_id,
            role: r.role,
            joined_at: r.joined_at,
            user: r.username.map(|un| OrgMemberUser {
                username: un,
                full_name: r.full_name,
                email: r.email,
                avatar_url: r.avatar_url,
            }),
        }).collect())
    }

    pub async fn get_user_role(
        pool: &SqlitePool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct RoleRow {
            role: String,
        }
        let result: Option<RoleRow> = sqlx::query_as(
            "SELECT role FROM organization_members WHERE organization_id = ? AND user_id = ?"
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(result.map(|r| r.role))
    }
}

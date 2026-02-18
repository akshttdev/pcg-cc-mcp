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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
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

    pub async fn add_member(
        pool: &SqlitePool,
        org_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<OrganizationMember, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, OrganizationMember>(
            r#"INSERT INTO organization_members (id, organization_id, user_id, role)
               VALUES (?, ?, ?, ?)
               RETURNING id, organization_id, user_id, role, joined_at"#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(pool)
        .await
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
        sqlx::query_as::<_, OrganizationMember>(
            r#"SELECT id, organization_id, user_id, role, joined_at
               FROM organization_members WHERE organization_id = ?
               ORDER BY joined_at ASC"#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
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

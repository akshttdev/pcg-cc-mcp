use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::db_uuid::DbUuid;

/// Thin bridge between platform users and CRM contacts.
/// Platform users (team members) get a user_profile that optionally
/// links to their CRM contact record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: DbUuid,
    pub user_id: String,
    pub crm_contact_id: Option<String>,
    pub display_name: Option<String>,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

impl UserProfile {
    pub async fn find_by_user(pool: &SqlitePool, user_id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM user_profiles WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_contact(
        pool: &SqlitePool,
        crm_contact_id: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM user_profiles WHERE crm_contact_id = ?")
            .bind(crm_contact_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        user_id: &str,
        crm_contact_id: Option<&str>,
        display_name: Option<&str>,
    ) -> sqlx::Result<Self> {
        let id = DbUuid::new();
        sqlx::query(
            "INSERT INTO user_profiles (id, user_id, crm_contact_id, display_name)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
               crm_contact_id = COALESCE(excluded.crm_contact_id, user_profiles.crm_contact_id),
               display_name = COALESCE(excluded.display_name, user_profiles.display_name),
               updated_at = datetime('now','subsec')",
        )
        .bind(&id)
        .bind(user_id)
        .bind(crm_contact_id)
        .bind(display_name)
        .execute(pool)
        .await?;

        Self::find_by_user(pool, user_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }
}

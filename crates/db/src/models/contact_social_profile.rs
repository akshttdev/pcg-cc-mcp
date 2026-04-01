use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::db_uuid::DbUuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContactSocialProfile {
    pub id: DbUuid,
    pub crm_contact_id: DbUuid,
    pub platform: String,
    pub handle: Option<String>,
    pub profile_url: Option<String>,
    pub follower_count: Option<i64>,
    pub following_count: Option<i64>,
    pub bio: Option<String>,
    pub verified: i32,
    pub raw_data: Option<String>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpsertContactSocialProfile {
    pub platform: String,
    pub handle: Option<String>,
    pub profile_url: Option<String>,
    pub follower_count: Option<i64>,
    pub following_count: Option<i64>,
    pub bio: Option<String>,
    pub verified: Option<bool>,
    pub raw_data: Option<serde_json::Value>,
}

impl ContactSocialProfile {
    pub async fn list_for_contact(
        pool: &SqlitePool,
        crm_contact_id: &DbUuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM contact_social_profiles WHERE crm_contact_id = ? ORDER BY platform",
        )
        .bind(crm_contact_id)
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        crm_contact_id: &DbUuid,
        data: UpsertContactSocialProfile,
    ) -> sqlx::Result<Self> {
        let id = DbUuid::new();
        let raw_data = data.raw_data.map(|v| v.to_string());
        let verified = data.verified.unwrap_or(false) as i32;

        sqlx::query(
            r#"INSERT INTO contact_social_profiles
               (id, crm_contact_id, platform, handle, profile_url, follower_count,
                following_count, bio, verified, raw_data)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
               ON CONFLICT(crm_contact_id, platform) DO UPDATE SET
                handle = COALESCE(?4, handle),
                profile_url = COALESCE(?5, profile_url),
                follower_count = COALESCE(?6, follower_count),
                following_count = COALESCE(?7, following_count),
                bio = COALESCE(?8, bio),
                verified = ?9,
                raw_data = COALESCE(?10, raw_data),
                updated_at = datetime('now','subsec')"#,
        )
        .bind(&id)
        .bind(crm_contact_id)
        .bind(&data.platform)
        .bind(&data.handle)
        .bind(&data.profile_url)
        .bind(data.follower_count)
        .bind(data.following_count)
        .bind(&data.bio)
        .bind(verified)
        .bind(&raw_data)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, Self>(
            "SELECT * FROM contact_social_profiles WHERE crm_contact_id = ?1 AND platform = ?2",
        )
        .bind(crm_contact_id)
        .bind(&data.platform)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(
        pool: &SqlitePool,
        crm_contact_id: &DbUuid,
        platform: &str,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM contact_social_profiles WHERE crm_contact_id = ?1 AND platform = ?2",
        )
        .bind(crm_contact_id)
        .bind(platform)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

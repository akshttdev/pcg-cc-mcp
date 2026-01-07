use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CmsSiteSettingError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Setting not found")]
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CmsSiteSetting {
    pub id: Uuid,
    pub site_id: Uuid,
    pub setting_key: String,
    pub setting_value: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct SetCmsSiteSetting {
    pub key: String,
    pub value: String,
}

impl CmsSiteSetting {
    pub async fn find_by_site(pool: &SqlitePool, site_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsSiteSetting,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                setting_key,
                setting_value,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_site_settings
            WHERE site_id = $1
            ORDER BY setting_key ASC"#,
            site_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_key(
        pool: &SqlitePool,
        site_id: Uuid,
        key: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsSiteSetting,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                setting_key,
                setting_value,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_site_settings
            WHERE site_id = $1 AND setting_key = $2"#,
            site_id,
            key
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_value(
        pool: &SqlitePool,
        site_id: Uuid,
        key: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let setting = Self::find_by_key(pool, site_id, key).await?;
        Ok(setting.map(|s| s.setting_value))
    }

    pub async fn set(
        pool: &SqlitePool,
        site_id: Uuid,
        key: &str,
        value: &str,
    ) -> Result<Self, sqlx::Error> {
        // Try to find existing setting
        if let Some(existing) = Self::find_by_key(pool, site_id, key).await? {
            // Update existing
            sqlx::query_as!(
                CmsSiteSetting,
                r#"UPDATE cms_site_settings SET
                    setting_value = $2,
                    updated_at = datetime('now', 'subsec')
                WHERE id = $1
                RETURNING
                    id as "id!: Uuid",
                    site_id as "site_id!: Uuid",
                    setting_key,
                    setting_value,
                    created_at as "created_at!: DateTime<Utc>",
                    updated_at as "updated_at!: DateTime<Utc>""#,
                existing.id,
                value
            )
            .fetch_one(pool)
            .await
        } else {
            // Create new
            let id = Uuid::new_v4();
            sqlx::query_as!(
                CmsSiteSetting,
                r#"INSERT INTO cms_site_settings (id, site_id, setting_key, setting_value)
                VALUES ($1, $2, $3, $4)
                RETURNING
                    id as "id!: Uuid",
                    site_id as "site_id!: Uuid",
                    setting_key,
                    setting_value,
                    created_at as "created_at!: DateTime<Utc>",
                    updated_at as "updated_at!: DateTime<Utc>""#,
                id,
                site_id,
                key,
                value
            )
            .fetch_one(pool)
            .await
        }
    }

    pub async fn delete(pool: &SqlitePool, site_id: Uuid, key: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM cms_site_settings WHERE site_id = $1 AND setting_key = $2",
            site_id,
            key
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

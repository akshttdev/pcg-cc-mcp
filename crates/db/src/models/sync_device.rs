//! Sync Devices — registered devices per user for file sync.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub platform: Option<String>,
    pub sync_folder: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub sync_error: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct RegisterDevice {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub device_name: String,
    pub device_type: Option<String>,
    pub platform: Option<String>,
    pub sync_folder: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateDeviceStatus {
    pub sync_status: Option<String>,
    pub sync_error: Option<String>,
    pub last_sync_at: Option<bool>,
}

impl SyncDevice {
    pub async fn register(pool: &SqlitePool, input: RegisterDevice) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_bytes = id.as_bytes().to_vec();
        let user_bytes = input.user_id.as_bytes().to_vec();
        let org_bytes = input.organization_id.as_bytes().to_vec();
        let device_type = input.device_type.unwrap_or_else(|| "desktop".to_string());

        sqlx::query(
            r#"INSERT INTO sync_devices
                (id, user_id, organization_id, device_name, device_type, platform,
                 sync_folder, last_seen_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now','subsec'))"#,
        )
        .bind(&id_bytes)
        .bind(&user_bytes)
        .bind(&org_bytes)
        .bind(&input.device_name)
        .bind(&device_type)
        .bind(&input.platform)
        .bind(&input.sync_folder)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sync_devices WHERE id = ?",
        )
        .bind(&id_bytes)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_user_org(
        pool: &SqlitePool,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let user_bytes = user_id.as_bytes().to_vec();
        let org_bytes = org_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_devices
            WHERE user_id = ? AND organization_id = ? AND is_active = 1
            ORDER BY last_seen_at DESC"#,
        )
        .bind(&user_bytes)
        .bind(&org_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        org_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let org_bytes = org_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_devices
            WHERE organization_id = ? AND is_active = 1
            ORDER BY last_seen_at DESC"#,
        )
        .bind(&org_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn heartbeat(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query(
            "UPDATE sync_devices SET last_seen_at = datetime('now','subsec'), updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&id_bytes)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateDeviceStatus,
    ) -> Result<Option<Self>, sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        let mut sets = vec!["last_seen_at = datetime('now','subsec')"];
        let mut binds: Vec<Option<String>> = Vec::new();

        if let Some(ref status) = input.sync_status {
            sets.push("sync_status = ?");
            binds.push(Some(status.clone()));
        }
        if let Some(ref err) = input.sync_error {
            sets.push("sync_error = ?");
            binds.push(Some(err.clone()));
        }
        if input.last_sync_at == Some(true) {
            sets.push("last_sync_at = datetime('now','subsec')");
        }

        sets.push("updated_at = datetime('now','subsec')");
        let query_str = format!(
            "UPDATE sync_devices SET {} WHERE id = ?",
            sets.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for val in &binds {
            query = query.bind(val);
        }
        query = query.bind(&id_bytes);
        query.execute(pool).await?;

        Self::find_by_id(pool, id).await
    }

    pub async fn deactivate(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query(
            "UPDATE sync_devices SET is_active = 0, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&id_bytes)
        .execute(pool)
        .await?;
        Ok(())
    }
}

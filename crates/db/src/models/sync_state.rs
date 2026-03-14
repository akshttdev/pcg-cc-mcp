//! Sync State — per-file tracking of sync status across devices.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncState {
    pub id: Uuid,
    pub device_id: Uuid,
    pub data_source_id: Option<Uuid>,
    pub sync_folder_id: Option<Uuid>,
    pub file_path: String,
    pub file_hash: Option<String>,
    pub file_size: Option<i64>,
    pub local_modified: Option<DateTime<Utc>>,
    pub remote_modified: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub sync_direction: Option<String>,
    pub conflict_type: Option<String>,
    pub resolution: Option<String>,
    pub error_message: Option<String>,
    pub synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertSyncState {
    pub device_id: Uuid,
    pub data_source_id: Option<Uuid>,
    pub sync_folder_id: Option<Uuid>,
    pub file_path: String,
    pub file_hash: Option<String>,
    pub file_size: Option<i64>,
    pub sync_status: String,
    pub sync_direction: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ResolveConflict {
    pub resolution: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SyncSummary {
    pub total_files: i64,
    pub synced: i64,
    pub pending: i64,
    pub conflicts: i64,
    pub errors: i64,
    pub total_size: i64,
}

impl SyncState {
    pub async fn upsert(pool: &SqlitePool, input: UpsertSyncState) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_bytes = id.as_bytes().to_vec();
        let device_bytes = input.device_id.as_bytes().to_vec();
        let ds_bytes = input.data_source_id.map(|u| u.as_bytes().to_vec());
        let folder_bytes = input.sync_folder_id.map(|u| u.as_bytes().to_vec());

        sqlx::query(
            r#"INSERT INTO sync_state
                (id, device_id, data_source_id, sync_folder_id, file_path,
                 file_hash, file_size, sync_status, sync_direction, error_message,
                 synced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                    CASE WHEN ? = 'synced' THEN datetime('now','subsec') ELSE NULL END)
            ON CONFLICT(device_id, file_path) DO UPDATE SET
                data_source_id = excluded.data_source_id,
                sync_folder_id = excluded.sync_folder_id,
                file_hash = excluded.file_hash,
                file_size = excluded.file_size,
                sync_status = excluded.sync_status,
                sync_direction = excluded.sync_direction,
                error_message = excluded.error_message,
                synced_at = CASE WHEN excluded.sync_status = 'synced' THEN datetime('now','subsec') ELSE sync_state.synced_at END,
                updated_at = datetime('now','subsec')"#,
        )
        .bind(&id_bytes)
        .bind(&device_bytes)
        .bind(&ds_bytes)
        .bind(&folder_bytes)
        .bind(&input.file_path)
        .bind(&input.file_hash)
        .bind(input.file_size)
        .bind(&input.sync_status)
        .bind(&input.sync_direction)
        .bind(&input.error_message)
        .bind(&input.sync_status) // for the CASE
        .execute(pool)
        .await?;

        // Fetch the upserted row
        let device_bytes2 = input.device_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sync_state WHERE device_id = ? AND file_path = ?",
        )
        .bind(&device_bytes2)
        .bind(&input.file_path)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_device(
        pool: &SqlitePool,
        device_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let device_bytes = device_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_state
            WHERE device_id = ?
            ORDER BY updated_at DESC"#,
        )
        .bind(&device_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_folder(
        pool: &SqlitePool,
        folder_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let folder_bytes = folder_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_state
            WHERE sync_folder_id = ?
            ORDER BY file_path ASC"#,
        )
        .bind(&folder_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn find_conflicts(
        pool: &SqlitePool,
        device_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let device_bytes = device_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_state
            WHERE device_id = ? AND sync_status = 'conflict'
            ORDER BY updated_at DESC"#,
        )
        .bind(&device_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn find_pending(
        pool: &SqlitePool,
        device_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let device_bytes = device_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_state
            WHERE device_id = ? AND sync_status = 'pending'
            ORDER BY updated_at ASC"#,
        )
        .bind(&device_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn resolve_conflict(
        pool: &SqlitePool,
        id: Uuid,
        resolution: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query(
            r#"UPDATE sync_state SET
                resolution = ?,
                sync_status = 'pending',
                updated_at = datetime('now','subsec')
            WHERE id = ? AND sync_status = 'conflict'"#,
        )
        .bind(resolution)
        .bind(&id_bytes)
        .execute(pool)
        .await?;

        let id_bytes2 = id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>("SELECT * FROM sync_state WHERE id = ?")
            .bind(&id_bytes2)
            .fetch_optional(pool)
            .await
    }

    pub async fn summary_for_device(
        pool: &SqlitePool,
        device_id: Uuid,
    ) -> Result<SyncSummary, sqlx::Error> {
        let device_bytes = device_id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
            r#"SELECT
                COUNT(*) as total_files,
                SUM(CASE WHEN sync_status = 'synced' THEN 1 ELSE 0 END) as synced,
                SUM(CASE WHEN sync_status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN sync_status = 'conflict' THEN 1 ELSE 0 END) as conflicts,
                SUM(CASE WHEN sync_status = 'error' THEN 1 ELSE 0 END) as errors,
                COALESCE(SUM(file_size), 0) as total_size
            FROM sync_state WHERE device_id = ?"#,
        )
        .bind(&device_bytes)
        .fetch_one(pool)
        .await?;

        Ok(SyncSummary {
            total_files: row.0,
            synced: row.1,
            pending: row.2,
            conflicts: row.3,
            errors: row.4,
            total_size: row.5,
        })
    }

    pub async fn summary_for_org(
        pool: &SqlitePool,
        org_id: Uuid,
    ) -> Result<SyncSummary, sqlx::Error> {
        let org_bytes = org_id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
            r#"SELECT
                COUNT(*) as total_files,
                SUM(CASE WHEN ss.sync_status = 'synced' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ss.sync_status = 'pending' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ss.sync_status = 'conflict' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ss.sync_status = 'error' THEN 1 ELSE 0 END),
                COALESCE(SUM(ss.file_size), 0)
            FROM sync_state ss
            JOIN sync_devices sd ON ss.device_id = sd.id
            WHERE sd.organization_id = ?"#,
        )
        .bind(&org_bytes)
        .fetch_one(pool)
        .await?;

        Ok(SyncSummary {
            total_files: row.0,
            synced: row.1,
            pending: row.2,
            conflicts: row.3,
            errors: row.4,
            total_size: row.5,
        })
    }
}

//! Cloud Storage Account — provider-specific sync state.
//!
//! Holds the operational bits that don't belong in `integration_connections`:
//! cursor / delta token, sync root path, sync interval, last_sync_at, and a
//! running file count. The OAuth credentials live in the unified store; this
//! row links to them via `integration_connection_id`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CloudStorageAccount {
    pub id: Uuid,
    pub organization_id: Uuid,
    #[ts(optional)]
    pub project_id: Option<Uuid>,
    #[ts(optional)]
    pub integration_connection_id: Option<Uuid>,

    pub provider: String,
    #[ts(optional)]
    pub account_email: Option<String>,
    #[ts(optional)]
    pub display_name: Option<String>,

    #[ts(optional)]
    pub sync_cursor: Option<String>,
    #[ts(optional)]
    pub sync_root_path: Option<String>,
    pub auto_sync: bool,
    pub sync_interval_secs: i64,
    #[ts(type = "Date | null", optional)]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub last_error: Option<String>,
    pub status: String,

    pub total_files_synced: i64,
    pub metadata: String,

    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCloudStorageAccount {
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
    pub integration_connection_id: Option<Uuid>,
    pub provider: String,
    pub account_email: Option<String>,
    pub display_name: Option<String>,
    pub sync_root_path: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCloudStorageAccount {
    pub auto_sync: Option<bool>,
    pub sync_interval_secs: Option<i64>,
    pub sync_root_path: Option<String>,
}

impl CloudStorageAccount {
    /// Insert a new account, or refresh `display_name`/`integration_connection_id`
    /// on conflict (org + provider + account_email). Sync state (cursor, last_sync)
    /// is preserved across reconnects so a re-auth doesn't re-crawl from scratch.
    pub async fn upsert(
        pool: &SqlitePool,
        c: CreateCloudStorageAccount,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO cloud_storage_accounts (
                id, organization_id, project_id, integration_connection_id,
                provider, account_email, display_name, sync_root_path, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, COALESCE(?9, '{}'))
            ON CONFLICT(organization_id, provider, account_email) DO UPDATE SET
                integration_connection_id = excluded.integration_connection_id,
                display_name              = COALESCE(excluded.display_name, cloud_storage_accounts.display_name),
                sync_root_path            = COALESCE(excluded.sync_root_path, cloud_storage_accounts.sync_root_path),
                project_id                = COALESCE(excluded.project_id, cloud_storage_accounts.project_id),
                status                    = 'active',
                last_error                = NULL,
                updated_at                = datetime('now','subsec')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(c.organization_id)
        .bind(c.project_id)
        .bind(c.integration_connection_id)
        .bind(&c.provider)
        .bind(&c.account_email)
        .bind(&c.display_name)
        .bind(&c.sync_root_path)
        .bind(&c.metadata)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, organization_id, project_id, integration_connection_id, \
                    provider, account_email, display_name, \
                    sync_cursor, sync_root_path, auto_sync, sync_interval_secs, \
                    last_sync_at, last_error, status, total_files_synced, metadata, \
                    created_at, updated_at \
             FROM cloud_storage_accounts WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_org(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, organization_id, project_id, integration_connection_id, \
                    provider, account_email, display_name, \
                    sync_cursor, sync_root_path, auto_sync, sync_interval_secs, \
                    last_sync_at, last_error, status, total_files_synced, metadata, \
                    created_at, updated_at \
             FROM cloud_storage_accounts \
             WHERE organization_id = ?1 ORDER BY created_at DESC",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    /// Active rows whose interval has elapsed since their last sync (or never
    /// synced). Drives the background sync worker.
    pub async fn find_due_for_sync(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, organization_id, project_id, integration_connection_id, \
                    provider, account_email, display_name, \
                    sync_cursor, sync_root_path, auto_sync, sync_interval_secs, \
                    last_sync_at, last_error, status, total_files_synced, metadata, \
                    created_at, updated_at \
             FROM cloud_storage_accounts \
             WHERE auto_sync = 1 \
               AND status = 'active' \
               AND (last_sync_at IS NULL \
                    OR datetime(last_sync_at, '+' || sync_interval_secs || ' seconds') <= datetime('now'))",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        upd: UpdateCloudStorageAccount,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut sets = vec!["updated_at = datetime('now','subsec')".to_string()];
        if let Some(v) = upd.auto_sync {
            sets.push(format!("auto_sync = {}", if v { 1 } else { 0 }));
        }
        if let Some(v) = upd.sync_interval_secs {
            sets.push(format!("sync_interval_secs = {v}"));
        }
        let mut sync_root_bind: Option<String> = None;
        if let Some(v) = upd.sync_root_path {
            sets.push("sync_root_path = ?".to_string());
            sync_root_bind = Some(v);
        }

        let sql = format!(
            "UPDATE cloud_storage_accounts SET {} WHERE id = ?",
            sets.join(", ")
        );

        let mut q = sqlx::query(&sql);
        if let Some(ref v) = sync_root_bind {
            q = q.bind(v);
        }
        q = q.bind(id);
        q.execute(pool).await?;

        Self::find_by_id(pool, id).await
    }

    /// Atomically advance sync state after a successful incremental sync.
    pub async fn record_sync_success(
        pool: &SqlitePool,
        id: Uuid,
        new_cursor: Option<&str>,
        files_added: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE cloud_storage_accounts SET \
                sync_cursor = COALESCE(?1, sync_cursor), \
                last_sync_at = datetime('now','subsec'), \
                last_error = NULL, \
                status = 'active', \
                total_files_synced = total_files_synced + ?2, \
                updated_at = datetime('now','subsec') \
             WHERE id = ?3",
        )
        .bind(new_cursor)
        .bind(files_added)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn record_sync_error(
        pool: &SqlitePool,
        id: Uuid,
        error_msg: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE cloud_storage_accounts SET \
                status = 'error', \
                last_error = ?1, \
                updated_at = datetime('now','subsec') \
             WHERE id = ?2",
        )
        .bind(error_msg)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_status(pool: &SqlitePool, id: Uuid, status: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE cloud_storage_accounts SET \
                status = ?1, updated_at = datetime('now','subsec') \
             WHERE id = ?2",
        )
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM cloud_storage_accounts WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

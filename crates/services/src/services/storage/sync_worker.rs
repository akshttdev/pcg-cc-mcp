//! Cloud storage sync worker — runs incremental syncs for connected accounts.
//!
//! Two entry points:
//! - `spawn_sync_loop(pool)` — starts the 15-minute background ticker
//! - `sync_account(pool, account_id)` — runs one sync now (used by the
//!    `POST /storage/accounts/:id/sync` route and by the loop)
//!
//! Sync produces two writes per remote change:
//! 1. `cloud_storage_files` — provider-specific row keyed by (account, remote_id)
//! 2. `cloud_files`         — generic file row (only when not a folder/deleted)
//!
//! Soft-deletes flow the other direction: a deletion event soft-deletes both
//! the sidecar and the linked cloud_files row.

use chrono::{DateTime, Utc};
use db::models::{
    cloud_file::{CloudFile, CreateCloudFile},
    cloud_storage_account::CloudStorageAccount,
};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::services::{
    oauth_token_manager::{self, TokenManagerError},
    storage::{self, RemoteFileChange, StorageError},
};

#[derive(Debug, Error)]
pub enum SyncWorkerError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    TokenManager(#[from] TokenManagerError),
    #[error("storage account {0} not found")]
    AccountNotFound(Uuid),
    #[error("account {0} has no integration connection")]
    NoConnection(Uuid),
}

/// Stats from one sync pass.
#[derive(Debug, Default, Clone)]
pub struct SyncStats {
    pub files_added: i64,
    pub files_updated: i64,
    pub files_deleted: i64,
    pub folders_seen: i64,
}

/// Run an incremental sync for one account. Touches:
///   - `cloud_storage_files` (provider mapping, upsert)
///   - `cloud_files`          (canonical file index)
///   - `cloud_storage_accounts` (cursor + last_sync_at)
pub async fn sync_account(
    pool: &SqlitePool,
    account_id: Uuid,
) -> Result<SyncStats, SyncWorkerError> {
    let account = CloudStorageAccount::find_by_id(pool, account_id)
        .await?
        .ok_or(SyncWorkerError::AccountNotFound(account_id))?;

    let connection_id = account
        .integration_connection_id
        .ok_or(SyncWorkerError::NoConnection(account.id))?;

    let access_token = oauth_token_manager::get_access_token(pool, connection_id).await?;
    let connector = storage::get_connector(&account.provider)?;

    CloudStorageAccount::mark_status(pool, account.id, "syncing").await?;

    let result = connector
        .sync_changes(
            &access_token,
            account.sync_cursor.as_deref(),
            account.sync_root_path.as_deref(),
        )
        .await;

    let batch = match result {
        Ok(b) => b,
        Err(e) => {
            CloudStorageAccount::record_sync_error(pool, account.id, &e.to_string()).await?;
            return Err(SyncWorkerError::Storage(e));
        }
    };

    let mut stats = SyncStats::default();
    for change in &batch.changes {
        if change.is_folder {
            stats.folders_seen += 1;
            apply_folder_change(pool, &account, change).await?;
            continue;
        }
        if change.deleted {
            if apply_deletion(pool, &account, change).await? {
                stats.files_deleted += 1;
            }
        } else if apply_upsert(pool, &account, change).await? {
            stats.files_added += 1;
        } else {
            stats.files_updated += 1;
        }
    }

    CloudStorageAccount::record_sync_success(
        pool,
        account.id,
        batch.next_cursor.as_deref(),
        stats.files_added,
    )
    .await?;

    Ok(stats)
}

async fn apply_folder_change(
    pool: &SqlitePool,
    account: &CloudStorageAccount,
    change: &RemoteFileChange,
) -> Result<(), sqlx::Error> {
    // Track folders so a later path-resolution pass can build full paths,
    // but skip cloud_files writes — folders aren't files.
    upsert_storage_file_row(pool, account, change, None).await
}

/// Returns whether the change produced a brand-new row vs updated an existing
/// one. Folders/deleted changes never reach this path.
async fn apply_upsert(
    pool: &SqlitePool,
    account: &CloudStorageAccount,
    change: &RemoteFileChange,
) -> Result<bool, sqlx::Error> {
    // Try to find an existing sidecar row for this (account, remote_id).
    // SQLite's rows_affected can't distinguish INSERT from ON CONFLICT UPDATE
    // (both return 1), so we do an explicit lookup to drive the stats counter.
    let existing: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, cloud_file_id \
         FROM cloud_storage_files \
         WHERE cloud_storage_account_id = ?1 AND remote_id = ?2",
    )
    .bind(account.id)
    .bind(&change.remote_id)
    .fetch_optional(pool)
    .await?;

    let was_new = existing.is_none();

    let cloud_file_id = match existing.as_ref().and_then(|(_, cf)| cf.clone()) {
        Some(existing_id) => {
            // Update the existing cloud_files row's basic metadata.
            sqlx::query(
                "UPDATE cloud_files SET \
                    file_name = ?1, file_path = ?2, mime_type = ?3, \
                    file_size_bytes = ?4, content_hash = ?5, \
                    deleted_at = NULL, \
                    updated_at = datetime('now','subsec') \
                 WHERE id = ?6",
            )
            .bind(&change.name)
            .bind(&change.path)
            .bind(&change.mime_type)
            .bind(change.size_bytes)
            .bind(&change.content_hash)
            .bind(&existing_id)
            .execute(pool)
            .await?;
            existing_id
        }
        None => {
            // Create a fresh cloud_files row. storage_volume='dropbox' is the
            // legacy bucket — the sync worker writes there for now until the
            // CHECK constraint is broadened to include 'cloud_sync'.
            let created = CloudFile::create(
                pool,
                &CreateCloudFile {
                    organization_id: account.organization_id.to_string(),
                    project_id: account.project_id.map(|p| p.to_string()),
                    task_id: None,
                    file_name: change.name.clone(),
                    file_path: change.path.clone(),
                    storage_volume: "dropbox".to_string(),
                    content_hash: change.content_hash.clone(),
                    file_size_bytes: change.size_bytes,
                    mime_type: change.mime_type.clone(),
                    source_type: Some("sync".to_string()),
                    source_id: Some(change.remote_id.clone()),
                    source_table: Some("cloud_storage_accounts".to_string()),
                    visibility: Some("org".to_string()),
                    contributed_by: None,
                    contributor_wallet: None,
                    contributor_device: None,
                },
            )
            .await?;
            created.id
        }
    };

    upsert_storage_file_row(pool, account, change, Some(&cloud_file_id)).await?;
    Ok(was_new)
}

/// Upsert a `cloud_storage_files` row.
async fn upsert_storage_file_row(
    pool: &SqlitePool,
    account: &CloudStorageAccount,
    change: &RemoteFileChange,
    cloud_file_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let new_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cloud_storage_files (\
            id, cloud_storage_account_id, cloud_file_id, remote_id, remote_path, \
            remote_name, mime_type, size_bytes, content_hash, web_url, is_folder, \
            remote_modified_at, last_seen_at\
         ) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now','subsec')) \
         ON CONFLICT(cloud_storage_account_id, remote_id) DO UPDATE SET \
            cloud_file_id      = COALESCE(excluded.cloud_file_id, cloud_storage_files.cloud_file_id), \
            remote_path        = excluded.remote_path, \
            remote_name        = excluded.remote_name, \
            mime_type          = excluded.mime_type, \
            size_bytes         = excluded.size_bytes, \
            content_hash       = excluded.content_hash, \
            web_url            = excluded.web_url, \
            remote_modified_at = excluded.remote_modified_at, \
            last_seen_at       = datetime('now','subsec'), \
            deleted_at         = NULL, \
            updated_at         = datetime('now','subsec')",
    )
    .bind(&new_id)
    .bind(account.id)
    .bind(cloud_file_id)
    .bind(&change.remote_id)
    .bind(&change.path)
    .bind(&change.name)
    .bind(&change.mime_type)
    .bind(change.size_bytes)
    .bind(&change.content_hash)
    .bind(&change.web_url)
    .bind(if change.is_folder { 1 } else { 0 })
    .bind(change.remote_modified_at_str())
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns true if a row was actually deleted (vs nothing found).
async fn apply_deletion(
    pool: &SqlitePool,
    account: &CloudStorageAccount,
    change: &RemoteFileChange,
) -> Result<bool, sqlx::Error> {
    // Locate the sidecar row by remote_id (Dropbox deletions also fall back to
    // path; we accept either as long as remote_id is set on the change).
    let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, cloud_file_id \
         FROM cloud_storage_files \
         WHERE cloud_storage_account_id = ?1 AND remote_id = ?2",
    )
    .bind(account.id)
    .bind(&change.remote_id)
    .fetch_optional(pool)
    .await?;

    let Some((sidecar_id, cf_id)) = row else {
        return Ok(false);
    };

    sqlx::query(
        "UPDATE cloud_storage_files SET \
            deleted_at = datetime('now','subsec'), \
            updated_at = datetime('now','subsec') \
         WHERE id = ?1",
    )
    .bind(sidecar_id)
    .execute(pool)
    .await?;

    if let Some(cf) = cf_id {
        CloudFile::soft_delete(pool, &cf).await?;
    }

    Ok(true)
}

trait RemoteFileChangeExt {
    fn remote_modified_at_str(&self) -> Option<String>;
}

impl RemoteFileChangeExt for RemoteFileChange {
    fn remote_modified_at_str(&self) -> Option<String> {
        self.modified_at.map(format_dt)
    }
}

fn format_dt(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

// ─── Background loop ───────────────────────────────────────────────────────

/// Spawns a tokio task that runs every 15 minutes, syncing every account whose
/// interval has elapsed.
pub fn spawn_sync_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        let mut ticker = interval(std::time::Duration::from_secs(900));
        ticker.tick().await; // discard immediate fire
        loop {
            ticker.tick().await;
            if let Err(e) = run_due_syncs(&pool).await {
                error!("Cloud storage sync loop error: {e}");
            }
        }
    });
    info!("Cloud storage sync worker spawned (15-min interval)");
}

async fn run_due_syncs(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let due = CloudStorageAccount::find_due_for_sync(pool).await?;
    if due.is_empty() {
        return Ok(());
    }
    info!("Cloud storage sync: {} account(s) due", due.len());

    for account in due {
        let id = account.id;
        let provider = account.provider.clone();
        match sync_account(pool, id).await {
            Ok(stats) => info!(
                "Synced {} ({}) — added {}, updated {}, deleted {}",
                provider, id, stats.files_added, stats.files_updated, stats.files_deleted
            ),
            Err(e) => warn!("Sync failed for {} ({}): {}", provider, id, e),
        }
    }
    Ok(())
}

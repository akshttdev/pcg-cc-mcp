//! Cloud storage extraction worker.
//!
//! Fills the gap between `sync_worker` (which catalogs remote files into
//! `cloud_files`) and the knowledge graph (which reads from `data_sources`).
//! Every tick, picks a small batch of newly-synced text-extractable files,
//! downloads them through the appropriate `StorageConnector`, runs text
//! extraction, and writes a `data_source` row with `source_type='integration'`.
//!
//! Dedup: a file is considered "already extracted" if any `data_sources` row
//! exists for its organization with `metadata.cloud_file_id = cloud_files.id`.
//! Failed extractions are persisted as `status='failed'` rows so we don't
//! retry forever; transient download failures leave no row and get retried.

use std::collections::HashMap;

use db::models::data_source::{CreateDataSource, DataSource};
use serde_json::json;
use sqlx::SqlitePool;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::services::{
    oauth_token_manager::{self, TokenManagerError},
    storage::{self, StorageError, extractor},
};

/// How often the worker wakes. The first extraction batch happens at t+5min
/// to give sync time to populate `cloud_files` on a fresh boot.
const EXTRACT_INTERVAL_SECS: u64 = 600;

/// Max files processed per tick — bounds memory + provider rate-limit blast.
const BATCH_LIMIT: i64 = 20;

/// Skip files above this size; PDFs especially can OOM. Currently 50 MB.
const MAX_FILE_BYTES: i64 = 50 * 1024 * 1024;

/// One row of work the SQL produces for the worker.
#[derive(Debug, Clone)]
struct PendingFile {
    cloud_file_id: String,
    organization_id: String,
    project_id: Option<String>,
    file_name: String,
    file_path: Option<String>,
    mime_type: Option<String>,
    content_hash: Option<String>,
    file_size_bytes: Option<i64>,
    remote_id: String,
    storage_account_id: Uuid,
    provider: String,
}

#[derive(Debug, Default, Clone)]
pub struct ExtractStats {
    pub extracted: i64,
    pub failed: i64,
    pub skipped: i64,
}

/// Spawn a detached Tokio task that runs the extraction loop forever.
pub fn spawn_extract_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        let mut ticker = interval(std::time::Duration::from_secs(EXTRACT_INTERVAL_SECS));
        ticker.tick().await; // discard the immediate fire
        loop {
            ticker.tick().await;
            match run_one_pass(&pool).await {
                Ok(stats) if stats.extracted + stats.failed + stats.skipped > 0 => {
                    info!(
                        "Cloud extraction pass: extracted={} failed={} skipped={}",
                        stats.extracted, stats.failed, stats.skipped
                    );
                }
                Ok(_) => {}
                Err(e) => error!("Cloud extraction loop error: {e}"),
            }
        }
    });
    info!(
        "Cloud storage extract worker spawned ({}-min interval, batch={}, max_bytes={})",
        EXTRACT_INTERVAL_SECS / 60,
        BATCH_LIMIT,
        MAX_FILE_BYTES
    );
}

/// One extraction pass. Picks up to `BATCH_LIMIT` pending files, groups them
/// by storage account so a single access-token resolution serves the whole
/// account, and processes each. Errors per file are logged; the loop never
/// aborts on a single bad file.
pub async fn run_one_pass(pool: &SqlitePool) -> Result<ExtractStats, sqlx::Error> {
    let pending = fetch_pending(pool, BATCH_LIMIT).await?;
    if pending.is_empty() {
        return Ok(ExtractStats::default());
    }

    let mut by_account: HashMap<Uuid, Vec<PendingFile>> = HashMap::new();
    for p in pending {
        by_account.entry(p.storage_account_id).or_default().push(p);
    }

    let mut stats = ExtractStats::default();
    for (account_id, files) in by_account {
        match process_account(pool, account_id, &files).await {
            Ok(account_stats) => {
                stats.extracted += account_stats.extracted;
                stats.failed += account_stats.failed;
                stats.skipped += account_stats.skipped;
            }
            Err(e) => {
                warn!(
                    "Extraction skipped account {account_id}: {e} ({} files)",
                    files.len()
                );
                stats.skipped += files.len() as i64;
            }
        }
    }
    Ok(stats)
}

/// Resolve the access token once for an account, then walk its files. Any
/// per-file failure stays scoped to that file.
async fn process_account(
    pool: &SqlitePool,
    account_id: Uuid,
    files: &[PendingFile],
) -> Result<ExtractStats, ProcessError> {
    let connection_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT integration_connection_id FROM cloud_storage_accounts WHERE id = ?1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .map_err(ProcessError::Database)?
    .flatten()
    .ok_or(ProcessError::NoConnection)?;

    let connection_uuid = Uuid::parse_str(&connection_id).map_err(|_| ProcessError::NoConnection)?;
    let access_token = oauth_token_manager::get_access_token(pool, connection_uuid).await?;

    let provider = &files[0].provider;
    let connector = storage::get_connector(provider).map_err(ProcessError::Storage)?;

    let mut stats = ExtractStats::default();
    for file in files {
        match process_file(pool, &*connector, &access_token, file).await {
            Outcome::Extracted => stats.extracted += 1,
            Outcome::FailedRecorded => stats.failed += 1,
            Outcome::Transient(reason) => {
                warn!(
                    "Transient skip for {} ({}): {reason}",
                    file.file_name, file.cloud_file_id
                );
                stats.skipped += 1;
            }
        }
    }
    Ok(stats)
}

enum Outcome {
    /// Text extracted and persisted as a fresh `data_source` row.
    Extracted,
    /// Parser recognized the format but failed; persisted a `status='failed'`
    /// row so we don't retry forever.
    FailedRecorded,
    /// Network/auth glitch or oversize file — left untouched so the next pass
    /// can retry.
    Transient(String),
}

async fn process_file(
    pool: &SqlitePool,
    connector: &dyn storage::StorageConnector,
    access_token: &str,
    file: &PendingFile,
) -> Outcome {
    let size = file.file_size_bytes.unwrap_or(0);
    if size <= 0 || size > MAX_FILE_BYTES {
        return Outcome::Transient(format!("size {size} outside extractable range"));
    }

    let bytes = match connector.download_file(access_token, &file.remote_id).await {
        Ok(b) => b,
        Err(e) => return Outcome::Transient(format!("download: {e}")),
    };

    match extractor::extract_text(file.mime_type.as_deref(), &bytes) {
        Ok(Some(text)) if !text.is_empty() => {
            match create_extracted_data_source(pool, file, &text).await {
                Ok(()) => Outcome::Extracted,
                Err(e) => {
                    warn!("Failed to persist extracted text for {}: {e}", file.file_name);
                    Outcome::Transient(format!("persist: {e}"))
                }
            }
        }
        Ok(Some(_)) => Outcome::Transient("extractor returned empty text".into()),
        Ok(None) => Outcome::Transient("mime not extractable".into()),
        Err(e) => {
            let _ = create_failed_data_source(pool, file, &e.to_string()).await;
            Outcome::FailedRecorded
        }
    }
}

async fn create_extracted_data_source(
    pool: &SqlitePool,
    file: &PendingFile,
    text: &str,
) -> Result<(), sqlx::Error> {
    let folder = folder_for(&file.provider, file.file_path.as_deref(), &file.file_name);
    let metadata = json!({
        "cloud_file_id": file.cloud_file_id,
        "provider": file.provider,
        "remote_id": file.remote_id,
        "storage_account_id": file.storage_account_id.to_string(),
        "extracted_at": chrono::Utc::now().to_rfc3339(),
    });
    DataSource::create(
        pool,
        CreateDataSource {
            organization_id: Some(file.organization_id.clone()),
            project_id: file.project_id.clone(),
            created_by: None,
            title: file.file_name.clone(),
            description: None,
            data_type: "document".into(),
            source_type: Some("integration".into()),
            file_type: file.mime_type.clone(),
            content: Some(text.to_string()),
            file_name: Some(file.file_name.clone()),
            file_path: file.file_path.clone(),
            file_size_bytes: file.file_size_bytes,
            file_hash: file.content_hash.clone(),
            metadata: Some(metadata.to_string()),
            folder: Some(folder),
        },
    )
    .await?;
    Ok(())
}

async fn create_failed_data_source(
    pool: &SqlitePool,
    file: &PendingFile,
    err: &str,
) -> Result<(), sqlx::Error> {
    let folder = folder_for(&file.provider, file.file_path.as_deref(), &file.file_name);
    let metadata = json!({
        "cloud_file_id": file.cloud_file_id,
        "provider": file.provider,
        "remote_id": file.remote_id,
        "storage_account_id": file.storage_account_id.to_string(),
        "extraction_error": err,
    });
    // Insert directly so we can stamp status='failed' + processing_error in
    // one shot (CreateDataSource has no status field).
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO data_sources \
            (id, organization_id, project_id, title, data_type, source_type, \
             file_type, file_name, file_path, file_size_bytes, file_hash, \
             metadata, status, processing_error, folder) \
         VALUES (?1, ?2, ?3, ?4, 'document', 'integration', ?5, ?6, ?7, ?8, ?9, ?10, 'failed', ?11, ?12)",
    )
    .bind(id)
    .bind(&file.organization_id)
    .bind(&file.project_id)
    .bind(&file.file_name)
    .bind(&file.mime_type)
    .bind(&file.file_name)
    .bind(&file.file_path)
    .bind(file.file_size_bytes)
    .bind(&file.content_hash)
    .bind(metadata.to_string())
    .bind(err)
    .bind(folder)
    .execute(pool)
    .await?;
    Ok(())
}

/// Build the slash-delimited folder string for a data_source. Drops the file
/// name from the tail of the path so a file at `Projects/Brand/brief.pdf`
/// gets `OneDrive/Projects/Brand`.
fn folder_for(provider: &str, path: Option<&str>, file_name: &str) -> String {
    let provider_label = match provider {
        "onedrive" => "OneDrive",
        "dropbox" => "Dropbox",
        "gdrive" => "Google Drive",
        other => other,
    };
    let raw = path.unwrap_or("").trim_matches('/');
    if raw.is_empty() {
        return provider_label.to_string();
    }
    let without_name = raw
        .strip_suffix(file_name)
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| raw.to_string());
    if without_name.is_empty() {
        provider_label.to_string()
    } else {
        format!("{provider_label}/{without_name}")
    }
}

type PendingRow = (
    String,         // cf.id
    String,         // cf.organization_id
    Option<String>, // cf.project_id
    String,         // cf.file_name
    Option<String>, // cf.file_path
    Option<String>, // cf.mime_type
    Option<String>, // cf.content_hash
    Option<i64>,    // cf.file_size_bytes
    String,         // csf.remote_id
    String,         // csf.cloud_storage_account_id
    String,         // csa.provider
);

async fn fetch_pending(pool: &SqlitePool, limit: i64) -> Result<Vec<PendingFile>, sqlx::Error> {
    let rows: Vec<PendingRow> = sqlx::query_as(
        "SELECT \
            cf.id, cf.organization_id, cf.project_id, cf.file_name, cf.file_path, \
            cf.mime_type, cf.content_hash, cf.file_size_bytes, \
            csf.remote_id, csf.cloud_storage_account_id, csa.provider \
         FROM cloud_files cf \
         INNER JOIN cloud_storage_files csf ON csf.cloud_file_id = cf.id \
         INNER JOIN cloud_storage_accounts csa ON csa.id = csf.cloud_storage_account_id \
         WHERE cf.deleted_at IS NULL \
           AND csf.deleted_at IS NULL \
           AND csf.is_folder = 0 \
           AND cf.mime_type IS NOT NULL \
           AND (cf.mime_type LIKE 'text/%' \
                OR cf.mime_type = 'application/pdf' \
                OR cf.mime_type = 'application/json') \
           AND COALESCE(cf.file_size_bytes, 0) > 0 \
           AND COALESCE(cf.file_size_bytes, 0) <= ?1 \
           AND NOT EXISTS ( \
                SELECT 1 FROM data_sources ds \
                WHERE ds.source_type = 'integration' \
                  AND ds.organization_id = cf.organization_id \
                  AND json_extract(ds.metadata, '$.cloud_file_id') = cf.id \
           ) \
         ORDER BY cf.updated_at ASC \
         LIMIT ?2",
    )
    .bind(MAX_FILE_BYTES)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let pending = rows
        .into_iter()
        .filter_map(|r| {
            let storage_account_id = Uuid::parse_str(&r.9).ok()?;
            Some(PendingFile {
                cloud_file_id: r.0,
                organization_id: r.1,
                project_id: r.2,
                file_name: r.3,
                file_path: r.4,
                mime_type: r.5,
                content_hash: r.6,
                file_size_bytes: r.7,
                remote_id: r.8,
                storage_account_id,
                provider: r.10,
            })
        })
        .collect();
    Ok(pending)
}

#[derive(Debug, thiserror::Error)]
enum ProcessError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Storage(StorageError),
    #[error(transparent)]
    Token(#[from] TokenManagerError),
    #[error("account has no integration_connection_id")]
    NoConnection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_for_strips_trailing_filename() {
        assert_eq!(
            folder_for("onedrive", Some("Projects/Brand/brief.pdf"), "brief.pdf"),
            "OneDrive/Projects/Brand"
        );
    }

    #[test]
    fn folder_for_root_file() {
        assert_eq!(folder_for("dropbox", Some("notes.txt"), "notes.txt"), "Dropbox");
        assert_eq!(folder_for("gdrive", None, "x.pdf"), "Google Drive");
        assert_eq!(folder_for("gdrive", Some(""), "x.pdf"), "Google Drive");
    }
}

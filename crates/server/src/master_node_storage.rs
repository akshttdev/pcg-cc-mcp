//! Master Node Storage Sync Service
//!
//! Runs as a background task on the master node, mirroring all dashboard data
//! (database, data-source uploads, media assets) to the sovereign storage
//! volume on the E: drive. Sync is hash-based — only changed files are copied.
//!
//! ## Directory layout on the storage root
//!
//! ```text
//! E:/topos/sovereign_storage/
//! ├── db/                    # SQLite database backup
//! │   └── db.sqlite
//! ├── data_sources/          # Uploaded data-source files
//! │   └── {org_slug}/
//! │       └── {filename}
//! ├── media/                 # Media pipeline assets
//! │   └── {project_id}/
//! │       └── {filename}
//! ├── team_files/            # Sirak Studios Team Dropbox mirror
//! │   └── ...                #   (recursive directory mirror)
//! └── manifest.json          # Sync manifest with checksums & stats
//! ```
//!
//! ## Environment variables
//!
//! | Variable                       | Default                        |
//! |-------------------------------|--------------------------------|
//! | `MASTER_NODE_STORAGE_ROOT`    | `E:/topos/sovereign_storage`   |
//! | `MASTER_NODE_SYNC_INTERVAL`   | `300` (seconds)                |
//! | `MASTER_NODE_SYNC_ENABLED`    | `true`                         |
//! | `TEAM_FILES_SOURCE`           | `E:/topos/Sirak Studios Team`  |

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct MasterNodeStorageConfig {
    pub enabled: bool,
    pub storage_root: PathBuf,
    pub sync_interval: Duration,
    pub device_id: String,
}

impl MasterNodeStorageConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("MASTER_NODE_SYNC_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let storage_root = PathBuf::from(
            std::env::var("MASTER_NODE_STORAGE_ROOT")
                .unwrap_or_else(|_| "E:/topos/sovereign_storage".to_string()),
        );

        let interval_secs: u64 = std::env::var("MASTER_NODE_SYNC_INTERVAL")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(300);

        let device_id = std::env::var("SOVEREIGN_STORAGE_DEVICE_ID")
            .or_else(|_| std::env::var("APN_DATA_SERVICE_DEVICE_ID"))
            .unwrap_or_else(|_| "sirak-studios-master-001".to_string());

        Self {
            enabled,
            storage_root,
            sync_interval: Duration::from_secs(interval_secs),
            device_id,
        }
    }
}

// ============================================================================
// Manifest types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub last_sync: String,
    pub device_id: String,
    pub storage_root: String,
    pub db_backup: DbBackupInfo,
    pub data_sources: SyncStats,
    pub media: SyncStats,
    pub team_files: SyncStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbBackupInfo {
    pub last_copied: String,
    pub size_bytes: u64,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub total_files: u64,
    pub total_bytes: u64,
    pub files_copied_this_cycle: u64,
    pub bytes_copied_this_cycle: u64,
}

impl SyncStats {
    fn empty() -> Self {
        Self {
            total_files: 0,
            total_bytes: 0,
            files_copied_this_cycle: 0,
            bytes_copied_this_cycle: 0,
        }
    }
}

// ============================================================================
// Entry point
// ============================================================================

/// Spawn the master-node storage sync loop as a background tokio task.
///
/// The task runs forever, syncing on each interval tick. Errors are logged
/// and never propagate — the loop continues regardless.
pub async fn spawn_master_node_sync(pool: SqlitePool) {
    let config = MasterNodeStorageConfig::from_env();

    if !config.enabled {
        info!("[MASTER-STORAGE] Sync disabled via MASTER_NODE_SYNC_ENABLED=false");
        return;
    }

    info!(
        "[MASTER-STORAGE] Starting sync service — root={}, interval={}s, device={}",
        config.storage_root.display(),
        config.sync_interval.as_secs(),
        config.device_id,
    );

    tokio::spawn(async move {
        let mut interval = time::interval(config.sync_interval);
        // First tick fires immediately
        interval.tick().await;

        loop {
            interval.tick().await;
            info!("[MASTER-STORAGE] Beginning sync cycle");

            match run_sync_cycle(&pool, &config).await {
                Ok(()) => info!("[MASTER-STORAGE] Sync cycle completed successfully"),
                Err(e) => error!("[MASTER-STORAGE] Sync cycle failed: {e:#}"),
            }
        }
    });
}

// ============================================================================
// Core sync cycle
// ============================================================================

async fn run_sync_cycle(pool: &SqlitePool, config: &MasterNodeStorageConfig) -> anyhow::Result<()> {
    let root = &config.storage_root;

    // Ensure top-level directories exist
    for sub in &["db", "data_sources", "media", "team_files"] {
        tokio::fs::create_dir_all(root.join(sub)).await?;
    }

    // 1. Database backup
    let db_info = sync_database(root).await?;

    // 2. Data-source files
    let ds_stats = sync_data_sources(pool, root).await.unwrap_or_else(|e| {
        error!("[MASTER-STORAGE] Data-source sync error: {e:#}");
        SyncStats::empty()
    });

    // 3. Media assets
    let media_stats = sync_media_assets(pool, root).await.unwrap_or_else(|e| {
        error!("[MASTER-STORAGE] Media sync error: {e:#}");
        SyncStats::empty()
    });

    // 4. Team files (Dropbox team folder mirror)
    let team_stats = sync_team_files(root).await.unwrap_or_else(|e| {
        error!("[MASTER-STORAGE] Team files sync error: {e:#}");
        SyncStats::empty()
    });

    // 5. Write manifest
    let manifest = SyncManifest {
        last_sync: Utc::now().to_rfc3339(),
        device_id: config.device_id.clone(),
        storage_root: config.storage_root.display().to_string(),
        db_backup: db_info,
        data_sources: ds_stats,
        media: media_stats,
        team_files: team_stats,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    tokio::fs::write(root.join("manifest.json"), manifest_json).await?;
    info!("[MASTER-STORAGE] Manifest updated");

    Ok(())
}

// ============================================================================
// Database backup
// ============================================================================

async fn sync_database(root: &Path) -> anyhow::Result<DbBackupInfo> {
    // Resolve the source DB path
    let db_path_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "dev_assets/db.sqlite".to_string());
    let db_path_str = db_path_str
        .strip_prefix("sqlite://")
        .or_else(|| db_path_str.strip_prefix("sqlite:"))
        .unwrap_or(&db_path_str);

    let source = if Path::new(db_path_str).is_absolute() {
        PathBuf::from(db_path_str)
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(db_path_str)
    };

    let dest = root.join("db").join("db.sqlite");

    if !source.exists() {
        warn!("[MASTER-STORAGE] Source DB not found at {}", source.display());
        return Ok(DbBackupInfo {
            last_copied: Utc::now().to_rfc3339(),
            size_bytes: 0,
            source_hash: String::new(),
        });
    }

    let source_hash = file_sha256(&source).await?;

    // Check if we need to copy (compare hashes)
    let needs_copy = if dest.exists() {
        let dest_hash = file_sha256(&dest).await?;
        dest_hash != source_hash
    } else {
        true
    };

    let meta = tokio::fs::metadata(&source).await?;
    let size_bytes = meta.len();

    if needs_copy {
        info!(
            "[MASTER-STORAGE] Copying database ({} bytes) to {}",
            size_bytes,
            dest.display()
        );
        tokio::fs::copy(&source, &dest).await?;

        // Also copy WAL and SHM if they exist, for consistency
        for ext in &["-wal", "-shm"] {
            let wal_src = source.with_extension(
                source
                    .extension()
                    .map(|e| format!("{}{ext}", e.to_string_lossy()))
                    .unwrap_or_else(|| ext.to_string()),
            );
            if wal_src.exists() {
                let wal_dest = dest.with_extension(
                    dest.extension()
                        .map(|e| format!("{}{ext}", e.to_string_lossy()))
                        .unwrap_or_else(|| ext.to_string()),
                );
                let _ = tokio::fs::copy(&wal_src, &wal_dest).await;
            }
        }
    } else {
        info!("[MASTER-STORAGE] Database unchanged, skipping copy");
    }

    Ok(DbBackupInfo {
        last_copied: Utc::now().to_rfc3339(),
        size_bytes,
        source_hash,
    })
}

// ============================================================================
// Data-source file sync
// ============================================================================

async fn sync_data_sources(pool: &SqlitePool, root: &Path) -> anyhow::Result<SyncStats> {
    let ds_root = root.join("data_sources");

    // Fetch all data sources that have files
    let rows = sqlx::query(
        r#"
        SELECT ds.id, ds.organization_id, ds.file_name, ds.file_path, ds.file_hash, ds.file_size_bytes,
               o.slug AS org_slug
        FROM data_sources ds
        LEFT JOIN organizations o ON o.id = ds.organization_id
        WHERE ds.file_name IS NOT NULL
          AND ds.file_path IS NOT NULL
          AND ds.archived_at IS NULL
        "#,
    )
    .fetch_all(pool)
    .await?;

    let uploads_dir = utils::cache_dir().join("data_sources");
    let mut stats = SyncStats::empty();

    for row in &rows {
        let file_name: String = match row.try_get("file_name") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let file_path: String = match row.try_get("file_path") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let org_slug: Option<String> = row.try_get("org_slug").ok();
        let stored_hash: Option<String> = row.try_get("file_hash").ok().flatten();
        let file_size: Option<i64> = row.try_get("file_size_bytes").ok().flatten();

        // Source file — could be absolute or relative to the uploads directory
        let source = if Path::new(&file_path).is_absolute() {
            PathBuf::from(&file_path)
        } else {
            uploads_dir.join(&file_path)
        };

        if !source.exists() {
            // Try just the file_name in the uploads dir
            let alt = uploads_dir.join(&file_name);
            if !alt.exists() {
                warn!(
                    "[MASTER-STORAGE] Data-source file missing: {} (tried {} and {})",
                    file_name,
                    source.display(),
                    alt.display()
                );
                continue;
            }
            // Use alt path below — we reassign via a let binding
            let source = alt;
            let slug_dir = org_slug.as_deref().unwrap_or("_unsorted");
            let dest_dir = ds_root.join(slug_dir);
            tokio::fs::create_dir_all(&dest_dir).await?;
            let dest = dest_dir.join(&file_name);

            let size = file_size.unwrap_or(0) as u64;
            stats.total_files += 1;
            stats.total_bytes += size;

            if should_copy(&source, &dest, stored_hash.as_deref()).await {
                info!("[MASTER-STORAGE] Syncing data-source: {slug_dir}/{file_name}");
                if let Err(e) = tokio::fs::copy(&source, &dest).await {
                    warn!("[MASTER-STORAGE] Failed to copy {}: {e}", source.display());
                } else {
                    stats.files_copied_this_cycle += 1;
                    stats.bytes_copied_this_cycle += size;
                }
            }
            continue;
        }

        let slug_dir = org_slug.as_deref().unwrap_or("_unsorted");
        let dest_dir = ds_root.join(slug_dir);
        tokio::fs::create_dir_all(&dest_dir).await?;
        let dest = dest_dir.join(&file_name);

        let size = file_size.unwrap_or(0) as u64;
        stats.total_files += 1;
        stats.total_bytes += size;

        if should_copy(&source, &dest, stored_hash.as_deref()).await {
            info!("[MASTER-STORAGE] Syncing data-source: {slug_dir}/{file_name}");
            if let Err(e) = tokio::fs::copy(&source, &dest).await {
                warn!("[MASTER-STORAGE] Failed to copy {}: {e}", source.display());
            } else {
                stats.files_copied_this_cycle += 1;
                stats.bytes_copied_this_cycle += size;
            }
        }
    }

    info!(
        "[MASTER-STORAGE] Data-sources: {}/{} files synced ({} bytes)",
        stats.files_copied_this_cycle, stats.total_files, stats.bytes_copied_this_cycle
    );

    Ok(stats)
}

// ============================================================================
// Media-asset file sync
// ============================================================================

async fn sync_media_assets(pool: &SqlitePool, root: &Path) -> anyhow::Result<SyncStats> {
    let media_root_env = std::env::var("MEDIA_ROOT")
        .unwrap_or_else(|_| "dev_assets/media".to_string());

    let media_source_root = if Path::new(&media_root_env).is_absolute() {
        PathBuf::from(&media_root_env)
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&media_root_env)
    };

    let media_dest_root = root.join("media");

    let rows = sqlx::query(
        r#"
        SELECT id, hex(project_id) AS project_id_hex, filename, file_path, file_size_bytes
        FROM media_assets
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut stats = SyncStats::empty();

    for row in &rows {
        let project_id: String = match row.try_get::<String, _>("project_id_hex") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let filename: String = match row.try_get("filename") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let file_path: String = match row.try_get("file_path") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let file_size: i64 = row.try_get("file_size_bytes").unwrap_or(0);

        // Resolve source path
        let source = if Path::new(&file_path).is_absolute() {
            PathBuf::from(&file_path)
        } else {
            media_source_root.join(&file_path)
        };

        if !source.exists() {
            // Try under media_source_root / project_id / filename
            let alt = media_source_root.join(&project_id).join(&filename);
            if !alt.exists() {
                warn!(
                    "[MASTER-STORAGE] Media file missing: {} (tried {} and {})",
                    filename,
                    source.display(),
                    alt.display()
                );
                continue;
            }
            let source = alt;
            let dest_dir = media_dest_root.join(&project_id);
            tokio::fs::create_dir_all(&dest_dir).await?;
            let dest = dest_dir.join(&filename);

            let size = file_size as u64;
            stats.total_files += 1;
            stats.total_bytes += size;

            if should_copy(&source, &dest, None).await {
                info!("[MASTER-STORAGE] Syncing media: {project_id}/{filename}");
                if let Err(e) = tokio::fs::copy(&source, &dest).await {
                    warn!("[MASTER-STORAGE] Failed to copy {}: {e}", source.display());
                } else {
                    stats.files_copied_this_cycle += 1;
                    stats.bytes_copied_this_cycle += size;
                }
            }
            continue;
        }

        let dest_dir = media_dest_root.join(&project_id);
        tokio::fs::create_dir_all(&dest_dir).await?;
        let dest = dest_dir.join(&filename);

        let size = file_size as u64;
        stats.total_files += 1;
        stats.total_bytes += size;

        if should_copy(&source, &dest, None).await {
            info!("[MASTER-STORAGE] Syncing media: {project_id}/{filename}");
            if let Err(e) = tokio::fs::copy(&source, &dest).await {
                warn!("[MASTER-STORAGE] Failed to copy {}: {e}", source.display());
            } else {
                stats.files_copied_this_cycle += 1;
                stats.bytes_copied_this_cycle += size;
            }
        }
    }

    info!(
        "[MASTER-STORAGE] Media: {}/{} files synced ({} bytes)",
        stats.files_copied_this_cycle, stats.total_files, stats.bytes_copied_this_cycle
    );

    Ok(stats)
}

// ============================================================================
// Team-files sync (Dropbox team folder mirror)
// ============================================================================

/// Size threshold (100 MB) above which we skip SHA-256 hashing and fall back
/// to a size + mtime comparison. This avoids reading 85 GB .cine files into
/// memory just to hash them.
const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

async fn sync_team_files(root: &Path) -> anyhow::Result<SyncStats> {
    let source_root = PathBuf::from(
        std::env::var("TEAM_FILES_SOURCE")
            .unwrap_or_else(|_| "E:/topos/Sirak Studios Team".to_string()),
    );
    let dest_root = root.join("team_files");

    if !source_root.exists() {
        warn!(
            "[MASTER-STORAGE] Team files source not found: {}",
            source_root.display()
        );
        return Ok(SyncStats::empty());
    }

    let mut stats = SyncStats::empty();
    let mut files_processed: u64 = 0;

    // Stack-based recursive directory walk
    let mut stack: Vec<PathBuf> = vec![source_root.clone()];

    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => {
                warn!(
                    "[MASTER-STORAGE] Cannot read team dir {}: {e}",
                    dir.display()
                );
                continue;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            // Compute relative path from source_root
            let rel = match path.strip_prefix(&source_root) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let dest = dest_root.join(rel);

            // Ensure parent directory exists
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let meta = match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size = meta.len();

            stats.total_files += 1;
            stats.total_bytes += size;

            // For large files use size+mtime only; for smaller files use the
            // normal hash-based should_copy helper.
            let needs_copy = if size > LARGE_FILE_THRESHOLD {
                should_copy_large(&path, &dest).await
            } else {
                should_copy(&path, &dest, None).await
            };

            if needs_copy {
                info!(
                    "[MASTER-STORAGE] Syncing team file: {}",
                    rel.display()
                );
                if let Err(e) = tokio::fs::copy(&path, &dest).await {
                    warn!(
                        "[MASTER-STORAGE] Failed to copy {}: {e}",
                        path.display()
                    );
                } else {
                    stats.files_copied_this_cycle += 1;
                    stats.bytes_copied_this_cycle += size;
                }
            }

            files_processed += 1;
            if files_processed % 1000 == 0 {
                info!(
                    "[MASTER-STORAGE] Team files progress: {files_processed} files processed, \
                     {} copied so far",
                    stats.files_copied_this_cycle
                );
            }
        }
    }

    info!(
        "[MASTER-STORAGE] Team files: {}/{} files synced ({} bytes copied)",
        stats.files_copied_this_cycle, stats.total_files, stats.bytes_copied_this_cycle
    );

    Ok(stats)
}

// ============================================================================
// Helpers
// ============================================================================

/// Decide whether `source` needs to be copied to `dest`.
///
/// If a `known_hash` (from the DB) is provided, we compare it against the
/// SHA-256 of the destination file. Otherwise we compare source and dest
/// hashes directly.
async fn should_copy(source: &Path, dest: &Path, known_hash: Option<&str>) -> bool {
    if !dest.exists() {
        return true;
    }

    // Fast path: compare file sizes first
    let src_meta = match tokio::fs::metadata(source).await {
        Ok(m) => m,
        Err(_) => return true,
    };
    let dst_meta = match tokio::fs::metadata(dest).await {
        Ok(m) => m,
        Err(_) => return true,
    };

    if src_meta.len() != dst_meta.len() {
        return true;
    }

    // If the DB already stores the file hash, compare against the dest hash
    if let Some(hash) = known_hash {
        match file_sha256(dest).await {
            Ok(dest_hash) => return dest_hash != hash,
            Err(_) => return true,
        }
    }

    // Fall back to comparing source and dest hashes
    match (file_sha256(source).await, file_sha256(dest).await) {
        (Ok(sh), Ok(dh)) => sh != dh,
        _ => true,
    }
}

/// Lightweight copy check for very large files (>100 MB).
///
/// Instead of hashing the entire file we compare size and last-modified time.
/// If either differs (or the dest doesn't exist), we signal a copy is needed.
async fn should_copy_large(source: &Path, dest: &Path) -> bool {
    if !dest.exists() {
        return true;
    }

    let src_meta = match tokio::fs::metadata(source).await {
        Ok(m) => m,
        Err(_) => return true,
    };
    let dst_meta = match tokio::fs::metadata(dest).await {
        Ok(m) => m,
        Err(_) => return true,
    };

    // Different sizes → definitely need to copy
    if src_meta.len() != dst_meta.len() {
        return true;
    }

    // Compare modification times
    let src_mtime = match src_meta.modified() {
        Ok(t) => t,
        Err(_) => return true,
    };
    let dst_mtime = match dst_meta.modified() {
        Ok(t) => t,
        Err(_) => return true,
    };

    // Source is newer than dest → re-copy
    src_mtime > dst_mtime
}

/// Compute the SHA-256 hex digest of a file.
async fn file_sha256(path: &Path) -> anyhow::Result<String> {
    let data = tokio::fs::read(path).await?;
    let hash = Sha256::digest(&data);
    Ok(format!("{:x}", hash))
}

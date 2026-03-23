//! Org Cloud Indexer
//!
//! Scans existing data_sources, execution_artifacts, and physical filesystem
//! volumes to populate the cloud_files index for an organization.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::ApiError;

/// Guess MIME type from file extension
fn mime_from_extension(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("pdf") => "application/pdf",
        Some("doc" | "docx") => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        Some("xls" | "xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("ppt" | "pptx") => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        Some("csv") => "text/csv",
        Some("txt" | "md") => "text/plain",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("html" | "htm") => "text/html",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("avi") => "video/x-msvideo",
        Some("mkv") => "video/x-matroska",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("flac") => "audio/flac",
        Some("aac") => "audio/aac",
        Some("zip") => "application/zip",
        Some("tar" | "gz" | "tgz") => "application/gzip",
        Some("psd") => "image/vnd.adobe.photoshop",
        Some("ai") => "application/postscript",
        Some("vcf") => "text/vcard",
        Some("ini") => "text/plain",
        Some("pages") => "application/x-iwork-pages-sffpages",
        Some("numbers") => "application/x-iwork-numbers-sffnumbers",
        Some("keynote") | Some("key") => "application/x-iwork-keynote-sffkey",
        Some("skp") => "application/x-sketchup",
        Some("fbx") => "model/fbx",
        Some("obj") => "model/obj",
        Some("stl") => "model/stl",
        Some("prproj") => "application/x-premiere-project",
        Some("aep") => "application/x-aftereffects-project",
        Some("indd") => "application/x-indesign",
        Some("ttf" | "otf" | "woff" | "woff2") => "font/ttf",
        Some("lrcat") => "application/x-lightroom-catalog",
        Some("xmp") => "application/rdf+xml",
        Some("dng") => "image/x-adobe-dng",
        Some("cr2" | "cr3") => "image/x-canon-cr2",
        Some("nef") => "image/x-nikon-nef",
        Some("arw") => "image/x-sony-arw",
        Some("raf") => "image/x-fuji-raf",
        Some("heic" | "heif") => "image/heif",
        Some("tif" | "tiff") => "image/tiff",
        Some("bmp") => "image/bmp",
        Some("m4a") => "audio/mp4",
        Some("ogg") => "audio/ogg",
        Some("webm") => "video/webm",
        Some("wmv") => "video/x-ms-wmv",
        Some("mts") => "video/mp2t",
        Some("3gp") => "video/3gpp",
        Some("rar") => "application/x-rar-compressed",
        Some("7z") => "application/x-7z-compressed",
        Some("dmg") => "application/x-apple-diskimage",
        Some("iso") => "application/x-iso-image",
        Some("sql") => "application/sql",
        Some("py") => "text/x-python",
        Some("rs") => "text/x-rust",
        Some("js" | "jsx" | "ts" | "tsx") => "text/javascript",
        Some("css" | "scss") => "text/css",
        Some("yaml" | "yml") => "text/yaml",
        Some("toml") => "text/x-toml",
        Some("sh" | "bash") => "text/x-shellscript",
        _ => "application/octet-stream",
    }
}

/// Index existing data sources, artifacts, and filesystem volumes into cloud_files for an org.
/// Returns the number of newly indexed files.
pub async fn index_existing_data(pool: &SqlitePool, org_id: &str) -> Result<i64, ApiError> {
    let mut indexed: i64 = 0;

    // 1. Index data_sources table
    indexed += index_data_sources(pool, org_id).await?;

    // 2. Index execution_artifacts table (skip if table schema doesn't match)
    match index_execution_artifacts(pool, org_id).await {
        Ok(n) => indexed += n,
        Err(e) => tracing::warn!("[CLOUD_INDEX] Skipping artifacts: {}", e),
    }

    // 3. Scan physical filesystem volumes
    indexed += index_filesystem_volumes(pool, org_id).await?;

    // 4. Sync cloud_files → data_sources so Intelligence tab shows sovereign stack files
    if let Err(e) = sync_cloud_to_data_sources(pool, org_id).await {
        tracing::warn!("[CLOUD_INDEX] data_sources sync failed: {}", e);
    }

    Ok(indexed)
}

/// Populate data_sources from cloud_files so the Intelligence Data Sources view
/// shows sovereign stack files. Uses INSERT OR IGNORE for idempotency.
/// source_type='integration' prevents index_data_sources() from re-indexing these back.
async fn sync_cloud_to_data_sources(pool: &SqlitePool, org_id: &str) -> Result<(), ApiError> {
    let result = sqlx::query(
        r#"INSERT OR IGNORE INTO data_sources
           (id, organization_id, title, file_name, file_path, file_type, file_size_bytes, file_hash,
            data_type, source_type, status, folder, metadata)
        SELECT
            cf.id, cf.organization_id, cf.file_name, cf.file_name, cf.file_path,
            cf.mime_type, cf.file_size_bytes, cf.content_hash,
            CASE
                WHEN cf.mime_type LIKE 'video/%' OR cf.mime_type LIKE 'audio/%' OR cf.mime_type LIKE 'image/%' THEN 'media'
                WHEN cf.mime_type LIKE 'text/%' OR cf.mime_type = 'application/pdf'
                     OR cf.mime_type LIKE 'application/vnd.openxmlformats%' THEN 'document'
                WHEN cf.mime_type IN ('application/json', 'text/csv') THEN 'dataset'
                ELSE 'other'
            END,
            'integration', 'ready',
            CASE cf.storage_volume
                WHEN 'sovereign_personal' THEN 'Personal/' || COALESCE(
                    CASE WHEN instr(cf.file_path, '/') > 0
                         THEN substr(cf.file_path, 1, instr(cf.file_path, '/') - 1)
                         ELSE 'Unfiled' END, 'Unfiled')
                ELSE COALESCE(
                    CASE WHEN instr(cf.file_path, '/') > 0
                         THEN substr(cf.file_path, 1, instr(cf.file_path, '/') - 1)
                         ELSE 'Unfiled' END, 'Unfiled')
            END,
            json_object('storage_volume', cf.storage_volume, 'cloud_file_id', cf.id, 'synced_from_cloud', 1)
        FROM cloud_files cf
        WHERE cf.organization_id = ? AND cf.deleted_at IS NULL
          AND cf.storage_volume IN ('sovereign_org', 'sovereign_personal')
          AND cf.file_path NOT LIKE '%/Cache/%'
          AND cf.file_path NOT LIKE '%/Thumbnails/%'
          AND cf.file_path NOT LIKE '%/Proxies/%'
          AND cf.file_path NOT LIKE '%.lrdata/%'
          AND cf.file_path NOT LIKE '%/CaptureOne/Settings%'
          AND cf.id NOT IN (SELECT id FROM data_sources WHERE organization_id = ?)"#,
    )
    .bind(org_id)
    .bind(org_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("data_sources sync: {}", e)))?;

    let count = result.rows_affected();
    if count > 0 {
        tracing::info!("[CLOUD_INDEX] Synced {} new data_sources from cloud_files", count);
    }
    Ok(())
}

/// Walk a filesystem directory and index all files into cloud_files.
/// Uses relative paths so resolve_volume_path can reconstruct the absolute path.
async fn index_filesystem_volumes(pool: &SqlitePool, org_id: &str) -> Result<i64, ApiError> {
    let volumes = utils::volume::all_sovereign_volumes();
    let mut total: i64 = 0;

    for (name, base_path) in &volumes {
        if !base_path.exists() {
            tracing::debug!(
                "[CLOUD_INDEX] Volume {} not found at {:?}, skipping",
                name,
                base_path
            );
            continue;
        }

        tracing::info!(
            "[CLOUD_INDEX] Scanning volume '{}' at {:?}",
            name,
            base_path
        );
        let count = index_directory(pool, org_id, name, base_path).await?;
        tracing::info!("[CLOUD_INDEX] Indexed {} files from '{}'", count, name);
        total += count;
    }

    Ok(total)
}

/// Iteratively walk a directory tree and index all files.
/// `base` is the volume root. file_path stored is relative to base.
async fn index_directory(
    pool: &SqlitePool,
    org_id: &str,
    volume_name: &str,
    base: &Path,
) -> Result<i64, ApiError> {
    let mut count: i64 = 0;
    let mut stack: Vec<PathBuf> = vec![base.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[CLOUD_INDEX] Cannot read {:?}: {}", dir, e);
                continue;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            // Skip hidden files/dirs and system files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "desktop.ini" || name == "Thumbs.db" {
                    continue;
                }
            }

            if path.is_dir() {
                // Skip cache/preview directories
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    let lower = dir_name.to_lowercase();
                    if lower.ends_with(".lrdata")
                        || lower.ends_with(".lrcat-data")
                        || lower == ".dropbox.cache"
                        || lower == "node_modules"
                        || lower == ".git"
                        || lower == "__pycache__"
                        || lower == "target"
                    {
                        continue;
                    }
                }
                stack.push(path);
            } else if path.is_file() {
                // Get relative path from volume base
                let rel_path = match path.strip_prefix(base) {
                    Ok(r) => r.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };

                // Check if already indexed (by volume + relative path)
                let existing: Option<i64> = sqlx::query_scalar(
                    "SELECT 1 FROM cloud_files WHERE storage_volume = ? AND file_path = ? AND organization_id = ? AND deleted_at IS NULL"
                )
                .bind(volume_name)
                .bind(&rel_path)
                .bind(org_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::InternalError(format!("DB check failed: {}", e)))?;

                if existing.is_some() {
                    continue;
                }

                // Get file metadata
                let metadata = match tokio::fs::metadata(&path).await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let file_size = metadata.len() as i64;
                let mime = mime_from_extension(&path);
                let id = Uuid::new_v4().to_string();

                let result = sqlx::query(
                    r#"INSERT OR IGNORE INTO cloud_files
                        (id, organization_id, file_name, file_path,
                         storage_volume, file_size_bytes, mime_type,
                         source_type, visibility)
                    VALUES (?, ?, ?, ?, ?, ?, ?, 'integration', 'org')"#,
                )
                .bind(&id)
                .bind(org_id)
                .bind(&file_name)
                .bind(&rel_path)
                .bind(volume_name)
                .bind(file_size)
                .bind(mime)
                .execute(pool)
                .await;

                if result.is_ok() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

/// Index data_sources that have files but aren't yet in cloud_files
async fn index_data_sources(pool: &SqlitePool, org_id: &str) -> Result<i64, ApiError> {
    #[derive(sqlx::FromRow)]
    struct DsRow {
        id: String,
        #[allow(dead_code)]
        organization_id: Option<String>,
        project_id: Option<String>,
        file_name: Option<String>,
        file_path: Option<String>,
        file_size_bytes: Option<i64>,
        file_hash: Option<String>,
        file_type: Option<String>,
        created_by: Option<String>,
    }

    let rows: Vec<DsRow> = sqlx::query_as(
        r#"SELECT id, organization_id, project_id, file_name, file_path,
                  file_size_bytes, file_hash, file_type, created_by
           FROM data_sources
           WHERE organization_id = ?
             AND file_path IS NOT NULL
             AND archived_at IS NULL
             AND source_type != 'integration'
             AND id NOT IN (
                SELECT source_id FROM cloud_files
                WHERE source_table = 'data_sources' AND organization_id = ? AND deleted_at IS NULL
             )"#,
    )
    .bind(org_id)
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to query data sources: {}", e)))?;

    let mut count: i64 = 0;
    for row in &rows {
        let file_name = row.file_name.as_deref().unwrap_or("unknown");
        let file_path = match &row.file_path {
            Some(p) => p.clone(),
            None => continue,
        };

        let id = Uuid::new_v4().to_string();
        let mime = row
            .file_type
            .as_deref()
            .unwrap_or("application/octet-stream");

        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO cloud_files
                (id, organization_id, project_id, file_name, file_path,
                 storage_volume, content_hash, file_size_bytes, mime_type,
                 source_type, source_id, source_table, visibility,
                 contributed_by)
            VALUES (?, ?, ?, ?, ?, 'data_sources', ?, ?, ?, 'sync', ?, 'data_sources', 'org', ?)"#,
        )
        .bind(&id)
        .bind(org_id)
        .bind(&row.project_id)
        .bind(file_name)
        .bind(&file_path)
        .bind(&row.file_hash)
        .bind(row.file_size_bytes.unwrap_or(0))
        .bind(mime)
        .bind(&row.id)
        .bind(&row.created_by)
        .execute(pool)
        .await;

        if result.is_ok() {
            count += 1;
        }
    }

    Ok(count)
}

/// Index execution artifacts that have files but aren't yet in cloud_files
async fn index_execution_artifacts(pool: &SqlitePool, org_id: &str) -> Result<i64, ApiError> {
    #[derive(sqlx::FromRow)]
    struct ArtRow {
        id: String,
        #[allow(dead_code)]
        organization_id: String,
        project_id: Option<String>,
        task_id: Option<String>,
        title: Option<String>,
        file_path: Option<String>,
        file_size_bytes: Option<i64>,
        content_type: Option<String>,
    }

    let rows: Vec<ArtRow> = sqlx::query_as(
        r#"SELECT ea.id,
                  ? as organization_id,
                  NULL as project_id,
                  NULL as task_id,
                  ea.title,
                  ea.file_path,
                  NULL as file_size_bytes,
                  NULL as content_type
           FROM execution_artifacts ea
           WHERE ea.file_path IS NOT NULL
             AND ea.id NOT IN (
                SELECT source_id FROM cloud_files
                WHERE source_table = 'execution_artifacts' AND organization_id = ? AND deleted_at IS NULL
             )
           LIMIT 1000"#,
    )
    .bind(org_id)
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to query artifacts: {}", e)))?;

    let mut count: i64 = 0;
    for row in &rows {
        let file_path = match &row.file_path {
            Some(p) => p.clone(),
            None => continue,
        };
        let file_name = row.title.as_deref().unwrap_or("artifact");

        let id = Uuid::new_v4().to_string();
        let mime = row
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");

        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO cloud_files
                (id, organization_id, project_id, task_id, file_name, file_path,
                 storage_volume, file_size_bytes, mime_type,
                 source_type, source_id, source_table, visibility)
            VALUES (?, ?, ?, ?, ?, ?, 'artifacts', ?, ?, 'sync', ?, 'execution_artifacts', 'org')"#,
        )
        .bind(&id)
        .bind(org_id)
        .bind(&row.project_id)
        .bind(&row.task_id)
        .bind(file_name)
        .bind(&file_path)
        .bind(row.file_size_bytes.unwrap_or(0))
        .bind(mime)
        .bind(&row.id)
        .execute(pool)
        .await;

        if result.is_ok() {
            count += 1;
        }
    }

    Ok(count)
}

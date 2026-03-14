//! Shared Storage routes — browse and download files from the master node's
//! storage volumes for organization members.
//!
//! The master node exposes multiple **named volumes** (sovereign backup, Dropbox
//! personal, Dropbox team, media pipeline, etc.).  All routes sit behind
//! `require_auth` so only authenticated org members can access them.
//!
//! GET  /shared-storage/volumes                         — list available volumes
//! GET  /shared-storage/browse?volume=&path=            — list directory entries
//! GET  /shared-storage/download?volume=&path=          — stream a file download
//! GET  /shared-storage/stats?volume=                   — per-volume statistics
//! GET  /shared-storage/search?q=&volume=               — filename search
//! POST /shared-storage/upload?volume=&path=            — upload a file
//! POST /shared-storage/mkdir                           — create a directory
//! DELETE /shared-storage/delete?volume=&path=          — delete a file or empty dir
//! PUT  /shared-storage/rename                          — rename / move an entry

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{header, HeaderValue},
    response::Response,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio_util::io::ReaderStream;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_SEARCH_RESULTS: usize = 50;

// ── Volume registry ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct StorageVolume {
    pub id: String,
    pub name: String,
    pub description: String,
    pub root_path: String,
    pub available: bool,
    pub writable: bool,
}

/// Build the list of storage volumes from env vars / well-known paths.
///
/// Volumes can be overridden via environment variables:
///   VOLUME_SOVEREIGN_ROOT, VOLUME_DROPBOX_PERSONAL_ROOT, etc.
fn get_volumes() -> Vec<StorageVolume> {
    //                   (id, name, description, default_root, writable)
    let defs: Vec<(&str, &str, &str, &str, bool)> = vec![
        (
            "sovereign",
            "Sovereign Storage",
            "Database backups, synced data sources, and media assets",
            "E:/topos/sovereign_storage",
            true,
        ),
        (
            "dropbox-personal",
            "Dropbox (Personal)",
            "Sirak Studios personal Dropbox — key docs, meeting recordings, client content",
            "E:/topos/Sirak Studios (sirak)",
            false,
        ),
        (
            "dropbox-team",
            "Dropbox (Team)",
            "Sirak Studios Team Dropbox — active clients, projects, proposals, resources",
            "E:/topos/Sirak Studios Team",
            false,
        ),
        (
            "media-pipeline",
            "Media Pipeline",
            "Video and audio production assets ingested through the media pipeline",
            "E:/topos/media_pipeline",
            false,
        ),
        (
            "dropbox-ingest",
            "Dropbox Ingest",
            "Raw files ingested from Dropbox shared links",
            "E:/topos/dropbox_ingest",
            true,
        ),
    ];

    defs.into_iter()
        .map(|(id, name, description, default_root, writable)| {
            let env_key = format!(
                "VOLUME_{}_ROOT",
                id.to_uppercase().replace('-', "_")
            );
            let root_path = std::env::var(&env_key).unwrap_or_else(|_| default_root.to_string());
            let available = Path::new(&root_path).is_dir();
            StorageVolume {
                id: id.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                root_path,
                available,
                writable,
            }
        })
        .collect()
}

/// Look up a volume by its ID and return the root path.
fn volume_root(volume_id: &str) -> Result<PathBuf, ApiError> {
    let volumes = get_volumes();
    let vol = volumes
        .iter()
        .find(|v| v.id == volume_id)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Volume '{}' not found. Available: {}",
                volume_id,
                volumes
                    .iter()
                    .map(|v| v.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

    if !vol.available {
        return Err(ApiError::InternalError(format!(
            "Volume '{}' is not available (path does not exist: {})",
            volume_id, vol.root_path
        )));
    }

    Ok(PathBuf::from(&vol.root_path))
}

/// Like `volume_root` but also verifies the volume is writable.
fn volume_root_writable(volume_id: &str) -> Result<PathBuf, ApiError> {
    let volumes = get_volumes();
    let vol = volumes
        .iter()
        .find(|v| v.id == volume_id)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Volume '{}' not found. Available: {}",
                volume_id,
                volumes
                    .iter()
                    .map(|v| v.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

    if !vol.available {
        return Err(ApiError::InternalError(format!(
            "Volume '{}' is not available (path does not exist: {})",
            volume_id, vol.root_path
        )));
    }

    if !vol.writable {
        return Err(ApiError::BadRequest(format!(
            "Volume '{}' is read-only",
            volume_id
        )));
    }

    Ok(PathBuf::from(&vol.root_path))
}

// ── Query types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    pub volume: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub volume: Option<String>,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub volume: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub volume: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub volume: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MkdirRequest {
    pub volume: Option<String>,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub volume: Option<String>,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub volume: Option<String>,
    pub from: String,
    pub to: String,
}

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BrowseResponse {
    pub volume: String,
    pub path: String,
    pub entries: Vec<StorageEntry>,
}

#[derive(Debug, Serialize)]
pub struct StorageEntry {
    pub name: String,
    pub entry_type: String,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub volume: String,
    pub root: String,
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub manifest: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AllStatsResponse {
    pub volumes: Vec<StorageStats>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub volume: String,
    pub name: String,
    pub path: String,
    pub entry_type: String,
    pub size: Option<u64>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve a user-supplied relative path against a volume root and ensure it
/// does not escape. Returns the canonical absolute path on success.
fn resolve_safe_path(root: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    if relative.contains("..") {
        return Err(ApiError::BadRequest(
            "Path traversal is not allowed".into(),
        ));
    }

    let cleaned = relative
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();

    let target = root.join(&cleaned);

    let canonical_root = root.canonicalize().map_err(|e| {
        ApiError::InternalError(format!("Volume root is not accessible: {}", e))
    })?;
    let canonical_target = target.canonicalize().map_err(|_| {
        ApiError::NotFound("Path does not exist".into())
    })?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err(ApiError::BadRequest(
            "Path is outside the volume root".into(),
        ));
    }

    Ok(canonical_target)
}

/// Like `resolve_safe_path` but works for paths that may not exist yet (write
/// operations).  The **parent** directory must already exist; only the final
/// component is allowed to be new.
fn resolve_safe_path_for_write(root: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    if relative.contains("..") {
        return Err(ApiError::BadRequest(
            "Path traversal is not allowed".into(),
        ));
    }

    let cleaned = relative
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();

    let target = root.join(&cleaned);

    let parent = target.parent().ok_or_else(|| {
        ApiError::BadRequest("Invalid path".into())
    })?;

    let canonical_root = root.canonicalize().map_err(|e| {
        ApiError::InternalError(format!("Volume root is not accessible: {}", e))
    })?;
    let canonical_parent = parent.canonicalize().map_err(|_| {
        ApiError::NotFound("Parent directory does not exist".into())
    })?;

    if !canonical_parent.starts_with(&canonical_root) {
        return Err(ApiError::BadRequest(
            "Path is outside the volume root".into(),
        ));
    }

    let file_name = target.file_name().ok_or_else(|| {
        ApiError::BadRequest("Path must include a file or directory name".into())
    })?;

    Ok(canonical_parent.join(file_name))
}

fn system_time_to_utc(st: std::time::SystemTime) -> Option<DateTime<Utc>> {
    st.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()).unwrap_or_default())
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /shared-storage/volumes — list all available storage volumes
async fn list_volumes(
    State(_deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<StorageVolume>>>, ApiError> {
    Ok(Json(ApiResponse::success(get_volumes())))
}

/// GET /shared-storage/browse?volume=<id>&path=<relative>
///
/// If no volume is specified, defaults to "sovereign".
async fn browse(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<BrowseQuery>,
) -> Result<Json<ApiResponse<BrowseResponse>>, ApiError> {
    let vol_id = params.volume.as_deref().unwrap_or("sovereign");
    let root = volume_root(vol_id)?;
    let relative = params.path.unwrap_or_default();

    let dir_path = if relative.is_empty() {
        root.canonicalize().map_err(|e| {
            ApiError::InternalError(format!("Volume root is not accessible: {}", e))
        })?
    } else {
        resolve_safe_path(&root, &relative)?
    };

    if !dir_path.is_dir() {
        return Err(ApiError::BadRequest("Path is not a directory".into()));
    }

    let mut entries = Vec::new();
    let mut read_dir = fs::read_dir(&dir_path).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to read directory: {}", e))
    })?;

    while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
        ApiError::InternalError(format!("Failed to read entry: {}", e))
    })? {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip system files
        if name == "desktop.ini" || name == ".DS_Store" || name.starts_with("~$") {
            continue;
        }

        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_type = if metadata.is_dir() { "directory" } else { "file" };
        let size = if metadata.is_file() { Some(metadata.len()) } else { None };
        let modified = metadata.modified().ok().and_then(system_time_to_utc);

        entries.push(StorageEntry {
            name,
            entry_type: entry_type.to_string(),
            size,
            modified,
        });
    }

    // Sort: directories first, then alphabetical.
    entries.sort_by(|a, b| {
        let type_ord = a.entry_type.cmp(&b.entry_type);
        if type_ord == std::cmp::Ordering::Equal {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else {
            type_ord
        }
    });

    let display_path = if relative.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", relative.trim_start_matches('/'))
    };

    Ok(Json(ApiResponse::success(BrowseResponse {
        volume: vol_id.to_string(),
        path: display_path,
        entries,
    })))
}

/// GET /shared-storage/download?volume=<id>&path=<relative>
async fn download(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
    let vol_id = params.volume.as_deref().unwrap_or("sovereign");
    let root = volume_root(vol_id)?;
    let file_path = resolve_safe_path(&root, &params.path)?;

    if !file_path.is_file() {
        return Err(ApiError::NotFound("File not found".into()));
    }

    let metadata = fs::metadata(&file_path).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to read file metadata: {}", e))
    })?;
    let file_size = metadata.len();

    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mime = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    let file = fs::File::open(&file_path).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to open file: {}", e))
    })?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let disposition = format!("attachment; filename=\"{}\"", file_name);

    let response = Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&mime)
                .unwrap_or(HeaderValue::from_static("application/octet-stream")),
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition)
                .unwrap_or(HeaderValue::from_static("attachment")),
        )
        .header(header::CONTENT_LENGTH, file_size)
        .body(body)
        .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// GET /shared-storage/stats?volume=<id>
///
/// If no volume is specified, returns stats for all volumes.
async fn stats(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<ApiResponse<AllStatsResponse>>, ApiError> {
    let volumes = get_volumes();

    let target_volumes: Vec<&StorageVolume> = if let Some(ref vol_id) = params.volume {
        volumes.iter().filter(|v| v.id == *vol_id).collect()
    } else {
        volumes.iter().filter(|v| v.available).collect()
    };

    let mut stats_list = Vec::new();

    for vol in target_volumes {
        let root = PathBuf::from(&vol.root_path);
        let canonical = match root.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Read manifest.json if present (only sovereign volume has one).
        let manifest_path = canonical.join("manifest.json");
        let manifest = if manifest_path.is_file() {
            let content = fs::read_to_string(&manifest_path).await.ok();
            content.and_then(|c| serde_json::from_str(&c).ok())
        } else {
            None
        };

        let (total_files, total_size) = walk_stats(&canonical).await;

        stats_list.push(StorageStats {
            volume: vol.id.clone(),
            root: canonical.to_string_lossy().to_string(),
            total_files,
            total_size_bytes: total_size,
            manifest,
        });
    }

    Ok(Json(ApiResponse::success(AllStatsResponse {
        volumes: stats_list,
    })))
}

/// Recursively compute file count and total size.
async fn walk_stats(dir: &Path) -> (u64, u64) {
    let mut files: u64 = 0;
    let mut size: u64 = 0;

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let mut read_dir = match fs::read_dir(&current).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let meta = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                files += 1;
                size += meta.len();
            }
        }
    }

    (files, size)
}

/// GET /shared-storage/search?q=<query>&volume=<id>
///
/// If no volume is specified, searches across all available volumes.
async fn search(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, ApiError> {
    let query = params.q.trim().to_string();
    if query.is_empty() {
        return Err(ApiError::BadRequest("Search query must not be empty".into()));
    }

    let volumes = get_volumes();
    let target_volumes: Vec<&StorageVolume> = if let Some(ref vol_id) = params.volume {
        volumes.iter().filter(|v| v.id == *vol_id && v.available).collect()
    } else {
        volumes.iter().filter(|v| v.available).collect()
    };

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for vol in target_volumes {
        if results.len() >= MAX_SEARCH_RESULTS {
            break;
        }

        let root = PathBuf::from(&vol.root_path);
        let canonical_root = match root.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let mut stack = vec![canonical_root.clone()];
        while let Some(current) = stack.pop() {
            if results.len() >= MAX_SEARCH_RESULTS {
                break;
            }

            let mut read_dir = match fs::read_dir(&current).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                if results.len() >= MAX_SEARCH_RESULTS {
                    break;
                }

                let name = entry.file_name().to_string_lossy().to_string();
                let meta = match entry.metadata().await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if meta.is_dir() {
                    stack.push(entry.path());
                }

                if name.to_lowercase().contains(&query_lower) {
                    let full = entry.path();
                    let rel = full
                        .strip_prefix(&canonical_root)
                        .unwrap_or(&full)
                        .to_string_lossy()
                        .replace('\\', "/");

                    let entry_type = if meta.is_dir() { "directory" } else { "file" };
                    let size = if meta.is_file() { Some(meta.len()) } else { None };

                    results.push(SearchResult {
                        volume: vol.id.clone(),
                        name,
                        path: format!("/{}", rel.trim_start_matches('/')),
                        entry_type: entry_type.to_string(),
                        size,
                    });
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(results)))
}

// ── Write handlers ───────────────────────────────────────────────────────────

/// POST /shared-storage/upload?volume=<id>&path=<relative-dir>
///
/// Upload a file via multipart form data. The `path` query param specifies the
/// destination directory (defaults to volume root).
async fn upload_file(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<StorageEntry>>, ApiError> {
    let vol_id = params.volume.as_deref().unwrap_or("sovereign");
    let root = volume_root_writable(vol_id)?;

    let dest_dir_relative = params.path.unwrap_or_default();

    // Validate destination directory exists.
    let dest_dir = if dest_dir_relative.is_empty() {
        root.canonicalize().map_err(|e| {
            ApiError::InternalError(format!("Volume root is not accessible: {}", e))
        })?
    } else {
        let resolved = resolve_safe_path(&root, &dest_dir_relative)?;
        if !resolved.is_dir() {
            return Err(ApiError::BadRequest(
                "Destination path is not a directory".into(),
            ));
        }
        resolved
    };

    // Extract the file field from multipart.
    let field = loop {
        match multipart.next_field().await.map_err(|e| {
            ApiError::BadRequest(format!("Invalid multipart data: {}", e))
        })? {
            Some(f) => {
                if f.name() == Some("file") {
                    break f;
                }
                // Skip non-"file" fields.
            }
            None => {
                return Err(ApiError::BadRequest(
                    "Missing 'file' field in multipart body".into(),
                ));
            }
        }
    };

    let file_name = field
        .file_name()
        .map(|s| s.to_string())
        .or_else(|| field.name().map(|s| s.to_string()))
        .unwrap_or_else(|| "upload".to_string());

    if file_name.is_empty() || file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(ApiError::BadRequest("Invalid file name".into()));
    }

    let data = field.bytes().await.map_err(|e| {
        ApiError::InternalError(format!("Failed to read upload data: {}", e))
    })?;

    // Build safe write path: dest_dir is already canonical and validated.
    let relative_for_write = if dest_dir_relative.is_empty() {
        file_name.clone()
    } else {
        format!(
            "{}/{}",
            dest_dir_relative.trim_start_matches('/').trim_end_matches('/'),
            file_name
        )
    };
    let write_path = resolve_safe_path_for_write(&root, &relative_for_write)?;

    fs::write(&write_path, &data).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to write file: {}", e))
    })?;

    let metadata = fs::metadata(&write_path).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to read file metadata: {}", e))
    })?;

    let modified = metadata.modified().ok().and_then(system_time_to_utc);

    Ok(Json(ApiResponse::success(StorageEntry {
        name: file_name,
        entry_type: "file".to_string(),
        size: Some(metadata.len()),
        modified,
    })))
}

/// POST /shared-storage/mkdir — create a single directory
async fn create_directory(
    State(_deployment): State<DeploymentImpl>,
    Json(body): Json<MkdirRequest>,
) -> Result<Json<ApiResponse<StorageEntry>>, ApiError> {
    let vol_id = body.volume.as_deref().unwrap_or("sovereign");
    let root = volume_root_writable(vol_id)?;

    let target = resolve_safe_path_for_write(&root, &body.path)?;

    fs::create_dir(&target).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to create directory: {}", e))
    })?;

    let metadata = fs::metadata(&target).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to read directory metadata: {}", e))
    })?;

    let name = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let modified = metadata.modified().ok().and_then(system_time_to_utc);

    Ok(Json(ApiResponse::success(StorageEntry {
        name,
        entry_type: "directory".to_string(),
        size: None,
        modified,
    })))
}

/// DELETE /shared-storage/delete?volume=<id>&path=<relative>
///
/// Deletes a file or an **empty** directory. Non-empty directories are rejected
/// for safety (no recursive delete).
async fn delete_entry(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<DeleteQuery>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let vol_id = params.volume.as_deref().unwrap_or("sovereign");
    let root = volume_root_writable(vol_id)?;
    let target = resolve_safe_path(&root, &params.path)?;

    let metadata = fs::metadata(&target).await.map_err(|e| {
        ApiError::NotFound(format!("Path not found: {}", e))
    })?;

    if metadata.is_file() {
        fs::remove_file(&target).await.map_err(|e| {
            ApiError::InternalError(format!("Failed to delete file: {}", e))
        })?;
    } else if metadata.is_dir() {
        fs::remove_dir(&target).await.map_err(|e| {
            ApiError::BadRequest(format!(
                "Failed to delete directory (must be empty): {}",
                e
            ))
        })?;
    } else {
        return Err(ApiError::BadRequest("Unsupported entry type".into()));
    }

    Ok(Json(ApiResponse::success(())))
}

/// PUT /shared-storage/rename — rename or move an entry within the same volume
async fn rename_entry(
    State(_deployment): State<DeploymentImpl>,
    Json(body): Json<RenameRequest>,
) -> Result<Json<ApiResponse<StorageEntry>>, ApiError> {
    let vol_id = body.volume.as_deref().unwrap_or("sovereign");
    let root = volume_root_writable(vol_id)?;

    let from = resolve_safe_path(&root, &body.from)?;
    let to = resolve_safe_path_for_write(&root, &body.to)?;

    if to.exists() {
        return Err(ApiError::BadRequest(
            "Destination already exists".into(),
        ));
    }

    let from_meta = fs::metadata(&from).await.map_err(|e| {
        ApiError::NotFound(format!("Source path not found: {}", e))
    })?;

    fs::rename(&from, &to).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to rename: {}", e))
    })?;

    let entry_type = if from_meta.is_dir() { "directory" } else { "file" };
    let size = if from_meta.is_file() { Some(from_meta.len()) } else { None };
    let modified = from_meta.modified().ok().and_then(system_time_to_utc);
    let name = to
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    Ok(Json(ApiResponse::success(StorageEntry {
        name,
        entry_type: entry_type.to_string(),
        size,
        modified,
    })))
}

// ── Router ───────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/shared-storage/volumes", get(list_volumes))
        .route("/shared-storage/browse", get(browse))
        .route("/shared-storage/download", get(download))
        .route("/shared-storage/stats", get(stats))
        .route("/shared-storage/search", get(search))
        .route(
            "/shared-storage/upload",
            post(upload_file).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route("/shared-storage/mkdir", post(create_directory))
        .route("/shared-storage/delete", delete(delete_entry))
        .route("/shared-storage/rename", put(rename_entry))
}

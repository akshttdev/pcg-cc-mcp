//! Media Library routes — native DAM (Shade-equivalent).
//!
//! POST /projects/{project_id}/media            — multipart upload → save → analyze
//! GET  /projects/{project_id}/media            — list assets
//! GET  /projects/{project_id}/media/search?q= — FTS5 search
//! POST /projects/{project_id}/media/import-dir — index a directory of files on disk
//! GET  /media/{id}                             — single asset
//! DELETE /media/{id}                           — delete
//! POST /media/{id}/analyze                     — re-trigger analysis
//! POST /media/backfill                         — import render_deliverable execution artifacts

use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::media_asset::{CreateMediaAsset, MediaAsset};
use services::services::editron::asset_intelligence;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /projects/{project_id}/media
async fn list_assets(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<MediaAsset>>>, ApiError> {
    let limit = q.limit.unwrap_or(100);
    let assets = MediaAsset::find_by_project(&d.db().pool, project_id, limit).await?;
    Ok(Json(ApiResponse::success(assets)))
}

/// GET /projects/{project_id}/media/search?q=
async fn search_assets(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<ApiResponse<Vec<MediaAsset>>>, ApiError> {
    let query_str = q.q.unwrap_or_default();
    let limit = q.limit.unwrap_or(50);

    if query_str.trim().is_empty() {
        let assets = MediaAsset::find_by_project(&d.db().pool, project_id, limit).await?;
        return Ok(Json(ApiResponse::success(assets)));
    }

    let assets = MediaAsset::search(&d.db().pool, project_id, &query_str, limit).await?;
    Ok(Json(ApiResponse::success(assets)))
}

/// POST /projects/{project_id}/media — multipart upload
async fn upload_asset(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<MediaAsset>>, ApiError> {
    let media_root = std::env::var("MEDIA_ROOT")
        .unwrap_or_else(|_| "/home/pythia/pcg-cc-mcp/dev_assets/media".into());

    let project_dir = PathBuf::from(&media_root).join(project_id.to_string());
    tokio::fs::create_dir_all(&project_dir)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Cannot create media dir: {}", e)))?;

    let mut filename = String::new();
    let mut file_path_str = String::new();
    let mut file_size: i64 = 0;
    let mut mime_type = "application/octet-stream".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            let orig_name = field
                .file_name()
                .unwrap_or("upload")
                .to_string();
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            mime_type = content_type;

            let asset_uuid = Uuid::new_v4();
            let safe_name = orig_name.replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '-', "_");
            filename = format!("{}_{}", asset_uuid.simple(), safe_name);
            let dest = project_dir.join(&filename);

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            file_size = data.len() as i64;

            let mut f = tokio::fs::File::create(&dest)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Cannot write file: {}", e)))?;
            f.write_all(&data)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Write error: {}", e)))?;

            file_path_str = dest.to_string_lossy().into_owned();
        }
    }

    if file_path_str.is_empty() {
        return Err(ApiError::BadRequest("No file field in upload".into()));
    }

    let asset = MediaAsset::create(
        &d.db().pool,
        CreateMediaAsset {
            project_id,
            batch_id: None,
            filename: filename.clone(),
            file_path: file_path_str.clone(),
            file_size_bytes: Some(file_size),
            mime_type: Some(mime_type),
            duration_seconds: None,
            width: None,
            height: None,
        },
    )
    .await?;

    // Fire non-blocking analysis
    asset_intelligence::analyze_async(
        d.db().pool.clone(),
        asset.id,
        file_path_str,
        project_id,
        filename,
    );

    Ok(Json(ApiResponse::success(asset)))
}

/// GET /media/{id}
async fn get_asset(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MediaAsset>>, ApiError> {
    MediaAsset::find_by_id(&d.db().pool, id)
        .await?
        .map(|a| Json(ApiResponse::success(a)))
        .ok_or_else(|| ApiError::NotFound("Asset not found".into()))
}

/// DELETE /media/{id}
async fn delete_asset(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let deleted = MediaAsset::delete(&d.db().pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound("Asset not found".into()))
    }
}

/// POST /media/{id}/analyze — re-trigger analysis
async fn retrigger_analysis(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MediaAsset>>, ApiError> {
    let asset = MediaAsset::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Asset not found".into()))?;

    asset_intelligence::analyze_async(
        d.db().pool.clone(),
        asset.id,
        asset.file_path.clone(),
        asset.project_id,
        asset.filename.clone(),
    );

    Ok(Json(ApiResponse::success(asset)))
}

// ── Backfill / Import ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub assets: Vec<Uuid>,
}

/// POST /media/backfill
/// Scans `execution_artifacts` of type `render_deliverable` and imports each
/// referenced MP4 file into `media_assets`, then fires Vision AI analysis.
/// Idempotent — skips files already present in `media_assets` by `file_path`.
async fn backfill_from_artifacts(
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<ImportResult>>, ApiError> {
    let pool = &d.db().pool;
    let asset_base = utils::assets::asset_dir();

    #[derive(sqlx::FromRow)]
    struct RawArtifact {
        id: Uuid,
        project_id: Option<Uuid>,
        content: Option<String>,
        file_path: Option<String>,
    }

    // Get render_deliverable artifacts joined to their project via task_artifacts → tasks
    let artifacts: Vec<RawArtifact> = sqlx::query_as(
        r#"SELECT ea.id, t.project_id, ea.content, ea.file_path
           FROM execution_artifacts ea
           JOIN task_artifacts ta ON ta.artifact_id = ea.id
           JOIN tasks t ON t.id = ta.task_id
           WHERE ea.artifact_type = 'render_deliverable'"#,
    )
    .fetch_all(pool)
    .await?;

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let mut asset_ids: Vec<Uuid> = Vec::new();

    for artifact in artifacts {
        let project_id = match artifact.project_id {
            Some(p) => p,
            None => {
                skipped += 1;
                continue;
            }
        };

        // Parse the deliverables list from content JSON
        let content: serde_json::Value = artifact
            .content
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Null);

        let deliverables = match content.get("deliverables").and_then(|v| v.as_array()) {
            Some(d) => d.clone(),
            None => {
                // No deliverables array — try the artifact file_path directly
                if let Some(fp) = &artifact.file_path {
                    vec![serde_json::json!({"path": fp, "file": fp})]
                } else {
                    skipped += 1;
                    continue;
                }
            }
        };

        for entry in deliverables {
            let rel_path = entry
                .get("path")
                .or_else(|| entry.get("file"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if rel_path.is_empty() {
                continue;
            }

            // Resolve: absolute path wins, otherwise relative to asset_dir
            let abs_path = if std::path::Path::new(rel_path).is_absolute() {
                PathBuf::from(rel_path)
            } else {
                asset_base.join(rel_path)
            };

            if !abs_path.exists() {
                errors.push(format!("File not found: {}", abs_path.display()));
                continue;
            }

            let file_path_str = abs_path.to_string_lossy().into_owned();

            // Idempotency check
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM media_assets WHERE file_path = ?)",
            )
            .bind(&file_path_str)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

            if exists {
                skipped += 1;
                continue;
            }

            let filename = abs_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| rel_path.to_string());

            let file_size = std::fs::metadata(&abs_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            let duration = entry.get("duration_seconds").and_then(|v| v.as_f64());
            let mime = if filename.ends_with(".mp4") {
                "video/mp4"
            } else if filename.ends_with(".mov") {
                "video/quicktime"
            } else {
                "video/mp4"
            };

            match MediaAsset::create(
                pool,
                CreateMediaAsset {
                    project_id,
                    batch_id: None,
                    filename: filename.clone(),
                    file_path: file_path_str.clone(),
                    file_size_bytes: Some(file_size),
                    mime_type: Some(mime.into()),
                    duration_seconds: duration,
                    width: entry.get("width").and_then(|v| v.as_i64()),
                    height: entry.get("height").and_then(|v| v.as_i64()),
                },
            )
            .await
            {
                Ok(asset) => {
                    asset_intelligence::analyze_async(
                        pool.clone(),
                        asset.id,
                        file_path_str,
                        project_id,
                        filename,
                    );
                    asset_ids.push(asset.id);
                    imported += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", rel_path, e));
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(ImportResult {
        imported,
        skipped,
        errors,
        assets: asset_ids,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ImportDirBody {
    pub path: String,
}

/// POST /projects/{project_id}/media/import-dir
/// Walks a directory on disk and imports all video/image files into media_assets.
/// Idempotent — skips files already indexed by file_path.
async fn import_directory(
    State(d): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(body): Json<ImportDirBody>,
) -> Result<Json<ApiResponse<ImportResult>>, ApiError> {
    let pool = &d.db().pool;
    let dir = PathBuf::from(&body.path);

    if !dir.exists() || !dir.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Directory not found: {}",
            body.path
        )));
    }

    const SUPPORTED: &[&str] = &["mp4", "mov", "avi", "mkv", "webm", "jpg", "jpeg", "png", "gif"];

    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Cannot read dir: {}", e)))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let path = entry.path();
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            if SUPPORTED.contains(&ext.as_str()) {
                entries.push(path);
            }
        }
    }

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let mut asset_ids: Vec<Uuid> = Vec::new();

    for path in entries {
        let file_path_str = path.to_string_lossy().into_owned();

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_assets WHERE file_path = ?)")
                .bind(&file_path_str)
                .fetch_one(pool)
                .await
                .unwrap_or(false);

        if exists {
            skipped += 1;
            continue;
        }

        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let file_size = std::fs::metadata(&path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let mime = match ext.as_str() {
            "mp4" => "video/mp4",
            "mov" => "video/quicktime",
            "avi" => "video/x-msvideo",
            "mkv" => "video/x-matroska",
            "webm" => "video/webm",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            _ => "application/octet-stream",
        };

        match MediaAsset::create(
            pool,
            CreateMediaAsset {
                project_id,
                batch_id: None,
                filename: filename.clone(),
                file_path: file_path_str.clone(),
                file_size_bytes: Some(file_size),
                mime_type: Some(mime.into()),
                duration_seconds: None,
                width: None,
                height: None,
            },
        )
        .await
        {
            Ok(asset) => {
                asset_intelligence::analyze_async(
                    pool.clone(),
                    asset.id,
                    file_path_str,
                    project_id,
                    filename,
                );
                asset_ids.push(asset.id);
                imported += 1;
            }
            Err(e) => {
                errors.push(format!("{}: {}", filename, e));
            }
        }
    }

    Ok(Json(ApiResponse::success(ImportResult {
        imported,
        skipped,
        errors,
        assets: asset_ids,
    })))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/projects/{project_id}/media",
            get(list_assets).post(upload_asset),
        )
        .route("/projects/{project_id}/media/search", get(search_assets))
        .route(
            "/projects/{project_id}/media/import-dir",
            post(import_directory),
        )
        .route("/media/backfill", post(backfill_from_artifacts))
        .route("/media/{id}", get(get_asset).delete(delete_asset))
        .route("/media/{id}/analyze", post(retrigger_analysis))
        .with_state(deployment.clone())
}

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Request, State},
    http::{StatusCode, header},
    response::Response,
    routing::{get, post},
};
use db::models::execution_artifact::ExecutionArtifact;
use deployment::Deployment;
use serde_json::json;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};
use tokio_util::io::ReaderStream;
use utils::assets::asset_dir;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Resolve a file_path from the database.
/// If it's a relative path, resolve it against the asset directory (dev_assets/).
/// If it's absolute, use it as-is.
fn resolve_artifact_path(file_path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        asset_dir().join(file_path)
    }
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/artifacts/{artifact_id}/content",
            get(get_artifact_content),
        )
        .route("/artifacts/{artifact_id}/download", get(download_artifact))
        .route(
            "/artifacts/{artifact_id}/files/{*filename}",
            get(get_artifact_file),
        )
        .route(
            "/artifacts/{artifact_id}/upload",
            post(upload_artifact_file).layer(DefaultBodyLimit::max(100 * 1024 * 1024)), // 100MB
        )
        .with_state(deployment.clone())
}

/// POST /api/artifacts/{artifact_id}/upload
/// Accepts a multipart file upload and stores it in the artifact's directory.
/// Updates file_path on the artifact record.
async fn upload_artifact_file(
    State(deployment): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;

    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    // Artifact storage directory: dev_assets/artifacts/{id}/
    let artifact_dir = asset_dir().join("artifacts").join(artifact_id.to_string());
    tokio::fs::create_dir_all(&artifact_dir)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create artifact dir: {e}")))?;

    let mut saved_filename: Option<String> = None;
    let mut saved_bytes: u64 = 0;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".to_string());

        // Sanitise
        let safe_name = std::path::Path::new(&filename)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        let dest = artifact_dir.join(&safe_name);
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to read upload: {e}")))?;

        saved_bytes = data.len() as u64;
        let mut f = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to write file: {e}")))?;
        f.write_all(&data)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to write file: {e}")))?;

        saved_filename = Some(safe_name);
        break; // only first file
    }

    let filename = saved_filename.ok_or_else(|| ApiError::BadRequest("No file received".into()))?;

    // Relative path for DB storage
    let relative_path = format!("artifacts/{}/{}", artifact_id, filename);

    // Update the artifact's file_path
    sqlx::query("UPDATE execution_artifacts SET file_path = ? WHERE id = ?")
        .bind(&relative_path)
        .bind(artifact.id)
        .execute(pool)
        .await?;

    Ok(Json(json!({
        "success": true,
        "file_path": relative_path,
        "filename": filename,
        "size_bytes": saved_bytes
    })))
}

/// GET /api/artifacts/{artifact_id}/content
/// Returns artifact content as JSON, streams files, or lists directories.
async fn get_artifact_content(
    State(deployment): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    serve_artifact_content(&artifact, false).await
}

/// GET /api/artifacts/{artifact_id}/download
/// Same as content but with Content-Disposition: attachment header.
async fn download_artifact(
    State(deployment): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    serve_artifact_content(&artifact, true).await
}

/// GET /api/artifacts/{artifact_id}/files/{filename}
/// Serves an individual file from the artifact's directory.
/// Supports Range requests for video seeking.
async fn get_artifact_file(
    State(deployment): State<DeploymentImpl>,
    Path((artifact_id, filename)): Path<(Uuid, String)>,
    req: Request,
) -> Result<Response, ApiError> {
    // Only allow simple filenames (no path traversal)
    let basename = std::path::Path::new(&filename)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if basename.is_empty() || basename.contains("..") {
        return Err(ApiError::BadRequest("Invalid filename".into()));
    }

    let pool = &deployment.db().pool;
    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    let file_path = artifact
        .file_path
        .as_deref()
        .ok_or_else(|| ApiError::NotFound("Artifact has no file path".into()))?;

    if file_path.contains("..") {
        return Err(ApiError::BadRequest("Invalid artifact file path".into()));
    }

    let dir = resolve_artifact_path(file_path);
    let target = if dir.is_dir() {
        dir.join(&basename)
    } else {
        let base_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if base_name != basename {
            return Err(ApiError::NotFound("File not found".into()));
        }
        dir.to_path_buf()
    };

    if !target.is_file() {
        return Err(ApiError::NotFound(format!("File '{}' not found", basename)));
    }

    let content_type = mime_guess::from_path(&target)
        .first_or_octet_stream()
        .to_string();

    let file_size = tokio::fs::metadata(&target).await?.len();

    // Parse Range header
    let range_header = req
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("bytes="))
        .and_then(|s| {
            let mut parts = s.splitn(2, '-');
            let start: u64 = parts.next()?.parse().ok()?;
            let end: u64 = parts
                .next()
                .and_then(|e| if e.is_empty() { None } else { e.parse().ok() })
                .unwrap_or(file_size.saturating_sub(1));
            Some((start, end))
        });

    if let Some((start, end)) = range_header {
        let end = end.min(file_size.saturating_sub(1));
        if start > end || start >= file_size {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{}", file_size))
                .body(Body::empty())
                .map_err(|e| ApiError::InternalError(e.to_string()));
        }
        let length = end - start + 1;
        let mut file = File::open(&target).await?;
        file.seek(std::io::SeekFrom::Start(start)).await?;
        let stream = ReaderStream::new(file.take(length));
        Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, &content_type)
            .header(header::CONTENT_LENGTH, length)
            .header(
                header::CONTENT_RANGE,
                format!("bytes {}-{}/{}", start, end, file_size),
            )
            .header(header::ACCEPT_RANGES, "bytes")
            .body(Body::from_stream(stream))
            .map_err(|e| ApiError::InternalError(e.to_string()))
    } else {
        let file = File::open(&target).await?;
        let stream = ReaderStream::new(file);
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, &content_type)
            .header(header::CONTENT_LENGTH, file_size)
            .header(header::ACCEPT_RANGES, "bytes")
            .body(Body::from_stream(stream))
            .map_err(|e| ApiError::InternalError(e.to_string()))
    }
}

async fn serve_artifact_content(
    artifact: &ExecutionArtifact,
    as_download: bool,
) -> Result<Response, ApiError> {
    // If file_path points to a regular file, stream it directly
    if let Some(ref file_path) = artifact.file_path {
        // Security: reject path traversal
        if file_path.contains("..") {
            return Err(ApiError::BadRequest("Invalid file path".into()));
        }

        let path = resolve_artifact_path(file_path);

        if path.is_file() {
            let file = File::open(&path).await?;
            let metadata = file.metadata().await?;
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            let content_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();

            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}.bin", artifact.id));

            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &content_type)
                .header(header::CONTENT_LENGTH, metadata.len());

            if as_download {
                builder = builder.header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                );
            }

            return builder
                .body(body)
                .map_err(|e| ApiError::InternalError(e.to_string()));
        }
        // file_path is a directory or doesn't exist — fall through to content
    }

    // Serve the JSON content field (preferred for artifacts with structured data)
    if let Some(ref content) = artifact.content {
        let filename = format!(
            "{}-{}.json",
            artifact.artifact_type,
            &artifact.id.to_string()[..8]
        );

        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json");

        if as_download {
            builder = builder.header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            );
        }

        return builder
            .body(Body::from(content.clone()))
            .map_err(|e| ApiError::InternalError(e.to_string()));
    }

    // Last resort: if file_path is a directory, list its contents
    if let Some(ref file_path) = artifact.file_path {
        let path = resolve_artifact_path(file_path);
        if path.is_dir() {
            let mut entries = Vec::new();
            let mut read_dir = tokio::fs::read_dir(&path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let meta = entry.metadata().await?;
                entries.push(json!({
                    "name": entry.file_name().to_string_lossy(),
                    "is_file": meta.is_file(),
                    "size_bytes": meta.len(),
                }));
            }

            let body = serde_json::to_string(&json!({
                "type": "directory",
                "path": file_path,
                "entries": entries,
            }))
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

            let dirname = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "listing".to_string());

            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json");

            if as_download {
                builder = builder.header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}-listing.json\"", dirname),
                );
            }

            return builder
                .body(Body::from(body))
                .map_err(|e| ApiError::InternalError(e.to_string()));
        }
    }

    Err(ApiError::NotFound("Artifact has no content or file".into()))
}

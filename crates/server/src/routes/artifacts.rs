use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use db::models::execution_artifact::ExecutionArtifact;
use deployment::Deployment;
use serde_json::json;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/artifacts/{artifact_id}/content", get(get_artifact_content))
        .route("/artifacts/{artifact_id}/download", get(download_artifact))
        .route(
            "/artifacts/{artifact_id}/files/{filename}",
            get(get_artifact_file),
        )
        .with_state(deployment.clone())
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
) -> Result<Response, ApiError> {
    // Security: reject path traversal in filename
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
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

    let dir = std::path::Path::new(file_path);
    let target = if dir.is_dir() {
        dir.join(&filename)
    } else {
        // file_path is the file itself — only serve if filename matches
        let base_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if base_name != filename {
            return Err(ApiError::NotFound("File not found".into()));
        }
        dir.to_path_buf()
    };

    if !target.is_file() {
        return Err(ApiError::NotFound(format!("File '{}' not found", filename)));
    }

    let file = File::open(&target).await?;
    let metadata = file.metadata().await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_type = mime_guess::from_path(&target)
        .first_or_octet_stream()
        .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &content_type)
        .header(header::CONTENT_LENGTH, metadata.len())
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|e| ApiError::InternalError(e.to_string()))
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

        let path = std::path::Path::new(file_path);

        if path.is_file() {
            let file = File::open(path).await?;
            let metadata = file.metadata().await?;
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            let content_type = mime_guess::from_path(path)
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
        let path = std::path::Path::new(file_path);
        if path.is_dir() {
            let mut entries = Vec::new();
            let mut read_dir = tokio::fs::read_dir(path).await?;
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

    Err(ApiError::NotFound(
        "Artifact has no content or file".into(),
    ))
}

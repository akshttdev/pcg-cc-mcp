use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::data_source::{metadata_template, CreateDataSource, DataSource, UpdateDataSource},
};
use deployment::Deployment;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ── List endpoints ──────────────────────────────────────────────────────────

/// GET /api/projects/:project_id/data-sources
async fn list_by_project(
    Path(project_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<DataSource>>>, ApiError> {
    let pool = &deployment.db().pool;
    let sources = DataSource::find_by_project(pool, &project_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list data sources: {e}")))?;
    Ok(Json(ApiResponse::success(sources)))
}

// ── CRUD ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateDataSourceRequest {
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub data_type: String,
    /// "file", "text", or "integration" — defaults to "text" for JSON create
    pub source_type: Option<String>,
    pub file_type: Option<String>,
    /// Raw text content (for source_type = "text")
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub folder: Option<String>,
}

/// POST /api/data-sources
async fn create_data_source(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateDataSourceRequest>,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;

    let valid_types = [
        "conversation",
        "document",
        "transcript",
        "report",
        "dataset",
        "media",
        "other",
    ];
    if !valid_types.contains(&body.data_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid data_type '{}'. Must be one of: {}",
            body.data_type,
            valid_types.join(", ")
        )));
    }

    let metadata_str = body
        .metadata
        .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "{}".to_string());

    let source_type = body.source_type.unwrap_or_else(|| "text".to_string());
    let valid_source_types = ["file", "text", "integration"];
    if !valid_source_types.contains(&source_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid source_type '{}'. Must be one of: {}",
            source_type,
            valid_source_types.join(", ")
        )));
    }

    let source = DataSource::create(
        pool,
        CreateDataSource {
            organization_id: body.organization_id.map(|u| u.to_string()),
            project_id: body.project_id.map(|u| u.to_string()),
            created_by: None, // TODO: extract from auth context
            title: body.title,
            description: body.description,
            data_type: body.data_type,
            source_type: Some(source_type),
            file_type: body.file_type,
            content: body.content,
            file_name: None,
            file_path: None,
            file_size_bytes: None,
            file_hash: None,
            metadata: Some(metadata_str),
            folder: body.folder,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create data source: {e}")))?;

    // For text sources, mark as ready immediately
    let source = if source.source_type == "text" {
        DataSource::update(
            pool,
            &source.id,
            UpdateDataSource {
                title: None,
                description: None,
                data_type: None,
                source_type: None,
                content: None,
                metadata: None,
                status: Some("ready".to_string()),
                processing_error: None,
                folder: None,
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .unwrap_or(source)
    } else {
        source
    };

    // Fire any matching workflow triggers in the background
    let trigger_pool = pool.clone();
    let trigger_ds_id = source.id.clone();
    let trigger_dep = deployment.clone();
    tokio::spawn(async move {
        super::data_source_workflows::fire_triggers_for_data_source(
            trigger_pool,
            trigger_ds_id,
            trigger_dep,
        )
        .await;
    });

    Ok(Json(ApiResponse::success(source)))
}

/// POST /api/data-sources/upload — multipart form with file + metadata fields
async fn upload_data_source(
    State(deployment): State<DeploymentImpl>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;

    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut data_type: Option<String> = None;
    let mut file_type_field: Option<String> = None;
    let mut organization_id: Option<Uuid> = None;
    let mut project_id: Option<Uuid> = None;
    let mut metadata_str: Option<String> = None;
    let mut folder_field: Option<String> = None;
    let mut file_data: Option<(String, Vec<u8>)> = None; // (filename, bytes)

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "upload".to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {e}")))?;
                file_data = Some((filename, bytes.to_vec()));
            }
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("{e}")))?,
                );
            }
            "description" => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("{e}")))?,
                );
            }
            "data_type" => {
                data_type = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("{e}")))?,
                );
            }
            "file_type" => {
                file_type_field = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("{e}")))?,
                );
            }
            "organization_id" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("{e}")))?;
                organization_id = Some(
                    DbUuid::parse(&val)
                        .map_err(|e| ApiError::BadRequest(format!("Invalid org ID: {e}")))?
                        .to_uuid(),
                );
            }
            "project_id" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("{e}")))?;
                project_id = Some(
                    DbUuid::parse(&val)
                        .map_err(|e| ApiError::BadRequest(format!("Invalid project ID: {e}")))?
                        .to_uuid(),
                );
            }
            "metadata" => {
                metadata_str = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("{e}")))?,
                );
            }
            "folder" => {
                folder_field = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("{e}")))?,
                );
            }
            _ => {}
        }
    }

    let title = title.ok_or_else(|| ApiError::BadRequest("title is required".to_string()))?;
    let data_type = data_type.unwrap_or_else(|| "other".to_string());

    // Determine file_type from extension if not provided
    let inferred_file_type = file_data.as_ref().and_then(|(name, _)| {
        let ext = std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext.to_lowercase().as_str() {
            "txt" => Some("text/plain"),
            "csv" => Some("text/csv"),
            "json" => Some("application/json"),
            "pdf" => Some("application/pdf"),
            "doc" | "docx" => Some("application/msword"),
            "xls" | "xlsx" => Some("application/vnd.ms-excel"),
            "md" => Some("text/markdown"),
            "html" | "htm" => Some("text/html"),
            "mp3" => Some("audio/mpeg"),
            "mp4" => Some("video/mp4"),
            "wav" => Some("audio/wav"),
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            _ => None,
        }
        .map(|s| s.to_string())
    });
    let file_type = file_type_field.or(inferred_file_type);

    // Create record — uploads are always source_type = "file"
    let source = DataSource::create(
        pool,
        CreateDataSource {
            organization_id: organization_id.map(|u| u.to_string()),
            project_id: project_id.map(|u| u.to_string()),
            created_by: None,
            title,
            description,
            data_type,
            source_type: Some("file".to_string()),
            file_type: file_type.clone(),
            content: None,
            file_name: file_data.as_ref().map(|(n, _)| n.clone()),
            file_path: None,
            file_size_bytes: file_data.as_ref().map(|(_, b)| b.len() as i64),
            file_hash: None,
            metadata: metadata_str,
            folder: folder_field,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create data source: {e}")))?;

    // Store file if provided — write to sovereign stack
    if let Some((filename, bytes)) = file_data {
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let uploads_dir =
            std::path::PathBuf::from("E:/topos/sovereign_stack/Sirak Studios/Uploads");
        std::fs::create_dir_all(&uploads_dir)
            .map_err(|e| ApiError::InternalError(format!("Failed to create uploads dir: {e}")))?;

        let ext = std::path::Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let stored_name = format!("{}_{}.{}", source.id, &hash[..8], ext);
        let stored_path = uploads_dir.join(&stored_name);

        std::fs::write(&stored_path, &bytes)
            .map_err(|e| ApiError::InternalError(format!("Failed to write file: {e}")))?;

        // Store file info in metadata and use content field for text-readable files
        let is_text_file = matches!(
            file_type.as_deref(),
            Some("text/plain" | "text/csv" | "text/markdown" | "text/html" | "application/json")
        );
        let file_content = if is_text_file {
            String::from_utf8(bytes.clone()).ok()
        } else {
            None
        };

        // Merge file info into metadata — store sovereign volume reference
        let relative_path = format!("Uploads/{}", stored_name);
        let mut meta: serde_json::Value =
            serde_json::from_str(&source.metadata).unwrap_or(serde_json::json!({}));
        if let Some(obj) = meta.as_object_mut() {
            obj.insert("file_name".into(), serde_json::json!(filename));
            obj.insert("file_path".into(), serde_json::json!(relative_path));
            obj.insert("file_size_bytes".into(), serde_json::json!(bytes.len()));
            obj.insert("file_hash".into(), serde_json::json!(hash));
            obj.insert("storage_volume".into(), serde_json::json!("sovereign_org"));
            obj.insert("original_path".into(), serde_json::json!(relative_path));
            if let Some(ref ft) = file_type {
                obj.insert("file_mime".into(), serde_json::json!(ft));
            }
        }

        DataSource::update(
            pool,
            &source.id,
            UpdateDataSource {
                title: None,
                description: None,
                data_type: None,
                source_type: None,
                content: file_content,
                metadata: Some(serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string())),
                status: Some("ready".to_string()),
                processing_error: None,
                folder: None,
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;

        // Update file columns
        DataSource::set_file_info(
            pool,
            &source.id,
            &filename,
            &relative_path,
            bytes.len() as i64,
            &hash,
            file_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update file info: {e}")))?;

        let updated = DataSource::find_by_id(pool, &source.id)
            .await
            .map_err(|e| ApiError::InternalError(format!("{e}")))?
            .ok_or_else(|| ApiError::InternalError("Source not found after update".to_string()))?;

        // Fire any matching workflow triggers in the background
        let trigger_pool = pool.clone();
        let trigger_ds_id = updated.id.clone();
        let trigger_dep = deployment.clone();
        tokio::spawn(async move {
            super::data_source_workflows::fire_triggers_for_data_source(
                trigger_pool,
                trigger_ds_id,
                trigger_dep,
            )
            .await;
        });

        return Ok(Json(ApiResponse::success(updated)));
    }

    // No file — mark as ready immediately
    DataSource::update(
        pool,
        &source.id,
        UpdateDataSource {
            title: None,
            description: None,
            data_type: None,
            source_type: None,
            content: None,
            metadata: None,
            status: Some("ready".to_string()),
            processing_error: None,
            folder: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    let updated = DataSource::find_by_id(pool, &source.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::InternalError("Source not found after update".to_string()))?;

    // Fire any matching workflow triggers in the background
    let trigger_pool = pool.clone();
    let trigger_ds_id = updated.id.clone();
    let trigger_dep = deployment.clone();
    tokio::spawn(async move {
        super::data_source_workflows::fire_triggers_for_data_source(
            trigger_pool,
            trigger_ds_id,
            trigger_dep,
        )
        .await;
    });

    Ok(Json(ApiResponse::success(updated)))
}

/// GET /api/data-sources/:id
async fn get_data_source(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;
    let source = DataSource::find_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;
    Ok(Json(ApiResponse::success(source)))
}

/// PUT /api/data-sources/:id
async fn update_data_source(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<UpdateDataSource>,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;
    let source = DataSource::update(pool, &id, body)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;
    Ok(Json(ApiResponse::success(source)))
}

/// DELETE /api/data-sources/:id (soft delete)
async fn delete_data_source(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    DataSource::archive(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/data-sources/:id/download — serve the stored file binary
async fn download_data_source(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let source = DataSource::find_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    // For text content, serve as a text file
    if source.source_type == "text" {
        if let Some(content) = &source.content {
            let bytes = content.clone().into_bytes();
            let file_name = format!("{}.txt", source.title.replace(['/', '\\', ':'], "_"));
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{}\"", file_name))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            );
            return Response::builder()
                .status(200)
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", file_name),
                )
                .body(Body::from(bytes))
                .map_err(|e| ApiError::InternalError(format!("{e}")));
        }
    }

    // For file uploads, read from disk
    let meta: serde_json::Value =
        serde_json::from_str(&source.metadata).unwrap_or(serde_json::json!({}));

    // Check if this is a cloud-indexed file with a storage_volume
    let storage_volume = meta.get("storage_volume").and_then(|v| v.as_str());
    let original_path = meta.get("original_path").and_then(|v| v.as_str());

    let file_path = if let (Some(volume), Some(rel_path)) = (storage_volume, original_path) {
        // Prevent path traversal
        if rel_path.contains("..") || rel_path.contains('\0') {
            return Err(ApiError::BadRequest("Invalid file path".into()));
        }
        // Cloud-indexed: resolve volume path (env vars match sovereign_stack.rs defaults)
        let stack_root = std::env::var("SOVEREIGN_STACK_ROOT")
            .unwrap_or_else(|_| "E:/topos/sovereign_stack".to_string());
        let org_name = std::env::var("SOVEREIGN_STACK_ORG_NAME")
            .unwrap_or_else(|_| "Sirak Studios".to_string());
        let base = match volume {
            "sovereign_personal" => std::path::PathBuf::from(&stack_root).join("Personal"),
            "sovereign_org" => std::path::PathBuf::from(&stack_root).join(&org_name),
            "media_pipeline" => std::path::PathBuf::from(&stack_root)
                .join(&org_name)
                .join("Media Pipeline"),
            "sovereign" => {
                let storage_root = std::env::var("SOVEREIGN_STORAGE_ROOT")
                    .unwrap_or_else(|_| "E:/topos/sovereign_storage".to_string());
                std::path::PathBuf::from(storage_root)
            }
            "data_sources" => utils::cache_dir().join("data_sources"),
            "artifacts" => utils::cache_dir().join("artifacts"),
            _ => return Err(ApiError::NotFound(format!("Unknown volume: {}", volume))),
        };
        base.join(rel_path)
    } else {
        // Legacy: look in cache_dir/data_sources
        let stored_name = meta
            .get("file_path")
            .and_then(|v| v.as_str())
            .or(source.file_path.as_deref())
            .ok_or_else(|| ApiError::NotFound("File not stored locally".to_string()))?;
        utils::cache_dir().join("data_sources").join(stored_name)
    };

    if !file_path.exists() {
        return Err(ApiError::NotFound("File not found on disk".to_string()));
    }

    let bytes = std::fs::read(&file_path)
        .map_err(|e| ApiError::InternalError(format!("Failed to read file: {e}")))?;

    let original_name = meta
        .get("file_name")
        .and_then(|v| v.as_str())
        .or(source.file_name.as_deref())
        .unwrap_or(&source.title);

    let mime = source
        .file_type
        .as_deref()
        .or_else(|| meta.get("file_mime").and_then(|v| v.as_str()))
        .unwrap_or("application/octet-stream");

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", original_name),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(Body::from(bytes))
        .map_err(|e| ApiError::InternalError(format!("{e}")))
}

/// GET /api/data-sources/metadata-template/:data_type?source_type=...
async fn get_metadata_template(
    Path(data_type): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let source_type = params
        .get("source_type")
        .map(|s| s.as_str())
        .unwrap_or("text");
    Json(ApiResponse::success(metadata_template(
        &data_type,
        source_type,
    )))
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/projects/{project_id}/data-sources", get(list_by_project))
        .route("/data-sources", post(create_data_source))
        .route(
            "/data-sources/upload",
            post(upload_data_source).layer(DefaultBodyLimit::max(50 * 1024 * 1024)), // 50MB
        )
        .route(
            "/data-sources/metadata-template/{data_type}",
            get(get_metadata_template),
        )
        .route(
            "/data-sources/{id}",
            get(get_data_source)
                .put(update_data_source)
                .delete(delete_data_source),
        )
        .route("/data-sources/{id}/download", get(download_data_source))
}

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    routing::{delete, get, post, put},
};
use db::models::data_source::{CreateDataSource, DataSource, UpdateDataSource, metadata_template};
use deployment::Deployment;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ── List endpoints ──────────────────────────────────────────────────────────

/// GET /api/organizations/:org_id/data-sources
async fn list_by_organization(
    Path(org_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<DataSource>>>, ApiError> {
    let pool = &deployment.db().pool;
    let sources = DataSource::find_by_organization_all(pool, org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list data sources: {e}")))?;
    Ok(Json(ApiResponse::success(sources)))
}

/// GET /api/projects/:project_id/data-sources
async fn list_by_project(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<DataSource>>>, ApiError> {
    let pool = &deployment.db().pool;
    let sources = DataSource::find_by_project(pool, project_id)
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
}

/// POST /api/data-sources
async fn create_data_source(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateDataSourceRequest>,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;

    let valid_types = ["conversation", "document", "transcript", "report", "dataset", "media", "other"];
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
            organization_id: body.organization_id,
            project_id: body.project_id,
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
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create data source: {e}")))?;

    // For text sources, mark as ready immediately
    let source = if source.source_type == "text" {
        DataSource::update(
            pool,
            source.id,
            UpdateDataSource {
                title: None, description: None, data_type: None,
                source_type: None, content: None, metadata: None,
                status: Some("ready".to_string()), processing_error: None,
            },
        ).await.map_err(|e| ApiError::InternalError(format!("{e}")))?
        .unwrap_or(source)
    } else {
        source
    };

    // Fire any matching workflow triggers in the background
    let trigger_pool = pool.clone();
    let trigger_ds_id = source.id;
    tokio::spawn(async move {
        super::data_source_workflows::fire_triggers_for_data_source(trigger_pool, trigger_ds_id).await;
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
    let mut file_data: Option<(String, Vec<u8>)> = None; // (filename, bytes)

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Multipart error: {e}"))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "upload".to_string());
                let bytes = field.bytes().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read file: {e}"))
                })?;
                file_data = Some((filename, bytes.to_vec()));
            }
            "title" => {
                title = Some(field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?);
            }
            "description" => {
                description = Some(field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?);
            }
            "data_type" => {
                data_type = Some(field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?);
            }
            "file_type" => {
                file_type_field = Some(field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?);
            }
            "organization_id" => {
                let val = field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?;
                organization_id = Some(Uuid::parse_str(&val).map_err(|e| ApiError::BadRequest(format!("Invalid org ID: {e}")))?);
            }
            "project_id" => {
                let val = field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?;
                project_id = Some(Uuid::parse_str(&val).map_err(|e| ApiError::BadRequest(format!("Invalid project ID: {e}")))?);
            }
            "metadata" => {
                metadata_str = Some(field.text().await.map_err(|e| ApiError::BadRequest(format!("{e}")))?);
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
            organization_id,
            project_id,
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
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create data source: {e}")))?;

    // Store file if provided
    if let Some((filename, bytes)) = file_data {
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let uploads_dir = utils::cache_dir().join("data_sources");
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

        // Merge file info into metadata
        let mut meta: serde_json::Value = serde_json::from_str(&source.metadata).unwrap_or(serde_json::json!({}));
        if let Some(obj) = meta.as_object_mut() {
            obj.insert("file_name".into(), serde_json::json!(filename));
            obj.insert("file_path".into(), serde_json::json!(stored_name));
            obj.insert("file_size_bytes".into(), serde_json::json!(bytes.len()));
            obj.insert("file_hash".into(), serde_json::json!(hash));
            if let Some(ref ft) = file_type {
                obj.insert("file_mime".into(), serde_json::json!(ft));
            }
        }

        DataSource::update(
            pool,
            source.id,
            UpdateDataSource {
                title: None, description: None, data_type: None,
                source_type: None,
                content: file_content,
                metadata: Some(serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string())),
                status: Some("ready".to_string()),
                processing_error: None,
            },
        ).await.map_err(|e| ApiError::InternalError(format!("{e}")))?;

        // Also update legacy file columns for backwards compat
        DataSource::set_file_info(
            pool, source.id, &filename, &stored_name,
            bytes.len() as i64, &hash, file_type.as_deref(),
        ).await.map_err(|e| ApiError::InternalError(format!("Failed to update file info: {e}")))?;

        let updated = DataSource::find_by_id(pool, source.id)
            .await
            .map_err(|e| ApiError::InternalError(format!("{e}")))?
            .ok_or_else(|| ApiError::InternalError("Source not found after update".to_string()))?;

        // Fire any matching workflow triggers in the background
        let trigger_pool = pool.clone();
        let trigger_ds_id = updated.id;
        tokio::spawn(async move {
            super::data_source_workflows::fire_triggers_for_data_source(trigger_pool, trigger_ds_id).await;
        });

        return Ok(Json(ApiResponse::success(updated)));
    }

    // No file — mark as ready immediately
    DataSource::update(
        pool,
        source.id,
        UpdateDataSource {
            title: None, description: None, data_type: None,
            source_type: None, content: None, metadata: None,
            status: Some("ready".to_string()), processing_error: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    let updated = DataSource::find_by_id(pool, source.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::InternalError("Source not found after update".to_string()))?;

    // Fire any matching workflow triggers in the background
    let trigger_pool = pool.clone();
    let trigger_ds_id = updated.id;
    tokio::spawn(async move {
        super::data_source_workflows::fire_triggers_for_data_source(trigger_pool, trigger_ds_id).await;
    });

    Ok(Json(ApiResponse::success(updated)))
}

/// GET /api/data-sources/:id
async fn get_data_source(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;
    let source = DataSource::find_by_id(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;
    Ok(Json(ApiResponse::success(source)))
}

/// PUT /api/data-sources/:id
async fn update_data_source(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<UpdateDataSource>,
) -> Result<Json<ApiResponse<DataSource>>, ApiError> {
    let pool = &deployment.db().pool;
    let source = DataSource::update(pool, id, body)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;
    Ok(Json(ApiResponse::success(source)))
}

/// DELETE /api/data-sources/:id (soft delete)
async fn delete_data_source(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    DataSource::archive(pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/data-sources/metadata-template/:data_type?source_type=...
async fn get_metadata_template(
    Path(data_type): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let source_type = params.get("source_type").map(|s| s.as_str()).unwrap_or("text");
    Json(ApiResponse::success(metadata_template(&data_type, source_type)))
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/organizations/{org_id}/data-sources", get(list_by_organization))
        .route("/projects/{project_id}/data-sources", get(list_by_project))
        .route("/data-sources", post(create_data_source))
        .route(
            "/data-sources/upload",
            post(upload_data_source).layer(DefaultBodyLimit::max(50 * 1024 * 1024)), // 50MB
        )
        .route("/data-sources/metadata-template/{data_type}", get(get_metadata_template))
        .route("/data-sources/{id}", get(get_data_source).put(update_data_source).delete(delete_data_source))
}

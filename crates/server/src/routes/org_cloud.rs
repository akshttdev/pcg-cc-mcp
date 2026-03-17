use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Extension, Multipart, Path, Query, State},
    http::header,
    response::Response,
    routing::{get, post, put},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::AccessContext,
};
use db::models::cloud_file::{
    CloudBrowseParams, CloudContribution, CloudFile, CreateCloudContribution, CreateCloudFile,
    OrgCloudSettings, UpdateCloudFile, UpdateOrgCloudSettings,
};

// ── Permission helper ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct OrgCloudAccess {
    pub role: String,
    pub can_download: bool,
    pub can_upload: bool,
}

async fn require_org_cloud_access(
    access: &AccessContext,
    pool: &sqlx::SqlitePool,
    org_id: &str,
) -> Result<OrgCloudAccess, ApiError> {
    // Admin bypass
    if access.is_admin {
        return Ok(OrgCloudAccess {
            role: "admin".to_string(),
            can_download: true,
            can_upload: true,
        });
    }

    let user_id_bytes = access.user_id.to_string();
    let org_uuid = Uuid::parse_str(org_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
    let org_id_bytes = org_uuid.to_string();

    #[derive(sqlx::FromRow)]
    struct RoleRow {
        role: String,
    }

    let org_role: Option<RoleRow> = sqlx::query_as(
        "SELECT role FROM organization_members WHERE organization_id = ? AND user_id = ?",
    )
    .bind(&org_id_bytes)
    .bind(&user_id_bytes)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let role = org_role
        .map(|r| r.role)
        .ok_or_else(|| ApiError::Forbidden("Not a member of this organization".to_string()))?;

    let settings = OrgCloudSettings::get_or_create(pool, org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load cloud settings: {}", e)))?;

    let can_download = match role.as_str() {
        "admin" | "member" => true,
        "viewer" => settings.viewer_can_download,
        _ => false,
    };
    let can_upload = match role.as_str() {
        "admin" => true,
        "member" => settings.member_can_upload,
        _ => false,
    };

    Ok(OrgCloudAccess {
        role,
        can_download,
        can_upload,
    })
}

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/org-cloud/{org_id}/browse", get(browse_files))
        .route(
            "/org-cloud/{org_id}/files/{file_id}/download",
            get(download_file),
        )
        .route(
            "/org-cloud/{org_id}/files/{file_id}/preview",
            get(preview_file),
        )
        .route(
            "/org-cloud/{org_id}/contribute",
            post(contribute_file).layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        .route("/org-cloud/{org_id}/stats", get(get_stats))
        .route("/org-cloud/{org_id}/settings", get(get_settings).put(update_settings))
        .route("/org-cloud/{org_id}/contributions", get(list_contributions))
        .route("/org-cloud/{org_id}/index", post(trigger_index))
        .route("/org-cloud/{org_id}/files/{file_id}", put(update_file))
        .with_state(deployment.clone())
}

// ── Browse ─────────────────────────────────────────────────────────────────

async fn browse_files(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(params): Query<CloudBrowseParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    // For non-admin members, scope to their visible projects
    let visible_projects: Option<Vec<String>> = if cloud_access.role != "admin" {
        let org_uuid = Uuid::parse_str(&org_id)
            .map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
        let org_id_bytes = org_uuid.to_string();

        #[derive(sqlx::FromRow)]
        struct IdRow {
            id: Vec<u8>,
        }

        let rows: Vec<IdRow> = sqlx::query_as(
            "SELECT p.id FROM projects p WHERE p.organization_id = ?",
        )
        .bind(&org_id_bytes)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

        let ids: Vec<String> = rows
            .iter()
            .filter_map(|r| Uuid::from_slice(&r.id).ok().map(|u| u.to_string()))
            .collect();
        Some(ids)
    } else {
        None
    };

    let files = CloudFile::browse(
        pool,
        &org_id,
        &params,
        visible_projects.as_deref(),
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to browse files: {}", e)))?;

    let total = CloudFile::count(pool, &org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to count files: {}", e)))?;

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "files": files,
        "total": total,
        "page": page,
        "per_page": per_page,
    }))))
}

// ── Volume resolution ─────────────────────────────────────────────────────

fn resolve_volume_path(volume: &str, file_path: &str) -> Result<std::path::PathBuf, ApiError> {
    // Prevent path traversal
    if file_path.contains("..") {
        return Err(ApiError::BadRequest("Invalid file path".into()));
    }

    let base = match volume {
        // Sovereign stack volumes (primary)
        "sovereign_personal" => std::path::PathBuf::from("E:/topos/sovereign_stack/Personal"),
        "sovereign_org" => std::path::PathBuf::from("E:/topos/sovereign_stack/Sirak Studios"),
        "media_pipeline" => std::path::PathBuf::from("E:/topos/sovereign_stack/Sirak Studios/Media Pipeline"),
        "sovereign" => std::path::PathBuf::from("E:/topos/sovereign_storage"),
        // Standard volumes
        "data_sources" => utils::cache_dir().join("data_sources"),
        "artifacts" => utils::cache_dir().join("artifacts"),
        "dropbox" => std::path::PathBuf::from("E:/topos/dropbox_ingest"),
        // Legacy fallbacks (existing DB rows still reference these)
        "dropbox_personal" | "sovereign_dropbox_personal" => std::path::PathBuf::from("E:/topos/Sirak Studios (sirak)"),
        "dropbox_team" | "sovereign_dropbox_team" => std::path::PathBuf::from("E:/topos/Sirak Studios Team"),
        _ => return Err(ApiError::BadRequest(format!("Unknown volume: {}", volume))),
    };

    Ok(base.join(file_path))
}

// ── Download ───────────────────────────────────────────────────────────────

async fn download_file(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((org_id, file_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if !cloud_access.can_download {
        return Err(ApiError::Forbidden(
            "Download not permitted for your role".into(),
        ));
    }

    let file = CloudFile::find_by_id(pool, &file_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("File not found".into()))?;

    if file.organization_id != org_id {
        return Err(ApiError::Forbidden(
            "File does not belong to this org".into(),
        ));
    }

    let path = resolve_volume_path(&file.storage_volume, &file.file_path)?;
    if !path.is_file() {
        return Err(ApiError::NotFound("File not found on disk".into()));
    }

    let content_type = file
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let file_size = tokio::fs::metadata(&path).await?.len();
    let disk_file = File::open(&path).await?;
    let stream = ReaderStream::new(disk_file);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file.file_name),
        )
        .body(Body::from_stream(stream))
        .map_err(|e| ApiError::InternalError(e.to_string()))
}

// ── Preview (inline) ──────────────────────────────────────────────────────

async fn preview_file(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((org_id, file_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if !cloud_access.can_download {
        return Err(ApiError::Forbidden(
            "Preview not permitted for your role".into(),
        ));
    }

    let file = CloudFile::find_by_id(pool, &file_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("File not found".into()))?;

    if file.organization_id != org_id {
        return Err(ApiError::Forbidden(
            "File does not belong to this org".into(),
        ));
    }

    let path = resolve_volume_path(&file.storage_volume, &file.file_path)?;
    if !path.is_file() {
        return Err(ApiError::NotFound("File not found on disk".into()));
    }

    let content_type = file
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let file_size = tokio::fs::metadata(&path).await?.len();
    let disk_file = File::open(&path).await?;
    let stream = ReaderStream::new(disk_file);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::CONTENT_DISPOSITION, "inline")
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .body(Body::from_stream(stream))
        .map_err(|e| ApiError::InternalError(e.to_string()))
}

// ── Contribute (upload) ────────────────────────────────────────────────────

async fn contribute_file(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<CloudFile>>, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if !cloud_access.can_upload {
        return Err(ApiError::Forbidden(
            "Upload not permitted for your role".into(),
        ));
    }

    let mut file_data: Option<(String, Vec<u8>)> = None;
    let mut project_id: Option<String> = None;
    let mut task_id: Option<String> = None;
    let mut description: Option<String> = None;
    let mut visibility: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
                file_data = Some((filename, bytes.to_vec()));
            }
            "project_id" => project_id = Some(field.text().await.unwrap_or_default()),
            "task_id" => task_id = Some(field.text().await.unwrap_or_default()),
            "description" => description = Some(field.text().await.unwrap_or_default()),
            "visibility" => visibility = Some(field.text().await.unwrap_or_default()),
            _ => {}
        }
    }

    let (filename, bytes) =
        file_data.ok_or_else(|| ApiError::BadRequest("No file provided".into()))?;
    if filename.is_empty() {
        return Err(ApiError::BadRequest("Filename is empty".into()));
    }

    // Compute SHA-256 hash
    let hash = format!("{:x}", Sha256::digest(&bytes));

    // Check for duplicate
    if let Some(existing) = CloudFile::find_by_content_hash(pool, &org_id, &hash)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    {
        return Ok(Json(ApiResponse::success(existing)));
    }

    // Store file
    let upload_dir = utils::cache_dir().join("cloud_uploads").join(&org_id);
    std::fs::create_dir_all(&upload_dir)
        .map_err(|e| ApiError::InternalError(format!("Failed to create upload dir: {}", e)))?;

    let ext = std::path::Path::new(&filename)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let stored_name = format!("{}_{}.{}", &hash[..8], Uuid::new_v4(), ext);
    let stored_path = upload_dir.join(&stored_name);
    std::fs::write(&stored_path, &bytes)
        .map_err(|e| ApiError::InternalError(format!("Failed to write file: {}", e)))?;

    let mime = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();

    let relative_path = format!("cloud_uploads/{}/{}", org_id, stored_name);

    let cloud_file = CloudFile::create(
        pool,
        &CreateCloudFile {
            organization_id: org_id.clone(),
            project_id: project_id.clone().filter(|s| !s.is_empty()),
            task_id: task_id.clone().filter(|s| !s.is_empty()),
            file_name: filename,
            file_path: relative_path,
            storage_volume: "data_sources".to_string(),
            content_hash: Some(hash),
            file_size_bytes: bytes.len() as i64,
            mime_type: Some(mime),
            source_type: Some("upload".to_string()),
            source_id: None,
            source_table: None,
            visibility,
            contributed_by: Some(access.user_id.to_string()),
            contributor_wallet: None,
            contributor_device: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create cloud file: {}", e)))?;

    // Create contribution audit record
    let _ = CloudContribution::create(
        pool,
        &CreateCloudContribution {
            organization_id: org_id,
            cloud_file_id: cloud_file.id.clone(),
            user_id: Some(access.user_id.to_string()),
            wallet_address: None,
            device_id: None,
            contribution_type: Some("upload".to_string()),
            channel: Some("http".to_string()),
            project_id,
            task_id,
            description,
            metadata: None,
        },
    )
    .await;

    Ok(Json(ApiResponse::success(cloud_file)))
}

// ── Stats ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CloudStats {
    total_files: i64,
    total_size_bytes: i64,
    total_contributions: i64,
    quota_bytes: i64,
}

async fn get_stats(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiResponse<CloudStats>>, ApiError> {
    let pool = &deployment.db().pool;
    let _ = require_org_cloud_access(&access, pool, &org_id).await?;

    let total_files = CloudFile::count(pool, &org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    let total_size = CloudFile::total_size(pool, &org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    let total_contribs = CloudContribution::count_by_org(pool, &org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    let settings = OrgCloudSettings::get_or_create(pool, &org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(Json(ApiResponse::success(CloudStats {
        total_files,
        total_size_bytes: total_size,
        total_contributions: total_contribs,
        quota_bytes: settings.storage_quota_bytes,
    })))
}

// ── Settings ───────────────────────────────────────────────────────────────

async fn get_settings(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiResponse<OrgCloudSettings>>, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if cloud_access.role != "admin" {
        return Err(ApiError::Forbidden(
            "Admin access required for settings".into(),
        ));
    }

    let settings = OrgCloudSettings::get_or_create(pool, &org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(Json(ApiResponse::success(settings)))
}

async fn update_settings(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Json(input): Json<UpdateOrgCloudSettings>,
) -> Result<Json<ApiResponse<OrgCloudSettings>>, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if cloud_access.role != "admin" {
        return Err(ApiError::Forbidden(
            "Admin access required for settings".into(),
        ));
    }

    let settings = OrgCloudSettings::update(pool, &org_id, &input)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(Json(ApiResponse::success(settings)))
}

// ── Contributions ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ContribQueryParams {
    limit: Option<i64>,
}

async fn list_contributions(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    Query(params): Query<ContribQueryParams>,
) -> Result<Json<ApiResponse<Vec<CloudContribution>>>, ApiError> {
    let pool = &deployment.db().pool;
    let _ = require_org_cloud_access(&access, pool, &org_id).await?;

    let limit = params.limit.unwrap_or(50).min(200);
    let contribs = CloudContribution::find_by_org(pool, &org_id, limit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(Json(ApiResponse::success(contribs)))
}

// ── Update file ────────────────────────────────────────────────────────────

async fn update_file(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((org_id, file_id)): Path<(String, String)>,
    Json(input): Json<UpdateCloudFile>,
) -> Result<Json<ApiResponse<CloudFile>>, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if cloud_access.role != "admin" && cloud_access.role != "member" {
        return Err(ApiError::Forbidden("Insufficient permissions".into()));
    }

    let file = CloudFile::update(pool, &file_id, &input)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("File not found".into()))?;

    if file.organization_id != org_id {
        return Err(ApiError::Forbidden(
            "File does not belong to this org".into(),
        ));
    }

    Ok(Json(ApiResponse::success(file)))
}

// ── Index existing data ────────────────────────────────────────────────────

async fn trigger_index(
    Extension(access): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let cloud_access = require_org_cloud_access(&access, pool, &org_id).await?;

    if cloud_access.role != "admin" {
        return Err(ApiError::Forbidden(
            "Admin access required to trigger indexing".into(),
        ));
    }

    let indexed = crate::org_cloud_indexer::index_existing_data(pool, &org_id).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "indexed_count": indexed,
    }))))
}

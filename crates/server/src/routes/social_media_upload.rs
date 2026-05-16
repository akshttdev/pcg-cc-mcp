use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    routing::post,
    Json, Router,
};
use db::models::social_media_asset::{CreateSocialMediaAsset, SocialMediaAsset};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024; // 50 MB

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub project_id: Uuid,
    pub deliverable_id: Option<Uuid>,
    pub social_post_id: Option<Uuid>,
    pub uploaded_by: Option<String>,
}

/// POST /api/media/social-upload
/// Multipart: field "file" (required)
async fn upload_social_media(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<SocialMediaAsset>>, ApiError> {
    let pool = &deployment.db().pool;

    let mut file_data: Option<(String, Vec<u8>, String)> = None; // (filename, bytes, mime)

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let original_filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "upload".to_string());
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
            file_data = Some((original_filename, bytes.to_vec(), content_type));
        }
    }

    let (original_filename, bytes, mime_type) =
        file_data.ok_or_else(|| ApiError::BadRequest("No file provided".into()))?;

    // Store under sovereign org Social Uploads dir
    let upload_dir = utils::volume::resolve_volume_path("sovereign_org", "Social Uploads")
        .map_err(|e| ApiError::InternalError(format!("Storage error: {}", e)))?;

    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create upload dir: {}", e)))?;

    let ext = std::path::Path::new(&original_filename)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_id = Uuid::new_v4();
    let stored_name = if ext.is_empty() {
        file_id.to_string()
    } else {
        format!("{}.{}", file_id, ext)
    };
    let stored_path = upload_dir.join(&stored_name);

    let file_size = bytes.len() as i64;
    tokio::fs::write(&stored_path, &bytes)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to write file: {}", e)))?;

    // Detect image dimensions if it's an image
    let (width_px, height_px) = detect_image_dimensions(&bytes, &mime_type);

    let app_base = std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".into());
    let public_url = format!("{}/uploads/social/{}", app_base, stored_name);
    let storage_path = stored_path.to_string_lossy().to_string();

    let asset = SocialMediaAsset::create(
        pool,
        CreateSocialMediaAsset {
            project_id: query.project_id,
            deliverable_id: query.deliverable_id,
            social_post_id: query.social_post_id,
            filename: stored_name,
            original_filename: Some(original_filename),
            storage_path,
            public_url,
            mime_type,
            file_size_bytes: Some(file_size),
            width_px,
            height_px,
            uploaded_by: query.uploaded_by,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(Json(ApiResponse::success(asset)))
}

fn detect_image_dimensions(bytes: &[u8], mime_type: &str) -> (Option<i64>, Option<i64>) {
    if !mime_type.starts_with("image/") {
        return (None, None);
    }
    // Read PNG dimensions from header (bytes 16-24) — zero-dep approach
    if mime_type == "image/png" && bytes.len() >= 24 {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return (Some(w as i64), Some(h as i64));
    }
    // Read JPEG dimensions from SOF marker — skip for MVP
    (None, None)
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/media/social-upload", post(upload_social_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
}

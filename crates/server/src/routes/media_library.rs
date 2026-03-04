//! Media Library routes — native DAM (Shade-equivalent).
//!
//! POST /projects/{project_id}/media       — multipart upload → save → analyze
//! GET  /projects/{project_id}/media       — list assets
//! GET  /projects/{project_id}/media/search?q= — FTS5 search
//! GET  /media/{id}                        — single asset
//! DELETE /media/{id}                      — delete
//! POST /media/{id}/analyze                — re-trigger analysis

use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
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

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/projects/{project_id}/media",
            get(list_assets).post(upload_asset),
        )
        .route("/projects/{project_id}/media/search", get(search_assets))
        .route("/media/{id}", get(get_asset).delete(delete_asset))
        .route("/media/{id}/analyze", post(retrigger_analysis))
        .with_state(deployment.clone())
}

#![allow(dead_code)]
//! MCP tools for media library operations.
//!
//! Lets an AI agent discover existing project assets, register new media from
//! a remote URL, and produce serve URLs that the social `Publisher` can pass
//! to per-platform connectors as `media_urls`. Pairs with `social_ops.rs` —
//! typical flow:
//!
//!   1. `upload_media_from_url` (or `list_media_assets`)  →  asset + url
//!   2. `create_social_post` with `media_urls: [<that url>]`
//!   3. `publish_social_post`

use std::path::PathBuf;

use db::models::media_asset::{CreateMediaAsset, MediaAsset};
use rmcp::{ErrorData, handler::server::tool::Parameters, model::CallToolResult, schemars, tool};
use serde::{Deserialize, Serialize};
use services::services::editron::asset_intelligence;
use uuid::Uuid;

use super::{TaskServer, helpers::*};

// ─── Request types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListMediaAssetsRequest {
    #[schemars(description = "Project UUID whose media library should be listed")]
    pub project_id: String,
    #[schemars(description = "Max number of assets to return (default: 50, max: 500)")]
    pub limit: Option<i64>,
    #[schemars(
        description = "Optional MIME-type prefix filter (e.g. \"video\" to return only video assets, \"image\" for images only)"
    )]
    pub mime_prefix: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetMediaAssetRequest {
    #[schemars(description = "Media asset UUID")]
    pub media_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadMediaFromUrlRequest {
    #[schemars(description = "Project UUID to register the asset under")]
    pub project_id: String,
    #[schemars(
        description = "Source URL the server should fetch the file from. Must be reachable from the backend (public CDN, signed S3 URL, etc.)."
    )]
    pub source_url: String,
    #[schemars(
        description = "Optional filename to store the asset as. Defaults to the last path segment of source_url."
    )]
    pub filename: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct MediaAssetSummary {
    /// Asset UUID — pass to `get_media_asset` or use as part of post media_urls
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub duration_seconds: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    /// Publicly-fetchable URL the social Publisher can pass to connectors as
    /// `media_urls[i]`. Streams the file bytes directly from the backend.
    pub url: String,
    pub ai_description: Option<String>,
    pub analysis_status: String,
    pub created_at: String,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Compose the public serve URL for an asset. Falls back to localhost when
/// the env var isn't set so MCP responses still produce a working URL during
/// dev.
fn public_serve_url(asset_id: &Uuid) -> String {
    let base = std::env::var("OAUTH_REDIRECT_BASE")
        .unwrap_or_else(|_| "http://localhost:3000/api".to_string());
    format!("{base}/media/{asset_id}/serve")
}

fn asset_to_summary(asset: &MediaAsset) -> MediaAssetSummary {
    MediaAssetSummary {
        id: asset.id.to_string(),
        filename: asset.filename.clone(),
        mime_type: asset.mime_type.clone(),
        file_size_bytes: asset.file_size_bytes,
        duration_seconds: asset.duration_seconds,
        width: asset.width,
        height: asset.height,
        url: public_serve_url(&asset.id),
        ai_description: asset.ai_description.clone(),
        analysis_status: asset.analysis_status.clone(),
        created_at: asset.created_at.to_rfc3339(),
    }
}

fn media_root() -> String {
    std::env::var("MEDIA_ROOT")
        .unwrap_or_else(|_| "/home/pythia/pcg-cc-mcp/dev_assets/media".into())
}

// ─── Tool implementations ───────────────────────────────────────────────────

impl TaskServer {
    #[tool(
        description = "List media assets (videos, images) registered to a project. Each asset comes back with a `url` field that social-post tools can use as `media_urls[i]` — Publisher fetches that URL when handing the asset off to a platform connector (YouTube, Instagram, etc.). Optionally filter by `mime_prefix` (e.g. \"video\") to narrow results."
    )]
    pub(super) async fn list_media_assets(
        &self,
        Parameters(req): Parameters<ListMediaAssetsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let limit = req.limit.unwrap_or(50).clamp(1, 500);

        match MediaAsset::find_by_project(&self.pool, project_uuid, limit).await {
            Ok(assets) => {
                let prefix = req.mime_prefix.as_deref();
                let summaries: Vec<MediaAssetSummary> = assets
                    .iter()
                    .filter(|a| match prefix {
                        Some(p) => a.mime_type.starts_with(p),
                        None => true,
                    })
                    .map(asset_to_summary)
                    .collect();
                Ok(success_json(&serde_json::json!({
                    "success": true,
                    "count": summaries.len(),
                    "assets": summaries,
                })))
            }
            Err(e) => Ok(error_result(
                "Failed to list media assets",
                Some(&e.to_string()),
            )),
        }
    }

    #[tool(
        description = "Fetch a single media asset's metadata + public serve URL by UUID. Use the `url` field as `media_urls[i]` when creating a social post."
    )]
    pub(super) async fn get_media_asset(
        &self,
        Parameters(req): Parameters<GetMediaAssetRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let media_uuid = match parse_uuid(&req.media_id, "media_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match MediaAsset::find_by_id(&self.pool, media_uuid).await {
            Ok(Some(asset)) => Ok(success_json(&serde_json::json!({
                "success": true,
                "asset": asset_to_summary(&asset),
            }))),
            Ok(None) => Ok(error_result("Asset not found", None)),
            Err(e) => Ok(error_result("Failed to load asset", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Register a remote file (video, image) as a project media asset. The backend downloads from `source_url` once, stores the bytes under MEDIA_ROOT, indexes it in media_assets, and triggers AI analysis. Returns the asset with its public `url` ready to pass to `create_social_post` as media_urls[i]. Idempotent: if the same filename already exists in the project, returns the existing asset without re-downloading."
    )]
    pub(super) async fn upload_media_from_url(
        &self,
        Parameters(req): Parameters<UploadMediaFromUrlRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let filename = req.filename.unwrap_or_else(|| {
            req.source_url
                .split('/')
                .next_back()
                .unwrap_or("download")
                .to_string()
        });

        let project_dir = PathBuf::from(media_root()).join(project_uuid.to_string());
        if let Err(e) = tokio::fs::create_dir_all(&project_dir).await {
            return Ok(error_result(
                "Cannot create project media directory",
                Some(&e.to_string()),
            ));
        }

        let dest = project_dir.join(&filename);
        let file_path_str = dest.to_string_lossy().into_owned();

        // Idempotency: if (project, filename) already indexed, return the existing row.
        let existing: Option<MediaAsset> =
            sqlx::query_as("SELECT * FROM media_assets WHERE file_path = ? AND project_id = ?")
                .bind(&file_path_str)
                .bind(project_uuid)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
        if let Some(asset) = existing {
            return Ok(success_json(&serde_json::json!({
                "success": true,
                "asset": asset_to_summary(&asset),
                "reused": true,
            })));
        }

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return Ok(error_result(
                    "Failed to construct HTTP client",
                    Some(&e.to_string()),
                ));
            }
        };

        let resp = match client.get(&req.source_url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(error_result(
                    "Failed to fetch source_url",
                    Some(&e.to_string()),
                ));
            }
        };
        if !resp.status().is_success() {
            return Ok(error_result(
                "Source URL returned non-success status",
                Some(&format!("HTTP {}", resp.status())),
            ));
        }

        let mime_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .split(';')
            .next()
            .unwrap_or("application/octet-stream")
            .to_string();

        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return Ok(error_result(
                    "Failed to read response bytes",
                    Some(&e.to_string()),
                ));
            }
        };
        let file_size = bytes.len() as i64;

        if let Err(e) = tokio::fs::write(&dest, &bytes).await {
            return Ok(error_result(
                "Failed to write asset to disk",
                Some(&e.to_string()),
            ));
        }

        let asset = match MediaAsset::create(
            &self.pool,
            CreateMediaAsset {
                project_id: project_uuid,
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
        .await
        {
            Ok(a) => a,
            Err(e) => {
                return Ok(error_result(
                    "Failed to insert media_assets row",
                    Some(&e.to_string()),
                ));
            }
        };

        // Fire-and-forget AI analysis (caption, scene tags, etc.).
        asset_intelligence::analyze_async(
            self.pool.clone(),
            asset.id,
            file_path_str,
            project_uuid,
            filename,
        );

        Ok(success_json(&serde_json::json!({
            "success": true,
            "asset": asset_to_summary(&asset),
            "reused": false,
        })))
    }
}

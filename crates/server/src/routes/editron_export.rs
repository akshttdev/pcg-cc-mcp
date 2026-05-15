use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use db::models::execution_artifact::{ArtifactType, ExecutionArtifact};
use deployment::Deployment;
use serde::Deserialize;
use services::services::editron::{
    edit_assembly::{AssembledEdit, TimelineClip},
    premiere_xml::PremiereXmlExporter,
};
use utils::assets::asset_dir;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    /// Pass mode=apn to get XML with /Volumes/PCG APN/ paths for APN Drive mount
    pub mode: Option<String>,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/editron/export/{artifact_id}", get(export_artifact))
        .route(
            "/editron/export/{artifact_id}/package",
            get(download_premiere_package),
        )
        .with_state(deployment.clone())
}

/// GET /api/editron/export/{artifact_id}?format=xml
/// Generates FCP XML from a video_edit_session artifact and returns it as a download.
async fn export_artifact(
    State(deployment): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let format = query.format.as_deref().unwrap_or("xml");
    if format != "xml" {
        return Err(ApiError::BadRequest(format!(
            "Unsupported export format: '{}'. Only 'xml' is supported.",
            format
        )));
    }

    let pool = &deployment.db().pool;
    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    // Only allow export for video edit types
    match artifact.artifact_type {
        ArtifactType::VideoEditSession | ArtifactType::RenderDeliverable => {}
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Cannot export artifact type '{}'. Only video_edit_session and render_deliverable are supported.",
                artifact.artifact_type
            )));
        }
    }

    let content = artifact
        .content
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Artifact has no content to export".into()))?;

    let edit_data: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| ApiError::BadRequest(format!("Invalid artifact JSON: {}", e)))?;

    // Build an AssembledEdit from the artifact's edit data
    let edit = build_assembled_edit(&artifact.title, &edit_data)?;

    let apn_mode = query.mode.as_deref() == Some("apn");
    let exporter = if apn_mode {
        PremiereXmlExporter::new(edit.frame_rate)
            .with_media_root(crate::routes::dav::APN_MOUNT_PATH)
    } else {
        PremiereXmlExporter::new(edit.frame_rate)
    };
    let xml = exporter.generate_xml(&edit);

    let filename = format!(
        "{}.xml",
        artifact
            .title
            .replace(' ', "-")
            .replace([':', '/', '\\'], "")
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xml")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(xml))
        .map_err(|e| ApiError::InternalError(e.to_string()))
}

/// GET /api/editron/export/{artifact_id}/package
/// Returns a ZIP archive containing the FCP XML + the rendered MP4 file.
/// The XML uses filename-only paths so Premiere auto-links when both files
/// are in the same folder.
async fn download_premiere_package(
    State(deployment): State<DeploymentImpl>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let artifact = ExecutionArtifact::find_by_id(pool, artifact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Artifact {} not found", artifact_id)))?;

    match artifact.artifact_type {
        ArtifactType::VideoEditSession | ArtifactType::RenderDeliverable => {}
        _ => {
            return Err(ApiError::BadRequest(
                "Only video_edit_session and render_deliverable artifacts can be packaged".into(),
            ));
        }
    }

    // If file_path is already an XML file, serve it directly (no re-generation needed)
    if let Some(ref fp) = artifact.file_path {
        let is_xml = fp.to_lowercase().ends_with(".xml");
        if is_xml {
            let p = if std::path::Path::new(fp).is_absolute() {
                std::path::PathBuf::from(fp)
            } else {
                asset_dir().join(fp)
            };
            if p.is_file() {
                let xml_bytes = tokio::fs::read(&p)
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read XML: {e}")))?;
                let filename = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "sequence.xml".to_string());
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/xml")
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    )
                    .header(header::CONTENT_LENGTH, xml_bytes.len())
                    .body(Body::from(xml_bytes))
                    .map_err(|e| ApiError::InternalError(e.to_string()));
            }
        }
    }

    let content = artifact
        .content
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Artifact has no content".into()))?;

    let edit_data: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| ApiError::BadRequest(format!("Invalid artifact JSON: {}", e)))?;

    // Resolve the actual media file path
    let media_path: Option<PathBuf> = artifact.file_path.as_deref().and_then(|fp| {
        let p = std::path::Path::new(fp);
        if p.is_absolute() {
            Some(p.to_path_buf())
        } else {
            Some(asset_dir().join(fp))
        }
    });

    // Build XML using filename-only path so Premiere can relink from same folder
    let media_filename = media_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "video.mp4".to_string());

    // Inject filename into content for XML generation
    let mut edit_data_for_xml = edit_data.clone();
    let key = if edit_data_for_xml.get("deliverables").is_some() {
        "deliverables"
    } else {
        "edits"
    };
    if let Some(arr) = edit_data_for_xml
        .get_mut(key)
        .and_then(|v| v.as_array_mut())
    {
        if let Some(first) = arr.first_mut() {
            if let Some(obj) = first.as_object_mut() {
                obj.insert(
                    "file".to_string(),
                    serde_json::Value::String(media_filename.clone()),
                );
            }
        }
    }

    let edit = build_assembled_edit(&artifact.title, &edit_data_for_xml)?;
    let exporter = PremiereXmlExporter::new(edit.frame_rate);
    let xml_content = exporter.generate_xml(&edit);

    let safe_title = artifact
        .title
        .replace(' ', "-")
        .replace([':', '/', '\\', '(', ')'], "");
    let xml_filename = format!("{}.xml", safe_title);
    let zip_filename = format!("{}_Premiere-Package.zip", safe_title);

    // Write XML and media to a temp dir, then zip
    let tmp_dir = std::env::temp_dir().join(format!("premiere-pkg-{}", artifact_id));
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create tmp dir: {e}")))?;

    let xml_path = tmp_dir.join(&xml_filename);
    tokio::fs::write(&xml_path, xml_content.as_bytes())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to write XML: {e}")))?;

    let zip_path = tmp_dir.join(&zip_filename);

    // If we have a media file, include it in the zip
    if let Some(ref mp) = media_path {
        if mp.is_file() {
            let media_dest = tmp_dir.join(&media_filename);
            tokio::fs::copy(mp, &media_dest)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to copy media: {e}")))?;

            // zip XML + media
            tokio::process::Command::new("zip")
                .arg("-j") // junk paths (store just filename)
                .arg(&zip_path)
                .arg(&xml_path)
                .arg(&media_dest)
                .output()
                .await
                .map_err(|e| ApiError::BadRequest(format!("zip command failed: {e}")))?;
        } else {
            // zip XML only
            tokio::process::Command::new("zip")
                .arg("-j")
                .arg(&zip_path)
                .arg(&xml_path)
                .output()
                .await
                .map_err(|e| ApiError::BadRequest(format!("zip command failed: {e}")))?;
        }
    } else {
        tokio::process::Command::new("zip")
            .arg("-j")
            .arg(&zip_path)
            .arg(&xml_path)
            .output()
            .await
            .map_err(|e| ApiError::BadRequest(format!("zip command failed: {e}")))?;
    }

    let zip_bytes = tokio::fs::read(&zip_path)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read zip: {e}")))?;

    // Cleanup temp files
    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", zip_filename),
        )
        .header(header::CONTENT_LENGTH, zip_bytes.len())
        .body(Body::from(zip_bytes))
        .map_err(|e| ApiError::InternalError(e.to_string()))
}

/// Build an AssembledEdit from artifact JSON content.
/// Handles the Le Chateau edit session format: { edits: [{ name, duration_seconds, segments, file }] }
fn build_assembled_edit(title: &str, data: &serde_json::Value) -> Result<AssembledEdit, ApiError> {
    let frame_rate = data
        .get("frame_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(30.0) as f32;

    let width = data.get("width").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;

    let height = data.get("height").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;

    // Calculate total duration from edits array or top-level duration
    let duration = if let Some(edits) = data.get("edits").and_then(|e| e.as_array()) {
        edits
            .iter()
            .filter_map(|e| e.get("duration_seconds").and_then(|d| d.as_f64()))
            .sum::<f64>()
    } else if let Some(deliverables) = data.get("deliverables").and_then(|d| d.as_array()) {
        deliverables
            .iter()
            .filter_map(|d| d.get("duration_seconds").and_then(|v| v.as_f64()))
            .sum::<f64>()
    } else {
        data.get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(60.0)
    };

    let mut edit = AssembledEdit::new(title, duration, width, height, frame_rate);

    // Add video clips from edits or deliverables
    let clips_source = data
        .get("edits")
        .or_else(|| data.get("deliverables"))
        .and_then(|v| v.as_array());

    if let Some(clips) = clips_source {
        let mut timeline_pos = 0.0;
        for clip_data in clips {
            let clip_duration = clip_data
                .get("duration_seconds")
                .and_then(|d| d.as_f64())
                .unwrap_or(30.0);

            let source = clip_data
                .get("file")
                .or_else(|| clip_data.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown.mp4");

            let clip = TimelineClip::new(PathBuf::from(source), 0.0, clip_duration, timeline_pos);

            edit.video_clips.push(clip);
            timeline_pos += clip_duration;
        }
    }

    Ok(edit)
}

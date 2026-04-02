use std::path::PathBuf;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use db::models::execution_artifact::{ArtifactType, ExecutionArtifact};
use deployment::Deployment;
use serde::Deserialize;
use services::services::editron::{
    edit_assembly::{AssembledEdit, TimelineClip},
    premiere_xml::PremiereXmlExporter,
};
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/editron/export/{artifact_id}", get(export_artifact))
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

    let exporter = PremiereXmlExporter::new(edit.frame_rate);
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

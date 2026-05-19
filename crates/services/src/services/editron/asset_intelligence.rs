//! Media Asset Intelligence — AI analysis pipeline for uploaded media files.
//!
//! On ingest, calls `analyze_async(asset_id, file_path, project_id)` which spawns
//! a Tokio task that:
//!   1. Runs SceneDetectionEngine to extract shot metadata
//!   2. Extracts a keyframe via ffmpeg
//!   3. Calls Vision API via PCG Router for description/tags
//!   4. Updates media_assets with AI metadata
//!   5. Registers in project knowledge graph

use std::process::Command;

use db::models::{
    media_asset::MediaAsset,
    project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
};
use serde_json::json;
use sqlx::SqlitePool;
use tracing::{error, info};
use uuid::Uuid;

use crate::services::workflow_llm::WorkflowLLMService;

/// Kick off async AI analysis — returns immediately.
pub fn analyze_async(
    pool: SqlitePool,
    asset_id: Uuid,
    file_path: String,
    project_id: Uuid,
    filename: String,
) {
    tokio::spawn(async move {
        if let Err(e) = run_analysis(&pool, asset_id, &file_path, project_id, &filename).await {
            error!("Asset analysis failed for {}: {}", asset_id, e);
            let _ = sqlx::query(
                "UPDATE media_assets SET analysis_status='failed', \
                 updated_at=datetime('now','subsec') WHERE id=?",
            )
            .bind(asset_id)
            .execute(&pool)
            .await;
        }
    });
}

async fn run_analysis(
    pool: &SqlitePool,
    asset_id: Uuid,
    file_path: &str,
    project_id: Uuid,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mark running
    sqlx::query(
        "UPDATE media_assets SET analysis_status='running', \
         updated_at=datetime('now','subsec') WHERE id=?",
    )
    .bind(asset_id)
    .execute(pool)
    .await?;

    info!("Analysing media asset {} ({})", asset_id, filename);

    // ── Step 1: extract keyframe at midpoint via ffmpeg ───────────────────────
    let keyframe_path = format!("/tmp/kf_{}.jpg", asset_id.simple());
    let _ = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            file_path,
            "-vf",
            "thumbnail,scale=640:-1",
            "-frames:v",
            "1",
            "-ss",
            "00:00:05",
            &keyframe_path,
        ])
        .output(); // best-effort — continue even if ffmpeg unavailable

    // ── Step 2: base64 encode keyframe (if it exists) ────────────────────────
    let image_data = if std::path::Path::new(&keyframe_path).exists() {
        let bytes = std::fs::read(&keyframe_path)?;
        let _ = std::fs::remove_file(&keyframe_path);
        Some(base64_encode(&bytes))
    } else {
        None
    };

    // ── Step 3: Vision API via PCG Router ─────────────────────────────────────
    let (description, shot_type, energy_level, scene_tags, confidence) =
        call_vision_api(pool, filename, image_data).await?;

    // ── Step 4: Persist results ───────────────────────────────────────────────
    let tags_json = serde_json::to_string(&scene_tags).unwrap_or_else(|_| "[]".into());

    MediaAsset::update_ai_metadata(
        pool,
        asset_id,
        Some(description.clone()),
        Some(shot_type.clone()),
        energy_level,
        0.5, // motion_intensity placeholder
        "[]".to_string(),
        tags_json,
        confidence,
        "done",
    )
    .await?;

    // ── Step 5: Knowledge graph ───────────────────────────────────────────────
    let _ = ProjectKnowledgeSource::upsert_source(
        pool,
        project_id,
        &KnowledgeSourceType::Artifact,
        &asset_id.to_string(),
        filename,
        Some(&description),
        confidence,
    )
    .await;

    info!(
        "Asset {} analysis done (shot: {}, confidence: {:.0}%)",
        asset_id,
        shot_type,
        confidence * 100.0
    );
    Ok(())
}

/// Call Vision API via PCG Router and return (description, shot_type, energy_level, tags, confidence).
async fn call_vision_api(
    pool: &SqlitePool,
    filename: &str,
    image_b64: Option<String>,
) -> Result<(String, String, f64, Vec<String>, f64), Box<dyn std::error::Error + Send + Sync>> {
    let prompt = "Describe this shot for a creative media library. \
        Return JSON with these keys only: \
        {\"description\": \"Wide shot, rooftop event, golden hour lighting\", \
        \"shot_type\": \"wide|medium|close_up|extreme_close_up|aerial|other\", \
        \"energy_level\": 0.7, \
        \"tags\": [\"tag1\",\"tag2\"], \
        \"confidence\": 0.85}";

    // Build message content — vision models expect image blocks for Anthropic
    let user_content = if let Some(b64) = image_b64 {
        json!([
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/jpeg",
                    "data": b64
                }
            },
            {"type": "text", "text": prompt}
        ])
    } else {
        json!(format!("Filename: {}. {}", filename, prompt))
    };

    let messages = vec![json!({ "role": "user", "content": user_content })];

    // Use vision-capable model via PCG Router
    let result = WorkflowLLMService::completion(
        pool,
        messages,
        Some("claude-opus-4-6"), // Vision-capable model
        Some(512),
        None,
    )
    .await;

    let text = match result {
        Ok((text, metadata)) => {
            info!(
                "[ASSET_INTELLIGENCE] Vision API routed to {} ({})",
                metadata.model_used, metadata.provider
            );
            text
        }
        Err(e) => {
            // Fallback to filename-based description if no models available
            info!(
                "[ASSET_INTELLIGENCE] Vision API unavailable, using filename: {}",
                e
            );
            return Ok((
                format!("Media file: {}", filename),
                "unknown".into(),
                0.5,
                vec![],
                0.1,
            ));
        }
    };

    // Parse JSON from response text
    let parsed = extract_json(&text);

    let description = parsed
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or(filename)
        .to_string();
    let shot_type = parsed
        .get("shot_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let energy_level = parsed
        .get("energy_level")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let confidence = parsed
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let tags: Vec<String> = parsed
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok((description, shot_type, energy_level, tags, confidence))
}

fn extract_json(text: &str) -> serde_json::Value {
    if let Ok(v) = serde_json::from_str(text) {
        return v;
    }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if let Ok(v) = serde_json::from_str(&text[start..=end]) {
                return v;
            }
        }
    }
    serde_json::Value::Object(serde_json::Map::new())
}

fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(
            result,
            "{}",
            ALPHABET[((combined >> 18) & 63) as usize] as char
        );
        let _ = write!(
            result,
            "{}",
            ALPHABET[((combined >> 12) & 63) as usize] as char
        );
        let _ = write!(
            result,
            "{}",
            if chunk.len() > 1 {
                ALPHABET[((combined >> 6) & 63) as usize] as char
            } else {
                '='
            }
        );
        let _ = write!(
            result,
            "{}",
            if chunk.len() > 2 {
                ALPHABET[(combined & 63) as usize] as char
            } else {
                '='
            }
        );
    }
    result
}

//! Dashboard registration for editron-runner.
//!
//! Conditionally compiled behind `#[cfg(feature = "dashboard")]`.
//! Uses raw sqlx queries against the PCG SQLite database so we avoid
//! pulling in the full `db` crate (and its transitive dep chain).

use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

/// All the info the dashboard needs from a completed pipeline run.
pub struct PipelineResult {
    pub project_name: String,
    pub input_dir: String,
    pub output_dir: String,
    pub source_files: Vec<SourceFileInfo>,
    pub edits: Vec<EditInfo>,
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub bpm: f64,
    pub clips_used: u32,
    pub clips_available: u32,
    pub beat_locked_cuts: u32,
    pub processing_time_ms: u64,
}

pub struct SourceFileInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
}

pub struct EditInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub duration_seconds: f64,
}

/// Register pipeline results in the PCG dashboard database.
///
/// Creates media_batches, media_files, execution_artifacts, and optionally
/// a task with links. Non-fatal: returns Ok(()) even on partial failure.
pub async fn register(
    db_path: &str,
    project_id: &str,
    task_id: Option<&str>,
    result: &PipelineResult,
) -> Result<()> {
    let pool = connect(db_path).await?;

    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let batch_id = format!("editron-runner-{}", Utc::now().format("%Y%m%d-%H%M%S"));

    // 1. Media batch
    let total_bytes: u64 = result.source_files.iter().map(|f| f.size_bytes).sum();
    insert_media_batch(
        &pool,
        &batch_id,
        project_id,
        &result.project_name,
        &result.input_dir,
        result.source_files.len() as i64,
        total_bytes as i64,
        &now,
    )
    .await?;
    info!("[DASHBOARD] Media batch '{}' registered", batch_id);

    // 2. Media files
    for (i, file) in result.source_files.iter().enumerate() {
        let file_id = format!("{}-file-{:02}", batch_id, i + 1);
        insert_media_file(&pool, &file_id, &batch_id, file, &now).await?;
    }
    info!(
        "[DASHBOARD] {} media files registered",
        result.source_files.len()
    );

    // 3. Execution artifacts (4)
    let ingest_id = Uuid::new_v4();
    let analysis_id = Uuid::new_v4();
    let edit_id = Uuid::new_v4();
    let render_id = Uuid::new_v4();

    insert_artifact(
        &pool,
        ingest_id,
        "media_ingest_manifest",
        &format!("Ingest: {} ({} files)", result.project_name, result.source_files.len()),
        &ingest_content(&batch_id, result),
        None,
        &json!({"phase": "execution", "batch_id": &batch_id}),
        &now,
    )
    .await?;

    insert_artifact(
        &pool,
        analysis_id,
        "media_analysis_report",
        &format!("Analysis: {}", result.project_name),
        &analysis_content(&batch_id, result),
        None,
        &json!({"phase": "execution", "batch_id": &batch_id}),
        &now,
    )
    .await?;

    insert_artifact(
        &pool,
        edit_id,
        "video_edit_session",
        &format!("Edit Session: {} ({} edits)", result.project_name, result.edits.len()),
        &edit_session_content(&batch_id, result),
        Some(&result.output_dir),
        &json!({"phase": "execution", "batch_id": &batch_id}),
        &now,
    )
    .await?;

    insert_artifact(
        &pool,
        render_id,
        "render_deliverable",
        &format!("Deliverables: {} ({} MP4s)", result.project_name, result.edits.len()),
        &render_content(&batch_id, result),
        Some(&result.output_dir),
        &json!({"phase": "execution", "batch_id": &batch_id}),
        &now,
    )
    .await?;

    info!("[DASHBOARD] 4 execution artifacts created");

    // 4. Task + links
    let artifact_ids = [
        (ingest_id, "supporting", 3, false),
        (analysis_id, "supporting", 4, false),
        (edit_id, "primary", 2, false),
        (render_id, "primary", 1, true),
    ];

    let resolved_task_id = resolve_or_create_task(
        &pool,
        task_id,
        project_id,
        &result.project_name,
        &batch_id,
        &now,
    )
    .await;

    if let Some(tid) = resolved_task_id {
        for (aid, role, order, pinned) in &artifact_ids {
            link_artifact(&pool, tid, *aid, role, *order, *pinned).await;
        }
        info!("[DASHBOARD] Task {} linked to 4 artifacts", tid);
    }

    pool.close().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

async fn connect(db_path: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .context("Failed to connect to dashboard database")
}

async fn insert_media_batch(
    pool: &SqlitePool,
    id: &str,
    project_id: &str,
    name: &str,
    source_url: &str,
    file_count: i64,
    total_bytes: i64,
    now: &str,
) -> Result<()> {
    sqlx::query(
        r#"INSERT OR IGNORE INTO media_batches
           (id, project_id, reference_name, source_url, storage_tier,
            checksum_required, status, file_count, total_size_bytes,
            metadata, created_at, updated_at)
           VALUES (?1, ?2, ?3, ?4, 'hot', 0, 'analyzed',
                   ?5, ?6, '{}', ?7, ?7)"#,
    )
    .bind(id)
    .bind(project_id)
    .bind(format!("{} Source Footage", name))
    .bind(source_url)
    .bind(file_count)
    .bind(total_bytes)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_media_file(
    pool: &SqlitePool,
    id: &str,
    batch_id: &str,
    file: &SourceFileInfo,
    now: &str,
) -> Result<()> {
    sqlx::query(
        r#"INSERT OR IGNORE INTO media_files
           (id, batch_id, filename, file_path, size_bytes, metadata, created_at)
           VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)"#,
    )
    .bind(id)
    .bind(batch_id)
    .bind(&file.filename)
    .bind(&file.path)
    .bind(file.size_bytes as i64)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_artifact(
    pool: &SqlitePool,
    id: Uuid,
    artifact_type: &str,
    title: &str,
    content: &str,
    file_path: Option<&str>,
    metadata: &Value,
    now: &str,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO execution_artifacts
           (id, execution_process_id, artifact_type, title, content,
            file_path, metadata, phase, created_at)
           VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 'execution', ?7)"#,
    )
    .bind(id.as_bytes().as_slice())
    .bind(artifact_type)
    .bind(title)
    .bind(content)
    .bind(file_path)
    .bind(metadata.to_string())
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn resolve_or_create_task(
    pool: &SqlitePool,
    task_id: Option<&str>,
    project_id: &str,
    project_name: &str,
    batch_id: &str,
    now: &str,
) -> Option<Uuid> {
    // 1. Explicit task_id
    if let Some(tid_str) = task_id {
        if let Ok(tid) = Uuid::parse_str(tid_str) {
            let exists: Option<(Vec<u8>,)> =
                sqlx::query_as("SELECT id FROM tasks WHERE id = ?1")
                    .bind(tid.as_bytes().as_slice())
                    .fetch_optional(pool)
                    .await
                    .ok()?;
            if exists.is_some() {
                return Some(tid);
            }
            warn!("[DASHBOARD] task_id '{}' not found, creating new task", tid_str);
        }
    }

    // 2. Parse project_id — could be UUID string or hex blob
    let project_uuid = Uuid::parse_str(project_id).ok()?;

    // 3. Create new task
    let task_uuid = Uuid::new_v4();
    let title = format!("{} — Editron Video Production", project_name);
    let description = format!("Editron pipeline run for batch {}", batch_id);
    let custom_props = json!({"editron_batch_id": batch_id}).to_string();
    let tags = json!(["editron", "media"]).to_string();

    let res = sqlx::query(
        r#"INSERT INTO tasks
           (id, project_id, title, description, status,
            assigned_agent, tags, custom_properties,
            created_by, created_at, updated_at)
           VALUES (?1, ?2, ?3, ?4, 'in_progress', 'editron', ?5, ?6,
                   'editron-runner', ?7, ?7)"#,
    )
    .bind(task_uuid.as_bytes().as_slice())
    .bind(project_uuid.as_bytes().as_slice())
    .bind(&title)
    .bind(&description)
    .bind(&tags)
    .bind(&custom_props)
    .bind(now)
    .execute(pool)
    .await;

    match res {
        Ok(_) => {
            info!("[DASHBOARD] Created task '{}' ({})", title, task_uuid);
            Some(task_uuid)
        }
        Err(e) => {
            warn!("[DASHBOARD] Failed to create task: {}", e);
            None
        }
    }
}

async fn link_artifact(
    pool: &SqlitePool,
    task_id: Uuid,
    artifact_id: Uuid,
    role: &str,
    display_order: i32,
    pinned: bool,
) {
    let res = sqlx::query(
        r#"INSERT OR IGNORE INTO task_artifacts
           (task_id, artifact_id, artifact_role, display_order, pinned, added_by)
           VALUES (?1, ?2, ?3, ?4, ?5, 'editron-runner')"#,
    )
    .bind(task_id.as_bytes().as_slice())
    .bind(artifact_id.as_bytes().as_slice())
    .bind(role)
    .bind(display_order)
    .bind(pinned as i32)
    .execute(pool)
    .await;

    if let Err(e) = res {
        warn!("[DASHBOARD] Failed to link artifact {}: {}", artifact_id, e);
    }
}

// ---------------------------------------------------------------------------
// JSON content builders
// ---------------------------------------------------------------------------

fn ingest_content(batch_id: &str, result: &PipelineResult) -> String {
    let files: Vec<&str> = result.source_files.iter().map(|f| f.filename.as_str()).collect();
    let total_bytes: u64 = result.source_files.iter().map(|f| f.size_bytes).sum();
    json!({
        "batch_id": batch_id,
        "file_count": result.source_files.len(),
        "total_bytes": total_bytes,
        "source_dir": result.input_dir,
        "files": files,
    })
    .to_string()
}

fn analysis_content(batch_id: &str, result: &PipelineResult) -> String {
    json!({
        "batch_id": batch_id,
        "total_clips": result.source_files.len(),
        "usable_clips": result.clips_available,
        "clips_used": result.clips_used,
        "beat_locked_cuts": result.beat_locked_cuts,
        "bpm": result.bpm,
        "target_duration": result.duration_seconds,
        "resolution": format!("{}x{}", result.width, result.height),
    })
    .to_string()
}

fn edit_session_content(batch_id: &str, result: &PipelineResult) -> String {
    let edits: Vec<Value> = result
        .edits
        .iter()
        .map(|e| {
            json!({
                "file": e.filename,
                "duration_seconds": e.duration_seconds,
                "size_bytes": e.size_bytes,
            })
        })
        .collect();
    json!({
        "batch_id": batch_id,
        "edits": edits,
        "processing_time_ms": result.processing_time_ms,
    })
    .to_string()
}

fn render_content(batch_id: &str, result: &PipelineResult) -> String {
    let deliverables: Vec<Value> = result
        .edits
        .iter()
        .map(|e| {
            json!({
                "file": e.filename,
                "path": e.path,
                "size_bytes": e.size_bytes,
                "duration_seconds": e.duration_seconds,
            })
        })
        .collect();
    json!({
        "batch_id": batch_id,
        "deliverables": deliverables,
    })
    .to_string()
}

/// Collect source file info from a directory on disk.
pub async fn collect_source_info(dir: &Path) -> Vec<SourceFileInfo> {
    let mut files = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return files;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Ok(meta) = tokio::fs::metadata(&path).await {
                files.push(SourceFileInfo {
                    filename: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    path: path.to_string_lossy().to_string(),
                    size_bytes: meta.len(),
                });
            }
        }
    }
    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    files
}

/// Collect edit output info from the output directory.
pub async fn collect_edit_info(dir: &Path) -> Vec<EditInfo> {
    let mut edits = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return edits;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "mp4" {
                if let Ok(meta) = tokio::fs::metadata(&path).await {
                    edits.push(EditInfo {
                        filename: path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        path: path.to_string_lossy().to_string(),
                        size_bytes: meta.len(),
                        duration_seconds: 0.0, // filled from assembly result if available
                    });
                }
            }
        }
    }
    edits.sort_by(|a, b| a.filename.cmp(&b.filename));
    edits
}

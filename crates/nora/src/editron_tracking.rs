//! Editron dashboard tracking helpers.
//!
//! Creates artifacts, activity logs, and VIBE transactions on workflow tasks
//! when Editron media tools execute.

use db::models::{
    activity::{ActorType, ActivityLog, CreateActivityLog},
    execution_artifact::{ArtifactType, CreateExecutionArtifact, ExecutionArtifact},
    task::{CreateTask, Priority, Task},
    task_artifact::{ArtifactRole, LinkArtifactToTask, TaskArtifact},
    vibe_transaction::{CreateVibeTransaction, VibeSourceType, VibeTransaction},
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::executor::TaskExecutor;

// ---------------------------------------------------------------------------
// Vibe cost schedule
// ---------------------------------------------------------------------------

pub struct EditronVibeCosts;

impl EditronVibeCosts {
    /// 500 base + 10 per file
    pub fn ingest(file_count: u32) -> i64 {
        500 + (file_count as i64) * 10
    }

    /// 2000 base + 500 per pass
    pub fn analyze(passes: u32) -> i64 {
        2000 + (passes as i64) * 500
    }

    /// 3000 base + 200 per aspect ratio
    pub fn generate(aspect_ratio_count: usize) -> i64 {
        3000 + (aspect_ratio_count as i64) * 200
    }

    /// 5000 base + 1000 per format, 2x for Rush priority
    pub fn render(format_count: usize, is_rush: bool) -> i64 {
        let base = 5000 + (format_count as i64) * 1000;
        if is_rush { base * 2 } else { base }
    }
}

// ---------------------------------------------------------------------------
// Activity logging
// ---------------------------------------------------------------------------

pub async fn log_editron_activity(
    pool: &SqlitePool,
    task_id: Uuid,
    action: &str,
    message: &str,
    vibe_cost: i64,
    extra_metadata: Value,
) -> Result<ActivityLog, sqlx::Error> {
    let mut meta = extra_metadata;
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("vibe_cost".to_string(), json!(vibe_cost));
        obj.insert("message".to_string(), json!(message));
    }

    ActivityLog::create(
        pool,
        &CreateActivityLog {
            task_id,
            actor_id: "editron".to_string(),
            actor_type: ActorType::Agent,
            action: action.to_string(),
            previous_state: None,
            new_state: None,
            metadata: Some(meta),
        },
    )
    .await
}

// ---------------------------------------------------------------------------
// VIBE transactions
// ---------------------------------------------------------------------------

pub async fn record_editron_vibe(
    pool: &SqlitePool,
    project_id: Uuid,
    task_id: Uuid,
    amount: i64,
    description: &str,
    operation_type: &str,
    extra_metadata: Value,
) -> Result<VibeTransaction, Box<dyn std::error::Error + Send + Sync>> {
    let cost_cents = (amount as f64 * 0.1) as i64; // 1 VIBE = $0.001 → cents = amount / 10

    let tx = VibeTransaction::create(
        pool,
        CreateVibeTransaction {
            source_type: VibeSourceType::Project,
            source_id: project_id,
            amount_vibe: amount,
            input_tokens: None,
            output_tokens: None,
            model: Some(format!("editron-{}", operation_type)),
            provider: Some("editron".to_string()),
            calculated_cost_cents: Some(cost_cents),
            task_id: Some(task_id),
            task_attempt_id: None,
            process_id: None,
            description: Some(description.to_string()),
            metadata: Some(extra_metadata),
        },
    )
    .await?;

    Ok(tx)
}

// ---------------------------------------------------------------------------
// Artifact creation + linking
// ---------------------------------------------------------------------------

pub async fn create_and_link_artifact(
    pool: &SqlitePool,
    task_id: Uuid,
    artifact_type: ArtifactType,
    title: &str,
    content: Option<String>,
    file_path: Option<String>,
    metadata: Value,
    role: ArtifactRole,
) -> Result<ExecutionArtifact, Box<dyn std::error::Error + Send + Sync>> {
    // Ensure execution phase is set in metadata
    let mut meta = metadata;
    if let Some(obj) = meta.as_object_mut() {
        obj.entry("phase".to_string()).or_insert(json!("execution"));
    }

    let artifact = ExecutionArtifact::create(
        pool,
        CreateExecutionArtifact {
            execution_process_id: None, // Editron artifacts aren't tied to git execution processes
            artifact_type,
            title: title.to_string(),
            content,
            file_path,
            metadata: Some(meta),
        },
    )
    .await?;

    // Link artifact to the task
    let _ = TaskArtifact::link(
        pool,
        LinkArtifactToTask {
            task_id,
            artifact_id: artifact.id,
            artifact_role: Some(role),
            display_order: None,
            pinned: None,
            added_by: Some("editron".to_string()),
        },
    )
    .await;

    Ok(artifact)
}

// ---------------------------------------------------------------------------
// Task resolution: find existing or create new
// ---------------------------------------------------------------------------

/// Resolve or create a task for Editron tracking.
///
/// Resolution order:
/// 1. `task_id` provided → validate and use it
/// 2. `batch_id` provided → look up task by custom_properties.editron_batch_id
/// 3. Neither → create a new task in the given project
/// 4. No project_id → return None (skip tracking)
pub async fn find_or_create_task(
    pool: &SqlitePool,
    executor: &TaskExecutor,
    task_id: Option<&str>,
    project_id: Option<&str>,
    title: &str,
    description: &str,
    custom_props: Value,
) -> Option<(Uuid, Uuid)> {
    // 1. Explicit task_id provided (workflow path)
    if let Some(tid) = task_id {
        if let Ok(task_uuid) = Uuid::parse_str(tid) {
            if let Ok(Some(task)) = Task::find_by_id(pool, task_uuid).await {
                return Some((task.id, task.project_id));
            }
        }
        tracing::warn!("[EDITRON_TRACKING] task_id '{}' not found, falling through", tid);
    }

    // 2. Try to find task by batch_id in custom_properties
    if let Some(batch_id) = custom_props.get("editron_batch_id").and_then(|v| v.as_str()) {
        if let Some((tid, pid)) = find_workflow_task_by_batch_id(pool, batch_id).await {
            return Some((tid, pid));
        }
    }

    // 3. Create new task if we have a project_id
    let project_uuid = project_id.and_then(|p| Uuid::parse_str(p).ok())?;

    let task_id = Uuid::new_v4();
    let create = CreateTask {
        project_id: project_uuid,
        pod_id: None,
        board_id: None,
        title: title.to_string(),
        description: Some(description.to_string()),
        parent_task_attempt: None,
        image_ids: None,
        priority: Some(Priority::High),
        assignee_id: None,
        assigned_agent: Some("editron".to_string()),
        agent_id: None,
        assigned_mcps: None,
        created_by: "editron".to_string(),
        requires_approval: Some(false),
        parent_task_id: None,
        tags: Some(vec!["editron".to_string(), "media".to_string()]),
        due_date: None,
        custom_properties: Some(custom_props),
        scheduled_start: None,
        scheduled_end: None,
    };

    match Task::create(pool, &create, task_id).await {
        Ok(task) => {
            tracing::info!(
                "[EDITRON_TRACKING] Created task '{}' ({})",
                task.title,
                task.id
            );

            // Try to assign to a board
            if let Ok(Some(board)) = executor.get_default_board_for_tasks(project_uuid).await {
                let _ = executor.add_task_to_board(task.id, board.id).await;
            }

            Some((task.id, project_uuid))
        }
        Err(e) => {
            tracing::error!("[EDITRON_TRACKING] Failed to create task: {}", e);
            None
        }
    }
}

/// Look up a task whose custom_properties contains the given editron_batch_id.
async fn find_workflow_task_by_batch_id(
    pool: &SqlitePool,
    batch_id: &str,
) -> Option<(Uuid, Uuid)> {
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"SELECT id, project_id
           FROM tasks
           WHERE json_extract(custom_properties, '$.editron_batch_id') = ?1
           LIMIT 1"#,
    )
    .bind(batch_id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row
}

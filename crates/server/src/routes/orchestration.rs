//! Orchestration Task Routes
//!
//! REST API and SSE endpoints for managing orchestration tasks within agent flows.

use std::{convert::Infallible, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use db::models::orchestration_task::{
    CreateOrchestrationTask, OrchestrationTask, OrchestrationTaskStatus, OrchestrationTaskType,
};
use deployment::Deployment;
use futures_util::stream;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

// ── Request/Response Types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTaskRequest {
    pub task_type: OrchestrationTaskType,
    pub description: String,
    #[serde(default)]
    pub is_concurrency_safe: bool,
    pub tool_use_id: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OrchestrationTaskResponse {
    pub task: OrchestrationTask,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OrchestrationTasksResponse {
    pub tasks: Vec<OrchestrationTask>,
    pub pending_count: usize,
    pub running_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/orchestration/{flow_id}/tasks",
            get(list_tasks).post(create_task),
        )
        .route("/orchestration/{flow_id}/tasks/{task_id}", get(get_task))
        .route(
            "/orchestration/{flow_id}/tasks/{task_id}/kill",
            post(kill_task),
        )
        .route("/orchestration/{flow_id}/stream", get(task_stream))
        .with_state(deployment.clone())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List all orchestration tasks for a flow
async fn list_tasks(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<String>,
) -> Result<Json<ApiResponse<OrchestrationTasksResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let tasks = OrchestrationTask::find_by_flow_id(pool, &flow_id).await?;

    let pending_count = tasks
        .iter()
        .filter(|t| t.status == OrchestrationTaskStatus::Pending)
        .count();
    let running_count = tasks
        .iter()
        .filter(|t| t.status == OrchestrationTaskStatus::Running)
        .count();
    let completed_count = tasks
        .iter()
        .filter(|t| t.status == OrchestrationTaskStatus::Completed)
        .count();
    let failed_count = tasks
        .iter()
        .filter(|t| t.status == OrchestrationTaskStatus::Failed)
        .count();

    Ok(Json(ApiResponse::success(OrchestrationTasksResponse {
        tasks,
        pending_count,
        running_count,
        completed_count,
        failed_count,
    })))
}

/// Get a single orchestration task
async fn get_task(
    State(deployment): State<DeploymentImpl>,
    Path((flow_id, task_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<OrchestrationTaskResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let task = OrchestrationTask::find_by_id(pool, &task_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Task {} not found", task_id)))?;

    // Verify task belongs to the specified flow
    if task.agent_flow_id != flow_id {
        return Err(ApiError::BadRequest(format!(
            "Task {} does not belong to flow {}",
            task_id, flow_id
        )));
    }

    Ok(Json(ApiResponse::success(OrchestrationTaskResponse {
        task,
    })))
}

/// Create a new orchestration task
async fn create_task(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<String>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<ApiResponse<OrchestrationTaskResponse>>), ApiError> {
    let pool = &deployment.db().pool;

    // Get current position (append to end)
    let existing_tasks = OrchestrationTask::find_by_flow_id(pool, &flow_id).await?;
    let position = existing_tasks.len() as i32;

    let task = OrchestrationTask::create(
        pool,
        CreateOrchestrationTask {
            agent_flow_id: flow_id,
            task_type: request.task_type,
            description: request.description,
            tool_use_id: request.tool_use_id,
            is_concurrency_safe: request.is_concurrency_safe,
            position,
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(OrchestrationTaskResponse { task })),
    ))
}

/// Kill a running task
async fn kill_task(
    State(deployment): State<DeploymentImpl>,
    Path((flow_id, task_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<OrchestrationTaskResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify task exists and belongs to flow
    let existing = OrchestrationTask::find_by_id(pool, &task_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Task {} not found", task_id)))?;

    if existing.agent_flow_id != flow_id {
        return Err(ApiError::BadRequest(format!(
            "Task {} does not belong to flow {}",
            task_id, flow_id
        )));
    }

    // Only running tasks can be killed
    if existing.status != OrchestrationTaskStatus::Running {
        return Err(ApiError::BadRequest(format!(
            "Task {} is not running (status: {:?})",
            task_id, existing.status
        )));
    }

    let task = OrchestrationTask::kill(pool, &task_id).await?;

    Ok(Json(ApiResponse::success(OrchestrationTaskResponse {
        task,
    })))
}

/// SSE stream for task updates
///
/// Emits events when tasks change status. Connect to this endpoint
/// and receive real-time updates about orchestration task progress.
async fn task_stream(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<String>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let pool = deployment.db().pool.clone();

    // Poll-based implementation: check for changes every second
    // In a production system, this would use database change notifications or pub/sub
    let stream = stream::unfold(
        (pool, flow_id, None::<Vec<OrchestrationTask>>),
        |(pool, flow_id, prev_tasks)| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let tasks = OrchestrationTask::find_by_flow_id(&pool, &flow_id)
                .await
                .ok()?;

            // Detect changes
            let changed = match &prev_tasks {
                None => true, // First poll
                Some(prev) => {
                    // Check if any task status changed
                    tasks.iter().any(|t| {
                        prev.iter()
                            .find(|p| p.id == t.id)
                            .map(|p| p.status != t.status || p.output != t.output)
                            .unwrap_or(true)
                    })
                }
            };

            if changed {
                let event_data = serde_json::json!({
                    "type": "tasks_updated",
                    "flow_id": flow_id,
                    "tasks": tasks,
                    "pending_count": tasks.iter().filter(|t| t.status == OrchestrationTaskStatus::Pending).count(),
                    "running_count": tasks.iter().filter(|t| t.status == OrchestrationTaskStatus::Running).count(),
                    "completed_count": tasks.iter().filter(|t| t.status == OrchestrationTaskStatus::Completed).count(),
                    "failed_count": tasks.iter().filter(|t| t.status == OrchestrationTaskStatus::Failed).count(),
                });

                let event = Event::default()
                    .event("orchestration")
                    .data(event_data.to_string());

                Some((Ok(event), (pool, flow_id, Some(tasks))))
            } else {
                // Send keepalive comment
                let event = Event::default().comment("keepalive");
                Some((Ok(event), (pool, flow_id, Some(tasks))))
            }
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::default())
}

use std::path::PathBuf;

use anyhow;
use axum::{
    Extension, Json, Router,
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json as ResponseJson},
    routing::{get, post},
};
use db::models::{
    agent_wallet::{AgentWallet, AgentWalletTransaction, CreateWalletTransaction},
    image::TaskImage,
    project::Project,
    project_board::ProjectBoard,
    project_pod::ProjectPod,
    task::{CreateTask, Task, TaskWithAttemptStatus, UpdateTask},
    task_attempt::{CreateTaskAttempt, TaskAttempt},
    vibe_transaction::{CreateVibeTransaction, VibeSourceType, VibeTransaction},
};
use deployment::Deployment;
use executors::profile::ExecutorProfileId;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use serde::Deserialize;
use serde_json::json;
use services::services::container::{
    ContainerService, WorktreeCleanupData, cleanup_worktrees_direct,
};
use sqlx::{Error as SqlxError, types::Json as SqlxJson};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_task_middleware, middleware::access_control::AccessContext};

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    pub project_id: Uuid,
}

pub async fn get_tasks(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskWithAttemptStatus>>>, ApiError> {
    let tasks =
        Task::find_by_project_id_with_attempt_status(&deployment.db().pool, query.project_id)
            .await?;

    Ok(ResponseJson(ApiResponse::success(tasks)))
}

pub async fn stream_tasks_ws(
    ws: WebSocketUpgrade,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_tasks_ws(socket, deployment, query.project_id).await {
            tracing::warn!("tasks WS closed: {}", e);
        }
    })
}

async fn handle_tasks_ws(
    socket: WebSocket,
    deployment: DeploymentImpl,
    project_id: Uuid,
) -> anyhow::Result<()> {
    // Get the raw stream and convert LogMsg to WebSocket messages
    let mut stream = deployment
        .events()
        .stream_tasks_raw(project_id)
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Drain (and ignore) any client->server messages so pings/pongs work
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    // Forward server messages
    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break; // client disconnected
                }
            }
            Err(e) => {
                tracing::error!("stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

pub async fn get_task(
    Extension(task): Extension<Task>,
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(task)))
}

pub async fn create_task(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let id = Uuid::new_v4();

    tracing::debug!(
        "Creating task '{}' in project {}",
        payload.title,
        payload.project_id
    );

    if let Some(pod_id) = payload.pod_id {
        match ProjectPod::find_by_id(&deployment.db().pool, pod_id).await? {
            Some(pod) if pod.project_id == payload.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Pod does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Pod not found".to_string())),
        }
    }

    if let Some(board_id) = payload.board_id {
        match ProjectBoard::find_by_id(&deployment.db().pool, board_id).await? {
            Some(board) if board.project_id == payload.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }

    let task = Task::create(&deployment.db().pool, &payload, id).await?;

    if let Some(image_ids) = &payload.image_ids {
        TaskImage::associate_many_dedup(&deployment.db().pool, task.id, image_ids).await?;
    }

    deployment
        .track_if_analytics_allowed(
            "task_created",
            serde_json::json!({
            "task_id": task.id.to_string(),
            "project_id": payload.project_id,
            "has_description": task.description.is_some(),
            "has_images": payload.image_ids.is_some(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(task)))
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAndStartTaskRequest {
    pub task: CreateTask,
    pub executor_profile_id: ExecutorProfileId,
    pub base_branch: String,
}

pub async fn create_task_and_start(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAndStartTaskRequest>,
) -> Result<ResponseJson<ApiResponse<TaskWithAttemptStatus>>, ApiError> {
    let CreateAndStartTaskRequest {
        task: task_payload,
        executor_profile_id,
        base_branch,
    } = payload;

    if let Some(pod_id) = task_payload.pod_id {
        match ProjectPod::find_by_id(&deployment.db().pool, pod_id).await? {
            Some(pod) if pod.project_id == task_payload.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Pod does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Pod not found".to_string())),
        }
    }

    if let Some(board_id) = task_payload.board_id {
        match ProjectBoard::find_by_id(&deployment.db().pool, board_id).await? {
            Some(board) if board.project_id == task_payload.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }

    let task_id = Uuid::new_v4();
    let task = Task::create(&deployment.db().pool, &task_payload, task_id).await?;

    if let Some(image_ids) = &task_payload.image_ids {
        TaskImage::associate_many(&deployment.db().pool, task.id, image_ids).await?;
    }

    deployment
        .track_if_analytics_allowed(
            "task_created",
            serde_json::json!({
                "task_id": task.id.to_string(),
                "project_id": task.project_id,
                "has_description": task.description.is_some(),
                "has_images": task_payload.image_ids.is_some(),
            }),
        )
        .await;

    // ========================================
    // VIBE Budget Enforcement
    // ========================================

    // Estimated VIBE cost for task execution
    // This is a conservative estimate; actual cost will be calculated after LLM usage
    const ESTIMATED_VIBE_COST: i64 = 100; // ~$0.10 USD worth of VIBE

    // Check project VIBE budget
    if let Some(project) = Project::find_by_id(&deployment.db().pool, task_payload.project_id).await? {
        if !project.has_vibe_budget(ESTIMATED_VIBE_COST) {
            let remaining = project.remaining_vibe().unwrap_or(0);
            return Err(ApiError::PaymentRequired(format!(
                "Project VIBE budget exceeded. Remaining: {} VIBE, Required: {} VIBE (~${})",
                remaining,
                ESTIMATED_VIBE_COST,
                ESTIMATED_VIBE_COST as f64 * 0.001
            )));
        }
    }

    // Check agent wallet VIBE budget and APT budget
    if let Some(wallet) =
        AgentWallet::find_by_profile_key(&deployment.db().pool, &executor_profile_id.to_string())
            .await?
    {
        // Check VIBE budget first
        if !wallet.has_vibe_budget(ESTIMATED_VIBE_COST) {
            let remaining = wallet.remaining_vibe().unwrap_or(0);
            return Err(ApiError::PaymentRequired(format!(
                "Agent VIBE budget exceeded. Remaining: {} VIBE, Required: {} VIBE (~${})",
                remaining,
                ESTIMATED_VIBE_COST,
                ESTIMATED_VIBE_COST as f64 * 0.001
            )));
        }

        // Also check APT budget for legacy support
        const EXECUTION_DEBIT: i64 = 1;
        let remaining = wallet.budget_limit - wallet.spent_amount;
        if remaining < EXECUTION_DEBIT {
            return Err(ApiError::Conflict(
                "Agent wallet APT budget exceeded for this profile".to_string(),
            ));
        }

        let metadata = json!({
            "task_id": task.id,
            "executor_profile": executor_profile_id.to_string(),
            "estimated_vibe_cost": ESTIMATED_VIBE_COST,
        })
        .to_string();

        // Record APT debit transaction (legacy)
        AgentWalletTransaction::create(
            &deployment.db().pool,
            &CreateWalletTransaction {
                wallet_id: wallet.id,
                direction: "debit".to_string(),
                amount: EXECUTION_DEBIT,
                description: Some("Task attempt start".to_string()),
                metadata: Some(metadata.clone()),
                task_id: Some(task.id),
                process_id: None,
            },
        )
        .await?;

        // Record VIBE transaction for cost tracking (pending - actual cost calculated after LLM usage)
        let _ = VibeTransaction::create(
            &deployment.db().pool,
            CreateVibeTransaction {
                source_type: VibeSourceType::Agent,
                source_id: wallet.id,
                amount_vibe: 0, // Will be updated after actual LLM usage
                input_tokens: None,
                output_tokens: None,
                model: None,
                provider: None,
                calculated_cost_cents: None,
                task_id: Some(task.id),
                task_attempt_id: None, // Will be set after attempt creation
                process_id: None,
                description: Some("Task execution started - pending cost calculation".to_string()),
                metadata: Some(serde_json::from_str(&metadata).unwrap_or_default()),
            },
        )
        .await;
    }

    let task_attempt = TaskAttempt::create(
        &deployment.db().pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor,
            base_branch,
        },
        task.id,
    )
    .await?;
    let execution_process = deployment
        .container()
        .start_attempt(&task_attempt, executor_profile_id.clone())
        .await?;
    deployment
        .track_if_analytics_allowed(
            "task_attempt_started",
            serde_json::json!({
                "task_id": task.id.to_string(),
                "executor": &executor_profile_id.executor,
                "variant": &executor_profile_id.variant,
                "attempt_id": task_attempt.id.to_string(),
            }),
        )
        .await;

    let task = Task::find_by_id(&deployment.db().pool, task.id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    tracing::info!("Started execution process {}", execution_process.id);
    Ok(ResponseJson(ApiResponse::success(TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: true,
        has_merged_attempt: false,
        last_attempt_failed: false,
        executor: task_attempt.executor,
        last_execution_summary: None,
        collaborators: None,
    })))
}

pub async fn update_task(
    Extension(existing_task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    // Use existing values if not provided in update
    let title = payload.title.unwrap_or(existing_task.title.clone());
    let description = payload.description.or(existing_task.description.clone());
    let status = payload.status.unwrap_or(existing_task.status.clone());
    let parent_task_attempt = payload
        .parent_task_attempt
        .or(existing_task.parent_task_attempt);
    let pod_change = payload.pod_id.clone();
    if let Some(Some(pod_id)) = pod_change.as_ref() {
        match ProjectPod::find_by_id(&deployment.db().pool, *pod_id).await? {
            Some(pod) if pod.project_id == existing_task.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Pod does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Pod not found".to_string())),
        }
    }
    let pod_id = pod_change.unwrap_or(existing_task.pod_id);
    let board_change = payload.board_id.clone();
    if let Some(Some(board_id)) = board_change.as_ref() {
        match ProjectBoard::find_by_id(&deployment.db().pool, *board_id).await? {
            Some(board) if board.project_id == existing_task.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }
    let board_id = board_change.unwrap_or(existing_task.board_id);
    let priority = payload.priority.unwrap_or(existing_task.priority.clone());
    let assignee_id = payload.assignee_id.or(existing_task.assignee_id.clone());
    let assigned_agent = payload
        .assigned_agent
        .or(existing_task.assigned_agent.clone());
    let assigned_mcps = if let Some(mcps) = &payload.assigned_mcps {
        Some(serde_json::to_string(mcps).unwrap())
    } else {
        existing_task.assigned_mcps.clone()
    };
    let requires_approval = payload
        .requires_approval
        .unwrap_or(existing_task.requires_approval);
    let approval_status = payload
        .approval_status
        .or(existing_task.approval_status.clone());
    let parent_task_id = payload.parent_task_id.or(existing_task.parent_task_id);
    let tags = if let Some(tags) = &payload.tags {
        Some(serde_json::to_string(tags).unwrap())
    } else {
        existing_task.tags.clone()
    };
    let due_date = payload.due_date.or(existing_task.due_date);
    let custom_properties_value = match &payload.custom_properties {
        Some(inner) => inner.clone(),
        None => existing_task
            .custom_properties
            .as_ref()
            .map(|json| json.0.clone()),
    };
    let custom_properties = custom_properties_value.map(SqlxJson);
    let scheduled_start = payload
        .scheduled_start
        .clone()
        .unwrap_or(existing_task.scheduled_start);
    let scheduled_end = payload
        .scheduled_end
        .clone()
        .unwrap_or(existing_task.scheduled_end);

    let task = Task::update(
        &deployment.db().pool,
        existing_task.id,
        existing_task.project_id,
        title,
        description,
        status,
        parent_task_attempt,
        pod_id,
        board_id,
        priority,
        assignee_id,
        assigned_agent,
        assigned_mcps,
        requires_approval,
        approval_status,
        parent_task_id,
        tags,
        due_date,
        custom_properties,
        scheduled_start,
        scheduled_end,
    )
    .await?;

    if let Some(image_ids) = &payload.image_ids {
        TaskImage::delete_by_task_id(&deployment.db().pool, task.id).await?;
        TaskImage::associate_many_dedup(&deployment.db().pool, task.id, image_ids).await?;
    }

    Ok(ResponseJson(ApiResponse::success(task)))
}

pub async fn delete_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    // Validate no running execution processes
    if deployment
        .container()
        .has_running_processes(task.id)
        .await?
    {
        return Err(ApiError::Conflict("Task has running execution processes. Please wait for them to complete or stop them first.".to_string()));
    }

    // Gather task attempts data needed for background cleanup
    let attempts = TaskAttempt::fetch_all(&deployment.db().pool, Some(task.id))
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch task attempts for task {}: {}", task.id, e);
            ApiError::TaskAttempt(e)
        })?;

    // Gather cleanup data before deletion
    let project = task
        .parent_project(&deployment.db().pool)
        .await?
        .ok_or_else(|| ApiError::Database(SqlxError::RowNotFound))?;

    let cleanup_data: Vec<WorktreeCleanupData> = attempts
        .iter()
        .filter_map(|attempt| {
            attempt
                .container_ref
                .as_ref()
                .map(|worktree_path| WorktreeCleanupData {
                    attempt_id: attempt.id,
                    worktree_path: PathBuf::from(worktree_path),
                    git_repo_path: Some(project.git_repo_path.clone()),
                })
        })
        .collect();

    // Delete task from database (FK CASCADE will handle task_attempts)
    let rows_affected = Task::delete(&deployment.db().pool, task.id).await?;

    if rows_affected == 0 {
        return Err(ApiError::Database(SqlxError::RowNotFound));
    }

    // Spawn background worktree cleanup task
    let task_id = task.id;
    tokio::spawn(async move {
        let span = tracing::info_span!("background_worktree_cleanup", task_id = %task_id);
        let _enter = span.enter();

        tracing::info!(
            "Starting background cleanup for task {} ({} worktrees)",
            task_id,
            cleanup_data.len()
        );

        if let Err(e) = cleanup_worktrees_direct(&cleanup_data).await {
            tracing::error!(
                "Background worktree cleanup failed for task {}: {}",
                task_id,
                e
            );
        } else {
            tracing::info!("Background cleanup completed for task {}", task_id);
        }
    });

    // Return 202 Accepted to indicate deletion was scheduled
    Ok((StatusCode::ACCEPTED, ResponseJson(ApiResponse::success(()))))
}

// Phase C: Approval workflow endpoints
pub async fn approve_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    use db::models::task::ApprovalStatus;

    let approved_task = Task::update(
        &deployment.db().pool,
        task.id,
        task.project_id,
        task.title,
        task.description,
        task.status,
        task.parent_task_attempt,
        task.pod_id,
        task.board_id,
        task.priority,
        task.assignee_id,
        task.assigned_agent,
        task.assigned_mcps,
        task.requires_approval,
        Some(ApprovalStatus::Approved),
        task.parent_task_id,
        task.tags,
        task.due_date,
        task.custom_properties.clone(),
        task.scheduled_start,
        task.scheduled_end,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(approved_task)))
}

pub async fn request_changes(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    use db::models::task::ApprovalStatus;

    let updated_task = Task::update(
        &deployment.db().pool,
        task.id,
        task.project_id,
        task.title,
        task.description,
        task.status,
        task.parent_task_attempt,
        task.pod_id,
        task.board_id,
        task.priority,
        task.assignee_id,
        task.assigned_agent,
        task.assigned_mcps,
        task.requires_approval,
        Some(ApprovalStatus::ChangesRequested),
        task.parent_task_id,
        task.tags,
        task.due_date,
        task.custom_properties.clone(),
        task.scheduled_start,
        task.scheduled_end,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(updated_task)))
}

/// Response type for assigned tasks including project name
#[derive(Debug, serde::Serialize, TS)]
pub struct AssignedTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub project_id: String,
    pub project_name: String,
}

/// Get all tasks assigned to the current user
pub async fn get_assigned_to_me(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AssignedTask>>>, ApiError> {
    use db::models::project::Project;

    let user_id_str = access_context.user_id.to_string();
    let tasks = Task::find_by_assignee(&deployment.db().pool, &user_id_str).await?;

    // Collect unique project IDs and fetch project names
    let mut project_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for task in &tasks {
        let project_id = task.project_id.to_string();
        if !project_names.contains_key(&project_id) {
            if let Ok(Some(project)) = Project::find_by_id(&deployment.db().pool, task.project_id).await {
                project_names.insert(project_id, project.name);
            }
        }
    }

    let assigned_tasks: Vec<AssignedTask> = tasks
        .into_iter()
        .map(|task| {
            let project_id = task.project_id.to_string();
            // Convert Priority enum to string using serde
            let priority_str = serde_json::to_value(&task.priority)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "low".to_string());
            // Convert TaskStatus enum to string using serde
            let status_str = serde_json::to_value(&task.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "todo".to_string());
            AssignedTask {
                id: task.id.to_string(),
                title: task.title,
                status: status_str,
                priority: priority_str,
                due_date: task.due_date.map(|d| d.to_rfc3339()),
                project_name: project_names.get(&project_id).cloned().unwrap_or_else(|| "Unknown".to_string()),
                project_id,
            }
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(assigned_tasks)))
}

pub async fn reject_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    use db::models::task::ApprovalStatus;

    let rejected_task = Task::update(
        &deployment.db().pool,
        task.id,
        task.project_id,
        task.title,
        task.description,
        task.status,
        task.parent_task_attempt,
        task.pod_id,
        task.board_id,
        task.priority,
        task.assignee_id,
        task.assigned_agent,
        task.assigned_mcps,
        task.requires_approval,
        Some(ApprovalStatus::Rejected),
        task.parent_task_id,
        task.tags,
        task.due_date,
        task.custom_properties.clone(),
        task.scheduled_start,
        task.scheduled_end,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(rejected_task)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let task_id_router = Router::new()
        .route("/", get(get_task).put(update_task).delete(delete_task))
        .route("/approve", post(approve_task))
        .route("/request-changes", post(request_changes))
        .route("/reject", post(reject_task))
        .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

    let inner = Router::new()
        .route("/", get(get_tasks).post(create_task))
        .route("/stream/ws", get(stream_tasks_ws))
        .route("/create-and-start", post(create_task_and_start))
        .nest("/{task_id}", task_id_router);

    // mount under /projects/:project_id/tasks
    Router::new().nest("/tasks", inner)
}

/// Global tasks router - mounts at /api/tasks (not nested under projects)
pub fn global_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/tasks/assigned-to-me", get(get_assigned_to_me))
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::middleware::require_auth,
        ))
}

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
    routing::{delete, get, post},
};
use db::models::{
    activity::{ActivityLog, ActorType, CreateActivityLog},
    agent::Agent,
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

/// Broadcast a task event to all connected WebSocket clients
fn broadcast_task_event(deployment: &DeploymentImpl, op: &str, task_id: &str, task: Option<&TaskWithAttemptStatus>) {
    let patch = match op {
        "add" | "replace" => {
            if let Some(t) = task {
                json!([{
                    "op": op,
                    "path": format!("/tasks/{}", task_id),
                    "value": t
                }])
            } else {
                return;
            }
        }
        "remove" => {
            json!([{
                "op": "remove",
                "path": format!("/tasks/{}", task_id)
            }])
        }
        _ => return,
    };

    if let Ok(patch) = serde_json::from_value::<json_patch::Patch>(patch) {
        deployment.events().msg_store().push_patch(patch);
    }
}

/// Convert a Task to TaskWithAttemptStatus with default values for a fresh task
fn task_to_with_attempt_status(task: Task) -> TaskWithAttemptStatus {
    TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: false,
        has_merged_attempt: false,
        last_attempt_failed: false,
        executor: String::new(),
        last_execution_summary: None,
        collaborators: None,
        vibe_cost: None,
        vibe_model: None,
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    pub project_id: Uuid,
}

pub async fn get_tasks(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskWithAttemptStatus>>>, ApiError> {
    // Verify user has at least viewer access to this project
    access_context
        .check_project_access(
            &deployment.db().pool,
            &query.project_id.to_string(),
            crate::middleware::access_control::ProjectRole::Viewer,
        )
        .await?;

    let tasks =
        Task::find_by_project_id_with_attempt_status(&deployment.db().pool, &query.project_id.to_string())
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
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    // Verify user has at least editor access to create tasks in this project
    access_context
        .check_project_access(
            &deployment.db().pool,
            &payload.project_id.to_string(),
            crate::middleware::access_control::ProjectRole::Editor,
        )
        .await?;

    let id = Uuid::new_v4().to_string();

    tracing::debug!(
        "Creating task '{}' in project {} by user {}",
        payload.title,
        payload.project_id,
        access_context.user_id
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
        match ProjectBoard::find_by_id(&deployment.db().pool, &board_id.to_string()).await? {
            Some(board) if board.project_id == payload.project_id.to_string() => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }

    // Override created_by with the authenticated user's ID
    let mut payload = payload;
    payload.created_by = access_context.user_id.to_string();

    let task = Task::create(&deployment.db().pool, &payload, &id).await?;

    if let Some(image_ids) = &payload.image_ids {
        let task_uuid = Uuid::parse_str(&task.id).map_err(|e| ApiError::BadRequest(e.to_string()))?;
        TaskImage::associate_many_dedup(&deployment.db().pool, task_uuid, image_ids).await?;
    }

    // Auto-register agent watchers based on assigned agent's config
    if let Some(ref agent_id) = task.agent_id {
        if let Ok(Some(config)) =
            db::models::agent_execution_config::AgentExecutionConfig::find_by_agent_id(
                &deployment.db().pool,
                agent_id,
            )
            .await
        {
            for watcher_id in config.get_auto_watch_agent_ids() {
                let _ = Task::add_agent_watcher(&deployment.db().pool, &task.id, &watcher_id)
                    .await;
            }
        }
    }

    deployment
        .track_if_analytics_allowed(
            "task_created",
            serde_json::json!({
            "task_id": &task.id,
            "project_id": payload.project_id,
            "has_description": task.description.is_some(),
            "has_images": payload.image_ids.is_some(),
            }),
        )
        .await;

    // Log activity: task created
    {
        let log_entry = CreateActivityLog {
            task_id: task.id.clone(),
            actor_id: access_context.user_id.to_string(),
            actor_type: ActorType::Human,
            action: "created".to_string(),
            previous_state: None,
            new_state: serde_json::to_value(&task).ok(),
            metadata: None,
        };
        if let Err(e) = ActivityLog::create(&deployment.db().pool, &log_entry).await {
            tracing::warn!("Failed to log task creation activity: {e}");
        }
    }

    // Broadcast task creation to WebSocket clients
    let task_with_status = task_to_with_attempt_status(task.clone());
    broadcast_task_event(&deployment, "add", &task.id, Some(&task_with_status));

    Ok(ResponseJson(ApiResponse::success(task)))
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAndStartTaskRequest {
    pub task: CreateTask,
    pub executor_profile_id: ExecutorProfileId,
    pub base_branch: String,
}

pub async fn create_task_and_start(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAndStartTaskRequest>,
) -> Result<ResponseJson<ApiResponse<TaskWithAttemptStatus>>, ApiError> {
    // Verify user has editor access to create and start tasks
    access_context
        .check_project_access(
            &deployment.db().pool,
            &payload.task.project_id.to_string(),
            crate::middleware::access_control::ProjectRole::Editor,
        )
        .await?;
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
        match ProjectBoard::find_by_id(&deployment.db().pool, &board_id.to_string()).await? {
            Some(board) if board.project_id == task_payload.project_id.to_string() => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }

    let task_id = Uuid::new_v4().to_string();
    // Override created_by with the authenticated user's ID
    let mut task_payload = task_payload;
    task_payload.created_by = access_context.user_id.to_string();
    let task = Task::create(&deployment.db().pool, &task_payload, &task_id).await?;

    if let Some(image_ids) = &task_payload.image_ids {
        let task_uuid = Uuid::parse_str(&task.id).map_err(|e| ApiError::BadRequest(e.to_string()))?;
        TaskImage::associate_many(&deployment.db().pool, task_uuid, image_ids).await?;
    }

    // Auto-register agent watchers based on assigned agent's config
    if let Some(ref agent_id) = task.agent_id {
        if let Ok(Some(config)) =
            db::models::agent_execution_config::AgentExecutionConfig::find_by_agent_id(
                &deployment.db().pool,
                agent_id,
            )
            .await
        {
            for watcher_id in config.get_auto_watch_agent_ids() {
                let _ = Task::add_agent_watcher(&deployment.db().pool, &task.id, &watcher_id)
                    .await;
            }
        }
    }

    deployment
        .track_if_analytics_allowed(
            "task_created",
            serde_json::json!({
                "task_id": &task.id,
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

    let vibe_bypass = crate::helpers::vibe_check::is_vibe_bypass_active(&deployment.db().pool).await;

    // Check project VIBE budget
    if !vibe_bypass {
        if let Some(project) = Project::find_by_id(&deployment.db().pool, &task_payload.project_id.to_string()).await? {
            if !project.has_vibe_budget(ESTIMATED_VIBE_COST) {
                let remaining = project.remaining_vibe().unwrap_or(0);
                return Err(ApiError::PaymentRequired(format!(
                    "Project VIBE budget exceeded. Remaining: {} VIBE, Required: {} VIBE (~${})",
                    remaining,
                    ESTIMATED_VIBE_COST,
                    ESTIMATED_VIBE_COST as f64 * 0.01
                )));
            }
        }
    }

    // Check agent wallet VIBE budget and APT budget
    if let Some(wallet) =
        AgentWallet::find_by_profile_key(&deployment.db().pool, &executor_profile_id.to_string())
            .await?
    {
        // Check VIBE budget first
        if !vibe_bypass && !wallet.has_vibe_budget(ESTIMATED_VIBE_COST) {
            let remaining = wallet.remaining_vibe().unwrap_or(0);
            return Err(ApiError::PaymentRequired(format!(
                "Agent VIBE budget exceeded. Remaining: {} VIBE, Required: {} VIBE (~${})",
                remaining,
                ESTIMATED_VIBE_COST,
                ESTIMATED_VIBE_COST as f64 * 0.01
            )));
        }

        // Also check APT budget for legacy support
        const EXECUTION_DEBIT: i64 = 1;
        let remaining = wallet.budget_limit - wallet.spent_amount;
        if !vibe_bypass && remaining < EXECUTION_DEBIT {
            return Err(ApiError::Conflict(
                "Agent wallet APT budget exceeded for this profile".to_string(),
            ));
        }

        let task_uuid = Uuid::parse_str(&task.id).map_err(|e| ApiError::BadRequest(e.to_string()))?;

        let metadata = json!({
            "task_id": &task.id,
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
                task_id: Some(task_uuid),
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
                task_id: Some(task_uuid),
                task_attempt_id: None, // Will be set after attempt creation
                process_id: None,
                description: Some("Task execution started - pending cost calculation".to_string()),
                metadata: Some(serde_json::from_str(&metadata).unwrap_or_default()),
            },
        )
        .await;
    }

    let task_uuid = Uuid::parse_str(&task.id).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let task_attempt = TaskAttempt::create(
        &deployment.db().pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor,
            base_branch,
        },
        task_uuid,
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
                "task_id": &task.id,
                "executor": &executor_profile_id.executor,
                "variant": &executor_profile_id.variant,
                "attempt_id": task_attempt.id.to_string(),
            }),
        )
        .await;

    let task = Task::find_by_id(&deployment.db().pool, &task.id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    tracing::info!("Started execution process {}", execution_process.id);
    let task_with_status = TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: true,
        has_merged_attempt: false,
        last_attempt_failed: false,
        executor: task_attempt.executor,
        last_execution_summary: None,
        collaborators: None,
        vibe_cost: None,
        vibe_model: None,
    };

    // Broadcast task creation to WebSocket clients
    broadcast_task_event(&deployment, "add", &task_with_status.id, Some(&task_with_status));

    Ok(ResponseJson(ApiResponse::success(task_with_status)))
}

pub async fn update_task(
    Extension(access_context): Extension<AccessContext>,
    Extension(existing_task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    access_context.require_editor(&deployment.db().pool, &existing_task.project_id).await?;

    // Capture old state for activity logging before fields get moved
    let old_status = existing_task.status.clone();
    let old_title = existing_task.title.clone();
    let old_description = existing_task.description.clone();
    let old_priority = existing_task.priority.clone();
    let old_assignee_id = existing_task.assignee_id.clone();
    let old_assigned_agent = existing_task.assigned_agent.clone();
    let old_due_date = existing_task.due_date.clone();
    let old_tags = existing_task.tags.clone();
    let old_pod_id = existing_task.pod_id.clone();
    let old_board_id = existing_task.board_id.clone();
    let old_state_json = serde_json::to_value(&existing_task).ok();

    // Use existing values if not provided in update
    let title = payload.title.unwrap_or(existing_task.title.clone());
    let description = payload.description.or(existing_task.description.clone());
    let status = payload.status.unwrap_or(existing_task.status.clone());
    let parent_task_attempt = payload
        .parent_task_attempt
        .map(|u| u.to_string())
        .or(existing_task.parent_task_attempt);
    let pod_change = payload.pod_id.clone();
    if let Some(Some(pod_id)) = pod_change.as_ref() {
        match ProjectPod::find_by_id(&deployment.db().pool, *pod_id).await? {
            Some(pod) if pod.project_id.to_string() == existing_task.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Pod does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Pod not found".to_string())),
        }
    }
    let pod_id = match pod_change {
        Some(opt) => opt.map(|u| u.to_string()),
        None => existing_task.pod_id,
    };
    let board_change = payload.board_id.clone();
    if let Some(Some(board_id)) = board_change.as_ref() {
        match ProjectBoard::find_by_id(&deployment.db().pool, &board_id.to_string()).await? {
            Some(board) if board.project_id == existing_task.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }
    let board_id = match board_change {
        Some(opt) => opt.map(|u| u.to_string()),
        None => existing_task.board_id,
    };
    let priority = payload.priority.unwrap_or(existing_task.priority.clone());
    let assignee_id = payload.assignee_id.or(existing_task.assignee_id.clone());
    let assignee_type = payload.assignee_type.or(existing_task.assignee_type.clone());
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
    let parent_task_id = payload.parent_task_id.map(|u| u.to_string()).or(existing_task.parent_task_id);
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
    let completion_criteria = payload
        .completion_criteria
        .clone()
        .or(existing_task.completion_criteria.clone());
    let output_format = payload
        .output_format
        .clone()
        .or(existing_task.output_format.clone());

    let task = Task::update(
        &deployment.db().pool,
        &existing_task.id,
        &existing_task.project_id,
        title,
        description,
        status,
        parent_task_attempt,
        pod_id,
        board_id,
        priority,
        assignee_id,
        assignee_type,
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
        completion_criteria,
        output_format,
    )
    .await?;

    if let Some(image_ids) = &payload.image_ids {
        let task_uuid = Uuid::parse_str(&task.id).map_err(|e| ApiError::BadRequest(e.to_string()))?;
        TaskImage::delete_by_task_id(&deployment.db().pool, task_uuid).await?;
        TaskImage::associate_many_dedup(&deployment.db().pool, task_uuid, image_ids).await?;
    }

    // Log activity: task updated
    {
        let status_changed = old_status != task.status;
        let (action, metadata) = if status_changed {
            ("status_changed".to_string(), Some(json!({
                "from": old_status,
                "to": task.status,
            })))
        } else {
            // Build list of changed fields
            let mut changed = Vec::new();
            if old_title != task.title { changed.push("title"); }
            if old_description != task.description { changed.push("description"); }
            if old_priority != task.priority { changed.push("priority"); }
            if old_assignee_id != task.assignee_id { changed.push("assignee_id"); }
            if old_assigned_agent != task.assigned_agent { changed.push("assigned_agent"); }
            if old_due_date != task.due_date { changed.push("due_date"); }
            if old_tags != task.tags { changed.push("tags"); }
            if old_pod_id != task.pod_id { changed.push("pod_id"); }
            if old_board_id != task.board_id { changed.push("board_id"); }
            ("updated".to_string(), if changed.is_empty() { None } else { Some(json!({"changed_fields": changed})) })
        };
        let log_entry = CreateActivityLog {
            task_id: task.id.clone(),
            actor_id: access_context.user_id.to_string(),
            actor_type: ActorType::Human,
            action,
            previous_state: old_state_json,
            new_state: serde_json::to_value(&task).ok(),
            metadata,
        };
        if let Err(e) = ActivityLog::create(&deployment.db().pool, &log_entry).await {
            tracing::warn!("Failed to log task update activity: {e}");
        }
    }

    // Trigger agent watchers on manual status change to InReview
    if old_status != task.status && task.status == db::models::task::TaskStatus::InReview {
        let dep = deployment.clone();
        let task_id = task.id.clone();
        tokio::spawn(async move {
            let pool = &dep.db().pool;
            // Find PR associated with this task's attempts
            let task_uuid = match Uuid::parse_str(&task_id) {
                Ok(u) => u,
                Err(_) => return,
            };
            let attempts = TaskAttempt::fetch_all(pool, Some(task_uuid)).await.unwrap_or_default();
            let mut pr_info = None;
            for attempt in &attempts {
                if let Ok(Some(merge)) = db::models::merge::Merge::find_latest_by_task_attempt_id(pool, attempt.id).await {
                    if let db::models::merge::Merge::Pr(pr_merge) = &merge {
                        let url = &pr_merge.pr_info.url;
                        let number = pr_merge.pr_info.number;
                        let (owner, repo) = url::Url::parse(url)
                            .ok()
                            .and_then(|parsed| {
                                let segments: Vec<&str> = parsed.path_segments()?.collect();
                                if segments.len() >= 2 {
                                    Some((segments[0].to_string(), segments[1].to_string()))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));
                        if owner == "unknown" && repo == "unknown" {
                            tracing::warn!("Skipping watcher review: could not parse owner/repo from PR URL: {url}");
                            continue;
                        }
                        pr_info = Some(services::services::qa_review::PrCreatedInfo {
                            number,
                            url: url.clone(),
                            repo_owner: owner,
                            repo_name: repo,
                        });
                        break;
                    }
                }
            }

            match pr_info {
                Some(pr) => {
                    services::services::qa_review::spawn_watcher_reviews(
                        pool, dep.container(), &task_id, &pr,
                    ).await;
                }
                None => {
                    tracing::warn!(
                        "Manual InReview on task {task_id}: watchers pending but no PR found — watchers not triggered"
                    );
                }
            }
        });
    }

    // Broadcast task update to WebSocket clients
    // Fetch full TaskWithAttemptStatus for accurate attempt info
    if let Ok(tasks) = Task::find_by_project_id_with_attempt_status(&deployment.db().pool, &task.project_id).await {
        if let Some(task_with_status) = tasks.into_iter().find(|t| t.id == task.id) {
            broadcast_task_event(&deployment, "replace", &task.id, Some(&task_with_status));
        }
    }

    Ok(ResponseJson(ApiResponse::success(task)))
}

pub async fn delete_task(
    Extension(access_context): Extension<AccessContext>,
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    access_context.require_editor(&deployment.db().pool, &task.project_id).await?;

    // Validate no running execution processes
    let task_uuid = Uuid::parse_str(&task.id).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    if deployment
        .container()
        .has_running_processes(task_uuid)
        .await?
    {
        return Err(ApiError::Conflict("Task has running execution processes. Please wait for them to complete or stop them first.".to_string()));
    }

    // Gather task attempts data needed for background cleanup
    let attempts = TaskAttempt::fetch_all(&deployment.db().pool, Some(task_uuid))
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
    let task_id = task.id;
    let rows_affected = Task::delete(&deployment.db().pool, &task_id).await?;

    if rows_affected == 0 {
        return Err(ApiError::Database(SqlxError::RowNotFound));
    }

    // Broadcast task deletion to WebSocket clients
    broadcast_task_event(&deployment, "remove", &task_id, None);

    // Spawn background worktree cleanup task
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
    Extension(access_context): Extension<AccessContext>,
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    access_context.require_editor(&deployment.db().pool, &task.project_id).await?;

    use db::models::task::ApprovalStatus;

    let approved_task = Task::update(
        &deployment.db().pool,
        &task.id,
        &task.project_id,
        task.title,
        task.description,
        task.status,
        task.parent_task_attempt,
        task.pod_id,
        task.board_id,
        task.priority,
        task.assignee_id,
        task.assignee_type,
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
        task.completion_criteria.clone(),
        task.output_format.clone(),
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(approved_task)))
}

pub async fn request_changes(
    Extension(access_context): Extension<AccessContext>,
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    access_context.require_editor(&deployment.db().pool, &task.project_id).await?;

    use db::models::task::ApprovalStatus;

    let updated_task = Task::update(
        &deployment.db().pool,
        &task.id,
        &task.project_id,
        task.title,
        task.description,
        task.status,
        task.parent_task_attempt,
        task.pod_id,
        task.board_id,
        task.priority,
        task.assignee_id,
        task.assignee_type,
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
        task.completion_criteria.clone(),
        task.output_format.clone(),
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
    pub description: Option<String>,
    pub assigned_agent: Option<String>,
    pub assignee_id: Option<String>,
    pub created_by: Option<String>,
    pub tags: Option<String>,
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
        if !project_names.contains_key(&task.project_id) {
            if let Ok(Some(project)) = Project::find_by_id(&deployment.db().pool, &task.project_id).await {
                project_names.insert(task.project_id.clone(), project.name);
            }
        }
    }

    let assigned_tasks: Vec<AssignedTask> = tasks
        .into_iter()
        .map(|task| {
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
            let project_id = task.project_id.clone();
            AssignedTask {
                id: task.id,
                title: task.title,
                status: status_str,
                priority: priority_str,
                due_date: task.due_date.map(|d| d.to_rfc3339()),
                project_name: project_names.get(&project_id).cloned().unwrap_or_else(|| "Unknown".to_string()),
                project_id,
                description: task.description,
                assigned_agent: task.assigned_agent,
                assignee_id: task.assignee_id,
                created_by: Some(task.created_by),
                tags: task.tags,
            }
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(assigned_tasks)))
}

/// Get all tasks created by the current user
pub async fn get_created_by_me(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AssignedTask>>>, ApiError> {
    use db::models::project::Project;

    let user_id_str = access_context.user_id.to_string();
    let tasks = Task::find_by_creator(&deployment.db().pool, &user_id_str).await?;

    // Collect unique project IDs and fetch project names
    let mut project_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for task in &tasks {
        if !project_names.contains_key(&task.project_id) {
            if let Ok(Some(project)) = Project::find_by_id(&deployment.db().pool, &task.project_id).await {
                project_names.insert(task.project_id.clone(), project.name);
            }
        }
    }

    let created_tasks: Vec<AssignedTask> = tasks
        .into_iter()
        .map(|task| {
            let priority_str = serde_json::to_value(&task.priority)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "low".to_string());
            let status_str = serde_json::to_value(&task.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "todo".to_string());
            let project_id = task.project_id.clone();
            AssignedTask {
                id: task.id,
                title: task.title,
                status: status_str,
                priority: priority_str,
                due_date: task.due_date.map(|d| d.to_rfc3339()),
                project_name: project_names.get(&project_id).cloned().unwrap_or_else(|| "Unknown".to_string()),
                project_id,
                description: task.description,
                assigned_agent: task.assigned_agent,
                assignee_id: task.assignee_id,
                created_by: Some(task.created_by),
                tags: task.tags,
            }
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(created_tasks)))
}

pub async fn reject_task(
    Extension(access_context): Extension<AccessContext>,
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    access_context.require_editor(&deployment.db().pool, &task.project_id).await?;

    use db::models::task::ApprovalStatus;

    let rejected_task = Task::update(
        &deployment.db().pool,
        &task.id,
        &task.project_id,
        task.title,
        task.description,
        task.status,
        task.parent_task_attempt,
        task.pod_id,
        task.board_id,
        task.priority,
        task.assignee_id,
        task.assignee_type,
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
        task.completion_criteria.clone(),
        task.output_format.clone(),
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(rejected_task)))
}

/// POST /projects/:project_id/tasks/:task_id/watch — add current user as watcher
pub async fn watch_task(
    Extension(task): Extension<Task>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let user_id = access_context.user_id.to_string();
    Task::add_watcher(&deployment.db().pool, &task.id, &user_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// DELETE /projects/:project_id/tasks/:task_id/watch — remove current user as watcher
pub async fn unwatch_task(
    Extension(task): Extension<Task>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let user_id = access_context.user_id.to_string();
    Task::remove_watcher(&deployment.db().pool, &task.id, &user_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// GET /tasks/watched — get all tasks the current user is watching
pub async fn get_watched_tasks(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AssignedTask>>>, ApiError> {
    use db::models::project::Project;

    let user_id_str = access_context.user_id.to_string();
    let tasks = Task::find_watched_by_user(&deployment.db().pool, &user_id_str).await?;

    let mut project_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for task in &tasks {
        if !project_names.contains_key(&task.project_id) {
            if let Ok(Some(project)) = Project::find_by_id(&deployment.db().pool, &task.project_id).await {
                project_names.insert(task.project_id.clone(), project.name);
            }
        }
    }

    let watched: Vec<AssignedTask> = tasks.into_iter().map(|task| {
        let priority_str = serde_json::to_value(&task.priority).ok()
            .and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "low".to_string());
        let status_str = serde_json::to_value(&task.status).ok()
            .and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "todo".to_string());
        let pid = task.project_id.clone();
        AssignedTask {
            id: task.id,
            title: task.title,
            status: status_str,
            priority: priority_str,
            due_date: task.due_date.map(|d| d.to_rfc3339()),
            project_name: project_names.get(&pid).cloned().unwrap_or_else(|| "Unknown".to_string()),
            project_id: pid,
            description: task.description,
            assigned_agent: task.assigned_agent,
            assignee_id: task.assignee_id,
            created_by: Some(task.created_by),
            tags: task.tags,
        }
    }).collect();

    Ok(ResponseJson(ApiResponse::success(watched)))
}

// ── Agent watcher management ─────────────────────────────────────────────────

#[derive(Deserialize, TS)]
pub struct AddAgentWatcherRequest {
    pub agent_id: String,
}

#[derive(serde::Serialize, TS)]
pub struct AgentWatcherInfo {
    pub agent_id: String,
    pub agent_name: String,
    pub agent_designation: String,
    pub last_action: String,
    pub last_action_at: String,
}

/// POST /tasks/:task_id/agent-watchers — add an agent as a watcher
pub(crate) async fn add_agent_watcher(
    Extension(access_context): Extension<AccessContext>,
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<AddAgentWatcherRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    access_context.require_editor(&deployment.db().pool, &task.project_id).await?;

    Agent::find_by_id(&deployment.db().pool, &body.agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to find agent: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", body.agent_id)))?;

    Task::add_agent_watcher(&deployment.db().pool, &task.id, &body.agent_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// Path extractor for `/tasks/{task_id}/agent-watchers/{agent_id}`.
/// `task_id` is required by the route pattern and consumed by `load_task_middleware`,
/// but unused in the handler itself — the task is already available via `Extension<Task>`.
#[derive(Deserialize)]
pub(crate) struct AgentWatcherPath {
    #[allow(dead_code)]
    task_id: Uuid,
    agent_id: String,
}

/// DELETE /tasks/:task_id/agent-watchers/:agent_id — remove an agent watcher
pub(crate) async fn remove_agent_watcher(
    Extension(access_context): Extension<AccessContext>,
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
    axum::extract::Path(AgentWatcherPath { agent_id, .. }): axum::extract::Path<AgentWatcherPath>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    access_context.require_editor(&deployment.db().pool, &task.project_id).await?;

    Task::remove_agent_watcher(&deployment.db().pool, &task.id, &agent_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// GET /tasks/:task_id/agent-watchers — list agent watchers with agent info
pub(crate) async fn list_agent_watchers(
    Extension(task): Extension<Task>,
    Extension(_access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AgentWatcherInfo>>>, ApiError> {
    let watchers = Task::find_agent_watchers(&deployment.db().pool, &task.id).await?;

    // Batch-fetch agent info to avoid N+1 queries
    let all_agents = Agent::find_all(&deployment.db().pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch agents: {e}")))?;
    let agent_map: std::collections::HashMap<String, &Agent> = all_agents
        .iter()
        .map(|a| (a.id.to_string(), a))
        .collect();

    let mut result = Vec::with_capacity(watchers.len());
    for w in watchers {
        let (name, designation) = agent_map
            .get(&w.actor_id)
            .map(|a| (a.short_name.clone(), a.designation.clone()))
            .unwrap_or_else(|| (w.actor_id.clone(), String::new()));

        result.push(AgentWatcherInfo {
            agent_id: w.actor_id,
            agent_name: name,
            agent_designation: designation,
            last_action: w.last_action,
            last_action_at: w.last_action_at.to_rfc3339(),
        });
    }

    Ok(ResponseJson(ApiResponse::success(result)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Task-ID routes with load_task_middleware — defined as explicit routes
    // instead of .nest() to avoid Axum 0.8 path parameter shadowing static routes
    // (/{task_id} via nest was matching /created-by-me, /assigned-to-me etc.)
    let task_id_routes = Router::new()
        .route("/{task_id}", get(get_task).put(update_task).delete(delete_task))
        .route("/{task_id}/approve", post(approve_task))
        .route("/{task_id}/request-changes", post(request_changes))
        .route("/{task_id}/reject", post(reject_task))
        .route("/{task_id}/watch", post(watch_task).delete(unwatch_task))
        .route("/{task_id}/agent-watchers", get(list_agent_watchers).post(add_agent_watcher))
        .route("/{task_id}/agent-watchers/{agent_id}", delete(remove_agent_watcher))
        .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

    let inner = Router::new()
        .route("/", get(get_tasks).post(create_task))
        .route("/stream/ws", get(stream_tasks_ws))
        .route("/create-and-start", post(create_task_and_start))
        .route("/assigned-to-me", get(get_assigned_to_me))
        .route("/created-by-me", get(get_created_by_me))
        .route("/watched", get(get_watched_tasks))
        .merge(task_id_routes);

    // mount under /tasks
    Router::new().nest("/tasks", inner)
}

/// Global tasks router - mounts at /api/tasks (not nested under projects)
/// Note: assigned-to-me and watched routes are now in the main router() to avoid
/// conflicts with the /{task_id} parameterized route. This router is kept for
/// backwards compatibility but delegates to the same routes via the main router.
pub fn global_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
}

use super::*;

#[axum::debug_handler]
pub async fn replace_process(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ReplaceProcessRequest>,
) -> Result<ResponseJson<ApiResponse<ReplaceProcessResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let proc_id = payload.process_id;
    let force_when_dirty = payload.force_when_dirty.unwrap_or(false);
    let perform_git_reset = payload.perform_git_reset.unwrap_or(true);

    // Validate process belongs to attempt
    let process =
        ExecutionProcess::find_by_id(pool, proc_id)
            .await?
            .ok_or(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
                "Process not found".to_string(),
            )))?;
    if process.task_attempt_id != task_attempt.id {
        return Err(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "Process does not belong to this attempt".to_string(),
        )));
    }

    // Determine target reset OID: before the target process
    let mut target_before_oid = process.before_head_commit.clone();
    if target_before_oid.is_none() {
        // Fallback: previous process's after_head_commit
        target_before_oid =
            ExecutionProcess::find_prev_after_head_commit(pool, task_attempt.id, proc_id).await?;
    }

    // Decide if Git reset is needed and apply it
    let mut git_reset_needed = false;
    let mut git_reset_applied = false;
    if perform_git_reset {
        if let Some(target_oid) = &target_before_oid {
            let container_ref = deployment
                .container()
                .ensure_container_exists(&task_attempt)
                .await?;
            let wt = std::path::Path::new(&container_ref);
            let head_oid = deployment.git().get_head_info(wt).ok().map(|h| h.oid);
            let is_dirty = deployment
                .container()
                .is_container_clean(&task_attempt)
                .await
                .map(|is_clean| !is_clean)
                .unwrap_or(false);
            if head_oid.as_deref() != Some(target_oid.as_str()) || is_dirty {
                git_reset_needed = true;
                if is_dirty && !force_when_dirty {
                    git_reset_applied = false; // cannot reset now
                } else if let Err(e) =
                    deployment
                        .git()
                        .reset_worktree_to_commit(wt, target_oid, force_when_dirty)
                {
                    tracing::error!("Failed to reset worktree: {}", e);
                    git_reset_applied = false;
                } else {
                    git_reset_applied = true;
                }
            }
        }
    } else {
        // Only compute necessity
        if let Some(target_oid) = &target_before_oid {
            let container_ref = deployment
                .container()
                .ensure_container_exists(&task_attempt)
                .await?;
            let wt = std::path::Path::new(&container_ref);
            let head_oid = deployment.git().get_head_info(wt).ok().map(|h| h.oid);
            let is_dirty = deployment
                .container()
                .is_container_clean(&task_attempt)
                .await
                .map(|is_clean| !is_clean)
                .unwrap_or(false);
            if head_oid.as_deref() != Some(target_oid.as_str()) || is_dirty {
                git_reset_needed = true;
            }
        }
    }

    // Stop any running processes for this attempt
    deployment.container().try_stop(&task_attempt).await;

    // Soft-drop the target process and all later processes
    let deleted_count = ExecutionProcess::drop_at_and_after(pool, task_attempt.id, proc_id).await?;

    // Build follow-up executor action using the original process profile
    let initial_executor_profile_id = match &process
        .executor_action()
        .map_err(|e| ApiError::TaskAttempt(TaskAttemptError::ValidationError(e.to_string())))?
        .typ
    {
        ExecutorActionType::CodingAgentInitialRequest(request) => {
            Ok(request.executor_profile_id.clone())
        }
        ExecutorActionType::CodingAgentFollowUpRequest(request) => {
            Ok(request.executor_profile_id.clone())
        }
        _ => Err(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "Couldn't find profile from executor action".to_string(),
        ))),
    }?;

    let executor_profile_id = ExecutorProfileId {
        executor: initial_executor_profile_id.executor,
        variant: payload
            .variant
            .or(initial_executor_profile_id.variant.clone()),
    };

    // Use latest session_id from remaining (earlier) processes; if none exists, start a fresh initial request
    let latest_session_id =
        ExecutionProcess::find_latest_session_id_by_task_attempt(pool, task_attempt.id).await?;

    let action = if let Some(session_id) = latest_session_id {
        let follow_up_request = CodingAgentFollowUpRequest {
            prompt: payload.prompt.clone(),
            session_id,
            executor_profile_id,
        };
        ExecutorAction::new(
            ExecutorActionType::CodingAgentFollowUpRequest(follow_up_request),
            None,
        )
    } else {
        // No prior session (e.g., replacing the first run) -> start a fresh initial request
        ExecutorAction::new(
            ExecutorActionType::CodingAgentInitialRequest(
                executors::actions::coding_agent_initial::CodingAgentInitialRequest {
                    prompt: payload.prompt.clone(),
                    executor_profile_id,
                },
            ),
            None,
        )
    };

    let execution_process = deployment
        .container()
        .start_execution(
            &task_attempt,
            &action,
            &ExecutionProcessRunReason::CodingAgent,
        )
        .await?;

    Ok(ResponseJson(ApiResponse::success(ReplaceProcessResult {
        deleted_count,
        git_reset_needed,
        git_reset_applied,
        target_before_oid,
        new_execution_id: Some(execution_process.id),
    })))
}

#[axum::debug_handler]
pub async fn start_dev_server(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Get parent task
    let task = task_attempt
        .parent_task(&deployment.db().pool)
        .await?
        .ok_or(SqlxError::RowNotFound)?;

    // Get parent project
    let project = task
        .parent_project(&deployment.db().pool)
        .await?
        .ok_or(SqlxError::RowNotFound)?;

    // Stop any existing dev servers for this project
    let existing_dev_servers = match ExecutionProcess::find_running_dev_servers_by_project(
        pool,
        Uuid::parse_str(&project.id).map_err(|e| ApiError::InternalError(e.to_string()))?,
    )
    .await
    {
        Ok(servers) => servers,
        Err(e) => {
            tracing::error!(
                "Failed to find running dev servers for project {}: {}",
                project.id,
                e
            );
            return Err(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
                e.to_string(),
            )));
        }
    };

    for dev_server in existing_dev_servers {
        tracing::info!(
            "Stopping existing dev server {} for project {}",
            dev_server.id,
            project.id
        );

        if let Err(e) = deployment.container().stop_execution(&dev_server).await {
            tracing::error!("Failed to stop dev server {}: {}", dev_server.id, e);
        }
    }

    if let Some(dev_server) = project.dev_script {
        // TODO: Derive script language from system config
        let executor_action = ExecutorAction::new(
            ExecutorActionType::ScriptRequest(ScriptRequest {
                script: dev_server,
                language: ScriptRequestLanguage::Bash,
                context: ScriptContext::DevServer,
            }),
            None,
        );

        deployment
            .container()
            .start_execution(
                &task_attempt,
                &executor_action,
                &ExecutionProcessRunReason::DevServer,
            )
            .await?
    } else {
        return Ok(ResponseJson(ApiResponse::error(
            "No dev server script configured for this project",
        )));
    };

    Ok(ResponseJson(ApiResponse::success(())))
}

pub async fn open_task_attempt_in_editor(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<Option<OpenEditorRequest>>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Get the task attempt to access the worktree path
    let attempt = &task_attempt;
    let base_path = attempt.container_ref.as_ref().ok_or_else(|| {
        tracing::error!(
            "No container ref found for task attempt {}",
            task_attempt.id
        );
        ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "No container ref found".to_string(),
        ))
    })?;

    // If a specific file path is provided, use it; otherwise use the base path
    let path = if let Some(file_path) = payload.as_ref().and_then(|req| req.file_path.as_ref()) {
        std::path::Path::new(base_path).join(file_path)
    } else {
        std::path::PathBuf::from(base_path)
    };

    let editor_config = {
        let config = deployment.config().read().await;
        let editor_type_str = payload.as_ref().and_then(|req| req.editor_type.as_deref());
        config.editor.with_override(editor_type_str)
    };

    match editor_config.open_file(&path.to_string_lossy()) {
        Ok(_) => {
            tracing::info!(
                "Opened editor for task attempt {} at path: {}",
                task_attempt.id,
                path.display()
            );
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!(
                "Failed to open editor for attempt {}: {}",
                task_attempt.id,
                e
            );
            Err(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
                format!("Failed to open editor: {}", e),
            )))
        }
    }
}

pub async fn get_task_attempt_children(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TaskRelationships>>, StatusCode> {
    match Task::find_relationships_for_attempt(&deployment.db().pool, &task_attempt).await {
        Ok(relationships) => Ok(ResponseJson(ApiResponse::success(relationships))),
        Err(e) => {
            tracing::error!(
                "Failed to fetch relationships for task attempt {}: {}",
                task_attempt.id,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn stop_task_attempt_execution(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    deployment.container().try_stop(&task_attempt).await;
    Ok(ResponseJson(ApiResponse::success(())))
}

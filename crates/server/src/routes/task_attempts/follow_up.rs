use super::*;

pub async fn follow_up(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateFollowUpAttempt>,
) -> Result<ResponseJson<ApiResponse<ExecutionProcess>>, ApiError> {
    tracing::info!("{:?}", task_attempt);

    // Ensure worktree exists (recreate if needed for cold task support)
    deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;

    // Get latest session id (ignoring dropped)
    let session_id = ExecutionProcess::find_latest_session_id_by_task_attempt(
        &deployment.db().pool,
        task_attempt.id,
    )
    .await?
    .ok_or(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
        "Couldn't find a prior session_id, please create a new task attempt".to_string(),
    )))?;

    // Get ExecutionProcess for profile data
    let latest_execution_process = ExecutionProcess::find_latest_by_task_attempt_and_run_reason(
        &deployment.db().pool,
        task_attempt.id,
        &ExecutionProcessRunReason::CodingAgent,
    )
    .await?
    .ok_or(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
        "Couldn't find initial coding agent process, has it run yet?".to_string(),
    )))?;
    let initial_executor_profile_id = match &latest_execution_process
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
            "Couldn't find profile from initial request".to_string(),
        ))),
    }?;

    let executor_profile_id = ExecutorProfileId {
        executor: initial_executor_profile_id.executor,
        variant: payload.variant,
    };

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

    let mut prompt = payload.prompt;
    if let Some(image_ids) = &payload.image_ids {
        TaskImage::associate_many_dedup(
            &deployment.db().pool,
            Uuid::parse_str(&task.id).map_err(|e| ApiError::InternalError(e.to_string()))?,
            image_ids,
        )
        .await?;

        // Copy new images from the image cache to the worktree
        if let Some(container_ref) = &task_attempt.container_ref {
            let worktree_path = std::path::PathBuf::from(container_ref);
            deployment
                .image()
                .copy_images_by_ids_to_worktree(&worktree_path, image_ids)
                .await?;

            // Update image paths in prompt with full worktree path
            prompt = ImageService::canonicalise_image_paths(&prompt, &worktree_path);
        }
    }

    let cleanup_action = project.cleanup_script.map(|script| {
        Box::new(ExecutorAction::new(
            ExecutorActionType::ScriptRequest(ScriptRequest {
                script,
                language: ScriptRequestLanguage::Bash,
                context: ScriptContext::CleanupScript,
            }),
            None,
        ))
    });

    let follow_up_request = CodingAgentFollowUpRequest {
        prompt,
        session_id,
        executor_profile_id: executor_profile_id.clone(),
    };

    let follow_up_action = ExecutorAction::new(
        ExecutorActionType::CodingAgentFollowUpRequest(follow_up_request),
        cleanup_action,
    );

    let execution_process = deployment
        .container()
        .start_execution(
            &task_attempt,
            &follow_up_action,
            &ExecutionProcessRunReason::CodingAgent,
        )
        .await?;

    publish_execution_events(&executor_profile_id, &task_attempt).await;

    // Clear any persisted follow-up draft for this attempt to avoid stale UI after manual send
    let _ = FollowUpDraft::clear_after_send(&deployment.db().pool, task_attempt.id).await;

    Ok(ResponseJson(ApiResponse::success(execution_process)))
}

pub(crate) async fn has_running_processes_for_attempt(
    pool: &sqlx::SqlitePool,
    attempt_id: Uuid,
) -> Result<bool, ApiError> {
    let processes = ExecutionProcess::find_by_task_attempt_id(pool, attempt_id, false).await?;
    Ok(processes.into_iter().any(|p| {
        matches!(
            p.status,
            db::models::execution_process::ExecutionProcessStatus::Running
        )
    }))
}

#[axum::debug_handler]
pub async fn get_follow_up_draft(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<FollowUpDraftResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let draft = FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id)
        .await?
        .map(|d| FollowUpDraftResponse {
            task_attempt_id: d.task_attempt_id,
            prompt: d.prompt,
            queued: d.queued,
            variant: d.variant,
            image_ids: d.image_ids,
            version: d.version,
        })
        .unwrap_or(FollowUpDraftResponse {
            task_attempt_id: task_attempt.id,
            prompt: "".to_string(),
            queued: false,
            variant: None,
            image_ids: None,
            version: 0,
        });
    Ok(ResponseJson(ApiResponse::success(draft)))
}

#[axum::debug_handler]
pub async fn save_follow_up_draft(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateFollowUpDraftRequest>,
) -> Result<ResponseJson<ApiResponse<FollowUpDraftResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Enforce: cannot edit while queued
    let d = match FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id).await? {
        Some(d) => d,
        None => {
            // Create empty draft implicitly
            let id = uuid::Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO follow_up_drafts (id, task_attempt_id, prompt, queued, sending)
                   VALUES (?, ?, '', 0, 0)"#,
            )
            .bind(id)
            .bind(task_attempt.id)
            .execute(pool)
            .await?;
            FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id)
                .await?
                .ok_or(SqlxError::RowNotFound)?
        }
    };
    if d.queued {
        return Err(ApiError::Conflict(
            "Draft is queued; click Edit to unqueue before editing".to_string(),
        ));
    }

    // Optimistic concurrency check
    if let Some(expected_version) = payload.version
        && d.version != expected_version
    {
        return Err(ApiError::Conflict(
            "Draft changed, please retry with latest".to_string(),
        ));
    }

    if payload.prompt.is_none() && payload.variant.is_none() && payload.image_ids.is_none() {
        // nothing to change; return current
    } else {
        // Build a conservative UPDATE using positional binds to avoid SQL builder quirks
        let mut set_clauses: Vec<&str> = Vec::new();
        let mut has_variant_null = false;
        if payload.prompt.is_some() {
            set_clauses.push("prompt = ?");
        }
        if let Some(variant_opt) = &payload.variant {
            match variant_opt {
                Some(_) => set_clauses.push("variant = ?"),
                None => {
                    has_variant_null = true;
                    set_clauses.push("variant = NULL");
                }
            }
        }
        if payload.image_ids.is_some() {
            set_clauses.push("image_ids = ?");
        }
        // Always bump metadata when something changes
        set_clauses.push("updated_at = CURRENT_TIMESTAMP");
        set_clauses.push("version = version + 1");

        let mut sql = String::from("UPDATE follow_up_drafts SET ");
        sql.push_str(&set_clauses.join(", "));
        sql.push_str(" WHERE task_attempt_id = ?");

        let mut q = sqlx::query(&sql);
        if let Some(prompt) = &payload.prompt {
            q = q.bind(prompt);
        }
        if let Some(variant_opt) = &payload.variant
            && let Some(v) = variant_opt
        {
            q = q.bind(v);
        }
        if let Some(image_ids) = &payload.image_ids {
            let image_ids_json =
                serde_json::to_string(image_ids).unwrap_or_else(|_| "[]".to_string());
            q = q.bind(image_ids_json);
        }
        // WHERE bind
        q = q.bind(task_attempt.id);
        q.execute(pool).await?;
        let _ = has_variant_null; // silence unused (document intent)
    }

    // Ensure images are associated with the task for preview/loading
    if let Some(image_ids) = &payload.image_ids
        && !image_ids.is_empty()
    {
        // get parent task
        let task = task_attempt
            .parent_task(&deployment.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;
        TaskImage::associate_many_dedup(
            pool,
            Uuid::parse_str(&task.id).map_err(|e| ApiError::InternalError(e.to_string()))?,
            image_ids,
        )
        .await?;
    }

    // If queued and no process running for this attempt, attempt to start immediately.
    // Use an atomic sending lock to prevent duplicate starts when concurrent requests occur.
    let current = FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id).await?;
    let should_consider_start = current.as_ref().map(|c| c.queued).unwrap_or(false)
        && !has_running_processes_for_attempt(pool, task_attempt.id).await?;
    if should_consider_start {
        if FollowUpDraft::try_mark_sending(pool, task_attempt.id)
            .await
            .unwrap_or(false)
        {
            // Start follow up with saved draft (current is guaranteed Some by the queued check above)
            if let Some(draft) = current.as_ref() {
                let _ = start_follow_up_from_draft(&deployment, &task_attempt, draft).await;
            }
        } else {
            tracing::debug!(
                "Follow-up draft for attempt {} already being sent or not eligible",
                task_attempt.id
            );
        }
    }

    // Return current draft state (may have been cleared if started immediately)
    let current = FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id)
        .await?
        .map(|d| FollowUpDraftResponse {
            task_attempt_id: d.task_attempt_id,
            prompt: d.prompt,
            queued: d.queued,
            variant: d.variant,
            image_ids: d.image_ids,
            version: d.version,
        })
        .unwrap_or(FollowUpDraftResponse {
            task_attempt_id: task_attempt.id,
            prompt: "".to_string(),
            queued: false,
            variant: None,
            image_ids: None,
            version: 0,
        });

    Ok(ResponseJson(ApiResponse::success(current)))
}

#[axum::debug_handler]
pub async fn set_follow_up_queue(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SetQueueRequest>,
) -> Result<ResponseJson<ApiResponse<FollowUpDraftResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let Some(d) = FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id).await? else {
        return Err(ApiError::Conflict("No draft to queue".to_string()));
    };

    // Optimistic concurrency: ensure caller's view matches current state (if provided)
    if let Some(expected) = payload.expected_queued
        && d.queued != expected
    {
        return Err(ApiError::Conflict(
            "Draft state changed, please refresh and try again".to_string(),
        ));
    }
    if let Some(expected_v) = payload.expected_version
        && d.version != expected_v
    {
        return Err(ApiError::Conflict(
            "Draft changed, please refresh and try again".to_string(),
        ));
    }

    if payload.queued {
        let should_queue = !d.prompt.trim().is_empty();
        sqlx::query(
            r#"UPDATE follow_up_drafts
                   SET queued = ?, updated_at = CURRENT_TIMESTAMP, version = version + 1
                 WHERE task_attempt_id = ?"#,
        )
        .bind(should_queue as i64)
        .bind(task_attempt.id)
        .execute(pool)
        .await?;
    } else {
        // Unqueue
        sqlx::query(
            r#"UPDATE follow_up_drafts
                   SET queued = 0, updated_at = CURRENT_TIMESTAMP, version = version + 1
                 WHERE task_attempt_id = ?"#,
        )
        .bind(task_attempt.id)
        .execute(pool)
        .await?;
    }

    // If queued and no process running for this attempt, attempt to start immediately.
    let current = FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id).await?;
    let should_consider_start = current.as_ref().map(|c| c.queued).unwrap_or(false)
        && !has_running_processes_for_attempt(pool, task_attempt.id).await?;
    if should_consider_start {
        if FollowUpDraft::try_mark_sending(pool, task_attempt.id)
            .await
            .unwrap_or(false)
        {
            let _ = start_follow_up_from_draft(
                &deployment,
                &task_attempt,
                current.as_ref().expect("current checked via queued above"),
            )
            .await;
        } else {
            // Schedule a short delayed recheck to handle timing edges
            let deployment_clone = deployment.clone();
            let task_attempt_clone = task_attempt.clone();
            tokio::spawn(async move {
                use std::time::Duration;
                tokio::time::sleep(Duration::from_millis(1200)).await;
                let pool = &deployment_clone.db().pool;
                // Still no running process?
                let running = match ExecutionProcess::find_by_task_attempt_id(
                    pool,
                    task_attempt_clone.id,
                    false,
                )
                .await
                {
                    Ok(procs) => procs.into_iter().any(|p| {
                        matches!(
                            p.status,
                            db::models::execution_process::ExecutionProcessStatus::Running
                        )
                    }),
                    Err(_) => true, // assume running on error to avoid duplicate starts
                };
                if running {
                    return;
                }
                // Still queued and eligible?
                let draft =
                    match FollowUpDraft::find_by_task_attempt_id(pool, task_attempt_clone.id).await
                    {
                        Ok(Some(d)) if d.queued && !d.sending && !d.prompt.trim().is_empty() => d,
                        _ => return,
                    };
                if FollowUpDraft::try_mark_sending(pool, task_attempt_clone.id)
                    .await
                    .unwrap_or(false)
                {
                    let _ =
                        start_follow_up_from_draft(&deployment_clone, &task_attempt_clone, &draft)
                            .await;
                }
            });
        }
    }

    let d = FollowUpDraft::find_by_task_attempt_id(pool, task_attempt.id)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
    let resp = FollowUpDraftResponse {
        task_attempt_id: d.task_attempt_id,
        prompt: d.prompt,
        queued: d.queued,
        variant: d.variant,
        image_ids: d.image_ids,
        version: d.version,
    };
    Ok(ResponseJson(ApiResponse::success(resp)))
}

pub(crate) async fn start_follow_up_from_draft(
    deployment: &DeploymentImpl,
    task_attempt: &TaskAttempt,
    draft: &FollowUpDraft,
) -> Result<ExecutionProcess, ApiError> {
    // Ensure worktree exists
    deployment
        .container()
        .ensure_container_exists(task_attempt)
        .await?;

    // Get latest session id (ignoring dropped)
    let session_id = ExecutionProcess::find_latest_session_id_by_task_attempt(
        &deployment.db().pool,
        task_attempt.id,
    )
    .await?
    .ok_or(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
        "Couldn't find a prior session_id, please create a new task attempt".to_string(),
    )))?;

    // Get latest coding agent process to inherit executor profile
    let latest_execution_process = ExecutionProcess::find_latest_by_task_attempt_and_run_reason(
        &deployment.db().pool,
        task_attempt.id,
        &ExecutionProcessRunReason::CodingAgent,
    )
    .await?
    .ok_or(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
        "Couldn't find initial coding agent process, has it run yet?".to_string(),
    )))?;
    let initial_executor_profile_id = match &latest_execution_process
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
            "Couldn't find profile from initial request".to_string(),
        ))),
    }?;

    // Inherit executor profile; override variant if provided in draft
    let executor_profile_id = ExecutorProfileId {
        executor: initial_executor_profile_id.executor,
        variant: draft.variant.clone(),
    };

    // Get parent task -> project and cleanup action
    let task = task_attempt
        .parent_task(&deployment.db().pool)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
    let project = task
        .parent_project(&deployment.db().pool)
        .await?
        .ok_or(SqlxError::RowNotFound)?;

    let cleanup_action = project.cleanup_script.map(|script| {
        Box::new(ExecutorAction::new(
            ExecutorActionType::ScriptRequest(ScriptRequest {
                script,
                language: ScriptRequestLanguage::Bash,
                context: ScriptContext::CleanupScript,
            }),
            None,
        ))
    });

    // Handle images: associate to task, copy to worktree, and canonicalize paths in prompt
    let mut prompt = draft.prompt.clone();
    if let Some(image_ids) = &draft.image_ids {
        TaskImage::associate_many_dedup(&deployment.db().pool, task_attempt.task_id, image_ids)
            .await?;
        if let Some(container_ref) = &task_attempt.container_ref {
            let worktree_path = std::path::PathBuf::from(container_ref);
            deployment
                .image()
                .copy_images_by_ids_to_worktree(&worktree_path, image_ids)
                .await?;
            prompt = ImageService::canonicalise_image_paths(&prompt, &worktree_path);
        }
    }

    let follow_up_request = CodingAgentFollowUpRequest {
        prompt,
        session_id,
        executor_profile_id,
    };

    let follow_up_action = ExecutorAction::new(
        ExecutorActionType::CodingAgentFollowUpRequest(follow_up_request),
        cleanup_action,
    );

    let execution_process = deployment
        .container()
        .start_execution(
            task_attempt,
            &follow_up_action,
            &ExecutionProcessRunReason::CodingAgent,
        )
        .await?;

    // Best-effort: clear the draft after scheduling the execution
    let _ = FollowUpDraft::clear_after_send(&deployment.db().pool, task_attempt.id).await;

    Ok(execution_process)
}

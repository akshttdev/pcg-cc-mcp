use super::*;

pub async fn get_task_attempt_diff(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    // ) -> Result<ResponseJson<ApiResponse<Diff>>, ApiError> {
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, BoxError>>>, ApiError> {
    let stream = deployment.container().get_diff(&task_attempt).await?;

    Ok(Sse::new(stream.map_err(|e| -> BoxError { e.into() })).keep_alive(KeepAlive::default()))
}

pub async fn get_commit_info(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<CommitInfo>>, ApiError> {
    let Some(sha) = params.get("sha").cloned() else {
        return Err(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "Missing sha param".to_string(),
        )));
    };
    let container_ref = deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;
    let wt = std::path::Path::new(&container_ref);
    let subject = deployment.git().get_commit_subject(wt, &sha)?;
    Ok(ResponseJson(ApiResponse::success(CommitInfo {
        sha,
        subject,
    })))
}

pub async fn compare_commit_to_head(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<CommitCompareResult>>, ApiError> {
    let Some(target_oid) = params.get("sha").cloned() else {
        return Err(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "Missing sha param".to_string(),
        )));
    };
    let container_ref = deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;
    let wt = std::path::Path::new(&container_ref);
    let head_info = deployment.git().get_head_info(wt)?;
    let (ahead_from_head, behind_from_head) =
        deployment
            .git()
            .ahead_behind_commits_by_oid(wt, &head_info.oid, &target_oid)?;
    let is_linear = behind_from_head == 0;
    Ok(ResponseJson(ApiResponse::success(CommitCompareResult {
        head_oid: head_info.oid,
        target_oid,
        ahead_from_head,
        behind_from_head,
        is_linear,
    })))
}

#[axum::debug_handler]
pub async fn merge_task_attempt(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    let task = task_attempt
        .parent_task(pool)
        .await?
        .ok_or(ApiError::TaskAttempt(TaskAttemptError::TaskNotFound))?;
    let task_id_uuid =
        Uuid::parse_str(&task.id).map_err(|e| ApiError::InternalError(e.to_string()))?;
    let project_id_uuid =
        Uuid::parse_str(&task.project_id).map_err(|e| ApiError::InternalError(e.to_string()))?;
    let ctx =
        TaskAttempt::load_context(pool, task_attempt.id, task_id_uuid, project_id_uuid).await?;

    let container_ref = deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;
    let worktree_path = std::path::Path::new(&container_ref);

    let task_uuid_str = task.id.to_string();
    let first_uuid_section = task_uuid_str.split('-').next().unwrap_or(&task_uuid_str);

    // Create commit message with task title and description
    let mut commit_message = format!("{} (duck-kanban {})", ctx.task.title, first_uuid_section);

    // Add description on next line if it exists
    if let Some(description) = &ctx.task.description
        && !description.trim().is_empty()
    {
        commit_message.push_str("\n\n");
        commit_message.push_str(description);
    }

    // Get branch name from task attempt
    let branch_name = ctx.task_attempt.branch.as_ref().ok_or_else(|| {
        ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "No branch found for task attempt".to_string(),
        ))
    })?;

    let merge_commit_id = deployment.git().merge_changes(
        &ctx.project.git_repo_path,
        worktree_path,
        branch_name,
        &ctx.task_attempt.base_branch,
        &commit_message,
    )?;

    Merge::create_direct(
        pool,
        task_attempt.id,
        &ctx.task_attempt.base_branch,
        &merge_commit_id,
    )
    .await?;
    Task::update_status(pool, &ctx.task.id, TaskStatus::Done).await?;

    deployment
        .track_if_analytics_allowed(
            "task_attempt_merged",
            serde_json::json!({
                "task_id": ctx.task.id.to_string(),
                "project_id": ctx.project.id.to_string(),
                "attempt_id": task_attempt.id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(())))
}

/// POST /task-attempts/:id/link-pr -- register an externally-created PR against this attempt.
/// Creates a PR merge record without requiring a worktree or pushing branches.
pub async fn link_pr(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<LinkPrRequest>,
) -> Result<ResponseJson<ApiResponse<PrMerge>>, ApiError> {
    let pool = &deployment.db().pool;
    let target_branch = payload
        .target_branch
        .unwrap_or_else(|| task_attempt.base_branch.clone());

    let pr_merge = Merge::create_pr(
        pool,
        task_attempt.id,
        &target_branch,
        payload.pr_number,
        &payload.pr_url,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to link PR: {e}")))?;

    Ok(ResponseJson(ApiResponse::success(pr_merge)))
}

pub async fn push_task_attempt_branch(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let github_config = deployment.config().read().await.github.clone();
    let Some(github_token) = github_config.token() else {
        return Err(GitHubServiceError::TokenInvalid.into());
    };

    let github_service = GitHubService::new(&github_token)?;
    github_service.check_token().await?;

    let branch_name = task_attempt.branch.as_ref().ok_or_else(|| {
        ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "No branch found for task attempt".to_string(),
        ))
    })?;
    let ws_path = PathBuf::from(
        deployment
            .container()
            .ensure_container_exists(&task_attempt)
            .await?,
    );

    deployment
        .git()
        .push_to_github(&ws_path, branch_name, &github_token)?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub async fn create_github_pr(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<CreateGitHubPrRequest>,
) -> Result<ResponseJson<ApiResponse<String, GitHubServiceError>>, ApiError> {
    let github_config = deployment.config().read().await.github.clone();
    let Some(github_token) = github_config.token() else {
        return Ok(ResponseJson(ApiResponse::error_with_data(
            GitHubServiceError::TokenInvalid,
        )));
    };
    // Create GitHub service instance
    let github_service = GitHubService::new(&github_token)?;
    // Get the task attempt to access the stored base branch
    let base_branch = request.base_branch.unwrap_or_else(|| {
        // Use the stored base branch from the task attempt as the default
        // Fall back to config default or "main" only if stored base branch is somehow invalid
        if !task_attempt.base_branch.trim().is_empty() {
            task_attempt.base_branch.clone()
        } else {
            github_config
                .default_pr_base
                .as_ref()
                .map_or_else(|| "main".to_string(), |b| b.to_string())
        }
    });

    let pool = &deployment.db().pool;
    let task = task_attempt
        .parent_task(pool)
        .await?
        .ok_or(ApiError::TaskAttempt(TaskAttemptError::TaskNotFound))?;
    let project = Project::find_by_id(pool, &task.project_id)
        .await?
        .ok_or(ApiError::Project(ProjectError::ProjectNotFound))?;

    // Get branch name from task attempt
    let branch_name = task_attempt.branch.as_ref().ok_or_else(|| {
        ApiError::TaskAttempt(TaskAttemptError::ValidationError(
            "No branch found for task attempt".to_string(),
        ))
    })?;
    let workspace_path = PathBuf::from(
        deployment
            .container()
            .ensure_container_exists(&task_attempt)
            .await?,
    );

    // Push the branch to GitHub first
    if let Err(e) = deployment
        .git()
        .push_to_github(&workspace_path, branch_name, &github_token)
    {
        tracing::error!("Failed to push branch to GitHub: {}", e);
        let gh_e = GitHubServiceError::from(e);
        if gh_e.is_api_data() {
            return Ok(ResponseJson(ApiResponse::error_with_data(gh_e)));
        } else {
            return Ok(ResponseJson(ApiResponse::error(
                format!("Failed to push branch to GitHub: {}", gh_e).as_str(),
            )));
        }
    }

    let norm_base_branch_name = if matches!(
        deployment
            .git()
            .find_branch_type(&project.git_repo_path, &base_branch)?,
        BranchType::Remote
    ) {
        // Remote branches are formatted as {remote}/{branch} locally.
        // For PR APIs, we must provide just the branch name.
        let remote = deployment
            .git()
            .get_remote_name_from_branch_name(&workspace_path, &base_branch)?;
        let remote_prefix = format!("{}/", remote);
        base_branch
            .strip_prefix(&remote_prefix)
            .unwrap_or(&base_branch)
            .to_string()
    } else {
        base_branch
    };
    // Create the PR using GitHub service
    let pr_request = CreatePrRequest {
        title: request.title.clone(),
        body: request.body.clone(),
        head_branch: branch_name.clone(),
        base_branch: norm_base_branch_name.clone(),
    };
    // Use GitService to get the remote URL, then create GitHubRepoInfo
    let repo_info = deployment
        .git()
        .get_github_repo_info(&project.git_repo_path)?;

    match github_service.create_pr(&repo_info, &pr_request).await {
        Ok(pr_info) => {
            // Update the task attempt with PR information
            if let Err(e) = Merge::create_pr(
                pool,
                task_attempt.id,
                &norm_base_branch_name,
                pr_info.number,
                &pr_info.url,
            )
            .await
            {
                tracing::error!("Failed to update task attempt PR status: {}", e);
            }

            // Auto-open PR in browser
            if let Err(e) = utils::browser::open_browser(&pr_info.url).await {
                tracing::warn!("Failed to open PR in browser: {}", e);
            }
            deployment
                .track_if_analytics_allowed(
                    "github_pr_created",
                    serde_json::json!({
                        "task_id": task.id.to_string(),
                        "project_id": project.id.to_string(),
                        "attempt_id": task_attempt.id.to_string(),
                    }),
                )
                .await;

            Ok(ResponseJson(ApiResponse::success(pr_info.url)))
        }
        Err(e) => {
            tracing::error!(
                "Failed to create GitHub PR for attempt {}: {}",
                task_attempt.id,
                e
            );
            if e.is_api_data() {
                Ok(ResponseJson(ApiResponse::error_with_data(e)))
            } else {
                Ok(ResponseJson(ApiResponse::error(
                    format!("Failed to create PR: {}", e).as_str(),
                )))
            }
        }
    }
}

pub async fn get_task_attempt_branch_status(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<BranchStatus>>, ApiError> {
    let pool = &deployment.db().pool;

    let task = task_attempt
        .parent_task(pool)
        .await?
        .ok_or(ApiError::TaskAttempt(TaskAttemptError::TaskNotFound))?;
    let task_id_uuid =
        Uuid::parse_str(&task.id).map_err(|e| ApiError::InternalError(e.to_string()))?;
    let project_id_uuid =
        Uuid::parse_str(&task.project_id).map_err(|e| ApiError::InternalError(e.to_string()))?;
    let ctx =
        TaskAttempt::load_context(pool, task_attempt.id, task_id_uuid, project_id_uuid).await?;
    let has_uncommitted_changes = deployment
        .container()
        .is_container_clean(&task_attempt)
        .await
        .ok()
        .map(|is_clean| !is_clean);
    let head_oid = {
        let container_ref = deployment
            .container()
            .ensure_container_exists(&task_attempt)
            .await?;
        let wt = std::path::Path::new(&container_ref);
        deployment.git().get_head_info(wt).ok().map(|h| h.oid)
    };
    // Detect conflicts and operation in progress (best-effort)
    let (is_rebase_in_progress, conflicted_files, conflict_op) = {
        let container_ref = deployment
            .container()
            .ensure_container_exists(&task_attempt)
            .await?;
        let wt = std::path::Path::new(&container_ref);
        let in_rebase = deployment.git().is_rebase_in_progress(wt).unwrap_or(false);
        let conflicts = deployment
            .git()
            .get_conflicted_files(wt)
            .unwrap_or_default();
        let op = if conflicts.is_empty() {
            None
        } else {
            deployment.git().detect_conflict_op(wt).unwrap_or(None)
        };
        (in_rebase, conflicts, op)
    };
    let (uncommitted_count, untracked_count) = {
        let container_ref = deployment
            .container()
            .ensure_container_exists(&task_attempt)
            .await?;
        let wt = std::path::Path::new(&container_ref);
        match deployment.git().get_worktree_change_counts(wt) {
            Ok((a, b)) => (Some(a), Some(b)),
            Err(_) => (None, None),
        }
    };

    let task_branch =
        task_attempt
            .branch
            .ok_or(ApiError::TaskAttempt(TaskAttemptError::ValidationError(
                "No branch found for task attempt".to_string(),
            )))?;
    let base_branch_type = deployment
        .git()
        .find_branch_type(&ctx.project.git_repo_path, &task_attempt.base_branch)?;

    let (commits_ahead, commits_behind) = if matches!(base_branch_type, BranchType::Local) {
        let (a, b) = deployment.git().get_branch_status(
            &ctx.project.git_repo_path,
            &task_branch,
            &task_attempt.base_branch,
        )?;
        (Some(a), Some(b))
    } else {
        (None, None)
    };
    // Fetch merges for this task attempt and add to branch status
    let merges = Merge::find_by_task_attempt_id(pool, task_attempt.id).await?;
    let mut branch_status = BranchStatus {
        commits_ahead,
        commits_behind,
        has_uncommitted_changes,
        head_oid,
        uncommitted_count,
        untracked_count,
        remote_commits_ahead: None,
        remote_commits_behind: None,
        merges,
        base_branch_name: task_attempt.base_branch.clone(),
        is_rebase_in_progress,
        conflict_op,
        conflicted_files,
    };
    let has_open_pr = branch_status.merges.first().is_some_and(|m| {
        matches!(
            m,
            Merge::Pr(PrMerge {
                pr_info: PullRequestInfo {
                    status: MergeStatus::Open,
                    ..
                },
                ..
            })
        )
    });

    // check remote status if the attempt has an open PR or the base_branch is a remote branch
    if has_open_pr || base_branch_type == BranchType::Remote {
        let github_config = deployment.config().read().await.github.clone();
        let token = github_config
            .token()
            .ok_or(ApiError::GitHubService(GitHubServiceError::TokenInvalid))?;

        // For an attempt with a remote base branch, we compare against that
        // After opening a PR, the attempt has a remote branch itself, so we use that
        let remote_base_branch = if base_branch_type == BranchType::Remote && !has_open_pr {
            Some(task_attempt.base_branch)
        } else {
            None
        };
        let (remote_commits_ahead, remote_commits_behind) =
            deployment.git().get_remote_branch_status(
                &ctx.project.git_repo_path,
                &task_branch,
                remote_base_branch.as_deref(),
                token,
            )?;
        branch_status.remote_commits_ahead = Some(remote_commits_ahead);
        branch_status.remote_commits_behind = Some(remote_commits_behind);
    }
    Ok(ResponseJson(ApiResponse::success(branch_status)))
}

#[axum::debug_handler]
pub async fn rebase_task_attempt(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
    request_body: Option<Json<RebaseTaskAttemptRequest>>,
) -> Result<ResponseJson<ApiResponse<(), GitOperationError>>, ApiError> {
    // Extract new base branch from request body if provided
    let new_base_branch = request_body.and_then(|body| body.new_base_branch.clone());

    let github_config = deployment.config().read().await.github.clone();

    let pool = &deployment.db().pool;

    let task = task_attempt
        .parent_task(pool)
        .await?
        .ok_or(ApiError::TaskAttempt(TaskAttemptError::TaskNotFound))?;
    let task_id_uuid =
        Uuid::parse_str(&task.id).map_err(|e| ApiError::InternalError(e.to_string()))?;
    let project_id_uuid =
        Uuid::parse_str(&task.project_id).map_err(|e| ApiError::InternalError(e.to_string()))?;
    let ctx =
        TaskAttempt::load_context(pool, task_attempt.id, task_id_uuid, project_id_uuid).await?;

    // Use the stored base branch if no new base branch is provided
    let effective_base_branch =
        new_base_branch.or_else(|| Some(ctx.task_attempt.base_branch.clone()));

    let container_ref = deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;
    let worktree_path = std::path::Path::new(&container_ref);

    let result = deployment.git().rebase_branch(
        &ctx.project.git_repo_path,
        worktree_path,
        effective_base_branch.clone().as_deref(),
        &ctx.task_attempt.base_branch.clone(),
        github_config.token(),
    );
    if let Err(e) = result {
        use services::services::git::GitServiceError;
        return match e {
            GitServiceError::MergeConflicts(msg) => Ok(ResponseJson(ApiResponse::<
                (),
                GitOperationError,
            >::error_with_data(
                GitOperationError::MergeConflicts {
                    message: msg,
                    op: ConflictOp::Rebase,
                },
            ))),
            GitServiceError::RebaseInProgress => Ok(ResponseJson(ApiResponse::<
                (),
                GitOperationError,
            >::error_with_data(
                GitOperationError::RebaseInProgress,
            ))),
            other => Err(ApiError::GitService(other)),
        };
    }

    if let Some(new_base_branch) = &effective_base_branch
        && new_base_branch != &ctx.task_attempt.base_branch
    {
        TaskAttempt::update_base_branch(&deployment.db().pool, task_attempt.id, new_base_branch)
            .await?;
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

#[axum::debug_handler]
pub async fn abort_conflicts_task_attempt(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Resolve worktree path for this attempt
    let container_ref = deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;
    let worktree_path = std::path::Path::new(&container_ref);

    deployment.git().abort_conflicts(worktree_path)?;

    Ok(ResponseJson(ApiResponse::success(())))
}

#[axum::debug_handler]
pub async fn delete_task_attempt_file(
    Extension(task_attempt): Extension<TaskAttempt>,
    Query(query): Query<DeleteFileQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let container_ref = deployment
        .container()
        .ensure_container_exists(&task_attempt)
        .await?;
    let worktree_path = std::path::Path::new(&container_ref);

    // Use GitService to delete file and commit
    let _commit_id = deployment
        .git()
        .delete_file_and_commit(worktree_path, &query.file_path)
        .map_err(|e| {
            tracing::error!(
                "Failed to delete file '{}' from task attempt {}: {}",
                query.file_path,
                task_attempt.id,
                e
            );
            ApiError::GitService(e)
        })?;

    Ok(ResponseJson(ApiResponse::success(())))
}

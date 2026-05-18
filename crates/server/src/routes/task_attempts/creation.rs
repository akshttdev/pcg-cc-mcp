use super::*;

pub async fn get_task_attempts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskAttemptQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskAttempt>>>, ApiError> {
    let pool = &deployment.db().pool;
    let attempts = TaskAttempt::fetch_all(pool, query.task_id).await?;
    Ok(ResponseJson(ApiResponse::success(attempts)))
}

pub async fn get_task_attempt(
    Extension(task_attempt): Extension<TaskAttempt>,
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TaskAttempt>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(task_attempt)))
}

#[axum::debug_handler]
pub async fn create_task_attempt(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTaskAttemptBody>,
) -> Result<ResponseJson<ApiResponse<TaskAttempt>>, ApiError> {
    let executor_profile_id = payload.get_executor_profile_id();

    let task_attempt = TaskAttempt::create(
        &deployment.db().pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor,
            base_branch: payload.base_branch.clone(),
        },
        payload.task_id,
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
                "task_id": task_attempt.task_id.to_string(),
                "variant": &executor_profile_id.variant,
                "executor": &executor_profile_id.executor,
                "attempt_id": task_attempt.id.to_string(),
            }),
        )
        .await;

    tracing::info!("Started execution process {}", execution_process.id);

    publish_execution_events(&executor_profile_id, &task_attempt).await;

    Ok(ResponseJson(ApiResponse::success(task_attempt)))
}

pub async fn create_task_attempt_record(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTaskAttemptRecordBody>,
) -> Result<ResponseJson<ApiResponse<CreateRecordResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Use a single connection with FK checks disabled to avoid TEXT/BLOB
    // mismatch between task_attempts.task_id (Uuid->BLOB) and tasks.id (TEXT).
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::InternalError(format!("Pool acquire: {e}")))?;

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await
        .map_err(|e| ApiError::InternalError(format!("FK pragma: {e}")))?;

    let attempt_id = Uuid::new_v4();
    let result = sqlx::query_as!(
        TaskAttempt,
        r#"INSERT INTO task_attempts (id, task_id, container_ref, branch, base_branch, executor, worktree_deleted, setup_completed_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", container_ref, branch, base_branch, executor as "executor!",  worktree_deleted as "worktree_deleted!: bool", setup_completed_at as "setup_completed_at: DateTime<Utc>", archived as "archived!: bool", pinned as "pinned!: bool", name, seen_at as "seen_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
        attempt_id,
        payload.task_id,
        Option::<String>::None,
        Option::<String>::None,
        payload.base_branch,
        payload.executor,
        false,
        Option::<DateTime<Utc>>::None,
    )
    .fetch_one(&mut *conn)
    .await;

    // Restore pool default (FK OFF) before returning connection
    let _ = sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await;

    let task_attempt = result
        .map_err(|e| ApiError::InternalError(format!("Failed to create task attempt: {e}")))?;

    Ok(ResponseJson(ApiResponse::success(CreateRecordResponse {
        id: task_attempt.id.to_string(),
        task_id: task_attempt.task_id.to_string(),
        base_branch: task_attempt.base_branch,
        executor: task_attempt.executor,
    })))
}

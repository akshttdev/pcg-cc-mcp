//! Nora project operations: create project, board, and task handlers

use super::*;

/// Create a project via Nora
pub async fn nora_create_project(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<NoraCreateProjectRequest>,
) -> Result<Json<NoraProjectResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let project = nora
        .executor
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Task executor not initialized".to_string()))?
        .create_project(
            request.name,
            request.git_repo_path,
            request.setup_script,
            request.dev_script,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create project: {}", e)))?;

    Ok(Json(NoraProjectResponse {
        project_id: project.id.to_string(),
        name: project.name,
        git_repo_path: project.git_repo_path.to_string_lossy().to_string(),
        created_at: project.created_at.to_string(),
    }))
}

/// Create a board via Nora
pub async fn nora_create_board(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<NoraCreateBoardRequest>,
) -> Result<Json<NoraBoardResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid project ID: {}", e)))?;

    // Simplified board types: only Default and Custom
    let board_type = match request.board_type.as_deref() {
        Some("default") | Some("main") => {
            Some(db::models::project_board::ProjectBoardType::Default)
        }
        Some("custom") | Some(_) => Some(db::models::project_board::ProjectBoardType::Custom),
        None => None,
    };

    let board = nora
        .executor
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Task executor not initialized".to_string()))?
        .create_board(&project_id.to_string(), request.name, request.description, board_type)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create board: {}", e)))?;

    Ok(Json(NoraBoardResponse {
        board_id: board.id.to_string(),
        project_id: board.project_id.to_string(),
        name: board.name,
        board_type: format!("{:?}", board.board_type),
        created_at: board.created_at.to_string(),
    }))
}

/// Create a task on a board via Nora
pub async fn nora_create_task(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<NoraCreateTaskRequest>,
) -> Result<Json<NoraTaskResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid project ID: {}", e)))?;

    let board_id = Uuid::parse_str(&request.board_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid board ID: {}", e)))?;

    let priority = request.priority.and_then(|p| match p.as_str() {
        "low" => Some(db::models::task::Priority::Low),
        "medium" => Some(db::models::task::Priority::Medium),
        "high" => Some(db::models::task::Priority::High),
        _ => None,
    });

    let task = nora
        .executor
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Task executor not initialized".to_string()))?
        .create_task_on_board(
            project_id,
            board_id,
            request.title,
            request.description,
            priority,
            request.tags,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create task: {}", e)))?;

    Ok(Json(NoraTaskResponse {
        task_id: task.id.to_string(),
        project_id: task.project_id.to_string(),
        board_id: task.board_id.map(|id| id.to_string()),
        title: task.title,
        status: format!("{:?}", task.status),
        priority: format!("{:?}", task.priority),
        created_at: task.created_at.to_string(),
    }))
}

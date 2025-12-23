use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};
use db::models::{
    project::Project,
    project_board::{CreateProjectBoard, ProjectBoard, ProjectBoardType, UpdateProjectBoard},
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, serde::Deserialize)]
pub struct CreateBoardPayload {
    pub name: String,
    pub slug: String,
    pub board_type: ProjectBoardType,
    pub description: Option<String>,
    pub metadata: Option<String>,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/boards", get(list_boards).post(create_board))
        .route(
            "/boards/{board_id}",
            patch(update_board).delete(delete_board),
        )
        .with_state(deployment.clone())
}

async fn list_boards(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ProjectBoard>>>, ApiError> {
    let boards = ProjectBoard::list_by_project(&deployment.db().pool, project.id).await?;
    Ok(Json(ApiResponse::success(boards)))
}

fn normalize_slug(source: &str) -> String {
    let slug = source
        .trim()
        .to_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    slug.trim_matches('-').to_string()
}

async fn create_board(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateBoardPayload>,
) -> Result<(StatusCode, Json<ApiResponse<ProjectBoard>>), ApiError> {
    let slug = if payload.slug.trim().is_empty() {
        normalize_slug(&payload.name)
    } else {
        normalize_slug(&payload.slug)
    };

    let board = ProjectBoard::create(
        &deployment.db().pool,
        &CreateProjectBoard {
            project_id: project.id,
            name: payload.name,
            slug,
            board_type: payload.board_type,
            description: payload.description,
            metadata: payload.metadata,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(board))))
}

async fn update_board(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Path(board_id): Path<Uuid>,
    Json(mut payload): Json<UpdateProjectBoard>,
) -> Result<Json<ApiResponse<ProjectBoard>>, ApiError> {
    if let Some(slug) = payload.slug.as_mut() {
        if slug.trim().is_empty() {
            *slug = normalize_slug("board");
        } else {
            *slug = normalize_slug(slug);
        }
    }
    let board = ProjectBoard::update(&deployment.db().pool, board_id, &payload)
        .await?
        .ok_or_else(|| ApiError::NotFound("Board not found".into()))?;
    if board.project_id != project.id {
        return Err(ApiError::BadRequest(
            "Board does not belong to project".into(),
        ));
    }
    Ok(Json(ApiResponse::success(board)))
}

async fn delete_board(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Path(board_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let board = ProjectBoard::find_by_id(&deployment.db().pool, board_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Board not found".into()))?;
    if board.project_id != project.id {
        return Err(ApiError::BadRequest(
            "Board does not belong to project".into(),
        ));
    }
    ProjectBoard::delete(&deployment.db().pool, board_id).await?;
    Ok(Json(ApiResponse::success(())))
}

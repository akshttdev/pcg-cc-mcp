use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::get,
};
use db::models::{
    board_share::{BoardShare, CreateBoardShare, UpdateBoardShare},
    project_board::ProjectBoard,
    user::Organization,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::AccessContext,
};

/// Check if user has admin access to an org
async fn require_org_admin(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: &str,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id.as_str()).await?;
    match role.as_deref() {
        Some("admin") => Ok(()),
        _ => Err(ApiError::Forbidden("Only org admins can manage board shares".into())),
    }
}

/// Check if user is at least a member of an org
async fn require_org_access(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: &str,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id.as_str()).await?;
    if role.is_none() {
        return Err(ApiError::Forbidden("Not a member of this organization".into()));
    }
    Ok(())
}

/// POST /api/organizations/:org_id/board-shares — create a board share (org admin only)
pub async fn create_board_share(
    Path(org_id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateBoardShare>,
) -> Result<Json<ApiResponse<BoardShare>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_admin(pool, &access_context, &org_id).await?;

    // Verify the board exists and belongs to a project in this org
    let board = ProjectBoard::find_by_id(pool, &data.board_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Board not found".into()))?;

    #[derive(sqlx::FromRow)]
    struct ProjOrg {
        organization_id: Option<String>,
    }

    let proj: Option<ProjOrg> = sqlx::query_as(
        "SELECT organization_id FROM projects WHERE id = ?"
    )
    .bind(&board.project_id)
    .fetch_optional(pool)
    .await?;

    match proj {
        Some(p) if p.organization_id.as_deref() == Some(org_id.as_str()) => {}
        _ => return Err(ApiError::Forbidden("Board does not belong to a project in this organization".into())),
    }

    // Cannot share with self
    if data.target_organization_id == org_id {
        return Err(ApiError::BadRequest("Cannot share a board with the same organization".into()));
    }

    let id = Uuid::new_v4().to_string();
    let share = BoardShare::create(pool, &id, &data, &org_id, &access_context.user_id.to_string()).await?;
    Ok(Json(ApiResponse::success(share)))
}

/// GET /api/organizations/:org_id/board-shares — list shares FROM this org
pub async fn list_org_shares(
    Path(org_id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<BoardShare>>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_access(pool, &access_context, &org_id).await?;

    let shares = BoardShare::find_by_source_org(pool, &org_id).await?;
    Ok(Json(ApiResponse::success(shares)))
}

/// GET /api/organizations/:org_id/shared-boards — list boards shared TO this org
pub async fn list_shared_boards(
    Path(org_id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<BoardShare>>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_access(pool, &access_context, &org_id).await?;

    let shares = BoardShare::find_by_target_org(pool, &org_id).await?;
    Ok(Json(ApiResponse::success(shares)))
}

/// GET /api/board-shares/:id — share details
pub async fn get_board_share(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<BoardShare>>, ApiError> {
    let pool = &deployment.db().pool;
    let share = BoardShare::find_by_id(pool, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Board share not found".into()))?;

    // User must be member of source or target org
    let src_ok = require_org_access(pool, &access_context, &share.source_organization_id).await.is_ok();
    let tgt_ok = require_org_access(pool, &access_context, &share.target_organization_id).await.is_ok();
    if !src_ok && !tgt_ok {
        return Err(ApiError::Forbidden("Not a member of either organization".into()));
    }

    Ok(Json(ApiResponse::success(share)))
}

/// PUT /api/board-shares/:id — update permission/type/active
pub async fn update_board_share(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<UpdateBoardShare>,
) -> Result<Json<ApiResponse<BoardShare>>, ApiError> {
    let pool = &deployment.db().pool;
    let existing = BoardShare::find_by_id(pool, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Board share not found".into()))?;

    // Must be admin of source org to manage shares
    require_org_admin(pool, &access_context, &existing.source_organization_id).await?;

    let share = BoardShare::update(pool, &id, &data).await?;
    Ok(Json(ApiResponse::success(share)))
}

/// DELETE /api/board-shares/:id — remove share
pub async fn delete_board_share(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let existing = BoardShare::find_by_id(pool, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Board share not found".into()))?;

    require_org_admin(pool, &access_context, &existing.source_organization_id).await?;

    BoardShare::delete(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/organizations/{org_id}/board-shares",
            get(list_org_shares).post(create_board_share),
        )
        .route(
            "/organizations/{org_id}/shared-boards",
            get(list_shared_boards),
        )
        .route(
            "/board-shares/{id}",
            get(get_board_share).put(update_board_share).delete(delete_board_share),
        )
}

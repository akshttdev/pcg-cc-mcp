use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use db::models::{
    project::Project,
    project_folder::{CreateProjectFolder, ProjectFolder, UpdateProjectFolder},
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

/// Check if user has access to an org (is member or admin)
async fn require_org_access(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: Uuid,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id).await?;
    if role.is_none() {
        return Err(ApiError::Forbidden("Not a member of this organization".into()));
    }
    Ok(())
}

/// Check if user has admin access to an org
async fn require_org_admin(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: Uuid,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id).await?;
    match role.as_deref() {
        Some("admin") => Ok(()),
        _ => Err(ApiError::Forbidden("Only org admins can manage project folders".into())),
    }
}

/// GET /api/organizations/:org_id/project-folders — list folders in org
pub async fn list_folders(
    Path(org_id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ProjectFolder>>>, ApiError> {
    require_org_access(&deployment.db().pool, &access_context, org_id).await?;
    let folders = ProjectFolder::find_by_organization(&deployment.db().pool, org_id).await?;
    Ok(Json(ApiResponse::success(folders)))
}

/// POST /api/organizations/:org_id/project-folders — create folder
pub async fn create_folder(
    Path(org_id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateProjectFolder>,
) -> Result<Json<ApiResponse<ProjectFolder>>, ApiError> {
    require_org_admin(&deployment.db().pool, &access_context, org_id).await?;

    let id = Uuid::new_v4();
    let folder = ProjectFolder::create(&deployment.db().pool, id, org_id, &data).await?;
    Ok(Json(ApiResponse::success(folder)))
}

/// GET /api/project-folders/:id — folder details
pub async fn get_folder(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<ProjectFolder>>, ApiError> {
    let folder = ProjectFolder::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project folder not found".into()))?;

    require_org_access(&deployment.db().pool, &access_context, folder.organization_id).await?;
    Ok(Json(ApiResponse::success(folder)))
}

/// PUT /api/project-folders/:id — update folder
pub async fn update_folder(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<UpdateProjectFolder>,
) -> Result<Json<ApiResponse<ProjectFolder>>, ApiError> {
    let existing = ProjectFolder::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project folder not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, existing.organization_id).await?;

    let folder = ProjectFolder::update(&deployment.db().pool, id, &data).await?;
    Ok(Json(ApiResponse::success(folder)))
}

/// DELETE /api/project-folders/:id — delete folder
pub async fn delete_folder(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let existing = ProjectFolder::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project folder not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, existing.organization_id).await?;

    ProjectFolder::delete(&deployment.db().pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/project-folders/:id/projects/:project_id — add project to folder
pub async fn add_project_to_folder(
    Path((id, project_id)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let folder = ProjectFolder::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project folder not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, folder.organization_id).await?;

    // Verify project exists
    if !Project::exists(&deployment.db().pool, project_id).await? {
        return Err(ApiError::NotFound("Project not found".into()));
    }

    Project::set_folder(&deployment.db().pool, project_id, Some(id)).await?;
    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/project-folders/:id/projects/:project_id — remove project from folder
pub async fn remove_project_from_folder(
    Path((id, project_id)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let folder = ProjectFolder::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project folder not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, folder.organization_id).await?;

    Project::set_folder(&deployment.db().pool, project_id, None).await?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Org-scoped folder routes
        .route(
            "/organizations/{org_id}/project-folders",
            get(list_folders).post(create_folder),
        )
        // Direct folder routes
        .route(
            "/project-folders/{id}",
            get(get_folder).put(update_folder).delete(delete_folder),
        )
        // Folder membership routes
        .route(
            "/project-folders/{id}/projects/{project_id}",
            axum::routing::put(add_project_to_folder).delete(remove_project_from_folder),
        )
}

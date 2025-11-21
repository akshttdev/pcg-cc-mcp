use axum::{
    Router,
    extract::{Path, State, Extension},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post, delete},
    middleware,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use uuid::Uuid;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, middleware::{AccessContext, ProjectRole}};

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/projects/{project_id}/members", get(list_project_members))
        .route("/projects/{project_id}/members", post(add_project_member))
        .route("/projects/{project_id}/members/{user_id}", delete(remove_project_member))
        .route("/projects/{project_id}/members/{user_id}/role", post(update_member_role))
        .route("/projects/{project_id}/access", get(check_project_access))
        .route("/my-projects", get(list_my_projects))
        .layer(middleware::from_fn_with_state(
            deployment.clone(),
            crate::middleware::require_auth,
        ))
}

// Database row structure (BLOBs as Vec<u8>)
#[derive(Debug, FromRow)]
struct ProjectMemberRow {
    id: Vec<u8>,
    project_id: String,
    user_id: Vec<u8>,
    username: String,
    full_name: String,
    email: String,
    avatar_url: Option<String>,
    role: String,
    granted_at: String,
    granted_by_username: Option<String>,
}

// API response structure (UUIDs as Strings)
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectMemberItem {
    #[ts(type = "string")]
    pub id: String,
    pub project_id: String,
    #[ts(type = "string")]
    pub user_id: String,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub granted_at: String,
    pub granted_by_username: Option<String>,
}

impl From<ProjectMemberRow> for ProjectMemberItem {
    fn from(row: ProjectMemberRow) -> Self {
        Self {
            id: Uuid::from_slice(&row.id)
                .map(|u| u.to_string())
                .unwrap_or_default(),
            project_id: row.project_id,
            user_id: Uuid::from_slice(&row.user_id)
                .map(|u| u.to_string())
                .unwrap_or_default(),
            username: row.username,
            full_name: row.full_name,
            email: row.email,
            avatar_url: row.avatar_url,
            role: row.role,
            granted_at: row.granted_at,
            granted_by_username: row.granted_by_username,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AddProjectMemberRequest {
    pub user_id: String,
    pub role: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectAccessResponse {
    pub has_access: bool,
    pub role: Option<String>,
    pub can_read: bool,
    pub can_write: bool,
    pub can_manage_members: bool,
    pub can_delete: bool,
}

#[derive(Debug, Serialize, Deserialize, TS, FromRow)]
#[ts(export)]
pub struct MyProjectItem {
    pub project_id: String,
    pub project_name: String,
    pub role: String,
    pub granted_at: String,
}

/// GET /api/projects/:project_id/members - List project members
async fn list_project_members(
    State(deployment): State<DeploymentImpl>,
    Extension(context): Extension<AccessContext>,
    Path(project_id): Path<String>,
) -> Result<ResponseJson<ApiResponse<Vec<ProjectMemberItem>>>, ApiError> {
    let pool = deployment.db().pool.clone();
    
    // Check if user has at least viewer access to the project
    context.check_project_access(&pool, &project_id, ProjectRole::Viewer).await?;
    
    let rows: Vec<ProjectMemberRow> = sqlx::query_as(
        r#"
        SELECT 
            pm.id,
            pm.project_id,
            pm.user_id,
            u.username,
            u.full_name,
            u.email,
            u.avatar_url,
            pm.role,
            pm.granted_at,
            gb.username as granted_by_username
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        LEFT JOIN users gb ON pm.granted_by = gb.id
        WHERE pm.project_id = ?
        ORDER BY pm.granted_at DESC
        "#
    )
    .bind(&project_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch project members: {}", e)))?;
    
    let members: Vec<ProjectMemberItem> = rows.into_iter().map(Into::into).collect();
    
    Ok(ResponseJson(ApiResponse::success(members)))
}

/// POST /api/projects/:project_id/members - Add project member
async fn add_project_member(
    State(deployment): State<DeploymentImpl>,
    Extension(context): Extension<AccessContext>,
    Path(project_id): Path<String>,
    ResponseJson(req): ResponseJson<AddProjectMemberRequest>,
) -> Result<ResponseJson<ApiResponse<ProjectMemberItem>>, ApiError> {
    let pool = deployment.db().pool.clone();
    
    // Check if user has admin access to the project
    context.check_project_access(&pool, &project_id, ProjectRole::Admin).await?;
    
    // Validate role
    let role: ProjectRole = req.role.parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;
    
    // Parse user_id
    let user_id = Uuid::parse_str(&req.user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;
    
    // Check if user exists
    let user_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE id = ?"
    )
    .bind(user_id.as_bytes().to_vec())
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    
    if user_exists == 0 {
        return Err(ApiError::BadRequest("User not found".to_string()));
    }
    
    // Check if member already exists
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_members WHERE project_id = ? AND user_id = ?"
    )
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    
    if existing > 0 {
        return Err(ApiError::BadRequest("User is already a member of this project".to_string()));
    }
    
    let member_id = Uuid::new_v4();
    
    // Add member
    sqlx::query(
        r#"
        INSERT INTO project_members (id, project_id, user_id, role, granted_by)
        VALUES (?, ?, ?, ?, ?)
        "#
    )
    .bind(member_id.as_bytes().to_vec())
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .bind(role.to_string())
    .bind(context.user_id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to add project member: {}", e)))?;
    
    // Log the action
    log_permission_action(
        &pool,
        &user_id,
        "grant",
        "project",
        Some(&project_id),
        &format!(r#"{{"role":"{}"}}"#, role),
        &context.user_id,
    ).await?;
    
    // Fetch and return the created member
    let row: ProjectMemberRow = sqlx::query_as(
        r#"
        SELECT 
            pm.id,
            pm.project_id,
            pm.user_id,
            u.username,
            u.full_name,
            u.email,
            u.avatar_url,
            pm.role,
            pm.granted_at,
            gb.username as granted_by_username
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        LEFT JOIN users gb ON pm.granted_by = gb.id
        WHERE pm.id = ?
        "#
    )
    .bind(member_id.as_bytes().to_vec())
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch created member: {}", e)))?;
    
    Ok(ResponseJson(ApiResponse::success(row.into())))
}

/// DELETE /api/projects/:project_id/members/:user_id - Remove project member
async fn remove_project_member(
    State(deployment): State<DeploymentImpl>,
    Extension(context): Extension<AccessContext>,
    Path((project_id, user_id_str)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let pool = deployment.db().pool.clone();
    
    // Check if user has admin access to the project
    context.check_project_access(&pool, &project_id, ProjectRole::Admin).await?;
    
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;
    
    // Don't allow removing the last owner
    let owner_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_members WHERE project_id = ? AND role = 'owner'"
    )
    .bind(&project_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    
    let member_role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM project_members WHERE project_id = ? AND user_id = ?"
    )
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    
    if member_role == Some("owner".to_string()) && owner_count <= 1 {
        return Err(ApiError::BadRequest(
            "Cannot remove the last owner from the project".to_string()
        ));
    }
    
    // Remove member
    sqlx::query(
        "DELETE FROM project_members WHERE project_id = ? AND user_id = ?"
    )
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to remove project member: {}", e)))?;
    
    // Log the action
    log_permission_action(
        &pool,
        &user_id,
        "revoke",
        "project",
        Some(&project_id),
        "{}",
        &context.user_id,
    ).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/projects/:project_id/members/:user_id/role - Update member role
async fn update_member_role(
    State(deployment): State<DeploymentImpl>,
    Extension(context): Extension<AccessContext>,
    Path((project_id, user_id_str)): Path<(String, String)>,
    ResponseJson(req): ResponseJson<UpdateMemberRoleRequest>,
) -> Result<ResponseJson<ApiResponse<ProjectMemberItem>>, ApiError> {
    let pool = deployment.db().pool.clone();
    
    // Check if user has admin access to the project
    context.check_project_access(&pool, &project_id, ProjectRole::Admin).await?;
    
    // Validate role
    let role: ProjectRole = req.role.parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;
    
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;
    
    // Don't allow changing the last owner's role
    let owner_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_members WHERE project_id = ? AND role = 'owner'"
    )
    .bind(&project_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    
    let current_role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM project_members WHERE project_id = ? AND user_id = ?"
    )
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
    
    if current_role == Some("owner".to_string()) && owner_count <= 1 && role != ProjectRole::Owner {
        return Err(ApiError::BadRequest(
            "Cannot change the role of the last owner".to_string()
        ));
    }
    
    // Update role
    sqlx::query(
        "UPDATE project_members SET role = ? WHERE project_id = ? AND user_id = ?"
    )
    .bind(role.to_string())
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to update member role: {}", e)))?;
    
    // Log the action
    log_permission_action(
        &pool,
        &user_id,
        "modify",
        "project",
        Some(&project_id),
        &format!(r#"{{"new_role":"{}"}}"#, role),
        &context.user_id,
    ).await?;
    
    // Fetch and return updated member
    let row: ProjectMemberRow = sqlx::query_as(
        r#"
        SELECT 
            pm.id,
            pm.project_id,
            pm.user_id,
            u.username,
            u.full_name,
            u.email,
            u.avatar_url,
            pm.role,
            pm.granted_at,
            gb.username as granted_by_username
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        LEFT JOIN users gb ON pm.granted_by = gb.id
        WHERE pm.project_id = ? AND pm.user_id = ?
        "#
    )
    .bind(&project_id)
    .bind(user_id.as_bytes().to_vec())
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch updated member: {}", e)))?;
    
    Ok(ResponseJson(ApiResponse::success(row.into())))
}

/// GET /api/projects/:project_id/access - Check current user's access to project
async fn check_project_access(
    State(deployment): State<DeploymentImpl>,
    Extension(context): Extension<AccessContext>,
    Path(project_id): Path<String>,
) -> Result<ResponseJson<ApiResponse<ProjectAccessResponse>>, ApiError> {
    let pool = deployment.db().pool.clone();
    
    let role_opt = context.get_project_role(&pool, &project_id).await?;
    
    let response = match role_opt {
        Some(role) => ProjectAccessResponse {
            has_access: true,
            role: Some(role.to_string()),
            can_read: role.can_read(),
            can_write: role.can_write(),
            can_manage_members: role.can_manage_members(),
            can_delete: role.can_delete(),
        },
        None => ProjectAccessResponse {
            has_access: false,
            role: None,
            can_read: false,
            can_write: false,
            can_manage_members: false,
            can_delete: false,
        },
    };
    
    Ok(ResponseJson(ApiResponse::success(response)))
}

/// GET /api/projects/my-projects - List all projects the current user has access to
async fn list_my_projects(
    State(deployment): State<DeploymentImpl>,
    Extension(context): Extension<AccessContext>,
) -> Result<ResponseJson<ApiResponse<Vec<MyProjectItem>>>, ApiError> {
    let pool = deployment.db().pool.clone();
    
    // If admin, return all projects
    if context.is_admin {
        // This would need to join with actual projects table
        // For now, return empty list for admins as they have access to all
        return Ok(ResponseJson(ApiResponse::success(vec![])));
    }
    
    let projects: Vec<MyProjectItem> = sqlx::query_as(
        r#"
        SELECT 
            pm.project_id,
            pm.project_id as project_name,
            pm.role,
            pm.granted_at
        FROM project_members pm
        WHERE pm.user_id = ?
        ORDER BY pm.granted_at DESC
        "#
    )
    .bind(context.user_id.as_bytes().to_vec())
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch projects: {}", e)))?;
    
    Ok(ResponseJson(ApiResponse::success(projects)))
}

/// Helper function to log permission actions
async fn log_permission_action(
    pool: &sqlx::SqlitePool,
    user_id: &Uuid,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    details: &str,
    performed_by: &Uuid,
) -> Result<(), ApiError> {
    let log_id = Uuid::new_v4();
    
    sqlx::query(
        r#"
        INSERT INTO permission_audit_log 
        (id, user_id, action, resource_type, resource_id, details, performed_by)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(log_id.as_bytes().to_vec())
    .bind(user_id.as_bytes().to_vec())
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(details)
    .bind(performed_by.as_bytes().to_vec())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to log permission action: {}", e)))?;
    
    Ok(())
}

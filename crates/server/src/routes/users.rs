use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{delete, get, patch, post},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}", get(get_user))
        .route("/users/{id}", patch(update_user))
        .route("/users/{id}", delete(deactivate_user))
        .route("/users/{id}/role", patch(update_user_role))
        .route("/users/{id}/suspend", patch(suspend_user))
        .route("/users/{id}/activate", patch(activate_user))
        .route("/users/create", post(create_user))
}

#[derive(Debug, Serialize, Deserialize, TS, FromRow)]
#[ts(export)]
pub struct UserListItem {
    #[ts(type = "string")]
    #[sqlx(try_from = "Vec<u8>")]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_active: i32,
    pub is_admin: i32,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, TS, FromRow)]
#[ts(export)]
pub struct UserDetail {
    #[ts(type = "string")]
    #[sqlx(try_from = "Vec<u8>")]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub is_active: i32,
    pub is_admin: i32,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub is_admin: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateUserRequest {
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateRoleRequest {
    pub is_admin: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub full_name: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateUserResponse {
    pub message: String,
    #[ts(type = "string")]
    pub user_id: Uuid,
    pub username: String,
}

/// GET /api/users - List all users (admin only)
/// Uses parameterized queries to prevent SQL injection
async fn list_users(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ListUsersQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<UserListItem>>>, ApiError> {
    let pool = deployment.db().pool.clone();

    // Use parameterized query to prevent SQL injection
    // Build search pattern safely
    let search_pattern = params.search.as_ref().map(|s| format!("%{}%", s));

    // Validate and sanitize limit/offset
    let limit = params.limit.unwrap_or(100).min(1000).max(1);
    let offset = params.offset.unwrap_or(0).max(0);

    let is_active_filter = params.is_active.map(|b| if b { 1i32 } else { 0i32 });
    let is_admin_filter = params.is_admin.map(|b| if b { 1i32 } else { 0i32 });

    let users = sqlx::query_as::<_, UserListItem>(
        r#"
        SELECT
            id, username, email, full_name, avatar_url,
            is_active, is_admin, last_login_at, created_at
        FROM users
        WHERE
            (? IS NULL OR username LIKE ? OR email LIKE ? OR full_name LIKE ?)
            AND (? IS NULL OR is_active = ?)
            AND (? IS NULL OR is_admin = ?)
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&search_pattern)
    .bind(&search_pattern)
    .bind(&search_pattern)
    .bind(&search_pattern)
    .bind(&is_active_filter)
    .bind(&is_active_filter)
    .bind(&is_admin_filter)
    .bind(&is_admin_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch users: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(users)))
}

/// GET /api/users/:id - Get user details
async fn get_user(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<UserDetail>>, ApiError> {
    let pool = deployment.db().pool.clone();

    let user = sqlx::query_as::<_, UserDetail>(
        r#"
        SELECT 
            id,
            username,
            email,
            full_name,
            avatar_url,
            is_active,
            is_admin,
            last_login_at,
            created_at,
            updated_at
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(id.as_bytes().to_vec())
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(ResponseJson(ApiResponse::success(user)))
}

/// PATCH /api/users/:id - Update user details
async fn update_user(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(req): ResponseJson<UpdateUserRequest>,
) -> Result<ResponseJson<ApiResponse<UserDetail>>, ApiError> {
    let pool = deployment.db().pool.clone();

    // Build update query dynamically
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(full_name) = &req.full_name {
        updates.push("full_name = ?");
        params.push(full_name.clone());
    }

    if let Some(email) = &req.email {
        updates.push("email = ?");
        params.push(email.clone());
    }

    if let Some(avatar_url) = &req.avatar_url {
        updates.push("avatar_url = ?");
        params.push(avatar_url.clone());
    }

    if updates.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    updates.push("updated_at = datetime('now')");

    let query = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));

    let mut query_builder = sqlx::query(&query);
    for param in params {
        query_builder = query_builder.bind(param);
    }
    query_builder = query_builder.bind(id);

    query_builder
        .execute(&pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update user: {}", e)))?;

    // Return updated user
    get_user(State(deployment), Path(id)).await
}

/// PATCH /api/users/:id/role - Update user role (admin only)
async fn update_user_role(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(req): ResponseJson<UpdateRoleRequest>,
) -> Result<ResponseJson<ApiResponse<UserDetail>>, ApiError> {
    let pool = deployment.db().pool.clone();

    let is_admin_i32 = if req.is_admin { 1i32 } else { 0i32 };

    sqlx::query(
        r#"
        UPDATE users 
        SET is_admin = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(is_admin_i32)
    .bind(id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to update role: {}", e)))?;

    get_user(State(deployment), Path(id)).await
}

/// PATCH /api/users/:id/suspend - Suspend user (admin only)
async fn suspend_user(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<UserDetail>>, ApiError> {
    let pool = deployment.db().pool.clone();

    sqlx::query(
        r#"
        UPDATE users 
        SET is_active = 0, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to suspend user: {}", e)))?;

    get_user(State(deployment), Path(id)).await
}

/// PATCH /api/users/:id/activate - Activate user (admin only)
async fn activate_user(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<UserDetail>>, ApiError> {
    let pool = deployment.db().pool.clone();

    sqlx::query(
        r#"
        UPDATE users 
        SET is_active = 1, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to activate user: {}", e)))?;

    get_user(State(deployment), Path(id)).await
}

/// DELETE /api/users/:id - Deactivate user (soft delete)
async fn deactivate_user(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = deployment.db().pool.clone();

    sqlx::query(
        r#"
        UPDATE users 
        SET is_active = 0, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(id.as_bytes().to_vec())
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to deactivate user: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/users/create - Create new user with credentials (admin only)
async fn create_user(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<CreateUserRequest>,
) -> Result<ResponseJson<ApiResponse<CreateUserResponse>>, ApiError> {
    let pool = deployment.db().pool.clone();

    // Validate username
    if req.username.is_empty() || req.username.len() < 3 {
        return Err(ApiError::BadRequest(
            "Username must be at least 3 characters".to_string(),
        ));
    }

    // Validate password
    if req.password.len() < 6 {
        return Err(ApiError::BadRequest(
            "Password must be at least 6 characters".to_string(),
        ));
    }

    // Check if username already exists
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_one(&pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if count > 0 {
        return Err(ApiError::BadRequest("Username already exists".to_string()));
    }

    // Generate email if not provided (username@local.system)
    // This ensures compatibility with the NOT NULL constraint
    let email = req
        .email
        .unwrap_or_else(|| format!("{}@local.system", req.username));

    // Check if email already exists
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_one(&pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if count > 0 {
        return Err(ApiError::BadRequest("Email already exists".to_string()));
    }

    // Hash the password using the db auth service
    let password_hash = db::services::AuthService::hash_password(&req.password)
        .map_err(|e| ApiError::InternalError(format!("Failed to hash password: {}", e)))?;

    let user_id = Uuid::new_v4();
    let is_admin_i32 = if req.is_admin { 1i32 } else { 0i32 };

    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
        VALUES (?, ?, ?, ?, ?, ?, 1)
        "#,
    )
    .bind(user_id.as_bytes().to_vec())
    .bind(&req.username)
    .bind(&email)
    .bind(&req.full_name)
    .bind(&password_hash)
    .bind(is_admin_i32)
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create user: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(CreateUserResponse {
        message: "User created successfully".to_string(),
        user_id,
        username: req.username.clone(),
    })))
}

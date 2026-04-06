use axum::{
    extract::{Extension, Path, Query, State},
    response::Json as ResponseJson,
    routing::{delete, get, put},
    Router,
};
use db::{
    db_uuid::DbUuid,
    models::{activity::ActivityLog, notification::Notification},
};
use deployment::Deployment;
use serde::Deserialize;
use sqlx::FromRow;
use utils::response::ApiResponse;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub limit: Option<i64>,
}

/// GET /notifications
/// Returns recent activity across all projects the current user has access to.
pub async fn get_notifications(
    State(deployment): State<DeploymentImpl>,
    Extension(access): Extension<AccessContext>,
    Query(query): Query<NotificationQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<ActivityLog>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50).min(200);

    // Get project IDs the user has access to
    // IMPORTANT: Use DbUuid to read IDs — projects table may store UUIDs as BLOB or TEXT.
    let project_ids: Vec<DbUuid> = if access.is_admin {
        #[derive(FromRow)]
        struct ProjectId {
            id: DbUuid,
        }
        let rows: Vec<ProjectId> = sqlx::query_as("SELECT id FROM projects")
            .fetch_all(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
        rows.into_iter().map(|r| r.id).collect()
    } else {
        #[derive(FromRow)]
        struct ProjectId {
            project_id: DbUuid,
        }
        // project_members.user_id is BLOB — bind as raw bytes for correct comparison
        let user_id_bytes = access.user_id.to_string();
        let rows: Vec<ProjectId> =
            sqlx::query_as("SELECT project_id FROM project_members WHERE user_id = ?")
                .bind(&user_id_bytes)
                .fetch_all(pool)
                .await
                .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
        rows.into_iter().map(|r| r.project_id).collect()
    };

    let activity = ActivityLog::find_recent_for_projects(pool, &project_ids, limit).await?;
    Ok(ResponseJson(ApiResponse::success(activity)))
}

/// GET /notifications/inbox
/// Returns in-app notifications for the current user.
pub async fn get_inbox(
    State(deployment): State<DeploymentImpl>,
    Extension(access): Extension<AccessContext>,
    Query(query): Query<NotificationQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<Notification>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit.unwrap_or(50).min(200);
    let user_id = access.user_id.to_string();
    let notifications = Notification::find_by_user(pool, &user_id, limit).await?;
    Ok(ResponseJson(ApiResponse::success(notifications)))
}

/// GET /notifications/unread-count
/// Returns the count of unread notifications for the current user.
pub async fn get_unread_count(
    State(deployment): State<DeploymentImpl>,
    Extension(access): Extension<AccessContext>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let user_id = access.user_id.to_string();
    let count = Notification::count_unread(pool, &user_id).await?;
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "count": count }),
    )))
}

/// PUT /notifications/:id/read
/// Mark a single notification as read (scoped to current user).
pub async fn mark_read(
    State(deployment): State<DeploymentImpl>,
    Extension(access): Extension<AccessContext>,
    Path(id): Path<String>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let user_id = access.user_id.to_string();
    Notification::mark_read(pool, &id, &user_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// PUT /notifications/mark-all-read
/// Mark all notifications as read for the current user.
pub async fn mark_all_read(
    State(deployment): State<DeploymentImpl>,
    Extension(access): Extension<AccessContext>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let user_id = access.user_id.to_string();
    let count = Notification::mark_all_read(pool, &user_id).await?;
    Ok(ResponseJson(ApiResponse::success(
        serde_json::json!({ "marked": count }),
    )))
}

/// DELETE /notifications/:id
/// Delete a notification (scoped to current user).
pub async fn delete_notification(
    State(deployment): State<DeploymentImpl>,
    Extension(access): Extension<AccessContext>,
    Path(id): Path<String>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let user_id = access.user_id.to_string();
    Notification::delete(pool, &id, &user_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/notifications", get(get_notifications))
        .route("/notifications/inbox", get(get_inbox))
        .route("/notifications/unread-count", get(get_unread_count))
        .route("/notifications/mark-all-read", put(mark_all_read))
        .route("/notifications/{id}/read", put(mark_read))
        .route("/notifications/{id}", delete(delete_notification))
}

use axum::{
    Router,
    extract::{Extension, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::activity::ActivityLog;
use deployment::Deployment;
use serde::Deserialize;
use sqlx::FromRow;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use crate::middleware::access_control::AccessContext;

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
    let project_ids: Vec<Uuid> = if access.is_admin {
        // Admins see all projects
        #[derive(FromRow)]
        struct ProjectId {
            id: Vec<u8>,
        }
        let rows: Vec<ProjectId> = sqlx::query_as("SELECT id FROM projects")
            .fetch_all(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
        rows.iter()
            .filter_map(|r| Uuid::from_slice(&r.id).ok())
            .collect()
    } else {
        #[derive(FromRow)]
        struct ProjectId {
            project_id: Vec<u8>,
        }
        let rows: Vec<ProjectId> =
            sqlx::query_as("SELECT project_id FROM project_members WHERE user_id = ?")
                .bind(access.user_id.as_bytes().as_slice())
                .fetch_all(pool)
                .await
                .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;
        rows.iter()
            .filter_map(|r| Uuid::from_slice(&r.project_id).ok())
            .collect()
    };

    let activity = ActivityLog::find_recent_for_projects(pool, &project_ids, limit).await?;
    Ok(ResponseJson(ApiResponse::success(activity)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/notifications", get(get_notifications))
}

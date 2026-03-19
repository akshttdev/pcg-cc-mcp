use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::session::{CreateSession, Session, UpdateSession};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub task_attempt_id: Option<Uuid>,
}

pub async fn get_sessions(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<SessionQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<Session>>>, ApiError> {
    let pool = &deployment.db().pool;
    let sessions = match query.task_attempt_id {
        Some(attempt_id) => Session::find_by_task_attempt_id(pool, attempt_id).await?,
        None => {
            return Err(ApiError::BadRequest(
                "task_attempt_id query parameter is required".to_string(),
            ));
        }
    };
    Ok(ResponseJson(ApiResponse::success(sessions)))
}

pub async fn get_session(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Session>>, ApiError> {
    let session = Session::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or(ApiError::NotFound("Session not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(session)))
}

pub async fn create_session(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateSession>,
) -> Result<ResponseJson<ApiResponse<Session>>, ApiError> {
    let session = Session::create(&deployment.db().pool, &data).await?;
    Ok(ResponseJson(ApiResponse::success(session)))
}

pub async fn update_session(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateSession>,
) -> Result<ResponseJson<ApiResponse<Session>>, ApiError> {
    let session = Session::update(&deployment.db().pool, id, &data)
        .await?
        .ok_or(ApiError::NotFound("Session not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(session)))
}

pub async fn delete_session(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Session::delete(&deployment.db().pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/sessions",
        Router::new()
            .route("/", get(get_sessions).post(create_session))
            .route(
                "/{id}",
                get(get_session).put(update_session).delete(delete_session),
            ),
    )
}

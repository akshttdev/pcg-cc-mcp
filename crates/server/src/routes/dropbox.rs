use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get},
    Router,
};
use db::models::dropbox_source::{CreateDropboxSource, DropboxSource};
use deployment::Deployment;
use tracing::{error, info};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::DeploymentImpl;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/dropbox/sources", get(list_sources).post(create_source))
        .route("/dropbox/sources/{id}", delete(delete_source))
}

async fn list_sources(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<DropboxSource>>>, StatusCode> {
    let pool = &deployment.db().pool;
    match DropboxSource::list(pool).await {
        Ok(sources) => Ok(Json(ApiResponse::success(sources))),
        Err(err) => {
            error!("Failed to list Dropbox sources: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn create_source(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateDropboxSource>,
) -> Result<Json<ApiResponse<DropboxSource>>, StatusCode> {
    let pool = &deployment.db().pool;
    match DropboxSource::create(pool, payload).await {
        Ok(source) => {
            info!("Created Dropbox source {}", source.id);
            Ok(Json(ApiResponse::success(source)))
        }
        Err(err) => {
            error!("Failed to create Dropbox source: {}", err);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn delete_source(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let pool = &deployment.db().pool;
    match DropboxSource::delete(pool, id).await {
        Ok(()) => {
            info!("Deleted Dropbox source {}", id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(err) => {
            error!("Failed to delete Dropbox source {}: {}", id, err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

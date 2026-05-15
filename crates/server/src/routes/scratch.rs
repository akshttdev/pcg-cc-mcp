use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::scratch::{CreateScratch, Scratch, UpdateScratch};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct ScratchQuery {
    pub scratch_type: Option<String>,
}

pub async fn get_scratches(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ScratchQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<Scratch>>>, ApiError> {
    let pool = &deployment.db().pool;
    let scratches = match query.scratch_type {
        Some(t) => Scratch::find_by_type(pool, &t).await?,
        None => Scratch::find_all(pool).await?,
    };
    Ok(ResponseJson(ApiResponse::success(scratches)))
}

pub async fn get_scratch(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Scratch>>, ApiError> {
    let scratch = Scratch::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or(ApiError::NotFound("Scratch not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(scratch)))
}

pub async fn upsert_scratch(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateScratch>,
) -> Result<ResponseJson<ApiResponse<Scratch>>, ApiError> {
    let scratch = Scratch::upsert(&deployment.db().pool, &data).await?;
    Ok(ResponseJson(ApiResponse::success(scratch)))
}

pub async fn update_scratch(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateScratch>,
) -> Result<ResponseJson<ApiResponse<Scratch>>, ApiError> {
    let scratch = Scratch::update(&deployment.db().pool, id, &data)
        .await?
        .ok_or(ApiError::NotFound("Scratch not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(scratch)))
}

pub async fn delete_scratch(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Scratch::delete(&deployment.db().pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/scratch",
        Router::new()
            .route("/", get(get_scratches).post(upsert_scratch))
            .route(
                "/{id}",
                get(get_scratch).put(update_scratch).delete(delete_scratch),
            ),
    )
}

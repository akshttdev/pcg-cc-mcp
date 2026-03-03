//! Operator Rates — admin-only.
//! Rates are private: only visible to org admins and project managers.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::operator_rate::{OperatorRate, UpsertOperatorRate};

#[derive(Debug, Deserialize)]
pub struct ListRatesParams {
    pub org_id: Option<Uuid>,
    pub operator_id: Option<Uuid>,
}

/// POST /api/operator-rates  (upsert)
async fn upsert_rate(
    State(d): State<DeploymentImpl>,
    Json(body): Json<UpsertOperatorRate>,
) -> Result<Json<ApiResponse<OperatorRate>>, ApiError> {
    let rate = OperatorRate::upsert(&d.db().pool, body).await?;
    Ok(Json(ApiResponse::success(rate)))
}

/// GET /api/operator-rates?org_id=...&operator_id=...
async fn list_rates(
    State(d): State<DeploymentImpl>,
    Query(p): Query<ListRatesParams>,
) -> Result<Json<ApiResponse<Vec<OperatorRate>>>, ApiError> {
    let rates = if let Some(org_id) = p.org_id {
        OperatorRate::list_for_org(&d.db().pool, org_id).await?
    } else if let Some(op_id) = p.operator_id {
        OperatorRate::list_for_operator(&d.db().pool, op_id).await?
    } else {
        return Err(ApiError::BadRequest(
            "Provide org_id or operator_id query param".into(),
        ));
    };
    Ok(Json(ApiResponse::success(rates)))
}

/// DELETE /api/operator-rates/:id
async fn delete_rate(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let deleted = OperatorRate::delete(&d.db().pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound("Operator rate not found".into()))
    }
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/operator-rates", get(list_rates).post(upsert_rate))
        .route("/operator-rates/{id}", delete(delete_rate))
        .with_state(deployment.clone())
}

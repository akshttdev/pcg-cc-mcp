//! CRM Activity Routes
//!
//! Handles activity listing and creation for the CRM timeline.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json, Router,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};
use db::models::crm_activity::{CrmActivity, CreateCrmActivity};

#[derive(Debug, Deserialize)]
pub struct ListActivitiesQuery {
    pub organization_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub deal_id: Option<Uuid>,
    pub limit: Option<i32>,
}

/// GET /crm/activities - List activities with optional filters
async fn list_activities(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListActivitiesQuery>,
) -> Result<Json<ApiResponse<Vec<CrmActivity>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = query.limit;

    let activities = if let Some(contact_id) = query.contact_id {
        CrmActivity::find_by_contact(pool, contact_id, limit).await?
    } else if let Some(deal_id) = query.deal_id {
        CrmActivity::find_by_deal(pool, deal_id, limit).await?
    } else if let Some(org_id) = query.organization_id {
        CrmActivity::find_by_organization(pool, org_id, limit).await?
    } else {
        return Err(ApiError::BadRequest(
            "Must provide organization_id, contact_id, or deal_id".to_string(),
        ));
    };

    Ok(Json(ApiResponse::success(activities)))
}

/// POST /crm/activities - Create a new activity
async fn create_activity(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateCrmActivity>,
) -> Result<Json<ApiResponse<CrmActivity>>, ApiError> {
    let pool = &deployment.db().pool;
    let activity = CrmActivity::create(pool, data).await?;
    Ok(Json(ApiResponse::success(activity)))
}

/// DELETE /crm/activities/:id - Delete an activity
async fn delete_activity(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmActivity::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/crm/activities", get(list_activities))
        .route("/crm/activities", post(create_activity))
        .route("/crm/activities/{id}", delete(delete_activity))
}

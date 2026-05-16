use axum::{extract::State, routing::post, Extension, Json, Router};
use db::models::entity_conversion;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ConvertEntityRequest {
    pub source_type: String,
    pub source_id: Uuid,
    pub target_type: String,
    pub target_parent_id: Option<Uuid>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ConvertEntityResponse {
    pub new_id: String,
    pub new_type: String,
}

pub async fn convert_entity(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ConvertEntityRequest>,
) -> Result<Json<ApiResponse<ConvertEntityResponse>>, ApiError> {
    // Only admins can convert entities
    if !access_context.is_admin {
        return Err(ApiError::Forbidden(
            "Only admins can convert entities".into(),
        ));
    }

    let pool = &deployment.db().pool;

    let result = match (payload.source_type.as_str(), payload.target_type.as_str()) {
        ("organization", "client") => {
            let target_org_id = payload.target_parent_id.ok_or_else(|| {
                ApiError::BadRequest(
                    "target_parent_id (destination org) is required for org → client conversion"
                        .into(),
                )
            })?;
            let new_id = entity_conversion::org_to_client(pool, payload.source_id, target_org_id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            ConvertEntityResponse {
                new_id: new_id.to_string(),
                new_type: "client".into(),
            }
        }
        ("organization", "project") => {
            let target_org_id = payload.target_parent_id.ok_or_else(|| {
                ApiError::BadRequest(
                    "target_parent_id (destination org) is required for org → project conversion"
                        .into(),
                )
            })?;
            let new_id = entity_conversion::org_to_project(pool, payload.source_id, target_org_id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            ConvertEntityResponse {
                new_id: new_id.to_string(),
                new_type: "project".into(),
            }
        }
        ("client", "organization") => {
            let new_id = entity_conversion::client_to_org(pool, payload.source_id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            ConvertEntityResponse {
                new_id: new_id.to_string(),
                new_type: "organization".into(),
            }
        }
        ("client", "project") => {
            let new_id = entity_conversion::client_to_project(
                pool,
                payload.source_id,
                payload.target_parent_id,
            )
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            ConvertEntityResponse {
                new_id: new_id.to_string(),
                new_type: "project".into(),
            }
        }
        ("project", "client") => {
            let new_id = entity_conversion::project_to_client(pool, payload.source_id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            ConvertEntityResponse {
                new_id: new_id.to_string(),
                new_type: "client".into(),
            }
        }
        ("project", "organization") => {
            let new_id = entity_conversion::project_to_org(pool, payload.source_id)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            ConvertEntityResponse {
                new_id: new_id.to_string(),
                new_type: "organization".into(),
            }
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported conversion: {} → {}",
                payload.source_type, payload.target_type
            )));
        }
    };

    Ok(Json(ApiResponse::success(result)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/entities/convert", post(convert_entity))
}

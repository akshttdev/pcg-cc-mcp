use axum::{
    Json,
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use cinematics::{CinematicsConfig, CinematicsService, Cinematographer};
use deployment::Deployment;
use db::models::cinematic_brief::{CinematicBrief, CinematicShotPlan, CreateCinematicBrief};
use serde::Deserialize;
use serde_json::Value;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct CreateCinematicBriefPayload {
    pub project_id: Uuid,
    pub requester_id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub script: Option<String>,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub fps: Option<i64>,
    #[serde(default)]
    pub style_tags: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub auto_render: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListBriefsQuery {
    pub project_id: Uuid,
}

fn cinematics_service(deployment: &DeploymentImpl) -> CinematicsService {
    CinematicsService::new(deployment.db().pool.clone(), CinematicsConfig::default())
}

fn map_err(err: anyhow::Error) -> ApiError {
    ApiError::InternalError(err.to_string())
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/nora/cinematics/briefs",
            post(create_brief).get(list_briefs),
        )
        .route("/nora/cinematics/briefs/{id}", get(get_brief))
        .route(
            "/nora/cinematics/briefs/{id}/render",
            post(trigger_render),
        )
        .route("/nora/cinematics/briefs/{id}/shots", get(list_shots))
        .with_state(deployment.clone())
}

pub async fn create_brief(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateCinematicBriefPayload>,
) -> Result<ResponseJson<ApiResponse<CinematicBrief>>, ApiError> {
    let service = cinematics_service(&deployment);
    let brief = service
        .create_brief(CreateCinematicBrief {
            project_id: payload.project_id,
            requester_id: payload.requester_id,
            nora_session_id: payload.session_id,
            title: payload.title,
            summary: payload.summary,
            script: payload.script,
            asset_ids: payload.asset_ids,
            duration_seconds: payload.duration_seconds,
            fps: payload.fps,
            style_tags: payload.style_tags,
            metadata: payload.metadata,
        })
        .await
        .map_err(map_err)?;

    let should_render = payload
        .auto_render
        .unwrap_or_else(|| service.auto_render_enabled());

    let final_brief = if should_render {
        service.trigger_render(brief.id).await.map_err(map_err)?
    } else {
        brief
    };

    Ok(ResponseJson(ApiResponse::success(final_brief)))
}

pub async fn list_briefs(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListBriefsQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<CinematicBrief>>>, ApiError> {
    let briefs = CinematicBrief::list_by_project(&deployment.db().pool, query.project_id).await?;
    Ok(ResponseJson(ApiResponse::success(briefs)))
}

pub async fn get_brief(
    State(deployment): State<DeploymentImpl>,
    Path(brief_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<CinematicBrief>>, ApiError> {
    let brief = CinematicBrief::find_by_id(&deployment.db().pool, brief_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Brief not found".into()))?;
    Ok(ResponseJson(ApiResponse::success(brief)))
}

pub async fn list_shots(
    State(deployment): State<DeploymentImpl>,
    Path(brief_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<CinematicShotPlan>>>, ApiError> {
    let shots = CinematicShotPlan::list_by_brief(&deployment.db().pool, brief_id).await?;
    Ok(ResponseJson(ApiResponse::success(shots)))
}

pub async fn trigger_render(
    State(deployment): State<DeploymentImpl>,
    Path(brief_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<CinematicBrief>>, ApiError> {
    let service = cinematics_service(&deployment);
    let brief = service.trigger_render(brief_id).await.map_err(map_err)?;
    Ok(ResponseJson(ApiResponse::success(brief)))
}

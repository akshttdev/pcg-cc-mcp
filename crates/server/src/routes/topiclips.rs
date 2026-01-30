use axum::{
    Json,
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post, put},
};
use db::models::topiclip::{
    CreateTopiClipConfig, CreateTopiClipDailySchedule, TopiClipConfig, TopiClipDailySchedule,
    TopiClipGalleryResponse, TopiClipSession, TopiClipSymbol, TopiClipTimelineEntry,
    TopiClipTriggerType,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use topiclips::{TopiClipGenerator, TopiClipsConfig, TopiClipsService};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionPayload {
    pub project_id: Uuid,
    #[serde(default)]
    pub trigger_type: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionsQuery {
    pub project_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GalleryQuery {
    pub project_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DailySchedulePayload {
    pub project_id: Uuid,
    pub scheduled_time: String,
    pub timezone: Option<String>,
    pub min_significance_threshold: Option<f64>,
    pub force_daily: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSchedulePayload {
    pub scheduled_time: Option<String>,
    pub timezone: Option<String>,
    pub is_enabled: Option<bool>,
    pub min_significance_threshold: Option<f64>,
    pub force_daily: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPayload {
    pub project_id: Uuid,
    pub default_style: Option<String>,
    pub color_palette: Option<Vec<String>>,
    pub visual_density: Option<String>,
    pub motion_intensity: Option<String>,
    pub llm_model: Option<String>,
    pub interpretation_temperature: Option<f64>,
    pub output_resolution: Option<String>,
    pub output_fps: Option<i64>,
    pub output_format: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DailyInfoResponse {
    pub schedule: Option<TopiClipDailySchedule>,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub total_clips: i64,
    pub last_generation: Option<String>,
}

// ============================================================================
// Service Helper
// ============================================================================

fn topiclips_service(deployment: &DeploymentImpl) -> TopiClipsService {
    TopiClipsService::new(deployment.db().pool.clone(), TopiClipsConfig::default())
}

fn map_err(err: anyhow::Error) -> ApiError {
    ApiError::InternalError(err.to_string())
}

// ============================================================================
// Routes
// ============================================================================

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Session endpoints
        .route("/topiclips/sessions", post(create_session).get(list_sessions))
        .route("/topiclips/sessions/{id}", get(get_session))
        .route("/topiclips/sessions/{id}/generate", post(generate_session))
        .route("/topiclips/sessions/{id}/timeline", get(get_timeline))
        // Gallery endpoint
        .route("/topiclips/gallery", get(get_gallery))
        // Daily schedule endpoints
        .route(
            "/topiclips/daily",
            get(get_daily_info).post(create_daily_schedule),
        )
        .route("/topiclips/daily/{project_id}", put(update_daily_schedule))
        .route("/topiclips/daily/{project_id}/generate", post(force_daily_generate))
        // Config endpoints
        .route("/topiclips/config", get(get_config).put(upsert_config))
        // Symbol library
        .route("/topiclips/symbols", get(list_symbols))
        .with_state(deployment.clone())
}

// ============================================================================
// Session Handlers
// ============================================================================

pub async fn create_session(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateSessionPayload>,
) -> Result<ResponseJson<ApiResponse<TopiClipSession>>, ApiError> {
    let service = topiclips_service(&deployment);

    let trigger_type = match payload.trigger_type.as_deref() {
        Some("daily") => TopiClipTriggerType::Daily,
        Some("event") => TopiClipTriggerType::Event,
        _ => TopiClipTriggerType::Manual,
    };

    let session = service
        .create_session(
            payload.project_id,
            trigger_type,
            payload.period_start,
            payload.period_end,
        )
        .await
        .map_err(map_err)?;

    Ok(ResponseJson(ApiResponse::success(session)))
}

pub async fn list_sessions(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TopiClipSession>>>, ApiError> {
    let sessions =
        TopiClipSession::list_by_project(&deployment.db().pool, query.project_id).await?;
    Ok(ResponseJson(ApiResponse::success(sessions)))
}

pub async fn get_session(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TopiClipSession>>, ApiError> {
    let session = TopiClipSession::find_by_id(&deployment.db().pool, session_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Session not found".into()))?;
    Ok(ResponseJson(ApiResponse::success(session)))
}

pub async fn generate_session(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TopiClipSession>>, ApiError> {
    let service = topiclips_service(&deployment);

    let session = service.generate(session_id).await.map_err(map_err)?;

    Ok(ResponseJson(ApiResponse::success(session)))
}

pub async fn get_timeline(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TopiClipTimelineEntry>>, ApiError> {
    let service = topiclips_service(&deployment);

    let timeline = service.get_timeline_entry(session_id).await.map_err(map_err)?;

    Ok(ResponseJson(ApiResponse::success(timeline)))
}

// ============================================================================
// Gallery Handler
// ============================================================================

pub async fn get_gallery(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<GalleryQuery>,
) -> Result<ResponseJson<ApiResponse<TopiClipGalleryResponse>>, ApiError> {
    let service = topiclips_service(&deployment);

    let gallery = service.get_gallery(query.project_id).await.map_err(map_err)?;

    Ok(ResponseJson(ApiResponse::success(gallery)))
}

// ============================================================================
// Daily Schedule Handlers
// ============================================================================

pub async fn get_daily_info(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<GalleryQuery>,
) -> Result<ResponseJson<ApiResponse<DailyInfoResponse>>, ApiError> {
    let schedule =
        TopiClipDailySchedule::find_by_project(&deployment.db().pool, query.project_id).await?;

    let (current_streak, longest_streak, total_clips, last_generation) = schedule
        .as_ref()
        .map(|s| {
            (
                s.current_streak,
                s.longest_streak,
                s.total_clips_generated,
                s.last_generation_date.clone(),
            )
        })
        .unwrap_or((0, 0, 0, None));

    Ok(ResponseJson(ApiResponse::success(DailyInfoResponse {
        schedule,
        current_streak,
        longest_streak,
        total_clips,
        last_generation,
    })))
}

pub async fn create_daily_schedule(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<DailySchedulePayload>,
) -> Result<ResponseJson<ApiResponse<TopiClipDailySchedule>>, ApiError> {
    let schedule = TopiClipDailySchedule::create(
        &deployment.db().pool,
        &CreateTopiClipDailySchedule {
            project_id: payload.project_id,
            scheduled_time: payload.scheduled_time,
            timezone: payload.timezone,
            min_significance_threshold: payload.min_significance_threshold,
            force_daily: payload.force_daily,
        },
        Uuid::new_v4(),
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(schedule)))
}

pub async fn update_daily_schedule(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<UpdateSchedulePayload>,
) -> Result<ResponseJson<ApiResponse<TopiClipDailySchedule>>, ApiError> {
    let existing = TopiClipDailySchedule::find_by_project(&deployment.db().pool, project_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Schedule not found".into()))?;

    // For now, we'll do a simple update by recreating
    // In a real implementation, we'd have an update method
    let schedule = TopiClipDailySchedule::create(
        &deployment.db().pool,
        &CreateTopiClipDailySchedule {
            project_id,
            scheduled_time: payload.scheduled_time.unwrap_or(existing.scheduled_time),
            timezone: payload.timezone.or(existing.timezone),
            min_significance_threshold: payload
                .min_significance_threshold
                .or(existing.min_significance_threshold),
            force_daily: payload.force_daily.or(Some(existing.force_daily)),
        },
        existing.id, // Reuse existing ID for upsert behavior
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(schedule)))
}

pub async fn force_daily_generate(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TopiClipSession>>, ApiError> {
    let service = topiclips_service(&deployment);

    // Calculate period (last 24 hours)
    let end = chrono::Utc::now();
    let start = end - chrono::Duration::hours(24);

    let session = service
        .create_session(
            project_id,
            TopiClipTriggerType::Daily,
            Some(start.to_rfc3339()),
            Some(end.to_rfc3339()),
        )
        .await
        .map_err(map_err)?;

    let generated = service.generate(session.id).await.map_err(map_err)?;

    Ok(ResponseJson(ApiResponse::success(generated)))
}

// ============================================================================
// Config Handlers
// ============================================================================

pub async fn get_config(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<GalleryQuery>,
) -> Result<ResponseJson<ApiResponse<Option<TopiClipConfig>>>, ApiError> {
    let config =
        TopiClipConfig::find_by_project(&deployment.db().pool, query.project_id).await?;
    Ok(ResponseJson(ApiResponse::success(config)))
}

pub async fn upsert_config(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ConfigPayload>,
) -> Result<ResponseJson<ApiResponse<TopiClipConfig>>, ApiError> {
    let config = TopiClipConfig::upsert(
        &deployment.db().pool,
        &CreateTopiClipConfig {
            project_id: payload.project_id,
            default_style: payload.default_style,
            color_palette: payload.color_palette,
            visual_density: payload.visual_density,
            motion_intensity: payload.motion_intensity,
            llm_model: payload.llm_model,
            interpretation_temperature: payload.interpretation_temperature,
            output_resolution: payload.output_resolution,
            output_fps: payload.output_fps,
            output_format: payload.output_format,
        },
        Uuid::new_v4(),
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(config)))
}

// ============================================================================
// Symbol Library Handler
// ============================================================================

pub async fn list_symbols(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TopiClipSymbol>>>, ApiError> {
    let symbols = TopiClipSymbol::list_all(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(symbols)))
}

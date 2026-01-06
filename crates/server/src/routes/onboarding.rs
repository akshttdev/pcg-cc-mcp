//! Project Onboarding API Routes
//!
//! Manages the Airo-style project onboarding workflow with carousel segments.

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use db::models::project_onboarding::{
    CreateProjectOnboarding, OnboardingSegment, ProjectOnboarding,
    SegmentStatus, UpdateOnboardingSegment, UpdateProjectOnboarding,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Create onboarding router
pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/onboarding/project/{project_id}", get(get_project_onboarding))
        .route("/onboarding/project/{project_id}/start", post(start_project_onboarding))
        .route("/onboarding/{id}", put(update_onboarding))
        .route("/onboarding/{id}/segments", get(list_segments))
        .route("/onboarding/segment/{segment_id}", get(get_segment))
        .route("/onboarding/segment/{segment_id}", put(update_segment))
        .route("/onboarding/segment/{segment_id}/start", post(start_segment))
        .route("/onboarding/segment/{segment_id}/complete", post(complete_segment))
}

/// Response for onboarding with segments
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OnboardingWithSegments {
    pub onboarding: ProjectOnboarding,
    pub segments: Vec<OnboardingSegment>,
}

/// Get onboarding for a project
async fn get_project_onboarding(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Option<OnboardingWithSegments>>, ApiError> {
    let pool = &deployment.db().pool;
    let onboarding = ProjectOnboarding::find_by_project(pool, project_id).await?;

    match onboarding {
        Some(ob) => {
            let segments = OnboardingSegment::list_by_onboarding(pool, ob.id).await?;
            Ok(Json(Some(OnboardingWithSegments {
                onboarding: ob,
                segments,
            })))
        }
        None => Ok(Json(None)),
    }
}

/// Start onboarding for a project
async fn start_project_onboarding(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<StartOnboardingRequest>,
) -> Result<Json<OnboardingWithSegments>, ApiError> {
    let pool = &deployment.db().pool;

    // Check if onboarding already exists
    if let Some(existing) = ProjectOnboarding::find_by_project(pool, project_id).await? {
        let segments = OnboardingSegment::list_by_onboarding(pool, existing.id).await?;
        return Ok(Json(OnboardingWithSegments {
            onboarding: existing,
            segments,
        }));
    }

    // Create new onboarding with default segments
    let create = CreateProjectOnboarding {
        project_id,
        context_data: payload.context_data,
    };

    let onboarding = ProjectOnboarding::create_with_segments(pool, &create).await?;
    let segments = OnboardingSegment::list_by_onboarding(pool, onboarding.id).await?;

    Ok(Json(OnboardingWithSegments {
        onboarding,
        segments,
    }))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct StartOnboardingRequest {
    #[serde(default)]
    pub context_data: Option<String>,
}

/// Update onboarding status/phase
async fn update_onboarding(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProjectOnboarding>,
) -> Result<Json<ProjectOnboarding>, ApiError> {
    let pool = &deployment.db().pool;
    let onboarding = ProjectOnboarding::update(pool, id, &payload)
        .await?
        .ok_or_else(|| ApiError::NotFound("Onboarding not found".to_string()))?;

    Ok(Json(onboarding))
}

/// List all segments for an onboarding
async fn list_segments(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<OnboardingSegment>>, ApiError> {
    let pool = &deployment.db().pool;
    let segments = OnboardingSegment::list_by_onboarding(pool, id).await?;
    Ok(Json(segments))
}

/// Get a single segment
async fn get_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
) -> Result<Json<OnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let segment = OnboardingSegment::find_by_id(pool, segment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Segment not found".to_string()))?;

    Ok(Json(segment))
}

/// Update a segment
async fn update_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
    Json(payload): Json<UpdateOnboardingSegment>,
) -> Result<Json<OnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let segment = OnboardingSegment::update(pool, segment_id, &payload)
        .await?
        .ok_or_else(|| ApiError::NotFound("Segment not found".to_string()))?;

    Ok(Json(segment))
}

/// Start a segment (set to in_progress)
async fn start_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
) -> Result<Json<OnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let update = UpdateOnboardingSegment {
        status: Some(SegmentStatus::InProgress),
        recommendations: None,
        user_decisions: None,
        assigned_agent_id: None,
        assigned_agent_name: None,
    };

    let segment = OnboardingSegment::update(pool, segment_id, &update)
        .await?
        .ok_or_else(|| ApiError::NotFound("Segment not found".to_string()))?;

    Ok(Json(segment))
}

/// Complete a segment
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CompleteSegmentRequest {
    #[serde(default)]
    pub user_decisions: Option<String>,
    #[serde(default)]
    pub skip: bool,
}

async fn complete_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
    Json(payload): Json<CompleteSegmentRequest>,
) -> Result<Json<OnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let status = if payload.skip {
        SegmentStatus::Skipped
    } else {
        SegmentStatus::Completed
    };

    let update = UpdateOnboardingSegment {
        status: Some(status),
        recommendations: None,
        user_decisions: payload.user_decisions,
        assigned_agent_id: None,
        assigned_agent_name: None,
    };

    let segment = OnboardingSegment::update(pool, segment_id, &update)
        .await?
        .ok_or_else(|| ApiError::NotFound("Segment not found".to_string()))?;

    Ok(Json(segment))
}

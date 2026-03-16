//! Organization Onboarding API Routes
//!
//! Manages the Airo-style organization onboarding workflow with carousel segments.
//! This is the org-scoped equivalent of the project onboarding system.

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use db::models::org_onboarding::{
    CreateOrgOnboarding, OrgOnboarding, OrgOnboardingSegment, UpdateOrgOnboarding,
};
use db::models::project_onboarding::{SegmentStatus, UpdateOnboardingSegment};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Create org onboarding router
pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/onboarding/organization/{org_id}",
            get(get_org_onboarding),
        )
        .route(
            "/onboarding/organization/{org_id}/start",
            post(start_org_onboarding),
        )
        .route("/onboarding/org/{id}", put(update_org_onboarding))
        .route("/onboarding/org/{id}/segments", get(list_org_segments))
        .route(
            "/onboarding/org-segment/{segment_id}",
            get(get_org_segment).put(update_org_segment),
        )
        .route(
            "/onboarding/org-segment/{segment_id}/start",
            post(start_org_segment),
        )
        .route(
            "/onboarding/org-segment/{segment_id}/complete",
            post(complete_org_segment),
        )
}

/// Response for org onboarding with segments
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OrgOnboardingWithSegments {
    pub onboarding: OrgOnboarding,
    pub segments: Vec<OrgOnboardingSegment>,
}

/// Get onboarding for an organization
async fn get_org_onboarding(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<Option<OrgOnboardingWithSegments>>, ApiError> {
    let pool = &deployment.db().pool;
    let onboarding = OrgOnboarding::find_by_organization(pool, org_id).await?;

    match onboarding {
        Some(ob) => {
            let segments = OrgOnboardingSegment::list_by_onboarding(pool, ob.id).await?;
            Ok(Json(Some(OrgOnboardingWithSegments {
                onboarding: ob,
                segments,
            })))
        }
        None => Ok(Json(None)),
    }
}

/// Start onboarding for an organization
async fn start_org_onboarding(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
    Json(payload): Json<StartOrgOnboardingRequest>,
) -> Result<Json<OrgOnboardingWithSegments>, ApiError> {
    let pool = &deployment.db().pool;

    // Check if onboarding already exists
    if let Some(existing) = OrgOnboarding::find_by_organization(pool, org_id).await? {
        let segments = OrgOnboardingSegment::list_by_onboarding(pool, existing.id).await?;
        return Ok(Json(OrgOnboardingWithSegments {
            onboarding: existing,
            segments,
        }));
    }

    // Create new onboarding with default segments
    let create = CreateOrgOnboarding {
        organization_id: org_id,
        context_data: payload.context_data,
    };

    let onboarding = OrgOnboarding::create_with_segments(pool, &create).await?;
    let segments = OrgOnboardingSegment::list_by_onboarding(pool, onboarding.id).await?;

    Ok(Json(OrgOnboardingWithSegments {
        onboarding,
        segments,
    }))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct StartOrgOnboardingRequest {
    #[serde(default)]
    pub context_data: Option<String>,
}

/// Update org onboarding status/phase
async fn update_org_onboarding(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateOrgOnboarding>,
) -> Result<Json<OrgOnboarding>, ApiError> {
    let pool = &deployment.db().pool;
    let onboarding = OrgOnboarding::update(pool, id, &payload)
        .await?
        .ok_or_else(|| ApiError::NotFound("Org onboarding not found".to_string()))?;

    Ok(Json(onboarding))
}

/// List all segments for an org onboarding
async fn list_org_segments(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<OrgOnboardingSegment>>, ApiError> {
    let pool = &deployment.db().pool;
    let segments = OrgOnboardingSegment::list_by_onboarding(pool, id).await?;
    Ok(Json(segments))
}

/// Get a single org segment
async fn get_org_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
) -> Result<Json<OrgOnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let segment = OrgOnboardingSegment::find_by_id(pool, segment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Org segment not found".to_string()))?;

    Ok(Json(segment))
}

/// Update an org segment
async fn update_org_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
    Json(payload): Json<UpdateOnboardingSegment>,
) -> Result<Json<OrgOnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let segment = OrgOnboardingSegment::update(pool, segment_id, &payload)
        .await?
        .ok_or_else(|| ApiError::NotFound("Org segment not found".to_string()))?;

    Ok(Json(segment))
}

/// Start an org segment (set to in_progress)
async fn start_org_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
) -> Result<Json<OrgOnboardingSegment>, ApiError> {
    let pool = &deployment.db().pool;
    let update = UpdateOnboardingSegment {
        status: Some(SegmentStatus::InProgress),
        recommendations: None,
        user_decisions: None,
        assigned_agent_id: None,
        assigned_agent_name: None,
    };

    let segment = OrgOnboardingSegment::update(pool, segment_id, &update)
        .await?
        .ok_or_else(|| ApiError::NotFound("Org segment not found".to_string()))?;

    Ok(Json(segment))
}

/// Complete an org segment
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CompleteOrgSegmentRequest {
    #[serde(default)]
    pub user_decisions: Option<String>,
    #[serde(default)]
    pub skip: bool,
}

async fn complete_org_segment(
    State(deployment): State<DeploymentImpl>,
    Path(segment_id): Path<Uuid>,
    Json(payload): Json<CompleteOrgSegmentRequest>,
) -> Result<Json<OrgOnboardingSegment>, ApiError> {
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

    let segment = OrgOnboardingSegment::update(pool, segment_id, &update)
        .await?
        .ok_or_else(|| ApiError::NotFound("Org segment not found".to_string()))?;

    Ok(Json(segment))
}

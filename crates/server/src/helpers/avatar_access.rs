//! Avatar Profile + Video Job access control helpers.
//!
//! Modeled on `require_deal_org_access` in `routes/crm_deals.rs`. Loads the
//! resource by id, admin-bypasses, otherwise asserts the caller is a member
//! of the resource's organization. Video jobs derive their org via the
//! linked avatar profile (jobs don't carry org_id directly).

use db::models::{avatar_profile::AvatarProfile, video_job::VideoJob};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{error::ApiError, middleware::access_control::AccessContext};

/// Load an avatar and verify the caller can access it.
///
/// - Admins bypass the membership check.
/// - Org-scoped avatars require the caller to be a member of `organization_id`.
/// - Un-scoped avatars (org_id = NULL, e.g. seed/system avatars) are accessible
///   to any authenticated user — only admins can create those via direct DB access.
///
/// Errors:
/// - `ApiError::NotFound` if the avatar doesn't exist.
/// - `ApiError::Forbidden` if the caller is not a member of the avatar's org.
pub async fn require_avatar_org_access(
    access: &AccessContext,
    pool: &SqlitePool,
    avatar_id: Uuid,
) -> Result<AvatarProfile, ApiError> {
    let avatar = AvatarProfile::find(pool, avatar_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", avatar_id)))?;

    if access.is_admin {
        return Ok(avatar);
    }

    if let Some(org_id) = avatar.organization_id {
        access
            .require_org_membership(pool, &org_id.to_string())
            .await?;
    }

    Ok(avatar)
}

/// Verify the caller can access a video job. Jobs derive their org through
/// the linked avatar profile, so this loads the job, then delegates to
/// `require_avatar_org_access`.
///
/// Errors mirror `require_avatar_org_access`.
pub async fn require_video_job_org_access(
    access: &AccessContext,
    pool: &SqlitePool,
    job_id: Uuid,
) -> Result<VideoJob, ApiError> {
    let job = VideoJob::find(pool, job_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("video job {}", job_id)))?;

    if access.is_admin {
        return Ok(job);
    }

    require_avatar_org_access(access, pool, job.avatar_profile_id).await?;
    Ok(job)
}

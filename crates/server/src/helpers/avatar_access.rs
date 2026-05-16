//! Avatar Profile access control helper.
//!
//! Modeled on `require_deal_org_access` in `routes/crm_deals.rs`. Loads
//! the avatar by id, admin-bypasses, otherwise asserts the caller is a
//! member of the avatar's organization.

use db::models::avatar_profile::AvatarProfile;
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

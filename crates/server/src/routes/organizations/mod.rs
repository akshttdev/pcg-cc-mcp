use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
};
use db::{
    db_uuid::DbUuid,
    models::{
        brand_intake_token::BrandIntakeToken,
        company::Company,
        contact_association::{ContactOrgLink, UpsertContactOrgLink},
        crm_contact::CrmContact,
        org_brand_profile::{OrgBrandProfile, UpsertOrgBrandProfile},
        user::{CreateOrganization, Organization, OrganizationMember, UpdateOrganization},
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

pub mod brand;
pub mod intake;
pub mod knowledge;
pub mod members;

// Re-export public items from sub-modules so callers don't break
pub use brand::*;
pub use intake::*;
pub use knowledge::*;
pub use members::*;

// ─────────────────────────────────────────────────────────────────────────────
// CRUD handlers (create, get, update, delete, list organizations)
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/organizations — list user's orgs
pub async fn list_organizations(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Organization>>>, ApiError> {
    let orgs = if access_context.is_admin {
        Organization::find_all(&deployment.db().pool).await?
    } else {
        Organization::find_by_user(&deployment.db().pool, access_context.user_id.as_str()).await?
    };
    Ok(Json(ApiResponse::success(orgs)))
}

/// GET /api/organizations/:id — org details
pub async fn get_organization(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Organization>>, ApiError> {
    let org = Organization::find_by_id(&deployment.db().pool, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Organization not found".into()))?;

    // Check user has access (is admin or org member)
    if !access_context.is_admin {
        let role = Organization::get_user_role(
            &deployment.db().pool,
            &id,
            access_context.user_id.as_str(),
        )
        .await?;
        if role.is_none() {
            return Err(ApiError::Forbidden(
                "Not a member of this organization".into(),
            ));
        }
    }

    Ok(Json(ApiResponse::success(org)))
}

/// POST /api/organizations — create org
pub async fn create_organization(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateOrganization>,
) -> Result<Json<ApiResponse<Organization>>, ApiError> {
    let id = Uuid::new_v4().to_string();
    let org = Organization::create(
        &deployment.db().pool,
        &id,
        access_context.user_id.as_str(),
        &data,
    )
    .await?;

    // Add creator as admin member
    Organization::add_member(
        &deployment.db().pool,
        &org.id,
        access_context.user_id.as_str(),
        "admin",
    )
    .await?;

    Ok(Json(ApiResponse::success(org)))
}

/// PUT /api/organizations/:id — update org
pub async fn update_organization(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<UpdateOrganization>,
) -> Result<Json<ApiResponse<Organization>>, ApiError> {
    // Only org admins or system admins can update
    if !access_context.is_admin {
        let role = Organization::get_user_role(
            &deployment.db().pool,
            &id,
            access_context.user_id.as_str(),
        )
        .await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => {
                return Err(ApiError::Forbidden(
                    "Only org admins can update organizations".into(),
                ));
            }
        }
    }

    let org = Organization::update(&deployment.db().pool, &id, &data).await?;
    Ok(Json(ApiResponse::success(org)))
}

/// PATCH /api/organizations/:id/activate — reactivate
pub async fn activate_organization(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        return Err(ApiError::Forbidden(
            "Only system admins can activate organizations".into(),
        ));
    }
    Organization::activate(&deployment.db().pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PATCH /api/organizations/:id/deactivate — deactivate (hide from sidebar)
pub async fn deactivate_organization(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        return Err(ApiError::Forbidden(
            "Only system admins can deactivate organizations".into(),
        ));
    }
    Organization::deactivate(&deployment.db().pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/organizations/:id — soft-delete (deactivate)
pub async fn delete_organization(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(
            &deployment.db().pool,
            &id,
            access_context.user_id.as_str(),
        )
        .await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => {
                return Err(ApiError::Forbidden(
                    "Only org admins can delete organizations".into(),
                ));
            }
        }
    }

    Organization::deactivate(&deployment.db().pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn uuid_from_bytes(bytes: &[u8]) -> Option<Uuid> {
    Uuid::from_slice(bytes).ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// Router composition
// ─────────────────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/organizations",
            get(list_organizations).post(create_organization),
        )
        .route(
            "/organizations/{id}",
            get(get_organization)
                .put(update_organization)
                .delete(delete_organization),
        )
        .route("/organizations/{id}/activate", patch(activate_organization))
        .route(
            "/organizations/{id}/deactivate",
            patch(deactivate_organization),
        )
        .route(
            "/organizations/{id}/members",
            get(members::list_members).post(members::add_member),
        )
        .route(
            "/organizations/{id}/members/{uid}/role",
            put(members::change_member_role),
        )
        .route(
            "/organizations/{id}/members/{uid}",
            delete(members::remove_member),
        )
        .route(
            "/organizations/{id}/generate-invite",
            post(members::generate_invite),
        )
        .route(
            "/organizations/{id}/contacts",
            get(members::get_org_persons),
        )
        .route(
            "/organizations/{id}/data-sources",
            get(members::list_org_data_sources),
        )
        .route("/data-sources", get(members::list_data_sources))
        .route(
            "/organizations/{id}/members/{uid}/assign",
            post(members::assign_member),
        )
        .route(
            "/organizations/{id}/members/{uid}/watch",
            post(members::watch_task_for_member),
        )
        .route(
            "/organizations/{id}/members/{uid}/assignments",
            get(members::get_member_assignments),
        )
        .route(
            "/organizations/{id}/members/{uid}/assignments/project/{pid}",
            delete(members::unassign_project),
        )
        .route(
            "/organizations/{id}/members/{uid}/assignments/client/{cid}",
            delete(members::unassign_client),
        )
        .route(
            "/organizations/{id}/contact-links",
            get(members::list_org_person_contacts).post(members::add_org_person_contact),
        )
        .route(
            "/organizations/{id}/companies",
            get(members::list_org_companies),
        )
        .route(
            "/organizations/{id}/brand-profile",
            get(brand::get_org_brand_profile).put(brand::upsert_org_brand_profile),
        )
        .route(
            "/organizations/{id}/brand-research",
            post(brand::trigger_brand_research),
        )
        .route(
            "/organizations/{id}/brand-research/status",
            get(brand::get_brand_research_status),
        )
        .route(
            "/organizations/{id}/seed-brand-project",
            post(brand::seed_brand_project),
        )
        .route(
            "/organizations/{id}/knowledge",
            get(knowledge::get_org_knowledge),
        )
        .route(
            "/organizations/{id}/intake-token",
            post(intake::generate_intake_token),
        )
}

/// Public router — no auth required (intake form submissions)
pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route(
        "/intake/{token}",
        get(intake::get_intake_context).post(intake::submit_intake),
    )
}

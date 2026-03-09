use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get, patch, post, put},
};
use db::models::user::{
    CreateOrganization, Organization, OrganizationMember, UpdateOrganization,
};
use db::models::company::Company;
use db::models::person_association::{PersonOrgContact, UpsertPersonOrgContact};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::AccessContext,
};

/// GET /api/organizations — list user's orgs
pub async fn list_organizations(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Organization>>>, ApiError> {
    let orgs = if access_context.is_admin {
        Organization::find_all(&deployment.db().pool).await?
    } else {
        Organization::find_by_user(&deployment.db().pool, access_context.user_id).await?
    };
    Ok(Json(ApiResponse::success(orgs)))
}

/// GET /api/organizations/:id — org details
pub async fn get_organization(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Organization>>, ApiError> {
    let org = Organization::find_by_id(&deployment.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Organization not found".into()))?;

    // Check user has access (is admin or org member)
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
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
    let id = Uuid::new_v4();
    let org = Organization::create(
        &deployment.db().pool,
        id,
        access_context.user_id,
        &data,
    )
    .await?;

    // Add creator as admin member
    Organization::add_member(
        &deployment.db().pool,
        org.id,
        access_context.user_id,
        "admin",
    )
    .await?;

    Ok(Json(ApiResponse::success(org)))
}

/// PUT /api/organizations/:id — update org
pub async fn update_organization(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<UpdateOrganization>,
) -> Result<Json<ApiResponse<Organization>>, ApiError> {
    // Only org admins or system admins can update
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can update organizations".into())),
        }
    }

    let org = Organization::update(&deployment.db().pool, id, &data).await?;
    Ok(Json(ApiResponse::success(org)))
}

/// GET /api/organizations/:id/members — list members
pub async fn list_members(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<OrganizationMember>>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
        }
    }

    let members = Organization::get_members(&deployment.db().pool, id).await?;
    Ok(Json(ApiResponse::success(members)))
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: Option<String>,
}

/// POST /api/organizations/:id/members — add member
pub async fn add_member(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<AddMemberRequest>,
) -> Result<Json<ApiResponse<OrganizationMember>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can add members".into())),
        }
    }

    let role = data.role.as_deref().unwrap_or("member");
    let member = Organization::add_member(&deployment.db().pool, id, data.user_id, role).await?;
    Ok(Json(ApiResponse::success(member)))
}

#[derive(Debug, Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

/// PUT /api/organizations/:id/members/:uid/role — change role
pub async fn change_member_role(
    Path((id, uid)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<ChangeRoleRequest>,
) -> Result<Json<ApiResponse<OrganizationMember>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can change roles".into())),
        }
    }

    // Remove and re-add with new role
    Organization::remove_member(&deployment.db().pool, id, uid).await?;
    let member = Organization::add_member(&deployment.db().pool, id, uid, &data.role).await?;
    Ok(Json(ApiResponse::success(member)))
}

/// DELETE /api/organizations/:id/members/:uid — remove member
pub async fn remove_member(
    Path((id, uid)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can remove members".into())),
        }
    }

    Organization::remove_member(&deployment.db().pool, id, uid).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PATCH /api/organizations/:id/activate — reactivate
pub async fn activate_organization(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        return Err(ApiError::Forbidden("Only system admins can activate organizations".into()));
    }
    Organization::activate(&deployment.db().pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PATCH /api/organizations/:id/deactivate — deactivate (hide from sidebar)
pub async fn deactivate_organization(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        return Err(ApiError::Forbidden("Only system admins can deactivate organizations".into()));
    }
    Organization::deactivate(&deployment.db().pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/organizations/:id — soft-delete (deactivate)
pub async fn delete_organization(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, id, access_context.user_id).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can delete organizations".into())),
        }
    }

    Organization::deactivate(&deployment.db().pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /organizations/:id/person-contacts — junction table entries (for context badges)
async fn list_org_person_contacts(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<PersonOrgContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let contacts = PersonOrgContact::list_for_org(pool, id).await?;
    Ok(Json(ApiResponse::success(contacts)))
}

#[derive(Debug, serde::Deserialize)]
struct AddOrgPersonContactBody {
    person_id: Uuid,
    context: Option<String>,
    notes: Option<String>,
}

/// POST /organizations/:id/person-contacts — link an existing person to this org
async fn add_org_person_contact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<AddOrgPersonContactBody>,
) -> Result<Json<ApiResponse<PersonOrgContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let upsert_data = UpsertPersonOrgContact {
        organization_id: id,
        context: data.context,
        notes: data.notes,
    };
    let contact = PersonOrgContact::upsert(pool, data.person_id, upsert_data).await?;
    Ok(Json(ApiResponse::success(contact)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/organizations", get(list_organizations).post(create_organization))
        .route("/organizations/{id}", get(get_organization).put(update_organization).delete(delete_organization))
        .route("/organizations/{id}/activate", patch(activate_organization))
        .route("/organizations/{id}/deactivate", patch(deactivate_organization))
        .route(
            "/organizations/{id}/members",
            get(list_members).post(add_member),
        )
        .route(
            "/organizations/{id}/members/{uid}/role",
            put(change_member_role),
        )
        .route(
            "/organizations/{id}/members/{uid}",
            delete(remove_member),
        )
        .route(
            "/organizations/{id}/person-contacts",
            get(list_org_person_contacts).post(add_org_person_contact),
        )
        .route("/organizations/{id}/companies", get(list_org_companies))
}

/// GET /organizations/:id/companies — list companies created by this org
async fn list_org_companies(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let companies = Company::list(pool, Some(id), None, Some(200)).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(companies))
}

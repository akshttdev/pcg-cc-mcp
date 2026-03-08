use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
};
use serde_json::Value;
use db::models::person::Person;
use db::models::user::{
    CreateOrganization, Organization, OrganizationMember, UpdateOrganization,
};
use db::models::person_association::{PersonOrgContact, UpsertPersonOrgContact};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
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

/// GET /api/organizations/:id/members — list members with user details
pub async fn list_members(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    if !access_context.is_admin {
        let role = Organization::get_user_role(pool, id, access_context.user_id).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
        }
    }

    #[derive(sqlx::FromRow)]
    struct MemberRow {
        #[sqlx(try_from = "Vec<u8>")]
        id: Uuid,
        #[sqlx(try_from = "Vec<u8>")]
        user_id: Uuid,
        role: String,
        joined_at: String,
        username: Option<String>,
        full_name: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
    }

    let rows = sqlx::query_as::<_, MemberRow>(
        r#"SELECT om.id, om.user_id, om.role, om.joined_at,
                  u.username, u.full_name, u.email, u.avatar_url
           FROM organization_members om
           LEFT JOIN users u ON u.id = om.user_id
           WHERE om.organization_id = ?
           ORDER BY om.joined_at ASC"#,
    )
    .bind(id.as_bytes().as_slice())
    .fetch_all(pool)
    .await?;

    let result: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "user_id": r.user_id.to_string(),
                "role": r.role,
                "joined_at": r.joined_at,
                "user": {
                    "username": r.username,
                    "full_name": r.full_name,
                    "email": r.email,
                    "avatar_url": r.avatar_url,
                }
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(result)))
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

// ---------------------------------------------------------------------------
// Invite link
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GenerateInviteBody {
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerateInviteResponse {
    pub invite_url: String,
}

/// POST /api/organizations/:id/generate-invite  (admin only)
pub async fn generate_invite(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<GenerateInviteBody>,
) -> Result<Json<ApiResponse<GenerateInviteResponse>>, ApiError> {
    if !access_context.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let pool = &deployment.db().pool;
    let token = Uuid::new_v4().to_string();

    sqlx::query(
        "UPDATE organizations SET invite_token = ?, pending_owner_email = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&token)
    .bind(&body.email)
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await?;

    let base_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".to_string());
    let invite_url = format!("{}/signup?invite={}", base_url, token);

    Ok(Json(ApiResponse::success(GenerateInviteResponse { invite_url })))
}

// ---------------------------------------------------------------------------
// Invite info (public)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct InviteInfoQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct InviteInfoResponse {
    pub org_name: String,
    pub pending_owner_email: Option<String>,
}

/// GET /api/auth/invite-info?token=TOKEN  (public, no auth)
pub async fn invite_info(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<InviteInfoQuery>,
) -> Result<Json<ApiResponse<InviteInfoResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    #[derive(sqlx::FromRow)]
    struct OrgRow {
        name: String,
        pending_owner_email: Option<String>,
    }

    let row: Option<OrgRow> = sqlx::query_as(
        "SELECT name, pending_owner_email FROM organizations WHERE invite_token = ? AND is_active = 1"
    )
    .bind(&params.token)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Invite token not found".into()))?;

    Ok(Json(ApiResponse::success(InviteInfoResponse {
        org_name: row.name,
        pending_owner_email: row.pending_owner_email,
    })))
}

/// GET /api/organizations/:id/persons — persons directly in org + bridged via crm_contacts
pub async fn get_org_persons(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Person>>>, ApiError> {
    let pool = &deployment.db().pool;

    if !access_context.is_admin {
        let role = Organization::get_user_role(pool, id, access_context.user_id).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
        }
    }

    // Return persons directly in org UNION persons bridged via crm_contacts
    let persons = sqlx::query_as::<_, Person>(
        "SELECT * FROM persons WHERE organization_id = ?
         UNION
         SELECT p.* FROM persons p
         INNER JOIN crm_contacts cc ON cc.person_id = p.id
         WHERE cc.organization_id = ? AND (p.organization_id IS NULL OR p.organization_id != ?)
         ORDER BY full_name ASC",
    )
    .bind(id.as_bytes().as_slice())
    .bind(id.as_bytes().as_slice())
    .bind(id.as_bytes().as_slice())
    .fetch_all(pool)
    .await?;

    Ok(Json(ApiResponse::success(persons)))
}

/// GET /api/data-sources?organization_id=<uuid>
pub async fn list_data_sources(
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows: Vec<Value> = if let Some(org_id) = params.get("organization_id") {
        let org_uuid = Uuid::parse_str(org_id)
            .map_err(|_| ApiError::BadRequest("Invalid organization_id".into()))?;
        let org_bytes = org_uuid.as_bytes().as_slice().to_vec();
        sqlx::query_scalar::<_, String>(
            r#"SELECT json_object(
                'id', lower(hex(id)),
                'organization_id', lower(hex(organization_id)),
                'project_id', lower(hex(project_id)),
                'created_by', lower(hex(created_by)),
                'title', title,
                'description', description,
                'data_type', data_type,
                'file_type', file_type,
                'file_name', file_name,
                'file_path', file_path,
                'file_size_bytes', file_size_bytes,
                'metadata', metadata,
                'status', status,
                'source_type', source_type,
                'created_at', created_at,
                'updated_at', updated_at
            ) FROM data_sources
            WHERE organization_id = ? AND archived_at IS NULL
            ORDER BY created_at DESC"#,
        )
        .bind(org_bytes)
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|s| serde_json::from_str(&s).ok())
        .collect()
    } else {
        vec![]
    };
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /api/organizations/:id/data-sources
pub async fn list_org_data_sources(
    Path(id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_bytes = id.as_bytes().as_slice().to_vec();
    let rows: Vec<Value> = sqlx::query_scalar::<_, String>(
        r#"SELECT json_object(
            'id', lower(hex(id)),
            'organization_id', lower(hex(organization_id)),
            'project_id', lower(hex(project_id)),
            'created_by', lower(hex(created_by)),
            'title', title,
            'description', description,
            'data_type', data_type,
            'file_type', file_type,
            'file_name', file_name,
            'file_path', file_path,
            'file_size_bytes', file_size_bytes,
            'metadata', metadata,
            'status', status,
            'source_type', source_type,
            'created_at', created_at,
            'updated_at', updated_at
        ) FROM data_sources
        WHERE organization_id = ? AND archived_at IS NULL
        ORDER BY created_at DESC"#,
    )
    .bind(org_bytes)
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter_map(|s| serde_json::from_str(&s).ok())
    .collect();
    Ok(Json(ApiResponse::success(rows)))
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
        .route("/organizations/{id}/generate-invite", post(generate_invite))
        .route("/organizations/{id}/persons", get(get_org_persons))
        .route("/organizations/{id}/data-sources", get(list_org_data_sources))
        .route("/data-sources", get(list_data_sources))
        .route(
            "/organizations/{id}/person-contacts",
            get(list_org_person_contacts).post(add_org_person_contact),
        )
}

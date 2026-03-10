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
use db::models::company::Company;
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

// ─────────────────────────────────────────────────────────────────────────────
// Member assignment endpoints
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AssignMemberRequest {
    /// "project", "client", or "task"
    #[serde(rename = "type")]
    pub assign_type: String,
    pub target_id: Uuid,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WatchTaskRequest {
    pub task_id: Uuid,
}

/// Helper: verify caller is org admin or platform admin
async fn require_org_admin_access(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: Uuid,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id).await?;
    match role.as_deref() {
        Some("admin") => Ok(()),
        _ => Err(ApiError::Forbidden("Only org admins can manage member assignments".into())),
    }
}

/// Helper: verify target user is an org member
async fn require_is_org_member(
    pool: &sqlx::SqlitePool,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let role = Organization::get_user_role(pool, org_id, user_id).await?;
    if role.is_none() {
        return Err(ApiError::BadRequest("User is not a member of this organization".into()));
    }
    Ok(())
}

/// POST /api/organizations/:id/members/:uid/assign — assign member to project/client/task
pub async fn assign_member(
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<AssignMemberRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_admin_access(pool, &access_context, org_id).await?;
    require_is_org_member(pool, org_id, user_id).await?;

    match data.assign_type.as_str() {
        "project" => {
            // Verify project belongs to this org
            let org_id_bytes = org_id.as_bytes().to_vec();
            let target_bytes = data.target_id.as_bytes().to_vec();
            let belongs: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM projects WHERE id = ? AND organization_id = ? AND deleted_at IS NULL LIMIT 1"
            )
            .bind(&target_bytes)
            .bind(&org_id_bytes)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

            if belongs.is_none() {
                return Err(ApiError::BadRequest("Project not found in this organization".into()));
            }

            let role = data.role.as_deref().unwrap_or("editor");
            let member_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by)
                   VALUES (?, ?, ?, ?, ?)"#,
            )
            .bind(member_id.as_bytes().to_vec())
            .bind(&target_bytes)
            .bind(user_id.as_bytes().to_vec())
            .bind(role)
            .bind(access_context.user_id.as_bytes().to_vec())
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to assign to project: {}", e)))?;

            Ok(Json(ApiResponse::success(serde_json::json!({
                "assigned": "project",
                "project_id": data.target_id.to_string(),
                "role": role,
            }))))
        }
        "client" => {
            // Verify client belongs to this org
            let org_id_bytes = org_id.as_bytes().to_vec();
            let target_bytes = data.target_id.as_bytes().to_vec();
            let belongs: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM clients WHERE id = ? AND organization_id = ? AND deleted_at IS NULL LIMIT 1"
            )
            .bind(&target_bytes)
            .bind(&org_id_bytes)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

            if belongs.is_none() {
                return Err(ApiError::BadRequest("Client not found in this organization".into()));
            }

            let role = data.role.as_deref().unwrap_or("viewer");
            let member_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT OR IGNORE INTO client_members (id, client_id, user_id, role, granted_by)
                   VALUES (?, ?, ?, ?, ?)"#,
            )
            .bind(member_id.as_bytes().to_vec())
            .bind(&target_bytes)
            .bind(user_id.as_bytes().to_vec())
            .bind(role)
            .bind(access_context.user_id.as_bytes().to_vec())
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to assign to client: {}", e)))?;

            Ok(Json(ApiResponse::success(serde_json::json!({
                "assigned": "client",
                "client_id": data.target_id.to_string(),
                "role": role,
            }))))
        }
        "task" => {
            // Verify task belongs to a project in this org
            let org_id_bytes = org_id.as_bytes().to_vec();
            let belongs: Option<i64> = sqlx::query_scalar(
                r#"SELECT 1 FROM tasks t
                   JOIN projects p ON p.id = t.project_id
                   WHERE t.id = ? AND p.organization_id = ? AND t.deleted_at IS NULL LIMIT 1"#
            )
            .bind(data.target_id.to_string())
            .bind(&org_id_bytes)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

            if belongs.is_none() {
                return Err(ApiError::BadRequest("Task not found in this organization".into()));
            }

            sqlx::query(
                "UPDATE tasks SET assignee_id = ?, assignee_type = 'user' WHERE id = ?"
            )
            .bind(user_id.to_string())
            .bind(data.target_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to assign task: {}", e)))?;

            Ok(Json(ApiResponse::success(serde_json::json!({
                "assigned": "task",
                "task_id": data.target_id.to_string(),
            }))))
        }
        _ => Err(ApiError::BadRequest("Invalid assignment type. Must be 'project', 'client', or 'task'".into())),
    }
}

/// POST /api/organizations/:id/members/:uid/watch — add user as task watcher
pub async fn watch_task_for_member(
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<WatchTaskRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_admin_access(pool, &access_context, org_id).await?;
    require_is_org_member(pool, org_id, user_id).await?;

    // Use the Task::add_watcher method from Phase 2
    db::models::task::Task::add_watcher(pool, data.task_id, &user_id.to_string()).await
        .map_err(|e| ApiError::InternalError(format!("Failed to add watcher: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "watching": "task",
        "task_id": data.task_id.to_string(),
        "user_id": user_id.to_string(),
    }))))
}

/// GET /api/organizations/:id/members/:uid/assignments — get all assignments for a member
pub async fn get_member_assignments(
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;

    // Any org member can view assignments; admins can view anyone's
    if !access_context.is_admin && access_context.user_id != user_id {
        require_org_admin_access(pool, &access_context, org_id).await?;
    }
    require_is_org_member(pool, org_id, user_id).await?;

    let org_id_bytes = org_id.as_bytes().to_vec();
    let user_id_bytes = user_id.as_bytes().to_vec();

    // Projects assigned via project_members within this org
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct ProjectAssignment {
        project_id: Vec<u8>,
        project_name: String,
        role: String,
    }
    let projects: Vec<serde_json::Value> = sqlx::query_as::<_, ProjectAssignment>(
        r#"SELECT pm.project_id, p.name as project_name, pm.role
           FROM project_members pm
           JOIN projects p ON p.id = pm.project_id
           WHERE pm.user_id = ? AND p.organization_id = ? AND p.deleted_at IS NULL
           ORDER BY p.name"#,
    )
    .bind(&user_id_bytes)
    .bind(&org_id_bytes)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter_map(|r| {
        let pid = uuid_from_bytes(&r.project_id)?;
        Some(serde_json::json!({
            "project_id": pid.to_string(),
            "project_name": r.project_name,
            "role": r.role,
        }))
    })
    .collect();

    // Clients assigned via client_members within this org
    #[derive(sqlx::FromRow)]
    struct ClientAssignment {
        client_id: Vec<u8>,
        client_name: String,
        role: String,
    }
    let clients: Vec<serde_json::Value> = sqlx::query_as::<_, ClientAssignment>(
        r#"SELECT cm.client_id, c.name as client_name, cm.role
           FROM client_members cm
           JOIN clients c ON c.id = cm.client_id
           WHERE cm.user_id = ? AND c.organization_id = ? AND c.deleted_at IS NULL
           ORDER BY c.name"#,
    )
    .bind(&user_id_bytes)
    .bind(&org_id_bytes)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter_map(|r| {
        let cid = uuid_from_bytes(&r.client_id)?;
        Some(serde_json::json!({
            "client_id": cid.to_string(),
            "client_name": r.client_name,
            "role": r.role,
        }))
    })
    .collect();

    // Tasks assigned to this user within this org
    #[derive(sqlx::FromRow)]
    struct TaskAssignment {
        id: String,
        title: String,
        status: String,
        project_name: String,
    }
    let tasks: Vec<serde_json::Value> = sqlx::query_as::<_, TaskAssignment>(
        r#"SELECT t.id, t.title, t.status, p.name as project_name
           FROM tasks t
           JOIN projects p ON p.id = t.project_id
           WHERE t.assignee_id = ? AND p.organization_id = ? AND t.deleted_at IS NULL
           ORDER BY t.updated_at DESC"#,
    )
    .bind(user_id.to_string())
    .bind(&org_id_bytes)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| serde_json::json!({
        "task_id": r.id,
        "title": r.title,
        "status": r.status,
        "project_name": r.project_name,
        "type": "assignee",
    }))
    .collect();

    // Tasks watched by this user within this org
    let watched: Vec<serde_json::Value> = sqlx::query_as::<_, TaskAssignment>(
        r#"SELECT t.id, t.title, t.status, p.name as project_name
           FROM tasks t
           JOIN projects p ON p.id = t.project_id
           WHERE t.collaborators LIKE ?
             AND p.organization_id = ? AND t.deleted_at IS NULL
           ORDER BY t.updated_at DESC"#,
    )
    .bind(format!("%\"user_id\":\"{}\",%\"actor_type\":\"watcher\"%", user_id))
    .bind(&org_id_bytes)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| serde_json::json!({
        "task_id": r.id,
        "title": r.title,
        "status": r.status,
        "project_name": r.project_name,
        "type": "watcher",
    }))
    .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "projects": projects,
        "clients": clients,
        "tasks": tasks,
        "watched_tasks": watched,
    }))))
}

/// DELETE /api/organizations/:id/members/:uid/assignments/project/:pid
pub async fn unassign_project(
    Path((org_id, user_id, project_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_admin_access(pool, &access_context, org_id).await?;

    sqlx::query("DELETE FROM project_members WHERE project_id = ? AND user_id = ?")
        .bind(project_id.as_bytes().to_vec())
        .bind(user_id.as_bytes().to_vec())
        .execute(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/organizations/:id/members/:uid/assignments/client/:cid
pub async fn unassign_client(
    Path((org_id, user_id, client_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    require_org_admin_access(pool, &access_context, org_id).await?;

    sqlx::query("DELETE FROM client_members WHERE client_id = ? AND user_id = ?")
        .bind(client_id.as_bytes().to_vec())
        .bind(user_id.as_bytes().to_vec())
        .execute(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

fn uuid_from_bytes(bytes: &[u8]) -> Option<Uuid> {
    Uuid::from_slice(bytes).ok()
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
            "/organizations/{id}/members/{uid}/assign",
            post(assign_member),
        )
        .route(
            "/organizations/{id}/members/{uid}/watch",
            post(watch_task_for_member),
        )
        .route(
            "/organizations/{id}/members/{uid}/assignments",
            get(get_member_assignments),
        )
        .route(
            "/organizations/{id}/members/{uid}/assignments/project/{pid}",
            delete(unassign_project),
        )
        .route(
            "/organizations/{id}/members/{uid}/assignments/client/{cid}",
            delete(unassign_client),
        )
        .route("/organizations/{id}/generate-invite", post(generate_invite))
        .route("/organizations/{id}/persons", get(get_org_persons))
        .route("/organizations/{id}/data-sources", get(list_org_data_sources))
        .route("/data-sources", get(list_data_sources))
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

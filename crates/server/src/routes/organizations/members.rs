use super::*;

/// GET /api/organizations/:id/members — list members with user details
pub async fn list_members(
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let id_uuid = Uuid::parse_str(&id).map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;
    let pool = &deployment.db().pool;
    if !access_context.is_admin {
        let role = Organization::get_user_role(pool, &id, access_context.user_id.as_str()).await?;
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
    .bind(id_uuid.as_bytes().as_slice())
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
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<AddMemberRequest>,
) -> Result<Json<ApiResponse<OrganizationMember>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, &id, access_context.user_id.as_str()).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can add members".into())),
        }
    }

    let role = data.role.as_deref().unwrap_or("member");
    let member = Organization::add_member(&deployment.db().pool, &id, &data.user_id.to_string(), role).await?;
    Ok(Json(ApiResponse::success(member)))
}

#[derive(Debug, Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

/// PUT /api/organizations/:id/members/:uid/role — change role
pub async fn change_member_role(
    Path((id, uid)): Path<(String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<ChangeRoleRequest>,
) -> Result<Json<ApiResponse<OrganizationMember>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, &id, access_context.user_id.as_str()).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can change roles".into())),
        }
    }

    // Remove and re-add with new role
    Organization::remove_member(&deployment.db().pool, &id, &uid).await?;
    let member = Organization::add_member(&deployment.db().pool, &id, &uid, &data.role).await?;
    Ok(Json(ApiResponse::success(member)))
}

/// DELETE /api/organizations/:id/members/:uid — remove member
pub async fn remove_member(
    Path((id, uid)): Path<(String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if !access_context.is_admin {
        let role = Organization::get_user_role(&deployment.db().pool, &id, access_context.user_id.as_str()).await?;
        match role.as_deref() {
            Some("admin") => {}
            _ => return Err(ApiError::Forbidden("Only org admins can remove members".into())),
        }
    }

    Organization::remove_member(&deployment.db().pool, &id, &uid).await?;
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
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<GenerateInviteBody>,
) -> Result<Json<ApiResponse<GenerateInviteResponse>>, ApiError> {
    if !access_context.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let id_uuid = Uuid::parse_str(&id).map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;
    let pool = &deployment.db().pool;
    let token = Uuid::new_v4().to_string();

    sqlx::query(
        "UPDATE organizations SET invite_token = ?, pending_owner_email = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&token)
    .bind(&body.email)
    .bind(id_uuid.as_bytes().as_slice())
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
    Path(id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Person>>>, ApiError> {
    let pool = &deployment.db().pool;

    if !access_context.is_admin {
        let role = Organization::get_user_role(pool, &id, access_context.user_id.as_str()).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
        }
    }

    // Return persons directly in org UNION persons bridged via crm_contacts
    let id_str = id.clone();
    let persons = sqlx::query_as::<_, Person>(
        "SELECT * FROM persons WHERE organization_id = ?
         UNION
         SELECT p.* FROM persons p
         INNER JOIN crm_contacts cc ON cc.person_id = p.id
         WHERE cc.organization_id = ? AND (p.organization_id IS NULL OR p.organization_id != ?)
         ORDER BY full_name ASC",
    )
    .bind(&id_str)
    .bind(&id_str)
    .bind(&id_str)
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
        let org_id_str = org_uuid.to_string();
        sqlx::query_scalar::<_, String>(
            r#"SELECT json_object(
                'id', id,
                'organization_id', organization_id,
                'project_id', project_id,
                'created_by', created_by,
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
        .bind(&org_id_str)
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
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Value>>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_id_str = id.clone();
    let rows: Vec<Value> = sqlx::query_scalar::<_, String>(
        r#"SELECT json_object(
            'id', id,
            'organization_id', organization_id,
            'project_id', project_id,
            'created_by', created_by,
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
    .bind(&org_id_str)
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter_map(|s| serde_json::from_str(&s).ok())
    .collect();
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /organizations/:id/person-contacts — junction table entries (for context badges)
pub async fn list_org_person_contacts(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<PersonOrgContact>>>, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;
    let pool = &deployment.db().pool;
    let contacts = PersonOrgContact::list_for_org(pool, id).await?;
    Ok(Json(ApiResponse::success(contacts)))
}

#[derive(Debug, serde::Deserialize)]
pub struct AddOrgPersonContactBody {
    person_id: Uuid,
    context: Option<String>,
    notes: Option<String>,
}

/// POST /organizations/:id/person-contacts — link an existing person to this org
pub async fn add_org_person_contact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<AddOrgPersonContactBody>,
) -> Result<Json<ApiResponse<PersonOrgContact>>, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;
    let pool = &deployment.db().pool;
    let upsert_data = UpsertPersonOrgContact {
        organization_id: id,
        context: data.context,
        notes: data.notes,
    };
    let contact = PersonOrgContact::upsert(pool, data.person_id, upsert_data).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// GET /api/organizations/:id/companies — list companies created by this org
pub async fn list_org_companies(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let companies = Company::list(pool, Some(db::db_uuid::DbUuid::from(id.clone())), None, Some(200)).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(companies))
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
pub async fn require_org_admin_access(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: Uuid,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, &org_id.to_string(), access_context.user_id.as_str()).await?;
    match role.as_deref() {
        Some("admin") => Ok(()),
        _ => Err(ApiError::Forbidden("Only org admins can manage member assignments".into())),
    }
}

/// Helper: verify target user is an org member
pub async fn require_is_org_member(
    pool: &sqlx::SqlitePool,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let role = Organization::get_user_role(pool, &org_id.to_string(), &user_id.to_string()).await?;
    if role.is_none() {
        return Err(ApiError::BadRequest("User is not a member of this organization".into()));
    }
    Ok(())
}

/// POST /api/organizations/:id/members/:uid/assign — assign member to project/client/task
pub async fn assign_member(
    Path((org_id, user_id)): Path<(String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<AssignMemberRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let org_id = Uuid::parse_str(&org_id).map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id).map_err(|e| ApiError::BadRequest(format!("Invalid user UUID: {}", e)))?;
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
            .bind(db::bind_uuid_blob(&access_context.user_id).map_err(|e| ApiError::InternalError(format!("Invalid UUID: {e}")))?)
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
            .bind(db::bind_uuid_blob(&access_context.user_id).map_err(|e| ApiError::InternalError(format!("Invalid UUID: {e}")))?)
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
    Path((org_id, user_id)): Path<(String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<WatchTaskRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let org_id = Uuid::parse_str(&org_id).map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id).map_err(|e| ApiError::BadRequest(format!("Invalid user UUID: {}", e)))?;
    let pool = &deployment.db().pool;
    require_org_admin_access(pool, &access_context, org_id).await?;
    require_is_org_member(pool, org_id, user_id).await?;

    // Use the Task::add_watcher method from Phase 2
    db::models::task::Task::add_watcher(pool, &data.task_id.to_string(), &user_id.to_string()).await
        .map_err(|e| ApiError::InternalError(format!("Failed to add watcher: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "watching": "task",
        "task_id": data.task_id.to_string(),
        "user_id": user_id.to_string(),
    }))))
}

/// GET /api/organizations/:id/members/:uid/assignments — get all assignments for a member
pub async fn get_member_assignments(
    Path((org_id, user_id)): Path<(String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let org_id = Uuid::parse_str(&org_id).map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id).map_err(|e| ApiError::BadRequest(format!("Invalid user UUID: {}", e)))?;
    let pool = &deployment.db().pool;

    // Any org member can view assignments; admins can view anyone's
    if !access_context.is_admin && access_context.user_id.as_str() != user_id.to_string() {
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
    Path((org_id, user_id, project_id)): Path<(String, String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let org_id = Uuid::parse_str(&org_id).map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id).map_err(|e| ApiError::BadRequest(format!("Invalid user UUID: {}", e)))?;
    let project_id = Uuid::parse_str(&project_id).map_err(|e| ApiError::BadRequest(format!("Invalid project UUID: {}", e)))?;
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
    Path((org_id, user_id, client_id)): Path<(String, String, String)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let org_id = Uuid::parse_str(&org_id).map_err(|e| ApiError::BadRequest(format!("Invalid org UUID: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id).map_err(|e| ApiError::BadRequest(format!("Invalid user UUID: {}", e)))?;
    let client_id = Uuid::parse_str(&client_id).map_err(|e| ApiError::BadRequest(format!("Invalid client UUID: {}", e)))?;
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

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use db::models::{
    client::{Client, ClientMember, CreateClient, CreateClientMember, UpdateClient},
    project::Project,
    user::Organization,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::AccessContext,
};

/// Check if user has access to an org (is member or admin)
async fn require_org_access(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: &str,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id).await?;
    if role.is_none() {
        return Err(ApiError::Forbidden("Not a member of this organization".into()));
    }
    Ok(())
}

/// Check if user has admin access to an org
async fn require_org_admin(
    pool: &sqlx::SqlitePool,
    access_context: &AccessContext,
    org_id: &str,
) -> Result<(), ApiError> {
    if access_context.is_admin {
        return Ok(());
    }
    let role = Organization::get_user_role(pool, org_id, access_context.user_id).await?;
    match role.as_deref() {
        Some("admin") => Ok(()),
        _ => Err(ApiError::Forbidden("Only org admins can manage clients".into())),
    }
}

/// GET /api/organizations/:org_id/clients — list clients in org
pub async fn list_clients(
    Path(org_id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Client>>>, ApiError> {
    let org_id_str = org_id.to_string();
    require_org_access(&deployment.db().pool, &access_context, &org_id_str).await?;
    let clients = Client::find_by_organization(&deployment.db().pool, &org_id_str).await?;
    Ok(Json(ApiResponse::success(clients)))
}

/// POST /api/organizations/:org_id/clients — create client
pub async fn create_client(
    Path(org_id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateClient>,
) -> Result<Json<ApiResponse<Client>>, ApiError> {
    let org_id_str = org_id.to_string();
    require_org_admin(&deployment.db().pool, &access_context, &org_id_str).await?;

    // Check slug uniqueness within org
    if let Some(_) = Client::find_by_slug(&deployment.db().pool, &org_id_str, &data.slug).await? {
        return Err(ApiError::Conflict("Client with this slug already exists in the organization".into()));
    }

    let id = Uuid::new_v4().to_string();
    let client = Client::create(&deployment.db().pool, &id, &org_id_str, &data).await?;
    Ok(Json(ApiResponse::success(client)))
}

/// GET /api/clients/:id — client details
pub async fn get_client(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Client>>, ApiError> {
    let client = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_access(&deployment.db().pool, &access_context, &client.organization_id).await?;
    Ok(Json(ApiResponse::success(client)))
}

/// PUT /api/clients/:id — update client
pub async fn update_client(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<UpdateClient>,
) -> Result<Json<ApiResponse<Client>>, ApiError> {
    let existing = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, &existing.organization_id).await?;

    let client = Client::update(&deployment.db().pool, &id.to_string(), &data).await?;
    Ok(Json(ApiResponse::success(client)))
}

/// DELETE /api/clients/:id — soft delete
pub async fn delete_client(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let existing = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, &existing.organization_id).await?;

    Client::soft_delete(&deployment.db().pool, &id.to_string(), &access_context.user_id.to_string()).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/clients/:id/members — list members with user details
pub async fn list_client_members(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let client = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_access(&deployment.db().pool, &access_context, &client.organization_id).await?;

    #[derive(sqlx::FromRow)]
    struct MemberWithUser {
        id: Vec<u8>,
        client_id: Vec<u8>,
        user_id: Vec<u8>,
        role: String,
        granted_by: Option<Vec<u8>>,
        granted_at: String,
        username: String,
        full_name: String,
        avatar_url: Option<String>,
        granted_by_username: Option<String>,
    }

    let rows: Vec<MemberWithUser> = sqlx::query_as(
        r#"SELECT cm.id, cm.client_id, cm.user_id, cm.role, cm.granted_by, cm.granted_at,
                  u.username, u.full_name, u.avatar_url,
                  gb.username as granted_by_username
           FROM client_members cm
           JOIN users u ON u.id = cm.user_id
           LEFT JOIN users gb ON gb.id = cm.granted_by
           WHERE cm.client_id = ?
           ORDER BY cm.granted_at ASC"#,
    )
    .bind(id)
    .fetch_all(&deployment.db().pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let members: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|row| {
            let id = Uuid::from_slice(&row.id).ok()?;
            let client_id = Uuid::from_slice(&row.client_id).ok()?;
            let user_id = Uuid::from_slice(&row.user_id).ok()?;
            let granted_by = row.granted_by.as_ref().and_then(|b| Uuid::from_slice(b).ok());
            Some(serde_json::json!({
                "id": id.to_string(),
                "client_id": client_id.to_string(),
                "user_id": user_id.to_string(),
                "role": row.role,
                "granted_by": granted_by.map(|u| u.to_string()),
                "granted_at": row.granted_at,
                "username": row.username,
                "full_name": row.full_name,
                "avatar_url": row.avatar_url,
                "granted_by_username": row.granted_by_username,
            }))
        })
        .collect();

    Ok(Json(ApiResponse::success(members)))
}

/// POST /api/clients/:id/members — add member
pub async fn add_client_member(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateClientMember>,
) -> Result<Json<ApiResponse<ClientMember>>, ApiError> {
    let client = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, &client.organization_id).await?;

    let member_id = Uuid::new_v4().to_string();
    let role = data.role.as_deref().unwrap_or("viewer");
    let granted_by = access_context.user_id.to_string();
    let member = Client::add_member(
        &deployment.db().pool,
        &member_id,
        &id.to_string(),
        Uuid::parse_str(&data.user_id).map_err(|_| ApiError::BadRequest("Invalid user_id".into()))?,
        role,
        Some(&granted_by),
    )
    .await?;
    Ok(Json(ApiResponse::success(member)))
}

/// DELETE /api/clients/:id/members/:uid — remove member
pub async fn remove_client_member(
    Path((id, uid)): Path<(Uuid, Uuid)>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let client = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_admin(&deployment.db().pool, &access_context, &client.organization_id).await?;

    Client::remove_member(&deployment.db().pool, &id.to_string(), uid).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/clients/:id/projects — list projects for client
pub async fn list_client_projects(
    Path(id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<Project>>>, ApiError> {
    let client = Client::find_by_id(&deployment.db().pool, &id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found".into()))?;

    require_org_access(&deployment.db().pool, &access_context, &client.organization_id).await?;

    let projects = Project::find_by_client(&deployment.db().pool, &id.to_string()).await?;
    Ok(Json(ApiResponse::success(projects)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Org-scoped client routes
        .route(
            "/organizations/{org_id}/clients",
            get(list_clients).post(create_client),
        )
        // Direct client routes
        .route(
            "/clients/{id}",
            get(get_client).put(update_client).delete(delete_client),
        )
        .route(
            "/clients/{id}/members",
            get(list_client_members).post(add_client_member),
        )
        .route(
            "/clients/{id}/members/{uid}",
            delete(remove_client_member),
        )
        .route("/clients/{id}/projects", get(list_client_projects))
}

/// Org-level invitation routes (copy-link flow)
///
/// Uses the `org_invitations` table created in the member management migration.
/// Org admins can create invite links that grant org membership upon acceptance.

use axum::{
    Extension, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

#[derive(Debug, Serialize)]
pub struct OrgInvitationResponse {
    pub id: String,
    pub organization_id: String,
    pub invite_code: String,
    pub invite_url: String,
    pub role: String,
    pub created_by: String,
    pub created_at: String,
    pub expires_at: String,
    pub max_uses: i64,
    pub use_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgInvitationRequest {
    pub role: Option<String>,
    pub max_uses: Option<i64>,
    /// Hours until expiry (default: 168 = 7 days)
    pub expires_in_hours: Option<i64>,
}

// Accept request uses the user's session - no body needed

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/organizations/{org_id}/invitations",
        Router::new()
            .route("/", post(create_org_invitation).get(list_org_invitations))
            .route("/{id}/revoke", post(revoke_org_invitation)),
    )
}

pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/org-invite/{code}", get(get_org_invitation))
        .route("/org-invite/{code}/accept", post(accept_org_invitation))
}

/// POST /api/organizations/:org_id/invitations — create a new invite link
async fn create_org_invitation(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
    ResponseJson(body): ResponseJson<CreateOrgInvitationRequest>,
) -> Result<ResponseJson<ApiResponse<OrgInvitationResponse>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let org_uuid = Uuid::parse_str(&org_id)
        .map_err(|_| ApiError::BadRequest("Invalid organization ID".into()))?;
    let org_id_bytes = org_uuid.as_bytes().to_vec();
    let user_id_bytes = access_context.user_id.as_bytes().to_vec();

    // Only org admins can create invitations
    #[derive(sqlx::FromRow)]
    struct RoleRow { role: String }
    let org_role: Option<RoleRow> = sqlx::query_as(
        "SELECT role FROM organization_members WHERE organization_id = ? AND user_id = ?"
    )
    .bind(&org_id_bytes)
    .bind(&user_id_bytes)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let is_org_admin = org_role.map(|r| r.role == "admin").unwrap_or(false);
    if !is_org_admin && !access_context.is_admin {
        return Err(ApiError::Forbidden("Only org admins can create invitations".into()));
    }

    let role = body.role.as_deref().unwrap_or("member");
    if !["admin", "member", "viewer"].contains(&role) {
        return Err(ApiError::BadRequest("Invalid role. Must be admin, member, or viewer".into()));
    }

    let max_uses = body.max_uses.unwrap_or(1);
    let expires_hours = body.expires_in_hours.unwrap_or(168); // 7 days default

    let invite_id = Uuid::new_v4();
    let invite_id_bytes = invite_id.as_bytes().to_vec();
    let invite_code = format!(
        "{}{}",
        &Uuid::new_v4().to_string().replace('-', "")[..16],
        &Uuid::new_v4().to_string().replace('-', "")[..8]
    );

    sqlx::query(
        r#"INSERT INTO org_invitations (id, organization_id, invite_code, role, created_by, expires_at, max_uses)
           VALUES (?, ?, ?, ?, ?, datetime('now', '+' || ? || ' hours'), ?)"#,
    )
    .bind(&invite_id_bytes)
    .bind(&org_id_bytes)
    .bind(&invite_code)
    .bind(role)
    .bind(&user_id_bytes)
    .bind(expires_hours)
    .bind(max_uses)
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create invitation: {}", e)))?;

    let base_url = std::env::var("TWILIO_WEBHOOK_BASE_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".into());
    let invite_url = format!("{}/org-invite/{}", base_url.trim_end_matches('/'), invite_code);

    #[derive(sqlx::FromRow)]
    struct DatesRow { created_at: String, expires_at: String }
    let dates: DatesRow = sqlx::query_as(
        "SELECT created_at, expires_at FROM org_invitations WHERE id = ?"
    )
    .bind(&invite_id_bytes)
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(OrgInvitationResponse {
        id: invite_id.to_string(),
        organization_id: org_id,
        invite_code,
        invite_url,
        role: role.to_string(),
        created_by: access_context.user_id.to_string(),
        created_at: dates.created_at,
        expires_at: dates.expires_at,
        max_uses,
        use_count: 0,
    })))
}

/// GET /api/organizations/:org_id/invitations — list active invitations
async fn list_org_invitations(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<String>,
) -> Result<ResponseJson<ApiResponse<Vec<OrgInvitationResponse>>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let org_uuid = Uuid::parse_str(&org_id)
        .map_err(|_| ApiError::BadRequest("Invalid organization ID".into()))?;
    let org_id_bytes = org_uuid.as_bytes().to_vec();

    let base_url = std::env::var("TWILIO_WEBHOOK_BASE_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".into());

    #[derive(sqlx::FromRow)]
    struct InviteRow {
        id: Vec<u8>,
        invite_code: String,
        role: String,
        created_by: Vec<u8>,
        created_at: String,
        expires_at: String,
        max_uses: i64,
        use_count: i64,
    }

    let rows: Vec<InviteRow> = sqlx::query_as(
        r#"SELECT id, invite_code, role, created_by, created_at, expires_at, max_uses, use_count
           FROM org_invitations
           WHERE organization_id = ? AND expires_at > datetime('now') AND use_count < max_uses
           ORDER BY created_at DESC"#,
    )
    .bind(&org_id_bytes)
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let invitations = rows
        .into_iter()
        .filter_map(|row| {
            let id = Uuid::from_slice(&row.id).ok()?.to_string();
            let created_by = Uuid::from_slice(&row.created_by).ok()?.to_string();
            let invite_url = format!(
                "{}/org-invite/{}",
                base_url.trim_end_matches('/'),
                row.invite_code
            );
            Some(OrgInvitationResponse {
                id,
                organization_id: org_id.clone(),
                invite_code: row.invite_code,
                invite_url,
                role: row.role,
                created_by,
                created_at: row.created_at,
                expires_at: row.expires_at,
                max_uses: row.max_uses,
                use_count: row.use_count,
            })
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(invitations)))
}

/// POST /api/organizations/:org_id/invitations/:id/revoke
async fn revoke_org_invitation(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((org_id, id)): Path<(String, String)>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let invite_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid invitation ID".into()))?;
    let invite_id_bytes = invite_uuid.as_bytes().to_vec();
    let org_uuid = Uuid::parse_str(&org_id)
        .map_err(|_| ApiError::BadRequest("Invalid organization ID".into()))?;
    let org_id_bytes = org_uuid.as_bytes().to_vec();

    // Expire it immediately
    let rows_affected = sqlx::query(
        "UPDATE org_invitations SET expires_at = datetime('now', '-1 second') WHERE id = ? AND organization_id = ?"
    )
    .bind(&invite_id_bytes)
    .bind(&org_id_bytes)
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ApiError::NotFound("Invitation not found".into()));
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

/// GET /api/org-invite/:code — public: get invite details
async fn get_org_invitation(
    State(deployment): State<DeploymentImpl>,
    Path(code): Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = deployment.db().pool.clone();

    #[derive(sqlx::FromRow)]
    struct InviteInfo {
        role: String,
        expires_at: String,
        max_uses: i64,
        use_count: i64,
        org_name: Option<String>,
    }

    let info: InviteInfo = sqlx::query_as(
        r#"SELECT i.role, i.expires_at, i.max_uses, i.use_count, o.name as org_name
           FROM org_invitations i
           JOIN organizations o ON o.id = i.organization_id
           WHERE i.invite_code = ?"#,
    )
    .bind(&code)
    .fetch_one(&pool)
    .await
    .map_err(|_| ApiError::NotFound("Invitation not found".into()))?;

    if info.expires_at < chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string() {
        return Err(ApiError::BadRequest("Invitation has expired".into()));
    }
    if info.use_count >= info.max_uses {
        return Err(ApiError::BadRequest("Invitation has reached its usage limit".into()));
    }

    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "organization_name": info.org_name,
        "role": info.role,
        "expires_at": info.expires_at,
    }))))
}

/// POST /api/org-invite/:code/accept — accept invite (requires auth via session)
async fn accept_org_invitation(
    State(deployment): State<DeploymentImpl>,
    Path(code): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = deployment.db().pool.clone();

    // Get the current user from session
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let access = crate::middleware::access_control::get_current_user(
        &deployment, auth_header, cookie_header
    ).await?;

    let user_id_bytes = access.user_id.as_bytes().to_vec();

    // Fetch invitation
    #[derive(sqlx::FromRow)]
    struct InviteRow {
        id: Vec<u8>,
        organization_id: Vec<u8>,
        role: String,
        expires_at: String,
        max_uses: i64,
        use_count: i64,
    }

    let invite: InviteRow = sqlx::query_as(
        "SELECT id, organization_id, role, expires_at, max_uses, use_count FROM org_invitations WHERE invite_code = ?"
    )
    .bind(&code)
    .fetch_one(&pool)
    .await
    .map_err(|_| ApiError::NotFound("Invitation not found".into()))?;

    if invite.expires_at < chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string() {
        return Err(ApiError::BadRequest("Invitation has expired".into()));
    }
    if invite.use_count >= invite.max_uses {
        return Err(ApiError::BadRequest("Invitation has reached its usage limit".into()));
    }

    // Check if user is already a member
    let already_member: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM organization_members WHERE organization_id = ? AND user_id = ? LIMIT 1"
    )
    .bind(&invite.organization_id)
    .bind(&user_id_bytes)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if already_member.is_some() {
        return Err(ApiError::BadRequest("You are already a member of this organization".into()));
    }

    // Add user as org member
    let member_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO organization_members (id, organization_id, user_id, role)
           VALUES (?, ?, ?, ?)"#,
    )
    .bind(member_id.as_bytes().to_vec())
    .bind(&invite.organization_id)
    .bind(&user_id_bytes)
    .bind(&invite.role)
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to add member: {}", e)))?;

    // Increment use_count and record accepted_by
    sqlx::query(
        "UPDATE org_invitations SET use_count = use_count + 1, accepted_by = ?, accepted_at = datetime('now') WHERE id = ?"
    )
    .bind(&user_id_bytes)
    .bind(&invite.id)
    .execute(&pool)
    .await
    .ok();

    let org_uuid = Uuid::from_slice(&invite.organization_id)
        .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "organization_id": org_uuid.to_string(),
        "role": invite.role,
        "message": "Successfully joined the organization",
    }))))
}

/// Invitation & Virtual Space routes
///
/// Access model:
///   - Only hosts (users with registered hardware) can invite guests
///   - Guests spawn in their inviting host's virtual space
///   - Invite tokens expire after 7 days

use axum::{
    Extension, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utils::response::ApiResponse;

use db::services::AuthService;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Invitation management (host only - enforced in handlers)
        .route("/invitations", post(create_invitation))
        .route("/invitations", get(list_invitations))
        .route("/invitations/{id}/revoke", post(revoke_invitation))
        // Virtual space
        .route("/virtual-spaces", get(list_spaces))
        .route("/virtual-space/me", get(my_spawn_point))
        .route("/virtual-space/{user_id}", get(get_user_space))
}

pub fn public_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Public: no auth needed to look up or accept an invite
        .route("/invitations/accept/{token}", get(get_invitation))
        .route("/invitations/accept/{token}", post(accept_invitation))
}

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvitationRequest {
    pub invitee_email: String,
    pub invitee_name: Option<String>,
    /// Optional: add them to a project on acceptance
    pub project_id: Option<String>,
    pub project_role: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvitationResponse {
    pub id: String,
    pub token: String,
    pub invite_url: String,
    pub invitee_email: String,
    pub invitee_name: Option<String>,
    pub status: String,
    pub expires_at: String,
    pub created_at: String,
    pub host_username: String,
}

#[derive(Debug, Serialize)]
pub struct SpawnPoint {
    pub space_name: String,
    pub host_username: String,
    pub world_x: f64,
    pub world_y: f64,
    pub world_z: f64,
    pub spawn_x: f64,
    pub spawn_y: f64,
    pub spawn_z: f64,
    pub theme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationRequest {
    pub username: String,
    pub password: String,
    pub full_name: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/invitations — host sends an invitation
async fn create_invitation(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(body): ResponseJson<CreateInvitationRequest>,
) -> Result<ResponseJson<ApiResponse<InvitationResponse>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let user_id_bytes = access_context.user_id.as_bytes().to_vec();

    // Check caller is a host
    let user_role: String = sqlx::query_scalar(
        "SELECT user_role FROM users WHERE id = ?",
    )
    .bind(&user_id_bytes)
    .fetch_one(&pool)
    .await
    .map_err(|_| ApiError::NotFound("User not found".into()))?;

    if user_role != "host" {
        return Err(ApiError::Forbidden(
            "Only hosts with registered hardware can invite guests".into(),
        ));
    }

    // Check invite email not already registered
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email = ? AND deleted_at IS NULL",
    )
    .bind(&body.invitee_email)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if existing.unwrap_or(0) > 0 {
        return Err(ApiError::BadRequest(
            "A user with that email already exists".into(),
        ));
    }

    // Generate a secure token
    let token = Uuid::new_v4().to_string().replace("-", "") + &Uuid::new_v4().to_string().replace("-", "");

    let invite_id = Uuid::new_v4();
    let invite_id_bytes = invite_id.as_bytes().to_vec();

    let project_id_bytes: Option<Vec<u8>> = body.project_id.as_ref().and_then(|s| {
        Uuid::parse_str(s).ok().map(|u| u.as_bytes().to_vec())
    });

    sqlx::query(
        r#"INSERT INTO user_invitations
           (id, token, host_id, invitee_email, invitee_name, project_id, project_role)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&invite_id_bytes)
    .bind(&token)
    .bind(&user_id_bytes)
    .bind(&body.invitee_email)
    .bind(&body.invitee_name)
    .bind(&project_id_bytes)
    .bind(body.project_role.as_deref().unwrap_or("viewer"))
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create invitation: {}", e)))?;

    let host_username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(&user_id_bytes)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|_| "unknown".into());

    let base_url = std::env::var("TWILIO_WEBHOOK_BASE_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".into());

    let invite_url = format!("{}/join/{}", base_url.trim_end_matches('/'), token);

    let expires_at: String = sqlx::query_scalar(
        "SELECT expires_at FROM user_invitations WHERE id = ?",
    )
    .bind(&invite_id_bytes)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| "".into());

    let created_at: String = sqlx::query_scalar(
        "SELECT created_at FROM user_invitations WHERE id = ?",
    )
    .bind(&invite_id_bytes)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| "".into());

    Ok(ResponseJson(ApiResponse::success(InvitationResponse {
        id: invite_id.to_string(),
        token,
        invite_url,
        invitee_email: body.invitee_email,
        invitee_name: body.invitee_name,
        status: "pending".into(),
        expires_at,
        created_at,
        host_username,
    })))
}

/// GET /api/invitations — list invitations sent by current host
async fn list_invitations(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<InvitationResponse>>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let user_id_bytes = access_context.user_id.as_bytes().to_vec();

    let base_url = std::env::var("TWILIO_WEBHOOK_BASE_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".into());

    #[derive(sqlx::FromRow)]
    struct InviteRow {
        id: Vec<u8>,
        token: String,
        invitee_email: String,
        invitee_name: Option<String>,
        status: String,
        expires_at: String,
        created_at: String,
    }

    let rows: Vec<InviteRow> = sqlx::query_as(
        r#"SELECT id, token, invitee_email, invitee_name, status, expires_at, created_at
           FROM user_invitations
           WHERE host_id = ?
           ORDER BY created_at DESC"#,
    )
    .bind(&user_id_bytes)
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to list invitations: {}", e)))?;

    let host_username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(&user_id_bytes)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|_| "unknown".into());

    let invitations = rows
        .into_iter()
        .filter_map(|row| {
            let id = Uuid::from_slice(&row.id).ok()?.to_string();
            let invite_url = format!(
                "{}/join/{}",
                base_url.trim_end_matches('/'),
                row.token
            );
            Some(InvitationResponse {
                id,
                invite_url,
                token: row.token,
                invitee_email: row.invitee_email,
                invitee_name: row.invitee_name,
                status: row.status,
                expires_at: row.expires_at,
                created_at: row.created_at,
                host_username: host_username.clone(),
            })
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(invitations)))
}

/// POST /api/invitations/:id/revoke — host revokes an invitation
async fn revoke_invitation(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let user_id_bytes = access_context.user_id.as_bytes().to_vec();

    let invite_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid invitation ID".into()))?;
    let invite_id_bytes = invite_id.as_bytes().to_vec();

    let rows_affected = sqlx::query(
        "UPDATE user_invitations SET status = 'revoked' WHERE id = ? AND host_id = ? AND status = 'pending'",
    )
    .bind(&invite_id_bytes)
    .bind(&user_id_bytes)
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to revoke invitation: {}", e)))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ApiError::NotFound(
            "Invitation not found or already processed".into(),
        ));
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

/// GET /api/invitations/accept/:token — public: get invite details
async fn get_invitation(
    State(deployment): State<DeploymentImpl>,
    Path(token): Path<String>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = deployment.db().pool.clone();

    #[derive(sqlx::FromRow)]
    struct InviteDetails {
        invitee_email: String,
        invitee_name: Option<String>,
        status: String,
        expires_at: String,
        host_username: Option<String>,
        space_name: Option<String>,
    }

    let row: InviteDetails = sqlx::query_as(
        r#"SELECT
               i.invitee_email,
               i.invitee_name,
               i.status,
               i.expires_at,
               u.username as host_username,
               vs.space_name
           FROM user_invitations i
           LEFT JOIN users u ON u.id = i.host_id
           LEFT JOIN virtual_spaces vs ON vs.owner_id = i.host_id
           WHERE i.token = ?"#,
    )
    .bind(&token)
    .fetch_one(&pool)
    .await
    .map_err(|_| ApiError::NotFound("Invitation not found".into()))?;

    if row.status == "expired" || row.status == "revoked" {
        return Err(ApiError::BadRequest(format!(
            "This invitation has been {}",
            row.status
        )));
    }

    if row.status == "accepted" {
        return Err(ApiError::BadRequest(
            "This invitation has already been accepted".into(),
        ));
    }

    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "invitee_email": row.invitee_email,
        "invitee_name": row.invitee_name,
        "host_username": row.host_username,
        "space_name": row.space_name,
        "expires_at": row.expires_at,
    }))))
}

/// POST /api/invitations/accept/:token — public: accept invite and create account
async fn accept_invitation(
    State(deployment): State<DeploymentImpl>,
    Path(token): Path<String>,
    ResponseJson(body): ResponseJson<AcceptInvitationRequest>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = deployment.db().pool.clone();

    // Fetch and validate the invitation
    #[derive(sqlx::FromRow)]
    struct InviteRow {
        id: Vec<u8>,
        host_id: Vec<u8>,
        invitee_email: String,
        status: String,
        expires_at: String,
        project_id: Option<Vec<u8>>,
        project_role: Option<String>,
    }

    let invite: InviteRow = sqlx::query_as(
        "SELECT id, host_id, invitee_email, status, expires_at, project_id, project_role
         FROM user_invitations WHERE token = ?",
    )
    .bind(&token)
    .fetch_one(&pool)
    .await
    .map_err(|_| ApiError::NotFound("Invitation not found or expired".into()))?;

    if invite.status != "pending" {
        return Err(ApiError::BadRequest(format!(
            "Invitation is {}",
            invite.status
        )));
    }

    // Check expiry
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if invite.expires_at < now {
        sqlx::query("UPDATE user_invitations SET status = 'expired' WHERE id = ?")
            .bind(&invite.id)
            .execute(&pool)
            .await
            .ok();
        return Err(ApiError::BadRequest("Invitation has expired".into()));
    }

    // Check username not taken
    let username_taken: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE username = ? AND deleted_at IS NULL",
    )
    .bind(&body.username)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    if username_taken > 0 {
        return Err(ApiError::BadRequest("Username already taken".into()));
    }

    // Hash the password
    let password_hash = AuthService::hash_password(&body.password)
        .map_err(|e| ApiError::InternalError(format!("Failed to hash password: {}", e)))?;

    let new_user_id = Uuid::new_v4();
    let new_user_id_bytes = new_user_id.as_bytes().to_vec();

    // Create the user as a guest, linked to the inviting host
    sqlx::query(
        r#"INSERT INTO users
           (id, username, email, password_hash, full_name, is_active, is_admin, user_role, invited_by)
           VALUES (?, ?, ?, ?, ?, 1, 0, 'guest', ?)"#,
    )
    .bind(&new_user_id_bytes)
    .bind(&body.username)
    .bind(&invite.invitee_email)
    .bind(&password_hash)
    .bind(body.full_name.as_deref().unwrap_or(&body.username))
    .bind(&invite.host_id)
    .execute(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create user: {}", e)))?;

    // Add to project if specified in invitation
    if let (Some(project_id), Some(role)) = (invite.project_id, invite.project_role) {
        sqlx::query(
            r#"INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by)
               VALUES (randomblob(16), ?, ?, ?, ?)"#,
        )
        .bind(&project_id)
        .bind(&new_user_id_bytes)
        .bind(&role)
        .bind(&invite.host_id)
        .execute(&pool)
        .await
        .ok();
    }

    // Mark invitation as accepted
    sqlx::query(
        "UPDATE user_invitations
         SET status = 'accepted', invitee_user_id = ?, accepted_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_user_id_bytes)
    .bind(&invite.id)
    .execute(&pool)
    .await
    .ok();

    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "user_id": new_user_id.to_string(),
        "username": body.username,
        "message": "Account created. You can now sign in.",
    }))))
}

/// GET /api/virtual-space/me — returns the caller's spawn point
async fn my_spawn_point(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<SpawnPoint>>, ApiError> {
    let pool = deployment.db().pool.clone();
    let user_id_bytes = access_context.user_id.as_bytes().to_vec();

    // If the user is a host, return their own space
    // If the user is a guest, return their host's space
    let spawn: Option<SpawnPoint> = sqlx::query(
        r#"SELECT
               vs.space_name,
               u_host.username as host_username,
               vs.world_x, vs.world_y, vs.world_z,
               vs.spawn_x, vs.spawn_y, vs.spawn_z,
               vs.theme
           FROM users u
           -- Host: has their own virtual space
           LEFT JOIN virtual_spaces vs_own ON vs_own.owner_id = u.id
           -- Guest: look up their inviting host's space
           LEFT JOIN users host ON host.id = u.invited_by
           LEFT JOIN virtual_spaces vs_host ON vs_host.owner_id = host.id
           -- Use own space if host, host's space if guest
           JOIN virtual_spaces vs ON vs.id = COALESCE(vs_own.id, vs_host.id)
           JOIN users u_host ON u_host.id = vs.owner_id
           WHERE u.id = ?"#,
    )
    .bind(&user_id_bytes)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch spawn point: {}", e)))?
    .and_then(|row| {
        use sqlx::Row;
        Some(SpawnPoint {
            space_name: row.try_get("space_name").ok()?,
            host_username: row.try_get("host_username").ok()?,
            world_x: row.try_get("world_x").ok()?,
            world_y: row.try_get("world_y").ok()?,
            world_z: row.try_get("world_z").ok()?,
            spawn_x: row.try_get("spawn_x").ok()?,
            spawn_y: row.try_get("spawn_y").ok()?,
            spawn_z: row.try_get("spawn_z").ok()?,
            theme: row.try_get("theme").ok()?,
        })
    });

    match spawn {
        Some(s) => Ok(ResponseJson(ApiResponse::success(s))),
        None => Err(ApiError::NotFound("No virtual space found for this user".into())),
    }
}

/// GET /api/virtual-spaces — list all virtual spaces for the global world map
async fn list_spaces(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let pool = deployment.db().pool.clone();

    let rows = sqlx::query(
        r#"SELECT
               vs.space_name,
               u.username as host_username,
               u.id as host_id,
               vs.world_x, vs.world_y, vs.world_z,
               vs.spawn_x, vs.spawn_y, vs.spawn_z,
               vs.theme, vs.max_guests
           FROM virtual_spaces vs
           JOIN users u ON u.id = vs.owner_id
           ORDER BY vs.world_x ASC"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to list spaces: {}", e)))?;

    use sqlx::Row;
    let spaces: Vec<serde_json::Value> = rows
        .iter()
        .filter_map(|row| {
            let host_id_bytes: Vec<u8> = row.try_get("host_id").ok()?;
            let host_id = Uuid::from_slice(&host_id_bytes).ok()?;
            Some(serde_json::json!({
                "space_name": row.try_get::<String, _>("space_name").ok()?,
                "host_username": row.try_get::<String, _>("host_username").ok()?,
                "host_id": host_id.to_string(),
                "world_x": row.try_get::<f64, _>("world_x").ok()?,
                "world_y": row.try_get::<f64, _>("world_y").ok()?,
                "world_z": row.try_get::<f64, _>("world_z").ok()?,
                "spawn_x": row.try_get::<f64, _>("spawn_x").ok()?,
                "spawn_y": row.try_get::<f64, _>("spawn_y").ok()?,
                "spawn_z": row.try_get::<f64, _>("spawn_z").ok()?,
                "theme": row.try_get::<Option<String>, _>("theme").ok()?,
                "max_guests": row.try_get::<i64, _>("max_guests").ok()?,
            }))
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(spaces)))
}

/// GET /api/virtual-space/:user_id — get a specific user's virtual space
async fn get_user_space(
    State(deployment): State<DeploymentImpl>,
    Path(user_id): Path<String>,
) -> Result<ResponseJson<ApiResponse<SpawnPoint>>, ApiError> {
    let pool = deployment.db().pool.clone();

    let uid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".into()))?;
    let uid_bytes = uid.as_bytes().to_vec();

    let spawn: Option<SpawnPoint> = sqlx::query(
        r#"SELECT
               vs.space_name,
               u.username as host_username,
               vs.world_x, vs.world_y, vs.world_z,
               vs.spawn_x, vs.spawn_y, vs.spawn_z,
               vs.theme
           FROM virtual_spaces vs
           JOIN users u ON u.id = vs.owner_id
           WHERE vs.owner_id = ?"#,
    )
    .bind(&uid_bytes)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch space: {}", e)))?
    .and_then(|row| {
        use sqlx::Row;
        Some(SpawnPoint {
            space_name: row.try_get("space_name").ok()?,
            host_username: row.try_get("host_username").ok()?,
            world_x: row.try_get("world_x").ok()?,
            world_y: row.try_get("world_y").ok()?,
            world_z: row.try_get("world_z").ok()?,
            spawn_x: row.try_get("spawn_x").ok()?,
            spawn_y: row.try_get("spawn_y").ok()?,
            spawn_z: row.try_get("spawn_z").ok()?,
            theme: row.try_get("theme").ok()?,
        })
    });

    match spawn {
        Some(s) => Ok(ResponseJson(ApiResponse::success(s))),
        None => Err(ApiError::NotFound("No virtual space found for this user".into())),
    }
}

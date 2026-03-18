//! Onboarding helpers: caller account creation, PCG team lookup, project setup.

use super::*;

// ---------------------------------------------------------------------------
// Account creation
// ---------------------------------------------------------------------------

/// Create a bare-bones PCG user account for a new phone caller.
/// Returns (user_id, project_id).
pub(super) async fn create_caller_account(
    pool: &sqlx::SqlitePool,
    phone: &str,
    full_name: &str,
) -> anyhow::Result<(Uuid, Uuid)> {
    // Sanitise phone into a valid username slug
    let slug = phone
        .replace('+', "")
        .replace(['-', ' ', '(', ')'], "_");
    let username = format!("caller_{}", slug);
    let email = format!("{}@pcg.phone.noreply", slug);

    // Check if user already exists
    let existing: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ? LIMIT 1")
            .bind(&username)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    if let Some((id_bytes,)) = existing {
        let user_id = Uuid::from_slice(&id_bytes)?;
        // Find their most recent project via project_members
        let project_row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT project_id FROM project_members WHERE user_id = ? ORDER BY granted_at DESC LIMIT 1"
        )
        .bind(user_id.as_bytes().as_slice())
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((pid_bytes,)) = project_row {
            let project_id = Uuid::from_slice(&pid_bytes)?;
            return Ok((user_id, project_id));
        }
        // No project yet — create one
        let project_id = create_caller_project(pool, user_id, full_name).await?;
        return Ok((user_id, project_id));
    }

    // New user — hash a random password (caller won't use password login)
    let random_pw = Uuid::new_v4().to_string();
    let password_hash = db::services::AuthService::hash_password(&random_pw)
        .unwrap_or_else(|_| format!("!invalid_{}", Uuid::new_v4()));

    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
           VALUES (?, ?, ?, ?, ?, 0, 1)"#,
    )
    .bind(user_id.as_bytes().as_slice())
    .bind(&username)
    .bind(&email)
    .bind(full_name)
    .bind(&password_hash)
    .execute(pool)
    .await?;

    let project_id = create_caller_project(pool, user_id, full_name).await?;

    info!("Created PCG account for caller {}: user={}, project={}", phone, user_id, project_id);
    Ok((user_id, project_id))
}

/// Create a project for a phone caller and add them as owner.
pub(super) async fn create_caller_project(
    pool: &sqlx::SqlitePool,
    user_id: Uuid,
    full_name: &str,
) -> anyhow::Result<Uuid> {
    let project_id = Uuid::new_v4();
    let project_name = format!("{}'s Projects", full_name);
    let git_repo_path = format!("/pcg/callers/{}", project_id);

    sqlx::query(
        r#"INSERT INTO projects (id, name, git_repo_path, created_at, updated_at)
           VALUES (?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"#,
    )
    .bind(project_id.as_bytes().as_slice())
    .bind(&project_name)
    .bind(&git_repo_path)
    .execute(pool)
    .await?;

    // Add as owner in project_members
    let member_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO project_members (id, project_id, user_id, role)
           VALUES (?, ?, ?, 'owner')"#,
    )
    .bind(member_id.as_bytes().as_slice())
    .bind(project_id.as_bytes().as_slice())
    .bind(user_id.as_bytes().as_slice())
    .execute(pool)
    .await?;

    Ok(project_id)
}

// ---------------------------------------------------------------------------
// PCG team phone recognition
// ---------------------------------------------------------------------------

/// Check if this phone number belongs to a PCG team member.
///
/// Reads `PCG_TEAM_PHONES` env var — comma-separated `phone:username` pairs, e.g.
/// `PCG_TEAM_PHONES="+13059847801:admin,+15551112222:Sirak"`
///
/// Returns `(user_id, full_name, is_admin)` when found.
pub(super) async fn lookup_pcg_team_member(
    pool: &sqlx::SqlitePool,
    phone: &str,
) -> Option<(Uuid, String, bool)> {
    let mapping = std::env::var("PCG_TEAM_PHONES").unwrap_or_default();
    if mapping.is_empty() {
        return None;
    }

    // Find the username for this phone
    let username = mapping
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.trim().splitn(2, ':');
            let p = parts.next()?.trim();
            let u = parts.next()?.trim();
            if p == phone.trim() { Some(u.to_string()) } else { None }
        })
        .next()?;

    // Look up user in DB
    let row: Option<(Vec<u8>, String, i64)> = sqlx::query_as(
        "SELECT id, full_name, is_admin FROM users WHERE username = ? LIMIT 1",
    )
    .bind(&username)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    row.and_then(|(id_bytes, full_name, is_admin)| {
        Uuid::from_slice(&id_bytes)
            .ok()
            .map(|uid| (uid, full_name, is_admin != 0))
    })
}

/// Load a compact project + task summary for a PCG team member.
/// Returns a JSON string suitable for inclusion in the LLM context.
pub(super) async fn build_pcg_team_context(pool: &sqlx::SqlitePool, user_id: Uuid) -> String {
    // Load their projects (most recently updated first)
    #[derive(sqlx::FromRow)]
    struct ProjRow {
        name: String,
        description: Option<String>,
    }

    let projects: Vec<ProjRow> = sqlx::query_as(
        r#"SELECT p.name, p.description
           FROM projects p
           JOIN project_members pm ON pm.project_id = p.id
           WHERE pm.user_id = ?
           ORDER BY p.updated_at DESC
           LIMIT 8"#,
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Load their recent active tasks
    #[derive(sqlx::FromRow)]
    struct TaskRow {
        title: String,
        status: String,
        project_name: String,
    }

    let tasks: Vec<TaskRow> = sqlx::query_as(
        r#"SELECT t.title, t.status, p.name as project_name
           FROM tasks t
           JOIN projects p ON p.id = t.project_id
           JOIN project_members pm ON pm.project_id = p.id
           WHERE pm.user_id = ?
             AND t.status NOT IN ('completed', 'cancelled', 'archived')
           ORDER BY t.updated_at DESC
           LIMIT 12"#,
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let projects_json: Vec<serde_json::Value> = projects
        .iter()
        .map(|p| json!({ "name": p.name, "description": p.description }))
        .collect();

    let tasks_json: Vec<serde_json::Value> = tasks
        .iter()
        .map(|t| json!({ "title": t.title, "status": t.status, "project": t.project_name }))
        .collect();

    json!({
        "projects": projects_json,
        "active_tasks": tasks_json,
    })
    .to_string()
}

/// Get a fallback project id (first project in DB).
pub(super) async fn get_fallback_project_id(pool: &sqlx::SqlitePool) -> Option<Uuid> {
    let row: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT id FROM projects ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    row.and_then(|(bytes,)| Uuid::from_slice(&bytes).ok())
}

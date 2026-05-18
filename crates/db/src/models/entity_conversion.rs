use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug)]
pub enum ConversionError {
    Database(sqlx::Error),
    NotFound(String),
    InvalidConversion(String),
}

impl From<sqlx::Error> for ConversionError {
    fn from(e: sqlx::Error) -> Self {
        ConversionError::Database(e)
    }
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::Database(e) => write!(f, "Database error: {e}"),
            ConversionError::NotFound(msg) => write!(f, "Not found: {msg}"),
            ConversionError::InvalidConversion(msg) => write!(f, "Invalid conversion: {msg}"),
        }
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert an Organization into a Client under target_parent_id (org)
pub async fn org_to_client(
    pool: &SqlitePool,
    org_id: Uuid,
    target_org_id: Uuid,
) -> Result<Uuid, ConversionError> {
    let new_client_id = Uuid::new_v4();

    // Read the org
    let org: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT name, slug, description, avatar_url FROM organizations WHERE id = ? AND is_active = 1",
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await?;

    let (name, slug, description, avatar_url) =
        org.ok_or_else(|| ConversionError::NotFound("Organization not found".into()))?;

    // Create client
    sqlx::query(
        r#"INSERT INTO clients (id, organization_id, name, slug, description, logo_url)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(new_client_id)
    .bind(target_org_id)
    .bind(&name)
    .bind(&slug)
    .bind(&description)
    .bind(&avatar_url)
    .execute(pool)
    .await?;

    // Move projects from old org to new client
    sqlx::query(
        "UPDATE projects SET client_id = ?, organization_id = ? WHERE organization_id = ? AND deleted_at IS NULL",
    )
    .bind(new_client_id)
    .bind(target_org_id)
    .bind(org_id)
    .execute(pool)
    .await?;

    // Migrate members: org_members → client_members
    migrate_org_members_to_client(pool, org_id, new_client_id).await?;

    // Soft-deactivate the org
    sqlx::query("UPDATE organizations SET is_active = 0 WHERE id = ?")
        .bind(org_id)
        .execute(pool)
        .await?;

    Ok(new_client_id)
}

/// Convert an Organization into a container Project under target_parent_id (org)
pub async fn org_to_project(
    pool: &SqlitePool,
    org_id: Uuid,
    target_org_id: Uuid,
) -> Result<Uuid, ConversionError> {
    let new_project_id = Uuid::new_v4();

    let org: Option<(String,)> =
        sqlx::query_as("SELECT name FROM organizations WHERE id = ? AND is_active = 1")
            .bind(org_id)
            .fetch_optional(pool)
            .await?;

    let (name,) = org.ok_or_else(|| ConversionError::NotFound("Organization not found".into()))?;

    // Create container project (empty git_repo_path)
    sqlx::query(
        r#"INSERT INTO projects (id, name, git_repo_path, organization_id)
           VALUES (?, ?, '', ?)"#,
    )
    .bind(new_project_id)
    .bind(&name)
    .bind(target_org_id)
    .execute(pool)
    .await?;

    // Reparent org's projects as children of the new container
    sqlx::query(
        "UPDATE projects SET parent_project_id = ?, organization_id = ? WHERE organization_id = ? AND id != ? AND deleted_at IS NULL",
    )
    .bind(new_project_id)
    .bind(target_org_id)
    .bind(org_id)
    .bind(new_project_id)
    .execute(pool)
    .await?;

    // Migrate members
    migrate_org_members_to_project(pool, org_id, new_project_id).await?;

    // Deactivate org
    sqlx::query("UPDATE organizations SET is_active = 0 WHERE id = ?")
        .bind(org_id)
        .execute(pool)
        .await?;

    Ok(new_project_id)
}

/// Convert a Client into an Organization
pub async fn client_to_org(pool: &SqlitePool, client_id: Uuid) -> Result<Uuid, ConversionError> {
    let new_org_id = Uuid::new_v4();

    let client: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT name, slug, description, logo_url FROM clients WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(client_id)
    .fetch_optional(pool)
    .await?;

    let (name, slug, description, logo_url) =
        client.ok_or_else(|| ConversionError::NotFound("Client not found".into()))?;

    // Create org
    sqlx::query(
        r#"INSERT INTO organizations (id, name, slug, description, avatar_url)
           VALUES (?, ?, ?, ?, ?)"#,
    )
    .bind(new_org_id)
    .bind(&name)
    .bind(&slug)
    .bind(&description)
    .bind(&logo_url)
    .execute(pool)
    .await?;

    // Move projects to new org
    sqlx::query(
        "UPDATE projects SET organization_id = ?, client_id = NULL WHERE client_id = ? AND deleted_at IS NULL",
    )
    .bind(new_org_id)
    .bind(client_id)
    .execute(pool)
    .await?;

    // Seed new org KG from client_visible deliverable entries
    let moved_projects: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM projects WHERE organization_id = ? AND deleted_at IS NULL")
            .bind(new_org_id)
            .fetch_all(pool)
            .await?;

    for (project_id,) in &moved_projects {
        sqlx::query(
            r#"INSERT OR IGNORE INTO project_knowledge_sources
               (id, owner_type, owner_id, project_id, source_type, source_id,
                source_title, source_summary, coverage_score, client_visible,
                is_active, is_stale, created_at, updated_at)
               SELECT
                 lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' ||
                 substr(lower(hex(randomblob(2))),2) || '-' ||
                 substr('89ab',abs(random()) % 4 + 1, 1) ||
                 substr(lower(hex(randomblob(2))),2) || '-' ||
                 lower(hex(randomblob(6))),
                 'organization', ?, project_id, source_type, source_id,
                 source_title, source_summary, coverage_score, 1,
                 1, 0, datetime('now','subsec'), datetime('now','subsec')
               FROM project_knowledge_sources
               WHERE project_id = ?
                 AND client_visible = 1
                 AND is_active = 1"#,
        )
        .bind(new_org_id.to_string())
        .bind(project_id)
        .execute(pool)
        .await?;
    }

    // Migrate members
    migrate_client_members_to_org(pool, client_id, new_org_id).await?;

    // Soft delete client
    sqlx::query("UPDATE clients SET deleted_at = datetime('now', 'subsec') WHERE id = ?")
        .bind(client_id)
        .execute(pool)
        .await?;

    Ok(new_org_id)
}

/// Convert a Client into a container Project
pub async fn client_to_project(
    pool: &SqlitePool,
    client_id: Uuid,
    parent_project_id: Option<Uuid>,
) -> Result<Uuid, ConversionError> {
    let new_project_id = Uuid::new_v4();

    let client: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT name, organization_id FROM clients WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(client_id)
    .fetch_optional(pool)
    .await?;

    let (name, org_id_bytes) =
        client.ok_or_else(|| ConversionError::NotFound("Client not found".into()))?;
    let org_id = Uuid::from_slice(&org_id_bytes)
        .map_err(|_| ConversionError::InvalidConversion("Invalid org ID".into()))?;

    // Create container project
    sqlx::query(
        r#"INSERT INTO projects (id, name, git_repo_path, organization_id, parent_project_id)
           VALUES (?, ?, '', ?, ?)"#,
    )
    .bind(new_project_id)
    .bind(&name)
    .bind(org_id)
    .bind(parent_project_id)
    .execute(pool)
    .await?;

    // Reparent client's projects as children
    sqlx::query(
        "UPDATE projects SET parent_project_id = ?, client_id = NULL WHERE client_id = ? AND deleted_at IS NULL",
    )
    .bind(new_project_id)
    .bind(client_id)
    .execute(pool)
    .await?;

    // Migrate members
    migrate_client_members_to_project(pool, client_id, new_project_id).await?;

    // Soft delete client
    sqlx::query("UPDATE clients SET deleted_at = datetime('now', 'subsec') WHERE id = ?")
        .bind(client_id)
        .execute(pool)
        .await?;

    Ok(new_project_id)
}

/// Convert a Project into a Client
pub async fn project_to_client(
    pool: &SqlitePool,
    project_id: Uuid,
) -> Result<Uuid, ConversionError> {
    let new_client_id = Uuid::new_v4();

    let project: Option<(String, Option<Vec<u8>>)> = sqlx::query_as(
        "SELECT name, organization_id FROM projects WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    let (name, org_id_bytes) =
        project.ok_or_else(|| ConversionError::NotFound("Project not found".into()))?;
    let org_id = org_id_bytes
        .and_then(|b| Uuid::from_slice(&b).ok())
        .ok_or_else(|| {
            ConversionError::InvalidConversion("Project must belong to an organization".into())
        })?;

    let slug = slugify(&name);

    // Create client
    sqlx::query(
        r#"INSERT INTO clients (id, organization_id, name, slug)
           VALUES (?, ?, ?, ?)"#,
    )
    .bind(new_client_id)
    .bind(org_id)
    .bind(&name)
    .bind(&slug)
    .execute(pool)
    .await?;

    // Move children to client
    sqlx::query(
        "UPDATE projects SET client_id = ?, parent_project_id = NULL WHERE parent_project_id = ? AND deleted_at IS NULL",
    )
    .bind(new_client_id)
    .bind(project_id)
    .execute(pool)
    .await?;

    // Migrate members
    migrate_project_members_to_client(pool, project_id, new_client_id).await?;

    // Soft delete project
    sqlx::query("UPDATE projects SET deleted_at = datetime('now', 'subsec') WHERE id = ?")
        .bind(project_id)
        .execute(pool)
        .await?;

    Ok(new_client_id)
}

/// Convert a Project into an Organization
pub async fn project_to_org(pool: &SqlitePool, project_id: Uuid) -> Result<Uuid, ConversionError> {
    let new_org_id = Uuid::new_v4();

    let project: Option<(String,)> =
        sqlx::query_as("SELECT name FROM projects WHERE id = ? AND deleted_at IS NULL")
            .bind(project_id)
            .fetch_optional(pool)
            .await?;

    let (name,) = project.ok_or_else(|| ConversionError::NotFound("Project not found".into()))?;
    let slug = slugify(&name);

    // Create org
    sqlx::query(
        r#"INSERT INTO organizations (id, name, slug)
           VALUES (?, ?, ?)"#,
    )
    .bind(new_org_id)
    .bind(&name)
    .bind(&slug)
    .execute(pool)
    .await?;

    // Move children to new org as top-level
    sqlx::query(
        "UPDATE projects SET organization_id = ?, parent_project_id = NULL, client_id = NULL WHERE parent_project_id = ? AND deleted_at IS NULL",
    )
    .bind(new_org_id)
    .bind(project_id)
    .execute(pool)
    .await?;

    // Migrate members
    migrate_project_members_to_org(pool, project_id, new_org_id).await?;

    // Soft delete project
    sqlx::query("UPDATE projects SET deleted_at = datetime('now', 'subsec') WHERE id = ?")
        .bind(project_id)
        .execute(pool)
        .await?;

    Ok(new_org_id)
}

// ============================================================================
// Member migration helpers
// ============================================================================

async fn migrate_org_members_to_client(
    pool: &SqlitePool,
    org_id: Uuid,
    client_id: Uuid,
) -> Result<(), ConversionError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Vec<u8>,
        role: String,
    }

    let members: Vec<MemberRow> =
        sqlx::query_as("SELECT user_id, role FROM organization_members WHERE organization_id = ?")
            .bind(org_id)
            .fetch_all(pool)
            .await?;

    for m in &members {
        let id = Uuid::new_v4();
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO client_members (id, client_id, user_id, role, granted_by) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(client_id)
        .bind(&m.user_id)
        .bind(&m.role)
        .bind(&m.user_id)
        .execute(pool)
        .await;
    }
    Ok(())
}

async fn migrate_org_members_to_project(
    pool: &SqlitePool,
    org_id: Uuid,
    project_id: Uuid,
) -> Result<(), ConversionError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Vec<u8>,
        role: String,
    }

    let members: Vec<MemberRow> =
        sqlx::query_as("SELECT user_id, role FROM organization_members WHERE organization_id = ?")
            .bind(org_id)
            .fetch_all(pool)
            .await?;

    let project_id_str = project_id.to_string();
    for m in &members {
        let id = Uuid::new_v4();
        let role = if m.role == "admin" { "owner" } else { "member" };
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&project_id_str)
        .bind(&m.user_id)
        .bind(role)
        .bind(&m.user_id)
        .execute(pool)
        .await;
    }
    Ok(())
}

async fn migrate_client_members_to_org(
    pool: &SqlitePool,
    client_id: Uuid,
    org_id: Uuid,
) -> Result<(), ConversionError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Vec<u8>,
        role: String,
    }

    let members: Vec<MemberRow> =
        sqlx::query_as("SELECT user_id, role FROM client_members WHERE client_id = ?")
            .bind(client_id)
            .fetch_all(pool)
            .await?;

    for m in &members {
        let id = Uuid::new_v4();
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(org_id)
        .bind(&m.user_id)
        .bind(&m.role)
        .execute(pool)
        .await;
    }
    Ok(())
}

async fn migrate_client_members_to_project(
    pool: &SqlitePool,
    client_id: Uuid,
    project_id: Uuid,
) -> Result<(), ConversionError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Vec<u8>,
        role: String,
    }

    let members: Vec<MemberRow> =
        sqlx::query_as("SELECT user_id, role FROM client_members WHERE client_id = ?")
            .bind(client_id)
            .fetch_all(pool)
            .await?;

    let project_id_str = project_id.to_string();
    for m in &members {
        let id = Uuid::new_v4();
        let role = if m.role == "admin" { "owner" } else { "member" };
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&project_id_str)
        .bind(&m.user_id)
        .bind(role)
        .bind(&m.user_id)
        .execute(pool)
        .await;
    }
    Ok(())
}

async fn migrate_project_members_to_client(
    pool: &SqlitePool,
    project_id: Uuid,
    client_id: Uuid,
) -> Result<(), ConversionError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Vec<u8>,
        role: String,
    }

    let project_id_str = project_id.to_string();
    let members: Vec<MemberRow> =
        sqlx::query_as("SELECT user_id, role FROM project_members WHERE project_id = ?")
            .bind(&project_id_str)
            .fetch_all(pool)
            .await?;

    for m in &members {
        let id = Uuid::new_v4();
        let role = if m.role == "owner" { "admin" } else { &m.role };
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO client_members (id, client_id, user_id, role, granted_by) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(client_id)
        .bind(&m.user_id)
        .bind(role)
        .bind(&m.user_id)
        .execute(pool)
        .await;
    }
    Ok(())
}

async fn migrate_project_members_to_org(
    pool: &SqlitePool,
    project_id: Uuid,
    org_id: Uuid,
) -> Result<(), ConversionError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Vec<u8>,
        role: String,
    }

    let project_id_str = project_id.to_string();
    let members: Vec<MemberRow> =
        sqlx::query_as("SELECT user_id, role FROM project_members WHERE project_id = ?")
            .bind(&project_id_str)
            .fetch_all(pool)
            .await?;

    for m in &members {
        let id = Uuid::new_v4();
        let role = if m.role == "owner" { "admin" } else { &m.role };
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(org_id)
        .bind(&m.user_id)
        .bind(role)
        .execute(pool)
        .await;
    }
    Ok(())
}

use axum::{
    Extension, Json, Router,
    extract::State,
    routing::get,
};
use deployment::Deployment;
use serde::Serialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use db::models::project_knowledge_source::{ProjectKnowledgeSource, ProjectHealthSummary};

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::access_control::AccessContext,
};

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarTree {
    pub owned_orgs: Vec<SidebarOrg>,
    pub member_orgs: Vec<SidebarOrg>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarOrg {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub role: String,
    pub health_status: Option<String>,
    pub active_issues_count: Option<i64>,
    pub knowledge_completeness: Option<f64>,
    pub last_activity_at: Option<String>,
    pub internal_projects: Vec<SidebarProject>,
    pub internal_folders: Vec<SidebarProjectFolder>,
    pub clients: Vec<SidebarClient>,
    pub shared_boards: Vec<SidebarSharedBoardGroup>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarProjectFolder {
    pub id: String,
    pub name: String,
    pub projects: Vec<SidebarProject>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarSharedBoardGroup {
    pub source_org_id: String,
    pub source_org_name: String,
    pub share_type: String,
    pub boards: Vec<SidebarSharedBoard>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarSharedBoard {
    pub board_id: String,
    pub board_name: String,
    pub project_id: String,
    pub project_name: String,
    pub permission: String,
    pub share_type: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarClient {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub health_status: Option<String>,
    pub active_issues_count: Option<i64>,
    pub knowledge_completeness: Option<f64>,
    pub last_activity_at: Option<String>,
    pub projects: Vec<SidebarProject>,
    pub folders: Vec<SidebarProjectFolder>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarProject {
    pub id: String,
    pub name: String,
    pub health_status: Option<String>,
    pub active_issues_count: Option<i64>,
    pub knowledge_completeness: Option<f64>,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct OrgRow {
    id: Vec<u8>,
    name: String,
    slug: String,
    role: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ClientRow {
    id: Vec<u8>,
    name: String,
    slug: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectRow {
    id: Vec<u8>,
    name: String,
    client_id: Option<Vec<u8>>,
    folder_id: Option<Vec<u8>>,
}

#[derive(Debug, sqlx::FromRow)]
struct FolderRow {
    id: Vec<u8>,
    name: String,
    client_id: Option<Vec<u8>>,
}

#[derive(Debug, sqlx::FromRow)]
struct SharedBoardRow {
    board_id: Vec<u8>,
    board_name: String,
    project_id: Vec<u8>,
    project_name: String,
    source_org_id: Vec<u8>,
    source_org_name: String,
    permission: String,
    share_type: String,
}

fn uuid_from_bytes(bytes: &[u8]) -> Option<Uuid> {
    Uuid::from_slice(bytes).ok()
}

/// Rollup health across a collection of projects:
/// - worst health_status wins (critical > warning > healthy)
/// - sum issue counts
/// - average knowledge_completeness
/// - most recent last_activity_at
fn rollup_health(
    projects: &[&SidebarProject],
) -> (Option<String>, Option<i64>, Option<f64>, Option<String>) {
    if projects.is_empty() {
        return (None, None, None, None);
    }

    let mut total_issues: i64 = 0;
    let mut kc_sum: f64 = 0.0;
    let mut kc_count: usize = 0;
    let mut worst: u8 = 0; // 0=healthy, 1=warning, 2=critical
    let mut latest_activity: Option<String> = None;

    for p in projects {
        if let Some(ref status) = p.health_status {
            let level = match status.as_str() {
                "critical" => 2,
                "warning" => 1,
                _ => 0,
            };
            if level > worst {
                worst = level;
            }
        }
        if let Some(count) = p.active_issues_count {
            total_issues += count;
        }
        if let Some(kc) = p.knowledge_completeness {
            kc_sum += kc;
            kc_count += 1;
        }
        if let Some(ref act) = p.last_activity_at {
            if latest_activity.as_ref().map_or(true, |la| act > la) {
                latest_activity = Some(act.clone());
            }
        }
    }

    let status = match worst {
        2 => "critical",
        1 => "warning",
        _ => "healthy",
    };
    let avg_kc = if kc_count > 0 {
        Some(kc_sum / kc_count as f64)
    } else {
        None
    };

    (
        Some(status.to_string()),
        Some(total_issues),
        avg_kc,
        latest_activity,
    )
}

/// GET /api/sidebar/tree
pub async fn get_sidebar_tree(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<SidebarTree>>, ApiError> {
    let pool = &deployment.db().pool;
    let user_id = access_context.user_id;
    let user_id_bytes = user_id.as_bytes().to_vec();

    // For admin: get ALL organizations. For regular users: only orgs they belong to.
    let org_rows: Vec<OrgRow> = if access_context.is_admin {
        sqlx::query_as::<_, OrgRow>(
            r#"SELECT o.id, o.name, o.slug,
                      COALESCE(om.role, 'admin') as role
               FROM organizations o
               LEFT JOIN organization_members om ON om.organization_id = o.id AND om.user_id = ?
               WHERE o.is_active = 1
               ORDER BY o.name ASC"#,
        )
        .bind(&user_id_bytes)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch orgs: {}", e)))?
    } else {
        sqlx::query_as::<_, OrgRow>(
            r#"SELECT o.id, o.name, o.slug, om.role
               FROM organizations o
               INNER JOIN organization_members om ON om.organization_id = o.id
               WHERE om.user_id = ? AND o.is_active = 1
               ORDER BY o.name ASC"#,
        )
        .bind(&user_id_bytes)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch orgs: {}", e)))?
    };

    let mut owned_orgs = Vec::new();
    let mut member_orgs = Vec::new();

    for org_row in &org_rows {
        let org_id = match uuid_from_bytes(&org_row.id) {
            Some(id) => id,
            None => continue,
        };
        let org_id_bytes = org_id.as_bytes().to_vec();

        // Get projects for this org (now including folder_id)
        let project_rows: Vec<ProjectRow> = if access_context.is_admin {
            sqlx::query_as::<_, ProjectRow>(
                r#"SELECT id, name, client_id, folder_id FROM projects WHERE organization_id = ? AND deleted_at IS NULL ORDER BY name ASC"#,
            )
            .bind(&org_id_bytes)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        } else {
            sqlx::query_as::<_, ProjectRow>(
                r#"SELECT DISTINCT p.id, p.name, p.client_id, p.folder_id FROM projects p
                   WHERE p.organization_id = ? AND p.deleted_at IS NULL AND (
                       p.id IN (SELECT pm.project_id FROM project_members pm WHERE pm.user_id = ?)
                       OR p.organization_id IN (SELECT om.organization_id FROM organization_members om WHERE om.user_id = ?)
                       OR p.client_id IN (SELECT cm.client_id FROM client_members cm WHERE cm.user_id = ?)
                   )
                   ORDER BY p.name ASC"#,
            )
            .bind(&org_id_bytes)
            .bind(&user_id_bytes)
            .bind(&user_id_bytes)
            .bind(&user_id_bytes)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        };

        // Get clients for this org
        let client_rows: Vec<ClientRow> = sqlx::query_as::<_, ClientRow>(
            r#"SELECT id, name, slug FROM clients
               WHERE organization_id = ? AND deleted_at IS NULL AND is_active = 1
               ORDER BY name ASC"#,
        )
        .bind(&org_id_bytes)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // Get folders for this org
        let folder_rows: Vec<FolderRow> = sqlx::query_as::<_, FolderRow>(
            r#"SELECT id, name, client_id FROM project_folders
               WHERE organization_id = ? AND is_active = 1
               ORDER BY sort_order ASC, name ASC"#,
        )
        .bind(&org_id_bytes)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // Batch fetch health data for all projects in this org
        let all_project_ids: Vec<Vec<u8>> = project_rows.iter().map(|p| p.id.clone()).collect();
        let health_map = ProjectKnowledgeSource::get_health_batch(pool, &all_project_ids)
            .await
            .unwrap_or_default();

        // Helper to build SidebarProject with health data
        let make_sidebar_proj = |proj: &ProjectRow| -> Option<SidebarProject> {
            let proj_uuid = uuid_from_bytes(&proj.id)?;
            let proj_id = proj_uuid.to_string();
            let health = health_map.get(&proj_id);
            Some(SidebarProject {
                id: proj_id,
                name: proj.name.clone(),
                health_status: health.map(|h| h.health_status.clone()),
                active_issues_count: health.map(|h| h.active_issues_count),
                knowledge_completeness: health.map(|h| h.knowledge_completeness),
                last_activity_at: health.and_then(|h| h.last_activity_at.clone()),
            })
        };

        // Separate projects by client_id and folder_id
        let mut internal_loose_projects: Vec<SidebarProject> = Vec::new();
        let mut internal_folder_projects: std::collections::HashMap<Vec<u8>, Vec<SidebarProject>> =
            std::collections::HashMap::new();
        let mut client_loose_projects: std::collections::HashMap<Vec<u8>, Vec<SidebarProject>> =
            std::collections::HashMap::new();
        let mut client_folder_projects: std::collections::HashMap<Vec<u8>, Vec<SidebarProject>> =
            std::collections::HashMap::new();

        for proj in &project_rows {
            let sidebar_proj = match make_sidebar_proj(proj) {
                Some(sp) => sp,
                None => continue,
            };

            match (&proj.client_id, &proj.folder_id) {
                (Some(cid), Some(fid)) => {
                    // Client project in a folder
                    client_folder_projects
                        .entry(fid.clone())
                        .or_default()
                        .push(sidebar_proj);
                    // Also ensure client gets a reference even if empty later
                    client_loose_projects.entry(cid.clone()).or_default();
                }
                (Some(cid), None) => {
                    // Client project, no folder
                    client_loose_projects
                        .entry(cid.clone())
                        .or_default()
                        .push(sidebar_proj);
                }
                (None, Some(fid)) => {
                    // Internal project in a folder
                    internal_folder_projects
                        .entry(fid.clone())
                        .or_default()
                        .push(sidebar_proj);
                }
                (None, None) => {
                    // Internal project, no folder
                    internal_loose_projects.push(sidebar_proj);
                }
            }
        }

        // Build internal folders (folders with no client_id)
        let internal_folders: Vec<SidebarProjectFolder> = folder_rows
            .iter()
            .filter(|f| f.client_id.is_none())
            .filter_map(|f| {
                let fid_str = uuid_from_bytes(&f.id)?.to_string();
                let projects = internal_folder_projects
                    .remove(&f.id)
                    .unwrap_or_default();
                Some(SidebarProjectFolder {
                    id: fid_str,
                    name: f.name.clone(),
                    projects,
                })
            })
            .collect();

        // Build sidebar clients with folders
        let clients: Vec<SidebarClient> = client_rows
            .iter()
            .map(|cr| {
                let projects = client_loose_projects
                    .remove(&cr.id)
                    .unwrap_or_default();

                // Get folders belonging to this client
                let folders: Vec<SidebarProjectFolder> = folder_rows
                    .iter()
                    .filter(|f| f.client_id.as_ref() == Some(&cr.id))
                    .filter_map(|f| {
                        let fid_str = uuid_from_bytes(&f.id)?.to_string();
                        let folder_projects = client_folder_projects
                            .remove(&f.id)
                            .unwrap_or_default();
                        Some(SidebarProjectFolder {
                            id: fid_str,
                            name: f.name.clone(),
                            projects: folder_projects,
                        })
                    })
                    .collect();

                // Rollup health for client: worst status, summed issues, avg knowledge
                let all_client_projects: Vec<&SidebarProject> = projects.iter()
                    .chain(folders.iter().flat_map(|f| f.projects.iter()))
                    .collect();
                let (client_health, client_issues, client_kc, client_activity) =
                    rollup_health(&all_client_projects);

                SidebarClient {
                    id: uuid_from_bytes(&cr.id)
                        .map(|u| u.to_string())
                        .unwrap_or_default(),
                    name: cr.name.clone(),
                    slug: cr.slug.clone(),
                    health_status: client_health,
                    active_issues_count: client_issues,
                    knowledge_completeness: client_kc,
                    last_activity_at: client_activity,
                    projects,
                    folders,
                }
            })
            .collect();

        // Query boards shared TO this org
        let shared_board_rows: Vec<SharedBoardRow> = sqlx::query_as::<_, SharedBoardRow>(
            r#"SELECT pb.id as board_id, pb.name as board_name,
                      p.id as project_id, p.name as project_name,
                      src_org.id as source_org_id, src_org.name as source_org_name,
                      bs.permission, bs.share_type
               FROM board_shares bs
               JOIN project_boards pb ON pb.id = bs.board_id
               JOIN projects p ON p.id = pb.project_id
               JOIN organizations src_org ON src_org.id = bs.source_organization_id
               WHERE bs.target_organization_id = ? AND bs.is_active = 1
               ORDER BY src_org.name, pb.name"#,
        )
        .bind(&org_id_bytes)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // Group shared boards by source org
        let mut shared_groups: std::collections::HashMap<Vec<u8>, SidebarSharedBoardGroup> =
            std::collections::HashMap::new();

        for row in &shared_board_rows {
            let group = shared_groups
                .entry(row.source_org_id.clone())
                .or_insert_with(|| SidebarSharedBoardGroup {
                    source_org_id: uuid_from_bytes(&row.source_org_id)
                        .map(|u| u.to_string())
                        .unwrap_or_default(),
                    source_org_name: row.source_org_name.clone(),
                    share_type: row.share_type.clone(),
                    boards: Vec::new(),
                });

            if let (Some(bid), Some(pid)) = (
                uuid_from_bytes(&row.board_id),
                uuid_from_bytes(&row.project_id),
            ) {
                group.boards.push(SidebarSharedBoard {
                    board_id: bid.to_string(),
                    board_name: row.board_name.clone(),
                    project_id: pid.to_string(),
                    project_name: row.project_name.clone(),
                    permission: row.permission.clone(),
                    share_type: row.share_type.clone(),
                });
            }
        }

        let shared_boards: Vec<SidebarSharedBoardGroup> = shared_groups.into_values().collect();

        // Rollup health for org: worst status across all projects
        let all_org_projects: Vec<&SidebarProject> = internal_loose_projects.iter()
            .chain(internal_folders.iter().flat_map(|f| f.projects.iter()))
            .chain(clients.iter().flat_map(|c| {
                c.projects.iter()
                    .chain(c.folders.iter().flat_map(|f| f.projects.iter()))
            }))
            .collect();
        let (org_health, org_issues, org_kc, org_activity) = rollup_health(&all_org_projects);

        let sidebar_org = SidebarOrg {
            id: org_id.to_string(),
            name: org_row.name.clone(),
            slug: org_row.slug.clone(),
            role: org_row.role.clone(),
            health_status: org_health,
            active_issues_count: org_issues,
            knowledge_completeness: org_kc,
            last_activity_at: org_activity,
            internal_projects: internal_loose_projects,
            internal_folders,
            clients,
            shared_boards,
        };

        if org_row.role == "admin" {
            owned_orgs.push(sidebar_org);
        } else {
            member_orgs.push(sidebar_org);
        }
    }

    // For non-admin users, also include projects not linked to any org
    // (orphaned projects they have direct access to)
    if !access_context.is_admin {
        let orphan_projects: Vec<ProjectRow> = sqlx::query_as::<_, ProjectRow>(
            r#"SELECT DISTINCT p.id, p.name, p.client_id, p.folder_id FROM projects p
               INNER JOIN project_members pm ON pm.project_id = p.id
               WHERE pm.user_id = ? AND p.organization_id IS NULL AND p.deleted_at IS NULL
               ORDER BY p.name ASC"#,
        )
        .bind(&user_id_bytes)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        if !orphan_projects.is_empty() {
            let orphan_ids: Vec<Vec<u8>> = orphan_projects.iter().map(|p| p.id.clone()).collect();
            let orphan_health = ProjectKnowledgeSource::get_health_batch(pool, &orphan_ids)
                .await
                .unwrap_or_default();

            let internal_projects: Vec<SidebarProject> = orphan_projects
                .iter()
                .filter_map(|p| {
                    let uid = uuid_from_bytes(&p.id)?;
                    let pid = uid.to_string();
                    let health = orphan_health.get(&pid);
                    Some(SidebarProject {
                        id: pid,
                        name: p.name.clone(),
                        health_status: health.map(|h| h.health_status.clone()),
                        active_issues_count: health.map(|h| h.active_issues_count),
                        knowledge_completeness: health.map(|h| h.knowledge_completeness),
                        last_activity_at: health.and_then(|h| h.last_activity_at.clone()),
                    })
                })
                .collect();

            let (orph_health, orph_issues, orph_kc, orph_activity) =
                rollup_health(&internal_projects.iter().collect::<Vec<_>>());

            member_orgs.push(SidebarOrg {
                id: String::new(),
                name: "Unassigned".into(),
                slug: "unassigned".into(),
                role: "member".into(),
                health_status: orph_health,
                active_issues_count: orph_issues,
                knowledge_completeness: orph_kc,
                last_activity_at: orph_activity,
                internal_projects,
                internal_folders: vec![],
                clients: vec![],
                shared_boards: vec![],
            });
        }
    }

    Ok(Json(ApiResponse::success(SidebarTree {
        owned_orgs,
        member_orgs,
    })))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/sidebar/tree", get(get_sidebar_tree))
}

use axum::{Extension, Json, Router, extract::State, routing::get};
use db::models::project_knowledge_source::{ProjectHealthSummary, ProjectKnowledgeSource};
use deployment::Deployment;
use serde::Serialize;
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

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
    pub clients: Vec<SidebarClient>,
    pub shared_boards: Vec<SidebarSharedBoardGroup>,
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
    pub crm_person_id: Option<String>,
    pub crm_confidence: Option<f64>,
    pub projects: Vec<SidebarProject>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SidebarProject {
    pub id: String,
    pub name: String,
    pub is_container: bool,
    pub children: Vec<SidebarProject>,
    pub health_status: Option<String>,
    pub active_issues_count: Option<i64>,
    pub knowledge_completeness: Option<f64>,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct OrgRow {
    id: String,
    name: String,
    slug: String,
    role: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ClientRow {
    id: String,
    name: String,
    slug: String,
    crm_contact_id: Option<String>,
    crm_confidence: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
struct ProjectRow {
    id: String,
    name: String,
    git_repo_path: String,
    client_id: Option<String>,
    parent_project_id: Option<String>,
    sort_order: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct SharedBoardRow {
    board_id: String,
    board_name: String,
    project_id: String,
    project_name: String,
    source_org_id: String,
    source_org_name: String,
    permission: String,
    share_type: String,
}

/// Collect all SidebarProject references recursively (for health rollup)
fn collect_all_projects(projects: &[SidebarProject]) -> Vec<&SidebarProject> {
    let mut result = Vec::new();
    for p in projects {
        result.push(p);
        result.extend(collect_all_projects(&p.children));
    }
    result
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
            if latest_activity.as_ref().is_none_or(|la| act > la) {
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

/// Build a recursive project tree from a flat list of ProjectRow items.
/// Returns top-level projects (those with parent_project_id matching the given filter).
fn build_project_tree(
    project_rows: &[ProjectRow],
    health_map: &std::collections::HashMap<String, ProjectHealthSummary>,
    parent_id: Option<&str>,
) -> Vec<SidebarProject> {
    let mut projects: Vec<SidebarProject> = project_rows
        .iter()
        .filter(|p| p.parent_project_id.as_deref() == parent_id)
        .map(|proj| {
            let health = health_map.get(&proj.id);
            let children = build_project_tree(project_rows, health_map, Some(&proj.id));
            let is_container =
                proj.git_repo_path.is_empty() || proj.git_repo_path.starts_with("container:");
            SidebarProject {
                id: proj.id.clone(),
                name: proj.name.clone(),
                is_container,
                children,
                health_status: health.map(|h| h.health_status.clone()),
                active_issues_count: health.map(|h| h.active_issues_count),
                knowledge_completeness: health.map(|h| h.knowledge_completeness),
                last_activity_at: health.and_then(|h| h.last_activity_at.clone()),
            }
        })
        .collect();
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

/// GET /api/sidebar/tree
pub async fn get_sidebar_tree(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<SidebarTree>>, ApiError> {
    let pool = &deployment.db().pool;
    let user_id = &access_context.user_id;

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
        .bind(user_id)
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
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch orgs: {}", e)))?
    };

    let mut owned_orgs = Vec::new();
    let mut member_orgs = Vec::new();

    for org_row in &org_rows {
        let org_id_str = &org_row.id;

        // Get projects for this org (with parent_project_id for tree building)
        let project_rows: Vec<ProjectRow> = if access_context.is_admin {
            sqlx::query_as::<_, ProjectRow>(
                r#"SELECT id, name, git_repo_path, client_id, parent_project_id, sort_order
                   FROM projects WHERE organization_id = ? AND deleted_at IS NULL
                   ORDER BY sort_order ASC, name ASC"#,
            )
            .bind(org_id_str)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        } else if org_row.role == "admin" {
            // Org admins see all projects in the org
            sqlx::query_as::<_, ProjectRow>(
                r#"SELECT id, name, git_repo_path, client_id, parent_project_id, sort_order
                   FROM projects WHERE organization_id = ? AND deleted_at IS NULL
                   ORDER BY sort_order ASC, name ASC"#,
            )
            .bind(org_id_str)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        } else {
            // Regular org members: only projects they're explicitly assigned to,
            // or client projects where they have a task assignment
            sqlx::query_as::<_, ProjectRow>(
                r#"SELECT DISTINCT p.id, p.name, p.git_repo_path, p.client_id, p.parent_project_id, p.sort_order
                   FROM projects p
                   WHERE p.organization_id = ? AND p.deleted_at IS NULL AND (
                       p.id IN (SELECT pm.project_id FROM project_members pm WHERE pm.user_id = ?)
                       OR (p.client_id IN (SELECT cm.client_id FROM client_members cm WHERE cm.user_id = ?)
                           AND p.id IN (SELECT t.project_id FROM tasks t WHERE t.assignee_id = ? AND t.deleted_at IS NULL))
                   )
                   ORDER BY p.sort_order ASC, p.name ASC"#,
            )
            .bind(org_id_str)
            .bind(user_id)
            .bind(user_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        };

        // Get clients for this org
        // Admins (platform or org) see all clients; regular members see only assigned clients
        let client_rows: Vec<ClientRow> = if access_context.is_admin || org_row.role == "admin" {
            sqlx::query_as::<_, ClientRow>(
                r#"SELECT c.id, c.name, c.slug, c.crm_contact_id,
                          p.intelligence_confidence as crm_confidence
                   FROM clients c
                   LEFT JOIN persons p ON p.id = c.crm_contact_id
                   WHERE c.organization_id = ? AND c.deleted_at IS NULL AND c.is_active = 1
                   ORDER BY c.name ASC"#,
            )
            .bind(org_id_str)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        } else {
            sqlx::query_as::<_, ClientRow>(
                r#"SELECT c.id, c.name, c.slug, c.crm_contact_id,
                          p.intelligence_confidence as crm_confidence
                   FROM clients c
                   INNER JOIN client_members cm ON cm.client_id = c.id AND cm.user_id = ?
                   LEFT JOIN persons p ON p.id = c.crm_contact_id
                   WHERE c.organization_id = ? AND c.deleted_at IS NULL AND c.is_active = 1
                   ORDER BY c.name ASC"#,
            )
            .bind(user_id)
            .bind(org_id_str)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        };

        // Batch fetch health data for all projects in this org
        let all_project_ids: Vec<String> = project_rows.iter().map(|p| p.id.clone()).collect();
        let health_map = ProjectKnowledgeSource::get_health_batch(pool, &all_project_ids)
            .await
            .unwrap_or_default();

        // Split projects by client_id for tree building
        let internal_project_rows: Vec<ProjectRow> = project_rows
            .iter()
            .filter(|p| p.client_id.is_none())
            .cloned()
            .collect();
        let internal_projects = build_project_tree(&internal_project_rows, &health_map, None);

        // Build sidebar clients with nested project trees
        let clients: Vec<SidebarClient> = client_rows
            .iter()
            .map(|cr| {
                let client_project_rows: Vec<ProjectRow> = project_rows
                    .iter()
                    .filter(|p| p.client_id.as_ref() == Some(&cr.id))
                    .cloned()
                    .collect();
                let projects = build_project_tree(&client_project_rows, &health_map, None);

                // Rollup health for client
                let all_client_projects = collect_all_projects(&projects);
                let (client_health, client_issues, client_kc, client_activity) =
                    rollup_health(&all_client_projects);

                SidebarClient {
                    id: cr.id.clone(),
                    name: cr.name.clone(),
                    slug: cr.slug.clone(),
                    health_status: client_health,
                    active_issues_count: client_issues,
                    knowledge_completeness: client_kc,
                    last_activity_at: client_activity,
                    crm_person_id: cr.crm_contact_id.clone(),
                    crm_confidence: cr.crm_confidence,
                    projects,
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
        .bind(org_id_str)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // Group shared boards by source org
        let mut shared_groups: std::collections::HashMap<String, SidebarSharedBoardGroup> =
            std::collections::HashMap::new();

        for row in &shared_board_rows {
            let group = shared_groups
                .entry(row.source_org_id.clone())
                .or_insert_with(|| SidebarSharedBoardGroup {
                    source_org_id: row.source_org_id.clone(),
                    source_org_name: row.source_org_name.clone(),
                    share_type: row.share_type.clone(),
                    boards: Vec::new(),
                });

            group.boards.push(SidebarSharedBoard {
                board_id: row.board_id.clone(),
                board_name: row.board_name.clone(),
                project_id: row.project_id.clone(),
                project_name: row.project_name.clone(),
                permission: row.permission.clone(),
                share_type: row.share_type.clone(),
            });
        }

        let shared_boards: Vec<SidebarSharedBoardGroup> = shared_groups.into_values().collect();

        // Rollup health for org: worst status across all projects
        let all_org_projects: Vec<&SidebarProject> = collect_all_projects(&internal_projects)
            .into_iter()
            .chain(
                clients
                    .iter()
                    .flat_map(|c| collect_all_projects(&c.projects)),
            )
            .collect();
        let (org_health, org_issues, org_kc, org_activity) = rollup_health(&all_org_projects);

        let sidebar_org = SidebarOrg {
            id: org_id_str.clone(),
            name: org_row.name.clone(),
            slug: org_row.slug.clone(),
            role: org_row.role.clone(),
            health_status: org_health,
            active_issues_count: org_issues,
            knowledge_completeness: org_kc,
            last_activity_at: org_activity,
            internal_projects,
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
            r#"SELECT DISTINCT p.id, p.name, p.git_repo_path, p.client_id, p.parent_project_id, p.sort_order
               FROM projects p
               INNER JOIN project_members pm ON pm.project_id = p.id
               WHERE pm.user_id = ? AND p.organization_id IS NULL AND p.deleted_at IS NULL
               ORDER BY p.sort_order ASC, p.name ASC"#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        if !orphan_projects.is_empty() {
            let orphan_ids: Vec<String> = orphan_projects.iter().map(|p| p.id.clone()).collect();
            let orphan_health = ProjectKnowledgeSource::get_health_batch(pool, &orphan_ids)
                .await
                .unwrap_or_default();

            let internal_projects = build_project_tree(&orphan_projects, &orphan_health, None);

            let all_projects = collect_all_projects(&internal_projects);
            let (orph_health, orph_issues, orph_kc, orph_activity) = rollup_health(&all_projects);

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

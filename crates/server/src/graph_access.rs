//! Batch access filter for entity-graph nodes.
//!
//! The graph endpoints return up to ~2k nodes per request. The existing
//! per-resource access helpers (`require_org_membership`, `require_viewer`,
//! etc.) are designed for one-resource-per-endpoint and would cause an
//! N-queries-per-request blow-up across a union of entity types.
//!
//! `filter_visible_nodes` resolves the user's org memberships once, then
//! applies per-node-type rules in-memory against the candidate set that
//! informs each rule (e.g., the user-visible companies).
//!
//! Rules mirror the **Access-control matrix** section of
//! `planning/2026-04-16--plan--graph-viz-enhancement.md`.

use std::collections::HashSet;

use db::models::entity_graph::{EntityGraphEdge, EntityGraphNode, EntitySubgraph};
use sqlx::SqlitePool;

use crate::middleware::access_control::AccessContext;

/// Ref-id sets the current user can reach, indexed by node_type.
/// Computed once per request; used by `is_visible` to decide each node.
#[derive(Default)]
struct VisibilityScope {
    org_ids: HashSet<String>,
    company_refs: HashSet<String>,
    public_company_refs: HashSet<String>,
    contact_refs: HashSet<String>,
    proposal_refs: HashSet<String>,
    public_proposal_refs: HashSet<String>,
    project_refs: HashSet<String>,
    client_refs: HashSet<String>,
    deal_refs: HashSet<String>,
    pipeline_refs: HashSet<String>,
    pipeline_stage_refs: HashSet<String>,
    brand_profile_refs: HashSet<String>,
    knowledge_source_refs: HashSet<String>,
}

/// Filter a subgraph in place, returning only nodes the user is allowed to
/// see plus edges whose endpoints are both visible. Admins see everything.
pub async fn filter_visible_nodes(
    ctx: &AccessContext,
    pool: &SqlitePool,
    subgraph: EntitySubgraph,
) -> Result<EntitySubgraph, sqlx::Error> {
    if ctx.is_admin {
        return Ok(subgraph);
    }

    let scope = resolve_visibility_scope(pool, ctx.user_id.as_str()).await?;

    let nodes: Vec<EntityGraphNode> = subgraph
        .nodes
        .into_iter()
        .filter(|n| is_visible(n, &scope))
        .collect();

    let node_id_set: HashSet<uuid::Uuid> = nodes.iter().map(|n| n.id).collect();
    let edges: Vec<EntityGraphEdge> = subgraph
        .edges
        .into_iter()
        .filter(|e| node_id_set.contains(&e.from_node_id) && node_id_set.contains(&e.to_node_id))
        .collect();

    Ok(EntitySubgraph { nodes, edges })
}

async fn resolve_visibility_scope(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<VisibilityScope, sqlx::Error> {
    let user_orgs = list_user_orgs(pool, user_id).await?;

    // Directly-org-scoped entity tables (have organization_id column).
    let company_refs = fetch_refs_by_orgs(pool, "companies", &user_orgs).await?;
    let contact_refs = fetch_refs_by_orgs(pool, "crm_contacts", &user_orgs).await?;
    let proposal_refs = fetch_refs_by_orgs(pool, "proposals", &user_orgs).await?;
    let project_refs = fetch_refs_by_orgs(pool, "projects", &user_orgs).await?;
    let client_refs = fetch_refs_by_orgs(pool, "clients", &user_orgs).await?;
    let deal_refs = fetch_refs_by_orgs(pool, "crm_deals", &user_orgs).await?;
    let pipeline_refs = fetch_refs_by_orgs(pool, "crm_pipelines", &user_orgs).await?;

    // Transitive / public.
    let public_company_refs = fetch_refs_with_null_org(pool, "companies").await?;
    let public_proposal_refs = fetch_refs_with_null_org(pool, "proposals").await?;
    let pipeline_stage_refs = fetch_pipeline_stage_refs(pool, &pipeline_refs).await?;
    let brand_profile_refs = fetch_brand_profiles_by_projects(pool, &project_refs).await?;
    let knowledge_source_refs = fetch_ks_by_projects(pool, &project_refs).await?;

    Ok(VisibilityScope {
        org_ids: user_orgs.into_iter().collect(),
        company_refs,
        public_company_refs,
        contact_refs,
        proposal_refs,
        public_proposal_refs,
        project_refs,
        client_refs,
        deal_refs,
        pipeline_refs,
        pipeline_stage_refs,
        brand_profile_refs,
        knowledge_source_refs,
    })
}

fn is_visible(n: &EntityGraphNode, s: &VisibilityScope) -> bool {
    match n.node_type.as_str() {
        "organization" => s.org_ids.contains(&n.ref_id),
        "company" => {
            s.company_refs.contains(&n.ref_id) || s.public_company_refs.contains(&n.ref_id)
        }
        "person" => s.contact_refs.contains(&n.ref_id),
        "proposal" => {
            s.proposal_refs.contains(&n.ref_id) || s.public_proposal_refs.contains(&n.ref_id)
        }
        "project" => s.project_refs.contains(&n.ref_id),
        "client" => s.client_refs.contains(&n.ref_id),
        "deal" => s.deal_refs.contains(&n.ref_id),
        "pipeline" => s.pipeline_refs.contains(&n.ref_id),
        "pipeline_stage" => s.pipeline_stage_refs.contains(&n.ref_id),
        "brand_profile" => s.brand_profile_refs.contains(&n.ref_id),
        "knowledge_source" => s.knowledge_source_refs.contains(&n.ref_id),
        // Unknown/future node types: hide until the matrix covers them.
        _ => false,
    }
}

/// List the org IDs the user is a member of.
pub async fn list_user_orgs(pool: &SqlitePool, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT organization_id FROM organization_members WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Fetch the set of ref_ids of rows in `table` whose `organization_id`
/// matches one of `org_ids`.
async fn fetch_refs_by_orgs(
    pool: &SqlitePool,
    table: &'static str,
    org_ids: &[String],
) -> Result<HashSet<String>, sqlx::Error> {
    if org_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let placeholders = vec!["?"; org_ids.len()].join(",");
    let sql = match table {
        "companies" => format!(
            "SELECT id FROM companies WHERE organization_id IN ({})",
            placeholders
        ),
        "crm_contacts" => format!(
            "SELECT id FROM crm_contacts WHERE organization_id IN ({})",
            placeholders
        ),
        "proposals" => format!(
            "SELECT id FROM proposals WHERE organization_id IN ({})",
            placeholders
        ),
        "projects" => format!(
            "SELECT id FROM projects WHERE organization_id IN ({}) AND deleted_at IS NULL",
            placeholders
        ),
        "clients" => format!(
            "SELECT id FROM clients WHERE organization_id IN ({}) AND deleted_at IS NULL",
            placeholders
        ),
        "crm_deals" => format!(
            "SELECT id FROM crm_deals WHERE organization_id IN ({})",
            placeholders
        ),
        "crm_pipelines" => format!(
            "SELECT id FROM crm_pipelines WHERE organization_id IN ({})",
            placeholders
        ),
        other => panic!("unsupported table in fetch_refs_by_orgs: {}", other),
    };

    let mut q = sqlx::query_scalar::<_, db::db_uuid::DbUuid>(&sql);
    for oid in org_ids {
        q = q.bind(oid);
    }
    let uuids: Vec<db::db_uuid::DbUuid> = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.as_str().to_string()).collect())
}

async fn fetch_refs_with_null_org(
    pool: &SqlitePool,
    table: &'static str,
) -> Result<HashSet<String>, sqlx::Error> {
    let sql = match table {
        "companies" => "SELECT id FROM companies WHERE organization_id IS NULL",
        "proposals" => "SELECT id FROM proposals WHERE organization_id IS NULL",
        other => panic!("unsupported table in fetch_refs_with_null_org: {}", other),
    };
    let uuids: Vec<db::db_uuid::DbUuid> = sqlx::query_scalar::<_, db::db_uuid::DbUuid>(sql)
        .fetch_all(pool)
        .await?;
    Ok(uuids.into_iter().map(|u| u.as_str().to_string()).collect())
}

async fn fetch_pipeline_stage_refs(
    pool: &SqlitePool,
    pipeline_refs: &HashSet<String>,
) -> Result<HashSet<String>, sqlx::Error> {
    if pipeline_refs.is_empty() {
        return Ok(HashSet::new());
    }
    let pipelines: Vec<String> = pipeline_refs.iter().cloned().collect();
    let placeholders = vec!["?"; pipelines.len()].join(",");
    let sql = format!(
        "SELECT id FROM crm_pipeline_stages WHERE pipeline_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_scalar::<_, db::db_uuid::DbUuid>(&sql);
    for pid in &pipelines {
        q = q.bind(pid);
    }
    let uuids = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.to_string()).collect())
}

async fn fetch_brand_profiles_by_projects(
    pool: &SqlitePool,
    project_refs: &HashSet<String>,
) -> Result<HashSet<String>, sqlx::Error> {
    if project_refs.is_empty() {
        return Ok(HashSet::new());
    }
    let projects: Vec<String> = project_refs.iter().cloned().collect();
    let placeholders = vec!["?"; projects.len()].join(",");
    let sql = format!(
        "SELECT id FROM project_brand_profiles WHERE project_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_scalar::<_, db::db_uuid::DbUuid>(&sql);
    for pid in &projects {
        q = q.bind(pid);
    }
    let uuids = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.to_string()).collect())
}

async fn fetch_ks_by_projects(
    pool: &SqlitePool,
    project_refs: &HashSet<String>,
) -> Result<HashSet<String>, sqlx::Error> {
    if project_refs.is_empty() {
        return Ok(HashSet::new());
    }
    let projects: Vec<String> = project_refs.iter().cloned().collect();
    let placeholders = vec!["?"; projects.len()].join(",");
    let sql = format!(
        "SELECT id FROM project_knowledge_sources \
         WHERE project_id IN ({}) AND is_active = 1",
        placeholders
    );
    let mut q = sqlx::query_scalar::<_, db::db_uuid::DbUuid>(&sql);
    for pid in &projects {
        q = q.bind(pid);
    }
    let uuids = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.to_string()).collect())
}

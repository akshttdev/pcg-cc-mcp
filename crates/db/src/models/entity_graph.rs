use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// A node in the business entity graph.
/// Tracks CRM entities (companies, persons, proposals, orgs, projects) for graph visualization.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntityGraphNode {
    #[ts(type = "string")]
    pub id: Uuid,
    pub node_type: String,
    pub ref_id: String,
    pub ref_table: String,
    pub label: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A directed edge between two entity graph nodes.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntityGraphEdge {
    #[ts(type = "string")]
    pub id: Uuid,
    #[ts(type = "string")]
    pub from_node_id: Uuid,
    #[ts(type = "string")]
    pub to_node_id: Uuid,
    pub edge_type: String,
    pub weight: Option<f64>,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// Subgraph centred on a single entity (or the whole graph).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntitySubgraph {
    pub nodes: Vec<EntityGraphNode>,
    pub edges: Vec<EntityGraphEdge>,
}

impl EntityGraphNode {
    /// Upsert a node. Returns the existing or newly created node.
    pub async fn upsert(
        pool: &SqlitePool,
        node_type: &str,
        ref_id: &str,
        ref_table: &str,
        label: &str,
        metadata: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query(
            "INSERT INTO entity_graph_nodes (id, node_type, ref_id, ref_table, label, metadata) \
             VALUES (randomblob(16), ?, ?, ?, ?, ?) \
             ON CONFLICT(node_type, ref_id) DO UPDATE SET \
             label = excluded.label, \
             metadata = COALESCE(excluded.metadata, entity_graph_nodes.metadata), \
             updated_at = datetime('now','subsec')",
        )
        .bind(node_type)
        .bind(ref_id)
        .bind(ref_table)
        .bind(label)
        .bind(metadata)
        .execute(pool)
        .await?;

        Self::find_by_ref(pool, node_type, ref_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_ref(
        pool: &SqlitePool,
        node_type: &str,
        ref_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, node_type, ref_id, ref_table, label, metadata, created_at, updated_at \
             FROM entity_graph_nodes WHERE node_type = ? AND ref_id = ?",
        )
        .bind(node_type)
        .bind(ref_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, node_type, ref_id, ref_table, label, metadata, created_at, updated_at \
             FROM entity_graph_nodes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}

impl EntityGraphEdge {
    /// Upsert an edge between two nodes.
    pub async fn upsert(
        pool: &SqlitePool,
        from_node_id: Uuid,
        to_node_id: Uuid,
        edge_type: &str,
        weight: f64,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query(
            "INSERT INTO entity_graph_edges (id, from_node_id, to_node_id, edge_type, weight) \
             VALUES (randomblob(16), ?, ?, ?, ?) \
             ON CONFLICT(from_node_id, to_node_id, edge_type) DO UPDATE SET \
             weight = excluded.weight",
        )
        .bind(from_node_id)
        .bind(to_node_id)
        .bind(edge_type)
        .bind(weight)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, Self>(
            "SELECT id, from_node_id, to_node_id, edge_type, weight, metadata, created_at \
             FROM entity_graph_edges \
             WHERE from_node_id = ? AND to_node_id = ? AND edge_type = ?",
        )
        .bind(from_node_id)
        .bind(to_node_id)
        .bind(edge_type)
        .fetch_one(pool)
        .await
    }

    pub async fn find_from(pool: &SqlitePool, node_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, from_node_id, to_node_id, edge_type, weight, metadata, created_at \
             FROM entity_graph_edges WHERE from_node_id = ?",
        )
        .bind(node_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_to(pool: &SqlitePool, node_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT id, from_node_id, to_node_id, edge_type, weight, metadata, created_at \
             FROM entity_graph_edges WHERE to_node_id = ?",
        )
        .bind(node_id)
        .fetch_all(pool)
        .await
    }
}

/// Build a subgraph centred on a company node (1-hop neighbourhood).
pub async fn company_subgraph(
    pool: &SqlitePool,
    company_id: Uuid,
) -> Result<EntitySubgraph, sqlx::Error> {
    let company_ref = company_id.to_string();
    let center = match EntityGraphNode::find_by_ref(pool, "company", &company_ref).await? {
        Some(n) => n,
        None => {
            return Ok(EntitySubgraph {
                nodes: vec![],
                edges: vec![],
            });
        }
    };

    // Collect all edges touching the company node
    let out_edges = EntityGraphEdge::find_from(pool, center.id).await?;
    let in_edges = EntityGraphEdge::find_to(pool, center.id).await?;

    let mut edges: Vec<EntityGraphEdge> = out_edges;
    edges.extend(in_edges);

    // Collect connected node IDs
    let mut node_ids: Vec<Uuid> = vec![center.id];
    for e in &edges {
        if e.from_node_id != center.id {
            node_ids.push(e.from_node_id);
        }
        if e.to_node_id != center.id {
            node_ids.push(e.to_node_id);
        }
    }
    node_ids.sort();
    node_ids.dedup();

    // Fetch all connected nodes
    let mut nodes: Vec<EntityGraphNode> = vec![];
    for nid in node_ids {
        if let Some(n) = EntityGraphNode::find_by_id(pool, nid).await? {
            nodes.push(n);
        }
    }

    Ok(EntitySubgraph { nodes, edges })
}

/// All entity-graph nodes in the database. Unfiltered.
/// Callers that need access-filtering should use the graph_access service layer.
pub async fn all_nodes(pool: &SqlitePool) -> Result<Vec<EntityGraphNode>, sqlx::Error> {
    sqlx::query_as::<_, EntityGraphNode>(
        "SELECT id, node_type, ref_id, ref_table, label, metadata, created_at, updated_at \
         FROM entity_graph_nodes",
    )
    .fetch_all(pool)
    .await
}

/// All entity-graph edges in the database. Unfiltered.
pub async fn all_edges(pool: &SqlitePool) -> Result<Vec<EntityGraphEdge>, sqlx::Error> {
    sqlx::query_as::<_, EntityGraphEdge>(
        "SELECT id, from_node_id, to_node_id, edge_type, weight, metadata, created_at \
         FROM entity_graph_edges",
    )
    .fetch_all(pool)
    .await
}

/// Global subgraph: every node and every edge. Intended to be passed through
/// an access filter before returning to a non-admin caller.
pub async fn global_subgraph(pool: &SqlitePool) -> Result<EntitySubgraph, sqlx::Error> {
    let nodes = all_nodes(pool).await?;
    let edges = all_edges(pool).await?;
    Ok(EntitySubgraph { nodes, edges })
}

/// Subgraph scoped to a single organization. Includes:
///   - the organization node itself
///   - every `company`, `person`, `proposal`, `project` node whose
///     underlying row has `organization_id = ?`
///   - every edge whose endpoints are both in the above set
///
/// Phase 0: 5 queries + in-memory filter. N is the table ID count, not the
/// node count, so this is bounded. Denormalization for O(1) lookup is a
/// later optimization.
pub async fn org_subgraph(
    pool: &SqlitePool,
    org_id: &crate::db_uuid::DbUuid,
) -> Result<EntitySubgraph, sqlx::Error> {
    use std::collections::HashSet;

    let org_ref = org_id.as_str().to_string();

    // Directly-org-scoped entities
    let company_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM companies WHERE organization_id = ?",
            org_id,
        )
        .await?,
    );
    let contact_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM crm_contacts WHERE organization_id = ?",
            org_id,
        )
        .await?,
    );
    let proposal_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM proposals WHERE organization_id = ?",
            org_id,
        )
        .await?,
    );
    let project_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM projects WHERE organization_id = ? AND deleted_at IS NULL",
            org_id,
        )
        .await?,
    );
    let client_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM clients WHERE organization_id = ? AND deleted_at IS NULL",
            org_id,
        )
        .await?,
    );
    let deal_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM crm_deals WHERE organization_id = ?",
            org_id,
        )
        .await?,
    );
    let pipeline_ref_ids: HashSet<String> = uuids_to_ref_set(
        query_ids(
            pool,
            "SELECT id FROM crm_pipelines WHERE organization_id = ?",
            org_id,
        )
        .await?,
    );

    // Transitive: pipeline_stages via pipeline, brand_profile + KS via project
    let pipeline_stage_ref_ids: HashSet<String> =
        stage_refs_for_pipelines(pool, &pipeline_ref_ids).await?;
    let brand_profile_ref_ids: HashSet<String> =
        brand_profile_refs_for_projects(pool, &project_ref_ids).await?;
    let knowledge_source_ref_ids: HashSet<String> =
        ks_refs_for_projects(pool, &project_ref_ids).await?;

    let nodes: Vec<EntityGraphNode> = all_nodes(pool)
        .await?
        .into_iter()
        .filter(|n| match n.node_type.as_str() {
            "organization" => n.ref_id == org_ref,
            "company" => company_ref_ids.contains(&n.ref_id),
            "person" => contact_ref_ids.contains(&n.ref_id),
            "proposal" => proposal_ref_ids.contains(&n.ref_id),
            "project" => project_ref_ids.contains(&n.ref_id),
            "client" => client_ref_ids.contains(&n.ref_id),
            "deal" => deal_ref_ids.contains(&n.ref_id),
            "pipeline" => pipeline_ref_ids.contains(&n.ref_id),
            "pipeline_stage" => pipeline_stage_ref_ids.contains(&n.ref_id),
            "brand_profile" => brand_profile_ref_ids.contains(&n.ref_id),
            "knowledge_source" => knowledge_source_ref_ids.contains(&n.ref_id),
            _ => false,
        })
        .collect();

    let node_id_set: HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
    let edges: Vec<EntityGraphEdge> = all_edges(pool)
        .await?
        .into_iter()
        .filter(|e| node_id_set.contains(&e.from_node_id) && node_id_set.contains(&e.to_node_id))
        .collect();

    Ok(EntitySubgraph { nodes, edges })
}

/// BFS subgraph centred on a focus node, up to `depth` hops. Depth is capped
/// at 5 to prevent accidental whole-graph fetches.
pub async fn subgraph_focused(
    pool: &SqlitePool,
    focus_node_id: Uuid,
    depth: u32,
) -> Result<EntitySubgraph, sqlx::Error> {
    use std::collections::{HashMap, HashSet, VecDeque};

    let depth = depth.min(5);

    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut frontier: VecDeque<(Uuid, u32)> = VecDeque::new();
    frontier.push_back((focus_node_id, 0));
    visited.insert(focus_node_id);

    let mut all_frontier_edges: Vec<EntityGraphEdge> = Vec::new();

    while let Some((nid, d)) = frontier.pop_front() {
        if d >= depth {
            continue;
        }
        let outs = EntityGraphEdge::find_from(pool, nid).await?;
        let ins = EntityGraphEdge::find_to(pool, nid).await?;
        for e in outs.iter().chain(ins.iter()) {
            if visited.insert(e.from_node_id) {
                frontier.push_back((e.from_node_id, d + 1));
            }
            if visited.insert(e.to_node_id) {
                frontier.push_back((e.to_node_id, d + 1));
            }
        }
        all_frontier_edges.extend(outs);
        all_frontier_edges.extend(ins);
    }

    // Dedupe edges by id
    let mut seen: HashSet<Uuid> = HashSet::new();
    let edges: Vec<EntityGraphEdge> = all_frontier_edges
        .into_iter()
        .filter(|e| seen.insert(e.id))
        .collect();

    // Fetch the discovered nodes
    let mut nodes_map: HashMap<Uuid, EntityGraphNode> = HashMap::new();
    for nid in visited {
        if let Some(n) = EntityGraphNode::find_by_id(pool, nid).await? {
            nodes_map.insert(nid, n);
        }
    }
    let nodes: Vec<EntityGraphNode> = nodes_map.into_values().collect();

    Ok(EntitySubgraph { nodes, edges })
}

async fn query_ids(
    pool: &SqlitePool,
    sql: &str,
    org_id: &crate::db_uuid::DbUuid,
) -> Result<Vec<crate::db_uuid::DbUuid>, sqlx::Error> {
    sqlx::query_scalar::<_, crate::db_uuid::DbUuid>(sql)
        .bind(org_id)
        .fetch_all(pool)
        .await
}

fn uuids_to_ref_set(ids: Vec<crate::db_uuid::DbUuid>) -> std::collections::HashSet<String> {
    ids.into_iter().map(|u| u.as_str().to_string()).collect()
}

async fn stage_refs_for_pipelines(
    pool: &SqlitePool,
    pipeline_refs: &std::collections::HashSet<String>,
) -> Result<std::collections::HashSet<String>, sqlx::Error> {
    if pipeline_refs.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let pipelines: Vec<String> = pipeline_refs.iter().cloned().collect();
    let placeholders = vec!["?"; pipelines.len()].join(",");
    let sql = format!(
        "SELECT id FROM crm_pipeline_stages WHERE pipeline_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_scalar::<_, crate::db_uuid::DbUuid>(&sql);
    for pid in &pipelines {
        q = q.bind(pid);
    }
    let uuids = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.as_str().to_string()).collect())
}

async fn brand_profile_refs_for_projects(
    pool: &SqlitePool,
    project_refs: &std::collections::HashSet<String>,
) -> Result<std::collections::HashSet<String>, sqlx::Error> {
    if project_refs.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let projects: Vec<String> = project_refs.iter().cloned().collect();
    let placeholders = vec!["?"; projects.len()].join(",");
    let sql = format!(
        "SELECT id FROM project_brand_profiles WHERE project_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_scalar::<_, crate::db_uuid::DbUuid>(&sql);
    for pid in &projects {
        q = q.bind(pid);
    }
    let uuids = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.as_str().to_string()).collect())
}

async fn ks_refs_for_projects(
    pool: &SqlitePool,
    project_refs: &std::collections::HashSet<String>,
) -> Result<std::collections::HashSet<String>, sqlx::Error> {
    if project_refs.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let projects: Vec<String> = project_refs.iter().cloned().collect();
    let placeholders = vec!["?"; projects.len()].join(",");
    let sql = format!(
        "SELECT id FROM project_knowledge_sources \
         WHERE project_id IN ({}) AND is_active = 1",
        placeholders
    );
    let mut q = sqlx::query_scalar::<_, crate::db_uuid::DbUuid>(&sql);
    for pid in &projects {
        q = q.bind(pid);
    }
    let uuids = q.fetch_all(pool).await?;
    Ok(uuids.into_iter().map(|u| u.as_str().to_string()).collect())
}

/// Upsert an organization node. No outbound edges — orgs are the top of
/// the scoping tree; edges point INTO orgs from companies/clients/etc.
pub async fn sync_org_graph(
    pool: &SqlitePool,
    org_id: Uuid,
    org_name: &str,
) -> Result<(), sqlx::Error> {
    let ref_id = org_id.to_string();
    EntityGraphNode::upsert(
        pool,
        "organization",
        &ref_id,
        "organizations",
        org_name,
        None,
    )
    .await?;
    Ok(())
}

/// Upsert a client node + `has_client` edge from its parent organization.
pub async fn sync_client_graph(
    pool: &SqlitePool,
    client_id: Uuid,
    client_name: &str,
    organization_id: Uuid,
) -> Result<(), sqlx::Error> {
    let client_ref = client_id.to_string();
    let org_ref = organization_id.to_string();

    let client_node =
        EntityGraphNode::upsert(pool, "client", &client_ref, "clients", client_name, None).await?;

    // Parent org node — label fetched lazily if we haven't seen it yet.
    let org_node = upsert_org_by_ref(pool, &org_ref).await?;
    EntityGraphEdge::upsert(pool, org_node.id, client_node.id, "has_client", 1.0).await?;
    Ok(())
}

/// Upsert a project node + edges to its organization and client.
pub async fn sync_project_graph(
    pool: &SqlitePool,
    project_id: Uuid,
    project_name: &str,
    organization_id: Option<Uuid>,
    client_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    let project_ref = project_id.to_string();

    let project_node = EntityGraphNode::upsert(
        pool,
        "project",
        &project_ref,
        "projects",
        project_name,
        None,
    )
    .await?;

    if let Some(org_id) = organization_id {
        let org_node = upsert_org_by_ref(pool, &org_id.to_string()).await?;
        EntityGraphEdge::upsert(pool, project_node.id, org_node.id, "part_of_org", 1.0).await?;
    }

    if let Some(cid) = client_id {
        let client_ref = cid.to_string();
        if let Some(client_node) = EntityGraphNode::find_by_ref(pool, "client", &client_ref).await?
        {
            EntityGraphEdge::upsert(
                pool,
                client_node.id,
                project_node.id,
                "has_deliverable",
                1.0,
            )
            .await?;
        }
    }

    Ok(())
}

/// Upsert a deal node + edges to contact, pipeline, stage, and org.
/// Note: crm_deals has no `company_id` column today — the deal↔company
/// relationship is implied through the contact's `company_id`.
pub async fn sync_deal_graph(
    pool: &SqlitePool,
    deal_id: Uuid,
    deal_name: &str,
    organization_id: Uuid,
    crm_contact_id: Option<Uuid>,
    crm_pipeline_id: Option<Uuid>,
    crm_stage_id: Option<Uuid>,
    amount: Option<f64>,
    stage_label: Option<&str>,
) -> Result<(), sqlx::Error> {
    let deal_ref = deal_id.to_string();
    let metadata = match (amount, stage_label) {
        (Some(a), Some(s)) => Some(format!(r#"{{"amount":{},"stage":"{}"}}"#, a, escape(s))),
        (Some(a), None) => Some(format!(r#"{{"amount":{}}}"#, a)),
        (None, Some(s)) => Some(format!(r#"{{"stage":"{}"}}"#, escape(s))),
        (None, None) => None,
    };
    let deal_node = EntityGraphNode::upsert(
        pool,
        "deal",
        &deal_ref,
        "crm_deals",
        deal_name,
        metadata.as_deref(),
    )
    .await?;

    // Deal → organization (scoping)
    let org_node = upsert_org_by_ref(pool, &organization_id.to_string()).await?;
    EntityGraphEdge::upsert(pool, deal_node.id, org_node.id, "part_of_org", 1.0).await?;

    // Deal → contact (person)
    if let Some(cid) = crm_contact_id {
        let contact_ref = cid.to_string();
        if let Some(contact_node) =
            EntityGraphNode::find_by_ref(pool, "person", &contact_ref).await?
        {
            EntityGraphEdge::upsert(pool, deal_node.id, contact_node.id, "has_deal", 1.0).await?;
        }
    }

    // Deal → pipeline / stage
    if let Some(pid) = crm_pipeline_id {
        let pipeline_ref = pid.to_string();
        if let Some(pipeline_node) =
            EntityGraphNode::find_by_ref(pool, "pipeline", &pipeline_ref).await?
        {
            EntityGraphEdge::upsert(pool, deal_node.id, pipeline_node.id, "in_pipeline", 1.0)
                .await?;
        }
    }
    if let Some(sid) = crm_stage_id {
        let stage_ref = sid.to_string();
        if let Some(stage_node) =
            EntityGraphNode::find_by_ref(pool, "pipeline_stage", &stage_ref).await?
        {
            EntityGraphEdge::upsert(pool, deal_node.id, stage_node.id, "in_stage", 1.0).await?;
        }
    }

    Ok(())
}

/// Upsert a pipeline node + its stages as `pipeline_stage` nodes,
/// with `in_stage` edges from pipeline to stage.
pub async fn sync_pipeline_graph(
    pool: &SqlitePool,
    pipeline_id: Uuid,
    pipeline_name: &str,
    organization_id: Uuid,
) -> Result<(), sqlx::Error> {
    let pipeline_ref = pipeline_id.to_string();
    let pipeline_node = EntityGraphNode::upsert(
        pool,
        "pipeline",
        &pipeline_ref,
        "crm_pipelines",
        pipeline_name,
        None,
    )
    .await?;

    // Pipeline → org
    let org_node = upsert_org_by_ref(pool, &organization_id.to_string()).await?;
    EntityGraphEdge::upsert(pool, pipeline_node.id, org_node.id, "part_of_org", 1.0).await?;

    // Stages
    #[derive(sqlx::FromRow)]
    struct StageRow {
        id: crate::db_uuid::DbUuid,
        name: String,
        position: i64,
    }
    let stages: Vec<StageRow> = sqlx::query_as(
        "SELECT id, name, position FROM crm_pipeline_stages \
         WHERE pipeline_id = ? ORDER BY position ASC",
    )
    .bind(pipeline_id.to_string())
    .fetch_all(pool)
    .await?;

    for s in stages {
        let stage_ref = s.id.as_str().to_string();
        let metadata = Some(format!(r#"{{"position":{}}}"#, s.position));
        let stage_node = EntityGraphNode::upsert(
            pool,
            "pipeline_stage",
            &stage_ref,
            "crm_pipeline_stages",
            &s.name,
            metadata.as_deref(),
        )
        .await?;
        EntityGraphEdge::upsert(pool, pipeline_node.id, stage_node.id, "in_stage", 1.0).await?;
    }

    Ok(())
}

/// Upsert a proposal node + `lead_on` edge from the lead contact.
/// Prefers `lead_contact_id` (points at `crm_contacts.id`, post-migration
/// 20260428100000). Falls back to the legacy `lead_id → persons` bridge via
/// `persons.crm_contact_id` for rows that predate the migration.
pub async fn sync_proposal_graph(
    pool: &SqlitePool,
    proposal_id: Uuid,
    proposal_title: &str,
    company_id: Option<Uuid>,
    lead_contact_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    status: Option<&str>,
) -> Result<(), sqlx::Error> {
    let proposal_ref = proposal_id.to_string();
    let metadata = status.map(|s| format!(r#"{{"status":"{}"}}"#, escape(s)));

    let proposal_node = EntityGraphNode::upsert(
        pool,
        "proposal",
        &proposal_ref,
        "proposals",
        proposal_title,
        metadata.as_deref(),
    )
    .await?;

    // Proposal → company (targets)
    if let Some(cid) = company_id {
        let company_ref = cid.to_string();
        if let Some(company_node) =
            EntityGraphNode::find_by_ref(pool, "company", &company_ref).await?
        {
            EntityGraphEdge::upsert(pool, proposal_node.id, company_node.id, "targets", 1.0)
                .await?;
        }
    }

    // Lead contact → proposal (lead_on)
    if let Some(lcid) = lead_contact_id {
        let contact_ref = lcid.to_string();
        if let Some(contact_node) =
            EntityGraphNode::find_by_ref(pool, "person", &contact_ref).await?
        {
            EntityGraphEdge::upsert(pool, contact_node.id, proposal_node.id, "lead_on", 1.0)
                .await?;
        }
    }

    // Proposal → organization
    if let Some(oid) = organization_id {
        let org_node = upsert_org_by_ref(pool, &oid.to_string()).await?;
        EntityGraphEdge::upsert(pool, proposal_node.id, org_node.id, "part_of_org", 1.0).await?;
    }

    Ok(())
}

/// Upsert a brand_profile node + `has_brand_profile` edge from its project.
pub async fn sync_brand_profile_graph(
    pool: &SqlitePool,
    profile_id: Uuid,
    project_id: Uuid,
    tagline: Option<&str>,
) -> Result<(), sqlx::Error> {
    let profile_ref = profile_id.to_string();
    let label = tagline.unwrap_or("Brand profile");

    let profile_node = EntityGraphNode::upsert(
        pool,
        "brand_profile",
        &profile_ref,
        "project_brand_profiles",
        label,
        None,
    )
    .await?;

    let project_ref = project_id.to_string();
    if let Some(project_node) = EntityGraphNode::find_by_ref(pool, "project", &project_ref).await? {
        EntityGraphEdge::upsert(
            pool,
            project_node.id,
            profile_node.id,
            "has_brand_profile",
            1.0,
        )
        .await?;
    }

    Ok(())
}

/// Upsert a knowledge_source node + `owns_knowledge` edge from its owner
/// (project, org, user, company, or deal depending on owner_scope).
pub async fn sync_knowledge_source_graph(
    pool: &SqlitePool,
    ks_id: Uuid,
    ref_table: &str, // "project_knowledge_sources" | "user_knowledge_sources"
    title: &str,
    owner_node_type: &str, // "project" | "organization" | "agent" | "user" → use "project" fallback
    owner_ref_id: Option<Uuid>,
    source_type: Option<&str>,
) -> Result<(), sqlx::Error> {
    let ks_ref = ks_id.to_string();
    let metadata = source_type.map(|s| format!(r#"{{"source_type":"{}"}}"#, escape(s)));

    let ks_node = EntityGraphNode::upsert(
        pool,
        "knowledge_source",
        &ks_ref,
        ref_table,
        title,
        metadata.as_deref(),
    )
    .await?;

    if let Some(owner_id) = owner_ref_id {
        let owner_ref = owner_id.to_string();
        if let Some(owner_node) =
            EntityGraphNode::find_by_ref(pool, owner_node_type, &owner_ref).await?
        {
            EntityGraphEdge::upsert(pool, owner_node.id, ks_node.id, "owns_knowledge", 1.0).await?;
        }
    }

    Ok(())
}

/// Upsert an org node looking up its name from the `organizations` table.
/// Falls back to a short-id label if the row is missing (dangling ref).
async fn upsert_org_by_ref(
    pool: &SqlitePool,
    org_ref: &str,
) -> Result<EntityGraphNode, sqlx::Error> {
    if let Some(existing) = EntityGraphNode::find_by_ref(pool, "organization", org_ref).await? {
        return Ok(existing);
    }

    #[derive(sqlx::FromRow)]
    struct OrgName {
        name: String,
    }
    let row: Option<OrgName> = sqlx::query_as("SELECT name FROM organizations WHERE id = ?")
        .bind(org_ref)
        .fetch_optional(pool)
        .await?;
    let label = row
        .map(|r| r.name)
        .unwrap_or_else(|| format!("Org {}", &org_ref[..8.min(org_ref.len())]));

    EntityGraphNode::upsert(pool, "organization", org_ref, "organizations", &label, None).await
}

/// Minimal JSON string escape for metadata blobs we construct.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Rebuild the entire entity graph from live data. Iterates every entity
/// table we materialize (organizations → clients → companies → projects →
/// pipelines → deals → proposals → brand_profiles → knowledge_sources) and
/// upserts nodes + edges. Idempotent.
///
/// Ordering matters: parent nodes are synced before children so that child
/// syncers can find them via `find_by_ref` and wire edges correctly.
pub async fn sync_global_graph(pool: &SqlitePool) -> Result<SyncGlobalStats, sqlx::Error> {
    use crate::db_uuid::DbUuid;
    let mut stats = SyncGlobalStats::default();

    // Mixed BLOB/TEXT columns: use DbUuid which decodes both, then convert to
    // Uuid for the sync function signatures. Rows with un-parseable IDs are
    // skipped (shouldn't happen in practice; defensive).
    let parse = |s: &str| -> Option<Uuid> { Uuid::parse_str(s).ok() };

    // 1. Organizations
    #[derive(sqlx::FromRow)]
    struct OrgRow {
        id: DbUuid,
        name: String,
    }
    let orgs: Vec<OrgRow> =
        sqlx::query_as("SELECT id, name FROM organizations WHERE deleted_at IS NULL")
            .fetch_all(pool)
            .await?;
    for o in &orgs {
        if let Some(id) = parse(o.id.as_str()) {
            sync_org_graph(pool, id, &o.name).await?;
            stats.organizations_synced += 1;
        }
    }

    // 2. Clients
    #[derive(sqlx::FromRow)]
    struct ClientRow {
        id: DbUuid,
        name: String,
        organization_id: DbUuid,
    }
    let clients: Vec<ClientRow> =
        sqlx::query_as("SELECT id, name, organization_id FROM clients WHERE deleted_at IS NULL")
            .fetch_all(pool)
            .await?;
    for c in &clients {
        if let (Some(id), Some(org_id)) = (parse(c.id.as_str()), parse(c.organization_id.as_str()))
        {
            sync_client_graph(pool, id, &c.name, org_id).await?;
            stats.clients_synced += 1;
        }
    }

    // 3. Companies (reuses existing sync)
    #[derive(sqlx::FromRow)]
    struct CompanyRow {
        id: DbUuid,
        name: String,
        organization_id: Option<DbUuid>,
    }
    let companies: Vec<CompanyRow> =
        sqlx::query_as("SELECT id, name, organization_id FROM companies")
            .fetch_all(pool)
            .await?;
    for c in &companies {
        if let Some(id) = parse(c.id.as_str()) {
            let org_id = c.organization_id.as_ref().and_then(|o| parse(o.as_str()));
            sync_company_graph(pool, id, &c.name, org_id).await?;
            stats.companies_synced += 1;
        }
    }

    // 4. Projects
    #[derive(sqlx::FromRow)]
    struct ProjectRow {
        id: DbUuid,
        name: String,
        organization_id: Option<DbUuid>,
        client_id: Option<DbUuid>,
    }
    let projects: Vec<ProjectRow> = sqlx::query_as(
        "SELECT id, name, organization_id, client_id FROM projects WHERE deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;
    for p in &projects {
        if let Some(id) = parse(p.id.as_str()) {
            let org_id = p.organization_id.as_ref().and_then(|o| parse(o.as_str()));
            let client_id = p.client_id.as_ref().and_then(|c| parse(c.as_str()));
            sync_project_graph(pool, id, &p.name, org_id, client_id).await?;
            stats.projects_synced += 1;
        }
    }

    // 5. Pipelines + stages
    #[derive(sqlx::FromRow)]
    struct PipelineRow {
        id: DbUuid,
        name: String,
        organization_id: DbUuid,
    }
    let pipelines: Vec<PipelineRow> =
        sqlx::query_as("SELECT id, name, organization_id FROM crm_pipelines")
            .fetch_all(pool)
            .await?;
    for p in &pipelines {
        if let (Some(id), Some(org_id)) = (parse(p.id.as_str()), parse(p.organization_id.as_str()))
        {
            sync_pipeline_graph(pool, id, &p.name, org_id).await?;
            stats.pipelines_synced += 1;
        }
    }

    // 6. Deals
    #[derive(sqlx::FromRow)]
    struct DealRow {
        id: DbUuid,
        name: String,
        organization_id: DbUuid,
        crm_contact_id: Option<DbUuid>,
        crm_pipeline_id: Option<DbUuid>,
        crm_stage_id: Option<DbUuid>,
        amount: Option<f64>,
        stage: Option<String>,
    }
    let deals: Vec<DealRow> = sqlx::query_as(
        "SELECT id, name, organization_id, crm_contact_id, crm_pipeline_id, \
                crm_stage_id, amount, stage FROM crm_deals",
    )
    .fetch_all(pool)
    .await?;
    for d in &deals {
        let Some(id) = parse(d.id.as_str()) else {
            continue;
        };
        let Some(org_id) = parse(d.organization_id.as_str()) else {
            continue;
        };
        sync_deal_graph(
            pool,
            id,
            &d.name,
            org_id,
            d.crm_contact_id.as_ref().and_then(|x| parse(x.as_str())),
            d.crm_pipeline_id.as_ref().and_then(|x| parse(x.as_str())),
            d.crm_stage_id.as_ref().and_then(|x| parse(x.as_str())),
            d.amount,
            d.stage.as_deref(),
        )
        .await?;
        stats.deals_synced += 1;
    }

    // 7. Proposals (prefer the new lead_contact_id column)
    #[derive(sqlx::FromRow)]
    struct ProposalRow {
        id: DbUuid,
        title: String,
        company_id: Option<DbUuid>,
        lead_contact_id: Option<DbUuid>,
        organization_id: Option<DbUuid>,
        status: Option<String>,
    }
    let proposals: Vec<ProposalRow> = sqlx::query_as(
        "SELECT id, title, company_id, lead_contact_id, organization_id, status \
         FROM proposals",
    )
    .fetch_all(pool)
    .await?;
    for p in &proposals {
        let Some(id) = parse(p.id.as_str()) else {
            continue;
        };
        sync_proposal_graph(
            pool,
            id,
            &p.title,
            p.company_id.as_ref().and_then(|x| parse(x.as_str())),
            p.lead_contact_id.as_ref().and_then(|x| parse(x.as_str())),
            p.organization_id.as_ref().and_then(|x| parse(x.as_str())),
            p.status.as_deref(),
        )
        .await?;
        stats.proposals_synced += 1;
    }

    // 8. Brand profiles
    #[derive(sqlx::FromRow)]
    struct BrandRow {
        id: DbUuid,
        project_id: DbUuid,
        tagline: Option<String>,
    }
    let brand_profiles: Vec<BrandRow> =
        sqlx::query_as("SELECT id, project_id, tagline FROM project_brand_profiles")
            .fetch_all(pool)
            .await?;
    for b in &brand_profiles {
        if let (Some(id), Some(pid)) = (parse(b.id.as_str()), parse(b.project_id.as_str())) {
            sync_brand_profile_graph(pool, id, pid, b.tagline.as_deref()).await?;
            stats.brand_profiles_synced += 1;
        }
    }

    // 9. Knowledge sources (project-scoped)
    #[derive(sqlx::FromRow)]
    struct ProjectKsRow {
        id: DbUuid,
        project_id: Option<DbUuid>,
        source_title: String,
        source_type: Option<String>,
    }
    let project_ks: Vec<ProjectKsRow> = sqlx::query_as(
        "SELECT id, project_id, source_title, source_type \
         FROM project_knowledge_sources WHERE is_active = 1",
    )
    .fetch_all(pool)
    .await?;
    for k in &project_ks {
        if let Some(id) = parse(k.id.as_str()) {
            sync_knowledge_source_graph(
                pool,
                id,
                "project_knowledge_sources",
                &k.source_title,
                "project",
                k.project_id.as_ref().and_then(|p| parse(p.as_str())),
                k.source_type.as_deref(),
            )
            .await?;
            stats.knowledge_sources_synced += 1;
        }
    }

    Ok(stats)
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncGlobalStats {
    pub organizations_synced: u32,
    pub clients_synced: u32,
    pub companies_synced: u32,
    pub projects_synced: u32,
    pub pipelines_synced: u32,
    pub deals_synced: u32,
    pub proposals_synced: u32,
    pub brand_profiles_synced: u32,
    pub knowledge_sources_synced: u32,
}

/// Sync entity graph nodes and edges for a company from live data.
/// Creates/updates nodes for the company, its persons, and related proposals.
pub async fn sync_company_graph(
    pool: &SqlitePool,
    company_id: Uuid,
    company_name: &str,
    organization_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    let company_ref = company_id.to_string();

    // Company node
    let company_node = EntityGraphNode::upsert(
        pool,
        "company",
        &company_ref,
        "companies",
        company_name,
        None,
    )
    .await?;

    // If linked to an org, add that node + edge
    if let Some(org_id) = organization_id {
        let org_ref = org_id.to_string();

        // Fetch org name
        #[derive(sqlx::FromRow)]
        struct OrgName {
            name: String,
        }
        let org_name_row: Option<OrgName> =
            sqlx::query_as("SELECT name FROM organizations WHERE id = ?")
                .bind(org_id)
                .fetch_optional(pool)
                .await?;
        let org_label = org_name_row
            .map(|r| r.name)
            .unwrap_or_else(|| format!("Org {}", &org_ref[..8]));

        let org_node = EntityGraphNode::upsert(
            pool,
            "organization",
            &org_ref,
            "organizations",
            &org_label,
            None,
        )
        .await?;

        EntityGraphEdge::upsert(pool, company_node.id, org_node.id, "part_of_org", 1.0).await?;
    }

    // Contacts associated with this company
    #[derive(sqlx::FromRow)]
    struct ContactRow {
        id: crate::db_uuid::DbUuid,
        full_name: Option<String>,
    }
    let contacts: Vec<ContactRow> =
        sqlx::query_as("SELECT id, full_name FROM crm_contacts WHERE company_id = ?")
            .bind(company_id.to_string())
            .fetch_all(pool)
            .await?;

    for p in contacts {
        let person_ref = p.id.as_str().to_string();
        let name = p.full_name.as_deref().unwrap_or("Unknown");
        let person_node =
            EntityGraphNode::upsert(pool, "person", &person_ref, "crm_contacts", name, None)
                .await?;
        EntityGraphEdge::upsert(pool, company_node.id, person_node.id, "employs", 1.0).await?;
    }

    // Proposals targeting this company
    #[derive(sqlx::FromRow)]
    struct ProposalRow {
        id: crate::db_uuid::DbUuid,
        title: String,
    }
    let proposals: Vec<ProposalRow> =
        sqlx::query_as("SELECT id, title FROM proposals WHERE company_id = ?")
            .bind(company_id.to_string())
            .fetch_all(pool)
            .await?;

    for pr in proposals {
        let proposal_ref = pr.id.as_str().to_string();
        let proposal_node = EntityGraphNode::upsert(
            pool,
            "proposal",
            &proposal_ref,
            "proposals",
            &pr.title,
            None,
        )
        .await?;
        EntityGraphEdge::upsert(pool, proposal_node.id, company_node.id, "targets", 1.0).await?;
    }

    Ok(())
}

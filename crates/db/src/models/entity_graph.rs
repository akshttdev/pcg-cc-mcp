use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// A node in the business entity graph.
/// Tracks CRM entities (companies, persons, proposals, orgs, projects) for graph visualization.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EntityGraphNode {
    pub id: Uuid,
    pub node_type: String, // 'company'|'person'|'organization'|'proposal'|'project'
    pub ref_id: String,    // UUID hex
    pub ref_table: String, // table name
    pub label: String,
    pub metadata: Option<String>, // JSON
    pub created_at: String,
    pub updated_at: String,
}

/// A directed edge between two entity graph nodes.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EntityGraphEdge {
    pub id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub edge_type: String, // 'employs'|'targets'|'lead_on'|'part_of_org'|'manages'
    pub weight: Option<f64>,
    pub metadata: Option<String>, // JSON
    pub created_at: String,
}

/// Subgraph centred on a single entity.
#[derive(Debug, Serialize, Deserialize)]
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

    // Persons associated with this company
    #[derive(sqlx::FromRow)]
    struct PersonRow {
        id: Uuid,
        full_name: String,
    }
    let persons: Vec<PersonRow> =
        sqlx::query_as("SELECT id, full_name FROM persons WHERE company_id = ?")
            .bind(company_id)
            .fetch_all(pool)
            .await?;

    for p in persons {
        let person_ref = p.id.to_string();
        let person_node =
            EntityGraphNode::upsert(pool, "person", &person_ref, "persons", &p.full_name, None)
                .await?;
        EntityGraphEdge::upsert(pool, company_node.id, person_node.id, "employs", 1.0).await?;
    }

    // Proposals targeting this company
    #[derive(sqlx::FromRow)]
    struct ProposalRow {
        id: Uuid,
        title: String,
    }
    let proposals: Vec<ProposalRow> =
        sqlx::query_as("SELECT id, title FROM proposals WHERE company_id = ?")
            .bind(company_id)
            .fetch_all(pool)
            .await?;

    for pr in proposals {
        let proposal_ref = pr.id.to_string();
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
